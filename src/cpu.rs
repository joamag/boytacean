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
}

impl Cpu {
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
