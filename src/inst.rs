use crate::cpu::Cpu;

pub const INSTRUCTIONS: [(fn(&mut Cpu), u8, &'static str); 256] = [
    // 0x0 opcodes
    (nop, 4, "NOP"),
    (ld_bc_u16, 12, "LD BC, u16"),
    (ld_mbc_a, 8, "LD [BC], A"),
    (inc_bc, 8, "INC BC"),
    (inc_b, 4, "INC B"),
    (dec_b, 4, "DEC B"),
    (ld_b_u8, 8, "LD B, u8"),
    (rlca, 4, "RLCA"),
    (ld_mu16_sp, 20, "LD [u16], SP"),
    (add_hl_bc, 8, "ADD HL, BC"),
    (noimpl, 4, "! UNIMP !"),
    (dec_bc, 8, "DEC BC"),
    (inc_c, 4, "INC C"),
    (dec_c, 4, "DEC C"),
    (ld_c_u8, 8, "LD C, u8"),
    (noimpl, 4, "! UNIMP !"),
    // 0x1 opcodes
    (noimpl, 4, "! UNIMP !"),
    (ld_de_u16, 12, "LD DE, u16"),
    (ld_mde_a, 8, "LD [DE], A"),
    (inc_de, 8, "INC DE"),
    (inc_d, 4, "INC D"),
    (dec_d, 4, "DEC D"),
    (ld_d_u8, 8, "LD D, u8"),
    (rla, 4, "RLA"),
    (jr_i8, 12, "JR i8"),
    (noimpl, 4, "! UNIMP !"),
    (ld_a_mde, 8, "LD A, [DE]"),
    (noimpl, 4, "! UNIMP !"),
    (inc_e, 4, "INC E"),
    (dec_e, 4, "DEC E"),
    (ld_e_u8, 8, "LD E, u8"),
    (noimpl, 4, "! UNIMP !"),
    // 0x2 opcodes
    (jr_nz_i8, 8, "JR NZ, i8"),
    (ld_hl_u16, 12, "LD HL, u16"),
    (ld_mhli_a, 8, "LD [HL+], A"),
    (inc_hl, 8, "INC HL"),
    (inc_h, 4, "INC H"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (jr_z_i8, 8, "JR Z, i8"),
    (noimpl, 4, "! UNIMP !"),
    (ld_a_mhli, 8, "LD A, [HL+] "),
    (noimpl, 4, "! UNIMP !"),
    (inc_l, 4, "INC L"),
    (noimpl, 4, "! UNIMP !"),
    (ld_l_u8, 8, "LD L, u8"),
    (cpl, 4, "CPL"),
    // 0x3 opcodes
    (jr_nc_i8, 8, "JR NC, i8"),
    (ld_sp_u16, 12, "LD SP, u16"),
    (ld_mhld_a, 8, "LD [HL-], A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_mhl_u8, 12, "LD [HL], u8 "),
    (scf, 4, "SCF"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (dec_a, 4, "DEC A"),
    (ld_a_u8, 8, "LD A, u8"),
    (noimpl, 4, "! UNIMP !"),
    // 0x4 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_b_h, 4, "LD B, H"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_b_a, 4, "LD B, A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_c_a, 4, "LD C, A"),
    // 0x5 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_d_a, 4, "LD D, A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x6 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_h_a, 4, "LD H, A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x7 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_mhl_a, 8, "LD [HL], A"),
    (ld_a_b, 4, "LD A, B"),
    (ld_a_c, 4, "LD A, C"),
    (noimpl, 4, "! UNIMP !"),
    (ld_a_e, 4, "LD A, E"),
    (ld_a_h, 4, "LD A, H"),
    (ld_a_l, 4, "LD A, L"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x8 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (add_a_mhl, 8, "ADD A, [HL]"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x9 opcodes
    (sub_a_b, 4, "SUB A, B"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0xa opcodes
    (noimpl, 4, "! UNIMP !"),
    (and_a_c, 4, "AND A, C"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (xor_a_c, 4, "XOR A, C"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (xor_a_a, 4, "XOR A, A"),
    // 0xb opcodes
    (or_a_b, 4, "OR A, B"),
    (or_a_c, 4, "OR A, C"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (cp_a_mhl, 8, "CP A, [HL]"),
    (noimpl, 4, "! UNIMP !"),
    // 0xc opcodes
    (ret_nz, 8, "RET NZ"),
    (pop_bc, 12, "POP BC"),
    (noimpl, 4, "! UNIMP !"),
    (jp_u16, 16, "JP u16"),
    (call_nz_u16, 12, "CALL NZ, u16"),
    (push_bc, 16, "PUSH BC"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ret, 16, "RET"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (call_u16, 24, "CALL u16"),
    (noimpl, 4, "! UNIMP !"),
    (rst_08h, 16, "RST 08h"),
    // 0xd opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0xe opcodes
    (ld_mff00u8_a, 12, "LD [FF00+u8], A"),
    (pop_hl, 12, "POP HL"),
    (ld_mff00c_a, 8, "LD [FF00+C], A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (push_hl, 16, "PUSH HL"),
    (and_a_u8, 8, "AND A, u8"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_mu16_a, 16, "LD [u16], A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (rst_18h, 16, "RST 18h"),
    // 0xf opcodes
    (ld_a_mff00u8, 12, "LD A, [FF00+u8]"),
    (pop_af, 12, "POP AF"),
    (noimpl, 4, "! UNIMP !"),
    (di, 4, "DI"),
    (noimpl, 4, "! UNIMP !"),
    (push_af, 16, "PUSH AF"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_a_mu16, 16, "LD A [u16]"),
    (ei, 4, "EI"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (cp_a_u8, 8, "CP A, u8"),
    (rst_38h, 16, "RST 38h"),
];

pub const BITWISE: [(fn(&mut Cpu), u8, &'static str); 176] = [
    // 0x0 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x1 opcodes
    (noimpl, 4, "! UNIMP !"),
    (rl_c, 8, "RL C"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x2 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x3 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (swap_a, 8, "SWAP A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x4 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x5 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x6 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x7 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (bit_7_h, 8, "BIT 7, H"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x8 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x9 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0xa opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
];

fn nop(_cpu: &mut Cpu) {}

fn noimpl(_cpu: &mut Cpu) {
    todo!("Instruction not implemented");
}

fn ld_bc_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.set_bc(word);
}

fn ld_mbc_a(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.bc(), cpu.a);
}

fn inc_bc(cpu: &mut Cpu) {
    cpu.set_bc(cpu.bc().wrapping_add(1));
}

fn inc_b(cpu: &mut Cpu) {
    let value = cpu.b.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((value & 0xf) == 0xf);

    cpu.b = value;
}

fn dec_b(cpu: &mut Cpu) {
    let b = cpu.b;
    let value = b.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((b & 0xf) == 0xf);

    cpu.b = value;
}

fn ld_b_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.b = byte;
}

fn rlca(cpu: &mut Cpu) {
    let carry = cpu.a >> 7;

    cpu.a = cpu.a << 1 | carry;

    cpu.set_sub(false);
    cpu.set_zero(false);
    cpu.set_half_carry(false);
    cpu.set_carry(carry == 1);
}

fn ld_mu16_sp(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.mmu.write(word, cpu.sp as u8);
    cpu.mmu.write(word + 1, (cpu.sp >> 8) as u8);
}

fn add_hl_bc(cpu: &mut Cpu) {
    let value = add_u16_u16(cpu, cpu.hl(), cpu.bc());
    cpu.set_hl(value);
}

fn dec_bc(cpu: &mut Cpu) {
    cpu.set_bc(cpu.bc().wrapping_sub(1));
}

fn inc_c(cpu: &mut Cpu) {
    let value = cpu.c.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((value & 0xf) == 0xf);

    cpu.c = value;
}

fn dec_c(cpu: &mut Cpu) {
    let c = cpu.c;
    let value = c.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((c & 0xf) == 0xf);

    cpu.c = value;
}

fn ld_c_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.c = byte;
}

fn ld_de_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.set_de(word);
}

fn ld_mde_a(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.de(), cpu.a);
}

fn inc_de(cpu: &mut Cpu) {
    cpu.set_de(cpu.de().wrapping_add(1));
}

fn inc_d(cpu: &mut Cpu) {
    let value = cpu.d.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((value & 0xf) == 0xf);

    cpu.d = value;
}

fn dec_d(cpu: &mut Cpu) {
    let d = cpu.d;
    let value = d.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((d & 0xf) == 0xf);

    cpu.d = value;
}

fn ld_d_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.d = byte;
}

fn rla(cpu: &mut Cpu) {
    let carry = cpu.get_carry();

    cpu.set_carry(cpu.a & 0x80 == 0x80);

    cpu.a = cpu.a << 1 | carry as u8;

    cpu.set_sub(false);
    cpu.set_zero(false);
    cpu.set_half_carry(false);
}

fn jr_i8(cpu: &mut Cpu) {
    let byte = cpu.read_u8() as i8;
    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
}

fn ld_a_mde(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.de());
    cpu.a = byte;
}

fn inc_e(cpu: &mut Cpu) {
    let value = cpu.e.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((value & 0xf) == 0xf);

    cpu.e = value;
}

fn dec_e(cpu: &mut Cpu) {
    let e = cpu.e;
    let value = e.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((e & 0xf) == 0xf);

    cpu.e = value;
}

fn ld_e_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.e = byte;
}

fn jr_nz_i8(cpu: &mut Cpu) {
    let byte = cpu.read_u8() as i8;

    if cpu.get_zero() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.ticks = cpu.ticks.wrapping_add(4);
}

fn ld_hl_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.set_hl(word);
}

fn ld_mhli_a(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.a);
    cpu.set_hl(cpu.hl().wrapping_add(1));
}

fn inc_hl(cpu: &mut Cpu) {
    cpu.set_hl(cpu.hl().wrapping_add(1));
}

fn inc_h(cpu: &mut Cpu) {
    let value = cpu.h.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((value & 0xf) == 0xf);

    cpu.h = value;
}

fn jr_z_i8(cpu: &mut Cpu) {
    let byte = cpu.read_u8() as i8;

    if !cpu.get_zero() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.ticks = cpu.ticks.wrapping_add(4);
}

fn ld_a_mhli(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = byte;
    cpu.set_hl(cpu.hl().wrapping_add(1));
}

fn inc_l(cpu: &mut Cpu) {
    let value = cpu.l.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((value & 0xf) == 0xf);

    cpu.l = value;
}

fn ld_l_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.l = byte;
}

fn cpl(cpu: &mut Cpu) {
    cpu.a = !cpu.a;

    cpu.set_sub(true);
    cpu.set_half_carry(true);
}

fn ld_sp_u16(cpu: &mut Cpu) {
    cpu.sp = cpu.read_u16();
}

fn jr_nc_i8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();

    if cpu.get_carry() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.ticks = cpu.ticks.wrapping_add(4);
}

fn ld_mhld_a(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.a);
    cpu.set_hl(cpu.hl().wrapping_sub(1));
}

fn ld_mhl_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.mmu.write(cpu.hl(), byte);
}

fn scf(cpu: &mut Cpu) {
    cpu.set_sub(false);
    cpu.set_half_carry(false);
    cpu.set_carry(true);
}

fn dec_a(cpu: &mut Cpu) {
    let a = cpu.a;
    let value = a.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((a & 0xf) == 0xf);

    cpu.a = value;
}

fn ld_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = byte;
}

fn ld_b_h(cpu: &mut Cpu) {
    cpu.b = cpu.h;
}

fn ld_b_a(cpu: &mut Cpu) {
    cpu.b = cpu.a;
}

fn ld_c_a(cpu: &mut Cpu) {
    cpu.c = cpu.a;
}

fn ld_d_a(cpu: &mut Cpu) {
    cpu.d = cpu.a;
}

fn ld_h_a(cpu: &mut Cpu) {
    cpu.h = cpu.a;
}

fn ld_mhl_a(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.a);
}

