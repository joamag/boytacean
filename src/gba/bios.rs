//! GBA BIOS HLE (High-Level Emulation) of SWI (Software Interrupt) calls.
//!
//! Implements the most commonly used BIOS functions as native Rust code
//! rather than requiring a real BIOS ROM dump.

use crate::{gba::cpu::Arm7Tdmi, warnln};

/// Handles a SWI call by dispatching to the appropriate HLE function.
///
/// The comment field identifies which SWI is being called. Returns
/// `true` when the handler redirected execution (set PC and CPSR
/// itself), in which case the caller must not overwrite PC with the
/// SWI return address.
pub fn handle_swi(cpu: &mut Arm7Tdmi, comment: u8) -> bool {
    match comment {
        0x00 => return swi_soft_reset(cpu),
        0x01 => swi_register_ram_reset(cpu),
        0x02 => swi_halt(cpu),
        0x03 => swi_stop(cpu),
        0x04 => swi_intr_wait(cpu),
        0x05 => swi_vblank_intr_wait(cpu),
        0x06 => swi_div(cpu),
        0x07 => swi_div_arm(cpu),
        0x08 => swi_sqrt(cpu),
        0x09 => swi_arctan(cpu),
        0x0a => swi_arctan2(cpu),
        0x0b => swi_cpu_set(cpu),
        0x0c => swi_cpu_fast_set(cpu),
        0x0d => swi_get_bios_checksum(cpu),
        0x0e => swi_bg_affine_set(cpu),
        0x0f => swi_obj_affine_set(cpu),
        0x10 => swi_bit_unpack(cpu),
        0x11 => swi_lz77_decomp_wram(cpu),
        0x12 => swi_lz77_decomp_vram(cpu),
        0x13 => swi_huff_decomp(cpu),
        0x14 => swi_rl_decomp_wram(cpu),
        0x15 => swi_rl_decomp_vram(cpu),
        0x16 => swi_diff_unfilt8_wram(cpu),
        0x17 => swi_diff_unfilt8_vram(cpu),
        0x18 => swi_diff_unfilt16(cpu),
        0x19 => swi_sound_bias(cpu),
        0x1f => swi_midi_key2freq(cpu),
        _ => {
            warnln!("Unhandled SWI 0x{:02X}", comment);
        }
    }
    false
}

/// SWI 0x00: SoftReset - resets the system.
///
/// Always returns `true` since it redirects execution to the entry
/// point instead of returning to the caller.
fn swi_soft_reset(cpu: &mut Arm7Tdmi) -> bool {
    // the return address flag selects the entry point and must be
    // read before the area that contains it is cleared
    let flag = cpu.bus_read8(0x0300_7FFA);

    // clear IWRAM 0x03007E00-0x03007FFF
    for addr in (0x0300_7E00u32..0x0300_8000).step_by(4) {
        cpu.bus_write32(addr, 0);
    }

    // set registers to reset state, with the banked stack pointers
    // re-initialized per mode (SVC=0x03007FE0, IRQ=0x03007FA0,
    // SYS=0x03007F00) and LR/SPSR cleared, matching the real BIOS
    for reg in 0..=12 {
        cpu.set_reg(reg, 0);
    }
    cpu.set_cpsr(0x93); // supervisor mode
    cpu.set_reg(13, 0x0300_7FE0); // SP_SVC
    cpu.set_reg(14, 0);
    cpu.set_spsr(0);
    cpu.set_cpsr(0x92); // IRQ mode
    cpu.set_reg(13, 0x0300_7FA0); // SP_IRQ
    cpu.set_reg(14, 0);
    cpu.set_spsr(0);
    cpu.set_cpsr(0x1F); // system mode
    cpu.set_reg(13, 0x0300_7F00); // SP_SYS
    cpu.set_reg(14, 0);

    // jump to the ROM entry point, or to RAM for multiboot images
    // (non-zero return address flag at 0x03007FFA)
    if flag == 0 {
        cpu.set_reg(15, 0x0800_0000);
    } else {
        cpu.set_reg(15, 0x0200_0000);
    }
    true
}

/// SWI 0x01: RegisterRamReset - clears specified memory regions.
fn swi_register_ram_reset(cpu: &mut Arm7Tdmi) {
    let flags = cpu.reg(0);

    // the real BIOS always sets DISPCNT to forced blank, regardless
    // of the requested flags (documented BIOS quirk)
    cpu.bus_write16(0x0400_0000, 0x0080);

    // bit 0: clear EWRAM (256KB)
    if flags & (1 << 0) != 0 {
        for addr in (0x0200_0000u32..0x0204_0000).step_by(4) {
            cpu.bus_write32(addr, 0);
        }
    }

    // bit 1: clear IWRAM (except last 512 bytes)
    if flags & (1 << 1) != 0 {
        for addr in (0x0300_0000u32..0x0300_7E00).step_by(4) {
            cpu.bus_write32(addr, 0);
        }
    }

    // bit 2: clear palette
    if flags & (1 << 2) != 0 {
        for addr in (0x0500_0000u32..0x0500_0400).step_by(4) {
            cpu.bus_write32(addr, 0);
        }
    }

    // bit 3: clear VRAM
    if flags & (1 << 3) != 0 {
        for addr in (0x0600_0000u32..0x0601_8000).step_by(4) {
            cpu.bus_write32(addr, 0);
        }
    }

    // bit 4: clear OAM
    if flags & (1 << 4) != 0 {
        for addr in (0x0700_0000u32..0x0700_0400).step_by(4) {
            cpu.bus_write32(addr, 0);
        }
    }

    // bit 5: reset serial communication registers
    if flags & (1 << 5) != 0 {
        cpu.bus_write16(0x0400_0128, 0);
        cpu.bus_write16(0x0400_0134, 0x8000);
        cpu.bus_write16(0x0400_012A, 0);
        cpu.bus_write16(0x0400_0140, 0);
        cpu.bus_write32(0x0400_0150, 0);
        cpu.bus_write32(0x0400_0154, 0);
    }

    // bit 6: reset sound registers and both wave RAM banks
    if flags & (1 << 6) != 0 {
        for bank in [0x40, 0] {
            cpu.bus_write16(0x0400_0070, bank);
            for addr in (0x0400_0090u32..0x0400_00A0).step_by(4) {
                cpu.bus_write32(addr, 0);
            }
        }
        for addr in (0x0400_0060u32..0x0400_0086).step_by(2) {
            cpu.bus_write16(addr, 0);
        }
        cpu.bus_write16(0x0400_0088, 0x0200);
    }

    // bit 7: reset the remaining display, DMA, timer, and interrupt registers
    if flags & (1 << 7) != 0 {
        for addr in (0x0400_0004u32..0x0400_0056).step_by(2) {
            cpu.bus_write16(addr, 0);
        }
        for addr in [0x0400_0020, 0x0400_0026, 0x0400_0030, 0x0400_0036] {
            cpu.bus_write16(addr, 0x0100);
        }
        for addr in (0x0400_00B0u32..0x0400_00E0).step_by(2) {
            cpu.bus_write16(addr, 0);
        }
        for addr in (0x0400_0100u32..0x0400_0110).step_by(2) {
            cpu.bus_write16(addr, 0);
        }
        cpu.bus_write16(0x0400_0200, 0);
        cpu.bus_write16(0x0400_0202, 0xFFFF);
        cpu.bus_write16(0x0400_0204, 0);
        cpu.bus_write16(0x0400_0208, 0);
    }
}

