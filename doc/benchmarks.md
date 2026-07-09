# Benchmarks

Timestamped benchmark reports comparing Boytacean against other Game Boy emulators. Each section is a self-contained snapshot with the exact environment, versions and raw per-round data, so that results remain reproducible and comparable over time. The methodology is described in [performance.md](performance.md).

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
