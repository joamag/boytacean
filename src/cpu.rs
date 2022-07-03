use crate::{
    inst::{EXTENDED, INSTRUCTIONS},
    mmu::Mmu,
    ppu::Ppu,
};

pub const PREFIX: u8 = 0xcb;

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
    zero: bool,
    sub: bool,
    half_carry: bool,
    carry: bool,
    pub mmu: Mmu,
    pub ticks: u32,
}

impl Cpu {
    pub fn new(mmu: Mmu) -> Cpu {
        let mut implemented = 0;
        let mut implemented_ext = 0;

        for instruction in INSTRUCTIONS {
            if instruction.2 != "! UNIMP !" {
                implemented += 1;
            }
        }

        for instruction in EXTENDED {
            if instruction.2 != "! UNIMP !" {
                implemented_ext += 1;
            }
        }

        println!(
            "Implemented {}/{} instructions",
            implemented,
            INSTRUCTIONS.len()
        );
        println!(
            "Implemented {}/{} extended instructions",
            implemented_ext,
            EXTENDED.len()
        );

        Cpu {
            pc: 0x0,
            sp: 0x0,
            a: 0x0,
            b: 0x0,
            c: 0x0,
            d: 0x0,
            e: 0x0,
            h: 0x0,
            l: 0x0,
            zero: false,
            sub: false,
            half_carry: false,
            carry: false,
            mmu: mmu,
            ticks: 0,
        }
    }

    pub fn clock(&mut self) -> u8 {
        let pc = self.pc;

        if pc >= 0x8000 && pc < 0x9fff {
            panic!("Invalid PC area at 0x{:04x}", pc);
        }

        // fetches the current instruction and increments
        // the PC (program counter) accordingly
        let mut opcode = self.mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        let is_prefix = opcode == PREFIX;
        let instruction: &(fn(&mut Cpu), u8, &str);

        if is_prefix {
            opcode = self.mmu.read(self.pc);
            self.pc = self.pc.wrapping_add(1);
            instruction = &EXTENDED[opcode as usize];
        } else {
            instruction = &INSTRUCTIONS[opcode as usize];
        }

        let (instruction_fn, instruction_time, instruction_str) = instruction;

        // if !self.mmu.boot_active() {
        if *instruction_str == "! UNIMP !" {
            println!(
                "{}\t(0x{:02x})\t${:04x} {}",
                instruction_str, opcode, pc, is_prefix
            );
        }

        // calls the current instruction and increments the number of
        // cycles executed by the instruction time of the instruction
        // that has just been executed
        instruction_fn(self);
        self.ticks = self.ticks.wrapping_add(*instruction_time as u32);

        // returns the number of cycles that the operation
        // that has been executed has taken
        *instruction_time
    }

    #[inline(always)]
    pub fn mmu(&mut self) -> &mut Mmu {
        &mut self.mmu
    }

    #[inline(always)]
    pub fn ppu(&mut self) -> &mut Ppu {
        self.mmu().ppu()
    }

    #[inline(always)]
    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    #[inline(always)]
    pub fn pc(&self) -> u16 {
        self.pc
    }

    #[inline(always)]
    pub fn sp(&self) -> u16 {
        self.sp
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
    pub fn read_u8(&mut self) -> u8 {
        let byte = self.mmu.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    #[inline(always)]
    pub fn read_u16(&mut self) -> u16 {
        let byte1 = self.read_u8();
        let byte2 = self.read_u8();
        let word = byte1 as u16 | ((byte2 as u16) << 8);
        word
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
        let word = self.pop_byte() as u16 | ((self.pop_byte() as u16) << 8);
        word
    }

    #[inline(always)]
    pub fn get_zero(&self) -> bool {
        self.zero
    }

    #[inline(always)]
    pub fn set_zero(&mut self, value: bool) {
        self.zero = value
    }

    #[inline(always)]
    pub fn get_sub(&self) -> bool {
        self.sub
    }

    #[inline(always)]
    pub fn set_sub(&mut self, value: bool) {
        self.sub = value;
    }

    #[inline(always)]
    pub fn get_half_carry(&self) -> bool {
        self.half_carry
    }

    #[inline(always)]
    pub fn set_half_carry(&mut self, value: bool) {
        self.half_carry = value
    }

    #[inline(always)]
    pub fn get_carry(&self) -> bool {
        self.carry
    }

    #[inline(always)]
    pub fn set_carry(&mut self, value: bool) {
        self.carry = value;
    }

    #[inline(always)]
    pub fn enable_int(&mut self) {
        // @todo implement this one
    }

    #[inline(always)]
    pub fn disable_int(&mut self) {
        // @todo implement this one
    }
}
