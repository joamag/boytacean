//! DMA (Direct Memory Access) functions and structures.

use std::fmt::{self, Display, Formatter};

use crate::{
    consts::{DMA_ADDR, HDMA1_ADDR, HDMA2_ADDR, HDMA3_ADDR, HDMA4_ADDR, HDMA5_ADDR},
    mmu::BusComponent,
    panic_gb, warnln,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DmaMode {
    General = 0x00,
    HBlank = 0x01,
}

impl DmaMode {
    pub fn description(&self) -> &'static str {
        match self {
            DmaMode::General => "General-Purpose DMA",
            DmaMode::HBlank => "HBlank DMA",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0x00 => DmaMode::General,
            0x01 => DmaMode::HBlank,
            _ => DmaMode::General,
        }
    }
}

impl Display for DmaMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u8> for DmaMode {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

pub struct Dma {
    source: u16,
    destination: u16,
    length: u16,
    pending: u16,
    mode: DmaMode,
    value_dma: u8,
    cycles_dma: u16,
    active_dma: bool,
    active_hdma: bool,
}

impl Dma {
    pub fn new() -> Self {
        Self {
            source: 0x0,
            destination: 0x0,
            length: 0x0,
            pending: 0x0,
            mode: DmaMode::General,
            value_dma: 0x0,
            cycles_dma: 0x0,
            active_dma: false,
            active_hdma: false,
        }
    }

    pub fn reset(&mut self) {
        self.source = 0x0;
        self.destination = 0x0;
        self.length = 0x0;
        self.pending = 0x0;
        self.mode = DmaMode::General;
        self.value_dma = 0x0;
        self.cycles_dma = 0x0;
        self.active_dma = false;
        self.active_hdma = false;
    }

    pub fn clock(&mut self, _cycles: u16) {}

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            // 0xFF46 — DMA: OAM DMA source address & start
            DMA_ADDR => self.value_dma,
            // 0xFF55 — HDMA5: VRAM DMA length/mode/start (CGB only)
            HDMA5_ADDR => {
                ((self.pending >> 4) as u8).wrapping_sub(1) | ((!self.active_hdma as u8) << 7)
            }
            _ => {
                warnln!("Reading from unknown DMA location 0x{:04x}", addr);
                #[allow(unreachable_code)]
                0xff
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // 0xFF46 — DMA: OAM DMA source address & start
            DMA_ADDR => {
                self.value_dma = value;
                self.cycles_dma = 640;
                self.active_dma = true;
            }
            // 0xFF51 — HDMA1: VRAM DMA source high (CGB only)
            HDMA1_ADDR => self.source = (self.source & 0x00ff) | ((value as u16) << 8),
            // 0xFF52 — HDMA2: VRAM DMA source low (CGB only)
            HDMA2_ADDR => self.source = (self.source & 0xff00) | ((value & 0xf0) as u16),
            // 0xFF53 — HDMA3: VRAM DMA destination high (CGB only)
            HDMA3_ADDR => self.destination = (self.destination & 0x00ff) | ((value as u16) << 8),
            // 0xFF54 — HDMA4: VRAM DMA destination low (CGB only)
            HDMA4_ADDR => self.destination = (self.destination & 0xff00) | ((value & 0xf0) as u16),
            // 0xFF55 — HDMA5: VRAM DMA length/mode/start (CGB only)
            HDMA5_ADDR => {
                // in case there's an active HDMA transfer and the
                // bit 7 is set to 0, the transfer is stopped
                if value & 0x80 == 0x00 && self.active_hdma && self.mode == DmaMode::HBlank {
                    self.pending = 0;
                    self.active_hdma = false;
                } else {
                    // ensures destination is set within VRAM range
                    // required for compatibility with some games (know bug)
                    self.destination = 0x8000 | (self.destination & 0x1fff);
                    self.length = (((value & 0x7f) + 0x1) as u16) << 4;
                    self.mode = ((value & 80) >> 7).into();
                    self.pending = self.length;
                    self.active_hdma = true;

                    // @TODO: implement HBlank DMA using the proper timing
                    // and during the HBlank period as described in the
                    // https://gbdev.io/pandocs/CGB_Registers.html#lcd-vram-dma-transfers
                    if self.mode == DmaMode::HBlank {
                        panic_gb!("HBlank DMA not implemented");
                    }
                }
            }
            _ => warnln!("Writing to unknown DMA location 0x{:04x}", addr),
        }
    }

