use boytacean_common::bench::generate_data;
use boytacean_encoding::{
    huffman::{decode_huffman, encode_huffman},
    rle::{decode_rle, encode_rle},
    zippy::{decode_zippy, encode_zippy},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn benchmark_encoding(c: &mut Criterion) {
    let data = generate_data(10_000_000_usize);

    let mut group = c.benchmark_group("encoding");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("encode_huffman", |b| {
        b.iter(|| {
            let encoded = encode_huffman(black_box(&data)).unwrap();
            black_box(encoded);
        })
    });

    group.bench_function("encode_rle", |b| {
        b.iter(|| {
            let encoded = encode_rle(black_box(&data));
            black_box(encoded);
        })
    });

    group.bench_function("encode_zippy", |b| {
        b.iter(|| {
            let encoded = encode_zippy(black_box(&data), None).unwrap();
            black_box(encoded);
        })
    });

    group.finish();
}

fn benchmark_decoding(c: &mut Criterion) {
    let data = generate_data(10_000_000_usize);
    let encoded_huffman = encode_huffman(black_box(&data)).unwrap();
    let encoded_rle = encode_rle(black_box(&data));
    let encoded_zippy = encode_zippy(black_box(&data), None).unwrap();

    let mut group = c.benchmark_group("decoding");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("decode_huffman", |b| {
        b.iter(|| {
            let decoded = decode_huffman(black_box(&encoded_huffman)).unwrap();
            black_box(decoded);
        })
    });

    group.bench_function("decode_rle", |b| {
        b.iter(|| {
            let decoded = decode_rle(black_box(&encoded_rle));
            black_box(decoded);
        })
    });

    group.bench_function("decode_zippy", |b| {
        b.iter(|| {
            let decoded = decode_zippy(black_box(&encoded_zippy), None).unwrap();
            black_box(decoded);
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_encoding, benchmark_decoding);
criterion_main!(benches);
