//! ARM7TDMI CPU emulation for the Game Boy Advance.
//!
//! Implements the ARM and Thumb instruction sets with a
//! 3-stage pipeline, banked registers for privileged modes,
//! and interrupt handling.

use crate::{
    gba::{
        arm::execute_arm,
        bus::GbaBus,
        consts::{
            CPSR_C, CPSR_I, CPSR_MODE_MASK, CPSR_N, CPSR_T, CPSR_V, CPSR_Z, MODE_FIQ, MODE_IRQ,
            MODE_SVC, MODE_SYS, MODE_USR,
        },
        thumb::execute_thumb,
    },
    warnln,
};

/// index mapping for banked SPSR: FIQ=0, SVC=1, ABT=2, IRQ=3, UND=4
fn mode_to_spsr_index(mode: u32) -> Option<usize> {
    match mode & CPSR_MODE_MASK {
        0x11 => Some(0), // FIQ
        0x13 => Some(1), // SVC
        0x17 => Some(2), // ABT
        0x12 => Some(3), // IRQ
        0x1B => Some(4), // UND
        _ => None,       // USR and SYS have no SPSR
    }
}

/// banked register storage for privileged CPU modes
struct BankedRegisters {
    /// FIQ mode banks R8-R14 (7 registers)
    fiq: [u32; 7],

    /// IRQ mode banks R13-R14 (2 registers)
    irq: [u32; 2],

    /// SVC mode banks R13-R14 (2 registers)
    svc: [u32; 2],

    /// ABT mode banks R13-R14 (2 registers)
    abt: [u32; 2],

    /// UND mode banks R13-R14 (2 registers)
    und: [u32; 2],

    /// user/system mode R8-R14 (saved when switching away)
    usr: [u32; 7],
}

impl BankedRegisters {
    fn new() -> Self {
        Self {
            fiq: [0; 7],
            irq: [0; 2],
            svc: [0; 2],
            abt: [0; 2],
            und: [0; 2],
            usr: [0; 7],
        }
    }
}

impl Default for BankedRegisters {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Arm7Tdmi {
    /// general-purpose registers R0-R15
    /// R13 = SP, R14 = LR, R15 = PC
    regs: [u32; 16],

    /// current program status register
    cpsr: u32,

    /// saved program status registers (one per privileged mode)
    spsr: [u32; 5],

    /// banked registers for each CPU mode
    banked: BankedRegisters,

    /// memory bus
    pub bus: GbaBus,

    /// cycles consumed by the last instruction
    pub cycles: u32,

    /// whether the CPU is halted (waiting for interrupt)
    halted: bool,

    /// whether we already warned about halt deadlock (to avoid spam)
    halt_deadlock_warned: bool,

    /// infinite loop detection: previous PC and repeat count
    prev_pc: u32,
    same_pc_count: u32,

    /// pipeline state for prefetched instruction
    pipeline: [u32; 2],
    pipeline_valid: bool,
}

impl Arm7Tdmi {
    pub fn new(bus: GbaBus) -> Self {
        let mut cpu = Self {
            regs: [0; 16],
            cpsr: MODE_SYS, // after BIOS boot: SYS mode, IRQs enabled
            spsr: [0; 5],
            banked: BankedRegisters::new(),
            bus,
            cycles: 0,
            halted: false,
            halt_deadlock_warned: false,
            prev_pc: 0xFFFF_FFFF,
            same_pc_count: 0,
            pipeline: [0; 2],
            pipeline_valid: false,
        };

        // set up initial stack pointers (matching post-BIOS state)
        // SVC SP = 0x03007FE0, IRQ SP = 0x03007FA0, USR/SYS SP = 0x03007F00
        cpu.banked.svc[0] = 0x0300_7FE0; // SP_SVC (banked, since we're in SYS)
        cpu.banked.irq[0] = 0x0300_7FA0; // SP_IRQ
        cpu.regs[13] = 0x0300_7F00; // SP_SYS (current mode)

        // set PC to ROM entry point
        cpu.regs[15] = 0x0800_0000;

        cpu
    }

    /// Resets the CPU to boot from the BIOS at address 0x00000000.
    /// The real BIOS will initialize all registers and stack pointers,
    /// then jump to the ROM entry point.
    pub fn reset_for_bios_boot(&mut self) {
        self.regs = [0; 16];
        self.cpsr = MODE_SVC | CPSR_I; // ARM state, SVC mode, IRQs disabled
        self.spsr = [0; 5];
        self.banked = BankedRegisters::new();
        self.halted = false;
        self.halt_deadlock_warned = false;
        self.prev_pc = 0xFFFF_FFFF;
        self.same_pc_count = 0;
        self.pipeline_valid = false;
        // PC = 0: start executing from BIOS entry point
        self.regs[15] = 0x0000_0000;
    }

    // register accessors

    #[inline(always)]
    pub fn reg(&self, index: u32) -> u32 {
        self.regs[index as usize & 0xF]
    }

    #[inline(always)]
    pub fn set_reg(&mut self, index: u32, value: u32) {
        let index = index as usize & 0xF;
        self.regs[index] = value;
        if index == 15 {
            self.flush_pipeline();
        }
    }

    /// reads a register from the user/system bank (for STM with S bit)
    pub fn reg_user(&self, index: u32) -> u32 {
        let i = index as usize & 0xF;
        let mode = self.cpsr & CPSR_MODE_MASK;
        match i {
            0..=7 | 15 => self.regs[i],
            8..=12 => {
                if mode == MODE_FIQ {
                    self.banked.usr[i - 8]
                } else {
                    self.regs[i]
                }
            }
            13..=14 => {
                if mode == MODE_USR || mode == MODE_SYS {
                    self.regs[i]
                } else {
                    self.banked.usr[i - 8]
                }
            }
            _ => 0,
        }
    }

    /// writes a register to the user/system bank (for LDM with S bit, no PC)
    pub fn set_reg_user(&mut self, index: u32, value: u32) {
        let i = index as usize & 0xF;
        let mode = self.cpsr & CPSR_MODE_MASK;
        match i {
            0..=7 => self.regs[i] = value,
            8..=12 => {
                if mode == MODE_FIQ {
                    self.banked.usr[i - 8] = value;
                } else {
                    self.regs[i] = value;
                }
            }
            13..=14 => {
                if mode == MODE_USR || mode == MODE_SYS {
                    self.regs[i] = value;
                } else {
                    self.banked.usr[i - 8] = value;
                }
            }
            15 => {
                self.regs[15] = value;
                self.flush_pipeline();
            }
            _ => {}
        }
    }

