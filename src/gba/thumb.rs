//! Thumb (16-bit) instruction decoder and handlers for the ARM7TDMI.
//!
//! Implements all 19 Thumb instruction formats used by the GBA.

use crate::gba::{
    bios::handle_swi,
    consts::{CPSR_MODE_MASK, CPSR_T, MODE_SVC},
    cpu::Arm7Tdmi,
};

/// executes a single Thumb instruction
pub fn execute_thumb(cpu: &mut Arm7Tdmi, instr: u16) {
    let bits_15_8 = instr >> 8;

    match bits_15_8 >> 5 {
        0b000 => {
            if bits_15_8 >> 3 == 0b00011 {
                // format 2: add/subtract
                thumb_add_sub(cpu, instr);
            } else {
                // format 1: move shifted register
                thumb_shift(cpu, instr);
            }
        }
        0b001 => {
            // format 3: move/compare/add/subtract immediate
            thumb_imm_op(cpu, instr);
        }
        0b010 => {
            if bits_15_8 >> 2 == 0b010000 {
                // format 4: ALU operations
                thumb_alu(cpu, instr);
            } else if bits_15_8 >> 2 == 0b010001 {
                // format 5: hi register operations / branch exchange
                thumb_hi_reg(cpu, instr);
            } else if bits_15_8 >> 3 == 0b01001 {
                // format 6: PC-relative load
                thumb_pc_load(cpu, instr);
            } else {
                // format 7/8: load/store with register offset
                thumb_load_store_reg(cpu, instr);
            }
        }
        0b011 => {
            // format 9: load/store with immediate offset
            thumb_load_store_imm(cpu, instr);
        }
        0b100 => {
            if bits_15_8 >> 4 == 0b1000 {
                // format 10: load/store halfword
                thumb_load_store_half(cpu, instr);
            } else {
                // format 11: SP-relative load/store
                thumb_sp_load_store(cpu, instr);
            }
        }
        0b101 => {
            if bits_15_8 >> 4 == 0b1010 {
                // format 12: load address
                thumb_load_addr(cpu, instr);
            } else if bits_15_8 == 0b10110000 {
                // format 13: add offset to SP
                thumb_sp_offset(cpu, instr);
            } else {
                // format 14: push/pop registers
                thumb_push_pop(cpu, instr);
            }
        }
        0b110 => {
            if bits_15_8 >> 4 == 0b1100 {
                // format 15: multiple load/store
                thumb_ldm_stm(cpu, instr);
            } else if bits_15_8 == 0b11011111 {
                // format 17: software interrupt
                thumb_swi(cpu, instr);
            } else {
                // format 16: conditional branch
                thumb_cond_branch(cpu, instr);
            }
        }
        0b111 => {
            if bits_15_8 >> 3 == 0b11100 {
                // format 18: unconditional branch
                thumb_branch(cpu, instr);
            } else {
                // format 19: long branch with link
                thumb_long_branch(cpu, instr);
            }
        }
        _ => {
            cpu.cycles = 1;
        }
    }
}

/// format 1: move shifted register (LSL, LSR, ASR)
fn thumb_shift(cpu: &mut Arm7Tdmi, instr: u16) {
    let op = (instr >> 11) & 0x03;
    let offset = ((instr >> 6) & 0x1F) as u32;
    let rs = ((instr >> 3) & 0x07) as u32;
    let rd = (instr & 0x07) as u32;

    let value = cpu.reg(rs);
    let (result, carry) = cpu.barrel_shift_immediate(value, op as u32, offset);

    cpu.set_reg(rd, result);
    cpu.set_nz_flags(result);
    cpu.set_flag_c(carry);

    cpu.cycles = 1;
}