/// SWI 0x02: Halt - halts the CPU until an interrupt occurs.
fn swi_halt(cpu: &mut Arm7Tdmi) {
    cpu.set_halted(true);
}

/// SWI 0x03: Stop - low-power stop mode (treat as halt).
fn swi_stop(cpu: &mut Arm7Tdmi) {
    cpu.set_halted(true);
}

/// SWI 0x04: IntrWait — halts until the requested interrupt flags
/// appear in IntrCheck (0x03007FF8).
///
/// Sets intr_wait_flags so the re-halt check in cpu.rs keeps the CPU halted until matched.
///
/// R0 = discard_old, R1 = interrupt flags to wait for.
fn swi_intr_wait(cpu: &mut Arm7Tdmi) {
    let discard_old = cpu.reg(0);
    let flags = cpu.reg(1) as u16;

    if discard_old != 0 {
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        let old = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        let cleared = old & !flags;
        let bytes = cleared.to_le_bytes();
        cpu.bus.iwram[offset] = bytes[0];
        cpu.bus.iwram[offset + 1] = bytes[1];
    }

    // the real BIOS forcefully sets IME=1 so the waited interrupt
    // can actually be delivered while halted
    cpu.bus.irq.set_ime(true);
    cpu.bus.intr_wait_flags = flags;
    cpu.set_halted(true);
}

/// SWI 0x05: VBlankIntrWait — waits for VBlank interrupt and is equivalent to IntrWait(1, 1).
///
/// Clears VBlank from IntrCheck and halts until VBlank arrives.
fn swi_vblank_intr_wait(cpu: &mut Arm7Tdmi) {
    let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
    let old = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
    let cleared = old & !1u16;
    let bytes = cleared.to_le_bytes();
    cpu.bus.iwram[offset] = bytes[0];
    cpu.bus.iwram[offset + 1] = bytes[1];

    // the real BIOS forcefully sets IME=1 so the waited interrupt
    // can actually be delivered while halted
    cpu.bus.irq.set_ime(true);
    cpu.bus.intr_wait_flags = 1; // wait for VBlank (IRQ bit 0)
    cpu.set_halted(true);
}

/// SWI 0x06: Div - signed division.
///
/// R0 = numerator, R1 = denominator.
/// Returns: R0 = result, R1 = remainder, R3 = abs(result).
fn swi_div(cpu: &mut Arm7Tdmi) {
    let num = cpu.reg(0) as i32;
    let den = cpu.reg(1) as i32;

    if den == 0 {
        // division by zero: return 0
        cpu.set_reg(0, 0);
        cpu.set_reg(1, num as u32);
        cpu.set_reg(3, 0);
        return;
    }

    let result = num.wrapping_div(den);
    let remainder = num.wrapping_rem(den);

    cpu.set_reg(0, result as u32);
    cpu.set_reg(1, remainder as u32);
    cpu.set_reg(3, result.unsigned_abs());
}

/// SWI 0x07: DivArm - same as Div but with swapped arguments.
///
/// R0 = denominator, R1 = numerator.
fn swi_div_arm(cpu: &mut Arm7Tdmi) {
    let den = cpu.reg(0) as i32;
    let num = cpu.reg(1) as i32;

    if den == 0 {
        cpu.set_reg(0, 0);
        cpu.set_reg(1, num as u32);
        cpu.set_reg(3, 0);
        return;
    }

    let result = num.wrapping_div(den);
    let remainder = num.wrapping_rem(den);

    cpu.set_reg(0, result as u32);
    cpu.set_reg(1, remainder as u32);
    cpu.set_reg(3, result.unsigned_abs());
}

/// SWI 0x08: Sqrt - integer square root.
///
/// R0 = value, returns R0 = sqrt(value).
fn swi_sqrt(cpu: &mut Arm7Tdmi) {
    let value = cpu.reg(0);
    let result = (value as f64).sqrt() as u32;
    cpu.set_reg(0, result);
}

/// SWI 0x09: ArcTan - arctangent.
///
/// R0 = tan (fixed point), returns R0 = angle.
fn swi_arctan(cpu: &mut Arm7Tdmi) {
    let tan = cpu.reg(0) as i16 as f64 / 16384.0;
    let angle = tan.atan();
    let result = (angle * 16384.0 / std::f64::consts::PI) as i16;
    cpu.set_reg(0, result as u32);
}

/// SWI 0x0A: ArcTan2 - arctangent of y/x.
///
/// R0 = x, R1 = y, returns R0 = angle (0x0000-0xFFFF).
fn swi_arctan2(cpu: &mut Arm7Tdmi) {
    let x = cpu.reg(0) as i16 as f64;
    let y = cpu.reg(1) as i16 as f64;
    let angle = y.atan2(x);
    // convert to 0x0000-0xFFFF range
    let result = (angle * 32768.0 / std::f64::consts::PI) as i16 as u16;
    cpu.set_reg(0, result as u32);
}

/// SWI 0x0B: CpuSet - memory copy/fill.
///
/// R0 = source, R1 = destination, R2 = length/mode.
fn swi_cpu_set(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let control = cpu.reg(2);

    let count = control & 0x001FFFFF;
    let is_fill = control & (1 << 24) != 0;
    let is_32bit = control & (1 << 26) != 0;

    if is_32bit {
        let fill_value = if is_fill { cpu.bus_read32(src) } else { 0 };
        for i in 0..count {
            let value = if is_fill {
                fill_value
            } else {
                cpu.bus_read32(src + i * 4)
            };
            cpu.bus_write32(dst + i * 4, value);
        }
    } else {
        let fill_value = if is_fill { cpu.bus_read16(src) } else { 0 };
        for i in 0..count {
            let value = if is_fill {
                fill_value
            } else {
                cpu.bus_read16(src + i * 2)
            };
            cpu.bus_write16(dst + i * 2, value);
        }
    }
}

/// SWI 0x0C: CpuFastSet - fast memory copy/fill (32-bit, 8-word aligned).
///
/// R0 = source, R1 = destination, R2 = length/mode.
fn swi_cpu_fast_set(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let control = cpu.reg(2);

    let count = ((control & 0x001FFFFF) + 7) & !7; // round up to 8-word boundary
    let is_fill = control & (1 << 24) != 0;

    let fill_value = if is_fill { cpu.bus_read32(src) } else { 0 };

    for i in 0..count {
        let value = if is_fill {
            fill_value
        } else {
            cpu.bus_read32(src + i * 4)
        };
        cpu.bus_write32(dst + i * 4, value);
    }
}

