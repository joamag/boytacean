//! Implementation of the core CPU ([Sharp LR35902](https://en.wikipedia.org/wiki/Game_Boy)) logic for the Game Boy.
//!
//! Does not include the instruction set implementation, only the core
//! CPU logic and the CPU struct definition.
//!
//! Most of the core CPU logic is implemented in the [`Cpu::clock`] method.

use std::{
    fmt::{self, Display, Formatter},
    sync::Mutex,
};

use crate::{
    apu::Apu,
    assert_pedantic_gb,
    consts::LCDC_ADDR,
    debugln,
    dma::Dma,
    gb::GameBoyConfig,
    inst::{EXTENDED, INSTRUCTIONS},
    mmu::Mmu,
    pad::Pad,
    ppu::Ppu,
    serial::Serial,
    timer::Timer,
    util::SharedThread,
};

pub const PREFIX: u8 = 0xcb;

pub type Instruction = &'static (fn(&mut Cpu), u8, &'static str);

pub struct Cpu {
    pub pc: u16,
    pub sp: u16,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,

    ime: bool,
    zero: bool,
    sub: bool,
    half_carry: bool,
    carry: bool,
    halted: bool,

    /// Reference to the MMU (Memory Management Unit) to be used
    /// for memory bus access operations.
    pub mmu: Mmu,

    /// Temporary counter used to control the number of cycles
    /// taken by the current or last CPU operation.
    pub cycles: u8,

    /// Reference to the PC (Program Counter) of the previous executed
    /// instruction, used to provide a reference to the instruction
    /// so that it can be logged or used for debugging purposes.
    pub ppc: u16,

    /// The pointer to the parent configuration of the running
    /// Game Boy emulator, that can be used to control the behaviour
    /// of Game Boy emulation.
    gbc: SharedThread<GameBoyConfig>,
}

impl Cpu {
    pub fn new(mmu: Mmu, gbc: SharedThread<GameBoyConfig>) -> Self {
        Self {
            pc: 0x0,
            sp: 0x0,
            a: 0x0,
            b: 0x0,
            c: 0x0,
            d: 0x0,
            e: 0x0,
            h: 0x0,
            l: 0x0,
            ime: false,
            zero: false,
            sub: false,
            half_carry: false,
            carry: false,
            halted: false,
            mmu,
            cycles: 0,
            ppc: 0x0,
            gbc,
        }
    }

    pub fn reset(&mut self) {
        self.pc = 0x0;
        self.sp = 0x0;
        self.a = 0x0;
        self.b = 0x0;
        self.c = 0x0;
        self.d = 0x0;
        self.e = 0x0;
        self.h = 0x0;
        self.l = 0x0;
        self.ime = false;
        self.zero = false;
        self.sub = false;
        self.half_carry = false;
        self.carry = false;
        self.halted = false;
        self.cycles = 0;
    }

    /// Sets the CPU registers and some of the memory space to the
    /// expected state after a typical Game Boy boot ROM finishes.
    ///
    /// Using this strategy it's possible to skip the "normal" boot
    /// loading process for the original DMG Game Boy.
    pub fn boot(&mut self) {
        self.pc = 0x0100;
        self.sp = 0xfffe;
        self.a = 0x01;
        self.b = 0xff;
        self.c = 0x13;
        self.d = 0x00;
        self.e = 0xc1;
        self.h = 0x84;
        self.l = 0x03;
        self.zero = false;
        self.sub = false;
        self.half_carry = false;
        self.carry = false;

        // updates part of the MMU state, disabling the
        // boot memory overlap and setting the LCD control
        // register to enabled (required by some ROMs)
        self.mmu.set_boot_active(false);
        self.mmu.write(LCDC_ADDR, 0x91);
    }

