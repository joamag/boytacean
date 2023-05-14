use crate::warnln;

pub enum DmaMode {
    General = 0x00,
    HBlank = 0x01,
}

pub struct Dma {
    source: u16,
    destination: u16,
    length: u8,
    mode: DmaMode,
    finished: bool,
}

impl Dma {
    pub fn new() -> Self {
        Self {
            source: 0x0,
            destination: 0x0,
            length: 0x0,
            mode: DmaMode::General,
            finished: false,
        }
    }

    pub fn reset(&mut self) {
        self.source = 0x0;
        self.destination = 0x0;
        self.length = 0x0;
        self.mode = DmaMode::General;
        self.finished = false;
    }

    pub fn clock(&mut self, _cycles: u8) {}

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // 0xFF55 — HDMA5: VRAM DMA length/mode/start (CGB only)
            0xff45 => self.length | ((self.finished as u8) << 7),
            _ => {
                warnln!("Reading from unknown DMA location 0x{:04x}", addr);
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF51 — HDMA1: VRAM DMA source high (CGB only)
            0xff41 => self.source = (self.source & 0x00ff) | ((value as u16) << 8),
            // 0xFF52 — HDMA2: VRAM DMA source low (CGB only)
            0xff42 => self.source = (self.source & 0xff00) | (value as u16),
            // 0xFF53 — HDMA3: VRAM DMA destination high (CGB only)
            0xff43 => self.destination = (self.destination & 0x00ff) | ((value as u16) << 8),
            // 0xFF54 — HDMA4: VRAM DMA destination low (CGB only)
            0xff44 => self.destination = (self.destination & 0xff00) | (value as u16),
            // 0xFF55 — HDMA5: VRAM DMA length/mode/start (CGB only)
            0xff45 => {
                self.length = value & 0x7f;
                self.mode = match (value & 80) >> 7 {
                    0 => DmaMode::General,
                    1 => DmaMode::HBlank,
                    _ => DmaMode::General,
                };

                // @TODO: Implement DMA transfer in a better way
                //let data = self.mmu.read_many(self.source, self.length as usize);
                //self.mmu.write_many(self.destination, &data);
            }
            _ => warnln!("Writing to unknown DMA location 0x{:04x}", addr),
        }
    }
}

impl Default for Dma {
    fn default() -> Self {
        Self::new()
    }
}
