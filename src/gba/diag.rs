//! GBA diagnostic utilities for debugging rendering and emulation issues.
//!
//! Provides tools to inspect the internal state of the GBA emulator during
//! execution, including CPU registers, PPU configuration, timer state and
//! IRQ status. The main entry point [`run_diagnostics`] runs a ROM for a
//! given number of frames, printing detailed state at key frames and saving
//! frame buffer snapshots as PPM images into the given directory.

use std::{fs::File, io::Write, path::Path};

use boytacean_common::error::Error;

use crate::gba::{consts::CPU_FREQ, GameBoyAdvance};

/// Runs diagnostics on a GBA instance, executing the given number of frames.
///
/// Prints the CPU, PPU, IRQ and timer state of the first 15 frames and of
/// the last one, saving each of their frame buffers into the directory as
/// a `gba_frame_<n>.ppm` image.
pub fn run_diagnostics(
    gba: &mut GameBoyAdvance,
    num_frames: u32,
    directory: &Path,
) -> Result<(), Error> {
    println!("=== GBA Diagnostics ===");

    for frame in 0..num_frames {
        let cycles = gba.next_frame();

        if frame < 15 || frame == num_frames - 1 {
            println!("{}", description_state(gba, frame, cycles));
            let handler = gba.cpu.bus.read32(0x03FF_FFFC);
            let intrcheck = gba.cpu.bus.read16(0x0300_7FF8);
            let iwf = gba.cpu.bus.intr_wait_flags;
            println!(
                "         | handler={handler:#010x} IntrCheck={intrcheck:#06x} iwf={iwf:#06x}"
            );
            let path = directory.join(format!("gba_frame_{frame}.ppm"));
            save_frame_buffer_ppm(gba, &path)?;
        }
    }

    Ok(())
}

/// Runs audio-focused diagnostics, tracing the full DirectSound pipeline.
///
/// The verbose output includes SOUNDCNT_H/X state, FIFO status, current samples,
/// timer frequencies, and DMA activity. At the end, it summarizes key events
/// like the first SOUNDCNT_H write, first FIFO population, and first non-zero
/// audio output, providing a diagnosis of common audio issues, and saves the
/// audio output into the directory as a `gba_audio_diag.wav` file.
pub fn run_audio_diagnostics(
    gba: &mut GameBoyAdvance,
    num_frames: u32,
    directory: &Path,
) -> Result<(), Error> {
    println!("=== GBA Audio Diagnostics ===\n");

    let mut first_soundcnt_h_frame: Option<u32> = None;
    let mut first_fifo_nonempty_frame: Option<u32> = None;
    let mut first_nonzero_audio_frame: Option<u32> = None;
    let mut first_ds_output_frame: Option<u32> = None;
    let mut prev_soundcnt_h: u16 = 0;
    let mut prev_soundcnt_x: u16 = 0;
    let mut all_samples: Vec<i16> = Vec::new();

    for frame in 0..num_frames {
        let fifo_a_before = gba.cpu.bus.apu.direct_sound[0].fifo_len();
        let fifo_b_before = gba.cpu.bus.apu.direct_sound[1].fifo_len();
        let sample_a_before = gba.cpu.bus.apu.direct_sound[0].current_sample();
        let sample_b_before = gba.cpu.bus.apu.direct_sound[1].current_sample();

        gba.cpu.bus.apu.direct_sound[0].reset_debug_counters();
        gba.cpu.bus.apu.direct_sound[1].reset_debug_counters();
        gba.clear_audio_buffer();
        gba.next_frame();
        gba.cpu.bus.apu.flush();

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
        let verbose = frame < 10 || frame % 60 == 0 || h_changed || x_changed;

        if verbose {
            println!("--- Frame {frame} ---");
            println!(
                "  SOUNDCNT_L={:#06x} SOUNDCNT_H={:#06x} SOUNDCNT_X={:#06x}{}{}",
                soundcnt_l,
                soundcnt_h,
                soundcnt_x,
                if h_changed { " [H CHANGED]" } else { "" },
                if x_changed { " [X CHANGED]" } else { "" },
            );

            let ds_a = &gba.cpu.bus.apu.direct_sound[0];
            println!(
                "  FIFO-A: len={fifo_a} (was {fifo_a_before}) sample={sample_a} (was {sample_a_before}) out={ds_a_out} L={ds_a_left} R={ds_a_right}",
            );
            println!(
                "    pops={} underflows={} resets={} writes={}",
                ds_a.debug_pops, ds_a.debug_underflows, ds_a.debug_resets, ds_a.debug_writes,
            );
            let ds_b = &gba.cpu.bus.apu.direct_sound[1];
            println!(
                "  FIFO-B: len={fifo_b} (was {fifo_b_before}) sample={sample_b} (was {sample_b_before}) out={ds_b_out} L={ds_b_left} R={ds_b_right}",
            );
            println!(
                "    pops={} underflows={} resets={} writes={}",
                ds_b.debug_pops, ds_b.debug_underflows, ds_b.debug_resets, ds_b.debug_writes,
            );

            println!("{}", description_audio(gba));

            println!(
                "  AudioBuf: {} samples, max_abs={}, nonzero={}",
                buf_len / 2,
                max_abs,
                has_nonzero
            );

            for issue in audio_issues(gba, frame) {
                println!("  !! {issue}");
            }
        }

        prev_soundcnt_h = soundcnt_h;
        prev_soundcnt_x = soundcnt_x;
    }

    println!("\n=== Audio Summary ({num_frames} frames) ===");
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
    println!("\n  Final SOUNDCNT_H={soundcnt_h:#06x} SOUNDCNT_X={soundcnt_x:#06x}");
    println!(
        "  Final FIFO-A={} FIFO-B={} sample-A={} sample-B={}",
        gba.cpu.bus.apu.direct_sound[0].fifo_len(),
        gba.cpu.bus.apu.direct_sound[1].fifo_len(),
        gba.cpu.bus.apu.direct_sound[0].current_sample(),
        gba.cpu.bus.apu.direct_sound[1].current_sample(),
    );

    println!();
    println!(
        "{}",
        audio_verdict(
            first_soundcnt_h_frame,
            first_fifo_nonempty_frame,
            first_ds_output_frame,
            first_nonzero_audio_frame
        )
    );

    // write WAV file
    save_audio_wav(&all_samples, &directory.join("gba_audio_diag.wav"))
}