    pub fn clock(&mut self) -> u8 {
        // gathers the PC (program counter) reference that
        // is going to be used in the fetching phase
        let pc = self.pc;

        // runs a series of assertions to guarantee CPU execution
        // state, only if pedantic mode is set
        assert_pedantic_gb!(
            !(0x8000..=0x9fff).contains(&pc),
            "Invalid PC area at 0x{:04x}",
            pc
        );
        assert_pedantic_gb!(
            !self.mmu.boot_active() || pc <= 0x08ff,
            "Invalid boot address: {:04x}",
            pc
        );

        // @TODO this is so bad, need to improve this by an order
        // of magnitude, to be able to have better performance
        // in case the CPU execution halted and there's an interrupt
        // to be handled, releases the CPU from the halted state
        // this verification is only done in case the IME (interrupt
        // master enable) is disabled, otherwise the CPU halt disabled
        // is going to be handled ahead
        if self.halted
            && !self.ime
            && self.mmu.ie != 0x00
            && (((self.mmu.ie & 0x01 == 0x01) && self.mmu.ppu().int_vblank())
                || ((self.mmu.ie & 0x02 == 0x02) && self.mmu.ppu().int_stat())
                || ((self.mmu.ie & 0x04 == 0x04) && self.mmu.timer().int_tima())
                || ((self.mmu.ie & 0x08 == 0x08) && self.mmu.serial().int_serial())
                || ((self.mmu.ie & 0x10 == 0x10) && self.mmu.pad().int_pad()))
        {
            self.halted = false;
        }

        // checks the IME (interrupt master enable) is enabled and then checks
        // if there's any interrupt to be handled, in case there's one, tries
        // to check which one should be handled and then handles it
        // this code assumes that the're no more that one interrupt triggered
        // per clock cycle, this is a limitation of the current implementation
        if self.ime && self.mmu.ie != 0x00 {
            if (self.mmu.ie & 0x01 == 0x01) && self.mmu.ppu().int_vblank() {
                debugln!("Going to run V-Blank interrupt handler (0x40)");

                self.disable_int();
                self.push_word(pc);
                self.pc = 0x40;

                // notifies the MMU about the V-Blank interrupt,
                // this may trigger some additional operations
                self.mmu.vblank();

                // acknowledges that the V-Blank interrupt has been
                // properly handled
                self.mmu.ppu().ack_vblank();

                // in case the CPU is currently halted waiting
                // for an interrupt, releases it
                if self.halted {
                    self.halted = false;
                }

                return 20;
            } else if (self.mmu.ie & 0x02 == 0x02) && self.mmu.ppu().int_stat() {
                debugln!("Going to run LCD STAT interrupt handler (0x48)");

                self.disable_int();
                self.push_word(pc);
                self.pc = 0x48;

                // acknowledges that the STAT interrupt has been
                // properly handled
                self.mmu.ppu().ack_stat();

                // in case the CPU is currently halted waiting
                // for an interrupt, releases it
                if self.halted {
                    self.halted = false;
                }

                return 20;
            } else if (self.mmu.ie & 0x04 == 0x04) && self.mmu.timer().int_tima() {
                debugln!("Going to run Timer interrupt handler (0x50)");

                self.disable_int();
                self.push_word(pc);
                self.pc = 0x50;

                // acknowledges that the timer interrupt has been
                // properly handled
                self.mmu.timer().ack_tima();

                // in case the CPU is currently halted waiting
                // for an interrupt, releases it
                if self.halted {
                    self.halted = false;
                }

                return 20;
            } else if (self.mmu.ie & 0x08 == 0x08) && self.mmu.serial().int_serial() {
                debugln!("Going to run Serial interrupt handler (0x58)");

                self.disable_int();
                self.push_word(pc);
                self.pc = 0x58;

                // acknowledges that the serial interrupt has been
                // properly handled
                self.mmu.serial().ack_serial();

                // in case the CPU is currently halted waiting
                // for an interrupt, releases it
                if self.halted {
                    self.halted = false;
                }

                return 20;
            } else if (self.mmu.ie & 0x10 == 0x10) && self.mmu.pad().int_pad() {
                debugln!("Going to run JoyPad interrupt handler (0x60)");

                self.disable_int();
                self.push_word(pc);
                self.pc = 0x60;

                // acknowledges that the pad interrupt has been
                // properly handled
                self.mmu.pad().ack_pad();

                // in case the CPU is currently halted waiting
                // for an interrupt, releases it
                if self.halted {
                    self.halted = false;
                }

                return 20;
            }
        }

        // in case the CPU is currently in the halted state
        // returns the control flow immediately with the associated
        // number of cycles estimated for the halted execution
        if self.halted {
            return 4;
        }

        // fetches the current instruction and updates the PC
        // (Program Counter) according to the final value returned
        // by the fetch operation (we may need to fetch instruction
        // more than one byte of length)
        let (inst, pc) = self.fetch(self.pc);
        self.ppc = self.pc;
        self.pc = pc;

        #[allow(unused_variables)]
        let (inst_fn, inst_time, inst_str) = inst;

        #[cfg(feature = "cpulog")]
        if *inst_str == "! UNIMP !" || *inst_str == "HALT" {
            if *inst_str == "HALT" {
                debugln!("HALT with IE=0x{:02x} IME={}", self.mmu.ie, self.ime);
            }
            debugln!(
                "{}\t(0x{:02x})\t${:04x} {}",
                inst_str,
                opcode,
                pc,
                is_prefix
            );
        }

        #[cfg(feature = "cpulog")]
        {
            println!("{}", self.description(inst, self.ppc));
        }

        // calls the current instruction and increments the number of
        // cycles executed by the instruction time of the instruction
        // that has just been executed
        self.cycles = 0;
        inst_fn(self);
        self.cycles = self.cycles.wrapping_add(*inst_time);

        // returns the number of cycles that the operation
        // that has been executed has taken
        self.cycles
    }