/// format 2: add/subtract
fn thumb_add_sub(cpu: &mut Arm7Tdmi, instr: u16) {
    let is_immediate = instr & (1 << 10) != 0;
    let is_sub = instr & (1 << 9) != 0;
    let rn_or_imm = ((instr >> 6) & 0x07) as u32;
    let rs = ((instr >> 3) & 0x07) as u32;
    let rd = (instr & 0x07) as u32;

    let op1 = cpu.reg(rs);
    let op2 = if is_immediate {
        rn_or_imm
    } else {
        cpu.reg(rn_or_imm)
    };

    let result = if is_sub {
        let (r, borrow) = op1.overflowing_sub(op2);
        cpu.set_flag_c(!borrow);
        cpu.set_flag_v(((op1 ^ op2) & (op1 ^ r)) >> 31 != 0);
        r
    } else {
        let (r, carry) = op1.overflowing_add(op2);
        cpu.set_flag_c(carry);
        cpu.set_flag_v(((!(op1 ^ op2)) & (op1 ^ r)) >> 31 != 0);
        r
    };

    cpu.set_reg(rd, result);
    cpu.set_nz_flags(result);

    cpu.cycles = 1;
}

/// format 3: move/compare/add/subtract immediate
fn thumb_imm_op(cpu: &mut Arm7Tdmi, instr: u16) {
    let op = (instr >> 11) & 0x03;
    let rd = ((instr >> 8) & 0x07) as u32;
    let imm = (instr & 0xFF) as u32;

    match op {
        0 => {
            // MOV
            cpu.set_reg(rd, imm);
            cpu.set_nz_flags(imm);
        }
        1 => {
            // CMP
            let op1 = cpu.reg(rd);
            let (r, borrow) = op1.overflowing_sub(imm);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(!borrow);
            cpu.set_flag_v(((op1 ^ imm) & (op1 ^ r)) >> 31 != 0);
        }
        2 => {
            // ADD
            let op1 = cpu.reg(rd);
            let (r, carry) = op1.overflowing_add(imm);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(carry);
            cpu.set_flag_v(((!(op1 ^ imm)) & (op1 ^ r)) >> 31 != 0);
        }
        3 => {
            // SUB
            let op1 = cpu.reg(rd);
            let (r, borrow) = op1.overflowing_sub(imm);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(!borrow);
            cpu.set_flag_v(((op1 ^ imm) & (op1 ^ r)) >> 31 != 0);
        }
        _ => unreachable!(),
    }

    cpu.cycles = 1;
}

/// format 4: ALU operations
fn thumb_alu(cpu: &mut Arm7Tdmi, instr: u16) {
    let op = (instr >> 6) & 0xF;
    let rs = ((instr >> 3) & 0x07) as u32;
    let rd = (instr & 0x07) as u32;

    let a = cpu.reg(rd);
    let b = cpu.reg(rs);

    match op {
        0x0 => {
            // AND
            let r = a & b;
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
        }
        0x1 => {
            // EOR
            let r = a ^ b;
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
        }
        0x2 => {
            // LSL
            let amount = b & 0xFF;
            let (r, carry) = cpu.barrel_shift(a, 0, amount);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            if amount != 0 {
                cpu.set_flag_c(carry);
            }
        }
        0x3 => {
            // LSR
            let amount = b & 0xFF;
            let (r, carry) = cpu.barrel_shift(a, 1, amount);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            if amount != 0 {
                cpu.set_flag_c(carry);
            }
        }
        0x4 => {
            // ASR
            let amount = b & 0xFF;
            let (r, carry) = cpu.barrel_shift(a, 2, amount);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            if amount != 0 {
                cpu.set_flag_c(carry);
            }
        }
        0x5 => {
            // ADC
            let c = cpu.flag_c() as u32;
            let r = a.wrapping_add(b).wrapping_add(c);
            let carry = (a as u64) + (b as u64) + (c as u64) > 0xFFFFFFFF;
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(carry);
            cpu.set_flag_v(((!(a ^ b)) & (a ^ r)) >> 31 != 0);
        }
        0x6 => {
            // SBC
            let c = cpu.flag_c() as u32;
            let r = a.wrapping_sub(b).wrapping_sub(1 - c);
            let borrow = (a as u64) < (b as u64) + (1 - c as u64);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(!borrow);
            cpu.set_flag_v(((a ^ b) & (a ^ r)) >> 31 != 0);
        }
        0x7 => {
            // ROR
            let amount = b & 0xFF;
            let (r, carry) = cpu.barrel_shift(a, 3, amount);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            if amount != 0 {
                cpu.set_flag_c(carry);
            }
        }
        0x8 => {
            // TST
            let r = a & b;
            cpu.set_nz_flags(r);
        }
        0x9 => {
            // NEG
            let (r, borrow) = 0u32.overflowing_sub(b);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(!borrow);
            cpu.set_flag_v((b & r) >> 31 != 0);
        }
        0xA => {
            // CMP
            let (r, borrow) = a.overflowing_sub(b);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(!borrow);
            cpu.set_flag_v(((a ^ b) & (a ^ r)) >> 31 != 0);
        }
        0xB => {
            // CMN
            let (r, carry) = a.overflowing_add(b);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(carry);
            cpu.set_flag_v(((!(a ^ b)) & (a ^ r)) >> 31 != 0);
        }
        0xC => {
            // ORR
            let r = a | b;
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
        }
        0xD => {
            // MUL
            let r = a.wrapping_mul(b);
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
        }
        0xE => {
            // BIC
            let r = a & !b;
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
        }
        0xF => {
            // MVN
            let r = !b;
            cpu.set_reg(rd, r);
            cpu.set_nz_flags(r);
        }
        _ => {}
    }

    cpu.cycles = 1;
}

