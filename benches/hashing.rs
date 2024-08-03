use boytacean_hashing::crc32::crc32;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn generate_data(size: usize) -> Vec<u8> {
    let patterns: [&[u8]; 6] = [
        b"aaaaa",
        b"bbbbbbbbb",
        b"ccccc",
        b"dddd",
        b"eeeeeeeeee",
        b"ffff",
    ];

    let mut data = Vec::with_capacity(size);
    let mut pattern_index = 0;

    while data.len() < size {
        let pattern = patterns[pattern_index];
        pattern_index = (pattern_index + 1) % patterns.len();
        for _ in 0..3 {
            // Repeat each pattern 3 times
            data.extend_from_slice(pattern);
            if data.len() >= size {
                break;
            }
        }
    }

    data.truncate(size);
    data
}

fn benchmark_hashing(c: &mut Criterion) {
    let data_size = 10_000_000_usize;
    let data = generate_data(data_size);

    let mut group = c.benchmark_group("encoding");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("crc32", |b| {
        b.iter(|| {
            let encoded = crc32(black_box(&data));
            black_box(encoded);
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_hashing);
criterion_main!(benches);
