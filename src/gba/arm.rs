//! ARM (32-bit) instruction decoder and handlers for the ARM7TDMI.
//!
//! Uses a lookup table indexed by bits [27:20] and [7:4] of the
//! instruction for fast dispatch.

use crate::gba::{
    bios::handle_swi,
    consts::{CPSR_MODE_MASK, CPSR_T, MODE_SVC},
    cpu::Arm7Tdmi,
};

/// executes a single ARM instruction
pub fn execute_arm(cpu: &mut Arm7Tdmi, instr: u32) {
    // check condition code first
    if !cpu.check_condition(instr) {
        cpu.cycles = 1;
        return;
    }

    // decode instruction class from bits [27:20] and [7:4]
    let bits_27_20 = (instr >> 20) & 0xFF;
    let bits_7_4 = (instr >> 4) & 0xF;

    match bits_27_20 >> 5 {
        0b000 => {
            // data processing / multiply / swap / halfword transfer
            if bits_27_20 & 0xFC == 0x00 && bits_7_4 == 0x09 {
                // multiply
                arm_multiply(cpu, instr);
            } else if bits_27_20 & 0xF8 == 0x08 && bits_7_4 == 0x09 {
                // multiply long
                arm_multiply_long(cpu, instr);
            } else if bits_27_20 & 0xFB == 0x10 && bits_7_4 == 0x09 {
                // single data swap
                arm_swap(cpu, instr);
            } else if bits_7_4 & 0x09 == 0x09 && bits_7_4 & 0x06 != 0x00 {
                // halfword/signed data transfer
                arm_halfword_transfer(cpu, instr);
            } else if bits_27_20 == 0x12 && bits_7_4 == 0x01 {
                // branch and exchange (BX): 0001 0010 ... 0001
                arm_branch_exchange(cpu, instr);
            } else if (bits_27_20 == 0x10 || bits_27_20 == 0x14) && bits_7_4 == 0x00 {
                // MRS: 0001 0x00 ... 0000
                arm_mrs(cpu, instr);
            } else if (bits_27_20 == 0x12 || bits_27_20 == 0x16) && bits_7_4 == 0x00 {
                // MSR (register): 0001 0x10 ... 0000
                arm_msr_reg(cpu, instr);
            } else {
                // data processing
                arm_data_processing(cpu, instr);
            }
        }
        0b001 => {
            // data processing immediate / MSR immediate
            if bits_27_20 & 0xFB == 0x32 {
                arm_msr_imm(cpu, instr);
            } else {
                arm_data_processing(cpu, instr);
            }
        }
        0b010 => {
            // single data transfer (immediate offset)
            arm_single_transfer(cpu, instr);
        }
        0b011 => {
            if bits_7_4 & 0x01 != 0 {
                // undefined instruction
                cpu.cycles = 1;
            } else {
                // single data transfer (register offset)
                arm_single_transfer(cpu, instr);
            }
        }
        0b100 => {
            // block data transfer (LDM/STM)
            arm_block_transfer(cpu, instr);
        }
        0b101 => {
            // branch / branch with link
            arm_branch(cpu, instr);
        }
        0b111 => {
            if bits_27_20 & 0x10 != 0 {
                // software interrupt
                arm_swi(cpu, instr);
            } else {
                // coprocessor (ignored on GBA)
                cpu.cycles = 1;
            }
        }
        _ => {
            // coprocessor data transfer / other (0b110)
            cpu.cycles = 1;
        }
    }
}

