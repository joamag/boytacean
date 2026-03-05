//! GBA diagnostic utilities for debugging rendering issues.

use super::GameBoyAdvance;

/// Runs diagnostics on a GBA instance after executing some frames.
/// Prints CPU state, PPU state, memory contents, and frame buffer status.
pub fn run_diagnostics(gba: &mut GameBoyAdvance, num_frames: u32) {
    println!("=== GBA Diagnostics ===");

    // first, trace the very first instructions from boot
    println!("\n=== Early Boot Trace (first 200 instructions) ===");
    for i in 0..200 {
        let pc_before = gba.cpu.pc();
        let thumb = gba.cpu.in_thumb_mode();
        let cpsr_before = gba.cpu.cpsr();
        let mode_before = cpsr_before & 0x1F;
        let r0_before = gba.cpu.reg(0);

        // Read what the pipeline will actually execute
        // In ARM: pipeline[0] is at PC-8, the instruction about to execute
        // The PC value points 2 instructions ahead due to pipelining
        let exec_addr = if thumb {
            pc_before.wrapping_sub(4)
        } else {
            pc_before.wrapping_sub(8)
        };
        let exec_instr = if thumb {
            gba.cpu.bus.read16(exec_addr) as u32
        } else {
            gba.cpu.bus.read32(exec_addr)
        };

        let _cycles = gba.clock();

        let cpsr_after = gba.cpu.cpsr();
        let mode_after = cpsr_after & 0x1F;

        // Print every step for first 50, then every 10th, or on mode change
        if i < 50 || i % 10 == 0 || mode_before != mode_after {
            let r0 = gba.cpu.reg(0);
            let r1 = gba.cpu.reg(1);
            let r13 = gba.cpu.reg(13);
            let r14 = gba.cpu.reg(14);
            let mode_change = if mode_before != mode_after {
                format!(" MODE CHANGE {:#04x}->{:#04x}", mode_before, mode_after)
            } else {
                String::new()
            };
            println!(
                "  [{:3}] {:#010x}:{:#010x} {} cpsr={:#010x}->{:#010x} r0={:#010x}->{:#010x} r1={:#010x} sp={:#010x} lr={:#010x}{}",
                i, exec_addr, exec_instr, if thumb { "T" } else { "A" },
                cpsr_before, cpsr_after,
                r0_before, r0, r1, r13, r14, mode_change
            );
        }
    }

    println!("\nRunning frames (will stop if CPU leaves valid memory)...");
    let mut trace_buffer: Vec<String> = Vec::with_capacity(20);

    for frame in 0..num_frames {
        // run individual clocks to catch the exact moment CPU goes off
        let last_frame = gba.cpu.bus.ppu.frame();
        let mut frame_cycles = 0u64;
        let mut bad_pc = false;
        while gba.cpu.bus.ppu.frame() == last_frame {
            let pc = gba.cpu.pc();
            let in_rom = (0x0800_0000..0x0A00_0000).contains(&pc);
            let in_iwram = (0x0300_0000..0x0300_8000).contains(&pc);
            let in_ewram = (0x0200_0000..0x0204_0000).contains(&pc);
            let in_bios = pc < 0x4000;

            if !in_rom && !in_iwram && !in_ewram && !in_bios && !bad_pc {
                bad_pc = true;
                println!("\n!!! CPU LEFT VALID MEMORY at frame {} !!!", frame);
                println!("  PC = {:#010x}", pc);
                println!("  CPSR = {:#010x}", gba.cpu.cpsr());
                println!("  Mode = {:#04x}", gba.cpu.cpsr() & 0x1F);
                println!("  R0={:#010x} R1={:#010x} R2={:#010x} R3={:#010x}",
                    gba.cpu.reg(0), gba.cpu.reg(1), gba.cpu.reg(2), gba.cpu.reg(3));
                println!("  R4={:#010x} R5={:#010x} R6={:#010x} R7={:#010x}",
                    gba.cpu.reg(4), gba.cpu.reg(5), gba.cpu.reg(6), gba.cpu.reg(7));
                println!("  R8={:#010x} R9={:#010x} R10={:#010x} R11={:#010x}",
                    gba.cpu.reg(8), gba.cpu.reg(9), gba.cpu.reg(10), gba.cpu.reg(11));
                println!("  R12={:#010x} SP={:#010x} LR={:#010x}",
                    gba.cpu.reg(12), gba.cpu.reg(13), gba.cpu.reg(14));
                println!("\n  Last 20 instructions from trace buffer:");
                for (j, entry) in trace_buffer.iter().enumerate() {
                    println!("    [{:2}] {}", j, entry);
                }
                break;
            }

            // record trace for debugging
            let thumb = gba.cpu.in_thumb_mode();
            let exec_addr = if thumb { pc.wrapping_sub(4) } else { pc.wrapping_sub(8) };
            let cpsr_before = gba.cpu.cpsr();
            let r12_before = gba.cpu.reg(12);
            let cycles = gba.clock();
            let cpsr_after = gba.cpu.cpsr();
            let r12_after = gba.cpu.reg(12);
            let pc_after = gba.cpu.pc();
            let trace_entry = format!(
                "{:#010x}(pc={:#010x}): cpsr={:#010x}->{:#010x} {} c={} r12={:#010x}->{:#010x} pc_after={:#010x}",
                exec_addr, pc, cpsr_before, cpsr_after,
                if thumb { "T" } else { "A" },
                cycles, r12_before, r12_after, pc_after
            );
            if trace_buffer.len() >= 20 {
                trace_buffer.remove(0);
            }
            trace_buffer.push(trace_entry);
            frame_cycles += cycles as u64;
        }

        if bad_pc {
            break;
        }

        if frame < 5 || frame % 10 == 0 {
            print_state(gba, frame, frame_cycles);
        }
    }

    println!("\n=== Final State ===");
    print_state(gba, num_frames - 1, 0);
    print_frame_buffer_stats(gba);
    print_memory_regions(gba);

    // dump instructions around where the CPU is stuck
    let pc = gba.cpu.pc();
    let start = if pc >= 0x20 { pc - 0x20 } else { pc };
    print_rom_instructions(gba, start & !3, 20);

    // also dump the ROM entry point area
    print_rom_instructions(gba, 0x0800_0000, 16);

    // dump the actual init code area
    print_rom_instructions(gba, 0x0800_03B8, 20);

    // dump first instruction trace
    println!("\n=== Instruction Trace (next 20 steps) ===");
    for _ in 0..20 {
        let pc_before = gba.cpu.pc();
        let thumb = gba.cpu.in_thumb_mode();
        let instr = if thumb {
            gba.cpu.bus.read16(pc_before.wrapping_sub(4)) as u32
        } else {
            gba.cpu.bus.read32(pc_before.wrapping_sub(8))
        };
        let mode = gba.cpu.cpsr() & 0x1F;
        let cycles = gba.clock();
        println!(
            "  PC={:#010x} instr={:#010x} {} mode={:#04x} cycles={}",
            pc_before,
            instr,
            if thumb { "THUMB" } else { "ARM  " },
            mode,
            cycles
        );
    }
}