/// SWI 0x0D: GetBiosChecksum - returns the BIOS checksum.
fn swi_get_bios_checksum(cpu: &mut Arm7Tdmi) {
    cpu.set_reg(0, 0xBAAE187F);
}

/// SWI 0x0E: BgAffineSet - calculates BG affine transformation parameters.
fn swi_bg_affine_set(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let count = cpu.reg(2);

    for i in 0..count {
        let src_addr = src + i * 20;
        let dst_addr = dst + i * 16;

        let original_x = cpu.bus_read32(src_addr) as i32;
        let original_y = cpu.bus_read32(src_addr + 4) as i32;
        let display_x = cpu.bus_read16(src_addr + 8) as i16 as i32;
        let display_y = cpu.bus_read16(src_addr + 10) as i16 as i32;
        let scale_x = cpu.bus_read16(src_addr + 12) as i16 as f64 / 256.0;
        let scale_y = cpu.bus_read16(src_addr + 14) as i16 as f64 / 256.0;
        let angle_raw = cpu.bus_read16(src_addr + 16);
        let angle = (angle_raw as f64) * 2.0 * std::f64::consts::PI / 65536.0;

        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let pa = (cos_a / scale_x * 256.0) as i16;
        let pb = (-sin_a / scale_x * 256.0) as i16;
        let pc = (sin_a / scale_y * 256.0) as i16;
        let pd = (cos_a / scale_y * 256.0) as i16;

        let start_x = original_x - (pa as i32 * display_x + pb as i32 * display_y);
        let start_y = original_y - (pc as i32 * display_x + pd as i32 * display_y);

        cpu.bus_write16(dst_addr, pa as u16);
        cpu.bus_write16(dst_addr + 2, pb as u16);
        cpu.bus_write16(dst_addr + 4, pc as u16);
        cpu.bus_write16(dst_addr + 6, pd as u16);
        cpu.bus_write32(dst_addr + 8, start_x as u32);
        cpu.bus_write32(dst_addr + 12, start_y as u32);
    }
}

/// SWI 0x0F: ObjAffineSet - calculates OBJ affine transformation parameters.
fn swi_obj_affine_set(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let count = cpu.reg(2);
    let stride = cpu.reg(3);

    for i in 0..count {
        let src_addr = src + i * 8;

        let scale_x = cpu.bus_read16(src_addr) as i16 as f64 / 256.0;
        let scale_y = cpu.bus_read16(src_addr + 2) as i16 as f64 / 256.0;
        let angle_raw = cpu.bus_read16(src_addr + 4);
        let angle = (angle_raw as f64) * 2.0 * std::f64::consts::PI / 65536.0;

        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let pa = (cos_a / scale_x * 256.0) as i16;
        let pb = (-sin_a / scale_x * 256.0) as i16;
        let pc = (sin_a / scale_y * 256.0) as i16;
        let pd = (cos_a / scale_y * 256.0) as i16;

        let base = dst + i * stride * 4;
        cpu.bus_write16(base, pa as u16);
        cpu.bus_write16(base + stride, pb as u16);
        cpu.bus_write16(base + stride * 2, pc as u16);
        cpu.bus_write16(base + stride * 3, pd as u16);
    }
}

/// SWI 0x10: BitUnPack - bit unpacking.
///
/// R0 = source, R1 = destination, R2 = pointer to unpack info.
fn swi_bit_unpack(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let info = cpu.reg(2);

    let length = cpu.bus_read16(info) as u32;
    let src_width = cpu.bus_read8(info + 2) as u32;
    let dst_width = cpu.bus_read8(info + 3) as u32;
    let data_offset = cpu.bus_read32(info + 4);

    let zero_flag = data_offset & (1 << 31) != 0;
    let offset = data_offset & 0x7FFF_FFFF;

    if src_width == 0 || dst_width == 0 || dst_width > 32 {
        return;
    }

    let src_mask = (1u32 << src_width) - 1;
    let mut src_offset = 0u32;
    let mut dst_offset = 0u32;
    let mut dst_buffer = 0u32;
    let mut dst_bits = 0u32;

    while src_offset < length {
        let byte = cpu.bus_read8(src + src_offset);
        src_offset += 1;

        let mut bit_pos = 0u32;
        while bit_pos < 8 {
            let value = ((byte as u32) >> bit_pos) & src_mask;
            bit_pos += src_width;

            let unpacked = if zero_flag || value != 0 {
                value + offset
            } else {
                0
            };

            dst_buffer |= (unpacked & ((1u32 << dst_width) - 1)) << dst_bits;
            dst_bits += dst_width;

            if dst_bits >= 32 {
                cpu.bus_write32(dst + dst_offset, dst_buffer);
                dst_offset += 4;
                dst_buffer = 0;
                dst_bits = 0;
            }
        }
    }

    // flush any remaining bits
    if dst_bits > 0 {
        cpu.bus_write32(dst + dst_offset, dst_buffer);
    }
}

/// SWI 0x11: LZ77UnCompWram - LZ77 decompression to WRAM.
fn swi_lz77_decomp_wram(cpu: &mut Arm7Tdmi) {
    lz77_decomp(cpu, false);
}

/// SWI 0x12: LZ77UnCompVram - LZ77 decompression to VRAM (16-bit writes).
fn swi_lz77_decomp_vram(cpu: &mut Arm7Tdmi) {
    lz77_decomp(cpu, true);
}

/// Shared LZ77 decompression logic.
fn lz77_decomp(cpu: &mut Arm7Tdmi, vram_mode: bool) {
    let src = cpu.reg(0);
    let mut dst = cpu.reg(1);

    // read header
    let header = cpu.bus_read32(src);
    let decomp_size = header >> 8;
    let mut src_offset = 4u32;
    let mut bytes_written = 0u32;
    let mut buffer = Vec::new();

    while bytes_written < decomp_size {
        let flags = cpu.bus_read8(src + src_offset);
        src_offset += 1;

        for bit in (0..8).rev() {
            if bytes_written >= decomp_size {
                break;
            }

            if flags & (1 << bit) != 0 {
                // compressed: reference to previous data
                let b1 = cpu.bus_read8(src + src_offset) as u32;
                let b2 = cpu.bus_read8(src + src_offset + 1) as u32;
                src_offset += 2;

                let length = ((b1 >> 4) + 3) as usize;
                let offset = (((b1 & 0x0F) << 8) | b2) as usize + 1;

                if offset > buffer.len() {
                    warnln!("Invalid LZ77 back-reference offset {}", offset);
                    return;
                }

                for _ in 0..length {
                    if bytes_written >= decomp_size {
                        break;
                    }
                    let index = buffer.len() - offset;
                    let byte = buffer[index];
                    buffer.push(byte);

                    if vram_mode {
                        // buffer and write 16 bits at a time
                        if buffer.len() % 2 == 0 {
                            let len = buffer.len();
                            let value = (buffer[len - 2] as u16) | ((buffer[len - 1] as u16) << 8);
                            cpu.bus_write16(dst, value);
                            dst += 2;
                        }
                    } else {
                        cpu.bus_write8(dst, byte);
                        dst += 1;
                    }
                    bytes_written += 1;
                }
            } else {
                // uncompressed: literal byte
                let byte = cpu.bus_read8(src + src_offset);
                src_offset += 1;
                buffer.push(byte);

                if vram_mode {
                    if buffer.len() % 2 == 0 {
                        let len = buffer.len();
                        let value = (buffer[len - 2] as u16) | ((buffer[len - 1] as u16) << 8);
                        cpu.bus_write16(dst, value);
                        dst += 2;
                    }
                } else {
                    cpu.bus_write8(dst, byte);
                    dst += 1;
                }
                bytes_written += 1;
            }
        }
    }

    // flush any remaining byte in the buffer (odd-sized decomp)
    if vram_mode && buffer.len() % 2 == 1 {
        let value = *buffer.last().unwrap() as u16;
        cpu.bus_write16(dst, value);
    }
}

