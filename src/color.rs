//! Color manipulation functions and constants.

use crate::util::copy_fast;

pub const RGB_SIZE: usize = 3;
pub const RGBA_SIZE: usize = 4;
pub const RGB888_SIZE: usize = 3;
pub const XRGB8888_SIZE: usize = 4;
pub const RGB1555_SIZE: usize = 2;
pub const RGB565_SIZE: usize = 2;

/// Defines the Game Boy pixel type as a buffer
/// with the size of RGB (3 bytes).
pub type Pixel = [u8; RGB_SIZE];

/// Defines a transparent Game Boy pixel type as a buffer
/// with the size of RGBA (4 bytes).
pub type PixelAlpha = [u8; RGBA_SIZE];

/// Defines a pixel with 5 bits per channel plus a padding
/// bit at the beginning.
pub type PixelRgb1555 = [u8; RGB1555_SIZE];

/// Defines a pixel with 5 bits per channel except for the
/// green channel which uses 6 bits.
pub type PixelRgb565 = [u8; RGB565_SIZE];

pub fn rgb555_to_rgb888(first: u8, second: u8) -> Pixel {
    let r = (first & 0x1f) << 3;
    let g = (((first & 0xe0) >> 5) | ((second & 0x03) << 3)) << 3;
    let b = ((second & 0x7c) >> 2) << 3;
    [r, g, b]
}

pub fn rgb888_to_rgb1555(first: u8, second: u8, third: u8) -> PixelRgb1555 {
    let pixel = rgb888_to_rgb1555_u16(first, second, third);
    [pixel as u8, (pixel >> 8) as u8]
}

pub fn rgb888_to_rgb1555_u16(first: u8, second: u8, third: u8) -> u16 {
    let r = (first as u16 >> 3) & 0x1f;
    let g = (second as u16 >> 3) & 0x1f;
    let b = (third as u16 >> 3) & 0x1f;
    let a = 1;
    (a << 15) | (r << 10) | (g << 5) | b
}

pub fn rgb888_to_rgb565(first: u8, second: u8, third: u8) -> PixelRgb565 {
    let pixel = rgb888_to_rgb565_u16(first, second, third);
    [pixel as u8, (pixel >> 8) as u8]
}

pub fn rgb888_to_rgb565_u16(first: u8, second: u8, third: u8) -> u16 {
    let r = (first as u16 >> 3) & 0x1f;
    let g = (second as u16 >> 2) & 0x3f;
    let b = (third as u16 >> 3) & 0x1f;
    (r << 11) | (g << 5) | b
}

pub fn rgb888_to_rgb1555_array(rgb888_pixels: &[u8], rgb1555_pixels: &mut [u8]) {
    #[cfg(feature = "simd")]
    {
        rgb888_to_rgb1555_simd(rgb888_pixels, rgb1555_pixels);
    }
    #[cfg(not(feature = "simd"))]
    {
        rgb888_to_rgb1555_scalar(rgb888_pixels, rgb1555_pixels);
    }
}

/// Converts an array of RGB888 pixels to RGB565 format using a scalar implementation.
///
/// This method should provide the same results as the SIMD implementation.
pub fn rgb888_to_rgb1555_scalar(rgb888_pixels: &[u8], rgb1555_pixels: &mut [u8]) {
    assert!(
        rgb888_pixels.len() % 3 == 0,
        "Length of rgb888_pixels must be a multiple of 3"
    );
    assert!(
        rgb1555_pixels.len() % 2 == 0,
        "Length of rgb1555_pixels must be a multiple of 2"
    );
    assert!(
        rgb888_pixels.len() / 3 == rgb1555_pixels.len() / 2,
        "Length of rgb1555_pixels must be two thirds the length of rgb888_pixels"
    );
    for index in 0..rgb888_pixels.len() / RGB_SIZE {
        let (r, g, b) = (
            rgb888_pixels[index * RGB_SIZE],
            rgb888_pixels[index * RGB_SIZE + 1],
            rgb888_pixels[index * RGB_SIZE + 2],
        );
        let rgb1555 = rgb888_to_rgb1555(r, g, b);
        let output_offset = index * RGB1555_SIZE;
        copy_fast(
            &rgb1555,
            &mut rgb1555_pixels[output_offset..output_offset + RGB1555_SIZE],
            RGB1555_SIZE,
        )
    }
}

