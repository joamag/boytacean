//! ISA (instruction set architecture) implementation for the [Sharp LR35902](https://en.wikipedia.org/wiki/Game_Boy) CPU.

use crate::cpu::Cpu;

pub const INSTRUCTIONS: [Instruction; 256] = [
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
    (rrca, 4, "RRCA"),
    // 0x1 opcodes
    (stop, 4, "STOP"),
    (ld_de_u16, 12, "LD DE, u16"),
    (ld_mde_a, 8, "LD [DE], A"),
    (inc_de, 8, "INC DE"),
    (inc_d, 4, "INC D"),
    (dec_d, 4, "DEC D"),
    (ld_d_u8, 8, "LD D, u8"),
    (rla, 4, "RLA"),
    (jr_i8, 12, "JR i8"),
    (add_hl_de, 8, "ADD HL, DE"),
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
    (daa, 4, "DAA"),
    (jr_z_i8, 8, "JR Z, i8"),
    (add_hl_hl, 8, "ADD HL, HL"),
    (ld_a_mhli, 8, "LD A, [HL+] "),
    (dec_hl, 8, "DEC HL"),
    (inc_l, 4, "INC L"),
    (dec_l, 4, "DEC L"),
    (ld_l_u8, 8, "LD L, u8"),
    (cpl, 4, "CPL"),
    // 0x3 opcodes
    (jr_nc_i8, 8, "JR NC, i8"),
    (ld_sp_u16, 12, "LD SP, u16"),
    (ld_mhld_a, 8, "LD [HL-], A"),
    (inc_sp, 8, "INC SP"),
    (inc_mhl, 12, "INC [HL]"),
    (dec_mhl, 12, "DEC [HL]"),
    (ld_mhl_u8, 12, "LD [HL], u8 "),
    (scf, 4, "SCF"),
    (jr_c_i8, 8, "JR C, i8"),
    (add_hl_sp, 8, "ADD HL, SP"),
    (ld_a_mhld, 8, "LD A, [HL-]"),
    (dec_sp, 8, "DEC SP"),
    (inc_a, 4, "INC A"),
    (dec_a, 4, "DEC A"),
    (ld_a_u8, 8, "LD A, u8"),
    (ccf, 4, "CCF"),
    // 0x4 opcodes
    (ld_b_b, 4, "LD B, B"),
    (ld_b_c, 4, "LD B, C"),
    (ld_b_d, 4, "LD B, D"),
    (ld_b_e, 4, "LD B, E"),
    (ld_b_h, 4, "LD B, H"),
    (ld_b_l, 4, "LD B, L"),
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
    (ld_d_b, 4, "LD D, B"),
    (ld_d_c, 4, "LD D, C"),
    (ld_d_d, 4, "LD D, D"),
    (ld_d_e, 4, "LD D, E"),
    (ld_d_h, 4, "LD D, H"),
    (ld_d_l, 4, "LD D, L"),
    (ld_d_mhl, 8, "LD D, [HL]"),
    (ld_d_a, 4, "LD D, A"),
    (ld_e_b, 4, "LD E, B"),
    (ld_e_c, 4, "LD E, C"),
    (ld_e_d, 4, "LD E, D"),
    (ld_e_e, 4, "LD E, E"),
    (ld_e_h, 4, "LD E, H"),
    (ld_e_l, 4, "LD E, L"),
    (ld_e_mhl, 8, "LD E, [HL]"),
    (ld_e_a, 4, "LD E, A"),
    // 0x6 opcodes
    (ld_h_b, 4, "LD H, B"),
    (ld_h_c, 4, "LD H, C"),
    (ld_h_d, 4, "LD H, D"),
    (ld_h_e, 4, "LD H, E"),
    (ld_h_h, 4, "LD H, H"),
    (ld_h_l, 4, "LD H, L"),
    (ld_h_mhl, 8, "LD H, [HL]"),
    (ld_h_a, 4, "LD H, A"),
    (ld_l_b, 4, "LD L, B"),
    (ld_l_c, 4, "LD L, C"),
    (ld_l_d, 4, "LD L, D"),
    (ld_l_e, 4, "LD L, E"),
    (ld_l_h, 4, "LD L, H"),
    (ld_l_l, 4, "LD L, L"),
    (ld_l_mhl, 8, "LD L, [HL]"),
    (ld_l_a, 4, "LD L, A"),
    // 0x7 opcodes
    (ld_mhl_b, 8, "LD [HL], B"),
    (ld_mhl_c, 8, "LD [HL], C"),
    (ld_mhl_d, 8, "LD [HL], D"),
    (ld_mhl_e, 8, "LD [HL], E"),
    (ld_mhl_h, 8, "LD [HL], H"),
    (ld_mhl_l, 8, "LD [HL], L"),
    (halt, 4, "HALT"),
    (ld_mhl_a, 8, "LD [HL], A"),
    (ld_a_b, 4, "LD A, B"),
    (ld_a_c, 4, "LD A, C"),
    (ld_a_d, 4, "LD A, D"),
    (ld_a_e, 4, "LD A, E"),
    (ld_a_h, 4, "LD A, H"),
    (ld_a_l, 4, "LD A, L"),
    (ld_a_mhl, 8, "LD A, [HL]"),
    (ld_a_a, 4, "LD A, A"),
    // 0x8 opcodes
    (add_a_b, 4, "ADD A, B"),
    (add_a_c, 4, "ADD A, C"),
    (add_a_d, 4, "ADD A, D"),
    (add_a_e, 4, "ADD A, E"),
    (add_a_h, 4, "ADD A, H"),
    (add_a_l, 4, "ADD A, L"),
    (add_a_mhl, 8, "ADD A, [HL]"),
    (add_a_a, 4, "ADD A, A"),
    (adc_a_b, 4, "ADC A, B"),
    (adc_a_c, 4, "ADC A, C"),
    (adc_a_d, 4, "ADC A, D"),
    (adc_a_e, 4, "ADC A, E"),
    (adc_a_h, 4, "ADC A, H"),
    (adc_a_l, 4, "ADC A, L"),
    (adc_a_mhl, 8, "ADC A, [HL]"),
    (adc_a_a, 4, "ADC A, A"),
    // 0x9 opcodes
    (sub_a_b, 4, "SUB A, B"),
    (sub_a_c, 4, "SUB A, C"),
    (sub_a_d, 4, "SUB A, D"),
    (sub_a_e, 4, "SUB A, E"),
    (sub_a_h, 4, "SUB A, H"),
    (sub_a_l, 4, "SUB A, L"),
    (sub_a_mhl, 8, "SUB A, [HL]"),
    (sub_a_a, 4, "SUB A, A"),
    (sbc_a_b, 4, "SBC A, B"),
    (sbc_a_c, 4, "SBC A, C"),
    (sbc_a_d, 4, "SBC A, D"),
    (sbc_a_e, 4, "SBC A, E"),
    (sbc_a_h, 4, "SBC A, H"),
    (sbc_a_l, 4, "SBC A, L"),
    (sbc_a_mhl, 8, "SBC A, [HL]"),
    (sbc_a_a, 4, "SBC A, A"),
    // 0xA opcodes
    (and_a_b, 4, "AND A, B"),
    (and_a_c, 4, "AND A, C"),
    (and_a_d, 4, "AND A, D"),
    (and_a_e, 4, "AND A, E"),
    (and_a_h, 4, "AND A, H"),
    (and_a_l, 4, "AND A, L"),
    (and_a_mhl, 8, "AND A, [HL]"),
    (and_a_a, 4, "AND A, A"),
    (xor_a_b, 4, "XOR A, B"),
    (xor_a_c, 4, "XOR A, C"),
    (xor_a_d, 4, "XOR A, D"),
    (xor_a_e, 4, "XOR A, E"),
    (xor_a_h, 4, "XOR A, H"),
    (xor_a_l, 4, "XOR A, L"),
    (xor_a_mhl, 8, "XOR A, [HL]"),
    (xor_a_a, 4, "XOR A, A"),
    // 0xB opcodes
    (or_a_b, 4, "OR A, B"),
    (or_a_c, 4, "OR A, C"),
    (or_a_d, 4, "OR A, D"),
    (or_a_e, 4, "OR A, E"),
    (or_a_h, 4, "OR A, H"),
    (or_a_l, 4, "OR A, L"),
    (or_a_mhl, 8, "OR A, [HL]"),
    (or_a_a, 4, "OR A, A"),
    (cp_a_b, 4, "CP A, B"),
    (cp_a_c, 4, "CP A, C"),
    (cp_a_d, 4, "CP A, D"),
    (cp_a_e, 4, "CP A, E"),
    (cp_a_h, 4, "CP A, H"),
    (cp_a_l, 4, "CP A, L"),
    (cp_a_mhl, 8, "CP A, [HL]"),
    (cp_a_a, 4, "CP A, A"),
    // 0xC opcodes
    (ret_nz, 8, "RET NZ"),
    (pop_bc, 12, "POP BC"),
    (jp_nz_u16, 12, "JP NZ, u16"),
    (jp_u16, 16, "JP u16"),
    (call_nz_u16, 12, "CALL NZ, u16"),
    (push_bc, 16, "PUSH BC"),
    (add_a_u8, 8, "ADD A, u8"),
    (rst_00h, 16, "RST 00h"),
    (ret_z, 8, "RET Z"),
    (ret, 16, "RET"),
    (jp_z_u16, 12, "JP Z, u16"),
    (illegal, 4, "ILLEGAL"),
    (call_z_u16, 12, "CALL Z, u16"),
    (call_u16, 24, "CALL u16"),
    (adc_a_u8, 8, "ADC A, u8 "),
    (rst_08h, 16, "RST 08h"),
    // 0xD opcodes
    (ret_nc, 8, "RET NC"),
    (pop_de, 12, "POP DE"),
    (jp_nc_u16, 12, "JP NC, u16"),
    (illegal, 4, "ILLEGAL"),
    (call_nc_u16, 12, "CALL NC, u16 "),
    (push_de, 16, "PUSH DE"),
    (sub_a_u8, 8, "SUB A, u8"),
    (rst_10h, 16, "RST 10h"),
    (ret_c, 8, "RET C"),
    (reti, 16, "RETI"),
    (jp_c_u16, 12, "JP C, u16"),
    (illegal, 4, "ILLEGAL"),
    (call_c_u16, 12, "CALL C, u16"),
    (illegal, 4, "ILLEGAL"),
    (sbc_a_u8, 8, "SBC A, u8"),
    (rst_18h, 16, "RST 18h"),
    // 0xE opcodes
    (ld_mff00u8_a, 12, "LD [FF00+u8], A"),
    (pop_hl, 12, "POP HL"),
    (ld_mff00c_a, 8, "LD [FF00+C], A"),
    (illegal, 4, "ILLEGAL"),
    (illegal, 4, "ILLEGAL"),
    (push_hl, 16, "PUSH HL"),
    (and_a_u8, 8, "AND A, u8"),
    (rst_20h, 16, "RST 20h"),
    (add_sp_i8, 16, "ADD SP, i8"),
    (jp_hl, 4, "JP HL"),
    (ld_mu16_a, 16, "LD [u16], A"),
    (illegal, 4, "ILLEGAL"),
    (illegal, 4, "ILLEGAL"),
    (illegal, 4, "ILLEGAL"),
    (xor_a_u8, 8, "XOR A, u8"),
    (rst_28h, 16, "RST 28h"),
    // 0xF opcodes
    (ld_a_mff00u8, 12, "LD A, [FF00+u8]"),
    (pop_af, 12, "POP AF"),
    (ld_a_mff00c, 8, "LD A, [FF00+C]"),
    (di, 4, "DI"),
    (illegal, 4, "ILLEGAL"),
    (push_af, 16, "PUSH AF"),
    (or_a_u8, 8, "OR A, u8"),
    (rst_30h, 16, "RST 30h"),
    (ld_hl_spi8, 12, "LD HL, SP+i8"),
    (ld_sp_hl, 8, "LD SP, HL"),
    (ld_a_mu16, 16, "LD A [u16]"),
    (ei, 4, "EI"),
    (illegal, 4, "ILLEGAL"),
    (illegal, 4, "ILLEGAL"),
    (cp_a_u8, 8, "CP A, u8"),
    (rst_38h, 16, "RST 38h"),
];