fn ld_a_b(cpu: &mut Cpu) {
    cpu.a = cpu.b;
}

fn ld_a_c(cpu: &mut Cpu) {
    cpu.a = cpu.c;
}

fn ld_a_e(cpu: &mut Cpu) {
    cpu.a = cpu.e;
}

fn ld_a_h(cpu: &mut Cpu) {
    cpu.a = cpu.h;
}

fn ld_a_l(cpu: &mut Cpu) {
    cpu.a = cpu.l;
}

fn add_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = add_set_flags(cpu, cpu.a, byte);
}

fn sub_a_b(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.b);
}

fn and_a_c(cpu: &mut Cpu) {
    cpu.a &= cpu.c;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn xor_a_c(cpu: &mut Cpu) {
    cpu.a ^= cpu.c;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn xor_a_a(cpu: &mut Cpu) {
    cpu.a ^= cpu.a;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn or_a_b(cpu: &mut Cpu) {
    cpu.a |= cpu.b;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn or_a_c(cpu: &mut Cpu) {
    cpu.a |= cpu.c;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn cp_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    sub_set_flags(cpu, cpu.a, byte);
}

fn ret_nz(cpu: &mut Cpu) {
    if cpu.get_zero() {
        return;
    }

    cpu.pc = cpu.pop_word();
    cpu.ticks = cpu.ticks.wrapping_add(12);
}

fn pop_bc(cpu: &mut Cpu) {
    let word = cpu.pop_word();
    cpu.set_bc(word);
}

fn jp_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.pc = word;
}

fn call_nz_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if cpu.get_zero() {
        return;
    }

    cpu.push_word(cpu.pc);
    cpu.pc = word;
    cpu.ticks = cpu.ticks.wrapping_add(12);
}

fn push_bc(cpu: &mut Cpu) {
    cpu.push_word(cpu.bc());
}

fn ret(cpu: &mut Cpu) {
    cpu.pc = cpu.pop_word();
}

fn call_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.push_word(cpu.pc);
    cpu.pc = word;
}

fn rst_08h(cpu: &mut Cpu) {
    rst(cpu, 0x0008);
}

fn ld_mff00u8_a(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.mmu.write(0xff00 + byte as u16, cpu.a);
}

fn pop_hl(cpu: &mut Cpu) {
    let word = cpu.pop_word();
    cpu.set_hl(word);
}

fn ld_mff00c_a(cpu: &mut Cpu) {
    cpu.mmu.write(0xff00 + cpu.c as u16, cpu.a);
}

fn push_hl(cpu: &mut Cpu) {
    cpu.push_word(cpu.hl());
}

fn and_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();

    cpu.a &= byte;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn ld_mu16_a(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.mmu.write(word, cpu.a);
}

fn rst_18h(cpu: &mut Cpu) {
    rst(cpu, 0x0018);
}

fn ld_a_mff00u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = cpu.mmu.read(0xff00 + byte as u16);
}