    #[inline(always)]
    fn fetch(&self, pc: u16) -> (Instruction, u16) {
        let mut pc = pc;

        // fetches the current instruction and increments
        // the PC (program counter) accordingly
        let mut opcode = self.mmu.read(pc);
        pc = pc.wrapping_add(1);

        // checks if the current instruction is a prefix
        // instruction, in case it is, fetches the next
        // instruction and increments the PC accordingly
        let inst: Instruction;
        let is_prefix = opcode == PREFIX;
        if is_prefix {
            opcode = self.mmu.read(pc);
            pc = pc.wrapping_add(1);
            inst = &EXTENDED[opcode as usize];
        } else {
            inst = &INSTRUCTIONS[opcode as usize];
        }

        // returns both the fetched instruction and the
        // updated PC (Program Counter) value
        (inst, pc)
    }

    #[inline(always)]
    pub fn mmu(&mut self) -> &mut Mmu {
        &mut self.mmu
    }

    #[inline(always)]
    pub fn mmu_i(&self) -> &Mmu {
        &self.mmu
    }

    #[inline(always)]
    pub fn ppu(&mut self) -> &mut Ppu {
        self.mmu().ppu()
    }

    #[inline(always)]
    pub fn ppu_i(&self) -> &Ppu {
        self.mmu_i().ppu_i()
    }

    #[inline(always)]
    pub fn apu(&mut self) -> &mut Apu {
        self.mmu().apu()
    }

    #[inline(always)]
    pub fn apu_i(&self) -> &Apu {
        self.mmu_i().apu_i()
    }

    #[inline(always)]
    pub fn dma(&mut self) -> &mut Dma {
        self.mmu().dma()
    }

    #[inline(always)]
    pub fn dma_i(&self) -> &Dma {
        self.mmu_i().dma_i()
    }

    #[inline(always)]
    pub fn pad(&mut self) -> &mut Pad {
        self.mmu().pad()
    }

    #[inline(always)]
    pub fn pad_i(&self) -> &Pad {
        self.mmu_i().pad_i()
    }

    #[inline(always)]
    pub fn timer(&mut self) -> &mut Timer {
        self.mmu().timer()
    }

    #[inline(always)]
    pub fn timer_i(&self) -> &Timer {
        self.mmu_i().timer_i()
    }

    #[inline(always)]
    pub fn serial(&mut self) -> &mut Serial {
        self.mmu().serial()
    }

    #[inline(always)]
    pub fn serial_i(&self) -> &Serial {
        self.mmu_i().serial_i()
    }

    #[inline(always)]
    pub fn halted(&self) -> bool {
        self.halted
    }

    #[inline(always)]
    pub fn set_halted(&mut self, value: bool) {
        self.halted = value
    }