pub const EXTENDED: [Instruction; 256] = [
    // 0x0 opcodes
    (rlc_b, 8, "RLC B"),
    (rlc_c, 8, "RLC C"),
    (rlc_d, 8, "RLC D"),
    (rlc_e, 8, "RLC E"),
    (rlc_h, 8, "RLC H"),
    (rlc_l, 8, "RLC L"),
    (rlc_mhl, 16, "RLC [HL]"),
    (rlc_a, 8, "RLC A"),
    (rrc_b, 8, "RRC B"),
    (rrc_c, 8, "RRC C"),
    (rrc_d, 8, "RRC D"),
    (rrc_e, 8, "RRC E"),
    (rrc_h, 8, "RRC H"),
    (rrc_l, 8, "RRC L"),
    (rrc_mhl, 16, "RRC [HL]"),
    (rrc_a, 8, "RRC A"),
    // 0x1 opcodes
    (rl_b, 8, "RL B"),
    (rl_c, 8, "RL C"),
    (rl_d, 8, "RL D"),
    (rl_e, 8, "RL E"),
    (rl_h, 8, "RL H"),
    (rl_l, 8, "RL L"),
    (rl_mhl, 16, "RL [HL]"),
    (rl_a, 8, "RL A"),
    (rr_b, 8, "RR B"),
    (rr_c, 8, "RR C"),
    (rr_d, 8, "RR D"),
    (rr_e, 8, "RR E"),
    (rr_h, 8, "RR H"),
    (rr_l, 8, "RR L"),
    (rr_mhl, 16, "RR [HL]"),
    (rr_a, 8, "RR A"),
    // 0x2 opcodes
    (sla_b, 8, "SLA B"),
    (sla_c, 8, "SLA C"),
    (sla_d, 8, "SLA D"),
    (sla_e, 8, "SLA E"),
    (sla_h, 8, "SLA H"),
    (sla_l, 8, "SLA L"),
    (sla_mhl, 16, "SLA [HL]"),
    (sla_a, 8, "SLA A"),
    (sra_b, 8, "SRA B"),
    (sra_c, 8, "SRA C"),
    (sra_d, 8, "SRA D"),
    (sra_e, 8, "SRA E"),
    (sra_h, 8, "SRA H"),
    (sra_l, 8, "SRA L"),
    (sra_mhl, 16, "SRA [HL]"),
    (sra_a, 8, "SRA A"),
    // 0x3 opcodes
    (swap_b, 8, "SWAP B"),
    (swap_c, 8, "SWAP C"),
    (swap_d, 8, "SWAP D"),
    (swap_e, 8, "SWAP E"),
    (swap_h, 8, "SWAP H"),
    (swap_l, 8, "SWAP L"),
    (swap_mhl, 16, "SWAP [HL]"),
    (swap_a, 8, "SWAP A"),
    (srl_b, 8, "SRL B"),
    (srl_c, 8, "SRL B"),
    (srl_d, 8, "SRL D"),
    (srl_e, 8, "SRL E"),
    (srl_h, 8, "SRL H"),
    (srl_l, 8, "SRL L"),
    (srl_mhl, 16, "SRL [HL]"),
    (srl_a, 8, "SRL A"),
    // 0x4 opcodes
    (bit_0_b, 8, "BIT 0, B"),
    (bit_0_c, 8, "BIT 0, C"),
    (bit_0_d, 8, "BIT 0, D"),
    (bit_0_e, 8, "BIT 0, E"),
    (bit_0_h, 8, "BIT 0, H"),
    (bit_0_l, 8, "BIT 0, L"),
    (bit_0_mhl, 12, "BIT 0, [HL]"),
    (bit_0_a, 8, "BIT 0, A"),
    (bit_1_b, 8, "BIT 1, B"),
    (bit_1_c, 8, "BIT 1, C"),
    (bit_1_d, 8, "BIT 1, D"),
    (bit_1_e, 8, "BIT 1, E"),
    (bit_1_h, 8, "BIT 1, H"),
    (bit_1_l, 8, "BIT 1, L"),
    (bit_1_mhl, 12, "BIT 1, [HL]"),
    (bit_1_a, 8, "BIT 1, A"),
    // 0x5 opcodes
    (bit_2_b, 8, "BIT 2, B"),
    (bit_2_c, 8, "BIT 2, C"),
    (bit_2_d, 8, "BIT 2, D"),
    (bit_2_e, 8, "BIT 2, E"),
    (bit_2_h, 8, "BIT 2, H"),
    (bit_2_l, 8, "BIT 2, L"),
    (bit_2_mhl, 12, "BIT 2, [HL]"),
    (bit_2_a, 8, "BIT 2, A"),
    (bit_3_b, 8, "BIT 3, B"),
    (bit_3_c, 8, "BIT 3, C"),
    (bit_3_d, 8, "BIT 3, D"),
    (bit_3_e, 8, "BIT 3, E"),
    (bit_3_h, 8, "BIT 3, H"),
    (bit_3_l, 8, "BIT 3, L"),
    (bit_3_mhl, 12, "BIT 3, [HL]"),
    (bit_3_a, 8, "BIT 3, A"),
    // 0x6 opcodes
    (bit_4_b, 8, "BIT 4, B"),
    (bit_4_c, 8, "BIT 4, C"),
    (bit_4_d, 8, "BIT 4, D"),
    (bit_4_e, 8, "BIT 4, E"),
    (bit_4_h, 8, "BIT 4, H"),
    (bit_4_l, 8, "BIT 4, L"),
    (bit_4_mhl, 12, "BIT 4, [HL]"),
    (bit_4_a, 8, "BIT 4, A"),
    (bit_5_b, 8, "BIT 5, B"),
    (bit_5_c, 8, "BIT 5, C"),
    (bit_5_d, 8, "BIT 5, D"),
    (bit_5_e, 8, "BIT 5, E"),
    (bit_5_h, 8, "BIT 5, H"),
    (bit_5_l, 8, "BIT 5, L"),
    (bit_5_mhl, 12, "BIT 5, [HL]"),
    (bit_5_a, 8, "BIT 5, A"),
    // 0x7 opcodes
    (bit_6_b, 8, "BIT 6, B"),
    (bit_6_c, 8, "BIT 6, C"),
    (bit_6_d, 8, "BIT 6, D"),
    (bit_6_e, 8, "BIT 6, E"),
    (bit_6_h, 8, "BIT 6, H"),
    (bit_6_l, 8, "BIT 6, L"),
    (bit_6_mhl, 12, "BIT 6, [HL]"),
    (bit_6_a, 8, "BIT 6, A"),
    (bit_7_b, 8, "BIT 7, B"),
    (bit_7_c, 8, "BIT 7, C"),
    (bit_7_d, 8, "BIT 7, D"),
    (bit_7_e, 8, "BIT 7, E"),
    (bit_7_h, 8, "BIT 7, H"),
    (bit_7_l, 8, "BIT 7, L"),
    (bit_7_mhl, 12, "BIT 7, [HL]"),
    (bit_7_a, 8, "BIT 7, A"),
    // 0x8 opcodes
    (res_0_b, 8, "RES 0, B"),
    (res_0_c, 8, "RES 0, C"),
    (res_0_d, 8, "RES 0, D"),
    (res_0_e, 8, "RES 0, E"),
    (res_0_h, 8, "RES 0, H"),
    (res_0_l, 8, "RES 0, L"),
    (res_0_mhl, 16, "RES 0, A"),
    (res_0_a, 8, "RES 0, A"),
    (res_1_b, 8, "RES 1, B"),
    (res_1_c, 8, "RES 1, C"),
    (res_1_d, 8, "RES 1, D"),
    (res_1_e, 8, "RES 1, E"),
    (res_1_h, 8, "RES 1, H"),
    (res_1_l, 8, "RES 1, L"),
    (res_1_mhl, 16, "RES 1, A"),
    (res_1_a, 8, "RES 1, A"),
    // 0x9 opcodes
    (res_2_b, 8, "RES 2, B"),
    (res_2_c, 8, "RES 2, C"),
    (res_2_d, 8, "RES 2, D"),
    (res_2_e, 8, "RES 2, E"),
    (res_2_h, 8, "RES 2, H"),
    (res_2_l, 8, "RES 2, L"),
    (res_2_mhl, 16, "RES 2, A"),
    (res_2_a, 8, "RES 2, A"),
    (res_3_b, 8, "RES 3, B"),
    (res_3_c, 8, "RES 3, C"),
    (res_3_d, 8, "RES 3, D"),
    (res_3_e, 8, "RES 3, E"),
    (res_3_h, 8, "RES 3, H"),
    (res_3_l, 8, "RES 3, L"),
    (res_3_mhl, 16, "RES 3, A"),
    (res_3_a, 8, "RES 3, A"),
    // 0xA opcodes
    (res_4_b, 8, "RES 4, B"),
    (res_4_c, 8, "RES 4, C"),
    (res_4_d, 8, "RES 4, D"),
    (res_4_e, 8, "RES 4, E"),
    (res_4_h, 8, "RES 4, H"),
    (res_4_l, 8, "RES 4, L"),
    (res_4_mhl, 16, "RES 4, A"),
    (res_4_a, 8, "RES 4, A"),
    (res_5_b, 8, "RES 5, B"),
    (res_5_c, 8, "RES 5, C"),
    (res_5_d, 8, "RES 5, D"),
    (res_5_e, 8, "RES 5, E"),
    (res_5_h, 8, "RES 5, H"),
    (res_5_l, 8, "RES 5, L"),
    (res_5_mhl, 16, "RES 5, A"),
    (res_5_a, 8, "RES 5, A"),
    // 0xB opcodes
    (res_6_b, 8, "RES 6, B"),
    (res_6_c, 8, "RES 6, C"),
    (res_6_d, 8, "RES 6, D"),
    (res_6_e, 8, "RES 6, E"),
    (res_6_h, 8, "RES 6, H"),
    (res_6_l, 8, "RES 6, L"),
    (res_6_mhl, 16, "RES 6, A"),
    (res_6_a, 8, "RES 6, A"),
    (res_7_b, 8, "RES 7, B"),
    (res_7_c, 8, "RES 7, C"),
    (res_7_d, 8, "RES 7, D"),
    (res_7_e, 8, "RES 7, E"),
    (res_7_h, 8, "RES 7, H"),
    (res_7_l, 8, "RES 7, L"),
    (res_7_mhl, 16, "RES 7, A"),
    (res_7_a, 8, "RES 7, A"),
    // 0xC opcodes
    (set_0_b, 8, "SET 0, B"),
    (set_0_c, 8, "SET 0, C"),
    (set_0_d, 8, "SET 0, D"),
    (set_0_e, 8, "SET 0, E"),
    (set_0_h, 8, "SET 0, H"),
    (set_0_l, 8, "SET 0, L"),
    (set_0_mhl, 16, "SET 0, [HL]"),
    (set_0_a, 8, "SET 0, A"),
    (set_1_b, 8, "SET 1, B"),
    (set_1_c, 8, "SET 1, C"),
    (set_1_d, 8, "SET 1, D"),
    (set_1_e, 8, "SET 1, E"),
    (set_1_h, 8, "SET 1, H"),
    (set_1_l, 8, "SET 1, L"),
    (set_1_mhl, 16, "SET 1, [HL]"),
    (set_1_a, 8, "SET 1, A"),
    // 0xD opcodes
    (set_2_b, 8, "SET 2, B"),
    (set_2_c, 8, "SET 2, C"),
    (set_2_d, 8, "SET 2, D"),
    (set_2_e, 8, "SET 2, E"),
    (set_2_h, 8, "SET 2, H"),
    (set_2_l, 8, "SET 2, L"),
    (set_2_mhl, 16, "SET 2, [HL]"),
    (set_2_a, 8, "SET 2, A"),
    (set_3_b, 8, "SET 3, B"),
    (set_3_c, 8, "SET 3, C"),
    (set_3_d, 8, "SET 3, D"),
    (set_3_e, 8, "SET 3, E"),
    (set_3_h, 8, "SET 3, H"),
    (set_3_l, 8, "SET 3, L"),
    (set_3_mhl, 16, "SET 3, [HL]"),
    (set_3_a, 8, "SET 3, A"),
    // 0xE opcodes
    (set_4_b, 8, "SET 4, B"),
    (set_4_c, 8, "SET 4, C"),
    (set_4_d, 8, "SET 4, D"),
    (set_4_e, 8, "SET 4, E"),
    (set_4_h, 8, "SET 4, H"),
    (set_4_l, 8, "SET 4, L"),
    (set_4_mhl, 16, "SET 4, [HL]"),
    (set_4_a, 8, "SET 4, A"),
    (set_5_b, 8, "SET 5, B"),
    (set_5_c, 8, "SET 5, C"),
    (set_5_d, 8, "SET 5, D"),
    (set_5_e, 8, "SET 5, E"),
    (set_5_h, 8, "SET 5, H"),
    (set_5_l, 8, "SET 5, L"),
    (set_5_mhl, 16, "SET 5, [HL]"),
    (set_5_a, 8, "SET 5, A"),
    // 0xF opcodes
    (set_6_b, 8, "SET 6, B"),
    (set_6_c, 8, "SET 6, C"),
    (set_6_d, 8, "SET 6, D"),
    (set_6_e, 8, "SET 6, E"),
    (set_6_h, 8, "SET 6, H"),
    (set_6_l, 8, "SET 6, L"),
    (set_6_mhl, 16, "SET 6, [HL]"),
    (set_6_a, 8, "SET 6, A"),
    (set_7_b, 8, "SET 7, B"),
    (set_7_c, 8, "SET 7, C"),
    (set_7_d, 8, "SET 7, D"),
    (set_7_e, 8, "SET 7, E"),
    (set_7_h, 8, "SET 7, H"),
    (set_7_l, 8, "SET 7, L"),
    (set_7_mhl, 16, "SET 7, [HL]"),
    (set_7_a, 8, "SET 7, A"),
];

