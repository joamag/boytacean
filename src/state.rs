//! System state (BEES format) functions and structures.

use std::{
    convert::TryInto,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::size_of,
};

use crate::{
    gb::{GameBoy, GameBoySpeed},
    info::Info,
    rom::MbcType,
};

pub trait Serialize {
    /// Writes the data from the internal structure into the
    /// provided buffer.
    fn write(&mut self, buffer: &mut Vec<u8>);

    /// Reads the data from the provided buffer and populates
    /// the internal structure with it.
    fn read(&mut self, data: &mut Cursor<Vec<u8>>);
}

pub trait State {
    /// Obtains a new instance of the state from the provided
    /// `GameBoy` instance and returns it.
    fn from_gb(gb: &mut GameBoy) -> Self;

    /// Applies the state to the provided `GameBoy` instance.
    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), String>;
}

#[derive(Default)]
pub struct BeesState {
    footer: BeesFooter,
    name: BeesName,
    info: BeesInfo,
    core: BeesCore,
    mbc: BeesMbc,
    end: BeesBlock,
}

impl BeesState {
    pub fn description(&self, column_length: usize) -> String {
        let emulator_l = format!("{:width$}", "Emulator", width = column_length);
        let title_l: String = format!("{:width$}", "Title", width = column_length);
        let version_l: String = format!("{:width$}", "Version", width = column_length);
        let model_l: String = format!("{:width$}", "Model", width = column_length);
        let ram_l: String = format!("{:width$}", "RAM", width = column_length);
        let vram_l: String = format!("{:width$}", "VRAM", width = column_length);
        let pc_l: String = format!("{:width$}", "PC", width = column_length);
        let sp_l: String = format!("{:width$}", "SP", width = column_length);
        format!(
            "{}  {}\n{}  {}\n{}  {}.{}\n{}  {}\n{}  {}\n{}  {}\n{}  0x{:04X}\n{}  0x{:04X}\n",
            emulator_l,
            self.name.name,
            title_l,
            self.info.title(),
            version_l,
            self.core.major,
            self.core.minor,
            model_l,
            self.core.model,
            ram_l,
            self.core.ram.size,
            vram_l,
            self.core.vram.size,
            pc_l,
            self.core.pc,
            sp_l,
            self.core.sp
        )
    }

    pub fn verify(&self) -> Result<(), String> {
        self.footer.verify()?;
        self.core.verify()?;
        Ok(())
    }

    /// Dumps the core data into the provided buffer and returns.
    /// This will effectively populate the majority of the save
    /// file with the core emulator contents.
    fn dump_core(&mut self, buffer: &mut Vec<u8>) -> u32 {
        let mut offset = 0x0000_u32;

        let mut buffers = vec![
            &mut self.core.ram,
            &mut self.core.vram,
            &mut self.core.mbc_ram,
            &mut self.core.oam,
            &mut self.core.hram,
            &mut self.core.background_palettes,
            &mut self.core.object_palettes,
        ];

        for item in buffers.iter_mut() {
            item.offset = offset;
            buffer.write_all(&item.buffer).unwrap();
            offset += item.size;
        }

        offset
    }
}

impl Serialize for BeesState {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        self.footer.start_offset = self.dump_core(buffer);
        self.name.write(buffer);
        self.info.write(buffer);
        self.core.write(buffer);
        self.mbc.write(buffer);
        self.end.write(buffer);
        self.footer.write(buffer);
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        // moves the cursor to the end of the file
        // to read the footer, and then places the
        // the cursor in the start of the BEES data
        // according to the footer information
        data.seek(SeekFrom::End(-8)).unwrap();
        self.footer.read(data);
        data.seek(SeekFrom::Start(self.footer.start_offset as u64))
            .unwrap();