/// format 5: hi register operations / branch exchange
fn thumb_hi_reg(cpu: &mut Arm7Tdmi, instr: u16) {
    let op = (instr >> 8) & 0x03;
    let h1 = (instr >> 7) & 1;
    let h2 = (instr >> 6) & 1;
    let rs = (((h2 << 3) | ((instr >> 3) & 0x07)) as u32) & 0xF;
    let rd = (((h1 << 3) | (instr & 0x07)) as u32) & 0xF;

    match op {
        0 => {
            // ADD
            let result = cpu.reg(rd).wrapping_add(cpu.reg(rs));
            cpu.set_reg(rd, result);
        }
        1 => {
            // CMP
            let a = cpu.reg(rd);
            let b = cpu.reg(rs);
            let (r, borrow) = a.overflowing_sub(b);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(!borrow);
            cpu.set_flag_v(((a ^ b) & (a ^ r)) >> 31 != 0);
        }
        2 => {
            // MOV
            cpu.set_reg(rd, cpu.reg(rs));
        }
        3 => {
            // BX
            let addr = cpu.reg(rs);
            if addr & 1 != 0 {
                // stay in Thumb mode
                cpu.set_reg(15, addr & !1);
            } else {
                // switch to ARM mode
                let new_cpsr = cpu.cpsr() & !CPSR_T;
                cpu.set_cpsr(new_cpsr);
                cpu.set_reg(15, addr & !3);
            }
        }
        _ => {}
    }

    cpu.cycles = 1;
}

/// format 6: PC-relative load (LDR Rd, [PC, #imm])
fn thumb_pc_load(cpu: &mut Arm7Tdmi, instr: u16) {
    let rd = ((instr >> 8) & 0x07) as u32;
    let offset = ((instr & 0xFF) as u32) * 4;

    // PC is aligned to word boundary and points 4 bytes ahead
    let addr = (cpu.pc() & !3).wrapping_add(offset);
    let value = cpu.bus_read32(addr);
    cpu.set_reg(rd, value);

    cpu.cycles = 3;
}