/// ARM data processing instructions
fn arm_data_processing(cpu: &mut Arm7Tdmi, instr: u32) {
    let opcode = (instr >> 21) & 0xF;
    let set_flags = instr & (1 << 20) != 0;
    let rn = (instr >> 16) & 0xF;
    let rd = (instr >> 12) & 0xF;
    let is_immediate = instr & (1 << 25) != 0;

    let op1 = cpu.reg(rn);

    // compute operand 2 with barrel shifter
    let (op2, shifter_carry) = if is_immediate {
        let imm = instr & 0xFF;
        let rotate = ((instr >> 8) & 0xF) * 2;
        if rotate == 0 {
            (imm, cpu.flag_c())
        } else {
            let result = imm.rotate_right(rotate);
            (result, result & 0x80000000 != 0)
        }
    } else {
        let rm = instr & 0xF;
        let shift_type = (instr >> 5) & 0x03;
        let shift_by_reg = instr & (1 << 4) != 0;

        let rm_val = if rm == 15 {
            cpu.reg(15) // PC already advanced by pipeline
        } else {
            cpu.reg(rm)
        };

        if shift_by_reg {
            let rs = (instr >> 8) & 0xF;
            let shift_amount = cpu.reg(rs) & 0xFF;
            cpu.barrel_shift(rm_val, shift_type, shift_amount)
        } else {
            let shift_amount = (instr >> 7) & 0x1F;
            cpu.barrel_shift_immediate(rm_val, shift_type, shift_amount)
        }
    };

    let result = match opcode {
        0x0 => {
            // AND
            let r = op1 & op2;
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(shifter_carry);
            }
            Some(r)
        }
        0x1 => {
            // EOR
            let r = op1 ^ op2;
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(shifter_carry);
            }
            Some(r)
        }
        0x2 => {
            // SUB
            let (r, borrow) = op1.overflowing_sub(op2);
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(!borrow);
                cpu.set_flag_v(((op1 ^ op2) & (op1 ^ r)) >> 31 != 0);
            }
            Some(r)
        }
        0x3 => {
            // RSB
            let (r, borrow) = op2.overflowing_sub(op1);
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(!borrow);
                cpu.set_flag_v(((op2 ^ op1) & (op2 ^ r)) >> 31 != 0);
            }
            Some(r)
        }
        0x4 => {
            // ADD
            let (r, carry) = op1.overflowing_add(op2);
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(carry);
                cpu.set_flag_v(((!(op1 ^ op2)) & (op1 ^ r)) >> 31 != 0);
            }
            Some(r)
        }
        0x5 => {
            // ADC
            let c = cpu.flag_c() as u32;
            let r = op1.wrapping_add(op2).wrapping_add(c);
            if set_flags {
                let carry = (op1 as u64) + (op2 as u64) + (c as u64) > 0xFFFFFFFF;
                cpu.set_nz_flags(r);
                cpu.set_flag_c(carry);
                cpu.set_flag_v(((!(op1 ^ op2)) & (op1 ^ r)) >> 31 != 0);
            }
            Some(r)
        }
        0x6 => {
            // SBC
            let c = cpu.flag_c() as u32;
            let r = op1.wrapping_sub(op2).wrapping_sub(1 - c);
            if set_flags {
                let borrow = (op1 as u64) < (op2 as u64) + (1 - c as u64);
                cpu.set_nz_flags(r);
                cpu.set_flag_c(!borrow);
                cpu.set_flag_v(((op1 ^ op2) & (op1 ^ r)) >> 31 != 0);
            }
            Some(r)
        }
        0x7 => {
            // RSC
            let c = cpu.flag_c() as u32;
            let r = op2.wrapping_sub(op1).wrapping_sub(1 - c);
            if set_flags {
                let borrow = (op2 as u64) < (op1 as u64) + (1 - c as u64);
                cpu.set_nz_flags(r);
                cpu.set_flag_c(!borrow);
                cpu.set_flag_v(((op2 ^ op1) & (op2 ^ r)) >> 31 != 0);
            }
            Some(r)
        }
        0x8 => {
            // TST (test, no write)
            let r = op1 & op2;
            cpu.set_nz_flags(r);
            cpu.set_flag_c(shifter_carry);
            None
        }
        0x9 => {
            // TEQ (test equivalence)
            let r = op1 ^ op2;
            cpu.set_nz_flags(r);
            cpu.set_flag_c(shifter_carry);
            None
        }
        0xA => {
            // CMP (compare)
            let (r, borrow) = op1.overflowing_sub(op2);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(!borrow);
            cpu.set_flag_v(((op1 ^ op2) & (op1 ^ r)) >> 31 != 0);
            None
        }
        0xB => {
            // CMN (compare negated)
            let (r, carry) = op1.overflowing_add(op2);
            cpu.set_nz_flags(r);
            cpu.set_flag_c(carry);
            cpu.set_flag_v(((!(op1 ^ op2)) & (op1 ^ r)) >> 31 != 0);
            None
        }
        0xC => {
            // ORR
            let r = op1 | op2;
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(shifter_carry);
            }
            Some(r)
        }
        0xD => {
            // MOV
            if set_flags {
                cpu.set_nz_flags(op2);
                cpu.set_flag_c(shifter_carry);
            }
            Some(op2)
        }
        0xE => {
            // BIC
            let r = op1 & !op2;
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(shifter_carry);
            }
            Some(r)
        }
        0xF => {
            // MVN
            let r = !op2;
            if set_flags {
                cpu.set_nz_flags(r);
                cpu.set_flag_c(shifter_carry);
            }
            Some(r)
        }
        _ => None,
    };

    if let Some(value) = result {
        if rd == 15 {
            if set_flags {
                // writing to R15 with S bit: restore CPSR from SPSR
                cpu.restore_cpsr();
            }
            cpu.set_reg(15, value);
        } else {
            cpu.set_reg(rd, value);
        }
    }

    cpu.cycles = 1;
}

