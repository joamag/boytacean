//! Benchmark related functions to be shared and used.
//!
//! Most of the these function are used to generate data for benchmarking
//! and used in Criterion benchmarks.

pub fn generate_data(size: usize) -> Vec<u8> {
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

pub fn multiply_array_size<T: Clone>(arr: &[T], multiplier: usize) -> Vec<T> {
    let mut new_arr = Vec::with_capacity(arr.len() * multiplier);
    for _ in 0..multiplier {
        new_arr.extend_from_slice(arr);
    }
    new_arr
}