fn pop_af(cpu: &mut Cpu) {
    let word = cpu.pop_word();
    cpu.set_af(word);
}

fn di(cpu: &mut Cpu) {
    cpu.disable_int();
}

fn push_af(cpu: &mut Cpu) {
    cpu.push_word(cpu.af());
}

fn ld_a_mu16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    let byte = cpu.mmu.read(word);
    cpu.a = byte;
}

fn ei(cpu: &mut Cpu) {
    cpu.enable_int();
}

fn cp_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    sub_set_flags(cpu, cpu.a, byte);
}

fn rst_38h(cpu: &mut Cpu) {
    rst(cpu, 0x0038);
}

fn rl_c(cpu: &mut Cpu) {
    cpu.c = rl(cpu, cpu.c);
}

fn swap_a(cpu: &mut Cpu) {
    cpu.a = swap(cpu, cpu.a)
}

fn bit_7_h(cpu: &mut Cpu) {
    bit_h(cpu, 7);
}

/// Helper function that rotates (shifts) the given
/// byte (probably from a register) and updates the
/// proper flag registers.
fn rl(cpu: &mut Cpu, byte: u8) -> u8 {
    let carry = cpu.get_carry();

    cpu.set_carry(byte & 0x80 == 0x80);

    let result = (byte << 1) | carry as u8;

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);

    result
}