    #[inline(always)]
    pub fn cycles(&self) -> u8 {
        self.cycles
    }

    #[inline(always)]
    pub fn pc(&self) -> u16 {
        self.pc
    }

    #[inline(always)]
    pub fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }

    #[inline(always)]
    pub fn sp(&self) -> u16 {
        self.sp
    }

    #[inline(always)]
    pub fn set_sp(&mut self, value: u16) {
        self.sp = value;
    }

    #[inline(always)]
    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | self.f() as u16
    }

    #[inline(always)]
    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    #[inline(always)]
    pub fn f(&self) -> u8 {
        let mut f = 0x0u8;
        if self.zero {
            f |= 0x80;
        }
        if self.sub {
            f |= 0x40;
        }
        if self.half_carry {
            f |= 0x20;
        }
        if self.carry {
            f |= 0x10;
        }
        f
    }

    #[inline(always)]
    pub fn set_f(&mut self, value: u8) {
        self.zero = value & 0x80 == 0x80;
        self.sub = value & 0x40 == 0x40;
        self.half_carry = value & 0x20 == 0x20;
        self.carry = value & 0x10 == 0x10;
    }

    #[inline(always)]
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.set_f(value as u8);
    }

    #[inline(always)]
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    #[inline(always)]
    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    #[inline(always)]
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    #[inline(always)]
    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    #[inline(always)]
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    #[inline(always)]
    pub fn ime(&self) -> bool {
        self.ime
    }

    #[inline(always)]
    pub fn set_ime(&mut self, value: bool) {
        self.ime = value;
    }

    #[inline(always)]
    pub fn read_u8(&mut self) -> u8 {
        let byte = self.mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    #[inline(always)]
    pub fn read_u16(&mut self) -> u16 {
        let byte1 = self.read_u8();
        let byte2 = self.read_u8();

        byte1 as u16 | ((byte2 as u16) << 8)
    }

    #[inline(always)]
    pub fn push_byte(&mut self, byte: u8) {
        self.sp = self.sp.wrapping_sub(1);
        self.mmu.write(self.sp, byte);
    }

    #[inline(always)]
    pub fn push_word(&mut self, word: u16) {
        self.push_byte((word >> 8) as u8);
        self.push_byte(word as u8);
    }

    #[inline(always)]
    pub fn pop_byte(&mut self) -> u8 {
        let byte = self.mmu.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        byte
    }

    #[inline(always)]
    pub fn pop_word(&mut self) -> u16 {
        self.pop_byte() as u16 | ((self.pop_byte() as u16) << 8)
    }

    #[inline(always)]
    pub fn zero(&self) -> bool {
        self.zero
    }

    #[inline(always)]
    pub fn set_zero(&mut self, value: bool) {
        self.zero = value
    }

    #[inline(always)]
    pub fn sub(&self) -> bool {
        self.sub
    }

    #[inline(always)]
    pub fn set_sub(&mut self, value: bool) {
        self.sub = value;
    }

    #[inline(always)]
    pub fn half_carry(&self) -> bool {
        self.half_carry
    }

    #[inline(always)]
    pub fn set_half_carry(&mut self, value: bool) {
        self.half_carry = value
    }

    #[inline(always)]
    pub fn carry(&self) -> bool {
        self.carry
    }

    #[inline(always)]
    pub fn set_carry(&mut self, value: bool) {
        self.carry = value;
    }

    #[inline(always)]
    pub fn halt(&mut self) {
        self.halted = true;
    }

    #[inline(always)]
    pub fn stop(&mut self) {
        let mmu = self.mmu();
        if mmu.switching {
            mmu.switch_speed()
        }
    }

    #[inline(always)]
    pub fn enable_int(&mut self) {
        self.ime = true;
    }

    #[inline(always)]
    pub fn disable_int(&mut self) {
        self.ime = false;
    }

    pub fn set_gbc(&mut self, value: SharedThread<GameBoyConfig>) {
        self.gbc = value;
    }

    pub fn description(&self, inst: Instruction, inst_pc: u16) -> String {
        let (_, inst_time, inst_str) = inst;
        let title_str: String = format!("[0x{:04x}] {}", inst_pc, inst_str);
        let inst_time_str = format!("({} cycles)", inst_time);
        let registers_str = format!("[PC=0x{:04x} SP=0x{:04x}] [A=0x{:02x} B=0x{:02x} C=0x{:02x} D=0x{:02x} E=0x{:02x} H=0x{:02x} L=0x{:02x}]",
        self.pc, self.sp, self.a, self.b, self.c, self.d, self.e, self.h, self.l);
        format!(
            "{0: <24} {1: <11} {2: <10}",
            title_str, inst_time_str, registers_str
        )
    }

    pub fn description_default(&self) -> String {
        let (inst, _) = self.fetch(self.ppc);
        self.description(inst, self.ppc)
    }
}