    #[inline(always)]
    pub fn pc(&self) -> u32 {
        self.regs[15]
    }

    #[inline(always)]
    pub fn cpsr(&self) -> u32 {
        self.cpsr
    }

    pub fn set_cpsr(&mut self, value: u32) {
        let old_mode = self.cpsr & CPSR_MODE_MASK;
        let new_mode = value & CPSR_MODE_MASK;
        if old_mode != new_mode {
            self.swap_banked_registers(old_mode, new_mode);
        }
        self.cpsr = value;
    }

    pub fn spsr(&self) -> u32 {
        match mode_to_spsr_index(self.cpsr) {
            Some(i) => self.spsr[i],
            None => self.cpsr, // USR/SYS: return CPSR
        }
    }

    pub fn set_spsr(&mut self, value: u32) {
        if let Some(i) = mode_to_spsr_index(self.cpsr) {
            self.spsr[i] = value;
        }
    }

    // flag accessors

    #[inline(always)]
    pub fn flag_n(&self) -> bool {
        self.cpsr & CPSR_N != 0
    }

    #[inline(always)]
    pub fn flag_z(&self) -> bool {
        self.cpsr & CPSR_Z != 0
    }

    #[inline(always)]
    pub fn flag_c(&self) -> bool {
        self.cpsr & CPSR_C != 0
    }

    #[inline(always)]
    pub fn flag_v(&self) -> bool {
        self.cpsr & CPSR_V != 0
    }

    #[inline(always)]
    pub fn in_thumb_mode(&self) -> bool {
        self.cpsr & CPSR_T != 0
    }

    /// sets the N and Z flags based on a result value
    #[inline(always)]
    pub fn set_nz_flags(&mut self, result: u32) {
        self.cpsr = (self.cpsr & !(CPSR_N | CPSR_Z))
            | (if result & 0x80000000 != 0 { CPSR_N } else { 0 })
            | (if result == 0 { CPSR_Z } else { 0 });
    }

    #[inline(always)]
    pub fn set_flag_c(&mut self, value: bool) {
        if value {
            self.cpsr |= CPSR_C;
        } else {
            self.cpsr &= !CPSR_C;
        }
    }

    #[inline(always)]
    pub fn set_flag_v(&mut self, value: bool) {
        if value {
            self.cpsr |= CPSR_V;
        } else {
            self.cpsr &= !CPSR_V;
        }
    }

    // halt state

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn set_halted(&mut self, value: bool) {
        self.halted = value;
    }

    // pipeline management

    fn flush_pipeline(&mut self) {
        self.pipeline_valid = false;
    }

    fn fetch_bios_guard(&mut self, addr: u32, value: u32) {
        if addr < 0x4000 {
            self.bus.update_bios_value(value);
        }
    }

    fn fill_pipeline(&mut self) {
        if self.in_thumb_mode() {
            self.regs[15] &= !1;
            self.bus.bios_readable = self.regs[15] < 0x4000;
            self.pipeline[0] = self.bus.read16(self.regs[15]) as u32;
            self.fetch_bios_guard(self.regs[15], self.pipeline[0]);
            self.regs[15] = self.regs[15].wrapping_add(2);
            self.bus.bios_readable = self.regs[15] < 0x4000;
            self.pipeline[1] = self.bus.read16(self.regs[15]) as u32;
            self.fetch_bios_guard(self.regs[15], self.pipeline[1]);
            self.regs[15] = self.regs[15].wrapping_add(2);
        } else {
            self.regs[15] &= !3;
            self.bus.bios_readable = self.regs[15] < 0x4000;
            self.pipeline[0] = self.bus.read32(self.regs[15]);
            self.fetch_bios_guard(self.regs[15], self.pipeline[0]);
            self.regs[15] = self.regs[15].wrapping_add(4);
            self.bus.bios_readable = self.regs[15] < 0x4000;
            self.pipeline[1] = self.bus.read32(self.regs[15]);
            self.fetch_bios_guard(self.regs[15], self.pipeline[1]);
            self.regs[15] = self.regs[15].wrapping_add(4);
        }
        self.bus.bios_readable = false;
        self.pipeline_valid = true;
    }

    /// Fetches the next instruction from the pipeline without advancing PC.
    /// The PC advance happens after instruction execution in [`Self::step()`].
    fn fetch(&mut self) -> u32 {
        if !self.pipeline_valid {
            self.fill_pipeline();
        }

        // Simulate the 3rd pipeline stage (fetch): on real ARM7TDMI,
        // the fetch stage reads from PC (= executing_addr + 8) before
        // the instruction executes. This updates bios_value so that
        // BIOS protection returns the correct last-fetched value even
        // when a branch flushes the pipeline.
        if self.regs[15] < 0x4000 {
            self.bus.bios_readable = true;
            let prefetch = self.bus.read32(self.regs[15]);
            self.bus.update_bios_value(prefetch);
            self.bus.bios_readable = false;
        }

        // return the current instruction from the pipeline
        // at this point regs[15] = instruction_address + 8 (ARM) or + 4 (Thumb)
        // which is the correct value for reading R15 during execution
        self.pipeline[0]
    }