/// SWI 0x13: HuffUnComp - huffman decompression.
///
/// R0 = source, R1 = destination.
fn swi_huff_decomp(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let mut dst = cpu.reg(1);

    let header = cpu.bus_read32(src);
    let data_size = header >> 8;
    let bit_size = (header >> 4) & 0x0F;

    if bit_size != 4 && bit_size != 8 {
        return;
    }

    let tree_size = cpu.bus_read8(src + 4) as u32;
    let tree_start = src + 5;
    let tree_bytes = (tree_size + 1) * 2;
    let data_start = tree_start + tree_bytes;

    let mut src_offset = data_start;
    let mut bytes_written = 0u32;
    let mut dst_buffer = 0u32;
    let mut dst_bits = 0u32;
    let mut bit_buffer = 0u32;
    let mut bits_left = 0u32;

    while bytes_written < data_size {
        // refill bit buffer
        if bits_left == 0 {
            bit_buffer = cpu.bus_read32(src_offset);
            src_offset += 4;
            bits_left = 32;
        }

        // traverse the tree from root
        let mut node_offset = 0u32;
        loop {
            let node = cpu.bus_read8(tree_start + node_offset);

            if bits_left == 0 {
                bit_buffer = cpu.bus_read32(src_offset);
                src_offset += 4;
                bits_left = 32;
            }

            // read one bit (MSB first)
            let bit = (bit_buffer >> 31) & 1;
            bit_buffer <<= 1;
            bits_left -= 1;

            let is_right = bit != 0;
            let child_offset = (node & 0x3F) as u32;
            let next = (node_offset & !1) + child_offset * 2 + 2;

            let is_leaf = if is_right {
                node & 0x80 != 0
            } else {
                node & 0x40 != 0
            };

            if is_leaf {
                let leaf = cpu.bus_read8(tree_start + next + if is_right { 1 } else { 0 });
                dst_buffer |= (leaf as u32) << dst_bits;
                dst_bits += bit_size;

                if dst_bits >= 32 {
                    cpu.bus_write32(dst, dst_buffer);
                    dst += 4;
                    bytes_written += 4;
                    dst_buffer = 0;
                    dst_bits = 0;
                }
                break;
            } else {
                node_offset = next + if is_right { 1 } else { 0 };
            }
        }
    }
}

/// SWI 0x14: RLUnCompWram - run-length decompression to WRAM.
fn swi_rl_decomp_wram(cpu: &mut Arm7Tdmi) {
    rl_decomp(cpu, false);
}

/// SWI 0x15: RLUnCompVram - run-length decompression to VRAM.
fn swi_rl_decomp_vram(cpu: &mut Arm7Tdmi) {
    rl_decomp(cpu, true);
}

/// Shared run-length decompression logic.
#[allow(unused_assignments)]
fn rl_decomp(cpu: &mut Arm7Tdmi, vram_mode: bool) {
    let src = cpu.reg(0);
    let mut dst = cpu.reg(1);

    let header = cpu.bus_read32(src);
    let decomp_size = header >> 8;
    let mut src_offset = 4u32;
    let mut bytes_written = 0u32;
    let mut vram_buffer: u16 = 0;
    let mut vram_count: u32 = 0;

    while bytes_written < decomp_size {
        let flag = cpu.bus_read8(src + src_offset);
        src_offset += 1;

        if flag & 0x80 != 0 {
            // compressed run
            let length = (flag & 0x7F) as u32 + 3;
            let data = cpu.bus_read8(src + src_offset);
            src_offset += 1;

            for _ in 0..length {
                if bytes_written >= decomp_size {
                    break;
                }
                if vram_mode {
                    if vram_count & 1 == 0 {
                        vram_buffer = data as u16;
                    } else {
                        vram_buffer |= (data as u16) << 8;
                        cpu.bus_write16(dst, vram_buffer);
                        dst += 2;
                    }
                    vram_count += 1;
                } else {
                    cpu.bus_write8(dst, data);
                    dst += 1;
                }
                bytes_written += 1;
            }
        } else {
            // uncompressed run
            let length = (flag & 0x7F) as u32 + 1;
            for _ in 0..length {
                if bytes_written >= decomp_size {
                    break;
                }
                let data = cpu.bus_read8(src + src_offset);
                src_offset += 1;

                if vram_mode {
                    if vram_count & 1 == 0 {
                        vram_buffer = data as u16;
                    } else {
                        vram_buffer |= (data as u16) << 8;
                        cpu.bus_write16(dst, vram_buffer);
                        dst += 2;
                    }
                    vram_count += 1;
                } else {
                    cpu.bus_write8(dst, data);
                    dst += 1;
                }
                bytes_written += 1;
            }
        }
    }

    // flush any remaining byte in the buffer (odd-sized decomp)
    if vram_mode && vram_count & 1 != 0 {
        cpu.bus_write16(dst, vram_buffer);
    }
}

/// SWI 0x16: DiffUnFilter8 - 8-bit differential unfilter to WRAM.
fn swi_diff_unfilt8_wram(cpu: &mut Arm7Tdmi) {
    diff_unfilt8(cpu, false);
}

/// SWI 0x17: DiffUnFilter8 - 8-bit differential unfilter to VRAM (16-bit writes).
fn swi_diff_unfilt8_vram(cpu: &mut Arm7Tdmi) {
    diff_unfilt8(cpu, true);
}

/// Shared 8-bit differential unfilter logic.
fn diff_unfilt8(cpu: &mut Arm7Tdmi, vram_mode: bool) {
    let src = cpu.reg(0);
    let mut dst = cpu.reg(1);

    let header = cpu.bus_read32(src);
    let size = header >> 8;
    let mut src_offset = 4u32;
    let mut bytes_written = 0u32;
    let mut accum = 0u8;
    let mut vram_buffer: u16 = 0;
    let mut vram_count: u32 = 0;

    while bytes_written < size {
        let byte = cpu.bus_read8(src + src_offset);
        src_offset += 1;
        accum = accum.wrapping_add(byte);

        if vram_mode {
            if vram_count & 1 == 0 {
                vram_buffer = accum as u16;
            } else {
                vram_buffer |= (accum as u16) << 8;
                cpu.bus_write16(dst, vram_buffer);
                dst += 2;
            }
            vram_count += 1;
        } else {
            cpu.bus_write8(dst, accum);
            dst += 1;
        }
        bytes_written += 1;
    }

    // flush any remaining byte in the buffer (odd-sized data)
    if vram_mode && vram_count & 1 != 0 {
        cpu.bus_write16(dst, vram_buffer);
    }
}