/// Builds a description of the DirectSound mixing, of the timers
/// that clock its FIFOs and of the DMA channels that feed them.
fn description_audio(gba: &GameBoyAdvance) -> String {
    let soundcnt_h = gba.cpu.bus.apu.soundcnt_h();
    let soundcnt_x = gba.cpu.bus.apu.soundcnt_x();
    let volume = |full: bool| if full { "100%" } else { "50%" };

    let mut lines = vec![format!(
        "  Master={} LegVol={} | A: vol={} L={} R={} tmr={} | B: vol={} L={} R={} tmr={}",
        if soundcnt_x & 0x80 != 0 { "ON" } else { "OFF" },
        soundcnt_h & 0x03,
        volume(soundcnt_h & (1 << 2) != 0),
        soundcnt_h & (1 << 9) != 0,
        soundcnt_h & (1 << 8) != 0,
        (soundcnt_h >> 10) & 1,
        volume(soundcnt_h & (1 << 3) != 0),
        soundcnt_h & (1 << 13) != 0,
        soundcnt_h & (1 << 12) != 0,
        (soundcnt_h >> 14) & 1,
    )];

    for (i, timer) in gba.cpu.bus.timers.timers.iter().enumerate().take(2) {
        if timer.enabled() {
            let reload = timer.reload();
            let period = 0x10000u32 - reload as u32;
            let prescaler = match timer.control() & 0x03 {
                0 => 1u32,
                1 => 64,
                2 => 256,
                _ => 1024,
            };
            let hz = CPU_FREQ as f64 / (period * prescaler) as f64;
            lines.push(format!(
                "  Timer{}: reload={:#06x} period={} prescaler={} freq={:.1}Hz cascade={}",
                i,
                reload,
                period,
                prescaler,
                hz,
                timer.cascade()
            ));
        }
    }

    for (i, ch) in gba.cpu.bus.dma.channels.iter().enumerate().take(3).skip(1) {
        if ch.enabled() {
            let ts = match ch.timing() {
                0 => "Imm",
                1 => "VBl",
                2 => "HBl",
                _ => "Spc",
            };
            lines.push(format!(
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
            ));
        }
    }

    lines.join("\n")
}