/// format 7/8: load/store with register offset
fn thumb_load_store_reg(cpu: &mut Arm7Tdmi, instr: u16) {
    let opcode = (instr >> 9) & 0x07;
    let ro = ((instr >> 6) & 0x07) as u32;
    let rb = ((instr >> 3) & 0x07) as u32;
    let rd = (instr & 0x07) as u32;

    let addr = cpu.reg(rb).wrapping_add(cpu.reg(ro));

    match opcode {
        0b000 => {
            // STR
            cpu.bus_write32(addr, cpu.reg(rd));
        }
        0b001 => {
            // STRH
            cpu.bus_write16(addr, cpu.reg(rd) as u16);
        }
        0b010 => {
            // STRB
            cpu.bus_write8(addr, cpu.reg(rd) as u8);
        }
        0b011 => {
            // LDRSB
            let value = cpu.bus_read8(addr) as i8 as i32 as u32;
            cpu.set_reg(rd, value);
        }
        0b100 => {
            // LDR — misaligned addresses rotate
            let rotate = (addr & 3) * 8;
            let value = cpu.bus_read32(addr & !3);
            let value = if rotate != 0 {
                value.rotate_right(rotate)
            } else {
                value
            };
            cpu.set_reg(rd, value);
        }
        0b101 => {
            // LDRH — misaligned addresses rotate
            let aligned = cpu.bus_read16(addr & !1) as u32;
            let value = if addr & 1 != 0 {
                aligned.rotate_right(8)
            } else {
                aligned
            };
            cpu.set_reg(rd, value);
        }
        0b110 => {
            // LDRB
            let value = cpu.bus_read8(addr) as u32;
            cpu.set_reg(rd, value);
        }
        0b111 => {
            // LDRSH — misaligned reads as LDRSB
            let value = if addr & 1 != 0 {
                cpu.bus_read8(addr) as i8 as i32 as u32
            } else {
                cpu.bus_read16(addr) as i16 as i32 as u32
            };
            cpu.set_reg(rd, value);
        }
        _ => {}
    }

    cpu.cycles = if opcode >= 0b011 { 3 } else { 2 };
}

/// format 9: load/store with immediate offset
fn thumb_load_store_imm(cpu: &mut Arm7Tdmi, instr: u16) {
    let is_byte = instr & (1 << 12) != 0;
    let is_load = instr & (1 << 11) != 0;
    let offset = ((instr >> 6) & 0x1F) as u32;
    let rb = ((instr >> 3) & 0x07) as u32;
    let rd = (instr & 0x07) as u32;

    let base = cpu.reg(rb);

    if is_byte {
        let addr = base.wrapping_add(offset);
        if is_load {
            let value = cpu.bus_read8(addr) as u32;
            cpu.set_reg(rd, value);
        } else {
            cpu.bus_write8(addr, cpu.reg(rd) as u8);
        }
    } else {
        let addr = base.wrapping_add(offset * 4);
        if is_load {
            // misaligned addresses rotate
            let rotate = (addr & 3) * 8;
            let value = cpu.bus_read32(addr & !3);
            let value = if rotate != 0 {
                value.rotate_right(rotate)
            } else {
                value
            };
            cpu.set_reg(rd, value);
        } else {
            cpu.bus_write32(addr, cpu.reg(rd));
        }
    }

    cpu.cycles = if is_load { 3 } else { 2 };
}

/// format 10: load/store halfword
fn thumb_load_store_half(cpu: &mut Arm7Tdmi, instr: u16) {
    let is_load = instr & (1 << 11) != 0;
    let offset = (((instr >> 6) & 0x1F) as u32) * 2;
    let rb = ((instr >> 3) & 0x07) as u32;
    let rd = (instr & 0x07) as u32;

    let addr = cpu.reg(rb).wrapping_add(offset);

    if is_load {
        // misaligned addresses rotate
        let aligned = cpu.bus_read16(addr & !1) as u32;
        let value = if addr & 1 != 0 {
            aligned.rotate_right(8)
        } else {
            aligned
        };
        cpu.set_reg(rd, value);
    } else {
        cpu.bus_write16(addr, cpu.reg(rd) as u16);
    }

    cpu.cycles = if is_load { 3 } else { 2 };
}

/// format 11: SP-relative load/store
fn thumb_sp_load_store(cpu: &mut Arm7Tdmi, instr: u16) {
    let is_load = instr & (1 << 11) != 0;
    let rd = ((instr >> 8) & 0x07) as u32;
    let offset = ((instr & 0xFF) as u32) * 4;

    let addr = cpu.reg(13).wrapping_add(offset);

    if is_load {
        // misaligned addresses rotate
        let rotate = (addr & 3) * 8;
        let value = cpu.bus_read32(addr & !3);
        let value = if rotate != 0 {
            value.rotate_right(rotate)
        } else {
            value
        };
        cpu.set_reg(rd, value);
    } else {
        cpu.bus_write32(addr, cpu.reg(rd));
    }

    cpu.cycles = if is_load { 3 } else { 2 };
}