impl Default for Cpu {
    fn default() -> Self {
        let gbc = SharedThread::new(Mutex::new(GameBoyConfig::default()));
        Cpu::new(Mmu::default(), gbc)
    }
}

impl Display for Cpu {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description_default())
    }
}

#[cfg(test)]
mod tests {
    use super::Cpu;

    #[test]
    fn test_cpu_clock() {
        let mut cpu = Cpu::default();
        cpu.boot();
        cpu.mmu.allocate_default();

        // test NOP instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x00);
        let cycles = cpu.clock();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.pc, 0xc001);

        // test LD A, d8 instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x3e);
        cpu.mmu.write(0xc001, 0x42);
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc002);
        assert_eq!(cpu.a, 0x42);

        // test LD (HL+), A instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x22);
        cpu.set_hl(0xc000);
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc001);
        assert_eq!(cpu.hl(), 0xc001);
        assert_eq!(cpu.mmu.read(cpu.hl()), 0x42);

        // test INC A instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x3c);
        cpu.a = 0x42;
        let cycles = cpu.clock();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.pc, 0xc001);
        assert_eq!(cpu.a, 0x43);

        // test DEC A instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x3d);
        cpu.a = 0x42;
        let cycles = cpu.clock();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.pc, 0xc001);
        assert_eq!(cpu.a, 0x41);

        // test LD A, (HL) instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x7e);
        cpu.set_hl(0xc001);
        cpu.mmu.write(0xc001, 0x42);
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc001);
        assert_eq!(cpu.a, 0x42);
        assert_eq!(cpu.hl(), 0xc001);

        // test LD (HL), d8 instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x36);
        cpu.set_hl(0xc000);
        cpu.mmu.write(0xc001, 0x42);
        let cycles = cpu.clock();
        assert_eq!(cycles, 12);
        assert_eq!(cpu.pc, 0xc002);
        assert_eq!(cpu.hl(), 0xc000);
        assert_eq!(cpu.mmu.read(cpu.hl()), 0x42);

        // test JR n instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0x18);
        cpu.mmu.write(0xc001, 0x03);
        let cycles = cpu.clock();
        assert_eq!(cycles, 12);
        assert_eq!(cpu.pc, 0xc005);

        // test ADD A, d8 instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0xc6);
        cpu.mmu.write(0xc001, 0x01);
        cpu.a = 0x42;
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc002);
        assert_eq!(cpu.a, 0x43);

        // test SUB A, d8 instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0xd6);
        cpu.mmu.write(0xc001, 0x01);
        cpu.a = 0x42;
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc002);
        assert_eq!(cpu.a, 0x41);

        // test AND A, d8 instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0xe6);
        cpu.mmu.write(0xc001, 0x0f);
        cpu.a = 0x0a;
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc002);
        assert_eq!(cpu.a, 0x0a & 0x0f);

        // test OR A, d8 instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0xf6);
        cpu.mmu.write(0xc001, 0x0f);
        cpu.a = 0x0a;
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc002);
        assert_eq!(cpu.a, 0x0a | 0x0f);

        // test XOR A, d8 instruction
        cpu.pc = 0xc000;
        cpu.mmu.write(0xc000, 0xee);
        cpu.mmu.write(0xc001, 0x0f);
        cpu.a = 0x0a;
        let cycles = cpu.clock();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.pc, 0xc002);
        assert_eq!(cpu.a, 0x0a ^ 0x0f);
    }
}
