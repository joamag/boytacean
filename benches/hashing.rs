use boytacean_common::bench::generate_data;
use boytacean_hashing::{crc32::crc32, crc32c::crc32c};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

fn benchmark_hashing(c: &mut Criterion) {
    let data = generate_data(10_000_000_usize);

    let mut group = c.benchmark_group("encoding");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("crc32", |b| {
        b.iter(|| {
            let encoded = crc32(black_box(&data));
            black_box(encoded);
        })
    });

    group.bench_function("crc32c", |b| {
        b.iter(|| {
            let encoded = crc32c(black_box(&data));
            black_box(encoded);
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_hashing);
criterion_main!(benches);