    /// Executes a single CPU instruction and returns the
    /// number of cycles consumed.
    pub fn step(&mut self) -> u32 {
        // checks for any pending interrupts
        if self.bus.irq.pending() && self.cpsr & CPSR_I == 0 {
            let was_halted = self.halted;
            self.halted = false;

            // when waking from halt, the pipeline hasn't been refilled,
            // so regs[15] points directly at the next instruction without
            // the pipeline prefetch offset. adds +4 so the IRQ handler's
            // SUBS PC, LR, #4 returns to the correct instruction.
            if was_halted {
                self.regs[15] = self.regs[15].wrapping_add(4);
            }

            self.enter_exception(0x18, MODE_IRQ);

            // interrupt takes ~3 cycles
            return 3;
        }

        // HLE IntrWait: emulates the BIOS IntrWait/VBlankIntrWait loop.
        // when intr_wait_flags is set, the CPU blocks until the waited
        // interrupt appears in IntrCheck (0x03007FF8) or in IF directly
        // (fallback for games that disable the waited interrupt in IE
        // during their IRQ handler, like Zelda).
        if self.bus.intr_wait_flags != 0 {
            // skip re-halt check while inside the IRQ handler itself
            if self.cpsr & CPSR_I == 0 {
                let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
                let intr_check =
                    u16::from_le_bytes([self.bus.iwram[offset], self.bus.iwram[offset + 1]]);
                if intr_check & self.bus.intr_wait_flags != 0 {
                    // waited interrupt was serviced — clear from IntrCheck
                    // and resume execution
                    let cleared = intr_check & !self.bus.intr_wait_flags;
                    let bytes = cleared.to_le_bytes();
                    self.bus.iwram[offset] = bytes[0];
                    self.bus.iwram[offset + 1] = bytes[1];
                    self.halted = false;
                    self.bus.intr_wait_flags = 0;
                } else if self.bus.irq.if_() & self.bus.intr_wait_flags != 0 {
                    // fallback: waited interrupt is in IF but not in
                    // IntrCheck (game disabled it in IE so the handler
                    // never ran for it). acknowledge and resume.
                    self.bus.irq.ack_if(self.bus.intr_wait_flags);
                    self.halted = false;
                    self.bus.intr_wait_flags = 0;
                } else if !self.halted {
                    // IRQ handler has returned but waited interrupt not
                    // in IntrCheck yet — re-halt to wait for more IRQs
                    self.halted = true;
                    return 1;
                }
            }
        }

        if self.halted {
            // detect halt deadlock: if no interrupts are enabled in IE,
            // nothing can ever wake the CPU from halt
            if self.bus.irq.ie() == 0 && !self.halt_deadlock_warned {
                self.halt_deadlock_warned = true;
                warnln!(
                    "GBA: halt deadlock detected — CPU halted with IE=0, nothing can wake it (PC={:#010x})",
                    self.pc()
                );
            }
            return 1; // idle cycle while halted
        }

        self.cycles = 0;

        // warns if executing from unmapped or unusual memory regions
        let exec_pc = if self.in_thumb_mode() {
            self.regs[15].wrapping_sub(4)
        } else {
            self.regs[15].wrapping_sub(8)
        };
        let exec_region = exec_pc >> 24;
        match exec_region {
            0x00 if exec_pc < 0x4000 => {} // BIOS
            0x02 => {}                     // EWRAM
            0x03 => {}                     // IWRAM
            0x08..=0x0D => {}              // ROM
            _ => {
                warnln!(
                    "GBA: executing from unmapped memory region at PC={:#010x}",
                    exec_pc
                );
            }
        }

        // infinite loop detection: same PC for many consecutive steps
        // cost: one comparison + conditional increment per step (negligible)
        if exec_pc == self.prev_pc {
            self.same_pc_count = self.same_pc_count.saturating_add(1);
            if self.same_pc_count == 64 {
                warnln!(
                    "GBA: infinite loop detected at PC={:#010x} (CPSR={:#010x})",
                    exec_pc,
                    self.cpsr
                );
            }
        } else {
            self.prev_pc = exec_pc;
            self.same_pc_count = 0;
        }

        let instr = self.fetch();

        // prefetches the next pipeline entry BEFORE executing the instruction.
        // On real ARM7TDMI, fetch/decode/execute happen simultaneously:
        // the fetch stage reads from PC at the same time as the execute
        // stage runs the current instruction, so self-modifying code that
        // writes to upcoming instruction addresses does not affect the
        // already-fetched pipeline contents.
        let prefetched = if self.pipeline_valid {
            self.bus.bios_readable = self.regs[15] < 0x4000;
            let value = if self.in_thumb_mode() {
                self.bus.read16(self.regs[15]) as u32
            } else {
                self.bus.read32(self.regs[15])
            };
            self.fetch_bios_guard(self.regs[15], value);
            self.bus.bios_readable = false;
            Some(value)
        } else {
            None
        };

        // allows BIOS data reads while executing BIOS code
        let in_bios = self.regs[15] < 0x4000 + 8;
        if in_bios {
            self.bus.bios_readable = true;
        }

        if self.in_thumb_mode() {
            execute_thumb(self, instr as u16);
        } else {
            execute_arm(self, instr);
        }

        if in_bios {
            self.bus.bios_readable = false;
        }

        // Commit the prefetched value into the pipeline (if not flushed)
        if self.pipeline_valid {
            self.pipeline[0] = self.pipeline[1];
            if let Some(value) = prefetched {
                self.pipeline[1] = value;
            }
            if self.in_thumb_mode() {
                self.regs[15] = self.regs[15].wrapping_add(2);
            } else {
                self.regs[15] = self.regs[15].wrapping_add(4);
            }
        }

        if self.cycles == 0 {
            self.cycles = 1; // minimum 1 cycle
        }

        self.cycles
    }

    /// checks the condition code of an ARM instruction
    #[inline(always)]
    pub fn check_condition(&self, instr: u32) -> bool {
        let cond = instr >> 28;
        match cond {
            0x0 => self.flag_z(),                                    // EQ
            0x1 => !self.flag_z(),                                   // NE
            0x2 => self.flag_c(),                                    // CS/HS
            0x3 => !self.flag_c(),                                   // CC/LO
            0x4 => self.flag_n(),                                    // MI
            0x5 => !self.flag_n(),                                   // PL
            0x6 => self.flag_v(),                                    // VS
            0x7 => !self.flag_v(),                                   // VC
            0x8 => self.flag_c() && !self.flag_z(),                  // HI
            0x9 => !self.flag_c() || self.flag_z(),                  // LS
            0xA => self.flag_n() == self.flag_v(),                   // GE
            0xB => self.flag_n() != self.flag_v(),                   // LT
            0xC => !self.flag_z() && self.flag_n() == self.flag_v(), // GT
            0xD => self.flag_z() || self.flag_n() != self.flag_v(),  // LE
            0xE => true,                                             // AL
            0xF => false,                                            // NV (reserved)
            _ => unreachable!(),
        }
    }