/// format 12: load address (ADD Rd, PC/SP, #imm)
fn thumb_load_addr(cpu: &mut Arm7Tdmi, instr: u16) {
    let use_sp = instr & (1 << 11) != 0;
    let rd = ((instr >> 8) & 0x07) as u32;
    let offset = ((instr & 0xFF) as u32) * 4;

    let base = if use_sp {
        cpu.reg(13)
    } else {
        cpu.pc() & !3 // word-aligned PC
    };

    cpu.set_reg(rd, base.wrapping_add(offset));

    cpu.cycles = 1;
}

/// format 13: add offset to SP
fn thumb_sp_offset(cpu: &mut Arm7Tdmi, instr: u16) {
    let offset = ((instr & 0x7F) as u32) * 4;
    let negative = instr & (1 << 7) != 0;

    let sp = cpu.reg(13);
    let new_sp = if negative {
        sp.wrapping_sub(offset)
    } else {
        sp.wrapping_add(offset)
    };

    cpu.set_reg(13, new_sp);

    cpu.cycles = 1;
}

/// format 14: push/pop registers
fn thumb_push_pop(cpu: &mut Arm7Tdmi, instr: u16) {
    let is_pop = instr & (1 << 11) != 0;
    let store_lr_load_pc = instr & (1 << 8) != 0;
    let reg_list = instr & 0xFF;

    if is_pop {
        // POP
        let mut addr = cpu.reg(13);
        for i in 0..8u32 {
            if reg_list & (1 << i) != 0 {
                let value = cpu.bus_read32(addr);
                cpu.set_reg(i, value);
                addr = addr.wrapping_add(4);
            }
        }
        if store_lr_load_pc {
            let value = cpu.bus_read32(addr);
            cpu.set_reg(15, value & !1);
            addr = addr.wrapping_add(4);
        }
        cpu.set_reg(13, addr);
    } else {
        // PUSH
        let count = reg_list.count_ones() + store_lr_load_pc as u32;
        let mut addr = cpu.reg(13).wrapping_sub(count * 4);
        cpu.set_reg(13, addr);

        for i in 0..8u32 {
            if reg_list & (1 << i) != 0 {
                cpu.bus_write32(addr, cpu.reg(i));
                addr = addr.wrapping_add(4);
            }
        }
        if store_lr_load_pc {
            cpu.bus_write32(addr, cpu.reg(14));
        }
    }

    cpu.cycles = 2 + reg_list.count_ones() + store_lr_load_pc as u32;
}

/// format 15: multiple load/store (LDMIA, STMIA)
fn thumb_ldm_stm(cpu: &mut Arm7Tdmi, instr: u16) {
    let is_load = instr & (1 << 11) != 0;
    let rb = ((instr >> 8) & 0x07) as u32;
    let reg_list = instr & 0xFF;
    let empty_rlist = reg_list == 0;

    let mut addr = cpu.reg(rb);

    if empty_rlist {
        // Empty register list: transfer PC, increment base by 0x40
        if is_load {
            let value = cpu.bus_read32(addr);
            cpu.set_reg(15, value);
        } else {
            cpu.bus_write32(addr, cpu.pc().wrapping_add(2));
        }
        addr = addr.wrapping_add(0x40);
    } else if is_load {
        // LDM: writeback is skipped if Rb is in the register list
        let skip_wb = reg_list & (1 << rb) != 0;
        for i in 0..8u32 {
            if reg_list & (1 << i) != 0 {
                let value = cpu.bus_read32(addr);
                cpu.set_reg(i, value);
                addr = addr.wrapping_add(4);
            }
        }
        if !skip_wb {
            cpu.set_reg(rb, addr);
        }
        cpu.cycles = 2 + reg_list.count_ones();
        return;
    } else {
        // STM: base-not-first stores written-back value
        let lowest_in_list = (0..8u32).find(|&i| reg_list & (1 << i) != 0);
        let base_is_first = lowest_in_list == Some(rb);
        let wb_base = addr.wrapping_add(reg_list.count_ones() * 4);
        for i in 0..8u32 {
            if reg_list & (1 << i) != 0 {
                let value = if i == rb && !base_is_first {
                    wb_base
                } else {
                    cpu.reg(i)
                };
                cpu.bus_write32(addr, value);
                addr = addr.wrapping_add(4);
            }
        }
    }

    cpu.set_reg(rb, addr);

    cpu.cycles = 2 + if empty_rlist { 16 } else { reg_list.count_ones() };
}