/// Helper function to test one bit in a u8.
/// Returns true if bit is 0.
fn bit_zero(val: u8, bit: u8) -> bool {
    (val & (1u8 << (bit as usize))) == 0
}

fn bit_h(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.h, bit));
    cpu.set_half_carry(true);
}

fn add_set_flags(cpu: &mut Cpu, first: u8, second: u8) -> u8 {
    let first = first as u32;
    let second = second as u32;

    let value = first.wrapping_add(second);
    let value_b = value as u8;

    cpu.set_sub(false);
    cpu.set_zero(value_b == 0);
    cpu.set_half_carry((first ^ second ^ value) & 0x10 == 0x10);
    cpu.set_carry(value & 0x100 == 0x100);

    value_b
}

fn sub_set_flags(cpu: &mut Cpu, first: u8, second: u8) -> u8 {
    let first = first as u32;
    let second = second as u32;

    let value = first.wrapping_sub(second);
    let value_b = value as u8;

    cpu.set_sub(true);
    cpu.set_zero(value_b == 0);
    cpu.set_half_carry((first ^ second ^ value) & 0x10 == 0x10);
    cpu.set_carry(value & 0x100 == 0x100);

    value_b
}

fn add_u16_u16(cpu: &mut Cpu, first: u16, second: u16) -> u16 {
    let first = first as u32;
    let second = second as u32;
    let value = first.wrapping_add(second);

    cpu.set_sub(false);
    cpu.set_half_carry((first ^ second ^ value) & 0x1000 == 0x1000);
    cpu.set_carry(value & 0x10000 == 0x10000);

    value as u16
}

fn swap(cpu: &mut Cpu, value: u8) -> u8 {
    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);

    (value << 4) | (value >> 4)
}

/// Helper function for RST instructions, pushes the
/// current PC to the stack and jumps to the provided
/// address.
fn rst(cpu: &mut Cpu, addr: u16) {
    cpu.push_word(cpu.pc);
    cpu.pc = addr;
}