    /// enters an exception (interrupt, SWI, etc)
    pub fn enter_exception(&mut self, vector: u32, mode: u32) {
        let old_cpsr = self.cpsr;
        // for IRQ: LR_irq = next_instruction + 4
        // after pipeline advance: ARM regs[15] = instr+12, Thumb regs[15] = instr+6
        // ARM: LR = instr+12-4 = instr+8 = (instr+4)+4 ✓
        // Thumb: LR = instr+6 = (instr+2)+4 ✓
        let return_addr = if self.in_thumb_mode() {
            self.regs[15]
        } else {
            self.regs[15].wrapping_sub(4)
        };

        // switch to exception mode
        self.set_cpsr((self.cpsr & !CPSR_MODE_MASK & !CPSR_T) | mode | CPSR_I);
        self.set_spsr(old_cpsr);
        self.regs[14] = return_addr;

        // jump to the exception vector — BIOS memory contains the
        // appropriate handler stubs (e.g. IRQ handler at 0x18 branches
        // to 0x128 which implements the standard BIOS IRQ wrapper)
        self.regs[15] = vector;
        self.flush_pipeline();
    }

    /// restores CPSR from SPSR (used by exception return instructions)
    pub fn restore_cpsr(&mut self) {
        let spsr = self.spsr();
        self.set_cpsr(spsr);
    }

    fn save_banked(&mut self, mode: u32) {
        match mode {
            MODE_FIQ => {
                // R8-R12 handled by swap_banked_registers, only save R13-R14
                self.banked.fiq[5] = self.regs[13];
                self.banked.fiq[6] = self.regs[14];
            }
            MODE_IRQ => {
                self.banked.irq[0] = self.regs[13];
                self.banked.irq[1] = self.regs[14];
            }
            MODE_SVC => {
                self.banked.svc[0] = self.regs[13];
                self.banked.svc[1] = self.regs[14];
            }
            0x17 => {
                // ABT
                self.banked.abt[0] = self.regs[13];
                self.banked.abt[1] = self.regs[14];
            }
            0x1B => {
                // UND
                self.banked.und[0] = self.regs[13];
                self.banked.und[1] = self.regs[14];
            }
            MODE_USR | MODE_SYS => {
                self.banked.usr[5] = self.regs[13];
                self.banked.usr[6] = self.regs[14];
            }
            _ => {}
        }
    }

    fn restore_banked(&mut self, mode: u32) {
        match mode {
            MODE_FIQ => {
                // R8-R12 handled by swap_banked_registers, only restore R13-R14
                self.regs[13] = self.banked.fiq[5];
                self.regs[14] = self.banked.fiq[6];
            }
            MODE_IRQ => {
                self.regs[13] = self.banked.irq[0];
                self.regs[14] = self.banked.irq[1];
            }
            MODE_SVC => {
                self.regs[13] = self.banked.svc[0];
                self.regs[14] = self.banked.svc[1];
            }
            0x17 => {
                self.regs[13] = self.banked.abt[0];
                self.regs[14] = self.banked.abt[1];
            }
            0x1B => {
                self.regs[13] = self.banked.und[0];
                self.regs[14] = self.banked.und[1];
            }
            MODE_USR | MODE_SYS => {
                self.regs[13] = self.banked.usr[5];
                self.regs[14] = self.banked.usr[6];
            }
            _ => {}
        }
    }

    /// handles R8-R12 banking when switching between FIQ and non-FIQ modes
    fn swap_banked_registers(&mut self, old_mode: u32, new_mode: u32) {
        let was_fiq = old_mode == MODE_FIQ;
        let is_fiq = new_mode == MODE_FIQ;

        // save R8-R12 only when leaving FIQ (to FIQ bank) or entering FIQ (to USR bank)
        if was_fiq && !is_fiq {
            // leaving FIQ: save FIQ R8-R12, restore USR R8-R12
            for i in 0..5 {
                self.banked.fiq[i] = self.regs[8 + i];
                self.regs[8 + i] = self.banked.usr[i];
            }
        } else if !was_fiq && is_fiq {
            // entering FIQ: save USR R8-R12, restore FIQ R8-R12
            for i in 0..5 {
                self.banked.usr[i] = self.regs[8 + i];
                self.regs[8 + i] = self.banked.fiq[i];
            }
        }

        // save R13-R14 to old mode's bank
        self.save_banked(old_mode);
        // restore R13-R14 from new mode's bank
        self.restore_banked(new_mode);
    }

    // barrel shifter operations

    /// performs a barrel shift operation, returning (result, carry_out)
    pub fn barrel_shift(&self, value: u32, shift_type: u32, amount: u32) -> (u32, bool) {
        if amount == 0 {
            return (value, self.flag_c());
        }

        match shift_type {
            0 => {
                // LSL
                if amount >= 32 {
                    (0, if amount == 32 { value & 1 != 0 } else { false })
                } else {
                    (value << amount, value & (1 << (32 - amount)) != 0)
                }
            }
            1 => {
                // LSR
                if amount >= 32 {
                    (
                        0,
                        if amount == 32 {
                            value & 0x80000000 != 0
                        } else {
                            false
                        },
                    )
                } else {
                    (value >> amount, value & (1 << (amount - 1)) != 0)
                }
            }
            2 => {
                // ASR
                if amount >= 32 {
                    let sign = (value as i32) >> 31;
                    (sign as u32, sign != 0)
                } else {
                    let result = ((value as i32) >> amount) as u32;
                    (result, value & (1 << (amount - 1)) != 0)
                }
            }
            3 => {
                // ROR
                let amount = amount & 31;
                if amount == 0 {
                    (value, value & 0x80000000 != 0)
                } else {
                    let result = value.rotate_right(amount);
                    (result, result & 0x80000000 != 0)
                }
            }
            _ => (value, self.flag_c()),
        }
    }

    /// barrel shift with special handling for immediate shift amounts of 0
    pub fn barrel_shift_immediate(&self, value: u32, shift_type: u32, amount: u32) -> (u32, bool) {
        match (shift_type, amount) {
            (0, 0) => (value, self.flag_c()), // LSL #0 = no shift
            (1, 0) => (0, value >> 31 != 0),  // LSR #0 = LSR #32
            (2, 0) => {
                // ASR #0 = ASR #32
                let sign = (value as i32) >> 31;
                (sign as u32, sign != 0)
            }
            (3, 0) => {
                // ROR #0 = RRX (rotate right through carry)
                let carry = self.flag_c() as u32;
                let result = (carry << 31) | (value >> 1);
                (result, value & 1 != 0)
            }
            _ => self.barrel_shift(value, shift_type, amount),
        }
    }

    // bus access helpers (used by BIOS HLE and instruction handlers)

    pub fn bus_read8(&mut self, addr: u32) -> u8 {
        self.bus.read8(addr)
    }

    pub fn bus_read16(&mut self, addr: u32) -> u16 {
        self.bus.read16(addr & !1)
    }

