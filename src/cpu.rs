pub const RAM_SIZE: usize = 8192;

pub const INSTRUCTIONS: [(fn(&mut Cpu), u8, &'static str); 4] = [
    // 0x0 opcodes
    (nop, 4, "NOP"),
    (ld_bc_u16, 12, "LD BC, NN"),
    (ld_bc_a, 8, "LD BC, A"),
    // 0x2 opcodes
    (ld_sp_u16, 12, "LD SP, NN"),
];

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
    fn read_u8(&mut self) -> u8 {
        let byte = self.ram[self.pc as usize];
        self.pc += 1;
        byte
    }

    #[inline(always)]
    fn read_u16(&mut self) -> u16 {
        let word = (self.ram[self.pc as usize] as u16) << 8 | self.ram[self.pc as usize + 1] as u16;
        self.pc += 2;
        word
    }

    #[inline(always)]
    fn get_zero(&self) -> bool {
        self.reg_f & 0x40 == 1
    }

    #[inline(always)]
    fn get_sub(&self) -> bool {
        self.reg_f & 0x20 == 1
    }

    #[inline(always)]
    fn get_half_carry(&self) -> bool {
        self.reg_f & 0x10 == 1
    }

    #[inline(always)]
    fn get_carry(&self) -> bool {
        self.reg_f & 0x08 == 1
    }
}

fn nop(_cpu: &mut Cpu) {}

fn ld_bc_u16(cpu: &mut Cpu) {
    cpu.reg_b = cpu.read_u8();
    cpu.reg_c = cpu.read_u8();
}

fn ld_bc_a(cpu: &mut Cpu) {
    cpu.ram[cpu.reg_bc() as usize] = cpu.reg_a;
}

fn ld_sp_u16(cpu: &mut Cpu) {
    cpu.sp = cpu.read_u16();
}