/// Converts an array of RGB888 pixels to RGB1555 format using SIMD.
///
/// This method is only available when the `simd` feature is enabled.
///
/// Note: The length of `rgb888_pixels` must be a multiple of 3, and
/// `rgb1555_pixels` must be a multiple of 2.
#[cfg(feature = "simd")]
pub fn rgb888_to_rgb1555_simd(rgb888_pixels: &[u8], rgb1555_pixels: &mut [u8]) {
    use std::simd::u8x16;

    use crate::util::interleave_arrays;

    const SIMD_WIDTH: usize = 16;

    assert!(
        rgb888_pixels.len() % 3 == 0,
        "Length of rgb888_pixels must be a multiple of 3"
    );
    assert!(
        rgb1555_pixels.len() % 2 == 0,
        "Length of rgb1555_pixels must be a multiple of 2"
    );
    assert!(
        rgb888_pixels.len() / 3 == rgb1555_pixels.len() / 2,
        "Length of rgb1555_pixels must be two thirds the length of rgb888_pixels"
    );

    let num_pixels = rgb888_pixels.len() / 3;
    let simd_chunks = num_pixels / SIMD_WIDTH;

    for index in 0..simd_chunks {
        let offset = index * SIMD_WIDTH * 3;
        let r = u8x16::from_slice(&[
            rgb888_pixels[offset],
            rgb888_pixels[offset + 3],
            rgb888_pixels[offset + 6],
            rgb888_pixels[offset + 9],
            rgb888_pixels[offset + 12],
            rgb888_pixels[offset + 15],
            rgb888_pixels[offset + 18],
            rgb888_pixels[offset + 21],
            rgb888_pixels[offset + 24],
            rgb888_pixels[offset + 27],
            rgb888_pixels[offset + 30],
            rgb888_pixels[offset + 33],
            rgb888_pixels[offset + 36],
            rgb888_pixels[offset + 39],
            rgb888_pixels[offset + 42],
            rgb888_pixels[offset + 45],
        ]);
        let g = u8x16::from_slice(&[
            rgb888_pixels[offset + 1],
            rgb888_pixels[offset + 4],
            rgb888_pixels[offset + 7],
            rgb888_pixels[offset + 10],
            rgb888_pixels[offset + 13],
            rgb888_pixels[offset + 16],
            rgb888_pixels[offset + 19],
            rgb888_pixels[offset + 22],
            rgb888_pixels[offset + 25],
            rgb888_pixels[offset + 28],
            rgb888_pixels[offset + 31],
            rgb888_pixels[offset + 34],
            rgb888_pixels[offset + 37],
            rgb888_pixels[offset + 40],
            rgb888_pixels[offset + 43],
            rgb888_pixels[offset + 46],
        ]);
        let b = u8x16::from_slice(&[
            rgb888_pixels[offset + 2],
            rgb888_pixels[offset + 5],
            rgb888_pixels[offset + 8],
            rgb888_pixels[offset + 11],
            rgb888_pixels[offset + 14],
            rgb888_pixels[offset + 17],
            rgb888_pixels[offset + 20],
            rgb888_pixels[offset + 23],
            rgb888_pixels[offset + 26],
            rgb888_pixels[offset + 29],
            rgb888_pixels[offset + 32],
            rgb888_pixels[offset + 35],
            rgb888_pixels[offset + 38],
            rgb888_pixels[offset + 41],
            rgb888_pixels[offset + 44],
            rgb888_pixels[offset + 47],
        ]);

        let r_shifted_high = (r >> 1) & u8x16::splat(0x7c);
        let g_shifted_high = (g >> 6) & u8x16::splat(0x03);
        let g_shifted_low = (g << 2) & u8x16::splat(0xe0);
        let b_shifted = (b >> 3) & u8x16::splat(0x3f);

        let high_byte = u8x16::splat(0x80) | r_shifted_high | g_shifted_high;
        let low_byte = g_shifted_low | b_shifted;

        let output_offset = index * SIMD_WIDTH * RGB1555_SIZE;
        interleave_arrays(
            low_byte.as_array(),
            high_byte.as_array(),
            &mut rgb1555_pixels[output_offset..output_offset + SIMD_WIDTH * RGB1555_SIZE],
        );
    }

    let remainder = num_pixels % SIMD_WIDTH;
    let offset = simd_chunks * SIMD_WIDTH * RGB_SIZE;
    let offset_rgb1555 = simd_chunks * SIMD_WIDTH * RGB1555_SIZE;

    for index in 0..remainder {
        let current_offset = offset + index * RGB_SIZE;
        let (r, g, b) = (
            rgb888_pixels[current_offset],
            rgb888_pixels[current_offset + 1],
            rgb888_pixels[current_offset + 2],
        );
        let rgb1555 = rgb888_to_rgb1555(r, g, b);
        let output_offset = offset_rgb1555 + index * RGB1555_SIZE;
        copy_fast(
            &rgb1555,
            &mut rgb1555_pixels[output_offset..output_offset + RGB1555_SIZE],
            RGB1555_SIZE,
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::zero_prefixed_literal)]

    use super::{rgb888_to_rgb1555, rgb888_to_rgb1555_scalar};

    #[test]
    fn test_rgb888_to_rgb1555() {
        let result = rgb888_to_rgb1555(255, 0, 0);
        assert_eq!(result, [0b00000000, 0b11111100]);

        let result = rgb888_to_rgb1555(0, 255, 0);
        assert_eq!(result, [0b11100000, 0b10000011]);

        let result = rgb888_to_rgb1555(0, 0, 255);
        assert_eq!(result, [0b00011111, 0b10000000]);

        let result = rgb888_to_rgb1555(255, 255, 0);
        assert_eq!(result, [0b11100000, 0b11111111]);
    }

    #[test]
    fn test_rgb888_to_rgb1555_scalar() {
        let rgb888_pixels: Vec<u8> = vec![
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            020, 020, 200, // Blueish
        ];
        let mut rgb1555_pixels: Vec<u8> = vec![0; 36];

        rgb888_to_rgb1555_scalar(&rgb888_pixels, &mut rgb1555_pixels);

        let expected_rgb1555: Vec<u8> = vec![
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b01011001, 0b10001000, // Blueish
        ];

        assert_eq!(rgb1555_pixels, expected_rgb1555);
    }

    #[test]
    #[cfg(feature = "simd")]
    fn test_rgb888_to_rgb1555_simd() {
        use super::rgb888_to_rgb1555_simd;

        let rgb888_pixels: Vec<u8> = vec![
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            000, 255, 000, // Green
            000, 000, 255, // Blue
            255, 255, 000, // Yellow
            255, 000, 000, // Red
            020, 020, 200, // Blueish
        ];
        let mut rgb1555_pixels: Vec<u8> = vec![0; 36];

        rgb888_to_rgb1555_simd(&rgb888_pixels, &mut rgb1555_pixels);

        let expected_rgb1555: Vec<u8> = vec![
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b11100000, 0b10000011, // Green
            0b00011111, 0b10000000, // Blue
            0b11100000, 0b11111111, // Yellow
            0b00000000, 0b11111100, // Red
            0b01011001, 0b10001000, // Blueish
        ];

        assert_eq!(rgb1555_pixels, expected_rgb1555);
    }
}
