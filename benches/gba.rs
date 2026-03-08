use boytacean::gba::GameBoyAdvance;
use criterion::{criterion_group, criterion_main, Criterion};

fn make_gba_ppu() -> GameBoyAdvance {
    let rom = include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba");
    let mut gba = GameBoyAdvance::new();
    gba.load_rom(rom).expect("Failed to load ppu_stripes.gba");
    for _ in 0..10 {
        gba.next_frame();
    }
    gba
}

fn make_gba_arm() -> GameBoyAdvance {
    let rom = include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/arm.gba");
    let mut gba = GameBoyAdvance::new();
    gba.load_rom(rom).expect("Failed to load arm.gba");
    for _ in 0..10 {
        gba.next_frame();
    }
    gba
}

fn make_gba_cpu_only() -> GameBoyAdvance {
    let rom = include_bytes!("../res/roms.gba/test/jsmolka_gba-tests/arm.gba");
    let mut gba = GameBoyAdvance::new();
    gba.load_rom(rom).expect("Failed to load arm.gba");
    gba.set_ppu_enabled(false);
    gba.set_apu_enabled(false);
    gba.set_dma_enabled(false);
    gba.set_timer_enabled(false);
    for _ in 0..10 {
        gba.next_frame();
    }
    gba
}

fn benchmark_gba_full_frame(c: &mut Criterion) {
    let mut gba = make_gba_ppu();
    c.bench_function("gba_full_frame", |b| {
        b.iter(|| {
            gba.next_frame();
        })
    });
}

fn benchmark_gba_arm_frame(c: &mut Criterion) {
    let mut gba = make_gba_arm();
    c.bench_function("gba_arm_frame", |b| {
        b.iter(|| {
            gba.next_frame();
        })
    });
}

fn benchmark_gba_cpu_only(c: &mut Criterion) {
    let mut gba = make_gba_cpu_only();
    c.bench_function("gba_cpu_only_1m_cycles", |b| {
        b.iter(|| {
            gba.clocks_cycles(1_000_000);
        })
    });
}

criterion_group!(
    benches,
    benchmark_gba_full_frame,
    benchmark_gba_arm_frame,
    benchmark_gba_cpu_only
);
criterion_main!(benches);