/// Diagnoses the common issues of the DirectSound pipeline, as found
/// in its state after the given frame.
fn audio_issues(gba: &GameBoyAdvance, frame: u32) -> Vec<String> {
    let apu = &gba.cpu.bus.apu;
    let channels = &gba.cpu.bus.dma.channels;
    let soundcnt_h = apu.soundcnt_h();
    let mut issues = Vec::new();

    if soundcnt_h == 0 {
        return issues;
    }
    if apu.soundcnt_x() & 0x80 == 0 {
        issues.push(String::from("Master sound OFF"));
    }
    if soundcnt_h & ((1 << 8) | (1 << 9) | (1 << 12) | (1 << 13)) == 0 {
        issues.push(String::from("No DirectSound L/R enabled"));
    }

    let fifo_a = apu.direct_sound[0].fifo_len();
    let fifo_b = apu.direct_sound[1].fifo_len();
    if fifo_a == 0 && fifo_b == 0 && frame > 5 {
        issues.push(String::from("Both FIFOs empty"));
        let dma1_s = channels[1].enabled() && channels[1].timing() == 3;
        let dma2_s = channels[2].enabled() && channels[2].timing() == 3;
        if !dma1_s && !dma2_s {
            issues.push(String::from("No DMA1/2 with SPECIAL timing"));
        }
        for (j, c) in channels.iter().enumerate().take(3).skip(1) {
            if c.enabled() && c.timing() == 3 {
                let d = c.dst_reg();
                if d != 0x0400_00A0 && d != 0x0400_00A4 {
                    issues.push(format!("DMA{j} dst={d:#010x} not FIFO addr"));
                }
            }
        }
    }
    if apu.direct_sound[0].output() == 0
        && apu.direct_sound[1].output() == 0
        && (fifo_a > 0 || fifo_b > 0)
    {
        issues.push(String::from("FIFOs have data but DS output=0"));
    }

    issues
}

/// Diagnoses the DirectSound pipeline from the first frames of its key
/// events (`None` for the events that never happened), pointing to the
/// stage that fails to deliver audio.
fn audio_verdict(
    first_soundcnt_h_frame: Option<u32>,
    first_fifo_nonempty_frame: Option<u32>,
    first_ds_output_frame: Option<u32>,
    first_nonzero_audio_frame: Option<u32>,
) -> &'static str {
    if first_ds_output_frame.is_some() {
        if first_nonzero_audio_frame.is_some() {
            "  VERDICT: DirectSound pipeline IS producing output.\n  Audio buffer has non-zero samples."
        } else {
            "  VERDICT: DirectSound pipeline IS producing output.\n  BUT audio buffer always zero, check generate_sample()."
        }
    } else if first_fifo_nonempty_frame.is_some() {
        "  VERDICT: FIFO populated but DS output always 0.\n  Check: timer_overflow() not popping, or enable L/R off."
    } else if first_soundcnt_h_frame.is_some() {
        "  VERDICT: SOUNDCNT_H set but FIFO never populated.\n  Check: DMA not triggering, or wrong source/dest."
    } else {
        "  VERDICT: Game never wrote SOUNDCNT_H, no DirectSound,\n  or hasn't reached audio init (try more frames)."
    }
}

/// Saves signed stereo samples as a PCM WAV file.
fn save_audio_wav(samples: &[i16], path: &Path) -> Result<(), Error> {
    let mut file = File::create(path)?;
    let sample_rate: u32 = 32768;
    let channels: u16 = 2;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_size = samples.len() as u32 * 2;
    let file_size = 36 + data_size;

    // riff header
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;
    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // pcm
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;
    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }
    println!(
        "\n  WAV written to {} ({} samples, {:.1}s)",
        path.display(),
        samples.len() / 2,
        samples.len() as f64 / 2.0 / sample_rate as f64
    );
    Ok(())
}

