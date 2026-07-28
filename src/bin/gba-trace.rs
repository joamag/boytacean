//! Tracing utility for GBA emulation debugging
//!
//! Steps a GBA ROM one instruction at a time, recording a ring buffer
//! of recent CPU states. When execution jumps into an unmapped memory
//! region (typical null function pointer crash) the ring buffer is
//! dumped with full register state so the faulty code path can be
//! identified.
//!
//! # Usage
//! gba-trace <rom.gba> \[max_frames\] \[--bios <bios.bin>\]

use std::env;

use boytacean::gba::GameBoyAdvance;

const RING_SIZE: usize = 65536;

#[derive(Clone, Copy, Default)]
struct TraceEntry {
    frame: u64,
    pc: u32,
    opcode: u32,
    cpsr: u32,
    regs: [u32; 16],
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: gba-trace <rom.gba> [max_frames] [--bios <bios.bin>]");
        std::process::exit(1);
    }

    let rom_path = &args[1];
    let mut max_frames = 600u64;
    let mut bios_path: Option<String> = None;
    let mut watch_addr: Option<u32> = None;
    let mut until_pc: Option<u32> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--bios" => {
                if i + 1 < args.len() {
                    bios_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("--bios requires a path argument");
                    std::process::exit(1);
                }
            }
            "--watch" => {
                if i + 1 < args.len() {
                    let raw = args[i + 1].trim_start_matches("0x");
                    watch_addr = u32::from_str_radix(raw, 16).ok();
                    i += 2;
                } else {
                    eprintln!("--watch requires a hex address argument");
                    std::process::exit(1);
                }
            }
            "--until-pc" => {
                if i + 1 < args.len() {
                    let raw = args[i + 1].trim_start_matches("0x");
                    until_pc = u32::from_str_radix(raw, 16).ok();
                    i += 2;
                } else {
                    eprintln!("--until-pc requires a hex address argument");
                    std::process::exit(1);
                }
            }
            _ => {
                max_frames = args[i].parse::<u64>().unwrap_or(600);
                i += 1;
            }
        }
    }

    let data = std::fs::read(rom_path).expect("Failed to read ROM file");
    let mut gba = GameBoyAdvance::new();
    let info = gba.load_rom(&data).expect("Failed to load ROM");
    println!("Loaded: {} ({})", info.title(), info.game_code());

    if let Some(path) = &bios_path {
        let bios_data = std::fs::read(path).expect("Failed to read BIOS file");
        gba.load_bios(&bios_data);
        println!("BIOS loaded from {path}");
    }

    let mut ring = vec![TraceEntry::default(); RING_SIZE];
    let mut ring_pos = 0usize;
    let mut ring_len = 0usize;
    let mut prev_handler = 0u32;
    let mut prev_watch = watch_addr.map(|addr| gba.cpu.bus.read32(addr));
    let mut instr_count = 0u64;

    while gba.ppu_frame() < max_frames {
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

        if let Some(target) = until_pc {
            if exec_pc == target || gba.cpu.pc() == target {
                println!(
                    "\n== reached PC={:#010x} at frame {} instr {}",
                    target,
                    gba.ppu_frame(),
                    instr_count
                );
                dump_regs(&gba);
                dump_entry_state(&mut gba);
                return;
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
            dump_regs(&gba);
            return;
        }

        let handler = gba.cpu.bus.read32(0x03FF_FFFC);
        if handler != prev_handler {
            println!(
                "-- frame {} instr {}: IRQ handler {:#010x} -> {:#010x} (PC={:#010x})",
                gba.ppu_frame(),
                instr_count,
                prev_handler,
                handler,
                exec_pc
            );
            // the handler being cleared after boot is the corruption
            // signature under investigation — dump the path that did it
            if prev_handler != 0 && handler == 0 {
                dump_ring(&ring, ring_pos, ring_len);
                dump_regs(&gba);
                return;
            }
            prev_handler = handler;
        }

        gba.clock();
        instr_count += 1;

        // reports each change of the watched word with the instruction
        // that produced it (the last change before a crash is the writer
        // of the corrupted value); IO addresses dump the ring instead so
        // the code path into the first write can be inspected
        if let (Some(addr), Some(prev)) = (watch_addr, prev_watch) {
            let value = if addr >> 24 == 0x04 {
                gba.cpu.bus.read16(addr) as u32
            } else {
                gba.cpu.bus.read32(addr)
            };
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
                    dump_regs(&gba);
                    return;
                }
                prev_watch = Some(value);
            }
        }
    }

    println!("\nCompleted {max_frames} frames without crash ({instr_count} instructions)");
    dump_ring(&ring, ring_pos, ring_len);
    dump_regs(&gba);
    dump_ppu(&gba);
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