pub type Instruction = (fn(&mut Cpu), u8, &'static str);

fn nop(_cpu: &mut Cpu) {}

fn illegal(_cpu: &mut Cpu) {
    panic!("Illegal instruction");
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
    let b = cpu.b;
    let value = b.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((b & 0xf) == 0xf);

    cpu.b = value;
}

fn dec_b(cpu: &mut Cpu) {
    let b = cpu.b;
    let value = b.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((b & 0xf) == 0x0);

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
    let c = cpu.c;
    let value = cpu.c.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((c & 0xf) == 0xf);

    cpu.c = value;
}

fn dec_c(cpu: &mut Cpu) {
    let c = cpu.c;
    let value = c.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((c & 0xf) == 0x0);

    cpu.c = value;
}

fn ld_c_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.c = byte;
}

fn rrca(cpu: &mut Cpu) {
    let carry = cpu.a & 0x1;
    cpu.a = (cpu.a >> 1) | (carry << 7);

    cpu.set_sub(false);
    cpu.set_zero(false);
    cpu.set_half_carry(false);
    cpu.set_carry(carry == 0x1);
}

fn stop(cpu: &mut Cpu) {
    cpu.stop();
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
    let d = cpu.d;
    let value = d.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((d & 0xf) == 0xf);

    cpu.d = value;
}