/// ARM multiply instruction (MUL, MLA)
fn arm_multiply(cpu: &mut Arm7Tdmi, instr: u32) {
    let rd = (instr >> 16) & 0xF;
    let rn = (instr >> 12) & 0xF;
    let rs = (instr >> 8) & 0xF;
    let rm = instr & 0xF;
    let accumulate = instr & (1 << 21) != 0;
    let set_flags = instr & (1 << 20) != 0;

    let mut result = cpu.reg(rm).wrapping_mul(cpu.reg(rs));
    if accumulate {
        result = result.wrapping_add(cpu.reg(rn));
    }

    cpu.set_reg(rd, result);

    if set_flags {
        cpu.set_nz_flags(result);
        // C flag is unpredictable in ARMv4
    }

    cpu.cycles = 2; // simplified timing
}

/// ARM multiply long instruction (UMULL, UMLAL, SMULL, SMLAL)
fn arm_multiply_long(cpu: &mut Arm7Tdmi, instr: u32) {
    let rd_hi = (instr >> 16) & 0xF;
    let rd_lo = (instr >> 12) & 0xF;
    let rs = (instr >> 8) & 0xF;
    let rm = instr & 0xF;
    let is_signed = instr & (1 << 22) != 0;
    let accumulate = instr & (1 << 21) != 0;
    let set_flags = instr & (1 << 20) != 0;

    let result = if is_signed {
        let a = cpu.reg(rm) as i32 as i64;
        let b = cpu.reg(rs) as i32 as i64;
        let mut r = a * b;
        if accumulate {
            let acc = ((cpu.reg(rd_hi) as u64) << 32) | cpu.reg(rd_lo) as u64;
            r = r.wrapping_add(acc as i64);
        }
        r as u64
    } else {
        let a = cpu.reg(rm) as u64;
        let b = cpu.reg(rs) as u64;
        let mut r = a * b;
        if accumulate {
            let acc = ((cpu.reg(rd_hi) as u64) << 32) | cpu.reg(rd_lo) as u64;
            r = r.wrapping_add(acc);
        }
        r
    };

    cpu.set_reg(rd_lo, result as u32);
    cpu.set_reg(rd_hi, (result >> 32) as u32);

    if set_flags {
        let hi = (result >> 32) as u32;
        let lo = result as u32;
        cpu.set_nz_flags(hi);
        if hi == 0 && lo == 0 {
            // Z already set by set_nz_flags for hi, but need to check lo too
        }
    }

    cpu.cycles = 3; // simplified timing
}