        loop {
            // reads the block header information and then moves the
            // cursor back to the original position to be able to
            // re-read the block data
            let block = BeesBlockHeader::from_data(data);
            let offset = -((size_of::<u32>() * 2) as i64);
            data.seek(SeekFrom::Current(offset)).unwrap();

            match block.magic.as_str() {
                "NAME" => self.name = BeesName::from_data(data),
                "INFO" => self.info = BeesInfo::from_data(data),
                "CORE" => self.core = BeesCore::from_data(data),
                "MBC " => self.mbc = BeesMbc::from_data(data),
                "END " => self.end = BeesBlock::from_data(data),
                _ => {
                    BeesBlock::from_data(data);
                }
            }

            if block.is_end() {
                break;
            }
        }
    }
}

impl State for BeesState {
    fn from_gb(gb: &mut GameBoy) -> Self {
        Self {
            footer: BeesFooter::default(),
            name: BeesName::from_gb(gb),
            info: BeesInfo::from_gb(gb),
            core: BeesCore::from_gb(gb),
            mbc: BeesMbc::from_gb(gb),
            end: BeesBlock::from_magic(String::from("END ")),
        }
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), String> {
        self.verify()?;
        self.name.to_gb(gb)?;
        self.info.to_gb(gb)?;
        self.core.to_gb(gb)?;
        self.mbc.to_gb(gb)?;
        Ok(())
    }
}

impl Display for BeesState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description(9))
    }
}

pub struct BeesBlockHeader {
    magic: String,
    size: u32,
}

impl BeesBlockHeader {
    pub fn new(magic: String, size: u32) -> Self {
        Self { magic, size }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }

    pub fn is_end(&self) -> bool {
        self.magic == "END "
    }
}

impl Serialize for BeesBlockHeader {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        buffer.write_all(self.magic.as_bytes()).unwrap();
        buffer.write_all(&self.size.to_le_bytes()).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.magic = String::from_utf8(Vec::from(buffer)).unwrap();
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.size = u32::from_le_bytes(buffer);
    }
}

impl Default for BeesBlockHeader {
    fn default() -> Self {
        Self::new(String::from("    "), 0)
    }
}

pub struct BeesBlock {
    header: BeesBlockHeader,
    buffer: Vec<u8>,
}

impl BeesBlock {
    pub fn new(header: BeesBlockHeader, buffer: Vec<u8>) -> Self {
        Self { header, buffer }
    }

    pub fn from_magic(magic: String) -> Self {
        Self::new(BeesBlockHeader::new(magic, 0), vec![])
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }

    pub fn magic(&self) -> &String {
        &self.header.magic
    }

    pub fn is_end(&self) -> bool {
        self.header.is_end()
    }
}

impl Serialize for BeesBlock {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        self.header.write(buffer);
        buffer.write_all(&self.buffer).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);
        self.buffer.reserve_exact(self.header.size as usize);
        data.read_exact(&mut self.buffer).unwrap();
    }
}

impl Default for BeesBlock {
    fn default() -> Self {
        Self::new(BeesBlockHeader::default(), vec![])
    }
}

pub struct BeesBuffer {
    size: u32,
    offset: u32,
    buffer: Vec<u8>,
}

impl BeesBuffer {
    pub fn new(size: u32, offset: u32, buffer: Vec<u8>) -> Self {
        Self {
            size,
            offset,
            buffer,
        }
    }

    /// Fills the buffer with new data and updating the size
    /// value accordingly.
    fn fill_buffer(&mut self, data: &[u8]) {
        self.size = data.len() as u32;
        self.buffer = data.to_vec();
    }

    /// Loads the internal buffer structure with the provided
    /// data according to the size and offset defined.
    fn load_buffer(&self, data: &mut Cursor<Vec<u8>>) -> Vec<u8> {
        let mut buffer = vec![0x00; self.size as usize];
        let position = data.position();
        data.seek(SeekFrom::Start(self.offset as u64)).unwrap();
        data.read_exact(&mut buffer).unwrap();
        data.set_position(position);
        buffer
    }
}

