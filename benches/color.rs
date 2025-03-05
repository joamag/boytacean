use boytacean::color::rgb888_to_rgb1555_scalar;
use boytacean_common::bench::multiply_array_size;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_rgb_conversion(c: &mut Criterion) {
    let rgb888_pixels: Vec<u8> = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];
    let rgb888_pixels_sized = multiply_array_size(&rgb888_pixels, 16 * 1024);

    let mut rgb1555_pixels: Vec<u8> = vec![0; rgb888_pixels_sized.len() / 3 * 2];

    c.bench_function("rgb888_to_rgb1555_scalar", |b| {
        b.iter(|| {
            rgb888_to_rgb1555_scalar(
                black_box(&rgb888_pixels_sized),
                black_box(&mut rgb1555_pixels),
            )
        })
    });

    #[cfg(feature = "simd")]
    {
        use boytacean::color::rgb888_to_rgb1555_simd;
        c.bench_function("rgb888_to_rgb1555_simd", |b| {
            b.iter(|| {
                rgb888_to_rgb1555_simd(
                    black_box(&rgb888_pixels_sized),
                    black_box(&mut rgb1555_pixels),
                )
            })
        });
    }
}

criterion_group!(benches, benchmark_rgb_conversion);
criterion_main!(benches);
