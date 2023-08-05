#[repr(packed)]
pub struct BeesState {
    pub name: BeesName,
    pub info: BeesInfo,
    pub core: BeesCore,
}

#[repr(packed)]
pub struct BeesBlockHeader {
    pub magic: u32,
    pub size: u32,
}

#[repr(packed)]
pub struct BeesBuffer {
    pub size: u32,
    pub offset: u32,
}

#[repr(packed)]
pub struct BeesFooter {
    pub start_offset: u32,
    pub magic: u32,
}

#[repr(packed)]
pub struct BeesName {
    pub header: BeesBlockHeader,
    pub name: String,
}

#[repr(packed)]
pub struct BeesInfo {
    pub header: BeesBlockHeader,
    pub title: [u8; 16],
    pub checksum: [u8; 2],
}

#[repr(packed)]
pub struct BeesCore {
    pub header: BeesBlockHeader,

    pub major: u16,
    pub minor: u16,

    pub model: u32,

    pub pc: u16,
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,

    pub ime: u8,
    pub ie: u8,
    // 0 = running; 1 = halted; 2 = stopped
    pub execution_mode: u8,
    _padding: u8,

    pub io_registers: [u8; 128],

    pub ram: BeesBuffer,
    pub vram: BeesBuffer,
    pub mbc_ram: BeesBuffer,
    pub oam: BeesBuffer,
    pub hram: BeesBuffer,
    pub background_palettes: BeesBuffer,
    pub object_palettes: BeesBuffer,
}

trait Serialize {
    fn store(&self, buffer: &mut Vec<u8>);
    fn load(&self, data: &[u8]);
}

impl Serialize for BeesState {
    fn store(&self, buffer: &mut Vec<u8>) {
        self.info.store(buffer);
    }

    fn load(&self, data: &[u8]) {
        todo!()
    }
}

impl Serialize for BeesName {
    fn store(&self, buffer: &mut Vec<u8>) {
    }

    fn load(&self, data: &[u8]) {}
}

impl Serialize for BeesInfo {
    fn store(&self, buffer: &mut Vec<u8>) {}

    fn load(&self, data: &[u8]) {}
}
