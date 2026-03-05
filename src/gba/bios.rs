//! GBA BIOS HLE (High-Level Emulation) of SWI (Software Interrupt) calls.
//!
//! Implements the most commonly used BIOS functions as native Rust code
//! rather than requiring a real BIOS ROM dump.

use crate::gba::cpu::Arm7Tdmi;

/// handles a SWI call by dispatching to the appropriate HLE function.
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
        0x0A => swi_arctan2(cpu),
        0x0B => swi_cpu_set(cpu),
        0x0C => swi_cpu_fast_set(cpu),
        0x0E => swi_bg_affine_set(cpu),
        0x0F => swi_obj_affine_set(cpu),
        0x11 => swi_lz77_decomp_wram(cpu),
        0x12 => swi_lz77_decomp_vram(cpu),
        0x14 => swi_rl_decomp_wram(cpu),
        0x15 => swi_rl_decomp_vram(cpu),
        _ => {
            // unsupported SWI - just return
        }
    }
}

/// SWI 0x00: SoftReset - resets the system
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

/// SWI 0x01: RegisterRamReset - clears specified memory regions
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

/// SWI 0x02: Halt - halts the CPU until an interrupt occurs
fn swi_halt(cpu: &mut Arm7Tdmi) {
    cpu.set_halted(true);
}

/// SWI 0x03: Stop - low-power stop mode (treat as halt)
fn swi_stop(cpu: &mut Arm7Tdmi) {
    cpu.set_halted(true);
}

/// SWI 0x04: IntrWait - waits for specified interrupt(s)
fn swi_intr_wait(cpu: &mut Arm7Tdmi) {
    let _discard_old = cpu.reg(0);
    let _flags = cpu.reg(1);
    // simplified: just halt and let the main loop handle it
    cpu.set_halted(true);
}

/// SWI 0x05: VBlankIntrWait - waits for VBlank interrupt
fn swi_vblank_intr_wait(cpu: &mut Arm7Tdmi) {
    cpu.set_halted(true);
}

/// SWI 0x06: Div - signed division
/// r0 = numerator, r1 = denominator
/// returns: r0 = result, r1 = remainder, r3 = abs(result)
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

/// SWI 0x07: DivArm - same as Div but with swapped arguments
/// r0 = denominator, r1 = numerator
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

/// SWI 0x08: Sqrt - integer square root
/// r0 = value, returns r0 = sqrt(value)
fn swi_sqrt(cpu: &mut Arm7Tdmi) {
    let value = cpu.reg(0);
    let result = (value as f64).sqrt() as u32;
    cpu.set_reg(0, result);
}

/// SWI 0x09: ArcTan - arctangent
/// r0 = tan (fixed point), returns r0 = angle
fn swi_arctan(cpu: &mut Arm7Tdmi) {
    let tan = cpu.reg(0) as i16 as f64 / 16384.0;
    let angle = tan.atan();
    let result = (angle * 16384.0 / std::f64::consts::PI) as i16;
    cpu.set_reg(0, result as u32);
}

/// SWI 0x0A: ArcTan2 - arctangent of y/x
/// r0 = x, r1 = y, returns r0 = angle (0x0000-0xFFFF)
fn swi_arctan2(cpu: &mut Arm7Tdmi) {
    let x = cpu.reg(0) as i16 as f64;
    let y = cpu.reg(1) as i16 as f64;
    let angle = y.atan2(x);
    // convert to 0x0000-0xFFFF range
    let result = (angle * 32768.0 / std::f64::consts::PI) as i16 as u16;
    cpu.set_reg(0, result as u32);
}

/// SWI 0x0B: CpuSet - memory copy/fill
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

/// SWI 0x0C: CpuFastSet - fast memory copy/fill (32-bit, 8-word aligned)
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

/// SWI 0x0E: BgAffineSet - calculates BG affine transformation parameters
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

/// SWI 0x0F: ObjAffineSet - calculates OBJ affine transformation parameters
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

/// SWI 0x11: LZ77UnCompWram - LZ77 decompression to WRAM
fn swi_lz77_decomp_wram(cpu: &mut Arm7Tdmi) {
    lz77_decomp(cpu, false);
}

/// SWI 0x12: LZ77UnCompVram - LZ77 decompression to VRAM (16-bit writes)
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

/// SWI 0x14: RLUnCompWram - run-length decompression to WRAM
fn swi_rl_decomp_wram(cpu: &mut Arm7Tdmi) {
    rl_decomp(cpu, false);
}

/// SWI 0x15: RLUnCompVram - run-length decompression to VRAM
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
    fn test_swi_unknown() {
        let mut cpu = make_cpu();
        handle_swi(&mut cpu, 0xFF);
    }
}
