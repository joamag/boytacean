const RAM_SIZE: usize = 8192;

pub struct Cpu {
    pc: u16,
    sp: u16,
    reg_a: u8,
    reg_b: u8,
    reg_c: u8,
    reg_d: u8,
    reg_e: u8,
    reg_f: u8,
    reg_h: u8,
    reg_l: u8,
    ram: [u8; RAM_SIZE],
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            pc: 0x0,
            sp: 0x0,
            reg_a: 0x0,
            reg_b: 0x0,
            reg_c: 0x0,
            reg_d: 0x0,
            reg_e: 0x0,
            reg_f: 0x0,
            reg_h: 0x0,
            reg_l: 0x0,
            ram: [0u8; RAM_SIZE],
        }
    }

    pub fn clock(&mut self) {
        // fetches the current instruction and increments
        // the PC (program counter) accordingly
        let instruction = self.ram[self.pc as usize];
        self.pc += 1;

        let opcode = instruction & 0xf000;
        let address = instruction & 0x0fff;
        let x = ((instruction & 0x0f00) >> 8) as usize;
        let y = ((instruction & 0x00f0) >> 4) as usize;
        let nibble = (instruction & 0x000f) as u8;
        let byte = (instruction & 0x00ff) as u8;
    }

    #[inline(always)]
    fn reg_af(&self) -> u16 {
        (self.reg_a as u16) << 8 | self.reg_f as u16
    }

    #[inline(always)]
    fn reg_bc(&self) -> u16 {
        (self.reg_b as u16) << 8 | self.reg_c as u16
    }

    #[inline(always)]
    fn reg_de(&self) -> u16 {
        (self.reg_d as u16) << 8 | self.reg_e as u16
    }

    #[inline(always)]
    fn reg_hl(&self) -> u16 {
        (self.reg_h as u16) << 8 | self.reg_l as u16
    }

    #[inline(always)]
    fn zero_flag(&self) -> bool {
        self.reg_f & 0x40 == 1
    }

    #[inline(always)]
    fn sub_flag(&self) -> bool {
        self.reg_f & 0x20 == 1
    }

    #[inline(always)]
    fn half_carry_flag(&self) -> bool {
        self.reg_f & 0x10 == 1
    }

    #[inline(always)]
    fn carry_flag(&self) -> bool {
        self.reg_f & 0x08 == 1
    }
}