fn print_state(gba: &GameBoyAdvance, frame: u32, cycles: u64) {
    let cpu = &gba.cpu;
    let mode = cpu.cpsr() & 0x1F;
    let mode_str = match mode {
        0x10 => "USR",
        0x11 => "FIQ",
        0x12 => "IRQ",
        0x13 => "SVC",
        0x17 => "ABT",
        0x1B => "UND",
        0x1F => "SYS",
        _ => "???",
    };
    let thumb = cpu.cpsr() & 0x20 != 0;

    println!(
        "Frame {:3} | PC={:#010x} | Mode={} | {} | CPSR={:#010x} | cycles={}",
        frame,
        cpu.pc(),
        mode_str,
        if thumb { "THUMB" } else { "ARM  " },
        cpu.cpsr(),
        cycles
    );

    // print DISPCNT and DISPSTAT
    let dispcnt = cpu.bus.ppu.dispcnt();
    let video_mode = dispcnt & 0x07;
    let bg_enables = (dispcnt >> 8) & 0x0F;
    let obj_enable = (dispcnt >> 12) & 1;
    let forced_blank = (dispcnt >> 7) & 1;

    println!(
        "         | DISPCNT={:#06x} mode={} BGs={:04b} OBJ={} forced_blank={} | DISPSTAT={:#06x} | VCOUNT={}",
        dispcnt,
        video_mode,
        bg_enables,
        obj_enable,
        forced_blank,
        cpu.bus.ppu.dispstat(),
        cpu.bus.ppu.vcount()
    );

    // IRQ state
    let ime = cpu.bus.irq.ime();
    let ie = cpu.bus.irq.ie();
    let if_ = cpu.bus.irq.if_();
    println!(
        "         | IME={} IE={:#06x} IF={:#06x} | halted={}",
        ime,
        ie,
        if_,
        cpu.halted()
    );

    // IRQ handler address
    let handler = cpu.bus.read32(0x03FF_FFFC);
    println!("         | IRQ handler @ 0x03FFFFFC = {:#010x}", handler);
}