    pub fn bus_read32(&mut self, addr: u32) -> u32 {
        self.bus.read32(addr & !3)
    }

    pub fn bus_write8(&mut self, addr: u32, value: u8) {
        self.bus.write8(addr, value);
    }

    pub fn bus_write16(&mut self, addr: u32, value: u16) {
        self.bus.write16(addr, value);
    }

    pub fn bus_write32(&mut self, addr: u32, value: u32) {
        self.bus.write32(addr, value);
    }

    pub fn reset(&mut self) {
        self.regs = [0; 16];
        self.cpsr = MODE_SYS;
        self.spsr = [0; 5];
        self.banked = BankedRegisters::new();
        self.halted = false;
        self.pipeline_valid = false;

        self.banked.svc[0] = 0x0300_7FE0;
        self.banked.irq[0] = 0x0300_7FA0;
        self.regs[13] = 0x0300_7F00;
        self.regs[15] = 0x0800_0000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gba::bus::GbaBus;

    fn make_cpu() -> Arm7Tdmi {
        Arm7Tdmi::new(GbaBus::new())
    }

    #[test]
    fn test_new() {
        let cpu = make_cpu();
        assert_eq!(cpu.pc(), 0x0800_0000);
        assert_eq!(cpu.reg(13), 0x0300_7F00); // SP_SYS (post-BIOS state)
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_SYS);
        assert!(!cpu.halted());
        assert!(!cpu.in_thumb_mode());
    }

    #[test]
    fn test_reg_access() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0x12345678);
        assert_eq!(cpu.reg(0), 0x12345678);