/// Builds a description of the GBA's CPU, display, IRQ, timer state, etc.
///
/// Describes (amongst other things) the frame number, PC, processor mode,
/// THUMB/ARM state, CPSR, DISPCNT fields, enabled backgrounds, interrupt
/// registers, active timers, and whether IRQs are masked via the CPSR I bit.
fn description_state(gba: &GameBoyAdvance, frame: u32, cycles: u64) -> String {
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

    let mut lines = vec![format!(
        "Frame {:3} | PC={:#010x} | Mode={} | {} | CPSR={:#010x} | cycles={}",
        frame,
        cpu.pc(),
        mode_str,
        if thumb { "THUMB" } else { "ARM  " },
        cpu.cpsr(),
        cycles
    )];

    let dispcnt = cpu.bus.ppu.dispcnt();
    let video_mode = dispcnt & 0x07;
    let bg_enables = (dispcnt >> 8) & 0x0F;
    let obj_enable = (dispcnt >> 12) & 1;
    let forced_blank = (dispcnt >> 7) & 1;

    lines.push(format!(
        "         | DISPCNT={:#06x} mode={} BGs={:04b} OBJ={} forced_blank={} | DISPSTAT={:#06x} | VCOUNT={}",
        dispcnt,
        video_mode,
        bg_enables,
        obj_enable,
        forced_blank,
        cpu.bus.ppu.dispstat(),
        cpu.bus.ppu.vcount()
    ));

    // describes the control and scroll of the enabled backgrounds
    for bg in 0..4 {
        if bg_enables & (1 << bg) != 0 {
            lines.push(format!(
                "         | BG{}: CNT={:#06x} hofs={} vofs={}",
                bg,
                cpu.bus.ppu.bgcnt(bg),
                cpu.bus.ppu.bg_hofs(bg),
                cpu.bus.ppu.bg_vofs(bg)
            ));
        }
    }

    let ime = cpu.bus.irq.ime();
    let ie = cpu.bus.irq.ie();
    let if_ = cpu.bus.irq.if_();
    lines.push(format!(
        "         | IME={} IE={:#06x} IF={:#06x} | halted={}",
        ime,
        ie,
        if_,
        cpu.halted()
    ));

    // describes timer state
    for (i, timer) in cpu.bus.timers.timers.iter().enumerate() {
        if timer.enabled() {
            lines.push(format!(
                "         | TM{}: cnt={:#06x} reload={:#06x} ctrl={:#06x} cascade={} irq={}",
                i,
                timer.counter(),
                timer.reload(),
                timer.control(),
                timer.cascade(),
                timer.irq_enable()
            ));
        }
    }
    // show CPSR I bit (IRQ disable)
    let irq_disabled = cpu.cpsr() & 0x80 != 0;
    if irq_disabled {
        lines.push(String::from("         | ** CPSR I=1 (IRQs disabled) **"));
    }

    lines.join("\n")
}