/// SWI 0x18: DiffUnFilter16 - 16-bit differential unfilter.
fn swi_diff_unfilt16(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let mut dst = cpu.reg(1);

    let header = cpu.bus_read32(src);
    let size = header >> 8;
    let mut src_offset = 4u32;
    let mut bytes_written = 0u32;
    let mut accum = 0u16;

    while bytes_written < size {
        let value = cpu.bus_read16(src + src_offset);
        src_offset += 2;
        accum = accum.wrapping_add(value);
        cpu.bus_write16(dst, accum);
        dst += 2;
        bytes_written += 2;
    }
}

/// SWI 0x19: SoundBias - adjusts the sound bias.
///
/// R0 selects the target: 0 sets SOUNDBIAS=0x000, non-zero sets SOUNDBIAS=0x200.
/// On real hardware this gradually fades with delays to avoid pops.
/// HLE shortcut: set the bias immediately, preserving upper bits.
fn swi_sound_bias(cpu: &mut Arm7Tdmi) {
    let flag = cpu.reg(0);
    let current = cpu.bus.apu.soundbias();
    let target_bias: u16 = if flag == 0 { 0x000 } else { 0x200 };

    // preserve upper bits (amplitude resolution), replace bias level
    let new_value = (current & 0xC000) | (target_bias & 0x3FFF);
    cpu.bus.apu.set_soundbias(new_value);
}

/// SWI 0x1F: MidiKey2Freq - converts MIDI key to frequency.
///
/// R0 = wave data pointer, R1 = MIDI key, R2 = pitch adjust (fp).
fn swi_midi_key2freq(cpu: &mut Arm7Tdmi) {
    let wave = cpu.reg(0);
    let mk = cpu.reg(1);
    let fp = cpu.reg(2);

    // read the frequency from the wave data header (at offset 4)
    let freq = cpu.bus_read32(wave + 4);

    // formula: freq * 2^((mk - 180) / 12 + fp / 2^16 / 12)
    let exponent = ((mk as f64) - 180.0) / 12.0 + (fp as f64) / 65536.0 / 12.0;
    let result = (freq as f64) * (2.0f64).powf(exponent);

    cpu.set_reg(0, (result as u32) & 0x7FFF_FFFF);
}

#[cfg(test)]
mod tests {
    use super::handle_swi;
    use crate::gba::{bus::GbaBus, cpu::Arm7Tdmi};

    fn make_cpu() -> Arm7Tdmi {
        Arm7Tdmi::new(GbaBus::new())
    }

    #[test]
    fn test_swi_soft_reset() {
        let mut cpu = make_cpu();
        for reg in 0..=12 {
            cpu.set_reg(reg, 0xDEAD_BEEF);
        }
        let redirected = handle_swi(&mut cpu, 0x00);
        assert!(redirected);
        // jumps to the ROM entry point in ARM system mode
        assert_eq!(cpu.pc(), 0x0800_0000);
        assert_eq!(cpu.cpsr() & 0x1F, 0x1F); // MODE_SYS
        assert!(cpu.cpsr() & 0x20 == 0); // ARM mode (T bit cleared)

        // r0-r12 are zeroed and LR is cleared
        for reg in 0..=12 {
            assert_eq!(cpu.reg(reg), 0);
        }
        assert_eq!(cpu.reg(14), 0);
        // banked stack pointers match the post-BIOS state
        assert_eq!(cpu.reg(13), 0x0300_7F00); // SP_SYS
        cpu.set_cpsr(0x92);
        assert_eq!(cpu.reg(13), 0x0300_7FA0); // SP_IRQ
        cpu.set_cpsr(0x93);
        assert_eq!(cpu.reg(13), 0x0300_7FE0); // SP_SVC
    }

    #[test]
    fn test_swi_soft_reset_clears_bios_ram() {
        let mut cpu = make_cpu();
        cpu.bus_write32(0x0300_7E00, 0xCAFE_BABE);
        cpu.bus_write32(0x0300_7FF8, 0xCAFE_BABE);
        handle_swi(&mut cpu, 0x00);
        assert_eq!(cpu.bus_read32(0x0300_7E00), 0);
        assert_eq!(cpu.bus_read32(0x0300_7FF8), 0);
    }

    #[test]
    fn test_swi_soft_reset_multiboot_flag() {
        let mut cpu = make_cpu();
        // a non-zero return address flag selects the RAM entry point
        cpu.bus_write8(0x0300_7FFA, 1);
        handle_swi(&mut cpu, 0x00);
        assert_eq!(cpu.pc(), 0x0200_0000);
    }

    #[test]
    fn test_swi_register_ram_reset_forces_blank() {
        let mut cpu = make_cpu();
        cpu.bus_write16(0x0400_0000, 0x1234);
        cpu.set_reg(0, 0);
        handle_swi(&mut cpu, 0x01);
        // DISPCNT is always destroyed (set to forced blank), even
        // with no reset flags requested
        assert_eq!(cpu.bus_read16(0x0400_0000), 0x0080);
    }

    #[test]
    fn test_swi_register_ram_reset_memory_flags() {
        for flags in [0, 1, 2, 4, 8, 16, 31, 0xFF] {
            let mut cpu = make_cpu();
            for (addr, size) in [
                (0x0200_0000, 0x40000),
                (0x0300_0000, 0x7E00),
                (0x0500_0000, 0x400),
                (0x0600_0000, 0x18000),
                (0x0700_0000, 0x400),
            ] {
                cpu.bus_write32(addr, 0xDEADBEEF);
                cpu.bus_write32(addr + size - 4, 0xDEADBEEF);
            }
            cpu.bus_write32(0x0300_7E00, 0x12345678);
            cpu.set_reg(0, flags);
            handle_swi(&mut cpu, 0x01);
            for (index, (addr, size)) in [
                (0x0200_0000, 0x40000),
                (0x0300_0000, 0x7E00),
                (0x0500_0000, 0x400),
                (0x0600_0000, 0x18000),
                (0x0700_0000, 0x400),
            ]
            .iter()
            .enumerate()
            {
                let expected = if flags & (1 << index) != 0 {
                    0
                } else {
                    0xDEADBEEF
                };
                assert_eq!(cpu.bus_read32(*addr), expected);
                assert_eq!(cpu.bus_read32(addr + size - 4), expected);
            }
            assert_eq!(cpu.bus_read32(0x0300_7E00), 0x12345678);
        }
    }