impl Serialize for BeesBuffer {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        buffer.write_all(&self.size.to_le_bytes()).unwrap();
        buffer.write_all(&self.offset.to_le_bytes()).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.size = u32::from_le_bytes(buffer);
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.offset = u32::from_le_bytes(buffer);
        self.buffer = self.load_buffer(data);
    }
}

impl Default for BeesBuffer {
    fn default() -> Self {
        Self::new(0, 0, vec![])
    }
}

pub struct BeesFooter {
    start_offset: u32,
    magic: u32,
}

impl BeesFooter {
    pub fn new(start_offset: u32, magic: u32) -> Self {
        Self {
            start_offset,
            magic,
        }
    }

    pub fn verify(&self) -> Result<(), String> {
        if self.magic != 0x53534542 {
            return Err(String::from("Invalid magic"));
        }
        Ok(())
    }
}

impl Serialize for BeesFooter {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        buffer.write_all(&self.start_offset.to_le_bytes()).unwrap();
        buffer.write_all(&self.magic.to_le_bytes()).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.start_offset = u32::from_le_bytes(buffer);
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.magic = u32::from_le_bytes(buffer);
    }
}

impl Default for BeesFooter {
    fn default() -> Self {
        Self::new(0x00, 0x53534542)
    }
}

pub struct BeesName {
    header: BeesBlockHeader,
    name: String,
}

impl BeesName {
    pub fn new(name: String) -> Self {
        Self {
            header: BeesBlockHeader::new(String::from("NAME"), name.len() as u32),
            name,
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }
}

impl Serialize for BeesName {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        self.header.write(buffer);
        buffer.write_all(self.name.as_bytes()).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);
        let mut buffer = vec![0x00; self.header.size as usize];
        data.read_exact(&mut buffer).unwrap();
        self.name = String::from_utf8(buffer).unwrap();
    }
}

impl State for BeesName {
    fn from_gb(_gb: &mut GameBoy) -> Self {
        Self::new(format!("{} v{}", Info::name(), Info::version()))
    }

    fn to_gb(&self, _gb: &mut GameBoy) -> Result<(), String> {
        Ok(())
    }
}

impl Default for BeesName {
    fn default() -> Self {
        Self::new(String::from(""))
    }
}

pub struct BeesInfo {
    header: BeesBlockHeader,
    title: [u8; 16],
    checksum: [u8; 2],
}

impl BeesInfo {
    pub fn new(title: &[u8], checksum: &[u8]) -> Self {
        Self {
            header: BeesBlockHeader::new(
                String::from("INFO"),
                title.len() as u32 + checksum.len() as u32,
            ),
            title: title.try_into().unwrap(),
            checksum: checksum.try_into().unwrap(),
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }

    pub fn title(&self) -> String {
        let mut final_index = 16;
        for (i, byte) in self.title.iter().enumerate() {
            if *byte == 0u8 {
                final_index = i;
                break;
            }
        }
        String::from(
            String::from_utf8(Vec::from(&self.title[..final_index]))
                .unwrap()
                .trim_matches(char::from(0))
                .trim(),
        )
    }
}

impl Serialize for BeesInfo {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        self.header.write(buffer);
        buffer.write_all(&self.title).unwrap();
        buffer.write_all(&self.checksum).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);
        data.read_exact(&mut self.title).unwrap();
        data.read_exact(&mut self.checksum).unwrap();
    }
}

impl State for BeesInfo {
    fn from_gb(gb: &mut GameBoy) -> Self {
        Self::new(
            &gb.cartridge_i().rom_data()[0x134..=0x143],
            &gb.cartridge_i().rom_data()[0x14e..=0x14f],
        )
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), String> {
        if self.title() != gb.rom_i().title() {
            return Err(format!(
                "Invalid ROM loaded, expected '{}' (len {}) got '{}' (len {})",
                self.title(),
                self.title().len(),
                gb.rom_i().title(),
                gb.rom_i().title().len(),
            ));
        }
        Ok(())
    }
}

