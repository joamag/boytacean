//! GBA diagnostic utilities for debugging rendering issues.

use super::GameBoyAdvance;

/// Runs diagnostics on a GBA instance after executing some frames.
/// Prints CPU state, PPU state, memory contents, and frame buffer status.
pub fn run_diagnostics(gba: &mut GameBoyAdvance, num_frames: u32) {
    println!("=== GBA Diagnostics ===");

    use crate::pad::PadKey;

    for frame in 0..num_frames {
        // Simulate button presses to advance through menus/intro
        if frame % 30 == 10 {
            gba.key_press(PadKey::A);
        }
        if frame % 30 == 12 {
            gba.key_lift(PadKey::A);
        }
        if frame == 1800 || frame == 1850 {
            gba.key_press(PadKey::Start);
        }
        if frame == 1802 || frame == 1852 {
            gba.key_lift(PadKey::Start);
        }

        // run a full frame
        gba.next_frame();

        if frame % 500 == 0 || frame == num_frames - 1 {
            print_state(gba, frame, 0);
            save_frame_buffer_ppm(gba, &format!("/tmp/gba_frame_{}.ppm", frame));
        }

        // Analyze BG rendering at frame 2000
        if frame == 2000 {
            analyze_bg_rendering(gba);
        }
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

    // print timer state
    for i in 0..4 {
        let t = &cpu.bus.timers.timers[i];
        if t.enabled() {
            println!(
                "         | TM{}: cnt={:#06x} reload={:#06x} ctrl={:#06x} cascade={} irq={}",
                i,
                t.counter(),
                t.reload(),
                t.control(),
                t.cascade(),
                t.irq_enable()
            );
        }
    }
    // show CPSR I bit (IRQ disable)
    let irq_disabled = cpu.cpsr() & 0x80 != 0;
    if irq_disabled {
        println!("         | ** CPSR I=1 (IRQs disabled) **");
    }
}

/// Analyzes BG rendering to diagnose the vertical strip glitch.
fn analyze_bg_rendering(gba: &GameBoyAdvance) {
    let dispcnt = gba.cpu.bus.ppu.dispcnt();
    let mode = dispcnt & 0x07;
    println!("\n=== BG Rendering Analysis (mode {}) ===", mode);

    for bg in 0..4 {
        if dispcnt & (1 << (8 + bg)) == 0 {
            continue;
        }
        let cnt = gba.cpu.bus.ppu.bgcnt(bg);
        let char_base = ((cnt >> 2) & 0x03) as usize * 0x4000;
        let screen_base = ((cnt >> 8) & 0x1F) as usize * 0x800;
        let is_8bpp = cnt & (1 << 7) != 0;
        let screen_size = (cnt >> 14) & 0x03;
        let priority = cnt & 0x03;
        let hofs = gba.cpu.bus.ppu.bg_hofs(bg);
        let vofs = gba.cpu.bus.ppu.bg_vofs(bg);

        println!(
            "\nBG{}: CNT={:#06x} char={:#06x} screen={:#06x} {}bpp size={} pri={} hofs={} vofs={}",
            bg,
            cnt,
            char_base,
            screen_base,
            if is_8bpp { 8 } else { 4 },
            screen_size,
            priority,
            hofs,
            vofs
        );

        // Dump first 32 tile map entries (raw hex)
        println!("  First 32 map entries:");
        for i in 0..32 {
            let addr = screen_base + i * 2;
            if addr + 1 < gba.cpu.bus.vram.len() {
                let entry =
                    gba.cpu.bus.vram[addr] as u16 | ((gba.cpu.bus.vram[addr + 1] as u16) << 8);
                print!("{:#06x} ", entry);
                if (i + 1) % 8 == 0 {
                    println!();
                }
            }
        }
    }

    // Frame buffer strip analysis
    println!("\n=== Frame Buffer Vertical Strip Analysis (scanline 80) ===");
    let fb = gba.frame_buffer();
    let line = 80;
    let offset = line * 240 * 3;
    print!("  8px cols: ");
    for col in 0..30 {
        let mut has_content = false;
        for px in 0..8 {
            let x = col * 8 + px;
            let r = fb[offset + x * 3];
            let g = fb[offset + x * 3 + 1];
            let b = fb[offset + x * 3 + 2];
            if r != 0 || g != 0 || b != 0 {
                has_content = true;
                break;
            }
        }
        print!("{}", if has_content { "#" } else { "." });
    }
    println!();
}

fn save_frame_buffer_ppm(gba: &GameBoyAdvance, path: &str) {
    use std::{fs::File, io::Write};

    let fb = gba.frame_buffer();
    let mut f = File::create(path).expect("Failed to create PPM file");
    write!(f, "P6\n240 160\n255\n").expect("Failed to write PPM header");
    f.write_all(fb).expect("Failed to write PPM data");
    println!("Frame buffer saved to {}", path);
}
