//! Benchmark related functions to be shared and used.
//!
//! Most of the these functions are used to generate data for benchmarking
//! and used in Criterion benchmarks.

/// Generate data with repeating patterns for benchmarking.
///
/// The generated data consists of repeating sequences of specific byte patterns
/// to simulate realistic data for compression benchmarks.
///
/// # Arguments
/// * `size` - The total size of the generated data in bytes.
///
/// # Returns
/// A `Vec<u8>` containing the generated data.
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

/// Multiply the size of an array by a given multiplier.
///
/// Returns a new array with repeated elements.
///
/// # Arguments
/// * `arr` - The input array to be multiplied.
/// * `multiplier` - The factor by which to multiply the size of the array.
///
/// # Returns
/// A new `Vec<T>` containing the elements of the original array repeated
/// according to the specified multiplier.
pub fn multiply_array_size<T: Clone>(arr: &[T], multiplier: usize) -> Vec<T> {
    let mut new_arr = Vec::with_capacity(arr.len() * multiplier);
    for _ in 0..multiplier {
        new_arr.extend_from_slice(arr);
    }
    new_arr
}
