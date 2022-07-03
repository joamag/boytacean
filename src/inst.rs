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
    (ld_a_mbc, 8, "LD A, [BC]"),
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
    (dec_de, 8, "DEC DE"),
    (inc_e, 4, "INC E"),
    (dec_e, 4, "DEC E"),
    (ld_e_u8, 8, "LD E, u8"),
    (rra, 4, "RRA"),
    // 0x2 opcodes
    (jr_nz_i8, 8, "JR NZ, i8"),
    (ld_hl_u16, 12, "LD HL, u16"),
    (ld_mhli_a, 8, "LD [HL+], A"),
    (inc_hl, 8, "INC HL"),
    (inc_h, 4, "INC H"),
    (dec_h, 4, "DEC H"),
    (ld_h_u8, 8, "LD H, u8"),
    (noimpl, 4, "! UNIMP !"),
    (jr_z_i8, 8, "JR Z, i8"),
    (noimpl, 4, "! UNIMP !"),
    (ld_a_mhli, 8, "LD A, [HL+] "),
    (noimpl, 4, "! UNIMP !"),
    (inc_l, 4, "INC L"),
    (dec_l, 4, "DEC L"),
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
    (jr_c_i8, 8, "JR C, i8"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (inc_a, 4, "INC A"),
    (dec_a, 4, "DEC A"),
    (ld_a_u8, 8, "LD A, u8"),
    (noimpl, 4, "! UNIMP !"),
    // 0x4 opcodes
    (ld_b_b, 4, "LD B, B"),
    (ld_b_c, 4, "LD B, C"),
    (ld_b_d, 4, "LD B, D"),
    (ld_b_e, 4, "LD B, E"),
    (ld_b_h, 4, "LD B, H"),
    (noimpl, 4, "! UNIMP !"),
    (ld_b_mhl, 8, "LD B, [HL]"),
    (ld_b_a, 4, "LD B, A"),
    (ld_c_b, 4, "LD C, B"),
    (ld_c_c, 4, "LD C, C"),
    (ld_c_d, 4, "LD C, D"),
    (ld_c_e, 4, "LD C, E"),
    (ld_c_h, 4, "LD C, H"),
    (ld_c_l, 4, "LD C, L"),
    (ld_c_mhl, 8, "LD C, [HL]"),
    (ld_c_a, 4, "LD C, A"),
    // 0x5 opcodes
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_d_mhl, 8, "LD D, [HL]"),
    (ld_d_a, 4, "LD D, A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ld_e_a, 4, "LD E, A"),
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
    (ld_mhl_b, 8, "LD [HL], B"),
    (ld_mhl_c, 8, "LD [HL], C"),
    (ld_mhl_d, 8, "LD [HL], D"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (halt, 4, "HALT"),
    (ld_mhl_a, 8, "LD [HL], A"),
    (ld_a_b, 4, "LD A, B"),
    (ld_a_c, 4, "LD A, C"),
    (ld_a_d, 4, "LD A, D"),
    (ld_a_e, 4, "LD A, E"),
    (ld_a_h, 4, "LD A, H"),
    (ld_a_l, 4, "LD A, L"),
    (ld_a_mhl, 8, "LD A, [HL]"),
    (noimpl, 4, "! UNIMP !"),
    // 0x8 opcodes
    (add_a_b, 4, "ADD A, B"),
    (add_a_c, 4, "ADD A, C"),
    (noimpl, 4, "! UNIMP !"),
    (add_a_e, 4, "ADD A, E"),
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
    (sub_a_a, 4, "SUB A, A"),
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
    (xor_a_mhl, 8, "XOR A, [HL]"),
    (xor_a_a, 4, "XOR A, A"),
    // 0xb opcodes
    (or_a_b, 4, "OR A, B"),
    (or_a_c, 4, "OR A, C"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (or_a_a, 4, "OR A, A"),
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
    (jp_nz_u16, 12, "JP NZ, u16"),
    (jp_u16, 16, "JP u16"),
    (call_nz_u16, 12, "CALL NZ, u16"),
    (push_bc, 16, "PUSH BC"),
    (add_a_u8, 8, "ADD A, u8"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (ret, 16, "RET"),
    (jp_z_u16, 12, "JP Z, u16"),
    (noimpl, 4, "! UNIMP !"),
    (call_z_u16, 12, "CALL Z, u16"),
    (call_u16, 24, "CALL u16"),
    (noimpl, 4, "! UNIMP !"),
    (rst_08h, 16, "RST 08h"),
    // 0xd opcodes
    (ret_nc, 8, "RET NC"),
    (pop_de, 12, "POP DE"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (push_de, 16, "PUSH DE"),
    (sub_a_u8, 8, "SUB A, u8"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (reti, 16, "RETI"),
    (jp_c_u16, 12, "JP C, u16"),
    (noimpl, 4, "! UNIMP !"),
    (call_c_u16, 12, "CALL C, u16"),
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
    (xor_a_u8, 8, "XOR A, u8"),
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

pub const EXTENDED: [(fn(&mut Cpu), u8, &'static str); 256] = [
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
    (rr_c, 8, "RR C"),
    (rr_d, 8, "RR D"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0x2 opcodes
    (sla_b, 8, "SLA B"),
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
    (srl_b, 8, "SRL B"),
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
    (bit_0_d, 8, "BIT 0, D"),
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
    // 0xb opcodes
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
    (res_7_mhl, 16, "RES 7, [HL]"),
    (noimpl, 4, "! UNIMP !"),
    // 0xc opcodes
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
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (set_4_a, 8, "SET 4, A"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    (noimpl, 4, "! UNIMP !"),
    // 0xf opcodes
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

fn ld_a_mbc(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.bc());
    cpu.a = byte;
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

fn dec_de(cpu: &mut Cpu) {
    cpu.set_de(cpu.de().wrapping_sub(1));
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

fn rra(cpu: &mut Cpu) {
    let carry = cpu.get_carry();

    cpu.set_carry(cpu.a & 0x01 == 0x01);

    cpu.a = cpu.a >> 1 | ((carry as u8) << 7);

    cpu.set_sub(false);
    cpu.set_zero(false);
    cpu.set_half_carry(false);
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

fn dec_h(cpu: &mut Cpu) {
    let h = cpu.h;
    let value = h.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((h & 0xf) == 0xf);

    cpu.h = value;
}

fn ld_h_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.h = byte;
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

fn dec_l(cpu: &mut Cpu) {
    let l = cpu.l;
    let value = l.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((l & 0xf) == 0xf);

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
    let byte = cpu.read_u8() as i8;

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

fn jr_c_i8(cpu: &mut Cpu) {
    let byte = cpu.read_u8() as i8;

    if !cpu.get_carry() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.ticks = cpu.ticks.wrapping_add(4);
}

fn inc_a(cpu: &mut Cpu) {
    let value = cpu.a.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((value & 0xf) == 0xf);

    cpu.a = value;
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

fn ld_b_b(cpu: &mut Cpu) {
    cpu.b = cpu.b;
}

fn ld_b_c(cpu: &mut Cpu) {
    cpu.b = cpu.c;
}

fn ld_b_d(cpu: &mut Cpu) {
    cpu.b = cpu.d;
}

fn ld_b_e(cpu: &mut Cpu) {
    cpu.b = cpu.e;
}

fn ld_b_h(cpu: &mut Cpu) {
    cpu.b = cpu.h;
}

fn ld_b_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.b = byte;
}

fn ld_b_a(cpu: &mut Cpu) {
    cpu.b = cpu.a;
}

fn ld_c_b(cpu: &mut Cpu) {
    cpu.c = cpu.b;
}

fn ld_c_c(cpu: &mut Cpu) {
    cpu.c = cpu.c;
}

fn ld_c_d(cpu: &mut Cpu) {
    cpu.c = cpu.d;
}

fn ld_c_e(cpu: &mut Cpu) {
    cpu.c = cpu.e;
}

fn ld_c_h(cpu: &mut Cpu) {
    cpu.c = cpu.h;
}

fn ld_c_l(cpu: &mut Cpu) {
    cpu.c = cpu.l;
}

fn ld_c_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.c = byte;
}

fn ld_c_a(cpu: &mut Cpu) {
    cpu.c = cpu.a;
}

fn ld_d_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.d = byte;
}

fn ld_d_a(cpu: &mut Cpu) {
    cpu.d = cpu.a;
}

fn ld_e_a(cpu: &mut Cpu) {
    cpu.e = cpu.a;
}

fn ld_h_a(cpu: &mut Cpu) {
    cpu.h = cpu.a;
}

fn ld_mhl_b(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.b);
}

fn ld_mhl_c(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.c);
}

fn ld_mhl_d(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.d);
}

fn halt(cpu: &mut Cpu) {
    cpu.halt();
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

fn ld_a_d(cpu: &mut Cpu) {
    cpu.a = cpu.d;
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

fn ld_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = byte;
}

fn add_a_b(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.b);
}

fn add_a_c(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.c);
}

fn add_a_e(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.e);
}

fn add_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = add_set_flags(cpu, cpu.a, byte);
}

fn sub_a_b(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.b);
}

fn sub_a_a(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.a);
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

fn xor_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a ^= byte;

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

fn or_a_a(cpu: &mut Cpu) {
    cpu.a |= cpu.a;

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

fn jp_nz_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if cpu.get_zero() {
        return;
    }

    cpu.pc = word;
    cpu.ticks = cpu.ticks.wrapping_add(4);
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

fn add_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = add_set_flags(cpu, cpu.a, byte);
}

fn ret(cpu: &mut Cpu) {
    cpu.pc = cpu.pop_word();
}

fn jp_z_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.get_zero() {
        return;
    }

    cpu.pc = word;
    cpu.ticks = cpu.ticks.wrapping_add(4);
}

fn call_z_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.get_zero() {
        return;
    }

    cpu.push_word(cpu.pc);
    cpu.pc = word;
    cpu.ticks = cpu.ticks.wrapping_add(12);
}

fn call_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.push_word(cpu.pc);
    cpu.pc = word;
}

fn rst_08h(cpu: &mut Cpu) {
    rst(cpu, 0x0008);
}

fn ret_nc(cpu: &mut Cpu) {
    if cpu.get_carry() {
        return;
    }

    cpu.pc = cpu.pop_word();
    cpu.ticks = cpu.ticks.wrapping_add(12);
}

fn pop_de(cpu: &mut Cpu) {
    let word = cpu.pop_word();
    cpu.set_de(word);
}

fn push_de(cpu: &mut Cpu) {
    cpu.push_word(cpu.de());
}

fn sub_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = sub_set_flags(cpu, cpu.a, byte);
}