/// Saves the current frame buffer as a PPM (P6) image file.
///
/// Writes a 240x160 raw RGB image to the given path, useful for
/// visual inspection and automated image comparison tests.
fn save_frame_buffer_ppm(gba: &GameBoyAdvance, path: &Path) -> Result<(), Error> {
    let frame_buffer = gba.frame_buffer();
    let mut file = File::create(path)?;
    write!(file, "P6\n240 160\n255\n")?;
    file.write_all(frame_buffer)?;
    println!("Frame buffer saved to {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, read, remove_dir_all},
        path::PathBuf,
        process,
    };

    use super::{
        audio_issues, audio_verdict, description_audio, description_state, run_audio_diagnostics,
        run_diagnostics, save_audio_wav, save_frame_buffer_ppm,
    };
    use crate::gba::GameBoyAdvance;

    /// Builds an empty directory for the files of a test, unique to the
    /// test and to the process running it, so that runs never collide.
    fn build_directory(name: &str) -> PathBuf {
        let directory = temp_dir().join(format!("boytacean-{name}-{}", process::id()));
        if directory.exists() {
            remove_dir_all(&directory).unwrap();
        }
        create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn test_run_diagnostics() {
        // the first 15 frames and the last one are saved
        let directory = build_directory("gba-diagnostics");
        let mut gba = GameBoyAdvance::new();
        run_diagnostics(&mut gba, 17, &directory).unwrap();
        assert_eq!(gba.ppu_frame(), 17);
        for frame in 0..15 {
            assert!(directory.join(format!("gba_frame_{frame}.ppm")).exists());
        }
        assert!(!directory.join("gba_frame_15.ppm").exists());
        let data = read(directory.join("gba_frame_16.ppm")).unwrap();
        assert_eq!(&data[..15], b"P6\n240 160\n255\n");
        assert_eq!(&data[15..], gba.frame_buffer());
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_run_diagnostics_missing_directory() {
        // a snapshot that can't be saved stops the run with its error
        let directory = build_directory("gba-diagnostics-missing");
        let mut gba = GameBoyAdvance::new();
        assert!(run_diagnostics(&mut gba, 2, &directory.join("missing")).is_err());
        assert_eq!(gba.ppu_frame(), 1);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_run_audio_diagnostics() {
        let directory = build_directory("gba-audio-diagnostics");
        let mut gba = GameBoyAdvance::new();
        gba.cpu.bus.apu.set_soundcnt_x(0x80);
        gba.cpu.bus.apu.set_soundcnt_h(0x0304);
        gba.cpu.bus.apu.direct_sound[0].write_fifo(0x04030201);
        gba.cpu.bus.timers.write_reload(0, 0xFDA8);
        gba.cpu.bus.timers.write_control(0, 0x80);
        run_audio_diagnostics(&mut gba, 1, &directory).unwrap();
        assert_eq!(gba.ppu_frame(), 1);
        let data = read(directory.join("gba_audio_diag.wav")).unwrap();
        assert_eq!(&data[..4], b"RIFF");
        assert!(data.len() > 44);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_run_audio_diagnostics_missing_directory() {
        let directory = build_directory("gba-audio-diagnostics-missing");
        let mut gba = GameBoyAdvance::new();
        assert!(run_audio_diagnostics(&mut gba, 1, &directory.join("missing")).is_err());
        assert_eq!(gba.ppu_frame(), 1);
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_description_audio() {
        let mut gba = GameBoyAdvance::new();
        assert_eq!(
            description_audio(&gba),
            "  Master=OFF LegVol=0 | A: vol=50% L=false R=false tmr=0 | B: vol=50% L=false R=false tmr=0"
        );

        // timer 0 at 1/600 of the clock feeds FIFO A (left and right at
        // full volume), timer 1 at 1/(256 * 64) of it feeds FIFO B (left)
        gba.cpu.bus.apu.set_soundcnt_x(0x80);
        gba.cpu.bus.apu.set_soundcnt_h(0x6306);
        gba.cpu.bus.timers.write_reload(0, 0xFDA8);
        gba.cpu.bus.timers.write_control(0, 0x80);
        gba.cpu.bus.timers.write_reload(1, 0xFF00);
        gba.cpu.bus.timers.write_control(1, 0x81);
        gba.cpu.bus.dma.channels[1].set_src_reg(0x0200_0000);
        gba.cpu.bus.dma.channels[1].set_dst_reg(0x0400_00A0);
        gba.cpu.bus.dma.channels[1].set_control(0xB600, 1);
        assert_eq!(
            description_audio(&gba),
            [
                "  Master=ON LegVol=2 | A: vol=100% L=true R=true tmr=0 | B: vol=50% L=true R=false tmr=1",
                "  Timer0: reload=0xfda8 period=600 prescaler=1 freq=27962.0Hz cascade=false",
                "  Timer1: reload=0xff00 period=256 prescaler=64 freq=1024.0Hz cascade=false",
                "  DMA1: src_reg=0x02000000 dst_reg=0x040000a0 src=0x02000000 dst=0x040000a0 cnt=0 timing=Spc w32=true rep=true act=false",
            ]
            .join("\n")
        );
    }

    #[test]
    fn test_description_audio_prescalers() {
        for (control, prescaler, freq) in [(0x82, 256, "65536.0Hz"), (0x83, 1024, "16384.0Hz")] {
            let mut gba = GameBoyAdvance::new();
            gba.cpu.bus.timers.write_reload(0, 0xFFFF);
            gba.cpu.bus.timers.write_control(0, control);
            let description = description_audio(&gba);
            assert!(description.contains(&format!("prescaler={prescaler} freq={freq}")));
        }
    }

    #[test]
    fn test_description_audio_dma_timing() {
        // only the enabled DMA1 and DMA2 channels are described
        for (control, timing) in [(0x8000, "Imm"), (0x9000, "VBl"), (0xA000, "HBl")] {
            let mut gba = GameBoyAdvance::new();
            gba.cpu.bus.dma.channels[2].set_control(control, 2);
            gba.cpu.bus.dma.channels[3].set_control(control, 3);
            let description = description_audio(&gba);
            assert!(description.contains("\n  DMA2: "));
            assert!(description.ends_with(&format!(
                " timing={timing} w32=false rep=false act={}",
                control == 0x8000
            )));
            assert!(!description.contains("DMA1"));
            assert!(!description.contains("DMA3"));
        }
    }

    #[test]
    fn test_audio_issues() {
        // without DirectSound configured there's nothing to diagnose
        let mut gba = GameBoyAdvance::new();
        assert!(audio_issues(&gba, 10).is_empty());

        // a DirectSound configured with the master sound off, no
        // output enabled and no DMA feeding its (empty) FIFOs
        gba.cpu.bus.apu.set_soundcnt_h(0x0004);
        assert_eq!(
            audio_issues(&gba, 6),
            vec![
                "Master sound OFF",
                "No DirectSound L/R enabled",
                "Both FIFOs empty",
                "No DMA1/2 with SPECIAL timing"
            ]
        );

        // the empty FIFOs are only an issue past the first frames
        assert_eq!(
            audio_issues(&gba, 5),
            vec!["Master sound OFF", "No DirectSound L/R enabled"]
        );
    }

    #[test]
    fn test_audio_issues_dma_destination() {
        // a DMA with special timing that doesn't write into a FIFO
        let mut gba = GameBoyAdvance::new();
        gba.cpu.bus.apu.set_soundcnt_x(0x80);
        gba.cpu.bus.apu.set_soundcnt_h(0x0300);
        gba.cpu.bus.dma.channels[1].set_dst_reg(0x0400_00A0);
        gba.cpu.bus.dma.channels[1].set_control(0xB600, 1);
        gba.cpu.bus.dma.channels[2].set_dst_reg(0x0200_0000);
        gba.cpu.bus.dma.channels[2].set_control(0xB600, 2);
        assert_eq!(
            audio_issues(&gba, 6),
            vec!["Both FIFOs empty", "DMA2 dst=0x02000000 not FIFO addr"]
        );
    }

    #[test]
    fn test_audio_issues_fifo_output() {
        // a FIFO with data that is never popped into the output
        let mut gba = GameBoyAdvance::new();
        gba.cpu.bus.apu.set_soundcnt_x(0x80);
        gba.cpu.bus.apu.set_soundcnt_h(0x3000);
        gba.cpu.bus.apu.direct_sound[1].write_fifo(0x04030201);
        assert_eq!(
            audio_issues(&gba, 6),
            vec!["FIFOs have data but DS output=0"]
        );
    }

    #[test]
    fn test_audio_verdict() {
        assert!(audio_verdict(None, None, None, None).contains("Game never wrote SOUNDCNT_H"));
        assert!(audio_verdict(Some(1), None, None, None).contains("FIFO never populated"));
        assert!(audio_verdict(Some(1), Some(2), None, None).contains("DS output always 0"));
        assert!(audio_verdict(Some(1), Some(2), Some(3), None).contains("audio buffer always zero"));
        assert!(audio_verdict(Some(1), Some(2), Some(3), Some(3)).contains("has non-zero samples"));

        // the output of the pipeline prevails over its earlier stages
        assert_eq!(
            audio_verdict(None, None, Some(3), Some(3)),
            audio_verdict(Some(1), Some(2), Some(3), Some(3))
        );
    }

    #[test]
    fn test_save_audio_wav() {
        let directory = build_directory("gba-audio-wav");
        let path = directory.join("audio.wav");
        for samples in [vec![], vec![i16::MIN, i16::MAX, -1, 0]] {
            save_audio_wav(&samples, &path).unwrap();
            let data = read(&path).unwrap();
            assert_eq!(data.len(), 44 + samples.len() * 2);
            assert_eq!(&data[..4], b"RIFF");
            assert_eq!(&data[4..8], &((data.len() - 8) as u32).to_le_bytes());
            assert_eq!(&data[8..16], b"WAVEfmt ");
            assert_eq!(&data[16..24], &[16, 0, 0, 0, 1, 0, 2, 0]);
            assert_eq!(&data[24..28], &32768u32.to_le_bytes());
            assert_eq!(&data[28..32], &131072u32.to_le_bytes());
            assert_eq!(&data[32..36], &[4, 0, 16, 0]);
            assert_eq!(&data[36..40], b"data");
            assert_eq!(&data[40..44], &((samples.len() * 2) as u32).to_le_bytes());
            for (i, sample) in samples.iter().enumerate() {
                assert_eq!(&data[44 + i * 2..46 + i * 2], &sample.to_le_bytes());
            }
        }
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_save_audio_wav_create_error() {
        assert!(save_audio_wav(&[1, 2], &temp_dir()).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_save_audio_wav_write_error() {
        assert!(save_audio_wav(&[1, 2], std::path::Path::new("/dev/full")).is_err());
    }

    #[test]
    fn test_description_state() {
        let mut gba = GameBoyAdvance::new();
        gba.cpu.set_cpsr(0x12 | 0x80 | 0x20);
        gba.cpu.bus.ppu.set_dispcnt(0x1503);
        gba.cpu.bus.ppu.set_bgcnt(2, 0x1F84);
        gba.cpu.bus.ppu.set_bg_hofs(2, 0x0123);
        gba.cpu.bus.ppu.set_bg_vofs(2, 0x0045);
        gba.cpu.bus.irq.set_ime(true);
        gba.cpu.bus.irq.set_ie(0x0009);
        gba.cpu.bus.timers.write_reload(2, 0x1234);
        gba.cpu.bus.timers.write_control(2, 0xC4);
        let description = description_state(&gba, 7, 280896);
        let lines: Vec<&str> = description.lines().collect();
        assert_eq!(lines.len(), 7);
        assert!(lines[0].starts_with("Frame   7 | PC="));
        assert!(lines[0].ends_with("| Mode=IRQ | THUMB | CPSR=0x000000b2 | cycles=280896"));
        assert!(lines[1].starts_with(
            "         | DISPCNT=0x1503 mode=3 BGs=0101 OBJ=1 forced_blank=0 | DISPSTAT="
        ));
        assert!(lines[2].starts_with("         | BG0: CNT="));
        assert_eq!(lines[3], "         | BG2: CNT=0x1f84 hofs=291 vofs=69");
        assert_eq!(
            lines[4],
            "         | IME=true IE=0x0009 IF=0x0000 | halted=false"
        );
        assert_eq!(
            lines[5],
            "         | TM2: cnt=0x1234 reload=0x1234 ctrl=0x00c4 cascade=true irq=true"
        );
        assert_eq!(lines[6], "         | ** CPSR I=1 (IRQs disabled) **");
    }

    #[test]
    fn test_description_state_mode() {
        for (mode, name) in [
            (0x10, "USR"),
            (0x11, "FIQ"),
            (0x12, "IRQ"),
            (0x13, "SVC"),
            (0x17, "ABT"),
            (0x1B, "UND"),
            (0x1F, "SYS"),
            (0x15, "???"),
        ] {
            let mut gba = GameBoyAdvance::new();
            gba.cpu.set_cpsr(mode);
            let description = description_state(&gba, 0, 0);
            assert!(description.contains(&format!("| Mode={name} | ARM   |")));
            assert!(!description.contains("IRQs disabled"));
        }
    }

    #[test]
    fn test_save_frame_buffer_ppm() {
        let directory = build_directory("gba-frame-ppm");
        let path = directory.join("frame.ppm");
        let mut gba = GameBoyAdvance::new();
        gba.cpu.bus.ppu.set_dispcnt(0x80);
        gba.next_frame();
        save_frame_buffer_ppm(&gba, &path).unwrap();
        let data = read(&path).unwrap();
        assert_eq!(&data[..15], b"P6\n240 160\n255\n");
        assert_eq!(data.len(), 15 + 240 * 160 * 3);
        assert!(data[15..].iter().all(|&b| b == 0xFF));
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_save_frame_buffer_ppm_create_error() {
        assert!(save_frame_buffer_ppm(&GameBoyAdvance::new(), &temp_dir()).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_save_frame_buffer_ppm_write_error() {
        assert!(
            save_frame_buffer_ppm(&GameBoyAdvance::new(), std::path::Path::new("/dev/full"))
                .is_err()
        );
    }
}