/// format 16: conditional branch
fn thumb_cond_branch(cpu: &mut Arm7Tdmi, instr: u16) {
    let cond = (instr >> 8) & 0xF;
    let offset = ((instr & 0xFF) as i8 as i32) * 2;

    // reuse ARM condition checking
    let condition_met = match cond {
        0x0 => cpu.flag_z(),
        0x1 => !cpu.flag_z(),
        0x2 => cpu.flag_c(),
        0x3 => !cpu.flag_c(),
        0x4 => cpu.flag_n(),
        0x5 => !cpu.flag_n(),
        0x6 => cpu.flag_v(),
        0x7 => !cpu.flag_v(),
        0x8 => cpu.flag_c() && !cpu.flag_z(),
        0x9 => !cpu.flag_c() || cpu.flag_z(),
        0xA => cpu.flag_n() == cpu.flag_v(),
        0xB => cpu.flag_n() != cpu.flag_v(),
        0xC => !cpu.flag_z() && cpu.flag_n() == cpu.flag_v(),
        0xD => cpu.flag_z() || cpu.flag_n() != cpu.flag_v(),
        _ => false,
    };

    if condition_met {
        let target = (cpu.pc() as i32).wrapping_add(offset) as u32;
        cpu.set_reg(15, target);
        cpu.cycles = 3;
    } else {
        cpu.cycles = 1;
    }
}

/// format 17: software interrupt
fn thumb_swi(cpu: &mut Arm7Tdmi, instr: u16) {
    let comment = (instr & 0xFF) as u8;

    let return_addr = cpu.pc().wrapping_sub(2);
    let old_cpsr = cpu.cpsr();

    cpu.set_cpsr((old_cpsr & !CPSR_MODE_MASK & !CPSR_T) | MODE_SVC | crate::gba::consts::CPSR_I);
    cpu.set_spsr(old_cpsr);
    cpu.set_reg(14, return_addr);

    handle_swi(cpu, comment);

    let lr = cpu.reg(14);
    cpu.restore_cpsr();
    cpu.set_reg(15, lr);

    cpu.cycles = 3;
}

/// format 18: unconditional branch
fn thumb_branch(cpu: &mut Arm7Tdmi, instr: u16) {
    let offset = ((instr & 0x07FF) as i32) << 21 >> 20; // sign-extend, shift left by 1
    let target = (cpu.pc() as i32).wrapping_add(offset) as u32;
    cpu.set_reg(15, target);

    cpu.cycles = 3;
}

