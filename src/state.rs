#[repr(packed)]
pub struct BeesBlock {
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
pub struct BeesCore {
    pub header: BeesBlock,

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

    pub io_registers: [u8; 0x80],

    pub ram: BeesBuffer,
    pub vram: BeesBuffer,
    pub mbc_ram: BeesBuffer,
    pub oam: BeesBuffer,
    pub hram: BeesBuffer,
    pub background_palettes: BeesBuffer,
    pub object_palettes: BeesBuffer,
}
