# BOY-001: GBA Emulation Performance

## Document Information

| Field               | Value                                           |
| ------------------- | ----------------------------------------------- |
| **Document Number** | BOY-001                                         |
| **Date**            | 2026-07-28                                      |
| **Author**          | João Magalhães <joamag@gmail.com>               |
| **Subject**         | Profiling and Optimization of the GBA Emulation |
| **Status**          | Implemented                                     |
| **Version**         | 1.0                                             |

## Description

### Problem

The first complete version of the GBA emulation ran commercial games at about 3x the speed of the real hardware natively (47.9 MHz in Mario Kart Super Circuit and 59.8 MHz in Zelda Minish Cap on an Apple M1), and at about 31 MHz (barely above the 16.78 MHz of the hardware) on the web, through WASM. Profiling showed that the PPU compositing, not the CPU, dominated the execution time, at about 77% of it.

### Solution

The PPU pixel pipeline was optimized first, as the highest leverage target: the text backgrounds are collected by tile columns (one map lookup per 8 pixels) instead of per pixel, the compositing loop only iterates the enabled backgrounds, and the sprite attributes are only read once the sprite is known to be on the line. Every change was benchmarked, and the changes that didn't improve both games were dropped (see the lessons learned).

### Usage

Validate every change with `gba-bench` on a ROM, comparing against the numbers before the change:

```bash
cargo run --release --bin gba-bench -- <rom.gba> 6000 --warmup 120 --runs 3
```

And with the Criterion regression benchmark:

```bash
cargo bench --bench gba
```

## Baseline

Apple M1, native release build, before any optimization:

| ROM                      | Avg MHz | Speedup | Frames |
| ------------------------ | ------- | ------- | ------ |
| Mario Kart Super Circuit | 47.9    | 2.85x   | 6000   |
| Zelda Minish Cap         | 59.8    | 3.56x   | 6000   |

WASM: about 31 MHz on Zelda Minish Cap (user reported).

## Profiling Results

Mario Kart, with samply and macOS sample, over 3000 frames.

### Time Breakdown Within `clock()`

| Component                 | % of time | Notes                         |
| ------------------------- | --------- | ----------------------------- |
| **PPU render_composited** | **76.9%** | Per-pixel compositing loop    |
| THUMB instruction exec    | 5.1%      | Instruction decode + dispatch |
| ARM instruction exec      | 4.9%      | Instruction decode + dispatch |
| ARM data processing       | 3.9%      | ALU operations (inlined)      |
| Bus read32                | 2.8%      | 32-bit memory/IO dispatch     |
| Bus read16                | 1.5%      | 16-bit memory/IO dispatch     |
| libc memset/memmove       | 1.1%      | Array zeroing in PPU          |
| ARM load/store            | 0.8%      | Memory transfer instructions  |
| Bus writes                | 0.6%      | Memory/IO writes              |

### Key Insight

The PPU `render_composited()` function and its callees (`collect_text_bg`, `collect_affine_bg`, `collect_sprites`, blending, `write_line_buffer`) dominate execution at **~77%** of total time. CPU instruction execution is only ~20%. Optimizing the PPU pixel pipeline is the highest leverage target.

## Optimization Targets

In priority order.

### 1. PPU render_composited pixel loop (77% of time)

**Current cost:** ~240 pixels x 160 lines = 38,400 iterations per frame, each doing: window mask check, up to 4 BG priority insertions, OBJ insertion, blend mode dispatch, RGB555 to RGB888 conversion.

**Opportunities:**

- Skip disabled layers entirely (check DISPCNT bits once per scanline)
- Fast path for no-blend mode (most common): skip blend dispatch
- Fast path for single-layer visible: write directly without priority sort
- Batch RGB555 to RGB888 conversion using lookup table or SIMD-style ops
- Reduce PixelEntry struct size for better cache utilization

### 2. Background tile collection (inside render_composited)

**Current cost per text BG pixel:** 3-4 VRAM reads (tilemap + tile data + palette). For 4 text BGs active: ~600K VRAM reads per frame.

**Opportunities:**

- Tile-stride rendering: decode 8 pixels per tile lookup instead of 1
- Cache decoded tile rows (tiles repeat across scanlines)
- Precompute tilemap offsets per scanline (avoid per-pixel scroll math)

### 3. Sprite collection optimization

**Current:** iterates all 128 OAM entries per scanline (128 x 160 = 20,480 iterations per frame) even when most sprites are offscreen.

**Opportunities:**

- Build per-scanline sprite list once per frame (Y-range bucketing)
- Early-out when max sprites per scanline reached (GBA limit: 128 OAM entries but only ~10-20 visible per line typically)

### 4. Array zeroing (1.1%, libc memset)

**Current:** `fill()` calls on pixel/has_pixel arrays each scanline.

**Opportunities:**

- Use dirty-flag tracking instead of clearing entire arrays
- Or use generation counters to distinguish stale entries

### 5. Bus dispatch optimization (4.9% combined)

**Current:** 16-way match on `addr >> 24` for every read/write.

**Opportunities:**

- Function pointer table indexed by `addr >> 24` (single indirect call instead of match chain)
- Specialize common paths (ROM reads, EWRAM reads)

## Applied Optimizations

### Tile-stride text BG rendering (+15% Mario Kart, +2% Zelda)

Rewrote `collect_text_bg()` to iterate by 8-pixel tile columns instead of per-pixel. One tilemap lookup per tile, then 8 pixel emissions per tile. Mosaic falls back to per-pixel path. This is the largest single win.

### Active BG iteration in compositing loop

Compositing inner loop only iterates enabled BG indices instead of all 4. Saves 2 branch checks per pixel per disabled BG (480 per scanline in Mode 1).

### Deferred sprite attr2 read

Reads OAM attr0 first for disabled/mode checks, attr1 for Y-range check, and only reads attr2 after confirming the sprite is on the current scanline.

## Results

Apple M1, native release build:

| ROM                      | Baseline | Current  | Change |
| ------------------------ | -------- | -------- | ------ |
| Mario Kart Super Circuit | 47.9 MHz | 55.2 MHz | +15.2% |
| Zelda Minish Cap         | 59.8 MHz | 60.8 MHz | +1.7%  |

## Lessons Learned

- `Vec<u8>` to `Box<[u8; N]>`: caused ~40% regression
- `#[inline(always)]` on large bus dispatchers: hurts (icache pressure)
- `try_into().unwrap()` slice reads: add panic branches
- Persistent struct buffers vs stack arrays: stack wins
- Condition lookup table vs match: match wins (branch predictor)
- Debug-gating with `#[cfg(feature)]`: mixed results per game
- Window vertical precompute: compiler already hoists; manual version hurts
- Fast-path no-blend compositing: scan for semi-transparent OBJs negates gain
- **Key rule: benchmark every change, never assume improvement**

---

**Document Classification**: Public Technical Documentation
