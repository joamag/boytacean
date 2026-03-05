//! ARM7TDMI CPU emulation for the Game Boy Advance.
//!
//! Implements the ARM and Thumb instruction sets with a
//! 3-stage pipeline, banked registers for privileged modes,
//! and interrupt handling.

use crate::gba::{
    arm::execute_arm,
    bus::GbaBus,
    consts::{
        CPSR_C, CPSR_F, CPSR_I, CPSR_MODE_MASK, CPSR_N, CPSR_T, CPSR_V, CPSR_Z, MODE_FIQ, MODE_IRQ,
        MODE_SVC, MODE_SYS, MODE_USR,
    },
    thumb::execute_thumb,
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

    /// pipeline state for prefetched instruction
    pipeline: [u32; 2],
    pipeline_valid: bool,
}

impl Arm7Tdmi {
    pub fn new(bus: GbaBus) -> Self {
        let mut cpu = Self {
            regs: [0; 16],
            cpsr: MODE_SVC | CPSR_I | CPSR_F, // start in SVC mode, IRQs disabled
            spsr: [0; 5],
            banked: BankedRegisters::new(),
            bus,
            cycles: 0,
            halted: false,
            pipeline: [0; 2],
            pipeline_valid: false,
        };

        // set up initial stack pointers
        cpu.regs[13] = 0x0300_7FE0; // SP_SVC
        cpu.banked.irq[0] = 0x0300_7FA0; // SP_IRQ
        cpu.banked.usr[5] = 0x0300_7F00; // SP_USR/SYS

        // set PC to ROM entry point
        cpu.regs[15] = 0x0800_0000;

        cpu
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

    fn fill_pipeline(&mut self) {
        if self.in_thumb_mode() {
            self.regs[15] &= !1; // align to halfword
            self.pipeline[0] = self.bus.read16(self.regs[15]) as u32;
            self.regs[15] = self.regs[15].wrapping_add(2);
            self.pipeline[1] = self.bus.read16(self.regs[15]) as u32;
            self.regs[15] = self.regs[15].wrapping_add(2);
        } else {
            self.regs[15] &= !3; // align to word
            self.pipeline[0] = self.bus.read32(self.regs[15]);
            self.regs[15] = self.regs[15].wrapping_add(4);
            self.pipeline[1] = self.bus.read32(self.regs[15]);
            self.regs[15] = self.regs[15].wrapping_add(4);
        }
        self.pipeline_valid = true;
    }

    /// fetches the next instruction from the pipeline without advancing PC.
    /// The PC advance happens after instruction execution in `step()`.
    fn fetch(&mut self) -> u32 {
        if !self.pipeline_valid {
            self.fill_pipeline();
        }

        // return the current instruction from the pipeline
        // at this point regs[15] = instruction_address + 8 (ARM) or + 4 (Thumb)
        // which is the correct value for reading R15 during execution
        self.pipeline[0]
    }

    /// advances the pipeline after instruction execution
    fn advance_pipeline(&mut self) {
        self.pipeline[0] = self.pipeline[1];

        if self.in_thumb_mode() {
            self.pipeline[1] = self.bus.read16(self.regs[15]) as u32;
            self.regs[15] = self.regs[15].wrapping_add(2);
        } else {
            self.pipeline[1] = self.bus.read32(self.regs[15]);
            self.regs[15] = self.regs[15].wrapping_add(4);
        }
    }

    /// executes a single CPU instruction and returns the number of cycles consumed
    pub fn step(&mut self) -> u32 {
        // check for pending interrupts
        if self.bus.irq.pending() && self.cpsr & CPSR_I == 0 {
            self.halted = false;
            self.enter_exception(0x18, MODE_IRQ);
            return 3; // interrupt takes ~3 cycles
        }

        if self.halted {
            return 1; // idle cycle while halted
        }

        self.cycles = 0;
        let instr = self.fetch();

        if self.in_thumb_mode() {
            execute_thumb(self, instr as u16);
        } else {
            execute_arm(self, instr);
        }

        // advance the pipeline only if it wasn't flushed (branch/exception)
        if self.pipeline_valid {
            self.advance_pipeline();
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
        let return_addr = if self.in_thumb_mode() {
            self.regs[15].wrapping_sub(2)
        } else {
            self.regs[15].wrapping_sub(4)
        };

        // switch to exception mode
        self.set_cpsr((self.cpsr & !CPSR_MODE_MASK & !CPSR_T) | mode | CPSR_I);
        self.set_spsr(old_cpsr);
        self.regs[14] = return_addr;

        // for IRQ, use HLE dispatch: read handler address from
        // 0x03FFFFFC (mirrored to 0x03007FFC in IWRAM) instead
        // of jumping to the BIOS vector at 0x18
        if vector == 0x18 {
            let handler = self.bus.read32(0x03FF_FFFC);
            if handler != 0 {
                self.regs[15] = handler;
            } else {
                self.regs[15] = vector;
            }
        } else {
            self.regs[15] = vector;
        }
        self.flush_pipeline();
    }

    /// restores CPSR from SPSR (used by exception return instructions)
    pub fn restore_cpsr(&mut self) {
        let spsr = self.spsr();
        self.set_cpsr(spsr);
    }

    /// swaps banked registers when changing CPU mode
    fn swap_banked_registers(&mut self, old_mode: u32, new_mode: u32) {
        // save current registers to old mode's bank
        self.save_banked(old_mode);
        // restore new mode's banked registers
        self.restore_banked(new_mode);
    }

    fn save_banked(&mut self, mode: u32) {
        match mode {
            MODE_FIQ => {
                for i in 0..7 {
                    self.banked.fiq[i] = self.regs[8 + i];
                }
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
                for i in 0..5 {
                    self.banked.usr[i] = self.regs[8 + i];
                }
                self.banked.usr[5] = self.regs[13];
                self.banked.usr[6] = self.regs[14];
            }
            _ => {}
        }
    }

    fn restore_banked(&mut self, mode: u32) {
        match mode {
            MODE_FIQ => {
                // save USR R8-R12 first
                for i in 0..5 {
                    self.banked.usr[i] = self.regs[8 + i];
                }
                self.banked.usr[5] = self.regs[13];
                self.banked.usr[6] = self.regs[14];
                for i in 0..7 {
                    self.regs[8 + i] = self.banked.fiq[i];
                }
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
                for i in 0..5 {
                    self.regs[8 + i] = self.banked.usr[i];
                }
                self.regs[13] = self.banked.usr[5];
                self.regs[14] = self.banked.usr[6];
            }
            _ => {}
        }
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

    pub fn bus_read8(&self, addr: u32) -> u8 {
        self.bus.read8(addr)
    }

    pub fn bus_read16(&self, addr: u32) -> u16 {
        self.bus.read16(addr & !1)
    }

    pub fn bus_read32(&self, addr: u32) -> u32 {
        self.bus.read32(addr & !3)
    }

    pub fn bus_write8(&mut self, addr: u32, value: u8) {
        self.bus.write8(addr, value);
    }

    pub fn bus_write16(&mut self, addr: u32, value: u16) {
        self.bus.write16(addr & !1, value);
    }

    pub fn bus_write32(&mut self, addr: u32, value: u32) {
        self.bus.write32(addr & !3, value);
    }

    pub fn reset(&mut self) {
        self.regs = [0; 16];
        self.cpsr = MODE_SVC | CPSR_I | CPSR_F;
        self.spsr = [0; 5];
        self.banked = BankedRegisters::new();
        self.halted = false;
        self.pipeline_valid = false;

        self.regs[13] = 0x0300_7FE0;
        self.banked.irq[0] = 0x0300_7FA0;
        self.banked.usr[5] = 0x0300_7F00;
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
        assert_eq!(cpu.reg(13), 0x0300_7FE0); // SP_SVC
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
        // in SVC mode, SPSR should be accessible
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
        assert_eq!(cpu.reg(13), 0x0300_7FE0);
    }
}
