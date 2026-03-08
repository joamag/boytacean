//! GBA diagnostic utilities for debugging rendering and emulation issues.
//!
//! Provides tools to inspect the internal state of the GBA emulator during
//! execution, including CPU registers, PPU configuration, timer state, IRQ
//! status, and BG tilemap contents. The main entry point [`run_diagnostics`]
//! runs a ROM for a given number of frames, printing detailed state at key
//! frames and saving frame buffer snapshots as PPM images to `/tmp/`.

use std::{fs::File, io::Write};

use crate::{gba::GameBoyAdvance, pad::PadKey};

/// Runs diagnostics on a GBA instance after executing some frames.
/// Prints CPU state, PPU state, memory contents, and frame buffer status.
pub fn run_diagnostics(gba: &mut GameBoyAdvance, num_frames: u32) {
    println!("=== GBA Diagnostics ===");

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

        gba.next_frame();

        if frame < 15 || frame == num_frames - 1 {
            print_state(gba, frame, 0);
            let handler = gba.cpu.bus.read32(0x03FF_FFFC);
            let intrcheck = gba.cpu.bus.read16(0x0300_7FF8);
            let iwf = gba.cpu.bus.intr_wait_flags;
            println!(
                "         | handler={handler:#010x} IntrCheck={intrcheck:#06x} iwf={iwf:#06x}"
            );
            // dump game IRQ handler code and code around PC
            if frame == 0 {
                print!("         | IRQ handler code:");
                for i in 0..64u32 {
                    let word = gba.cpu.bus.read32(handler.wrapping_add(i * 4));
                    if i % 4 == 0 {
                        print!("\n         |   {:#010x}:", handler.wrapping_add(i * 4));
                    }
                    print!(" {word:08x}");
                }
                println!();
            }
            if frame == 2 {
                let pc = gba.cpu.reg(15);
                print!("         | code around PC={pc:#010x}:");
                for i in 0..8u32 {
                    let addr = pc.wrapping_sub(8).wrapping_add(i * 4);
                    let word = gba.cpu.bus.read32(addr);
                    if i % 4 == 0 {
                        print!("\n         |   {addr:#010x}:");
                    }
                    print!(" {word:08x}");
                }
                println!();
            }
            save_frame_buffer_ppm(gba, &format!("/tmp/gba_frame_{frame}.ppm"));
        }

        // Analyze BG rendering at frame 2000
        if frame == 2000 {
            analyze_bg_rendering(gba);
        }
    }
}