    #[test]
    fn test_swi_register_ram_reset_io_flags() {
        for flags in [0, 0x20, 0x40, 0x80, 0xE0] {
            let mut cpu = make_cpu();
            cpu.bus_write16(0x0400_0128, 0x4000);
            cpu.bus_write16(0x0400_0134, 0);
            cpu.bus_write16(0x0400_012A, 0x1234);
            cpu.bus_write16(0x0400_0080, 0x1177);
            cpu.bus_write16(0x0400_0082, 0x0304);
            cpu.bus_write16(0x0400_0084, 0x80);
            cpu.bus_write16(0x0400_0088, 0x0100);
            for bank in [0x40, 0] {
                cpu.bus_write16(0x0400_0070, bank);
                for addr in (0x0400_0090..0x0400_00A0).step_by(4) {
                    cpu.bus_write32(addr, 0xDEADBEEF);
                }
            }
            cpu.bus_write16(0x0400_0008, 0x1234);
            cpu.bus_write16(0x0400_0010, 42);
            cpu.bus_write16(0x0400_0020, 0x1234);
            cpu.bus_write16(0x0400_0048, 0x3F3F);
            cpu.bus_write16(0x0400_004C, 0x1234);
            cpu.bus_write16(0x0400_0050, 0x1234);
            cpu.bus_write32(0x0400_00B0, 0x0200_0000);
            cpu.bus_write16(0x0400_00B8, 2);
            cpu.bus_write16(0x0400_00BA, 0x8000);
            cpu.bus_write16(0x0400_0100, 0x1234);
            cpu.bus_write16(0x0400_0102, 0x80);
            cpu.bus_write16(0x0400_0200, 1);
            cpu.bus.irq.raise_vblank();
            cpu.bus_write16(0x0400_0204, 0x1234);
            cpu.bus_write16(0x0400_0208, 1);
            cpu.set_reg(0, flags);
            handle_swi(&mut cpu, 0x01);

            let serial = flags & 0x20 != 0;
            assert_eq!(cpu.bus_read16(0x0400_0128), if serial { 4 } else { 0x4004 });
            assert_eq!(cpu.bus_read16(0x0400_0134), if serial { 0x8000 } else { 0 });
            cpu.bus_write16(0x0400_0134, 0);
            cpu.bus_write16(0x0400_0128, 0x2080);
            assert_eq!(cpu.bus_read16(0x0400_0120), if serial { 0 } else { 0x1234 });
            let sound = flags & 0x40 != 0;
            assert_eq!(cpu.bus.apu.soundcnt_l(), if sound { 0 } else { 0x1177 });
            assert_eq!(cpu.bus.apu.soundcnt_h(), if sound { 0 } else { 0x0304 });
            assert_eq!(cpu.bus.apu.soundcnt_x(), if sound { 0x70 } else { 0xF0 });
            assert_eq!(cpu.bus.apu.soundbias(), if sound { 0x0200 } else { 0x0100 });
            for bank in [0x40, 0] {
                cpu.bus_write16(0x0400_0070, bank);
                assert_eq!(
                    cpu.bus_read32(0x0400_0090),
                    if sound { 0 } else { 0xDEADBEEF }
                );
                assert_eq!(
                    cpu.bus_read32(0x0400_009C),
                    if sound { 0 } else { 0xDEADBEEF }
                );
            }
            let other = flags & 0x80 != 0;
            assert_eq!(cpu.bus_read16(0x0400_0000), 0x0080);
            assert_eq!(cpu.bus_read16(0x0400_0008), if other { 0 } else { 0x1234 });
            assert_eq!(cpu.bus.ppu.bg_hofs(0), if other { 0 } else { 42 });
            assert_eq!(cpu.bus.ppu.bg_pa(0), if other { 0x0100 } else { 0x1234 });
            assert_eq!(cpu.bus.ppu.bg_pd(0), 0x0100);
            assert_eq!(cpu.bus.ppu.bg_pa(1), 0x0100);
            assert_eq!(cpu.bus.ppu.bg_pd(1), 0x0100);
            assert_eq!(cpu.bus.ppu.winin(), if other { 0 } else { 0x3F3F });
            assert_eq!(cpu.bus.ppu.mosaic(), if other { 0 } else { 0x1234 });
            assert_eq!(cpu.bus.ppu.bldcnt(), if other { 0 } else { 0x1234 });
            assert_eq!(
                cpu.bus.dma.channels[0].src_reg(),
                if other { 0 } else { 0x0200_0000 }
            );
            assert_eq!(
                cpu.bus.dma.channels[0].count_reg(),
                if other { 0 } else { 2 }
            );
            assert_eq!(cpu.bus.dma.channels[0].active(), !other);
            assert_eq!(
                cpu.bus.timers.timers[0].reload(),
                if other { 0 } else { 0x1234 }
            );
            assert_eq!(cpu.bus.timers.timers[0].enabled(), !other);
            assert_eq!(cpu.bus_read16(0x0400_0200), if other { 0 } else { 1 });
            assert_eq!(cpu.bus_read16(0x0400_0202), if other { 0 } else { 1 });
            assert_eq!(cpu.bus_read16(0x0400_0204), if other { 0 } else { 0x1234 });
            assert_eq!(cpu.bus_read16(0x0400_0208), if other { 0 } else { 1 });
        }
    }