fn reti(cpu: &mut Cpu) {
    cpu.pc = cpu.pop_word();
    cpu.enable_int();
}

fn jp_c_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.get_carry() {
        return;
    }

    cpu.pc = word;
    cpu.ticks = cpu.ticks.wrapping_add(4);
}

fn call_c_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.get_carry() {
        return;
    }

    cpu.push_word(cpu.pc);
    cpu.pc = word;
    cpu.ticks = cpu.ticks.wrapping_add(12);
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

fn xor_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a ^= byte;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
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

fn rr_c(cpu: &mut Cpu) {
    cpu.c = rr(cpu, cpu.c);
}

fn rr_d(cpu: &mut Cpu) {
    cpu.d = rr(cpu, cpu.d);
}

fn sla_b(cpu: &mut Cpu) {
    cpu.b = sla(cpu, cpu.b);
}

fn swap_a(cpu: &mut Cpu) {
    cpu.a = swap(cpu, cpu.a)
}

fn srl_b(cpu: &mut Cpu) {
    cpu.b = srl(cpu, cpu.b);
}

fn bit_0_d(cpu: &mut Cpu) {
    bit_d(cpu, 0);
}

fn bit_7_h(cpu: &mut Cpu) {
    bit_h(cpu, 7);
}

fn res_7_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 7);
    cpu.mmu.write(hl, value);
}