impl Default for BeesInfo {
    fn default() -> Self {
        Self::new(&[0_u8; 16], &[0_u8; 2])
    }
}

pub struct BeesCore {
    header: BeesBlockHeader,

    major: u16,
    minor: u16,

    model: String,

    pc: u16,
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    sp: u16,

    ime: bool,
    ie: u8,
    // 0 = running; 1 = halted; 2 = stopped
    execution_mode: u8,
    _padding: u8,

    io_registers: [u8; 128],

    ram: BeesBuffer,
    vram: BeesBuffer,
    mbc_ram: BeesBuffer,
    oam: BeesBuffer,
    hram: BeesBuffer,
    background_palettes: BeesBuffer,
    object_palettes: BeesBuffer,
}

impl BeesCore {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: String,
        pc: u16,
        af: u16,
        bc: u16,
        de: u16,
        hl: u16,
        sp: u16,
        ime: bool,
        ie: u8,
        execution_mode: u8,
        io_registers: [u8; 128],
    ) -> Self {
        Self {
            header: BeesBlockHeader::new(
                String::from("CORE"),
                ((size_of::<u16>() * 2)
                    + size_of::<u32>()
                    + (size_of::<u16>() * 6)
                    + (size_of::<u8>() * 4)
                    + (size_of::<u8>() * 128)
                    + ((size_of::<u32>() + size_of::<u32>()) * 7)) as u32,
            ),
            major: 1,
            minor: 1,
            model,
            pc,
            af,
            bc,
            de,
            hl,
            sp,
            ime,
            ie,
            execution_mode,
            _padding: 0,
            io_registers,
            ram: BeesBuffer::default(),
            vram: BeesBuffer::default(),
            mbc_ram: BeesBuffer::default(),
            oam: BeesBuffer::default(),
            hram: BeesBuffer::default(),
            background_palettes: BeesBuffer::default(),
            object_palettes: BeesBuffer::default(),
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }

    pub fn verify(&self) -> Result<(), String> {
        if self.header.magic != "CORE" {
            return Err(String::from("Invalid magic"));
        }
        if self.oam.size != 0xa0 {
            return Err(String::from("Invalid OAM size"));
        }
        if self.hram.size != 0x7f {
            return Err(String::from("Invalid HRAM size"));
        }
        if (self.is_cgb() && self.background_palettes.size != 0x40)
            || (self.is_dmg() && self.background_palettes.size != 0x00)
        {
            return Err(String::from("Invalid background palettes size"));
        }
        if (self.is_cgb() && self.object_palettes.size != 0x40)
            || (self.is_dmg() && self.object_palettes.size != 0x00)
        {
            return Err(String::from("Invalid object palettes size"));
        }
        Ok(())
    }

    /// Obtains the BEES (Game Boy) model string using the
    /// provided `GameBoy` instance.
    fn bees_model(gb: &GameBoy) -> String {
        let mut buffer = [0x00_u8; 4];

        if gb.is_dmg() {
            buffer[0] = b'G';
        } else if gb.is_cgb() {
            buffer[0] = b'C';
        } else if gb.is_sgb() {
            buffer[0] = b'S';
        } else {
            buffer[0] = b' ';
        }

        if gb.is_dmg() {
            buffer[1] = b'D';
        } else if gb.is_cgb() {
            buffer[1] = b'C';
        } else if gb.is_sgb() {
            buffer[1] = b'N';
        } else {
            buffer[1] = b' ';
        }

        if gb.is_dmg() {
            buffer[2] = b'B';
        } else if gb.is_cgb() {
            buffer[2] = b'A';
        } else {
            buffer[2] = b' ';
        }

        buffer[3] = b' ';

        String::from_utf8(Vec::from(buffer)).unwrap()
    }

    fn is_dmg(&self) -> bool {
        if let Some(first_char) = self.model.chars().next() {
            return first_char == 'G';
        }
        false
    }

    fn is_cgb(&self) -> bool {
        if let Some(first_char) = self.model.chars().next() {
            return first_char == 'C';
        }
        false
    }
}

