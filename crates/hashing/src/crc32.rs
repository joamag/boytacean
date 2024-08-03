//! CRC32 implementation according to the CRC-32/ISO-HDLC specification.
//!
//! The CRC32 algorithm is a widely used checksum algorithm that is used in many
//! network protocols and file formats. The algorithm is based on a polynomial
//! division of the input data and a predefined polynomial value.
//! 
//! This implementation is optimized for modern CPUs by using hardware acceleration
//! when available.

#[cfg(target_arch = "x86_64")]
use std::arch::{is_x86_feature_detected, x86_64::_mm_crc32_u8};

#[cfg(target_arch = "aarch64")]
use std::arch::{aarch64::__crc32b, is_aarch64_feature_detected};

pub struct Crc32 {
    table: [u32; 256],
    value: u32,
}

impl Crc32 {
    pub fn new() -> Self {
        let mut crc32 = Crc32 {
            table: [0; 256],
            value: 0xffffffff,
        };
        crc32.init_table();
        crc32
    }

    fn init_table(&mut self) {
        const POLYNOMIAL: u32 = 0xedb88320;

        for i in 0..256 {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = POLYNOMIAL ^ (crc >> 1);
                } else {
                    crc >>= 1;
                }
            }
            self.table[i] = crc;
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse4.2")]
    unsafe fn update_hw_x86(&mut self, bytes: &[u8]) {
        let mut value = self.value;
        for &byte in bytes {
            value = _mm_crc32_u8(value, byte);
        }
        self.value = value;
    }

    #[cfg(target_arch = "aarch64")]
    #[target_feature(enable = "crc")]
    unsafe fn update_hw_aarch64(&mut self, bytes: &[u8]) {
        let mut value = self.value;
        for &byte in bytes {
            value = __crc32b(value, byte);
        }
        self.value = value;
    }

    pub fn update(&mut self, bytes: &[u8]) {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("sse4.2") {
                unsafe {
                    self.update_hw_x86(bytes);
                }
                return;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if is_aarch64_feature_detected!("crc") {
                unsafe {
                    self.update_hw_aarch64(bytes);
                }
                return;
            }
        }

        let mut value = self.value;
        for &byte in bytes {
            let index = (value ^ byte as u32) & 0xFF;
            value = self.table[index as usize] ^ (value >> 8);
        }
        self.value = value;
    }

    pub fn finalize(self) -> u32 {
        self.value ^ 0xffffffff
    }
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc32 = Crc32::new();
    crc32.update(data);
    crc32.finalize()
}

#[cfg(test)]
mod tests {
    use super::crc32;

    #[test]
    fn test_crc32_empty() {
        let data: [u8; 0] = [];
        assert_eq!(crc32(&data), 0x00);
    }

    #[test]
    fn test_crc32_single_byte() {
        let data: [u8; 1] = [0xab];
        assert_eq!(crc32(&data), 0x930695Ed);
    }

    #[test]
    fn test_crc32_multiple_bytes() {
        let data: [u8; 5] = [0x12, 0x34, 0x56, 0x78, 0x9a];
        assert_eq!(crc32(&data), 0x3c4687af);
    }

    #[test]
    fn test_crc32_large_data() {
        let data: Vec<u8> = vec![0xff; 1000];
        assert_eq!(crc32(&data), 0xe0533230);
    }
}