fn set_4_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 4);
}

/// Helper function to set one bit in a u8.
fn set(value: u8, bit: u8) -> u8 {
    value | (1u8 << (bit as usize))
}

/// Helper function to clear one bit in a u8
fn res(value: u8, bit: u8) -> u8 {
    value & !(1u8 << (bit as usize))
}

/// Helper function that rotates (shifts) left the given
/// byte (probably from a register) and updates the
/// proper flag registers.
fn rl(cpu: &mut Cpu, value: u8) -> u8 {
    let carry = cpu.get_carry();

    cpu.set_carry(value & 0x80 == 0x80);

    let result = (value << 1) | carry as u8;

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);

    result
}

/// Helper function that rotates (shifts) right the given
/// byte (probably from a register) and updates the
/// proper flag registers.
fn rr(cpu: &mut Cpu, value: u8) -> u8 {
    let carry = cpu.get_carry();

    cpu.set_carry(value & 0x01 == 0x01);

    let result = (value >> 1) | ((carry as u8) << 7);

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

fn bit_d(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.d, bit));
    cpu.set_half_carry(true);
}

fn bit_h(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.h, bit));
    cpu.set_half_carry(true);
}

fn add_set_flags(cpu: &mut Cpu, first: u8, second: u8) -> u8 {
    let first = first as u32;
    let second = second as u32;

    let result = first.wrapping_add(second);
    let result_b = result as u8;

    cpu.set_sub(false);
    cpu.set_zero(result_b == 0);
    cpu.set_half_carry((first ^ second ^ result) & 0x10 == 0x10);
    cpu.set_carry(result & 0x100 == 0x100);

    result_b
}

fn sub_set_flags(cpu: &mut Cpu, first: u8, second: u8) -> u8 {
    let first = first as u32;
    let second = second as u32;

    let result = first.wrapping_sub(second);
    let result_b = result as u8;

    cpu.set_sub(true);
    cpu.set_zero(result_b == 0);
    cpu.set_half_carry((first ^ second ^ result) & 0x10 == 0x10);
    cpu.set_carry(result & 0x100 == 0x100);

    result_b
}

fn add_u16_u16(cpu: &mut Cpu, first: u16, second: u16) -> u16 {
    let first = first as u32;
    let second = second as u32;
    let result = first.wrapping_add(second);

    cpu.set_sub(false);
    cpu.set_half_carry((first ^ second ^ result) & 0x1000 == 0x1000);
    cpu.set_carry(result & 0x10000 == 0x10000);

    result as u16
}

fn swap(cpu: &mut Cpu, value: u8) -> u8 {
    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);

    (value << 4) | (value >> 4)
}

/// Helper function to shift an `u8` to the left and update CPU
/// flags.
fn sla(cpu: &mut Cpu, value: u8) -> u8 {
    let result = value << 1;

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(value & 0x80 != 0);

    result
}

fn srl(cpu: &mut Cpu, value: u8) -> u8 {
    let result = value >> 1;

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(value & 0x01 == 0x01);

    result
}

/// Helper function for RST instructions, pushes the
/// current PC to the stack and jumps to the provided
/// address.
fn rst(cpu: &mut Cpu, addr: u16) {
    cpu.push_word(cpu.pc);
    cpu.pc = addr;
}