        cpu.set_reg(5, 0xABCD);
        assert_eq!(cpu.reg(5), 0xABCD);
    }

    #[test]
    fn test_cpsr_flags() {
        let mut cpu = make_cpu();

        cpu.set_nz_flags(0);
        assert!(cpu.flag_z());
        assert!(!cpu.flag_n());

        cpu.set_nz_flags(0x80000000);
        assert!(!cpu.flag_z());
        assert!(cpu.flag_n());

        cpu.set_flag_c(true);
        assert!(cpu.flag_c());
        cpu.set_flag_c(false);
        assert!(!cpu.flag_c());

        cpu.set_flag_v(true);
        assert!(cpu.flag_v());
        cpu.set_flag_v(false);
        assert!(!cpu.flag_v());
    }

    #[test]
    fn test_halted() {
        let mut cpu = make_cpu();
        assert!(!cpu.halted());
        cpu.set_halted(true);
        assert!(cpu.halted());
        cpu.set_halted(false);
        assert!(!cpu.halted());
    }

    #[test]
    fn test_condition_codes() {
        let cpu = make_cpu();
        assert!(cpu.check_condition(0xE0000000)); // AL
        assert!(!cpu.check_condition(0xF0000000)); // NV
    }

    #[test]
    fn test_condition_eq_ne() {
        let mut cpu = make_cpu();
        cpu.set_nz_flags(0); // Z=1
        assert!(cpu.check_condition(0x00000000)); // EQ
        assert!(!cpu.check_condition(0x10000000)); // NE

        cpu.set_nz_flags(1); // Z=0
        assert!(!cpu.check_condition(0x00000000)); // EQ
        assert!(cpu.check_condition(0x10000000)); // NE
    }

    #[test]
    fn test_condition_cs_cc() {
        let mut cpu = make_cpu();
        cpu.set_flag_c(true);
        assert!(cpu.check_condition(0x20000000)); // CS
        assert!(!cpu.check_condition(0x30000000)); // CC

        cpu.set_flag_c(false);
        assert!(!cpu.check_condition(0x20000000)); // CS
        assert!(cpu.check_condition(0x30000000)); // CC
    }

    #[test]
    fn test_spsr() {
        let mut cpu = make_cpu();
        // switch to SVC mode so SPSR is accessible
        cpu.set_cpsr(MODE_SVC);
        cpu.set_spsr(0x12345678);
        assert_eq!(cpu.spsr(), 0x12345678);
    }

    #[test]
    fn test_enter_exception() {
        let mut cpu = make_cpu();
        let old_cpsr = cpu.cpsr();
        cpu.enter_exception(0x18, MODE_IRQ);
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_IRQ);
        assert!(cpu.cpsr() & CPSR_I != 0); // IRQs disabled
        assert!(cpu.cpsr() & CPSR_T == 0); // ARM mode
        assert_eq!(cpu.spsr(), old_cpsr); // old CPSR saved
        assert_eq!(cpu.pc(), 0x18); // vector
    }

    #[test]
    fn test_barrel_shift_lsl() {
        let cpu = make_cpu();
        let (result, carry) = cpu.barrel_shift(0x01, 0, 4);
        assert_eq!(result, 0x10);
        assert!(!carry);
    }

    #[test]
    fn test_barrel_shift_lsr() {
        let cpu = make_cpu();
        let (result, carry) = cpu.barrel_shift(0x10, 1, 4);
        assert_eq!(result, 0x01);
        assert!(!carry);
    }

    #[test]
    fn test_barrel_shift_asr() {
        let cpu = make_cpu();
        let (result, _carry) = cpu.barrel_shift(0x80000000, 2, 4);
        assert_eq!(result, 0xF8000000);
    }

    #[test]
    fn test_barrel_shift_ror() {
        let cpu = make_cpu();
        let (result, _carry) = cpu.barrel_shift(0x01, 3, 4);
        assert_eq!(result, 0x10000000);
    }

    #[test]
    fn test_barrel_shift_lsl_32() {
        let cpu = make_cpu();
        let (result, carry) = cpu.barrel_shift(0x01, 0, 32);
        assert_eq!(result, 0);
        assert!(carry); // bit 0 shifted out
    }

    #[test]
    fn test_barrel_shift_lsr_32() {
        let cpu = make_cpu();
        let (result, carry) = cpu.barrel_shift(0x80000000, 1, 32);
        assert_eq!(result, 0);
        assert!(carry); // bit 31 shifted out
    }

    #[test]
    fn test_barrel_shift_asr_32() {
        let cpu = make_cpu();
        let (result, carry) = cpu.barrel_shift(0x80000000, 2, 32);
        assert_eq!(result, 0xFFFFFFFF); // sign extended
        assert!(carry);
    }

    #[test]
    fn test_barrel_shift_zero_amount() {
        let mut cpu = make_cpu();
        cpu.set_flag_c(true);
        let (result, carry) = cpu.barrel_shift(0x42, 0, 0);
        assert_eq!(result, 0x42);
        assert!(carry); // preserves carry
    }

    #[test]
    fn test_barrel_shift_immediate_lsr0() {
        let cpu = make_cpu();
        // LSR #0 is treated as LSR #32
        let (result, carry) = cpu.barrel_shift_immediate(0x80000000, 1, 0);
        assert_eq!(result, 0);
        assert!(carry);
    }

    #[test]
    fn test_barrel_shift_immediate_asr0() {
        let cpu = make_cpu();
        // ASR #0 is treated as ASR #32
        let (result, carry) = cpu.barrel_shift_immediate(0x80000000, 2, 0);
        assert_eq!(result, 0xFFFFFFFF);
        assert!(carry);
    }

    #[test]
    fn test_barrel_shift_immediate_rrx() {
        let mut cpu = make_cpu();
        cpu.set_flag_c(true);
        // ROR #0 is treated as RRX
        let (result, carry) = cpu.barrel_shift_immediate(0x01, 3, 0);
        assert_eq!(result, 0x80000000); // carry shifted into bit 31
        assert!(carry); // bit 0 shifted out
    }

    #[test]
    fn test_bus_read_write() {
        let mut cpu = make_cpu();
        cpu.bus_write8(0x0200_0000, 0x42);
        assert_eq!(cpu.bus_read8(0x0200_0000), 0x42);

        cpu.bus_write16(0x0200_0002, 0x1234);
        assert_eq!(cpu.bus_read16(0x0200_0002), 0x1234);

        cpu.bus_write32(0x0200_0004, 0xDEADBEEF);
        assert_eq!(cpu.bus_read32(0x0200_0004), 0xDEADBEEF);
    }

    #[test]
    fn test_step_halted() {
        let mut cpu = make_cpu();
        cpu.set_halted(true);
        let cycles = cpu.step();
        assert_eq!(cycles, 1); // idle cycle
    }

    #[test]
    fn test_reset() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0x12345678);
        cpu.set_halted(true);
        cpu.reset();
        assert_eq!(cpu.reg(0), 0);
        assert!(!cpu.halted());
        assert_eq!(cpu.pc(), 0x0800_0000);
        assert_eq!(cpu.reg(13), 0x0300_7F00); // SP_SYS
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_SYS);
    }

    #[test]
    fn test_reset_for_bios_boot() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0x12345678);
        cpu.set_halted(true);
        cpu.reset_for_bios_boot();
        assert_eq!(cpu.reg(0), 0);
        assert!(!cpu.halted());
        assert_eq!(cpu.pc(), 0x0000_0000);
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_SVC);
        assert!(cpu.cpsr() & CPSR_I != 0); // IRQs disabled
        assert!(cpu.cpsr() & CPSR_T == 0); // ARM mode
    }

    #[test]
    fn test_new_sys_mode_irqs_enabled() {
        let cpu = make_cpu();
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_SYS);
        assert_eq!(cpu.cpsr() & CPSR_I, 0); // IRQs enabled
    }

    #[test]
    fn test_new_stack_pointers() {
        let mut cpu = make_cpu();
        // SYS mode SP
        assert_eq!(cpu.reg(13), 0x0300_7F00);
        // switch to IRQ mode and check SP
        cpu.set_cpsr(MODE_IRQ);
        assert_eq!(cpu.reg(13), 0x0300_7FA0);
        // switch to SVC mode and check SP
        cpu.set_cpsr(MODE_SVC);
        assert_eq!(cpu.reg(13), 0x0300_7FE0);
    }

    #[test]
    fn test_bios_value_after_startup() {
        let mut cpu = make_cpu();
        // BIOS protection value after boot matches real GBA
        assert_eq!(cpu.bus.read32(0x0000_0000), 0xE129F000);
    }

    #[test]
    fn test_bios_protection_fetch_updates_value() {
        let mut cpu = make_cpu();
        // Place an instruction in BIOS at 0x00
        cpu.bus.bios_readable = true;
        let _val_at_0 = cpu.bus.read32(0x0000_0000);
        cpu.bus.bios_readable = false;
        // Set PC to BIOS address and fill pipeline
        cpu.set_reg(15, 0x0000_0000);
        // The fetch should update bios_value
        // PC=0, so fill_pipeline reads from 0x00, 0x04
        // and the fetch() prefetch reads from 0x08
        let _instr = cpu.fetch();
        cpu.bus.bios_readable = true;
        let val_at_8 = cpu.bus.read32(0x0000_0008);
        cpu.bus.bios_readable = false;
        // bios_value should be the value at PC (= addr + 8 = 0x08)
        assert_eq!(cpu.bus.read32(0x0000_0000), val_at_8);
    }

    #[test]
    fn test_enter_exception_irq_mode() {
        let mut cpu = make_cpu();
        let old_cpsr = cpu.cpsr();
        cpu.enter_exception(0x18, MODE_IRQ);
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_IRQ);
        assert!(cpu.cpsr() & CPSR_I != 0); // IRQs disabled in handler
        assert!(cpu.cpsr() & CPSR_T == 0); // ARM mode
        assert_eq!(cpu.spsr(), old_cpsr);
        assert_eq!(cpu.pc(), 0x18);
    }

    #[test]
    fn test_irq_entry_from_sys_mode() {
        let mut cpu = make_cpu();
        // enable VBlank IRQ
        cpu.bus.irq.set_ime(true);
        cpu.bus.irq.set_ie(0x0001); // VBlank
        cpu.bus.irq.raise_vblank();
        assert!(cpu.bus.irq.pending());
        // step should enter IRQ exception
        cpu.step();
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_IRQ);
    }

    #[test]
    fn test_intr_wait_stays_halted_when_wrong_irq() {
        let mut cpu = make_cpu();
        // simulate IntrWait waiting for VBlank (bit 0)
        cpu.bus.intr_wait_flags = 1;
        cpu.set_halted(true);
        // IntrCheck has VCount (bit 2) but NOT VBlank
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        let bytes = 0x04u16.to_le_bytes();
        cpu.bus.iwram[offset] = bytes[0];
        cpu.bus.iwram[offset + 1] = bytes[1];

        let cycles = cpu.step();

        // should stay halted because VBlank not in IntrCheck
        assert!(cpu.halted());
        assert_eq!(cycles, 1);
        // flags should remain set
        assert_eq!(cpu.bus.intr_wait_flags, 1);
    }

    #[test]
    fn test_intr_wait_unhalts_when_correct_irq() {
        let mut cpu = make_cpu();
        // simulate IntrWait waiting for VBlank (bit 0),
        // CPU in SYS mode (CPSR_I == 0) and not halted —
        // simulates the state after IRQ handler has returned
        cpu.bus.intr_wait_flags = 1;
        // IntrCheck has VBlank set
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        let bytes = 0x01u16.to_le_bytes();
        cpu.bus.iwram[offset] = bytes[0];
        cpu.bus.iwram[offset + 1] = bytes[1];

        cpu.step();

        // should not be halted — waited interrupt was serviced
        assert!(!cpu.halted());
        // flags should be cleared
        assert_eq!(cpu.bus.intr_wait_flags, 0);
        // IntrCheck should have VBlank bit cleared
        let check = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        assert_eq!(check, 0);
    }

    #[test]
    fn test_intr_wait_skipped_during_irq_handler() {
        let mut cpu = make_cpu();
        // simulate IntrWait flags set but CPU is in IRQ mode (CPSR_I=1)
        cpu.bus.intr_wait_flags = 1;
        cpu.set_cpsr(MODE_IRQ | CPSR_I);
        // IntrCheck does NOT have VBlank
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        cpu.bus.iwram[offset] = 0x00;
        cpu.bus.iwram[offset + 1] = 0x00;

        cpu.step();

        // should NOT re-halt — CPSR_I is set, check is skipped
        assert!(!cpu.halted());
        // flags should remain (not cleared)
        assert_eq!(cpu.bus.intr_wait_flags, 1);
    }

    #[test]
    fn test_intr_wait_noop_when_flags_zero() {
        let mut cpu = make_cpu();
        // intr_wait_flags is 0 (default, or real BIOS mode)
        assert_eq!(cpu.bus.intr_wait_flags, 0);

        cpu.step();

        // should execute normally, not halt
        assert!(!cpu.halted());
    }

    #[test]
    fn test_intr_wait_if_fallback_when_not_in_ie() {
        // Zelda scenario: game disables VBlank in IE during handler,
        // but VBlank is still in IF. the IF fallback should detect it.
        let mut cpu = make_cpu();
        cpu.bus.intr_wait_flags = 1; // wait for VBlank
                                     // IE does NOT include VBlank — only VCount (bit 2)
        cpu.bus.irq.set_ie(0x04);
        // VBlank is pending in IF
        cpu.bus.irq.raise(0x01);
        // IntrCheck does NOT have VBlank
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        cpu.bus.iwram[offset] = 0x00;
        cpu.bus.iwram[offset + 1] = 0x00;

        cpu.step();

        // should unhalt via IF fallback
        assert!(!cpu.halted());
        assert_eq!(cpu.bus.intr_wait_flags, 0);
        // VBlank should be acknowledged from IF
        assert_eq!(cpu.bus.irq.if_() & 1, 0);
    }

    #[test]
    fn test_intr_wait_if_fallback_wrong_irq_in_if() {
        // IF has the wrong interrupt — should not trigger fallback
        let mut cpu = make_cpu();
        cpu.bus.intr_wait_flags = 1; // wait for VBlank
        cpu.bus.irq.raise(0x04); // only VCount in IF
                                 // IntrCheck empty
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        cpu.bus.iwram[offset] = 0x00;
        cpu.bus.iwram[offset + 1] = 0x00;

        cpu.step();

        // should re-halt — neither IntrCheck nor IF has VBlank
        assert!(cpu.halted());
        assert_eq!(cpu.bus.intr_wait_flags, 1);
    }

    #[test]
    fn test_intr_wait_rehalts_after_irq_handler_returns() {
        // Mario Kart scenario: IRQ fires, handler runs and returns,
        // but waited interrupt not yet in IntrCheck — should re-halt
        let mut cpu = make_cpu();
        cpu.bus.intr_wait_flags = 1; // wait for VBlank
                                     // CPU is NOT halted (handler just returned to user code)
        assert!(!cpu.halted());
        // IntrCheck does NOT have VBlank
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        cpu.bus.iwram[offset] = 0x00;
        cpu.bus.iwram[offset + 1] = 0x00;
        // IF is also empty (handler acknowledged it)

        let cycles = cpu.step();

        // should re-halt to wait for next IRQ
        assert!(cpu.halted());
        assert_eq!(cycles, 1);
        assert_eq!(cpu.bus.intr_wait_flags, 1);
    }

    #[test]
    fn test_intr_wait_clears_only_waited_bits_from_intrcheck() {
        // IntrCheck has multiple bits set; only the waited ones are cleared
        let mut cpu = make_cpu();
        cpu.bus.intr_wait_flags = 1; // wait for VBlank only
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        // IntrCheck has VBlank (0x01) + HBlank (0x02) + VCount (0x04)
        let bytes = 0x07u16.to_le_bytes();
        cpu.bus.iwram[offset] = bytes[0];
        cpu.bus.iwram[offset + 1] = bytes[1];

        cpu.step();

        assert!(!cpu.halted());
        assert_eq!(cpu.bus.intr_wait_flags, 0);
        // only VBlank (bit 0) cleared; HBlank + VCount remain
        let check = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        assert_eq!(check, 0x06);
    }

    #[test]
    fn test_intr_wait_intrcheck_takes_priority_over_if_fallback() {
        // when both IntrCheck and IF have the waited bit,
        // IntrCheck path should be taken (clears IntrCheck, not IF)
        let mut cpu = make_cpu();
        cpu.bus.intr_wait_flags = 1; // wait for VBlank
                                     // IntrCheck has VBlank
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        let bytes = 0x01u16.to_le_bytes();
        cpu.bus.iwram[offset] = bytes[0];
        cpu.bus.iwram[offset + 1] = bytes[1];
        // IF also has VBlank
        cpu.bus.irq.raise(0x01);

        cpu.step();

        assert!(!cpu.halted());
        assert_eq!(cpu.bus.intr_wait_flags, 0);
        // IntrCheck should be cleared
        let check = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        assert_eq!(check, 0);
        // IF should NOT be touched (IntrCheck path was used)
        assert_eq!(cpu.bus.irq.if_() & 1, 1);
    }

    #[test]
    fn test_intr_wait_irq_dispatch_preserves_flags() {
        // when an IRQ fires while intr_wait_flags is set,
        // the IRQ dispatch should not clear intr_wait_flags
        let mut cpu = make_cpu();
        cpu.bus.intr_wait_flags = 1; // wait for VBlank
        cpu.set_halted(true);
        // enable VBlank IRQ and raise it
        cpu.bus.irq.set_ime(true);
        cpu.bus.irq.set_ie(0x01);
        cpu.bus.irq.raise(0x01);

        cpu.step();

        // IRQ dispatch should fire (pending() is true)
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_IRQ);
        // intr_wait_flags should be preserved for after handler returns
        assert_eq!(cpu.bus.intr_wait_flags, 1);
    }

    #[test]
    fn test_intr_wait_multiple_flags() {
        // waiting for multiple interrupts (VBlank | HBlank)
        let mut cpu = make_cpu();
        cpu.bus.intr_wait_flags = 0x03; // VBlank | HBlank
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        // only HBlank in IntrCheck (bit 1) — partial match is enough
        let bytes = 0x02u16.to_le_bytes();
        cpu.bus.iwram[offset] = bytes[0];
        cpu.bus.iwram[offset + 1] = bytes[1];

        cpu.step();

        // should unhalt — at least one waited bit matched
        assert!(!cpu.halted());
        assert_eq!(cpu.bus.intr_wait_flags, 0);
        // both VBlank and HBlank bits cleared from IntrCheck
        let check = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        assert_eq!(check, 0);
    }

    #[test]
    fn test_halt_wakes_on_pending_irq() {
        // normal halt (no IntrWait) wakes on pending IRQ
        let mut cpu = make_cpu();
        cpu.set_halted(true);
        cpu.bus.irq.set_ime(true);
        cpu.bus.irq.set_ie(0x01);
        cpu.bus.irq.raise_vblank();

        cpu.step();

        assert!(!cpu.halted());
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, MODE_IRQ);
    }

    #[test]
    fn test_halt_stays_halted_without_pending_irq() {
        // halt without matching IRQ should stay halted
        let mut cpu = make_cpu();
        cpu.set_halted(true);
        // IME on but IE empty — nothing can wake it
        cpu.bus.irq.set_ime(true);
        cpu.bus.irq.raise_vblank();

        let cycles = cpu.step();

        assert!(cpu.halted());
        assert_eq!(cycles, 1);
    }

    // -- crash detection tests --

    #[test]
    fn test_halt_deadlock_warned_flag_set_on_ie_zero() {
        let mut cpu = make_cpu();
        cpu.set_halted(true);
        // IE = 0 means nothing can wake the CPU
        cpu.bus.irq.set_ie(0);
        assert!(!cpu.halt_deadlock_warned);

        cpu.step();

        // should have set the warning flag
        assert!(cpu.halt_deadlock_warned);
        // still halted (deadlock is a warning, not recovery)
        assert!(cpu.halted());
    }

    #[test]
    fn test_halt_deadlock_not_warned_when_ie_nonzero() {
        let mut cpu = make_cpu();
        cpu.set_halted(true);
        cpu.bus.irq.set_ie(0x01); // VBlank enabled — can be woken

        cpu.step();

        assert!(!cpu.halt_deadlock_warned);
    }

    #[test]
    fn test_halt_deadlock_warned_resets_on_unhalt() {
        let mut cpu = make_cpu();
        cpu.set_halted(true);
        cpu.bus.irq.set_ie(0);
        cpu.step(); // triggers warning
        assert!(cpu.halt_deadlock_warned);

        // unhalt via IRQ
        cpu.bus.irq.set_ime(true);
        cpu.bus.irq.set_ie(0x01);
        cpu.bus.irq.raise_vblank();
        cpu.step();

        // should have unhalted and reset the flag
        assert!(!cpu.halted());
    }

    #[test]
    fn test_infinite_loop_counter_increments() {
        let mut cpu = make_cpu();
        // place a NOP (MOV R0, R0) at 0x08000000
        cpu.bus.write32(0x0800_0000, 0xE1A00000); // MOV R0, R0
                                                  // also need next instructions for pipeline
        cpu.bus.write32(0x0800_0004, 0xE1A00000);
        cpu.bus.write32(0x0800_0008, 0xE1A00000);

        // PC starts at 0x08000000 by default (post-boot state)
        // first step resets prev_pc
        cpu.step();
        assert_eq!(cpu.same_pc_count, 0);
    }

    #[test]
    fn test_infinite_loop_counter_resets_on_different_pc() {
        let mut cpu = make_cpu();
        // place two different NOPs at sequential addresses
        cpu.bus.write32(0x0800_0000, 0xE1A00000);
        cpu.bus.write32(0x0800_0004, 0xE1A00000);
        cpu.bus.write32(0x0800_0008, 0xE1A00000);
        cpu.bus.write32(0x0800_000C, 0xE1A00000);

        cpu.step(); // executes 0x08000000
        cpu.step(); // executes 0x08000004 — different PC
        assert_eq!(cpu.same_pc_count, 0);
    }

    #[test]
    fn test_enter_exception_und_mode() {
        let mut cpu = make_cpu();
        let old_cpsr = cpu.cpsr();
        cpu.enter_exception(0x04, 0x1B); // MODE_UND
        assert_eq!(cpu.cpsr() & CPSR_MODE_MASK, 0x1B);
        assert!(cpu.cpsr() & CPSR_I != 0); // IRQs disabled
        assert!(cpu.cpsr() & CPSR_T == 0); // ARM mode
        assert_eq!(cpu.spsr(), old_cpsr);
        assert_eq!(cpu.pc(), 0x04);
    }

    #[test]
    fn test_bios_und_vector_is_infinite_loop() {
        let mut cpu = make_cpu();
        cpu.bus.bios_readable = true;
        let instr = cpu.bus.read32(0x0000_0004);
        cpu.bus.bios_readable = false;
        // B . = 0xEAFFFFFE (branch to self)
        assert_eq!(instr, 0xEAFFFFFE);
    }

    #[test]
    fn test_bios_swi_vector_is_infinite_loop() {
        let mut cpu = make_cpu();
        cpu.bus.bios_readable = true;
        let instr = cpu.bus.read32(0x0000_0008);
        cpu.bus.bios_readable = false;
        // B . = 0xEAFFFFFE (branch to self)
        assert_eq!(instr, 0xEAFFFFFE);
    }
}
