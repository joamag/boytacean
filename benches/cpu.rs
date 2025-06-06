use boytacean::test::{build_test, TestOptions};
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_cpu_clock(c: &mut Criterion) {
    let mut gb = build_test(TestOptions {
        ppu_enabled: Some(false),
        apu_enabled: Some(false),
        dma_enabled: Some(false),
        timer_enabled: Some(false),
        ..Default::default()
    });
    gb.load_rom_empty().unwrap();

    c.bench_function("cpu_cycles", |b| {
        b.iter(|| {
            gb.clocks_cycles(1_000_000);
        })
    });
}

criterion_group!(benches, benchmark_cpu_clock);
criterion_main!(benches);