/// ARM single data swap (SWP, SWPB)
fn arm_swap(cpu: &mut Arm7Tdmi, instr: u32) {
    let rn = (instr >> 16) & 0xF;
    let rd = (instr >> 12) & 0xF;
    let rm = instr & 0xF;
    let is_byte = instr & (1 << 22) != 0;

    let addr = cpu.reg(rn);

    if is_byte {
        let old = cpu.bus_read8(addr);
        cpu.bus_write8(addr, cpu.reg(rm) as u8);
        cpu.set_reg(rd, old as u32);
    } else {
        let old = cpu.bus_read32(addr);
        cpu.bus_write32(addr, cpu.reg(rm));
        cpu.set_reg(rd, old);
    }

    cpu.cycles = 4;
}

/// ARM halfword and signed data transfer (LDRH, STRH, LDRSB, LDRSH)
fn arm_halfword_transfer(cpu: &mut Arm7Tdmi, instr: u32) {
    let pre_index = instr & (1 << 24) != 0;
    let add_offset = instr & (1 << 23) != 0;
    let imm_offset = instr & (1 << 22) != 0;
    let write_back = instr & (1 << 21) != 0;
    let is_load = instr & (1 << 20) != 0;
    let rn = (instr >> 16) & 0xF;
    let rd = (instr >> 12) & 0xF;
    let sh = (instr >> 5) & 0x03;

    let offset = if imm_offset {
        let hi = (instr >> 8) & 0xF;
        let lo = instr & 0xF;
        (hi << 4) | lo
    } else {
        let rm = instr & 0xF;
        cpu.reg(rm)
    };

    let base = cpu.reg(rn);
    let addr = if pre_index {
        if add_offset {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        }
    } else {
        base
    };

    if is_load {
        let value = match sh {
            1 => cpu.bus_read16(addr) as u32,               // LDRH
            2 => cpu.bus_read8(addr) as i8 as i32 as u32,   // LDRSB
            3 => cpu.bus_read16(addr) as i16 as i32 as u32, // LDRSH
            _ => 0,
        };
        cpu.set_reg(rd, value);
    } else {
        // STRH (sh == 1)
        cpu.bus_write16(addr, cpu.reg(rd) as u16);
    }

    // post-index: write back the modified address
    if !pre_index {
        let new_addr = if add_offset {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        cpu.set_reg(rn, new_addr);
    } else if write_back {
        cpu.set_reg(rn, addr);
    }

    cpu.cycles = if is_load { 3 } else { 2 };
}

/// ARM single data transfer (LDR, STR, LDRB, STRB)
fn arm_single_transfer(cpu: &mut Arm7Tdmi, instr: u32) {
    let is_register = instr & (1 << 25) != 0;
    let pre_index = instr & (1 << 24) != 0;
    let add_offset = instr & (1 << 23) != 0;
    let is_byte = instr & (1 << 22) != 0;
    let write_back = instr & (1 << 21) != 0;
    let is_load = instr & (1 << 20) != 0;
    let rn = (instr >> 16) & 0xF;
    let rd = (instr >> 12) & 0xF;

    let offset = if is_register {
        let rm = instr & 0xF;
        let shift_type = (instr >> 5) & 0x03;
        let shift_amount = (instr >> 7) & 0x1F;
        let (result, _) = cpu.barrel_shift_immediate(cpu.reg(rm), shift_type, shift_amount);
        result
    } else {
        instr & 0xFFF
    };

    let base = cpu.reg(rn);
    let addr = if pre_index {
        if add_offset {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        }
    } else {
        base
    };

    if is_load {
        let value = if is_byte {
            cpu.bus_read8(addr) as u32
        } else {
            // LDR with misaligned address: rotate
            let aligned = cpu.bus_read32(addr);
            let rotate = (addr & 3) * 8;
            if rotate == 0 {
                aligned
            } else {
                aligned.rotate_right(rotate)
            }
        };
        cpu.set_reg(rd, value);
    } else {
        let value = cpu.reg(rd);
        if is_byte {
            cpu.bus_write8(addr, value as u8);
        } else {
            cpu.bus_write32(addr, value);
        }
    }

    if !pre_index {
        let new_addr = if add_offset {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        cpu.set_reg(rn, new_addr);
    } else if write_back {
        cpu.set_reg(rn, addr);
    }

    cpu.cycles = if is_load { 3 } else { 2 };
}

/// ARM block data transfer (LDM, STM)
fn arm_block_transfer(cpu: &mut Arm7Tdmi, instr: u32) {
    let pre_index = instr & (1 << 24) != 0;
    let add_offset = instr & (1 << 23) != 0;
    let load_psr = instr & (1 << 22) != 0;
    let write_back = instr & (1 << 21) != 0;
    let is_load = instr & (1 << 20) != 0;
    let rn = (instr >> 16) & 0xF;
    let reg_list = instr & 0xFFFF;

    let count = reg_list.count_ones();
    let base = cpu.reg(rn);

    // calculate starting address
    let start_addr = if add_offset {
        if pre_index {
            base.wrapping_add(4)
        } else {
            base
        }
    } else if pre_index {
        base.wrapping_sub(count * 4)
    } else {
        base.wrapping_sub(count * 4).wrapping_add(4)
    };

    let mut addr = start_addr;

    if is_load {
        for i in 0..16u32 {
            if reg_list & (1 << i) != 0 {
                let value = cpu.bus_read32(addr);
                cpu.set_reg(i, value);
                addr = addr.wrapping_add(4);
            }
        }
        if load_psr && reg_list & (1 << 15) != 0 {
            cpu.restore_cpsr();
        }
    } else {
        for i in 0..16u32 {
            if reg_list & (1 << i) != 0 {
                cpu.bus_write32(addr, cpu.reg(i));
                addr = addr.wrapping_add(4);
            }
        }
    }

    if write_back {
        let new_base = if add_offset {
            base.wrapping_add(count * 4)
        } else {
            base.wrapping_sub(count * 4)
        };
        cpu.set_reg(rn, new_base);
    }

    cpu.cycles = count + if is_load { 2 } else { 1 };
}

/// ARM branch and branch with link (B, BL)
fn arm_branch(cpu: &mut Arm7Tdmi, instr: u32) {
    let link = instr & (1 << 24) != 0;
    let offset = ((instr & 0x00FFFFFF) as i32) << 8 >> 6; // sign-extend and shift left by 2

    if link {
        cpu.set_reg(14, cpu.pc().wrapping_sub(4)); // save return address
    }

    let target = (cpu.pc() as i32).wrapping_add(offset) as u32;
    cpu.set_reg(15, target);

    cpu.cycles = 3;
}

/// ARM branch and exchange (BX)
fn arm_branch_exchange(cpu: &mut Arm7Tdmi, instr: u32) {
    let rm = instr & 0xF;
    let addr = cpu.reg(rm);

    if addr & 1 != 0 {
        // switch to Thumb mode
        let new_cpsr = cpu.cpsr() | CPSR_T;
        cpu.set_cpsr(new_cpsr);
        cpu.set_reg(15, addr & !1);
    } else {
        // stay in ARM mode
        cpu.set_reg(15, addr & !3);
    }

    cpu.cycles = 3;
}

/// ARM MRS (transfer PSR to register)
fn arm_mrs(cpu: &mut Arm7Tdmi, instr: u32) {
    let rd = (instr >> 12) & 0xF;
    let use_spsr = instr & (1 << 22) != 0;

    let value = if use_spsr { cpu.spsr() } else { cpu.cpsr() };
    cpu.set_reg(rd, value);

    cpu.cycles = 1;
}

/// ARM MSR (transfer register to PSR)
fn arm_msr_reg(cpu: &mut Arm7Tdmi, instr: u32) {
    let use_spsr = instr & (1 << 22) != 0;
    let rm = instr & 0xF;
    let value = cpu.reg(rm);

    let mask = build_psr_mask(instr);
    write_psr(cpu, use_spsr, value, mask);

    cpu.cycles = 1;
}

/// ARM MSR immediate
fn arm_msr_imm(cpu: &mut Arm7Tdmi, instr: u32) {
    let use_spsr = instr & (1 << 22) != 0;
    let imm = instr & 0xFF;
    let rotate = ((instr >> 8) & 0xF) * 2;
    let value = imm.rotate_right(rotate);

    let mask = build_psr_mask(instr);
    write_psr(cpu, use_spsr, value, mask);

    cpu.cycles = 1;
}

/// builds a mask for PSR writes based on the field mask bits (16-19)
fn build_psr_mask(instr: u32) -> u32 {
    let mut mask = 0u32;
    if instr & (1 << 19) != 0 {
        mask |= 0xFF000000;
    } // flags
    if instr & (1 << 18) != 0 {
        mask |= 0x00FF0000;
    } // status
    if instr & (1 << 17) != 0 {
        mask |= 0x0000FF00;
    } // extension
    if instr & (1 << 16) != 0 {
        mask |= 0x000000FF;
    } // control
    mask
}

/// writes to CPSR or SPSR with the given mask
fn write_psr(cpu: &mut Arm7Tdmi, use_spsr: bool, value: u32, mask: u32) {
    if use_spsr {
        let old = cpu.spsr();
        cpu.set_spsr((old & !mask) | (value & mask));
    } else {
        let old = cpu.cpsr();
        // only allow mode bits in privileged mode
        let effective_mask = if old & CPSR_MODE_MASK == 0x10 {
            mask & 0xFF000000 // user mode: only flags
        } else {
            mask
        };
        cpu.set_cpsr((old & !effective_mask) | (value & effective_mask));
    }
}

/// ARM software interrupt (SWI)
fn arm_swi(cpu: &mut Arm7Tdmi, instr: u32) {
    let comment = ((instr >> 16) & 0xFF) as u8;

    // save state and enter SVC mode
    let return_addr = cpu.pc().wrapping_sub(4);
    let old_cpsr = cpu.cpsr();

    cpu.set_cpsr((old_cpsr & !CPSR_MODE_MASK & !CPSR_T) | MODE_SVC | crate::gba::consts::CPSR_I);
    cpu.set_spsr(old_cpsr);
    cpu.set_reg(14, return_addr);

    // HLE: handle the SWI in Rust
    handle_swi(cpu, comment);

    // return from SWI (restore CPSR and jump to LR)
    let lr = cpu.reg(14);
    cpu.restore_cpsr();
    cpu.set_reg(15, lr);

    cpu.cycles = 3;
}

#[cfg(test)]
mod tests {
    use crate::gba::{bus::GbaBus, cpu::Arm7Tdmi};

    fn make_cpu() -> Arm7Tdmi {
        Arm7Tdmi::new(GbaBus::new())
    }

    #[test]
    fn test_arm_mov() {
        let mut cpu = make_cpu();
        // MOV R0, #42 (E3A0002A)
        super::execute_arm(&mut cpu, 0xE3A0002A);
        assert_eq!(cpu.reg(0), 42);
    }

    #[test]
    fn test_arm_add() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 10);
        // ADD R1, R0, #5 (E2801005)
        super::execute_arm(&mut cpu, 0xE2801005);
        assert_eq!(cpu.reg(1), 15);
    }

    #[test]
    fn test_arm_sub() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 20);
        // SUB R1, R0, #5 (E2401005)
        super::execute_arm(&mut cpu, 0xE2401005);
        assert_eq!(cpu.reg(1), 15);
    }

    #[test]
    fn test_arm_cmp_sets_flags() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 5);
        // CMP R0, #5 (E3500005)
        super::execute_arm(&mut cpu, 0xE3500005);
        assert!(cpu.flag_z());
    }

    #[test]
    fn test_arm_and() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0xFF);
        // AND R1, R0, #0x0F (E200100F)
        super::execute_arm(&mut cpu, 0xE200100F);
        assert_eq!(cpu.reg(1), 0x0F);
    }

    #[test]
    fn test_arm_orr() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0xF0);
        // ORR R1, R0, #0x0F (E380100F)
        super::execute_arm(&mut cpu, 0xE380100F);
        assert_eq!(cpu.reg(1), 0xFF);
    }

    #[test]
    fn test_arm_eor() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0xFF);
        // EOR R1, R0, #0x0F (E220100F)
        super::execute_arm(&mut cpu, 0xE220100F);
        assert_eq!(cpu.reg(1), 0xF0);
    }

    #[test]
    fn test_arm_bic() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0xFF);
        // BIC R1, R0, #0x0F (E3C0100F)
        super::execute_arm(&mut cpu, 0xE3C0100F);
        assert_eq!(cpu.reg(1), 0xF0);
    }

    #[test]
    fn test_arm_mvn() {
        let mut cpu = make_cpu();
        // MVN R0, #0 (E3E00000) - NOT 0 = 0xFFFFFFFF
        super::execute_arm(&mut cpu, 0xE3E00000);
        assert_eq!(cpu.reg(0), 0xFFFFFFFF);
    }

    #[test]
    fn test_arm_rsb() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 5);
        // RSB R1, R0, #10 (E260100A)
        super::execute_arm(&mut cpu, 0xE260100A);
        assert_eq!(cpu.reg(1), 5); // 10 - 5
    }

    #[test]
    fn test_arm_tst() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0x00);
        // TST R0, #0xFF (E31000FF) - AND sets flags only
        super::execute_arm(&mut cpu, 0xE31000FF);
        assert!(cpu.flag_z());
    }

    #[test]
    fn test_arm_cmn() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0xFFFFFFFF);
        // CMN R0, #1 (E3700001) - (-1) + 1 = 0
        super::execute_arm(&mut cpu, 0xE3700001);
        assert!(cpu.flag_z());
    }

    #[test]
    fn test_arm_condition_fail() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0);
        // NV condition (0xF...) should not execute
        super::execute_arm(&mut cpu, 0xF3A00005);
        assert_eq!(cpu.reg(0), 0); // unchanged
    }

    #[test]
    fn test_arm_str_ldr() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0xDEADBEEF);
        cpu.set_reg(1, 0x0200_0000);
        // STR R0, [R1] (E5810000)
        super::execute_arm(&mut cpu, 0xE5810000);
        assert_eq!(cpu.bus_read32(0x0200_0000), 0xDEADBEEF);

        // LDR R2, [R1] (E5912000)
        super::execute_arm(&mut cpu, 0xE5912000);
        assert_eq!(cpu.reg(2), 0xDEADBEEF);
    }

    #[test]
    fn test_arm_mul() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 6);
        cpu.set_reg(1, 7);
        // MUL R2, R0, R1 (E0020190)
        super::execute_arm(&mut cpu, 0xE0020190);
        assert_eq!(cpu.reg(2), 42);
    }

    #[test]
    fn test_arm_movs_flags() {
        let mut cpu = make_cpu();
        // MOVS R0, #0 (E3B00000) - sets Z flag
        super::execute_arm(&mut cpu, 0xE3B00000);
        assert!(cpu.flag_z());
        assert!(!cpu.flag_n());
    }

    #[test]
    fn test_arm_adds_carry() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0xFFFFFFFF);
        // ADDS R1, R0, #1 (E2901001) - should overflow to 0 with carry
        super::execute_arm(&mut cpu, 0xE2901001);
        assert_eq!(cpu.reg(1), 0);
        assert!(cpu.flag_z());
        assert!(cpu.flag_c());
    }
}