impl Serialize for BeesCore {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        self.header.write(buffer);

        buffer.write_all(&self.major.to_le_bytes()).unwrap();
        buffer.write_all(&self.minor.to_le_bytes()).unwrap();

        buffer.write_all(self.model.as_bytes()).unwrap();

        buffer.write_all(&self.pc.to_le_bytes()).unwrap();
        buffer.write_all(&self.af.to_le_bytes()).unwrap();
        buffer.write_all(&self.bc.to_le_bytes()).unwrap();
        buffer.write_all(&self.de.to_le_bytes()).unwrap();
        buffer.write_all(&self.hl.to_le_bytes()).unwrap();
        buffer.write_all(&self.sp.to_le_bytes()).unwrap();

        buffer.write_all(&(self.ime as u8).to_le_bytes()).unwrap();
        buffer.write_all(&self.ie.to_le_bytes()).unwrap();
        buffer
            .write_all(&self.execution_mode.to_le_bytes())
            .unwrap();
        buffer.write_all(&self._padding.to_le_bytes()).unwrap();

        buffer.write_all(&self.io_registers).unwrap();

        self.ram.write(buffer);
        self.vram.write(buffer);
        self.mbc_ram.write(buffer);
        self.oam.write(buffer);
        self.hram.write(buffer);
        self.background_palettes.write(buffer);
        self.object_palettes.write(buffer);
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);

        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.major = u16::from_le_bytes(buffer);
        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.minor = u16::from_le_bytes(buffer);

        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.model = String::from_utf8(Vec::from(buffer)).unwrap();

        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.pc = u16::from_le_bytes(buffer);
        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.af = u16::from_le_bytes(buffer);
        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.bc = u16::from_le_bytes(buffer);
        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.de = u16::from_le_bytes(buffer);
        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.hl = u16::from_le_bytes(buffer);
        let mut buffer = [0x00; 2];
        data.read_exact(&mut buffer).unwrap();
        self.sp = u16::from_le_bytes(buffer);

        let mut buffer = [0x00; 1];
        data.read_exact(&mut buffer).unwrap();
        self.ime = buffer[0] != 0;
        let mut buffer = [0x00; 1];
        data.read_exact(&mut buffer).unwrap();
        self.ie = u8::from_le_bytes(buffer);
        let mut buffer = [0x00; 1];
        data.read_exact(&mut buffer).unwrap();
        self.execution_mode = u8::from_le_bytes(buffer);
        let mut buffer = [0x00; 1];
        data.read_exact(&mut buffer).unwrap();
        self._padding = u8::from_le_bytes(buffer);

        data.read_exact(&mut self.io_registers).unwrap();

        self.ram.read(data);
        self.vram.read(data);
        self.mbc_ram.read(data);
        self.oam.read(data);
        self.hram.read(data);
        self.background_palettes.read(data);
        self.object_palettes.read(data);
    }
}

