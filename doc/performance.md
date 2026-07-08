# Performance

This document describes the performance characteristics of the Boytacean emulation core, the results of a structured audit of its hot paths, a benchmark comparison against other Game Boy emulators and the list of optimization opportunities that were identified (both the ones already applied and the ones left as future work).

## Methodology

All measurements were taken on an Apple M5 Pro (macOS), using release builds (`opt-level = 3`, fat LTO, single codegen unit) and the `bench_headless` example, which clocks the emulator as fast as possible with the APU disabled (unless stated otherwise) and no frame-buffer extraction:

```bash
cargo build --release --example bench_headless
./target/release/examples/bench_headless <rom.gb> [frames] [--apu] [--cgb]
```

The comparison against [mGBA](https://mgba.io) uses the `mgba-perf` tool built from the mGBA `master` branch (`cmake -DBUILD_PERF=ON`), running the same ROMs for the same number of frames. Numbers are the best of three interleaved runs of 12000 frames each, which bounds the run-to-run noise (~±3% on this machine, mostly scheduling and thermal effects).

Emulation output equivalence was verified by hashing the complete frame buffer of every frame across long runs (dmg-acid2, cgb-acid2 and multiple game ROMs, in both DMG and CGB modes) before and after the changes, with bit-exact results.

## Results

Frames per second, best of three interleaved runs of 12000 frames, higher is better:

| ROM | Mode | Boytacean | Boytacean (APU) | mGBA |
| --- | ---- | --------- | --------------- | ---- |
| pocket.gb | DMG | 16318 | 12068 | 8595 |
| shocklobster.gb | DMG | 17188 | 11628 | 21868 |
| opus5.gb | DMG | 18492 | 14387 | 8435 |
| cgb_acid2.gbc | CGB | 10439 | - | 22986 |

With audio emulation on both sides, Boytacean is ~1.4-1.7x faster than mGBA on typical DMG workloads, while mGBA is ~1.9x faster on shocklobster (a sprite and interrupt heavy action game) and ~2.2x faster on the CGB test ROM. The pattern is consistent with the audit findings below: Boytacean wins on rendering-bound scenes, mGBA wins on halt/interrupt-bound and CGB scenes thanks to its event-driven scheduler.

Notes on fairness: `mgba-perf` always emulates audio, so the closest comparison column is Boytacean with `--apu`. mGBA is also a full-system emulator whose Game Boy core (SM83) favours accuracy features (event scheduler, cycle-level timings) that Boytacean's simpler loop does not implement.

## Internal profiling

The `profile` feature adds low-overhead instrumentation to the emulation core, giving visibility over where the time is spent and how memory is accessed, without any cost when the feature is disabled:

```bash
cargo build --release --example bench_headless --features profile
./target/release/examples/bench_headless <rom.gb> [frames]
```

The gathered metrics include per-frame time, background/window and object rendering times, instruction and rendered line counters and MMU read/write counters per memory region. Timing is only measured around chunky operations (complete frames and scanline render passes) so that the timer overhead does not skew the attribution, while the counters are updated per event.

Example output for a typical DMG game (values per benchmark run):

```text
Frame            354.665 ms (100.00%)
Render BG         54.251 ms (15.30%)
Render OBJ        15.660 ms ( 4.42%)
Other            284.754 ms (80.29%)
Frames        5000
Instructions  84853026
Render lines  720000
APU samples   0
Reads         5101766 (rom=3137912 vram=0 eram=0 wram=1282783 oam=0 io=9650 hram=665000)
Writes        1177724 (rom=0 vram=0 eram=0 wram=290722 oam=800000 io=52002 hram=35000)
```

## Audit findings

The main findings of the audit, from profiling data and code inspection:

* The emulation loop runs at roughly 3.5-4 ns per `clock()` call in release builds, meaning most micro-optimizations are at the ~1% level and hard to distinguish from measurement noise.
* Games spend the large majority of each frame in the `HALT` state waiting for the V-Blank interrupt, and every halted tick (4 cycles) still pays the full loop cost: interrupt flag synthesis from five separate component flags, speed normalization and the clocking of all devices. This per-tick overhead, not rendering, dominates the frame time (~80%).
* Background/window map rendering accounts for ~15% of frame time and object rendering for ~4% in sprite-heavy DMG games.
* Memory access is dominated by ROM reads (~61%), followed by WRAM (~25%) and HRAM (~13%). OAM writes are almost entirely produced by the per-frame OAM DMA transfer.
* Logging and assertion machinery (`debug`, `pedantic`, `cpulog` features) is cleanly compiled out of release builds and does not leak into the hot path.

## Applied optimizations

The following low-risk optimizations were applied, keeping the emulation output bit-exact:

* Interrupt flag synthesis in `Cpu::clock()` is now skipped when it cannot affect execution (IME disabled and not halted, or no interrupt source enabled in IE).
* Cycle normalization for double speed (CGB) uses a shift (`GameBoySpeed::shift()`) instead of an integer division on every `clock()` call.
* DMG background/window rendering fetches a tile row slice once per tile instead of computing a per-pixel tile buffer index, mirroring the approach already used for objects.
* Object height is computed once per scanline instead of once per OAM entry in `render_objects()`.
* The hot `Tile` accessors (`get`, `get_flipped`, `get_row`) are now marked `#[inline(always)]`.

The combined effect on frame throughput is in the +1% to +6% range depending on the workload (largest on background-heavy scenes), with no regression on any of the benchmarked ROMs. On the `cpu_cycles` criterion benchmark, which isolates the clocking loop, the improvement is ~13% (510 us to 445 us per iteration).

## Future work

Higher-impact opportunities that require structural changes and were intentionally left out of the safe tuning pass:

* **Cached IF register**: the interrupt flags are currently synthesized from five booleans scattered across the PPU, timer, serial and gamepad on every instruction. Maintaining a single hardware-like IF byte updated at raise/acknowledge time would collapse the per-tick interrupt check to one AND operation. This is the single most promising change given the halt-dominated profile.
* **Event-driven scheduling (mGBA style)**: instead of ticking every component every 4 cycles during `HALT`, compute the next event timestamp (PPU mode transition, timer overflow, DMA completion) and jump directly to it. This is the main architectural difference explaining mGBA's advantage on CPU-heavy workloads.
* **Flat page-table memory map (Gambatte/mGBA style)**: replace the nested `match` plus MBC function-pointer dispatch with a precomputed table of page pointers for plain-memory regions, reserving the slow path for IO registers. This would cut the cost of every opcode fetch and memory operand access.
* **Deferred CGB pixel expansion**: CGB rendering writes 3 RGB bytes per pixel in the hot loop, while DMG mode already defers the palette expansion to `frame_buffer()`. Applying the same technique to CGB would remove the per-pixel palette dereference and stores.
* **Avoid the per-scanline `bg_map_attrs` copy (CGB)**: `render_map()` copies the background attributes array by value on every scanline; borrowing it instead requires restructuring the borrows in the render path.
* **Reusable DMA/frame buffers**: OAM DMA and HDMA allocate a temporary `Vec` per transfer and `clocks_frame_buffer()` allocates a fresh frame copy per frame; both are once-per-frame level costs, relevant mostly for FFI-heavy consumers (Python/WASM).