    #[test]
    fn test_swi_div() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 10);
        cpu.set_reg(1, 3);
        handle_swi(&mut cpu, 0x06);
        assert_eq!(cpu.reg(0) as i32, 3);
        assert_eq!(cpu.reg(1) as i32, 1);
        assert_eq!(cpu.reg(3), 3);
    }

    #[test]
    fn test_swi_div_negative() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, (-10i32) as u32);
        cpu.set_reg(1, 3);
        handle_swi(&mut cpu, 0x06);
        assert_eq!(cpu.reg(0) as i32, -3);
        assert_eq!(cpu.reg(1) as i32, -1);
        assert_eq!(cpu.reg(3), 3);
    }

    #[test]
    fn test_swi_div_by_zero() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 42);
        cpu.set_reg(1, 0);
        handle_swi(&mut cpu, 0x06);
        assert_eq!(cpu.reg(0), 0);
        assert_eq!(cpu.reg(3), 0);
    }

    #[test]
    fn test_swi_div_arm() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 3);
        cpu.set_reg(1, 10);
        handle_swi(&mut cpu, 0x07);
        assert_eq!(cpu.reg(0) as i32, 3);
        assert_eq!(cpu.reg(1) as i32, 1);
    }

    #[test]
    fn test_swi_div_signed_limits() {
        for comment in [0x06, 0x07] {
            for (num, den, quotient, remainder) in [
                (i32::MIN, -1, i32::MIN, 0),
                (i32::MIN, 1, i32::MIN, 0),
                (i32::MIN, i32::MAX, -1, -1),
                (i32::MAX, -1, -i32::MAX, 0),
                (10, -3, -3, 1),
                (-10, -3, 3, -1),
                (0, -1, 0, 0),
                (i32::MIN, 0, 0, i32::MIN),
            ] {
                let mut cpu = make_cpu();
                let (r0, r1) = if comment == 0x06 {
                    (num, den)
                } else {
                    (den, num)
                };
                cpu.set_reg(0, r0 as u32);
                cpu.set_reg(1, r1 as u32);
                assert!(!handle_swi(&mut cpu, comment));
                assert_eq!(cpu.reg(0) as i32, quotient);
                assert_eq!(cpu.reg(1) as i32, remainder);
                assert_eq!(cpu.reg(3), quotient.unsigned_abs());
            }
        }
    }

    #[test]
    fn test_swi_sqrt() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 144);
        handle_swi(&mut cpu, 0x08);
        assert_eq!(cpu.reg(0), 12);
    }

    #[test]
    fn test_swi_sqrt_zero() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 0);
        handle_swi(&mut cpu, 0x08);
        assert_eq!(cpu.reg(0), 0);
    }

    #[test]
    fn test_swi_halt() {
        let mut cpu = make_cpu();
        assert!(!cpu.halted());
        handle_swi(&mut cpu, 0x02);
        assert!(cpu.halted());
    }

    #[test]
    fn test_swi_vblank_intr_wait() {
        let mut cpu = make_cpu();
        handle_swi(&mut cpu, 0x05);
        assert!(cpu.halted());
        // should set intr_wait_flags to VBlank (bit 0)
        assert_eq!(cpu.bus.intr_wait_flags, 1);
    }

    #[test]
    fn test_swi_vblank_intr_wait_clears_intr_check() {
        let mut cpu = make_cpu();
        // pre-set VBlank bit in IntrCheck at 0x03007FF8
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        cpu.bus.iwram[offset] = 0x03; // VBlank | HBlank
        cpu.bus.iwram[offset + 1] = 0x00;

        handle_swi(&mut cpu, 0x05);

        // VBlank bit should be cleared, HBlank preserved
        let check = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        assert_eq!(check, 0x02); // only HBlank remains
    }

    #[test]
    fn test_swi_intr_wait_sets_flags_and_halts() {
        let mut cpu = make_cpu();
        cpu.set_reg(0, 1); // discard_old = true
        cpu.set_reg(1, 0x04); // wait for VCount (bit 2)
        handle_swi(&mut cpu, 0x04);
        assert!(cpu.halted());
        assert_eq!(cpu.bus.intr_wait_flags, 0x04);
    }

    #[test]
    fn test_swi_intr_wait_discard_old_clears_intr_check() {
        let mut cpu = make_cpu();
        // pre-set VCount and VBlank in IntrCheck
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        cpu.bus.iwram[offset] = 0x05; // VBlank | VCount
        cpu.bus.iwram[offset + 1] = 0x00;

        cpu.set_reg(0, 1); // discard_old = true
        cpu.set_reg(1, 0x04); // wait for VCount
        handle_swi(&mut cpu, 0x04);

        // VCount bit cleared, VBlank preserved
        let check = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        assert_eq!(check, 0x01);
    }

    #[test]
    fn test_swi_intr_wait_no_discard_preserves_intr_check() {
        let mut cpu = make_cpu();
        // pre-set VCount in IntrCheck
        let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
        cpu.bus.iwram[offset] = 0x04; // VCount
        cpu.bus.iwram[offset + 1] = 0x00;

        cpu.set_reg(0, 0); // discard_old = false
        cpu.set_reg(1, 0x04); // wait for VCount
        handle_swi(&mut cpu, 0x04);

        // IntrCheck unchanged (discard_old == 0)
        let check = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
        assert_eq!(check, 0x04);
    }

    #[test]
    fn test_swi_cpu_set_copy_16bit() {
        let mut cpu = make_cpu();
        cpu.bus_write16(0x0200_0000, 0x1234);
        cpu.bus_write16(0x0200_0002, 0x5678);
        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 0x0200_1000);
        cpu.set_reg(2, 2);
        handle_swi(&mut cpu, 0x0B);
        assert_eq!(cpu.bus_read16(0x0200_1000), 0x1234);
        assert_eq!(cpu.bus_read16(0x0200_1002), 0x5678);
    }

    #[test]
    fn test_swi_cpu_set_fill_32bit() {
        let mut cpu = make_cpu();
        cpu.bus_write32(0x0200_0000, 0xDEADBEEF);
        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 0x0200_1000);
        cpu.set_reg(2, 4 | (1 << 24) | (1 << 26));
        handle_swi(&mut cpu, 0x0B);
        assert_eq!(cpu.bus_read32(0x0200_1000), 0xDEADBEEF);
        assert_eq!(cpu.bus_read32(0x0200_100C), 0xDEADBEEF);
    }

    #[test]
    fn test_swi_cpu_fast_set_copy() {
        let mut cpu = make_cpu();
        for i in 0..8u32 {
            cpu.bus_write32(0x0200_0000 + i * 4, i + 1);
        }
        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 0x0200_1000);
        cpu.set_reg(2, 8);
        handle_swi(&mut cpu, 0x0C);
        assert_eq!(cpu.bus_read32(0x0200_1000), 1);
        assert_eq!(cpu.bus_read32(0x0200_101C), 8);
    }

    #[test]
    fn test_swi_cpu_fast_set_rounds_up() {
        for (count, rounded) in [(0, 0), (1, 8), (7, 8), (8, 8), (9, 16)] {
            for fill in [false, true] {
                let mut cpu = make_cpu();
                for i in 0..17 {
                    cpu.bus_write32(0x0200_0000 + i * 4, i + 1);
                    cpu.bus_write32(0x0200_1000 + i * 4, 0xDEADBEEF);
                }
                cpu.set_reg(0, 0x0200_0000);
                cpu.set_reg(1, 0x0200_1000);
                cpu.set_reg(2, count | if fill { 1 << 24 } else { 0 });
                handle_swi(&mut cpu, 0x0C);
                for i in 0..rounded {
                    assert_eq!(
                        cpu.bus_read32(0x0200_1000 + i * 4),
                        if fill { 1 } else { i + 1 }
                    );
                }
                assert_eq!(cpu.bus_read32(0x0200_1000 + rounded * 4), 0xDEADBEEF);
            }
        }
    }

    #[test]
    fn test_swi_get_bios_checksum() {
        let mut cpu = make_cpu();
        handle_swi(&mut cpu, 0x0D);
        assert_eq!(cpu.reg(0), 0xBAAE187F);
    }

    #[test]
    fn test_swi_bit_unpack_1to8() {
        let mut cpu = make_cpu();

        // source: 2 bytes of 1-bit data
        cpu.bus_write8(0x0200_0000, 0b10110001);
        cpu.bus_write8(0x0200_0001, 0b00000001);

        // unpack info: length=2, src_width=1, dst_width=8, offset=0 with zero flag
        cpu.bus_write16(0x0200_0100, 2); // length
        cpu.bus_write8(0x0200_0102, 1); // src_width
        cpu.bus_write8(0x0200_0103, 8); // dst_width
        cpu.bus_write32(0x0200_0104, 0); // data_offset (no zero flag)

        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 0x0200_1000);
        cpu.set_reg(2, 0x0200_0100);
        handle_swi(&mut cpu, 0x10);

        // first 4 bytes: bits 0b10110001 -> [1, 0, 0, 0] (LSB first)
        assert_eq!(cpu.bus_read32(0x0200_1000), 0x00000001);
        // next 4 bytes: [1, 1, 0, 1]
        assert_eq!(cpu.bus_read32(0x0200_1004), 0x01000101);
    }

    #[test]
    fn test_swi_bit_unpack_4to8_with_offset() {
        let mut cpu = make_cpu();

        // source: 1 byte of 4-bit data (two nibbles: 0x3, 0x5)
        cpu.bus_write8(0x0200_0000, 0x53);

        // unpack info: length=1, src_width=4, dst_width=8, offset=1 with zero flag
        cpu.bus_write16(0x0200_0100, 1);
        cpu.bus_write8(0x0200_0102, 4);
        cpu.bus_write8(0x0200_0103, 8);
        cpu.bus_write32(0x0200_0104, 1 | (1 << 31)); // offset=1, zero_flag=true

        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 0x0200_1000);
        cpu.set_reg(2, 0x0200_0100);
        handle_swi(&mut cpu, 0x10);

        // nibble 0x3 + offset 1 = 4, nibble 0x5 + offset 1 = 6
        assert_eq!(cpu.bus_read32(0x0200_1000), 0x00000604);
    }

    #[test]
    fn test_swi_lz77_decomp() {
        for (comment, dst) in [(0x11, 0x0200_1000), (0x12, 0x0600_0000)] {
            for (data, expected) in [
                (vec![0x10, 0, 0, 0], vec![]),
                (vec![0x10, 3, 0, 0, 0, 1, 2, 3], vec![1, 2, 3]),
                (vec![0x10, 6, 0, 0, 0x40, 0xAB, 0xF0, 0], vec![0xAB; 6]),
                (vec![0x10, 5, 0, 0, 0x40, 0xAB, 0x10, 0], vec![0xAB; 5]),
            ] {
                let mut cpu = make_cpu();
                for (i, byte) in data.iter().enumerate() {
                    cpu.bus_write8(0x0200_0000 + i as u32, *byte);
                }
                cpu.bus_write16(dst + 8, 0xBEEF);
                cpu.set_reg(0, 0x0200_0000);
                cpu.set_reg(1, dst);
                handle_swi(&mut cpu, comment);
                for (i, byte) in expected.iter().enumerate() {
                    assert_eq!(cpu.bus_read8(dst + i as u32), *byte);
                }
                assert_eq!(cpu.bus_read8(dst + expected.len() as u32), 0);
                assert_eq!(cpu.bus_read16(dst + 8), 0xBEEF);
            }
        }
    }

    #[test]
    fn test_swi_lz77_decomp_invalid_offset() {
        for (comment, dst) in [(0x11, 0x0200_1000), (0x12, 0x0600_0000)] {
            for data in [
                vec![0x10, 4, 0, 0, 0x80, 0, 0],
                vec![0x10, 4, 0, 0, 0x40, 0xAB, 0, 1],
                vec![0x10, 4, 0, 0, 0x40, 0xAB, 0x0F, 0xFF],
            ] {
                let mut cpu = make_cpu();
                for (i, byte) in data.iter().enumerate() {
                    cpu.bus_write8(0x0200_0000 + i as u32, *byte);
                }
                cpu.bus_write32(dst, 0xDEADBEEF);
                cpu.set_reg(0, 0x0200_0000);
                cpu.set_reg(1, dst);
                assert!(!handle_swi(&mut cpu, comment));
                let expected = if comment == 0x11 && data[4] == 0x40 {
                    0xDEADBEAB
                } else {
                    0xDEADBEEF
                };
                assert_eq!(cpu.bus_read32(dst), expected);
            }
        }
    }

    #[test]
    fn test_swi_diff_unfilt8_wram() {
        let mut cpu = make_cpu();

        // header: type=0x81 (8-bit diff), size=4
        cpu.bus_write32(0x0200_0000, 0x00000481);
        // deltas: 10, 5, 3, 2
        cpu.bus_write8(0x0200_0004, 10);
        cpu.bus_write8(0x0200_0005, 5);
        cpu.bus_write8(0x0200_0006, 3);
        cpu.bus_write8(0x0200_0007, 2);

        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 0x0200_1000);
        handle_swi(&mut cpu, 0x16);

        // accumulated: 10, 15, 18, 20
        assert_eq!(cpu.bus_read8(0x0200_1000), 10);
        assert_eq!(cpu.bus_read8(0x0200_1001), 15);
        assert_eq!(cpu.bus_read8(0x0200_1002), 18);
        assert_eq!(cpu.bus_read8(0x0200_1003), 20);
    }

    #[test]
    fn test_swi_diff_unfilt16() {
        let mut cpu = make_cpu();

        // header: type=0x82 (16-bit diff), size=4 (2 halfwords = 4 bytes)
        cpu.bus_write32(0x0200_0000, 0x00000482);
        // deltas: 100, 50
        cpu.bus_write16(0x0200_0004, 100);
        cpu.bus_write16(0x0200_0006, 50);

        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 0x0200_1000);
        handle_swi(&mut cpu, 0x18);

        // accumulated: 100, 150
        assert_eq!(cpu.bus_read16(0x0200_1000), 100);
        assert_eq!(cpu.bus_read16(0x0200_1002), 150);
    }

    #[test]
    fn test_swi_sound_bias() {
        let mut cpu = make_cpu();

        // R0 != 0 sets SOUNDBIAS to 0x200
        cpu.set_reg(0, 1);
        handle_swi(&mut cpu, 0x19);
        assert_eq!(cpu.bus.apu.soundbias(), 0x200);

        // R0 == 0 sets SOUNDBIAS to 0x000
        cpu.set_reg(0, 0);
        handle_swi(&mut cpu, 0x19);
        assert_eq!(cpu.bus.apu.soundbias(), 0x000);
    }

    #[test]
    fn test_swi_midi_key2freq() {
        let mut cpu = make_cpu();

        // write a wave data header with frequency at offset 4
        cpu.bus_write32(0x0200_0000, 0); // dummy first word
        cpu.bus_write32(0x0200_0004, 7040); // base frequency (A7 = 7040 Hz)

        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 180); // MIDI key 180 = no shift
        cpu.set_reg(2, 0); // no pitch adjust
        handle_swi(&mut cpu, 0x1F);

        // at key 180 with no pitch adjust, result should be the base frequency
        assert_eq!(cpu.reg(0), 7040);
    }

    #[test]
    fn test_swi_midi_key2freq_octave_up() {
        let mut cpu = make_cpu();

        cpu.bus_write32(0x0200_0000, 0);
        cpu.bus_write32(0x0200_0004, 7040);

        cpu.set_reg(0, 0x0200_0000);
        cpu.set_reg(1, 192); // 180 + 12 = one octave up
        cpu.set_reg(2, 0);
        handle_swi(&mut cpu, 0x1F);

        // one octave up doubles the frequency
        assert_eq!(cpu.reg(0), 14080);
    }

    #[test]
    fn test_swi_unknown() {
        let mut cpu = make_cpu();
        handle_swi(&mut cpu, 0xFF);
    }
}