fn print_frame_buffer_stats(gba: &GameBoyAdvance) {
    let fb = gba.frame_buffer();
    let total_pixels = 240 * 160;
    let mut black_pixels = 0u32;
    let mut white_pixels = 0u32;
    let mut non_black_pixels = 0u32;

    for i in 0..total_pixels {
        let r = fb[i * 3];
        let g = fb[i * 3 + 1];
        let b = fb[i * 3 + 2];
        if r == 0 && g == 0 && b == 0 {
            black_pixels += 1;
        } else if r == 255 && g == 255 && b == 255 {
            white_pixels += 1;
        } else {
            non_black_pixels += 1;
        }
    }

    println!("\n=== Frame Buffer ===");
    println!("Total pixels: {}", total_pixels);
    println!("Black pixels: {} ({:.1}%)", black_pixels, black_pixels as f64 / total_pixels as f64 * 100.0);
    println!("White pixels: {} ({:.1}%)", white_pixels, white_pixels as f64 / total_pixels as f64 * 100.0);
    println!("Other pixels: {} ({:.1}%)", non_black_pixels, non_black_pixels as f64 / total_pixels as f64 * 100.0);

    // sample some non-black pixels
    if non_black_pixels > 0 || white_pixels > 0 {
        println!("\nSample non-black pixels:");
        let mut count = 0;
        for i in 0..total_pixels {
            let r = fb[i * 3];
            let g = fb[i * 3 + 1];
            let b = fb[i * 3 + 2];
            if r != 0 || g != 0 || b != 0 {
                let x = i % 240;
                let y = i / 240;
                println!("  ({}, {}) = RGB({}, {}, {})", x, y, r, g, b);
                count += 1;
                if count >= 10 {
                    break;
                }
            }
        }
    }
}

fn print_rom_instructions(gba: &GameBoyAdvance, start: u32, count: usize) {
    println!("\n=== ROM Instructions @ {:#010x} ===", start);
    for i in 0..count {
        let addr = start + (i as u32) * 4;
        let instr = gba.cpu.bus.read32(addr);
        println!("  {:#010x}: {:#010x}", addr, instr);
    }
}

fn print_memory_regions(gba: &GameBoyAdvance) {
    println!("\n=== Memory Regions ===");

    // Check palette RAM (first 32 bytes)
    print!("Palette[0..32]: ");
    let mut palette_nonzero = 0;
    for i in 0..512 {
        if gba.cpu.bus.palette[i] != 0 {
            palette_nonzero += 1;
        }
    }
    println!("{} non-zero bytes out of 512", palette_nonzero);

    // Check VRAM (first 32 bytes)
    let mut vram_nonzero = 0;
    for byte in gba.cpu.bus.vram.iter() {
        if *byte != 0 {
            vram_nonzero += 1;
        }
    }
    println!("VRAM: {} non-zero bytes out of {}", vram_nonzero, gba.cpu.bus.vram.len());

    // Check OAM
    let mut oam_nonzero = 0;
    for byte in gba.cpu.bus.oam.iter() {
        if *byte != 0 {
            oam_nonzero += 1;
        }
    }
    println!("OAM: {} non-zero bytes out of {}", oam_nonzero, gba.cpu.bus.oam.len());

    // Check IWRAM
    let mut iwram_nonzero = 0;
    for byte in gba.cpu.bus.iwram.iter() {
        if *byte != 0 {
            iwram_nonzero += 1;
        }
    }
    println!("IWRAM: {} non-zero bytes out of {}", iwram_nonzero, gba.cpu.bus.iwram.len());

    // Check EWRAM
    let mut ewram_nonzero = 0;
    for byte in gba.cpu.bus.ewram.iter() {
        if *byte != 0 {
            ewram_nonzero += 1;
        }
    }
    println!("EWRAM: {} non-zero bytes out of {}", ewram_nonzero, gba.cpu.bus.ewram.len());

    // Print first few BG control registers
    for i in 0..4 {
        println!("BG{}CNT = {:#06x}", i, gba.cpu.bus.ppu.bgcnt(i));
    }

    // Dump IRQ handler code (if set in IWRAM)
    let handler = gba.cpu.bus.read32(0x03FF_FFFC);
    if handler != 0 && (0x0300_0000..0x0300_8000).contains(&handler) {
        println!("\n=== IRQ Handler Code @ {:#010x} ===", handler);
        for i in 0..48 {
            let addr = handler + i * 4;
            let word = gba.cpu.bus.read32(addr);
            println!("  {:#010x}: {:#010x}", addr, word);
        }
    }

    // Check DISPSTAT bits
    let dispstat = gba.cpu.bus.ppu.dispstat();
    println!("\nDISPSTAT breakdown:");
    println!("  VBlank IRQ enable: {}", dispstat & (1 << 3) != 0);
    println!("  HBlank IRQ enable: {}", dispstat & (1 << 4) != 0);
    println!("  VCount IRQ enable: {}", dispstat & (1 << 5) != 0);
    println!("  VCount target: {}", (dispstat >> 8) & 0xFF);
}
