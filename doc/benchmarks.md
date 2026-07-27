# Benchmarks

Timestamped benchmark reports comparing Boytacean against other Game Boy emulators. Each section is a self-contained snapshot with the exact environment, versions and raw per-round data, so that results remain reproducible and comparable over time. The methodology is described in [performance.md](performance.md).

## 2026-07-28 — 0.13.1 versus 0.12.0

Release to release comparison of the native frame throughput, covering the performance work of [#34](https://github.com/joamag/boytacean/issues/34) and the PPU fixes that landed in the same range.

### Environment

| Item      | Value                                                                                         |
| --------- | --------------------------------------------------------------------------------------------- |
| Hardware  | AMD Ryzen 5 5600X (6 cores, 12 threads), Windows 11                                           |
| Boytacean | tags `0.12.0` (`0042d14`) and `0.13.1` (`83fd9a5`), release profile (fat LTO, `opt-level = 3`) |
| Toolchain | rustc 1.97.0-nightly (`4b0c9d76a`)                                                            |

### Harnesses

* `bench_headless` example, 12000 frames per run, 600 warmup frames, DMG mode, APU disabled.
* Runs interleaved between the two versions, 3 rounds each, to bound scheduling and thermal drift.
* The harness differs slightly between the two tags (the newer one adds `--apu` and `--cgb` flags plus `profile` hooks that compile out), the measured loop is the same.

### Results (fps, per round)

| ROM        | 0.12.0             | 0.13.1             | Delta |
| ---------- | ------------------ | ------------------ | ----- |
| 20y.gb     | 8299 / 7544 / 7769 | 8065 / 8223 / 7837 | ~flat |
| gejmboj.gb | 6610 / 6847 / 6025 | 6720 / 6910 / 6932 | +4%   |

### Findings

* Frame throughput on the two ROMs that render continuously across the whole run is flat to slightly positive, within or just above the ~3% run-to-run noise of this machine.
* The performance work of [#34](https://github.com/joamag/boytacean/issues/34) was measured on Apple silicon in the sections below, this report does not reproduce those gains on this machine.

### Excluded ROMs

`pocket.gb` measures 6930 fps on `0.12.0` against 5363 fps on `0.13.1`, which reads as a 21% regression but is an artifact of the V-Blank freeze fixed in `bfd1cfb`. Hashing the frame buffer every 1000 frames shows the `0.12.0` screen going static from frame ~5000 onwards and staying identical for the remaining 8000 frames, while `0.13.1` keeps producing new frames. The CPU executes the same number of cycles per frame in both (70223), so the difference is in the render path that the frozen build never reaches. The comparison is not meaningful.

`dmg_acid2.gb` settles into a static test pattern on both versions and is therefore not representative of sustained rendering throughput, even though the behaviour is identical across them.

## 2026-07-10 — CGB rendering tuning

Environment identical to the 2026-07-09 section below, measuring the effect of the CGB rendering optimizations (borrowed background attributes map and per-tile row slice rendering in `render_map()`). Before is the branch state of the 2026-07-09 report, after includes the CGB rendering changes. Runs were interleaved, 3 rounds of 12000 frames each.

### Results (fps, per round)

| ROM           | Mode | Before                | After                 | mGBA (2026-07-09) |
| ------------- | ---- | --------------------- | --------------------- | ----------------- |
| cgb_acid2.gbc | CGB  | 10966 / 10771 / 10437 | 12512 / 12167 / 12408 | 22986             |

### Findings

* The CGB background attributes map (`[TileData; 1024]`, 5 KB) was copied by value on every scanline render pass; borrowing it instead yields +13-19% CGB frame throughput.
* Background map rendering dropped from ~45% to ~37% of the CGB frame time (internal `profile` instrumentation).
* Emulation output remains bit-exact (frame-buffer hashes match across cgb-acid2 and game ROMs in DMG and CGB modes).
* The remaining gap to mGBA on CGB (~1.8x) is dominated by the halt-tick loop, consistent with the event-scheduler future work item in [performance.md](performance.md).

## 2026-07-09 — DMG/CGB frame throughput vs mGBA and SameBoy

### Environment

| Item      | Value                                                                                                       |
| --------- | ----------------------------------------------------------------------------------------------------------- |
| Hardware  | Apple M5 Pro (18 cores), macOS 26.5.1                                                                       |
| Boytacean | `feat/core-perf-tuning` (`3f6c34a`, v0.12.1 base), rustc 1.95.0, release profile (fat LTO, `opt-level = 3`) |
| mGBA      | `master` (`5157ce2`, 2026-07-05), `mgba-perf` built with `-DBUILD_PERF=ON`, Release                         |
| SameBoy   | `master`, tester v1.0.3, `make tester CONF=release`                                                         |

### Harnesses

* **Boytacean**: `bench_headless` example, best of 3 runs of 12000 frames, 600 warmup frames, APU disabled by default (`--apu` column enables it).
* **mGBA**: `mgba-perf -F 12000`, 3 runs, audio always emulated (no audio sync).
* **SameBoy**: `sameboy_tester --dmg --length 200` (200 emulated seconds = 12000 frames) wall-clock timed, 3 runs, full accuracy mode, audio always emulated. The tester simulates button presses during the run, a minor workload difference versus the other harnesses.

Boytacean and mGBA runs were interleaved to bound scheduling/thermal drift (~±3% run-to-run on this machine).

### Results (fps, best of 3 runs)

| ROM             | Mode | Boytacean | Boytacean (APU) | mGBA  | SameBoy |
| --------------- | ---- | --------- | --------------- | ----- | ------- |
| pocket.gb       | DMG  | 16318     | 12068           | 8595  | 2510    |
| shocklobster.gb | DMG  | 17188     | 11628           | 21868 | 2532    |
| opus5.gb        | DMG  | 18492     | 14387           | 8435  | n/a     |
| cgb_acid2.gbc   | CGB  | 10439     | -               | 22986 | 4196    |

The SameBoy tester aborts early on opus5.gb (its stuck-game heuristic triggers on the demo's idle loop), so no valid measurement is possible with that harness. The cgb_acid2.gbc Boytacean/mGBA values are from a single measurement pass.

### Raw per-round data (fps)

| ROM             | Boytacean             | Boytacean (APU)       | mGBA                  | SameBoy            |
| --------------- | --------------------- | --------------------- | --------------------- | ------------------ |
| pocket.gb       | 16318 / 15378 / 15755 | 12068 / 11697 / 11695 | 8595 / 8352 / 8325    | 2505 / 2500 / 2510 |
| shocklobster.gb | 16802 / 16284 / 17188 | 11223 / 11216 / 11628 | 21499 / 21868 / 20981 | 2532 / 2505 / 2521 |
| opus5.gb        | 18227 / 18045 / 18492 | 14387 / 13950 / 14008 | 8384 / 8435 / 8347    | n/a                |
| cgb_acid2.gbc   | 10439                 | -                     | 22986                 | 4110 / 4196 / 4110 |

### Findings

* With audio enabled on both sides, Boytacean is ~1.4x (pocket.gb) to ~1.7x (opus5.gb) faster than mGBA on rendering-bound DMG workloads.
* mGBA is ~1.9x faster on shocklobster.gb (interrupt and HALT heavy action game) and ~2.2x faster on the CGB test ROM, consistent with its event-driven scheduler skipping idle emulation work that Boytacean ticks through 4 cycles at a time.
* SameBoy trades throughput for accuracy (per-T-cycle stepping) and runs at ~2500 fps (DMG) / ~4200 fps (CGB), roughly 5-8x slower than Boytacean and mGBA, while remaining far above realtime.
* All three emulators are comfortably above the ~60 fps realtime requirement on this hardware; the differences only matter for fast-forward, headless and AI-training style workloads.