/// Runs audio-focused diagnostics, tracing the full DirectSound pipeline.
///
/// The verbose output includes SOUNDCNT_H/X state, FIFO status, current samples,
/// timer frequencies, and DMA activity. At the end, it summarizes key events
/// like the first SOUNDCNT_H write, first FIFO population, and first non-zero
/// audio output, providing a diagnosis of common audio issues.
pub fn run_audio_diagnostics(gba: &mut GameBoyAdvance, num_frames: u32) {
    println!("=== GBA Audio Diagnostics ===\n");

    let mut first_soundcnt_h_frame: Option<u32> = None;
    let mut first_fifo_nonempty_frame: Option<u32> = None;
    let mut first_nonzero_audio_frame: Option<u32> = None;
    let mut first_ds_output_frame: Option<u32> = None;
    let mut prev_soundcnt_h: u16 = 0;
    let mut prev_soundcnt_x: u16 = 0;
    let mut all_samples: Vec<i16> = Vec::new();

    for frame in 0..num_frames {
        if frame % 30 == 10 {
            gba.key_press(PadKey::A);
        }
        if frame % 30 == 12 {
            gba.key_lift(PadKey::A);
        }

        let fifo_a_before = gba.cpu.bus.apu.direct_sound[0].fifo_len();
        let fifo_b_before = gba.cpu.bus.apu.direct_sound[1].fifo_len();
        let sample_a_before = gba.cpu.bus.apu.direct_sound[0].current_sample();
        let sample_b_before = gba.cpu.bus.apu.direct_sound[1].current_sample();

        gba.cpu.bus.apu.direct_sound[0].reset_debug_counters();
        gba.cpu.bus.apu.direct_sound[1].reset_debug_counters();
        gba.clear_audio_buffer();
        gba.next_frame();

        let soundcnt_h = gba.cpu.bus.apu.soundcnt_h();
        let soundcnt_x = gba.cpu.bus.apu.soundcnt_x();
        let soundcnt_l = gba.cpu.bus.apu.soundcnt_l();
        let fifo_a = gba.cpu.bus.apu.direct_sound[0].fifo_len();
        let fifo_b = gba.cpu.bus.apu.direct_sound[1].fifo_len();
        let sample_a = gba.cpu.bus.apu.direct_sound[0].current_sample();
        let sample_b = gba.cpu.bus.apu.direct_sound[1].current_sample();
        let ds_a_out = gba.cpu.bus.apu.direct_sound[0].output();
        let ds_b_out = gba.cpu.bus.apu.direct_sound[1].output();
        let ds_a_left = gba.cpu.bus.apu.direct_sound[0].output_left();
        let ds_a_right = gba.cpu.bus.apu.direct_sound[0].output_right();
        let ds_b_left = gba.cpu.bus.apu.direct_sound[1].output_left();
        let ds_b_right = gba.cpu.bus.apu.direct_sound[1].output_right();

        let audio_buf = gba.audio_buffer();
        let buf_len = audio_buf.len();
        let has_nonzero = audio_buf.iter().any(|&s| s != 0);
        let max_abs = audio_buf
            .iter()
            .map(|s| s.unsigned_abs() as u32)
            .max()
            .unwrap_or(0);
        all_samples.extend(audio_buf.iter());

        if soundcnt_h != 0 && first_soundcnt_h_frame.is_none() {
            first_soundcnt_h_frame = Some(frame);
        }
        if (fifo_a > 0 || fifo_b > 0) && first_fifo_nonempty_frame.is_none() {
            first_fifo_nonempty_frame = Some(frame);
        }
        if has_nonzero && first_nonzero_audio_frame.is_none() {
            first_nonzero_audio_frame = Some(frame);
        }
        if (ds_a_out != 0 || ds_b_out != 0) && first_ds_output_frame.is_none() {
            first_ds_output_frame = Some(frame);
        }

        let h_changed = soundcnt_h != prev_soundcnt_h;
        let x_changed = soundcnt_x != prev_soundcnt_x;
        // compact DMA source log every frame (to track re-latch behavior)
        if (200..=230).contains(&frame) {
            let ch = &gba.cpu.bus.dma.channels[1];
            println!(
                "  [F{:3}] DMA1 en={} act={} src_reg={:#010x} src={:#010x} off={:+6} t={} | pops={} uf={} wr={} rst={} nz={}/{}",
                frame,
                ch.enabled(),
                ch.active(),
                ch.src_reg(),
                ch.src(),
                ch.src() as i64 - ch.src_reg() as i64,
                ch.timing(),
                gba.cpu.bus.apu.direct_sound[0].debug_pops,
                gba.cpu.bus.apu.direct_sound[0].debug_underflows,
                gba.cpu.bus.apu.direct_sound[0].debug_writes,
                gba.cpu.bus.apu.direct_sound[0].debug_resets,
                buf_len / 2 - audio_buf.iter().filter(|&&s| s == 0).count() / 2,
                buf_len / 2,
            );
        }

        let verbose = frame < 10 || frame % 60 == 0 || h_changed || x_changed;

        if verbose {
            println!("--- Frame {} ---", frame);
            println!(
                "  SOUNDCNT_L={:#06x} SOUNDCNT_H={:#06x} SOUNDCNT_X={:#06x}{}{}",
                soundcnt_l,
                soundcnt_h,
                soundcnt_x,
                if h_changed { " [H CHANGED]" } else { "" },
                if x_changed { " [X CHANGED]" } else { "" },
            );

            let ds_a_vol = if soundcnt_h & (1 << 2) != 0 {
                "100%"
            } else {
                "50%"
            };
            let ds_b_vol = if soundcnt_h & (1 << 3) != 0 {
                "100%"
            } else {
                "50%"
            };
            let ds_a_r = soundcnt_h & (1 << 8) != 0;
            let ds_a_l = soundcnt_h & (1 << 9) != 0;
            let ds_a_t = (soundcnt_h >> 10) & 1;
            let ds_b_r = soundcnt_h & (1 << 12) != 0;
            let ds_b_l = soundcnt_h & (1 << 13) != 0;
            let ds_b_t = (soundcnt_h >> 14) & 1;
            let master = soundcnt_x & 0x80 != 0;
            let legacy_vol = soundcnt_h & 0x03;

            println!(
                "  Master={} LegVol={} | A: vol={} L={} R={} tmr={} | B: vol={} L={} R={} tmr={}",
                if master { "ON" } else { "OFF" },
                legacy_vol,
                ds_a_vol,
                ds_a_l,
                ds_a_r,
                ds_a_t,
                ds_b_vol,
                ds_b_l,
                ds_b_r,
                ds_b_t,
            );

            let ds_a = &gba.cpu.bus.apu.direct_sound[0];
            println!(
                "  FIFO-A: len={} (was {}) sample={} (was {}) out={} L={} R={}",
                fifo_a, fifo_a_before, sample_a, sample_a_before, ds_a_out, ds_a_left, ds_a_right,
            );
            println!(
                "    pops={} underflows={} resets={} writes={}",
                ds_a.debug_pops, ds_a.debug_underflows, ds_a.debug_resets, ds_a.debug_writes,
            );
            let ds_b = &gba.cpu.bus.apu.direct_sound[1];
            println!(
                "  FIFO-B: len={} (was {}) sample={} (was {}) out={} L={} R={}",
                fifo_b, fifo_b_before, sample_b, sample_b_before, ds_b_out, ds_b_left, ds_b_right,
            );
            println!(
                "    pops={} underflows={} resets={} writes={}",
                ds_b.debug_pops, ds_b.debug_underflows, ds_b.debug_resets, ds_b.debug_writes,
            );

            for i in 0..2 {
                let timer = &gba.cpu.bus.timers.timers[i];
                if timer.enabled() {
                    let reload = timer.reload();
                    let period = 0x10000u32 - reload as u32;
                    let prescaler = match timer.control() & 0x03 {
                        0 => 1u32,
                        1 => 64,
                        2 => 256,
                        3 => 1024,
                        _ => 1,
                    };
                    let eff = period * prescaler;
                    let hz = if eff > 0 {
                        16_777_216.0 / eff as f64
                    } else {
                        0.0
                    };
                    println!(
                        "  Timer{}: reload={:#06x} period={} prescaler={} freq={:.1}Hz cascade={}",
                        i,
                        reload,
                        period,
                        prescaler,
                        hz,
                        timer.cascade()
                    );
                }
            }

            for i in 1..=2 {
                let ch = &gba.cpu.bus.dma.channels[i];
                if ch.enabled() {
                    let ts = match ch.timing() {
                        0 => "Imm",
                        1 => "VBl",
                        2 => "HBl",
                        3 => "Spc",
                        _ => "?",
                    };
                    println!(
                        "  DMA{}: src_reg={:#010x} dst_reg={:#010x} src={:#010x} dst={:#010x} cnt={} timing={} w32={} rep={} act={}",
                        i,
                        ch.src_reg(),
                        ch.dst_reg(),
                        ch.src(),
                        ch.dst(),
                        ch.count_reg(),
                        ts,
                        ch.word_size(),
                        ch.repeat(),
                        ch.active(),
                    );
                }
            }

            println!(
                "  AudioBuf: {} samples, max_abs={}, nonzero={}",
                buf_len / 2,
                max_abs,
                has_nonzero
            );

            // diagnose issues
            if soundcnt_x & 0x80 == 0 && soundcnt_h != 0 {
                println!("  !! Master sound OFF");
            }
            if soundcnt_h != 0 {
                if !ds_a_l && !ds_a_r && !ds_b_l && !ds_b_r {
                    println!("  !! No DirectSound L/R enabled");
                }
                if fifo_a == 0 && fifo_b == 0 && frame > 5 {
                    println!("  !! Both FIFOs empty");
                    let dma1_s = gba.cpu.bus.dma.channels[1].enabled()
                        && gba.cpu.bus.dma.channels[1].timing() == 3;
                    let dma2_s = gba.cpu.bus.dma.channels[2].enabled()
                        && gba.cpu.bus.dma.channels[2].timing() == 3;
                    if !dma1_s && !dma2_s {
                        println!("  !! No DMA1/2 with SPECIAL timing");
                    }
                    for j in 1..=2 {
                        let c = &gba.cpu.bus.dma.channels[j];
                        if c.enabled() && c.timing() == 3 {
                            let d = c.dst_reg();
                            if d != 0x0400_00A0 && d != 0x0400_00A4 {
                                println!("  !! DMA{} dst={:#010x} not FIFO addr", j, d);
                            }
                        }
                    }
                }
                if ds_a_out == 0 && ds_b_out == 0 && (fifo_a > 0 || fifo_b > 0) {
                    println!("  !! FIFOs have data but DS output=0");
                }
            }
        }

        prev_soundcnt_h = soundcnt_h;
        prev_soundcnt_x = soundcnt_x;
    }

    println!("\n=== Audio Summary ({} frames) ===", num_frames);
    let fmt = |o: Option<u32>| o.map(|f| format!("frame {f}")).unwrap_or("NEVER".into());
    println!(
        "  First SOUNDCNT_H write:   {}",
        fmt(first_soundcnt_h_frame)
    );
    println!(
        "  First FIFO non-empty:     {}",
        fmt(first_fifo_nonempty_frame)
    );
    println!("  First DS output non-zero: {}", fmt(first_ds_output_frame));
    println!(
        "  First audio buf non-zero: {}",
        fmt(first_nonzero_audio_frame)
    );

    let soundcnt_h = gba.cpu.bus.apu.soundcnt_h();
    let soundcnt_x = gba.cpu.bus.apu.soundcnt_x();
    println!(
        "\n  Final SOUNDCNT_H={:#06x} SOUNDCNT_X={:#06x}",
        soundcnt_h, soundcnt_x
    );
    println!(
        "  Final FIFO-A={} FIFO-B={} sample-A={} sample-B={}",
        gba.cpu.bus.apu.direct_sound[0].fifo_len(),
        gba.cpu.bus.apu.direct_sound[1].fifo_len(),
        gba.cpu.bus.apu.direct_sound[0].current_sample(),
        gba.cpu.bus.apu.direct_sound[1].current_sample(),
    );

    println!();
    if first_ds_output_frame.is_some() {
        println!("  VERDICT: DirectSound pipeline IS producing output.");
        if first_nonzero_audio_frame.is_some() {
            println!("  Audio buffer has non-zero samples.");
        } else {
            println!("  BUT audio buffer always zero — check generate_sample().");
        }
    } else if first_fifo_nonempty_frame.is_some() {
        println!("  VERDICT: FIFO populated but DS output always 0.");
        println!("  Check: timer_overflow() not popping, or enable L/R off.");
    } else if first_soundcnt_h_frame.is_some() {
        println!("  VERDICT: SOUNDCNT_H set but FIFO never populated.");
        println!("  Check: DMA not triggering, or wrong source/dest.");
    } else {
        println!("  VERDICT: Game never wrote SOUNDCNT_H — no DirectSound,");
        println!("  or hasn't reached audio init (try more frames).");
    }

    // write WAV file
    let wav_path = "/tmp/gba_audio_diag.wav";
    if let Ok(mut f) = File::create(wav_path) {
        let sample_rate: u32 = 32768;
        let channels: u16 = 2;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
        let block_align = channels * bits_per_sample / 8;
        let data_size = all_samples.len() as u32 * 2;
        let file_size = 36 + data_size;

        // RIFF header
        let _ = f.write_all(b"RIFF");
        let _ = f.write_all(&file_size.to_le_bytes());
        let _ = f.write_all(b"WAVE");
        // fmt chunk
        let _ = f.write_all(b"fmt ");
        let _ = f.write_all(&16u32.to_le_bytes());
        let _ = f.write_all(&1u16.to_le_bytes()); // PCM
        let _ = f.write_all(&channels.to_le_bytes());
        let _ = f.write_all(&sample_rate.to_le_bytes());
        let _ = f.write_all(&byte_rate.to_le_bytes());
        let _ = f.write_all(&block_align.to_le_bytes());
        let _ = f.write_all(&bits_per_sample.to_le_bytes());
        // data chunk
        let _ = f.write_all(b"data");
        let _ = f.write_all(&data_size.to_le_bytes());
        for &sample in &all_samples {
            let _ = f.write_all(&sample.to_le_bytes());
        }
        println!(
            "\n  WAV written to {wav_path} ({} samples, {:.1}s)",
            all_samples.len() / 2,
            all_samples.len() as f64 / 2.0 / sample_rate as f64
        );
    }
}