    pub fn source(&self) -> u16 {
        self.source
    }

    pub fn set_source(&mut self, value: u16) {
        self.source = value;
    }

    pub fn destination(&self) -> u16 {
        self.destination
    }

    pub fn set_destination(&mut self, value: u16) {
        self.destination = value;
    }

    pub fn length(&self) -> u16 {
        self.length
    }

    pub fn set_length(&mut self, value: u16) {
        self.length = value;
    }

    pub fn pending(&self) -> u16 {
        self.pending
    }

    pub fn set_pending(&mut self, value: u16) {
        self.pending = value;
    }

    pub fn mode(&self) -> DmaMode {
        self.mode
    }

    pub fn set_mode(&mut self, value: DmaMode) {
        self.mode = value;
    }

    pub fn value_dma(&self) -> u8 {
        self.value_dma
    }

    pub fn set_value_dma(&mut self, value: u8) {
        self.value_dma = value;
    }

    pub fn cycles_dma(&self) -> u16 {
        self.cycles_dma
    }

    pub fn set_cycles_dma(&mut self, value: u16) {
        self.cycles_dma = value;
    }

    pub fn active_dma(&self) -> bool {
        self.active_dma
    }

    pub fn set_active_dma(&mut self, value: bool) {
        self.active_dma = value;
    }

    pub fn active_hdma(&self) -> bool {
        self.active_hdma
    }

    pub fn set_active_hdma(&mut self, value: bool) {
        self.active_hdma = value;
    }

    pub fn active(&self) -> bool {
        self.active_dma || self.active_hdma
    }

    pub fn description(&self) -> String {
        format!(
            "DMA: {}\nHDMA: {}",
            self.description_dma(),
            self.description_hdma()
        )
    }

    pub fn description_dma(&self) -> String {
        format!(
            "active: {}, cycles: {}, value: 0x{:02x}",
            self.active_dma, self.cycles_dma, self.value_dma
        )
    }

    pub fn description_hdma(&self) -> String {
        format!(
            "active: {}, length: 0x{:04x}, mode: {}, source: 0x{:04x}, destination: 0x{:04x}",
            self.active_hdma, self.length, self.mode, self.source, self.destination
        )
    }
}

impl BusComponent for Dma {
    fn read(&self, addr: u16) -> u8 {
        self.read(addr)
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.write(addr, value);
    }
}

impl Default for Dma {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Dma {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::{Dma, DmaMode};

    #[test]
    fn test_dma_default() {
        let dma = Dma::default();
        assert!(!dma.active_dma);
        assert!(!dma.active_hdma);
        assert!(!dma.active());
    }

    #[test]
    fn test_dma_reset() {
        let mut dma = Dma::new();
        dma.source = 0x1234;
        dma.destination = 0x5678;
        dma.length = 0x9abc;
        dma.pending = 0x9abc;
        dma.mode = DmaMode::HBlank;
        dma.value_dma = 0xff;
        dma.cycles_dma = 0x0012;
        dma.active_dma = true;
        dma.active_hdma = true;

        dma.reset();

        assert_eq!(dma.source, 0x0);
        assert_eq!(dma.destination, 0x0);
        assert_eq!(dma.length, 0x0);
        assert_eq!(dma.pending, 0x0);
        assert_eq!(dma.mode, DmaMode::General);
        assert_eq!(dma.value_dma, 0x0);
        assert_eq!(dma.cycles_dma, 0x0);
        assert!(!dma.active_dma);
        assert!(!dma.active_hdma);
    }

    #[test]
    fn test_dma_set_active() {
        let mut dma = Dma::new();
        dma.set_active_dma(true);
        assert!(dma.active_dma);
        assert!(dma.active());
    }
}
