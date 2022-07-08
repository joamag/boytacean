pub struct Rom {
    data: Vec<u8>,
}

pub enum RomType {
    RomOnly = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
    Mbc2 = 0x05,
    Mbc2Battery = 0x06,
    Unknown = 0xff,
}

impl Rom {
    pub fn title() -> &'static str {
        "asdas"
    }

    pub fn rom_type() -> RomType {
        RomType::RomOnly
    }
}