/// Prints a snapshot of the GBA's CPU, display, IRQ, timer state, etc.
///
/// Outputs (amongst other things) the frame number, PC, processor mode,
/// THUMB/ARM state, CPSR, DISPCNT fields, interrupt registers, active
/// timers, and whether IRQs are masked via the CPSR I bit.
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
        let timer = &cpu.bus.timers.timers[i];
        if timer.enabled() {
            println!(
                "         | TM{}: cnt={:#06x} reload={:#06x} ctrl={:#06x} cascade={} irq={}",
                i,
                timer.counter(),
                timer.reload(),
                timer.control(),
                timer.cascade(),
                timer.irq_enable()
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
    println!("\n=== BG Rendering Analysis (mode {mode}) ===");

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
                print!("{entry:#06x} ");
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

/// Saves the current frame buffer as a PPM (P6) image file.
///
/// Writes a 240x160 raw RGB image to the given path, useful for
/// visual inspection and automated image comparison tests.
fn save_frame_buffer_ppm(gba: &GameBoyAdvance, path: &str) {
    let frame_buffer = gba.frame_buffer();
    let mut file = File::create(path).expect("Failed to create PPM file");
    write!(file, "P6\n240 160\n255\n").expect("Failed to write PPM header");
    file.write_all(frame_buffer)
        .expect("Failed to write PPM data");
    println!("Frame buffer saved to {path}");
}
