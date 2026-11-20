//! Tracing utility for GBA emulation debugging
//!
//! Steps a GBA ROM one instruction at a time, recording a ring buffer
//! of recent CPU states. When execution jumps into an unmapped memory
//! region (typical null function pointer crash) the ring buffer is
//! dumped with full register state so the faulty code path can be
//! identified. The changes of a watched memory word are reported with
//! the instruction that made them, and the trace can stop once the
//! execution reaches a given address.
//!
//! # Usage
//! gba-trace <rom.gba> \[max_frames\] \[--bios <bios.bin>\] \[--watch <address>\] \[--until-pc <address>\]

use std::{env::args, error::Error};

use boytacean::gba::GameBoyAdvance;
use boytacean_common::util::read_file;

const RING_SIZE: usize = 65536;
const DEFAULT_FRAMES: u64 = 600;

#[derive(Clone, Copy, Default)]
struct TraceEntry {
    frame: u64,
    pc: u32,
    opcode: u32,
    cpsr: u32,
    regs: [u32; 16],
}

/// The options of a run of the utility, as provided in its arguments.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Options {
    rom: String,
    frames: u64,
    bios: Option<String>,
    watch: Option<u32>,
    until_pc: Option<u32>,
}

fn print_usage() {
    println!("Usage: gba-trace <rom.gba> [max_frames] [options]");
    println!("If max_frames is not specified, it traces up to {DEFAULT_FRAMES} frames");
    println!("Options:");
    println!("  --bios <bios.bin>      Boots the ROM through the BIOS file");
    println!("  --watch <address>      Prints each change of the word at the hex address");
    println!("                         (stopping at the first change of an IO register)");
    println!("  --until-pc <address>   Stops once the execution reaches the hex address");
}

/// Parses the arguments of the utility, `None` when they are not valid.
fn parse_args(args: &[String]) -> Option<Options> {
    let mut options = Options {
        rom: args.first().filter(|arg| !arg.starts_with("--"))?.clone(),
        frames: DEFAULT_FRAMES,
        bios: None,
        watch: None,
        until_pc: None,
    };
    let mut frames = None;
    let mut args = args[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bios" => options.bios = Some(args.next()?.clone()),
            "--watch" => options.watch = Some(parse_address(args.next()?)?),
            "--until-pc" => options.until_pc = Some(parse_address(args.next()?)?),
            arg if arg.starts_with("--") || frames.is_some() => return None,
            arg => frames = Some(arg.parse::<u64>().ok().filter(|&value| value > 0)?),
        }
    }
    options.frames = frames.unwrap_or(DEFAULT_FRAMES);
    Some(options)
}