fn dec_d(cpu: &mut Cpu) {
    let d = cpu.d;
    let value = d.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((d & 0xf) == 0x0);

    cpu.d = value;
}

fn ld_d_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.d = byte;
}

fn rla(cpu: &mut Cpu) {
    let carry = cpu.carry();

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

fn add_hl_de(cpu: &mut Cpu) {
    let value = add_u16_u16(cpu, cpu.hl(), cpu.de());
    cpu.set_hl(value);
}

fn ld_a_mde(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.de());
    cpu.a = byte;
}

fn dec_de(cpu: &mut Cpu) {
    cpu.set_de(cpu.de().wrapping_sub(1));
}

fn inc_e(cpu: &mut Cpu) {
    let e = cpu.e;
    let value = cpu.e.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((e & 0xf) == 0xf);

    cpu.e = value;
}

fn dec_e(cpu: &mut Cpu) {
    let e = cpu.e;
    let value = e.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((e & 0xf) == 0x0);

    cpu.e = value;
}

fn ld_e_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.e = byte;
}

fn rra(cpu: &mut Cpu) {
    let carry = cpu.carry();

    cpu.set_carry((cpu.a & 0x1) == 0x1);

    cpu.a = cpu.a >> 1 | ((carry as u8) << 7);

    cpu.set_sub(false);
    cpu.set_zero(false);
    cpu.set_half_carry(false);
}

fn jr_nz_i8(cpu: &mut Cpu) {
    let byte = cpu.read_u8() as i8;

    if cpu.zero() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.cycles = cpu.cycles.wrapping_add(4);
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
    let h = cpu.h;
    let value = cpu.h.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((h & 0xf) == 0xf);

    cpu.h = value;
}

fn dec_h(cpu: &mut Cpu) {
    let h = cpu.h;
    let value = h.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((h & 0xf) == 0x0);

    cpu.h = value;
}

fn ld_h_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.h = byte;
}

fn daa(cpu: &mut Cpu) {
    let a = cpu.a;
    let mut adjust = 0;

    if cpu.half_carry() {
        adjust |= 0x06;
    }

    if cpu.carry() {
        // Yes, we have to adjust it.
        adjust |= 0x60;
    }

    let res = if cpu.sub() {
        a.wrapping_sub(adjust)
    } else {
        if a & 0x0f > 0x09 {
            adjust |= 0x06;
        }

        if a > 0x99 {
            adjust |= 0x60;
        }

        a.wrapping_add(adjust)
    };

    cpu.a = res;

    cpu.set_zero(res == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(adjust & 0x60 == 0x60);
}

fn jr_z_i8(cpu: &mut Cpu) {
    let byte = cpu.read_u8() as i8;

    if !cpu.zero() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.cycles = cpu.cycles.wrapping_add(4);
}

fn add_hl_hl(cpu: &mut Cpu) {
    let value = add_u16_u16(cpu, cpu.hl(), cpu.hl());
    cpu.set_hl(value);
}

fn ld_a_mhli(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = byte;
    cpu.set_hl(cpu.hl().wrapping_add(1));
}

fn dec_hl(cpu: &mut Cpu) {
    cpu.set_hl(cpu.hl().wrapping_sub(1));
}

fn inc_l(cpu: &mut Cpu) {
    let l = cpu.l;
    let value = l.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((l & 0xf) == 0xf);

    cpu.l = value;
}

fn dec_l(cpu: &mut Cpu) {
    let l = cpu.l;
    let value = l.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((l & 0xf) == 0x0);

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

    if cpu.carry() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.cycles = cpu.cycles.wrapping_add(4);
}

fn ld_mhld_a(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.a);
    cpu.set_hl(cpu.hl().wrapping_sub(1));
}

fn inc_sp(cpu: &mut Cpu) {
    cpu.sp = cpu.sp.wrapping_add(1);
}

fn inc_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    let value = byte.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((byte & 0xf) == 0xf);

    cpu.mmu.write(cpu.hl(), value);
}

fn dec_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    let value = byte.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((byte & 0xf) == 0x0);

    cpu.mmu.write(cpu.hl(), value);
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

    if !cpu.carry() {
        return;
    }

    cpu.pc = (cpu.pc as i16).wrapping_add(byte as i16) as u16;
    cpu.cycles = cpu.cycles.wrapping_add(4);
}

fn add_hl_sp(cpu: &mut Cpu) {
    let value = add_u16_u16(cpu, cpu.hl(), cpu.sp());
    cpu.set_hl(value);
}

fn ld_a_mhld(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = byte;
    cpu.set_hl(cpu.hl().wrapping_sub(1));
}

fn dec_sp(cpu: &mut Cpu) {
    cpu.sp = cpu.sp.wrapping_sub(1);
}

fn inc_a(cpu: &mut Cpu) {
    let a = cpu.a;
    let value = a.wrapping_add(1);

    cpu.set_sub(false);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((a & 0xf) == 0xf);

    cpu.a = value;
}

fn dec_a(cpu: &mut Cpu) {
    let a = cpu.a;
    let value = a.wrapping_sub(1);

    cpu.set_sub(true);
    cpu.set_zero(value == 0);
    cpu.set_half_carry((a & 0xf) == 0x0);

    cpu.a = value;
}

fn ld_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = byte;
}

fn ccf(cpu: &mut Cpu) {
    cpu.set_sub(false);
    cpu.set_half_carry(false);
    cpu.set_carry(!cpu.carry());
}

fn ld_b_b(_cpu: &mut Cpu) {}

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

fn ld_b_l(cpu: &mut Cpu) {
    cpu.b = cpu.l;
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

fn ld_c_c(_cpu: &mut Cpu) {}

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

fn ld_d_b(cpu: &mut Cpu) {
    cpu.d = cpu.b;
}

fn ld_d_c(cpu: &mut Cpu) {
    cpu.d = cpu.c;
}

fn ld_d_d(_cpu: &mut Cpu) {}

fn ld_d_e(cpu: &mut Cpu) {
    cpu.d = cpu.e;
}

fn ld_d_h(cpu: &mut Cpu) {
    cpu.d = cpu.h;
}

fn ld_d_l(cpu: &mut Cpu) {
    cpu.d = cpu.l;
}

fn ld_d_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.d = byte;
}

fn ld_d_a(cpu: &mut Cpu) {
    cpu.d = cpu.a;
}

fn ld_e_b(cpu: &mut Cpu) {
    cpu.e = cpu.b;
}

fn ld_e_c(cpu: &mut Cpu) {
    cpu.e = cpu.c;
}

fn ld_e_d(cpu: &mut Cpu) {
    cpu.e = cpu.d;
}

fn ld_e_e(_cpu: &mut Cpu) {}

fn ld_e_h(cpu: &mut Cpu) {
    cpu.e = cpu.h;
}

fn ld_e_l(cpu: &mut Cpu) {
    cpu.e = cpu.l;
}

fn ld_e_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.e = byte;
}

fn ld_e_a(cpu: &mut Cpu) {
    cpu.e = cpu.a;
}

fn ld_h_b(cpu: &mut Cpu) {
    cpu.h = cpu.b;
}

fn ld_h_c(cpu: &mut Cpu) {
    cpu.h = cpu.c;
}

fn ld_h_d(cpu: &mut Cpu) {
    cpu.h = cpu.d;
}

fn ld_h_e(cpu: &mut Cpu) {
    cpu.h = cpu.e;
}

fn ld_h_h(_cpu: &mut Cpu) {}

fn ld_h_l(cpu: &mut Cpu) {
    cpu.h = cpu.l;
}