impl State for BeesCore {
    fn from_gb(gb: &mut GameBoy) -> Self {
        let mut core = Self::new(
            Self::bees_model(gb),
            gb.cpu_i().pc(),
            gb.cpu_i().af(),
            gb.cpu_i().bc(),
            gb.cpu_i().de(),
            gb.cpu_i().hl(),
            gb.cpu_i().sp(),
            gb.cpu_i().ime(),
            gb.mmu_i().ie,
            u8::from(gb.cpu().halted()),
            // @TODO: these registers cannot be totally retrieved
            // because of that some audio noise exists
            // The loading of the registers should be done in a much
            // more manual way like SameBoy does here
            // https://github.com/LIJI32/SameBoy/blob/7e6f1f866e89430adaa6be839aecc4a2ccabd69c/Core/save_state.c#L673
            gb.mmu().read_many(0xff00, 128).try_into().unwrap(),
        );
        core.ram.fill_buffer(gb.mmu().ram());
        core.vram.fill_buffer(gb.ppu().vram_device());
        core.mbc_ram.fill_buffer(gb.rom_i().ram_data());
        core.oam.fill_buffer(&gb.mmu().read_many(0xfe00, 0x00a0));
        core.hram.fill_buffer(&gb.mmu().read_many(0xff80, 0x007f));
        if gb.is_cgb() {
            core.background_palettes
                .fill_buffer(&gb.ppu_i().palettes_color()[0]);
            core.object_palettes
                .fill_buffer(&gb.ppu_i().palettes_color()[1]);
        }
        core
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), String> {
        gb.cpu().set_pc(self.pc);
        gb.cpu().set_af(self.af);
        gb.cpu().set_bc(self.bc);
        gb.cpu().set_de(self.de);
        gb.cpu().set_hl(self.hl);
        gb.cpu().set_sp(self.sp);

        gb.cpu().set_ime(self.ime);
        gb.mmu().ie = self.ie;

        // @TODO: we need to be careful about this writing and
        // should make this a bit more robust, to handle this
        // special case/situations
        // The registers should be handled in a more manual manner
        // to avoid unwanted side effects
        // https://github.com/LIJI32/SameBoy/blob/7e6f1f866e89430adaa6be839aecc4a2ccabd69c/Core/save_state.c#L1003
        gb.mmu().write_many(0xff00, &self.io_registers);

        gb.mmu().set_ram(self.ram.buffer.to_vec());
        gb.ppu().set_vram(&self.vram.buffer);
        gb.rom().set_ram_data(&self.mbc_ram.buffer);
        gb.mmu().write_many(0xfe00, &self.oam.buffer);
        gb.mmu().write_many(0xff80, &self.hram.buffer);

        if gb.is_cgb() {
            // updates the internal palettes for the CGB with the values
            // stored in the BEES state
            gb.ppu().set_palettes_color([
                self.background_palettes.buffer.to_vec().try_into().unwrap(),
                self.object_palettes.buffer.to_vec().try_into().unwrap(),
            ]);

            // updates the speed of the CGB according to the KEY1 register
            let is_double = self.io_registers[0x4d_usize] & 0x80 == 0x80;
            gb.mmu().set_speed(if is_double {
                GameBoySpeed::Double
            } else {
                GameBoySpeed::Normal
            });

            // need to disable DMA transfer to avoid unwanted
            // DMA transfers when loading the state
            gb.dma().set_active(false);
        }

        Ok(())
    }
}

impl Default for BeesCore {
    fn default() -> Self {
        Self::new(
            String::from("GD  "),
            0x0000_u16,
            0x0000_u16,
            0x0000_u16,
            0x0000_u16,
            0x0000_u16,
            0x0000_u16,
            false,
            0x00,
            0,
            [0x00; 128],
        )
    }
}

pub struct BeesMbrRegister {
    address: u16,
    value: u8,
}

impl BeesMbrRegister {
    pub fn new(address: u16, value: u8) -> Self {
        Self { address, value }
    }
}

pub struct BeesMbc {
    header: BeesBlockHeader,
    registers: Vec<BeesMbrRegister>,
}

impl BeesMbc {
    pub fn new(registers: Vec<BeesMbrRegister>) -> Self {
        Self {
            header: BeesBlockHeader::new(
                String::from("MBC "),
                ((size_of::<u8>() + size_of::<u16>()) * registers.len()) as u32,
            ),
            registers,
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }
}

impl Serialize for BeesMbc {
    fn write(&mut self, buffer: &mut Vec<u8>) {
        self.header.write(buffer);
        for register in self.registers.iter() {
            buffer.write_all(&register.address.to_le_bytes()).unwrap();
            buffer.write_all(&register.value.to_le_bytes()).unwrap();
        }
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);
        for _ in 0..(self.header.size / 3) {
            let mut buffer = [0x00; 2];
            data.read_exact(&mut buffer).unwrap();
            let address = u16::from_le_bytes(buffer);
            let mut buffer = [0x00; 1];
            data.read_exact(&mut buffer).unwrap();
            let value = u8::from_le_bytes(buffer);
            self.registers.push(BeesMbrRegister::new(address, value));
        }
    }
}

