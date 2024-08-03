//! CRC32 implementation according to the CRC-32/ISO-HDLC specification.

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
        for i in 0..256 {
            let mut crc = i as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = 0xedb88320 ^ (crc >> 1);
                } else {
                    crc >>= 1;
                }
            }
            self.table[i] = crc;
        }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            let index = (self.value ^ byte as u32) & 0xFF;
            self.value = self.table[index as usize] ^ (self.value >> 8);
        }
    }

    pub fn finalize(self) -> u32 {
        self.value ^ 0xFFFFFFFF
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
