//! GBA BIOS HLE (High-Level Emulation) of SWI (Software Interrupt) calls.
//!
//! Implements the most commonly used BIOS functions as native Rust code
//! rather than requiring a real BIOS ROM dump.

use crate::{gba::cpu::Arm7Tdmi, warnln};

/// Handles a SWI call by dispatching to the appropriate HLE function.
/// the comment field identifies which SWI is being called.
pub fn handle_swi(cpu: &mut Arm7Tdmi, comment: u8) {
    match comment {
        0x00 => swi_soft_reset(cpu),
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
}

/// SWI 0x00: SoftReset - resets the system.
fn swi_soft_reset(cpu: &mut Arm7Tdmi) {
    // clear IWRAM 0x03007E00-0x03007FFF
    for addr in (0x0300_7E00u32..0x0300_8000).step_by(4) {
        cpu.bus_write32(addr, 0);
    }

    // set registers to reset state
    cpu.set_reg(13, 0x0300_7F00); // SP_IRQ
    cpu.set_cpsr(0x1F); // system mode
    cpu.set_reg(13, 0x0300_7FE0); // SP_SYS
    cpu.set_reg(14, 0);
    cpu.set_reg(15, 0x0800_0000); // jump to ROM entry
}

/// SWI 0x01: RegisterRamReset - clears specified memory regions.
fn swi_register_ram_reset(cpu: &mut Arm7Tdmi) {
    let flags = cpu.reg(0);

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
/// sets intr_wait_flags so the re-halt check in cpu.rs keeps the CPU halted until matched.
///
/// r0 = discard_old, r1 = interrupt flags to wait for
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

    cpu.bus.intr_wait_flags = flags;
    cpu.set_halted(true);
}

/// SWI 0x05: VBlankIntrWait — waits for VBlank interrupt and is equivalent to IntrWait(1, 1).
///
/// clears VBlank from IntrCheck and halts until VBlank arrives.
fn swi_vblank_intr_wait(cpu: &mut Arm7Tdmi) {
    let offset = (0x0300_7FF8u32 & 0x7FFF) as usize;
    let old = u16::from_le_bytes([cpu.bus.iwram[offset], cpu.bus.iwram[offset + 1]]);
    let cleared = old & !1u16;
    let bytes = cleared.to_le_bytes();
    cpu.bus.iwram[offset] = bytes[0];
    cpu.bus.iwram[offset + 1] = bytes[1];

    cpu.bus.intr_wait_flags = 1; // wait for VBlank (IRQ bit 0)
    cpu.set_halted(true);
}

/// SWI 0x06: Div - signed division.
///
/// r0 = numerator, r1 = denominator.
/// returns: r0 = result, r1 = remainder, r3 = abs(result).
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

    let result = num / den;
    let remainder = num % den;

    cpu.set_reg(0, result as u32);
    cpu.set_reg(1, remainder as u32);
    cpu.set_reg(3, result.unsigned_abs());
}

/// SWI 0x07: DivArm - same as Div but with swapped arguments.
///
/// r0 = denominator, r1 = numerator.
fn swi_div_arm(cpu: &mut Arm7Tdmi) {
    let den = cpu.reg(0) as i32;
    let num = cpu.reg(1) as i32;

    if den == 0 {
        cpu.set_reg(0, 0);
        cpu.set_reg(1, num as u32);
        cpu.set_reg(3, 0);
        return;
    }

    let result = num / den;
    let remainder = num % den;

    cpu.set_reg(0, result as u32);
    cpu.set_reg(1, remainder as u32);
    cpu.set_reg(3, result.unsigned_abs());
}

/// SWI 0x08: Sqrt - integer square root.
///
/// r0 = value, returns r0 = sqrt(value)
fn swi_sqrt(cpu: &mut Arm7Tdmi) {
    let value = cpu.reg(0);
    let result = (value as f64).sqrt() as u32;
    cpu.set_reg(0, result);
}

/// SWI 0x09: ArcTan - arctangent.
///
/// r0 = tan (fixed point), returns r0 = angle
fn swi_arctan(cpu: &mut Arm7Tdmi) {
    let tan = cpu.reg(0) as i16 as f64 / 16384.0;
    let angle = tan.atan();
    let result = (angle * 16384.0 / std::f64::consts::PI) as i16;
    cpu.set_reg(0, result as u32);
}

/// SWI 0x0A: ArcTan2 - arctangent of y/x.
///
/// r0 = x, r1 = y, returns r0 = angle (0x0000-0xFFFF)
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
/// r0 = source, r1 = destination, r2 = length/mode
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
/// r0 = source, r1 = destination, r2 = length/mode
fn swi_cpu_fast_set(cpu: &mut Arm7Tdmi) {
    let src = cpu.reg(0);
    let dst = cpu.reg(1);
    let control = cpu.reg(2);

    let count = (control & 0x001FFFFF) & !7; // round down to 8-word boundary
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
/// r0 = source, r1 = destination, r2 = pointer to unpack info
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

/// shared LZ77 decompression logic
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
/// r0 = source, r1 = destination
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

/// shared run-length decompression logic
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

/// shared 8-bit differential unfilter logic
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
fn swi_sound_bias(cpu: &mut Arm7Tdmi) {
    let _bias = cpu.reg(0);
    // simplified: sound bias is handled by the APU directly
}

/// SWI 0x1F: MidiKey2Freq - converts MIDI key to frequency.
///
/// r0 = wave data pointer, r1 = MIDI key, r2 = pitch adjust (fp)
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
        cpu.set_reg(0, 0x200);
        handle_swi(&mut cpu, 0x19);
        // should not crash; SWI 0x19 is a no-op in HLE
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