fn ld_h_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.h = byte;
}

fn ld_h_a(cpu: &mut Cpu) {
    cpu.h = cpu.a;
}

fn ld_l_b(cpu: &mut Cpu) {
    cpu.l = cpu.b;
}

fn ld_l_c(cpu: &mut Cpu) {
    cpu.l = cpu.c;
}

fn ld_l_d(cpu: &mut Cpu) {
    cpu.l = cpu.d;
}

fn ld_l_e(cpu: &mut Cpu) {
    cpu.l = cpu.e;
}

fn ld_l_h(cpu: &mut Cpu) {
    cpu.l = cpu.h;
}

fn ld_l_l(_cpu: &mut Cpu) {}

fn ld_l_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.l = byte;
}

fn ld_l_a(cpu: &mut Cpu) {
    cpu.l = cpu.a;
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

fn ld_mhl_e(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.e);
}

fn ld_mhl_h(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.h);
}

fn ld_mhl_l(cpu: &mut Cpu) {
    cpu.mmu.write(cpu.hl(), cpu.l);
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

fn ld_a_a(_cpu: &mut Cpu) {}

fn add_a_b(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.b);
}

fn add_a_c(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.c);
}

fn add_a_d(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.d);
}

fn add_a_e(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.e);
}

fn add_a_h(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.h);
}

fn add_a_l(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.l);
}

fn add_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = add_set_flags(cpu, cpu.a, byte);
}

fn add_a_a(cpu: &mut Cpu) {
    cpu.a = add_set_flags(cpu, cpu.a, cpu.a);
}

fn adc_a_b(cpu: &mut Cpu) {
    cpu.a = add_carry_set_flags(cpu, cpu.a, cpu.b);
}

fn adc_a_c(cpu: &mut Cpu) {
    cpu.a = add_carry_set_flags(cpu, cpu.a, cpu.c);
}

fn adc_a_d(cpu: &mut Cpu) {
    cpu.a = add_carry_set_flags(cpu, cpu.a, cpu.d);
}

fn adc_a_e(cpu: &mut Cpu) {
    cpu.a = add_carry_set_flags(cpu, cpu.a, cpu.e);
}

fn adc_a_h(cpu: &mut Cpu) {
    cpu.a = add_carry_set_flags(cpu, cpu.a, cpu.h);
}

fn adc_a_l(cpu: &mut Cpu) {
    cpu.a = add_carry_set_flags(cpu, cpu.a, cpu.l);
}

fn adc_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = add_carry_set_flags(cpu, cpu.a, byte);
}

fn adc_a_a(cpu: &mut Cpu) {
    cpu.a = add_carry_set_flags(cpu, cpu.a, cpu.a);
}

fn sub_a_b(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.b);
}

fn sub_a_c(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.c);
}

fn sub_a_d(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.d);
}

fn sub_a_e(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.e);
}

fn sub_a_h(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.h);
}

fn sub_a_l(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.l);
}

fn sub_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = sub_set_flags(cpu, cpu.a, byte);
}

fn sub_a_a(cpu: &mut Cpu) {
    cpu.a = sub_set_flags(cpu, cpu.a, cpu.a);
}

fn sbc_a_b(cpu: &mut Cpu) {
    cpu.a = sub_carry_set_flags(cpu, cpu.a, cpu.b);
}

fn sbc_a_c(cpu: &mut Cpu) {
    cpu.a = sub_carry_set_flags(cpu, cpu.a, cpu.c);
}

fn sbc_a_d(cpu: &mut Cpu) {
    cpu.a = sub_carry_set_flags(cpu, cpu.a, cpu.d);
}

fn sbc_a_e(cpu: &mut Cpu) {
    cpu.a = sub_carry_set_flags(cpu, cpu.a, cpu.e);
}

fn sbc_a_h(cpu: &mut Cpu) {
    cpu.a = sub_carry_set_flags(cpu, cpu.a, cpu.h);
}

fn sbc_a_l(cpu: &mut Cpu) {
    cpu.a = sub_carry_set_flags(cpu, cpu.a, cpu.l);
}

fn sbc_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a = sub_carry_set_flags(cpu, cpu.a, byte);
}

fn sbc_a_a(cpu: &mut Cpu) {
    cpu.a = sub_carry_set_flags(cpu, cpu.a, cpu.a);
}