impl State for BeesMbc {
    fn from_gb(gb: &mut GameBoy) -> Self {
        let mut registers = vec![];
        match gb.cartridge().rom_type().mbc_type() {
            MbcType::NoMbc => (),
            MbcType::Mbc1 => {
                registers.push(BeesMbrRegister::new(
                    0x0000,
                    if gb.rom().ram_enabled() {
                        0x0a_u8
                    } else {
                        0x00_u8
                    },
                ));
                registers.push(BeesMbrRegister::new(
                    0x2000,
                    gb.rom().rom_bank() as u8 & 0x1f,
                ));
                registers.push(BeesMbrRegister::new(0x4000, gb.rom().ram_bank()));
                registers.push(BeesMbrRegister::new(0x6000, 0x00_u8));
            }
            MbcType::Mbc3 => {
                registers.push(BeesMbrRegister::new(
                    0x0000,
                    if gb.rom().ram_enabled() {
                        0x0a_u8
                    } else {
                        0x00_u8
                    },
                ));
                registers.push(BeesMbrRegister::new(0x2000, gb.rom().rom_bank() as u8));
                registers.push(BeesMbrRegister::new(0x4000, gb.rom().ram_bank()));
            }
            MbcType::Mbc5 => {
                registers.push(BeesMbrRegister::new(
                    0x0000,
                    if gb.rom().ram_enabled() {
                        0x0a_u8
                    } else {
                        0x00_u8
                    },
                ));
                registers.push(BeesMbrRegister::new(0x2000, gb.rom().rom_bank() as u8));
                registers.push(BeesMbrRegister::new(
                    0x3000,
                    (gb.rom().rom_bank() >> 8) as u8 & 0x01,
                ));
                registers.push(BeesMbrRegister::new(0x4000, gb.rom().ram_bank()));
            }
            _ => unimplemented!(),
        }

        Self::new(registers)
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), String> {
        for register in self.registers.iter() {
            gb.mmu().write(register.address, register.value);
        }
        Ok(())
    }
}

impl Default for BeesMbc {
    fn default() -> Self {
        Self::new(vec![])
    }
}

/// Top level manager structure containing the
/// entrypoint static methods for saving and loading
/// BEES states for the Game Boy.
pub struct StateManager;

impl StateManager {
    pub fn save_file(file_path: &str, gb: &mut GameBoy) -> Result<(), String> {
        let mut file = match File::create(file_path) {
            Ok(file) => file,
            Err(_) => return Err(format!("Failed to open file: {}", file_path)),
        };
        let data = Self::save(gb)?;
        file.write_all(&data).unwrap();
        Ok(())
    }

    pub fn save(gb: &mut GameBoy) -> Result<Vec<u8>, String> {
        let mut data: Vec<u8> = vec![];
        BeesState::from_gb(gb).write(&mut data);
        Ok(data)
    }

    pub fn load_file(file_path: &str, gb: &mut GameBoy) -> Result<(), String> {
        let mut file = match File::open(file_path) {
            Ok(file) => file,
            Err(_) => return Err(format!("Failed to open file: {}", file_path)),
        };
        let mut data = vec![];
        file.read_to_end(&mut data).unwrap();
        Self::load(&data, gb)?;
        Ok(())
    }

    pub fn load(data: &[u8], gb: &mut GameBoy) -> Result<(), String> {
        let mut state = BeesState::default();
        state.read(&mut Cursor::new(data.to_vec()));
        state.to_gb(gb)?;
        print!("{}", state);
        Ok(())
    }
}