/// format 19: long branch with link (two-instruction sequence)
fn thumb_long_branch(cpu: &mut Arm7Tdmi, instr: u16) {
    let is_second = instr & (1 << 11) != 0;

    if !is_second {
        // first instruction: LR = PC + (offset << 12)
        let offset = ((instr & 0x07FF) as i32) << 21 >> 9; // sign-extend, shift left by 12
        let lr = (cpu.pc() as i32).wrapping_add(offset) as u32;
        cpu.set_reg(14, lr);
        cpu.cycles = 1;
    } else {
        // second instruction: PC = LR + (offset << 1), LR = old PC - 2 | 1
        let offset = ((instr & 0x07FF) as u32) << 1;
        let lr = cpu.reg(14);
        let return_addr = cpu.pc().wrapping_sub(2) | 1;
        let target = lr.wrapping_add(offset);

        cpu.set_reg(14, return_addr);
        cpu.set_reg(15, target);
        cpu.cycles = 3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gba::{bus::GbaBus, cpu::Arm7Tdmi};

    fn make_thumb_cpu() -> Arm7Tdmi {
        let mut cpu = Arm7Tdmi::new(GbaBus::new());
        let cpsr = cpu.cpsr() | crate::gba::consts::CPSR_T;
        cpu.set_cpsr(cpsr);
        cpu
    }

    #[test]
    fn test_thumb_mov_imm() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x202A);
        assert_eq!(cpu.reg(0), 42);
    }

    #[test]
    fn test_thumb_add_imm() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x200A);
        execute_thumb(&mut cpu, 0x3005);
        assert_eq!(cpu.reg(0), 15);
    }

    #[test]
    fn test_thumb_sub_imm() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x2014);
        execute_thumb(&mut cpu, 0x3805);
        assert_eq!(cpu.reg(0), 15);
    }

    #[test]
    fn test_thumb_cmp_imm() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x200A);
        execute_thumb(&mut cpu, 0x280A);
        assert!(cpu.flag_z());
    }

    #[test]
    fn test_thumb_mov_zero_sets_z() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x2000);
        assert!(cpu.flag_z());
    }

    #[test]
    fn test_thumb_lsl() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x2001);
        execute_thumb(&mut cpu, 0x0101);
        assert_eq!(cpu.reg(1), 0x10);
    }

    #[test]
    fn test_thumb_lsr() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x2010);
        execute_thumb(&mut cpu, 0x0901);
        assert_eq!(cpu.reg(1), 1);
    }

    #[test]
    fn test_thumb_asr() {
        let mut cpu = make_thumb_cpu();
        cpu.set_reg(0, 0x80000000u32);
        execute_thumb(&mut cpu, 0x1101);
        assert_eq!(cpu.reg(1), 0xF8000000);
    }

    #[test]
    fn test_thumb_add_reg() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x200A);
        execute_thumb(&mut cpu, 0x2105);
        execute_thumb(&mut cpu, 0x1842);
        assert_eq!(cpu.reg(2), 15);
    }

    #[test]
    fn test_thumb_sub_reg() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x200A);
        execute_thumb(&mut cpu, 0x2103);
        execute_thumb(&mut cpu, 0x1A42);
        assert_eq!(cpu.reg(2), 7);
    }

    #[test]
    fn test_thumb_str_ldr() {
        let mut cpu = make_thumb_cpu();
        cpu.set_reg(0, 0xDEADBEEF);
        cpu.set_reg(1, 0x02000000);
        execute_thumb(&mut cpu, 0x6008);
        assert_eq!(cpu.bus_read32(0x02000000), 0xDEADBEEF);
        execute_thumb(&mut cpu, 0x680A);
        assert_eq!(cpu.reg(2), 0xDEADBEEF);
    }

    #[test]
    fn test_thumb_add_imm3() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x200A);
        execute_thumb(&mut cpu, 0x1CC1);
        assert_eq!(cpu.reg(1), 13);
    }

    #[test]
    fn test_thumb_sub_imm3() {
        let mut cpu = make_thumb_cpu();
        execute_thumb(&mut cpu, 0x200A);
        execute_thumb(&mut cpu, 0x1EC1);
        assert_eq!(cpu.reg(1), 7);
    }

    #[test]
    fn test_thumb_ldr_reg_misaligned_rotates() {
        let mut cpu = make_thumb_cpu();
        let addr = 0x02000000u32;
        cpu.bus_write32(addr, 0x04030201);
        cpu.set_reg(0, addr + 1);
        cpu.set_reg(1, 0);
        execute_thumb(&mut cpu, 0x5842);
        assert_eq!(cpu.reg(2), 0x04030201u32.rotate_right(8));
    }

    #[test]
    fn test_thumb_ldrh_reg_misaligned_rotates() {
        let mut cpu = make_thumb_cpu();
        let addr = 0x02000000u32;
        cpu.bus_write16(addr, 0xBEEF);
        cpu.set_reg(0, addr);
        cpu.set_reg(1, 1);
        execute_thumb(&mut cpu, 0x5A42);
        assert_eq!(cpu.reg(2), 0xBEEFu32.rotate_right(8));
    }

    #[test]
    fn test_thumb_ldrsh_reg_misaligned_reads_byte() {
        let mut cpu = make_thumb_cpu();
        let addr = 0x02000000u32;
        cpu.bus_write8(addr + 1, 0x80);
        cpu.set_reg(0, addr);
        cpu.set_reg(1, 1);
        execute_thumb(&mut cpu, 0x5E42);
        assert_eq!(cpu.reg(2), 0xFFFFFF80);
    }

    #[test]
    fn test_thumb_ldr_imm_misaligned_rotates() {
        let mut cpu = make_thumb_cpu();
        let addr = 0x02000000u32;
        cpu.bus_write32(addr, 0x04030201);
        cpu.set_reg(0, addr + 1);
        execute_thumb(&mut cpu, 0x6801);
        assert_eq!(cpu.reg(1), 0x04030201u32.rotate_right(8));
    }

    #[test]
    fn test_thumb_ldrh_imm_misaligned_rotates() {
        let mut cpu = make_thumb_cpu();
        let addr = 0x02000000u32;
        cpu.bus_write16(addr, 0xCAFE);
        cpu.set_reg(0, addr + 1);
        execute_thumb(&mut cpu, 0x8801);
        assert_eq!(cpu.reg(1), 0xCAFEu32.rotate_right(8));
    }

    #[test]
    fn test_thumb_sp_ldr_misaligned_rotates() {
        let mut cpu = make_thumb_cpu();
        let addr = 0x02000000u32;
        cpu.bus_write32(addr, 0x04030201);
        cpu.set_reg(13, addr + 1);
        execute_thumb(&mut cpu, 0x9800);
        assert_eq!(cpu.reg(0), 0x04030201u32.rotate_right(8));
    }

    #[test]
    fn test_thumb_ldm_stm_empty_rlist_stm() {
        let mut cpu = make_thumb_cpu();
        let base = 0x02000000u32;
        let pc_val = 0x08000100u32;
        cpu.set_reg(15, pc_val + 4);
        cpu.set_reg(0, base);
        execute_thumb(&mut cpu, 0xC000);
        assert_eq!(cpu.bus_read32(base), pc_val + 6);
        assert_eq!(cpu.reg(0), base + 0x40);
    }

    #[test]
    fn test_thumb_ldm_stm_empty_rlist_ldm() {
        let mut cpu = make_thumb_cpu();
        let base = 0x02000000u32;
        let target = 0x08000200u32;
        cpu.bus_write32(base, target);
        cpu.set_reg(0, base);
        execute_thumb(&mut cpu, 0xC800);
        assert_eq!(cpu.reg(15), target);
        assert_eq!(cpu.reg(0), base + 0x40);
    }

    #[test]
    fn test_thumb_ldm_writeback_skip_when_rb_in_rlist() {
        let mut cpu = make_thumb_cpu();
        let base = 0x02000000u32;
        cpu.bus_write32(base, 0xAA);
        cpu.bus_write32(base + 4, 0xBB);
        cpu.set_reg(0, base);
        execute_thumb(&mut cpu, 0xC803);
        assert_eq!(cpu.reg(0), 0xAA);
        assert_eq!(cpu.reg(1), 0xBB);
    }

    #[test]
    fn test_thumb_stm_base_first_stores_original() {
        let mut cpu = make_thumb_cpu();
        let base = 0x02000000u32;
        cpu.set_reg(0, base);
        cpu.set_reg(1, 0xBB);
        execute_thumb(&mut cpu, 0xC003);
        assert_eq!(cpu.bus_read32(base), base);
        assert_eq!(cpu.bus_read32(base + 4), 0xBB);
    }

    #[test]
    fn test_thumb_stm_base_not_first_stores_writeback() {
        let mut cpu = make_thumb_cpu();
        let base = 0x02000010u32;
        cpu.set_reg(0, 0xAA);
        cpu.set_reg(1, base);
        execute_thumb(&mut cpu, 0xC103);
        let wb_base = base + 8;
        assert_eq!(cpu.bus_read32(base), 0xAA);
        assert_eq!(cpu.bus_read32(base + 4), wb_base);
    }
}
