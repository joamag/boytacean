pub struct Dma {}

impl Dma {
    pub fn new() -> Self {
        Self {}
    }

    pub fn reset(&mut self) {}

    pub fn clock(&mut self, _cycles: u8) {}
}

impl Default for Dma {
    fn default() -> Self {
        Self::new()
    }
}