/// Parses an address in hexadecimal, with or without its `0x` prefix.
fn parse_address(value: &str) -> Option<u32> {
    let digits = value.strip_prefix("0x").unwrap_or(value);
    if digits.is_empty() || !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

/// Loads the ROM of the options into a new GBA, booting it through
/// the BIOS of the options when one is provided.
fn load(options: &Options) -> Result<GameBoyAdvance, Box<dyn Error>> {
    let mut gba = GameBoyAdvance::new();
    let info = gba.load_rom(&read_file(&options.rom)?)?;
    println!("Loaded: {} ({})", info.title(), info.game_code());

    if let Some(path) = &options.bios {
        gba.load_bios(&read_file(path)?);
        println!("BIOS loaded from {path}");
    }
    Ok(gba)
}

/// Traces the execution of the GBA for up to the frames of the options,
/// stopping early when the execution enters an unmapped region, reaches
/// the address to stop at or changes a watched IO register, returning
/// the number of instructions executed.
fn trace(gba: &mut GameBoyAdvance, options: &Options) -> u64 {
    let mut ring = vec![TraceEntry::default(); RING_SIZE];
    let mut ring_pos = 0usize;
    let mut ring_len = 0usize;
    let mut prev_watch = options.watch.map(|addr| read_watch(gba, addr));
    let mut instr_count = 0u64;

    while gba.ppu_frame() < options.frames {
        let thumb = gba.cpu.cpsr() & 0x20 != 0;
        let exec_pc = if thumb {
            gba.cpu.pc().wrapping_sub(4)
        } else {
            gba.cpu.pc().wrapping_sub(8)
        };

        // records the entry only when execution is in a sane region so
        // the ring holds the path that led into the bad jump; right after
        // a pipeline flush regs[15] has no prefetch offset yet, so both
        // the raw and the offset PC must be bogus to flag a crash
        let sane = |addr: u32| matches!(addr >> 24, 0x02 | 0x03 | 0x08..=0x0D) || addr < 0x4000;
        let region_ok = sane(exec_pc) || sane(gba.cpu.pc());
        if region_ok && !gba.cpu.halted() {
            let opcode = if thumb {
                gba.cpu.bus.read16(exec_pc) as u32
            } else {
                gba.cpu.bus.read32(exec_pc)
            };
            let mut regs = [0u32; 16];
            for (r, reg) in regs.iter_mut().enumerate() {
                *reg = gba.cpu.reg(r as u32);
            }
            ring[ring_pos] = TraceEntry {
                frame: gba.ppu_frame(),
                pc: exec_pc,
                opcode,
                cpsr: gba.cpu.cpsr(),
                regs,
            };
            ring_pos = (ring_pos + 1) % RING_SIZE;
            ring_len = ring_len.saturating_add(1).min(RING_SIZE);
        }

        if let Some(target) = options.until_pc {
            if exec_pc == target || gba.cpu.pc() == target {
                println!(
                    "\n== reached PC={:#010x} at frame {} instr {}",
                    target,
                    gba.ppu_frame(),
                    instr_count
                );
                dump_regs(gba);
                dump_entry_state(gba);
                return instr_count;
            }
        }

        if !region_ok && !gba.cpu.halted() {
            println!(
                "\n!! PC entered unmapped region at frame {} instr {}: PC={:#010x}",
                gba.ppu_frame(),
                instr_count,
                exec_pc
            );
            dump_ring(&ring, ring_pos, ring_len);
            dump_regs(gba);
            return instr_count;
        }

        gba.clock();
        instr_count += 1;

        // reports each change of the watched word with the instruction
        // that produced it (the last change before a crash is the writer
        // of the corrupted value); IO addresses dump the ring instead so
        // the code path into the first write can be inspected
        if let (Some(addr), Some(prev)) = (options.watch, prev_watch) {
            let value = read_watch(gba, addr);
            if value != prev {
                println!(
                    "-- frame {} instr {}: [{:#010x}] {:#010x} -> {:#010x} (PC={:#010x})",
                    gba.ppu_frame(),
                    instr_count,
                    addr,
                    prev,
                    value,
                    exec_pc
                );
                if addr >> 24 == 0x04 {
                    dump_ring(&ring, ring_pos, ring_len);
                    dump_regs(gba);
                    return instr_count;
                }
                prev_watch = Some(value);
            }
        }
    }

    println!(
        "\nCompleted {} frames without crash ({instr_count} instructions)",
        options.frames
    );
    dump_ring(&ring, ring_pos, ring_len);
    dump_regs(gba);
    dump_ppu(gba);
    instr_count
}

/// Reads the watched value at the address, a halfword for the IO
/// registers (so that the next register is not taken as part of it)
/// and a word otherwise.
fn read_watch(gba: &mut GameBoyAdvance, addr: u32) -> u32 {
    if addr >> 24 == 0x04 {
        gba.cpu.bus.read16(addr) as u32
    } else {
        gba.cpu.bus.read32(addr)
    }
}

/// Dumps the IO and BIOS work RAM state relevant at ROM entry.
fn dump_entry_state(gba: &mut GameBoyAdvance) {
    println!(
        "  cpsr={:#010x} spsr={:#010x}",
        gba.cpu.cpsr(),
        gba.cpu.spsr()
    );
    for (name, addr) in [
        ("DISPCNT", 0x0400_0000u32),
        ("DISPSTAT", 0x0400_0004),
        ("IE", 0x0400_0200),
        ("IF", 0x0400_0202),
        ("IME", 0x0400_0208),
        ("POSTFLG", 0x0400_0300),
        ("SOUNDCNT_X", 0x0400_0084),
        ("SOUNDBIAS", 0x0400_0088),
        ("RCNT", 0x0400_0134),
        ("SIOCNT", 0x0400_0128),
        ("WAITCNT", 0x0400_0204),
    ] {
        println!("  {name}={:#06x}", gba.cpu.bus.read16(addr));
    }
    print!("  0x03007F00..0x03008000:");
    for i in 0..64u32 {
        let addr = 0x0300_7F00 + i * 4;
        if i % 8 == 0 {
            print!("\n    {addr:#010x}:");
        }
        print!(" {:08x}", gba.cpu.bus.read32(addr));
    }
    println!();
}

/// Dumps the PPU compositing state and the visible OAM entries.
fn dump_ppu(gba: &GameBoyAdvance) {
    let ppu = &gba.cpu.bus.ppu;
    println!(
        "ppu: DISPCNT={:#06x} BLDCNT={:#06x} WININ={:#06x} WINOUT={:#06x}",
        ppu.dispcnt(),
        ppu.bldcnt(),
        ppu.winin(),
        ppu.winout()
    );
    for bg in 0..4 {
        println!(
            "  BG{}: CNT={:#06x} hofs={} vofs={}",
            bg,
            ppu.bgcnt(bg),
            ppu.bg_hofs(bg),
            ppu.bg_vofs(bg)
        );
    }
    let mut shown = 0;
    for i in 0..128 {
        let base = i * 8;
        let attr0 = u16::from_le_bytes([gba.cpu.bus.oam[base], gba.cpu.bus.oam[base + 1]]);
        let attr1 = u16::from_le_bytes([gba.cpu.bus.oam[base + 2], gba.cpu.bus.oam[base + 3]]);
        let attr2 = u16::from_le_bytes([gba.cpu.bus.oam[base + 4], gba.cpu.bus.oam[base + 5]]);
        // skip disabled entries (non-affine with the disable bit set)
        if attr0 & (1 << 8) == 0 && attr0 & (1 << 9) != 0 {
            continue;
        }
        if shown < 24 {
            println!("  OAM{i}: attr0={attr0:#06x} attr1={attr1:#06x} attr2={attr2:#06x}");
        }
        shown += 1;
    }
    println!("  visible OAM entries: {shown}");
}

/// Dumps the most recent entries of the trace ring buffer.
fn dump_ring(ring: &[TraceEntry], ring_pos: usize, ring_len: usize) {
    println!("--- last {ring_len} instructions ---");
    for i in 0..ring_len {
        let index = (ring_pos + RING_SIZE - ring_len + i) % RING_SIZE;
        let entry = &ring[index];
        let thumb = entry.cpsr & 0x20 != 0;
        let mode = entry.cpsr & 0x1F;
        println!(
            "F{:<4} {:#010x}: {:08x} [{}|{:02x}] r0={:08x} r1={:08x} r2={:08x} r3={:08x} r12={:08x} sp={:08x} lr={:08x}",
            entry.frame,
            entry.pc,
            entry.opcode,
            if thumb { "T" } else { "A" },
            mode,
            entry.regs[0],
            entry.regs[1],
            entry.regs[2],
            entry.regs[3],
            entry.regs[12],
            entry.regs[13],
            entry.regs[14],
        );
    }
}

/// Dumps the current CPU register state.
fn dump_regs(gba: &GameBoyAdvance) {
    print!("regs:");
    for r in 0..16 {
        print!(" r{}={:#010x}", r, gba.cpu.reg(r));
    }
    println!(" cpsr={:#010x}", gba.cpu.cpsr());
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().skip(1).collect();
    let Some(options) = parse_args(&args) else {
        print_usage();
        return Ok(());
    };
    let mut gba = load(&options)?;
    trace(&mut gba, &options);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_dir_all, write},
        path::PathBuf,
        process,
    };

    use boytacean::gba::GameBoyAdvance;

    use super::{load, parse_address, parse_args, read_watch, trace, Options, DEFAULT_FRAMES};

    /// The test ROM loaded by the tests, relative to the package root.
    const ROM_PATH: &str = "res/roms.gba/test/jsmolka_gba-tests/arm.gba";

    /// The `mov r0, r0` ARM instruction, that does nothing.
    const NOP: u32 = 0xE1A0_0000;

    /// The `b .` ARM instruction, that loops forever.
    const LOOP: u32 = 0xEAFF_FFFE;

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

    fn build_args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| String::from(*arg)).collect()
    }

    fn build_options(frames: u64, watch: Option<u32>, until_pc: Option<u32>) -> Options {
        Options {
            rom: String::from(ROM_PATH),
            frames,
            bios: None,
            watch,
            until_pc,
        }
    }

    /// Builds a GBA with a minimal ROM (only the fixed value of its
    /// header set) whose entry point runs the provided ARM program.
    fn build_gba(program: &[u32]) -> GameBoyAdvance {
        let mut rom = vec![0x00; 0x200];
        rom[0xB2] = 0x96;
        for (index, word) in program.iter().enumerate() {
            rom[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        let mut gba = GameBoyAdvance::new();
        gba.load_rom(&rom).unwrap();
        gba
    }

    #[test]
    fn test_parse_args() {
        assert_eq!(
            parse_args(&build_args(&["game.gba"])),
            Some(Options {
                rom: String::from("game.gba"),
                frames: DEFAULT_FRAMES,
                bios: None,
                watch: None,
                until_pc: None,
            })
        );
        assert_eq!(
            parse_args(&build_args(&[
                "game.gba",
                "--watch",
                "0x03007ffc",
                "60",
                "--until-pc",
                "8000000",
                "--bios",
                "bios.bin"
            ])),
            Some(Options {
                rom: String::from("game.gba"),
                frames: 60,
                bios: Some(String::from("bios.bin")),
                watch: Some(0x0300_7FFC),
                until_pc: Some(0x0800_0000),
            })
        );
    }

    #[test]
    fn test_parse_args_invalid() {
        for args in [
            vec![],
            vec!["--watch", "0x03007ffc"],
            vec!["game.gba", "100", "--bois", "x"],
            vec!["game.gba", "100", "200"],
            vec!["game.gba", "0"],
            vec!["game.gba", "x"],
            vec!["game.gba", "--until-pc", "0xZZ"],
            vec!["game.gba", "--watch", "1003007ffc"],
            vec!["game.gba", "--watch"],
            vec!["game.gba", "--until-pc"],
            vec!["game.gba", "--bios"],
        ] {
            assert_eq!(parse_args(&build_args(&args)), None);
        }
    }

    #[test]
    fn test_parse_address() {
        assert_eq!(parse_address("0x03007ffc"), Some(0x0300_7FFC));
        assert_eq!(parse_address("0x03007FFC"), Some(0x0300_7FFC));
        assert_eq!(parse_address("8000000"), Some(0x0800_0000));
        assert_eq!(parse_address("0xffffffff"), Some(u32::MAX));
        assert_eq!(parse_address("0x100000000"), None);
        assert_eq!(parse_address("0x"), None);
        assert_eq!(parse_address(""), None);
        assert_eq!(parse_address("0x0x10"), None);
        assert_eq!(parse_address("+10"), None);
        assert_eq!(parse_address("0xZZ"), None);
    }

    #[test]
    fn test_load() {
        let gba = load(&build_options(1, None, None)).unwrap();
        assert_eq!(gba.rom_info().unwrap().title(), "GBA Tests");
        assert!(!gba.cpu.bus.use_real_bios);
        assert_eq!(gba.cpu.pc(), 0x0800_0000);
    }

    #[test]
    fn test_load_bios() {
        // a BIOS boots the ROM from its own entry point
        let directory = build_directory("gba-trace-bios");
        let bios = directory.join("bios.bin");
        write(&bios, vec![0x00; 0x4000]).unwrap();
        let mut options = build_options(1, None, None);
        options.bios = Some(bios.to_string_lossy().into_owned());
        let gba = load(&options).unwrap();
        assert!(gba.cpu.bus.use_real_bios);
        assert_eq!(gba.cpu.pc(), 0x0000_0000);

        options.bios = Some(String::from("/dev/boytacean-none.bin"));
        assert_eq!(
            load(&options).err().unwrap().to_string(),
            "Failed to open file: /dev/boytacean-none.bin"
        );
        remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_load_missing_rom() {
        let mut options = build_options(1, None, None);
        options.rom = String::from("/dev/boytacean-none.gba");
        assert_eq!(
            load(&options).err().unwrap().to_string(),
            "Failed to open file: /dev/boytacean-none.gba"
        );
    }

    #[test]
    fn test_trace() {
        let mut gba = build_gba(&[NOP, LOOP]);
        assert!(trace(&mut gba, &build_options(2, None, None)) > 0);
        assert_eq!(gba.ppu_frame(), 2);
    }

    #[test]
    fn test_trace_unmapped() {
        // the jump into an unmapped region stops the trace before its
        // first instruction is executed (mov pc, #0x0F000000)
        let mut gba = build_gba(&[NOP, 0xE3A0_F40F, LOOP]);
        assert_eq!(trace(&mut gba, &build_options(1, None, None)), 2);
        assert_eq!(gba.ppu_frame(), 0);
        assert_eq!(gba.cpu.pc(), 0x0F00_0000);
    }

    #[test]
    fn test_trace_until_pc() {
        // stops before the instruction at the address is executed
        let mut gba = build_gba(&[NOP, NOP, NOP, LOOP]);
        assert_eq!(
            trace(&mut gba, &build_options(1, None, Some(0x0800_0008))),
            2
        );
        assert_eq!(gba.ppu_frame(), 0);

        // an address never reached runs all the frames
        let mut gba = build_gba(&[NOP, LOOP]);
        trace(&mut gba, &build_options(1, None, Some(0x0800_0100)));
        assert_eq!(gba.ppu_frame(), 1);
    }

    #[test]
    fn test_trace_until_pc_thumb() {
        // stops at a THUMB address, reached through a branch into the
        // THUMB state (add r0, pc, #1; bx r0; b .)
        let mut gba = build_gba(&[0xE28F_0001, 0xE12F_FF10, 0x0000_E7FE]);
        assert_eq!(
            trace(&mut gba, &build_options(1, None, Some(0x0800_0008))),
            2
        );
        assert_eq!(gba.ppu_frame(), 0);
        assert_ne!(gba.cpu.cpsr() & 0x20, 0);
    }

    #[test]
    fn test_trace_watch() {
        // the changes of a memory word are reported without stopping,
        // even when clearing the IRQ handler of the BIOS (mov r0,
        // #0x04000000; sub r0, r0, #4; mov r1, #5; str r1, [r0];
        // mov r1, #0; str r1, [r0])
        let mut gba = build_gba(&[
            0xE3A0_0404,
            0xE240_0004,
            0xE3A0_1005,
            0xE580_1000,
            0xE3A0_1000,
            0xE580_1000,
            LOOP,
        ]);
        trace(&mut gba, &build_options(1, Some(0x03FF_FFFC), None));
        assert_eq!(gba.ppu_frame(), 1);
        assert_eq!(gba.cpu.reg(1), 0);
    }

    #[test]
    fn test_trace_watch_io() {
        // the first change of an IO register stops the trace right
        // after the instruction that made it (mov r0, #0x04000000;
        // mov r1, #0x100; str r1, [r0])
        let mut gba = build_gba(&[0xE3A0_0404, 0xE3A0_1C01, 0xE580_1000, LOOP]);
        assert_eq!(
            trace(&mut gba, &build_options(1, Some(0x0400_0000), None)),
            3
        );
        assert_eq!(gba.ppu_frame(), 0);
        assert_eq!(gba.cpu.bus.ppu.dispcnt(), 0x0100);
    }

    #[test]
    fn test_trace_watch_io_halfword() {
        // an IO register is watched as a halfword from its first value,
        // the next register (IF after IE) never being taken as a change
        let mut gba = build_gba(&[NOP, LOOP]);
        gba.cpu.bus.irq.raise(0x0001);
        trace(&mut gba, &build_options(1, Some(0x0400_0200), None));
        assert_eq!(gba.ppu_frame(), 1);
    }

    #[test]
    fn test_read_watch() {
        let mut gba = GameBoyAdvance::new();
        gba.cpu.bus.irq.set_ie(0x0001);
        gba.cpu.bus.irq.raise(0x0002);
        gba.cpu.bus.write32(0x0300_0000, 0x1234_5678);
        assert_eq!(read_watch(&mut gba, 0x0400_0200), 0x0001);
        assert_eq!(read_watch(&mut gba, 0x0400_0202), 0x0002);
        assert_eq!(read_watch(&mut gba, 0x0300_0000), 0x1234_5678);
    }
}