fn and_a_b(cpu: &mut Cpu) {
    cpu.a &= cpu.b;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn and_a_c(cpu: &mut Cpu) {
    cpu.a &= cpu.c;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn and_a_d(cpu: &mut Cpu) {
    cpu.a &= cpu.d;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn and_a_e(cpu: &mut Cpu) {
    cpu.a &= cpu.e;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn and_a_h(cpu: &mut Cpu) {
    cpu.a &= cpu.h;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn and_a_l(cpu: &mut Cpu) {
    cpu.a &= cpu.l;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn and_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a &= byte;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn and_a_a(cpu: &mut Cpu) {
    cpu.a &= cpu.a;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(true);
    cpu.set_carry(false);
}

fn xor_a_b(cpu: &mut Cpu) {
    cpu.a ^= cpu.b;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn xor_a_c(cpu: &mut Cpu) {
    cpu.a ^= cpu.c;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn xor_a_d(cpu: &mut Cpu) {
    cpu.a ^= cpu.d;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn xor_a_e(cpu: &mut Cpu) {
    cpu.a ^= cpu.e;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn xor_a_h(cpu: &mut Cpu) {
    cpu.a ^= cpu.h;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn xor_a_l(cpu: &mut Cpu) {
    cpu.a ^= cpu.l;

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

fn or_a_d(cpu: &mut Cpu) {
    cpu.a |= cpu.d;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn or_a_e(cpu: &mut Cpu) {
    cpu.a |= cpu.e;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn or_a_h(cpu: &mut Cpu) {
    cpu.a |= cpu.h;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn or_a_l(cpu: &mut Cpu) {
    cpu.a |= cpu.l;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn or_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.a |= byte;

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

fn cp_a_b(cpu: &mut Cpu) {
    sub_set_flags(cpu, cpu.a, cpu.b);
}

fn cp_a_c(cpu: &mut Cpu) {
    sub_set_flags(cpu, cpu.a, cpu.c);
}

fn cp_a_d(cpu: &mut Cpu) {
    sub_set_flags(cpu, cpu.a, cpu.d);
}

fn cp_a_e(cpu: &mut Cpu) {
    sub_set_flags(cpu, cpu.a, cpu.e);
}

fn cp_a_h(cpu: &mut Cpu) {
    sub_set_flags(cpu, cpu.a, cpu.h);
}

fn cp_a_l(cpu: &mut Cpu) {
    sub_set_flags(cpu, cpu.a, cpu.l);
}

fn cp_a_mhl(cpu: &mut Cpu) {
    let byte = cpu.mmu.read(cpu.hl());
    sub_set_flags(cpu, cpu.a, byte);
}

fn cp_a_a(cpu: &mut Cpu) {
    sub_set_flags(cpu, cpu.a, cpu.a);
}

fn ret_nz(cpu: &mut Cpu) {
    if cpu.zero() {
        return;
    }

    cpu.pc = cpu.pop_word();
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn pop_bc(cpu: &mut Cpu) {
    let word = cpu.pop_word();
    cpu.set_bc(word);
}

fn jp_nz_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if cpu.zero() {
        return;
    }

    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(4);
}

fn jp_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.pc = word;
}

fn call_nz_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if cpu.zero() {
        return;
    }

    cpu.push_word(cpu.pc);
    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn push_bc(cpu: &mut Cpu) {
    cpu.push_word(cpu.bc());
}

fn add_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = add_set_flags(cpu, cpu.a, byte);
}

fn rst_00h(cpu: &mut Cpu) {
    rst(cpu, 0x0000);
}

fn ret_z(cpu: &mut Cpu) {
    if !cpu.zero() {
        return;
    }

    cpu.pc = cpu.pop_word();
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn ret(cpu: &mut Cpu) {
    cpu.pc = cpu.pop_word();
}

fn jp_z_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.zero() {
        return;
    }

    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(4);
}

fn call_z_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.zero() {
        return;
    }

    cpu.push_word(cpu.pc);
    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn call_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();
    cpu.push_word(cpu.pc);
    cpu.pc = word;
}

fn adc_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = add_carry_set_flags(cpu, cpu.a, byte);
}

fn rst_08h(cpu: &mut Cpu) {
    rst(cpu, 0x0008);
}

fn ret_nc(cpu: &mut Cpu) {
    if cpu.carry() {
        return;
    }

    cpu.pc = cpu.pop_word();
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn pop_de(cpu: &mut Cpu) {
    let word = cpu.pop_word();
    cpu.set_de(word);
}

fn jp_nc_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if cpu.carry() {
        return;
    }

    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(4);
}

fn call_nc_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if cpu.carry() {
        return;
    }

    cpu.push_word(cpu.pc);
    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn push_de(cpu: &mut Cpu) {
    cpu.push_word(cpu.de());
}

fn sub_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = sub_set_flags(cpu, cpu.a, byte);
}

fn rst_10h(cpu: &mut Cpu) {
    rst(cpu, 0x0010);
}

fn ret_c(cpu: &mut Cpu) {
    if !cpu.carry() {
        return;
    }

    cpu.pc = cpu.pop_word();
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn reti(cpu: &mut Cpu) {
    cpu.pc = cpu.pop_word();
    cpu.enable_int();
}

fn jp_c_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.carry() {
        return;
    }

    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(4);
}

fn call_c_u16(cpu: &mut Cpu) {
    let word = cpu.read_u16();

    if !cpu.carry() {
        return;
    }

    cpu.push_word(cpu.pc);
    cpu.pc = word;
    cpu.cycles = cpu.cycles.wrapping_add(12);
}

fn sbc_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = sub_carry_set_flags(cpu, cpu.a, byte);
}

fn rst_18h(cpu: &mut Cpu) {
    rst(cpu, 0x0018);
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

fn rst_20h(cpu: &mut Cpu) {
    rst(cpu, 0x0020);
}

fn add_sp_i8(cpu: &mut Cpu) {
    let sp = cpu.sp as i32;
    let byte = cpu.read_u8() as i8;
    let byte_i32 = byte as i32;

    let result = sp.wrapping_add(byte_i32);

    cpu.set_sub(false);
    cpu.set_zero(false);
    cpu.set_half_carry((sp ^ byte_i32 ^ result) & 0x10 == 0x10);
    cpu.set_carry((sp ^ byte_i32 ^ result) & 0x100 == 0x100);

    cpu.sp = result as u16;
}

fn jp_hl(cpu: &mut Cpu) {
    cpu.pc = cpu.hl();
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

fn rst_28h(cpu: &mut Cpu) {
    rst(cpu, 0x0028);
}

fn ld_a_mff00u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a = cpu.mmu.read(0xff00 + byte as u16);
}

fn pop_af(cpu: &mut Cpu) {
    let word = cpu.pop_word();
    cpu.set_af(word);
}

fn ld_a_mff00c(cpu: &mut Cpu) {
    cpu.a = cpu.mmu.read(0xff00 + cpu.c as u16);
}

fn di(cpu: &mut Cpu) {
    cpu.disable_int();
}

fn push_af(cpu: &mut Cpu) {
    cpu.push_word(cpu.af());
}

fn or_a_u8(cpu: &mut Cpu) {
    let byte = cpu.read_u8();
    cpu.a |= byte;

    cpu.set_sub(false);
    cpu.set_zero(cpu.a == 0);
    cpu.set_half_carry(false);
    cpu.set_carry(false);
}

fn rst_30h(cpu: &mut Cpu) {
    rst(cpu, 0x0030);
}

fn ld_hl_spi8(cpu: &mut Cpu) {
    let sp = cpu.sp as i32;
    let byte = cpu.read_u8() as i8;
    let byte_i32 = byte as i32;

    let result = sp.wrapping_add(byte_i32);

    cpu.set_sub(false);
    cpu.set_zero(false);
    cpu.set_half_carry((sp ^ byte_i32 ^ result) & 0x10 == 0x10);
    cpu.set_carry((sp ^ byte_i32 ^ result) & 0x100 == 0x100);

    cpu.set_hl(result as u16);
}

fn ld_sp_hl(cpu: &mut Cpu) {
    cpu.sp = cpu.hl();
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

fn rlc_b(cpu: &mut Cpu) {
    cpu.b = rlc(cpu, cpu.b);
}

fn rlc_c(cpu: &mut Cpu) {
    cpu.c = rlc(cpu, cpu.c);
}

fn rlc_d(cpu: &mut Cpu) {
    cpu.d = rlc(cpu, cpu.d);
}

fn rlc_e(cpu: &mut Cpu) {
    cpu.e = rlc(cpu, cpu.e);
}

fn rlc_h(cpu: &mut Cpu) {
    cpu.h = rlc(cpu, cpu.h);
}

fn rlc_l(cpu: &mut Cpu) {
    cpu.l = rlc(cpu, cpu.l);
}

fn rlc_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = rlc(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn rlc_a(cpu: &mut Cpu) {
    cpu.a = rlc(cpu, cpu.a);
}

fn rrc_b(cpu: &mut Cpu) {
    cpu.b = rrc(cpu, cpu.b);
}

fn rrc_c(cpu: &mut Cpu) {
    cpu.c = rrc(cpu, cpu.c);
}

fn rrc_d(cpu: &mut Cpu) {
    cpu.d = rrc(cpu, cpu.d);
}

fn rrc_e(cpu: &mut Cpu) {
    cpu.e = rrc(cpu, cpu.e);
}

fn rrc_h(cpu: &mut Cpu) {
    cpu.h = rrc(cpu, cpu.h);
}

fn rrc_l(cpu: &mut Cpu) {
    cpu.l = rrc(cpu, cpu.l);
}

fn rrc_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = rrc(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn rrc_a(cpu: &mut Cpu) {
    cpu.a = rrc(cpu, cpu.a);
}

fn rl_b(cpu: &mut Cpu) {
    cpu.b = rl(cpu, cpu.b);
}

fn rl_c(cpu: &mut Cpu) {
    cpu.c = rl(cpu, cpu.c);
}

fn rl_d(cpu: &mut Cpu) {
    cpu.d = rl(cpu, cpu.d);
}

fn rl_e(cpu: &mut Cpu) {
    cpu.e = rl(cpu, cpu.e);
}

fn rl_h(cpu: &mut Cpu) {
    cpu.h = rl(cpu, cpu.h);
}

fn rl_l(cpu: &mut Cpu) {
    cpu.l = rl(cpu, cpu.l);
}

fn rl_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = rl(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn rl_a(cpu: &mut Cpu) {
    cpu.a = rl(cpu, cpu.a);
}

fn rr_b(cpu: &mut Cpu) {
    cpu.b = rr(cpu, cpu.b);
}

fn rr_c(cpu: &mut Cpu) {
    cpu.c = rr(cpu, cpu.c);
}

fn rr_d(cpu: &mut Cpu) {
    cpu.d = rr(cpu, cpu.d);
}

fn rr_e(cpu: &mut Cpu) {
    cpu.e = rr(cpu, cpu.e);
}

fn rr_h(cpu: &mut Cpu) {
    cpu.h = rr(cpu, cpu.h);
}

fn rr_l(cpu: &mut Cpu) {
    cpu.l = rr(cpu, cpu.l);
}

fn rr_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = rr(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn rr_a(cpu: &mut Cpu) {
    cpu.a = rr(cpu, cpu.a);
}

fn sla_b(cpu: &mut Cpu) {
    cpu.b = sla(cpu, cpu.b);
}

fn sla_c(cpu: &mut Cpu) {
    cpu.c = sla(cpu, cpu.c);
}

fn sla_d(cpu: &mut Cpu) {
    cpu.d = sla(cpu, cpu.d);
}

fn sla_e(cpu: &mut Cpu) {
    cpu.e = sla(cpu, cpu.e);
}

fn sla_h(cpu: &mut Cpu) {
    cpu.h = sla(cpu, cpu.h);
}

fn sla_l(cpu: &mut Cpu) {
    cpu.l = sla(cpu, cpu.l);
}

fn sla_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = sla(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn sla_a(cpu: &mut Cpu) {
    cpu.a = sla(cpu, cpu.a);
}

fn sra_b(cpu: &mut Cpu) {
    cpu.b = sra(cpu, cpu.b);
}

fn sra_c(cpu: &mut Cpu) {
    cpu.c = sra(cpu, cpu.c);
}

fn sra_d(cpu: &mut Cpu) {
    cpu.d = sra(cpu, cpu.d);
}

fn sra_e(cpu: &mut Cpu) {
    cpu.e = sra(cpu, cpu.e);
}

fn sra_h(cpu: &mut Cpu) {
    cpu.h = sra(cpu, cpu.h);
}

fn sra_l(cpu: &mut Cpu) {
    cpu.l = sra(cpu, cpu.l);
}

fn sra_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = sra(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn sra_a(cpu: &mut Cpu) {
    cpu.a = sra(cpu, cpu.a);
}

fn swap_b(cpu: &mut Cpu) {
    cpu.b = swap(cpu, cpu.b)
}

fn swap_c(cpu: &mut Cpu) {
    cpu.c = swap(cpu, cpu.c)
}

fn swap_d(cpu: &mut Cpu) {
    cpu.d = swap(cpu, cpu.d)
}

fn swap_e(cpu: &mut Cpu) {
    cpu.e = swap(cpu, cpu.e)
}

fn swap_h(cpu: &mut Cpu) {
    cpu.h = swap(cpu, cpu.h)
}

fn swap_l(cpu: &mut Cpu) {
    cpu.l = swap(cpu, cpu.l)
}

fn swap_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = swap(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn swap_a(cpu: &mut Cpu) {
    cpu.a = swap(cpu, cpu.a)
}

fn srl_b(cpu: &mut Cpu) {
    cpu.b = srl(cpu, cpu.b);
}

fn srl_c(cpu: &mut Cpu) {
    cpu.c = srl(cpu, cpu.c);
}

fn srl_d(cpu: &mut Cpu) {
    cpu.d = srl(cpu, cpu.d);
}

fn srl_e(cpu: &mut Cpu) {
    cpu.e = srl(cpu, cpu.e);
}

fn srl_h(cpu: &mut Cpu) {
    cpu.h = srl(cpu, cpu.h);
}

fn srl_l(cpu: &mut Cpu) {
    cpu.l = srl(cpu, cpu.l);
}

fn srl_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let result = srl(cpu, byte);
    cpu.mmu.write(hl, result);
}

fn srl_a(cpu: &mut Cpu) {
    cpu.a = srl(cpu, cpu.a);
}

fn bit_0_b(cpu: &mut Cpu) {
    bit_b(cpu, 0);
}

fn bit_0_c(cpu: &mut Cpu) {
    bit_c(cpu, 0);
}

fn bit_0_d(cpu: &mut Cpu) {
    bit_d(cpu, 0);
}

fn bit_0_e(cpu: &mut Cpu) {
    bit_e(cpu, 0);
}

fn bit_0_h(cpu: &mut Cpu) {
    bit_h(cpu, 0);
}

fn bit_0_l(cpu: &mut Cpu) {
    bit_l(cpu, 0);
}

fn bit_0_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 0);
}

fn bit_0_a(cpu: &mut Cpu) {
    bit_a(cpu, 0);
}

fn bit_1_b(cpu: &mut Cpu) {
    bit_b(cpu, 1);
}

fn bit_1_c(cpu: &mut Cpu) {
    bit_c(cpu, 1);
}

fn bit_1_d(cpu: &mut Cpu) {
    bit_d(cpu, 1);
}

fn bit_1_e(cpu: &mut Cpu) {
    bit_e(cpu, 1);
}

fn bit_1_h(cpu: &mut Cpu) {
    bit_h(cpu, 1);
}

fn bit_1_l(cpu: &mut Cpu) {
    bit_l(cpu, 1);
}

fn bit_1_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 1);
}

fn bit_1_a(cpu: &mut Cpu) {
    bit_a(cpu, 1);
}

fn bit_2_b(cpu: &mut Cpu) {
    bit_b(cpu, 2);
}

fn bit_2_c(cpu: &mut Cpu) {
    bit_c(cpu, 2);
}

fn bit_2_d(cpu: &mut Cpu) {
    bit_d(cpu, 2);
}

fn bit_2_e(cpu: &mut Cpu) {
    bit_e(cpu, 2);
}

fn bit_2_h(cpu: &mut Cpu) {
    bit_h(cpu, 2);
}

fn bit_2_l(cpu: &mut Cpu) {
    bit_l(cpu, 2);
}

fn bit_2_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 2);
}

fn bit_2_a(cpu: &mut Cpu) {
    bit_a(cpu, 2);
}

fn bit_3_b(cpu: &mut Cpu) {
    bit_b(cpu, 3);
}

fn bit_3_c(cpu: &mut Cpu) {
    bit_c(cpu, 3);
}

fn bit_3_d(cpu: &mut Cpu) {
    bit_d(cpu, 3);
}

fn bit_3_e(cpu: &mut Cpu) {
    bit_e(cpu, 3);
}

fn bit_3_h(cpu: &mut Cpu) {
    bit_h(cpu, 3);
}

fn bit_3_l(cpu: &mut Cpu) {
    bit_l(cpu, 3);
}

fn bit_3_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 3);
}

fn bit_3_a(cpu: &mut Cpu) {
    bit_a(cpu, 3);
}

fn bit_4_b(cpu: &mut Cpu) {
    bit_b(cpu, 4);
}

fn bit_4_c(cpu: &mut Cpu) {
    bit_c(cpu, 4);
}

fn bit_4_d(cpu: &mut Cpu) {
    bit_d(cpu, 4);
}

fn bit_4_e(cpu: &mut Cpu) {
    bit_e(cpu, 4);
}

fn bit_4_h(cpu: &mut Cpu) {
    bit_h(cpu, 4);
}

fn bit_4_l(cpu: &mut Cpu) {
    bit_l(cpu, 4);
}

fn bit_4_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 4);
}

fn bit_4_a(cpu: &mut Cpu) {
    bit_a(cpu, 4);
}

fn bit_5_b(cpu: &mut Cpu) {
    bit_b(cpu, 5);
}

fn bit_5_c(cpu: &mut Cpu) {
    bit_c(cpu, 5);
}

fn bit_5_d(cpu: &mut Cpu) {
    bit_d(cpu, 5);
}

fn bit_5_e(cpu: &mut Cpu) {
    bit_e(cpu, 5);
}

fn bit_5_h(cpu: &mut Cpu) {
    bit_h(cpu, 5);
}

fn bit_5_l(cpu: &mut Cpu) {
    bit_l(cpu, 5);
}

fn bit_5_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 5);
}

fn bit_5_a(cpu: &mut Cpu) {
    bit_a(cpu, 5);
}

fn bit_6_b(cpu: &mut Cpu) {
    bit_b(cpu, 6);
}

fn bit_6_c(cpu: &mut Cpu) {
    bit_c(cpu, 6);
}

fn bit_6_d(cpu: &mut Cpu) {
    bit_d(cpu, 6);
}

fn bit_6_e(cpu: &mut Cpu) {
    bit_e(cpu, 6);
}

fn bit_6_h(cpu: &mut Cpu) {
    bit_h(cpu, 6);
}

fn bit_6_l(cpu: &mut Cpu) {
    bit_l(cpu, 6);
}

fn bit_6_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 6);
}

fn bit_6_a(cpu: &mut Cpu) {
    bit_a(cpu, 6);
}

fn bit_7_b(cpu: &mut Cpu) {
    bit_b(cpu, 7);
}

fn bit_7_c(cpu: &mut Cpu) {
    bit_c(cpu, 7);
}

fn bit_7_d(cpu: &mut Cpu) {
    bit_d(cpu, 7);
}

fn bit_7_e(cpu: &mut Cpu) {
    bit_e(cpu, 7);
}

fn bit_7_h(cpu: &mut Cpu) {
    bit_h(cpu, 7);
}

fn bit_7_l(cpu: &mut Cpu) {
    bit_l(cpu, 7);
}

fn bit_7_mhl(cpu: &mut Cpu) {
    bit_mhl(cpu, 7);
}

fn bit_7_a(cpu: &mut Cpu) {
    bit_a(cpu, 7);
}

fn res_0_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 0);
}

fn res_0_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 0);
}

