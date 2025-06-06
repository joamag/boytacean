//! Implementation of the core CPU ([Sharp LR35902](https://en.wikipedia.org/wiki/Game_Boy)) logic for the Game Boy.
//!
//! Does not include the instruction set implementation, only the core
//! CPU logic and the CPU struct definition.
//!
//! Most of the core CPU logic is implemented in the [`Cpu::clock`] method.

use boytacean_common::{
    data::{read_u16, read_u8, write_u16, write_u8},
    error::Error,
    util::SharedThread,
};
use std::{
    fmt::{self, Display, Formatter},
    io::Cursor,
    sync::Mutex,
};

use crate::{
    apu::Apu,
    assert_pedantic_gb,
    consts::{IF_ADDR, LCDC_ADDR},
    debugln,
    dma::Dma,
    gb::GameBoyConfig,
    inst::{EXTENDED, INSTRUCTIONS},
    mmu::Mmu,
    pad::Pad,
    ppu::Ppu,
    serial::Serial,
    state::{StateComponent, StateFormat},
    timer::Timer,
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

        // prefetch the pending interrupt flags so we can quickly check
        // if any enabled interrupt is waiting to be served. This is used
        // both to release the CPU from a halted state and to execute the
        // correct handler when IME is enabled.
        let pending = self.mmu.read(IF_ADDR) & self.mmu.ie;

        // in case the CPU execution halted and there's a pending interrupt
        // while IME is disabled, release the CPU from the halted state so
        // execution can continue until the interrupt is serviced
        if self.halted && !self.ime && pending != 0 {
            self.halted = false;
        }

        // checks the IME (interrupt master enable) is enabled and then checks
        // if there's any interrupt to be handled, in case there's one, tries
        // to check which one should be handled and then handles it
        // this code assumes that the're no more that one interrupt triggered
        // per clock cycle, this is a limitation of the current implementation
        if self.ime && pending != 0 {
            if pending & 0x01 == 0x01 {
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
            } else if pending & 0x02 == 0x02 {
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
            } else if pending & 0x04 == 0x04 {
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
            } else if pending & 0x08 == 0x08 {
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
            } else if pending & 0x10 == 0x10 {
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
        ((self.a as u16) << 8) | self.f() as u16
    }

    #[inline(always)]
    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
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
        ((self.d as u16) << 8) | self.e as u16
    }

    #[inline(always)]
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    #[inline(always)]
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
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
        let title_str: String = format!("[0x{inst_pc:04x}] {inst_str}");
        let inst_time_str = format!("({inst_time} cycles)");
        let registers_str = format!("[PC=0x{:04x} SP=0x{:04x}] [A=0x{:02x} B=0x{:02x} C=0x{:02x} D=0x{:02x} E=0x{:02x} H=0x{:02x} L=0x{:02x}]",
        self.pc, self.sp, self.a, self.b, self.c, self.d, self.e, self.h, self.l);
        format!("{title_str: <24} {inst_time_str: <11} {registers_str: <10}")
    }

    pub fn description_default(&self) -> String {
        let (inst, _) = self.fetch(self.ppc);
        self.description(inst, self.ppc)
    }
}

impl StateComponent for Cpu {
    fn state(&self, _format: Option<StateFormat>) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(vec![]);
        write_u16(&mut cursor, self.pc)?;
        write_u16(&mut cursor, self.sp)?;
        write_u8(&mut cursor, self.a)?;
        write_u8(&mut cursor, self.b)?;
        write_u8(&mut cursor, self.c)?;
        write_u8(&mut cursor, self.d)?;
        write_u8(&mut cursor, self.e)?;
        write_u8(&mut cursor, self.h)?;
        write_u8(&mut cursor, self.l)?;
        write_u8(&mut cursor, self.ime as u8)?;
        write_u8(&mut cursor, self.zero as u8)?;
        write_u8(&mut cursor, self.sub as u8)?;
        write_u8(&mut cursor, self.half_carry as u8)?;
        write_u8(&mut cursor, self.carry as u8)?;
        write_u8(&mut cursor, self.halted as u8)?;
        write_u8(&mut cursor, self.cycles)?;
        write_u16(&mut cursor, self.ppc)?;
        Ok(cursor.into_inner())
    }

    fn set_state(&mut self, data: &[u8], _format: Option<StateFormat>) -> Result<(), Error> {
        let mut cursor = Cursor::new(data);
        self.pc = read_u16(&mut cursor)?;
        self.sp = read_u16(&mut cursor)?;
        self.a = read_u8(&mut cursor)?;
        self.b = read_u8(&mut cursor)?;
        self.c = read_u8(&mut cursor)?;
        self.d = read_u8(&mut cursor)?;
        self.e = read_u8(&mut cursor)?;
        self.h = read_u8(&mut cursor)?;
        self.l = read_u8(&mut cursor)?;
        self.ime = read_u8(&mut cursor)? != 0;
        self.zero = read_u8(&mut cursor)? != 0;
        self.sub = read_u8(&mut cursor)? != 0;
        self.half_carry = read_u8(&mut cursor)? != 0;
        self.carry = read_u8(&mut cursor)? != 0;
        self.halted = read_u8(&mut cursor)? != 0;
        self.cycles = read_u8(&mut cursor)?;
        self.ppc = read_u16(&mut cursor)?;
        Ok(())
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
    use std::sync::Mutex;

    use boytacean_common::util::SharedThread;

    use crate::{gb::GameBoyConfig, mmu::Mmu, state::StateComponent};

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

    #[test]
    fn test_state_and_set_state() {
        let cpu = Cpu {
            pc: 0x1234,
            sp: 0x5678,
            a: 0x9a,
            b: 0xbc,
            c: 0xde,
            d: 0xf0,
            e: 0x12,
            h: 0x34,
            l: 0x56,
            ime: true,
            zero: true,
            sub: false,
            half_carry: true,
            carry: false,
            halted: true,
            mmu: Mmu::default(),
            cycles: 0x78,
            ppc: 0x9abc,
            gbc: SharedThread::new(Mutex::new(GameBoyConfig::default())),
        };

        let state = cpu.state(None).unwrap();
        assert_eq!(state.len(), 20);

        let mut new_cpu = Cpu::default();
        new_cpu.set_state(&state, None).unwrap();

        assert_eq!(new_cpu.pc, 0x1234);
        assert_eq!(new_cpu.sp, 0x5678);
        assert_eq!(new_cpu.a, 0x9a);
        assert_eq!(new_cpu.b, 0xbc);
        assert_eq!(new_cpu.c, 0xde);
        assert_eq!(new_cpu.d, 0xf0);
        assert_eq!(new_cpu.e, 0x12);
        assert_eq!(new_cpu.h, 0x34);
        assert_eq!(new_cpu.l, 0x56);
        assert!(new_cpu.ime);
        assert!(new_cpu.zero);
        assert!(!new_cpu.sub);
        assert!(new_cpu.half_carry);
        assert!(!new_cpu.carry);
        assert!(new_cpu.halted);
        assert_eq!(new_cpu.cycles, 0x78);
        assert_eq!(new_cpu.ppc, 0x9abc);
    }
}