fn res_0_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 0);
}

fn res_0_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 0);
}

fn res_0_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 0);
}

fn res_0_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 0);
}

fn res_0_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 0);
    cpu.mmu.write(hl, value);
}

fn res_0_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 0);
}

fn res_1_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 1);
}

fn res_1_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 1);
}

fn res_1_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 1);
}

fn res_1_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 1);
}

fn res_1_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 1);
}

fn res_1_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 1);
}

fn res_1_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 1);
    cpu.mmu.write(hl, value);
}

fn res_1_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 1);
}

fn res_2_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 2);
}

fn res_2_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 2);
}

fn res_2_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 2);
}

fn res_2_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 2);
}

fn res_2_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 2);
}

fn res_2_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 2);
}

fn res_2_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 2);
    cpu.mmu.write(hl, value);
}

fn res_2_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 2);
}

fn res_3_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 3);
}

fn res_3_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 3);
}

fn res_3_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 3);
}

fn res_3_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 3);
}

fn res_3_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 3);
}

fn res_3_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 3);
}

fn res_3_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 3);
    cpu.mmu.write(hl, value);
}

fn res_3_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 3);
}

fn res_4_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 4);
}

fn res_4_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 4);
}

fn res_4_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 4);
}

fn res_4_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 4);
}

fn res_4_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 4);
}

fn res_4_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 4);
}

fn res_4_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 4);
    cpu.mmu.write(hl, value);
}

fn res_4_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 4);
}

fn res_5_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 5);
}

fn res_5_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 5);
}

fn res_5_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 5);
}

fn res_5_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 5);
}

fn res_5_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 5);
}

fn res_5_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 5);
}

fn res_5_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 5);
    cpu.mmu.write(hl, value);
}

fn res_5_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 5);
}

fn res_6_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 6);
}

fn res_6_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 6);
}

fn res_6_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 6);
}

fn res_6_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 6);
}

fn res_6_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 6);
}

fn res_6_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 6);
}

fn res_6_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 6);
    cpu.mmu.write(hl, value);
}

fn res_6_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 6);
}

fn res_7_b(cpu: &mut Cpu) {
    cpu.b = res(cpu.b, 7);
}

fn res_7_c(cpu: &mut Cpu) {
    cpu.c = res(cpu.c, 7);
}

fn res_7_d(cpu: &mut Cpu) {
    cpu.d = res(cpu.d, 7);
}

fn res_7_e(cpu: &mut Cpu) {
    cpu.e = res(cpu.e, 7);
}

fn res_7_h(cpu: &mut Cpu) {
    cpu.h = res(cpu.h, 7);
}

fn res_7_l(cpu: &mut Cpu) {
    cpu.l = res(cpu.l, 7);
}

fn res_7_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = res(byte, 7);
    cpu.mmu.write(hl, value);
}

fn res_7_a(cpu: &mut Cpu) {
    cpu.a = res(cpu.a, 7);
}

fn set_0_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 0);
}

fn set_0_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 0);
}

fn set_0_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 0);
}

fn set_0_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 0);
}

fn set_0_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 0);
}

fn set_0_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 0);
}

fn set_0_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 0);
    cpu.mmu.write(hl, value);
}

fn set_0_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 0);
}

fn set_1_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 1);
}

fn set_1_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 1);
}

fn set_1_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 1);
}

fn set_1_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 1);
}

fn set_1_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 1);
}

fn set_1_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 1);
}

fn set_1_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 1);
    cpu.mmu.write(hl, value);
}

fn set_1_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 1);
}

fn set_2_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 2);
}

fn set_2_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 2);
}

fn set_2_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 2);
}

fn set_2_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 2);
}

fn set_2_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 2);
}

fn set_2_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 2);
}

fn set_2_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 2);
    cpu.mmu.write(hl, value);
}

fn set_2_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 2);
}

fn set_3_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 3);
}

fn set_3_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 3);
}

fn set_3_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 3);
}

fn set_3_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 3);
}

fn set_3_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 3);
}

fn set_3_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 3);
}

fn set_3_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 3);
    cpu.mmu.write(hl, value);
}

fn set_3_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 3);
}

fn set_4_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 4);
}

fn set_4_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 4);
}

fn set_4_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 4);
}

fn set_4_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 4);
}

fn set_4_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 4);
}

fn set_4_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 4);
}

fn set_4_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 4);
    cpu.mmu.write(hl, value);
}

fn set_4_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 4);
}

fn set_5_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 5);
}

fn set_5_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 5);
}

fn set_5_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 5);
}

fn set_5_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 5);
}

fn set_5_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 5);
}

fn set_5_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 5);
}

fn set_5_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 5);
    cpu.mmu.write(hl, value);
}

fn set_5_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 5);
}

fn set_6_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 6);
}

fn set_6_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 6);
}

fn set_6_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 6);
}

fn set_6_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 6);
}

fn set_6_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 6);
}

fn set_6_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 6);
}

fn set_6_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 6);
    cpu.mmu.write(hl, value);
}

fn set_6_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 6);
}

fn set_7_b(cpu: &mut Cpu) {
    cpu.b = set(cpu.b, 7);
}

fn set_7_c(cpu: &mut Cpu) {
    cpu.c = set(cpu.c, 7);
}

fn set_7_d(cpu: &mut Cpu) {
    cpu.d = set(cpu.d, 7);
}

fn set_7_e(cpu: &mut Cpu) {
    cpu.e = set(cpu.e, 7);
}

fn set_7_h(cpu: &mut Cpu) {
    cpu.h = set(cpu.h, 7);
}

fn set_7_l(cpu: &mut Cpu) {
    cpu.l = set(cpu.l, 7);
}

fn set_7_mhl(cpu: &mut Cpu) {
    let hl = cpu.hl();
    let byte = cpu.mmu.read(hl);
    let value = set(byte, 7);
    cpu.mmu.write(hl, value);
}

fn set_7_a(cpu: &mut Cpu) {
    cpu.a = set(cpu.a, 7);
}

/// Helper function to set one bit in a u8.
fn set(value: u8, bit: u8) -> u8 {
    value | (1u8 << (bit as usize))
}

/// Helper function to clear one bit in a u8.
fn res(value: u8, bit: u8) -> u8 {
    value & !(1u8 << (bit as usize))
}

/// Helper function that rotates (shifts) left the given
/// byte (probably from a register) and updates the
/// proper flag registers.
fn rl(cpu: &mut Cpu, value: u8) -> u8 {
    let carry = cpu.carry();

    cpu.set_carry((value & 0x80) == 0x80);

    let result = (value << 1) | carry as u8;

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);

    result
}

fn rlc(cpu: &mut Cpu, value: u8) -> u8 {
    cpu.set_carry((value & 0x80) == 0x80);

    let result = (value << 1) | (value >> 7);

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);

    result
}

/// Helper function that rotates (shifts) right the given
/// byte (probably from a register) and updates the
/// proper flag registers.
fn rr(cpu: &mut Cpu, value: u8) -> u8 {
    let carry = cpu.carry();

    cpu.set_carry((value & 0x1) == 0x1);

    let result = (value >> 1) | ((carry as u8) << 7);

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);

    result
}

fn rrc(cpu: &mut Cpu, value: u8) -> u8 {
    cpu.set_carry((value & 0x1) == 0x1);

    let result = (value >> 1) | (value << 7);

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

fn bit_a(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.a, bit));
    cpu.set_half_carry(true);
}

fn bit_b(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.b, bit));
    cpu.set_half_carry(true);
}

fn bit_c(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.c, bit));
    cpu.set_half_carry(true);
}

fn bit_d(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.d, bit));
    cpu.set_half_carry(true);
}

fn bit_e(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.e, bit));
    cpu.set_half_carry(true);
}

fn bit_h(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.h, bit));
    cpu.set_half_carry(true);
}

fn bit_l(cpu: &mut Cpu, bit: u8) {
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(cpu.l, bit));
    cpu.set_half_carry(true);
}

fn bit_mhl(cpu: &mut Cpu, bit: u8) {
    let byte = cpu.mmu.read(cpu.hl());
    cpu.set_sub(false);
    cpu.set_zero(bit_zero(byte, bit));
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
    cpu.set_carry((result & 0x100) == 0x100);

    result_b
}

fn add_carry_set_flags(cpu: &mut Cpu, first: u8, second: u8) -> u8 {
    let first = first as u32;
    let second = second as u32;
    let carry = cpu.carry() as u32;

    let result = first.wrapping_add(second).wrapping_add(carry);
    let result_b = result as u8;

    cpu.set_sub(false);
    cpu.set_zero(result_b == 0);
    cpu.set_half_carry((first ^ second ^ result) & 0x10 == 0x10);
    cpu.set_carry((result & 0x100) == 0x100);

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
    cpu.set_carry((result & 0x100) == 0x100);

    result_b
}

fn sub_carry_set_flags(cpu: &mut Cpu, first: u8, second: u8) -> u8 {
    let first = first as u32;
    let second = second as u32;
    let carry = cpu.carry() as u32;

    let result = first.wrapping_sub(second).wrapping_sub(carry);
    let result_b = result as u8;

    cpu.set_sub(true);
    cpu.set_zero(result_b == 0);
    cpu.set_half_carry((first ^ second ^ result) & 0x10 == 0x10);
    cpu.set_carry((result & 0x100) == 0x100);

    result_b
}

fn add_u16_u16(cpu: &mut Cpu, first: u16, second: u16) -> u16 {
    let first = first as u32;
    let second = second as u32;
    let result = first.wrapping_add(second);

    cpu.set_sub(false);
    cpu.set_half_carry((first ^ second ^ result) & 0x1000 == 0x1000);
    cpu.set_carry((result & 0x10000) == 0x10000);

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
    cpu.set_carry((value & 0x80) == 0x80);

    result
}

fn sra(cpu: &mut Cpu, value: u8) -> u8 {
    let result = (value >> 1) | (value & 0x80);

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);
    cpu.set_carry((value & 0x1) == 0x1);

    result
}

fn srl(cpu: &mut Cpu, value: u8) -> u8 {
    let result = value >> 1;

    cpu.set_sub(false);
    cpu.set_zero(result == 0);
    cpu.set_half_carry(false);
    cpu.set_carry((value & 0x1) == 0x1);

    result
}

/// Helper function for RST instructions, pushes the
/// current PC to the stack and jumps to the provided
/// address.
fn rst(cpu: &mut Cpu, addr: u16) {
    cpu.push_word(cpu.pc);
    cpu.pc = addr;
}
