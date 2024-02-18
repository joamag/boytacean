//! System save state (BOS and [BESS](https://github.com/LIJI32/SameBoy/blob/master/BESS.md) formats) functions and structures.

use std::{
    convert::TryInto,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::size_of,
};

use crate::{
    error::Error,
    gb::{GameBoy, GameBoySpeed},
    info::Info,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_SIZE},
    rom::{CgbMode, MbcType},
    util::{get_timestamp, save_bmp},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Magic string for the BOS (Boytacean Save) format.
pub const BOS_MAGIC: &str = "BOS\0";

/// Magic string ("BOS\0") in little endian unsigned 32 bit format.
pub const BOS_MAGIC_UINT: u32 = 0x00534f42;

/// Current version of the BOS (Boytacean Save) format.
pub const BOS_VERSION: u8 = 1;

/// Magic number for the BESS file format.
pub const BESS_MAGIC: u32 = 0x53534542;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum SaveStateFormat {
    Bos,
    Bess,
}

#[derive(Clone, Copy)]
pub enum BosBlockKind {
    Info = 0x01,
    ImageBuffer = 0x02,
    Unknown = 0xff,
}

impl BosBlockKind {
    fn from_u8(value: u8) -> Self {
        match value {
            0x01 => Self::Info,
            0x02 => Self::ImageBuffer,
            _ => Self::Unknown,
        }
    }
}

pub trait Serialize {
    /// Writes the data from the internal structure into the
    /// provided buffer.
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>);

    /// Reads the data from the provided buffer and populates
    /// the internal structure with it.
    fn read(&mut self, data: &mut Cursor<Vec<u8>>);
}

pub trait State {
    /// Obtains a new instance of the state from the provided
    /// `GameBoy` instance and returns it.
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error>
    where
        Self: Sized;

    /// Applies the state to the provided `GameBoy` instance.
    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), Error>;
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Default)]
pub struct BosState {
    magic: u32,
    version: u8,
    block_count: u8,
    info: Option<BosInfo>,
    image_buffer: Option<BosImageBuffer>,
    bess: BessState,
}

impl BosState {
    /// Checks if the data contained in the provided
    /// buffer represents a valid BOS (Boytacean Save)
    /// file structure, thought magic string validation.
    pub fn is_bos(data: &mut Cursor<Vec<u8>>) -> bool {
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        let magic = u32::from_le_bytes(buffer);
        data.rewind().unwrap();
        magic == BOS_MAGIC_UINT
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.magic != BOS_MAGIC_UINT {
            return Err(Error::CustomError(String::from("Invalid magic")));
        }
        self.bess.verify()?;
        Ok(())
    }

    pub fn save_image_bmp(&self, file_path: &str) -> Result<(), Error> {
        if let Some(image_buffer) = &self.image_buffer {
            image_buffer.save_bmp(file_path)?;
            Ok(())
        } else {
            Err(Error::CustomError(String::from("No image buffer found")))
        }
    }

    fn build_block_count(&self) -> u8 {
        let mut count = 0_u8;
        if self.info.is_some() {
            count += 1;
        }
        if self.image_buffer.is_some() {
            count += 1;
        }
        count
    }
}

impl BosState {
    pub fn timestamp(&self) -> Result<u64, Error> {
        if let Some(info) = &self.info {
            Ok(info.timestamp)
        } else {
            Err(Error::CustomError(String::from("No timestamp available")))
        }
    }

    pub fn agent(&self) -> Result<String, Error> {
        if let Some(info) = &self.info {
            Ok(format!("{}/{}", info.agent, info.agent_version))
        } else {
            Err(Error::CustomError(String::from("No agent available")))
        }
    }

    pub fn model(&self) -> Result<String, Error> {
        if let Some(info) = &self.info {
            Ok(info.model.clone())
        } else {
            Err(Error::CustomError(String::from("No model available")))
        }
    }

    pub fn image_eager(&self) -> Result<Vec<u8>, Error> {
        if let Some(image_buffer) = &self.image_buffer {
            Ok(image_buffer.image.to_vec())
        } else {
            Err(Error::CustomError(String::from("No image available")))
        }
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl BosState {
    pub fn timestamp_wa(&self) -> Result<u64, String> {
        Self::timestamp(self).map_err(|e| e.to_string())
    }

    pub fn agent_wa(&self) -> Result<String, String> {
        Self::agent(self).map_err(|e| e.to_string())
    }

    pub fn model_wa(&self) -> Result<String, String> {
        Self::model(self).map_err(|e| e.to_string())
    }

    pub fn image_eager_wa(&self) -> Result<Vec<u8>, String> {
        Self::image_eager(self).map_err(|e| e.to_string())
    }
}

impl Serialize for BosState {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
        self.block_count = self.build_block_count();

        buffer.write_all(&self.magic.to_le_bytes()).unwrap();
        buffer.write_all(&self.version.to_le_bytes()).unwrap();
        buffer.write_all(&self.block_count.to_le_bytes()).unwrap();

        if let Some(info) = &mut self.info {
            info.write(buffer);
        }
        if let Some(image_buffer) = &mut self.image_buffer {
            image_buffer.write(buffer);
        }

        self.bess.write(buffer);
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.magic = u32::from_le_bytes(buffer);
        let mut buffer = [0x00; 1];
        data.read_exact(&mut buffer).unwrap();
        self.version = u8::from_le_bytes(buffer);
        let mut buffer = [0x00; 1];
        data.read_exact(&mut buffer).unwrap();
        self.block_count = u8::from_le_bytes(buffer);

        for _ in 0..self.block_count {
            let block = BosBlock::from_data(data);
            let offset = -((size_of::<u8>() + size_of::<u32>()) as i64);
            data.seek(SeekFrom::Current(offset)).unwrap();

            match block.kind {
                BosBlockKind::Info => {
                    self.info = Some(BosInfo::from_data(data));
                }
                BosBlockKind::ImageBuffer => {
                    self.image_buffer = Some(BosImageBuffer::from_data(data));
                }
                _ => {
                    data.seek(SeekFrom::Current(-offset)).unwrap();
                    data.seek(SeekFrom::Current(block.size as i64)).unwrap();
                }
            }
        }

        self.block_count = self.build_block_count();

        self.bess.read(data);
    }
}

impl State for BosState {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        Ok(Self {
            magic: BOS_MAGIC_UINT,
            version: BOS_VERSION,
            block_count: 2,
            info: Some(BosInfo::from_gb(gb)?),
            image_buffer: Some(BosImageBuffer::from_gb(gb)?),
            bess: BessState::from_gb(gb)?,
        })
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), Error> {
        self.verify()?;
        self.bess.to_gb(gb)?;
        Ok(())
    }
}

pub struct BosBlock {
    kind: BosBlockKind,
    size: u32,
}

impl BosBlock {
    pub fn new(kind: BosBlockKind, size: u32) -> Self {
        Self { kind, size }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }
}

impl Serialize for BosBlock {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
        buffer.write_all(&(self.kind as u8).to_le_bytes()).unwrap();
        buffer.write_all(&self.size.to_le_bytes()).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        let mut buffer = [0x00; 1];
        data.read_exact(&mut buffer).unwrap();
        self.kind = BosBlockKind::from_u8(u8::from_le_bytes(buffer));
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.size = u32::from_le_bytes(buffer);
    }
}

impl Default for BosBlock {
    fn default() -> Self {
        Self::new(BosBlockKind::Info, 0)
    }
}

pub struct BosInfo {
    header: BosBlock,
    timestamp: u64,
    agent: String,
    agent_version: String,
    model: String,
}

impl BosInfo {
    pub fn new(model: String, timestamp: u64, agent: String, agent_version: String) -> Self {
        Self {
            header: BosBlock::new(
                BosBlockKind::Info,
                (size_of::<u64>()
                    + size_of::<u8>() * agent.len()
                    + size_of::<u8>() * agent_version.len()
                    + size_of::<u8>() * model.len()
                    + size_of::<u32>() * 4) as u32,
            ),
            model,
            timestamp,
            agent,
            agent_version,
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }
}

impl Serialize for BosInfo {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
        self.header.write(buffer);

        buffer
            .write_all(&(size_of::<u64>() as u32).to_le_bytes())
            .unwrap();
        buffer.write_all(&self.timestamp.to_le_bytes()).unwrap();

        buffer
            .write_all(&(self.agent.as_bytes().len() as u32).to_le_bytes())
            .unwrap();
        buffer.write_all(self.agent.as_bytes()).unwrap();

        buffer
            .write_all(&(self.agent_version.as_bytes().len() as u32).to_le_bytes())
            .unwrap();
        buffer.write_all(self.agent_version.as_bytes()).unwrap();

        buffer
            .write_all(&(self.model.as_bytes().len() as u32).to_le_bytes())
            .unwrap();
        buffer.write_all(self.model.as_bytes()).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer).unwrap();
        let mut buffer = vec![0x00; u32::from_le_bytes(buffer) as usize];
        data.read_exact(&mut buffer).unwrap();
        self.timestamp = u64::from_le_bytes(buffer.try_into().unwrap());

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer).unwrap();
        let mut buffer = vec![0x00; u32::from_le_bytes(buffer) as usize];
        data.read_exact(&mut buffer).unwrap();
        self.agent = String::from_utf8(buffer).unwrap();

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer).unwrap();
        let mut buffer = vec![0x00; u32::from_le_bytes(buffer) as usize];
        data.read_exact(&mut buffer).unwrap();
        self.agent_version = String::from_utf8(buffer).unwrap();

        let mut buffer = [0x00; size_of::<u32>()];
        data.read_exact(&mut buffer).unwrap();
        let mut buffer = vec![0x00; u32::from_le_bytes(buffer) as usize];
        data.read_exact(&mut buffer).unwrap();
        self.model = String::from_utf8(buffer).unwrap();
    }
}

impl State for BosInfo {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        let timestamp = get_timestamp();
        Ok(Self::new(
            gb.mode().to_string(Some(true)),
            timestamp,
            Info::name(),
            Info::version(),
        ))
    }

    fn to_gb(&self, _gb: &mut GameBoy) -> Result<(), Error> {
        Ok(())
    }
}

impl Default for BosInfo {
    fn default() -> Self {
        Self::new(String::from(""), 0, String::from(""), String::from(""))
    }
}

pub struct BosImageBuffer {
    header: BosBlock,
    image: [u8; FRAME_BUFFER_SIZE],
}

impl BosImageBuffer {
    pub fn new(image: [u8; FRAME_BUFFER_SIZE]) -> Self {
        Self {
            header: BosBlock::new(
                BosBlockKind::ImageBuffer,
                (size_of::<u8>() * FRAME_BUFFER_SIZE) as u32,
            ),
            image,
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }

    pub fn save_bmp(&self, file_path: &str) -> Result<(), Error> {
        save_bmp(
            file_path,
            &self.image,
            DISPLAY_WIDTH as u32,
            DISPLAY_HEIGHT as u32,
        )?;
        Ok(())
    }
}

impl Serialize for BosImageBuffer {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
        self.header.write(buffer);
        buffer.write_all(&self.image).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);
        data.read_exact(&mut self.image).unwrap();
    }
}

impl State for BosImageBuffer {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        Ok(Self::new(gb.ppu_i().frame_buffer_raw()))
    }

    fn to_gb(&self, _gb: &mut GameBoy) -> Result<(), Error> {
        Ok(())
    }
}

impl Default for BosImageBuffer {
    fn default() -> Self {
        Self::new([0x00; FRAME_BUFFER_SIZE])
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Default)]
pub struct BessState {
    footer: BessFooter,
    name: BessName,
    info: BessInfo,
    core: BessCore,
    mbc: BessMbc,
    end: BessBlock,
}

impl BessState {
    /// Checks if the data contained in the provided
    /// buffer represents a valid BESS (Best Effort Save State)
    /// file structure, thought magic string validation.
    pub fn is_bess(data: &mut Cursor<Vec<u8>>) -> bool {
        data.seek(SeekFrom::End(-4)).unwrap();
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        let magic = u32::from_le_bytes(buffer);
        data.rewind().unwrap();
        magic == BESS_MAGIC
    }

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

    pub fn verify(&self) -> Result<(), Error> {
        self.footer.verify()?;
        self.core.verify()?;
        Ok(())
    }

    /// Dumps the core data into the provided buffer and returns.
    /// This will effectively populate the majority of the save
    /// file with the core emulator contents.
    fn dump_core(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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
            item.offset = buffer.position() as u32;
            buffer.write_all(&item.buffer).unwrap();
        }
    }
}

impl Serialize for BessState {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
        self.dump_core(buffer);
        self.footer.start_offset = buffer.position() as u32;
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
        // the cursor in the start of the BESS data
        // according to the footer information
        data.seek(SeekFrom::End(-8)).unwrap();
        self.footer.read(data);
        data.seek(SeekFrom::Start(self.footer.start_offset as u64))
            .unwrap();

        loop {
            // reads the block header information and then moves the
            // cursor back to the original position to be able to
            // re-read the block data
            let block = BessBlockHeader::from_data(data);
            let offset = -((size_of::<u32>() * 2) as i64);
            data.seek(SeekFrom::Current(offset)).unwrap();

            match block.magic.as_str() {
                "NAME" => self.name = BessName::from_data(data),
                "INFO" => self.info = BessInfo::from_data(data),
                "CORE" => self.core = BessCore::from_data(data),
                "MBC " => self.mbc = BessMbc::from_data(data),
                "END " => self.end = BessBlock::from_data(data),
                _ => {
                    BessBlock::from_data(data);
                }
            }

            if block.is_end() {
                break;
            }
        }
    }
}

impl State for BessState {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        Ok(Self {
            footer: BessFooter::default(),
            name: BessName::from_gb(gb)?,
            info: BessInfo::from_gb(gb)?,
            core: BessCore::from_gb(gb)?,
            mbc: BessMbc::from_gb(gb)?,
            end: BessBlock::from_magic(String::from("END ")),
        })
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), Error> {
        self.verify()?;
        self.name.to_gb(gb)?;
        self.info.to_gb(gb)?;
        self.core.to_gb(gb)?;
        self.mbc.to_gb(gb)?;
        Ok(())
    }
}

impl Display for BessState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description(9))
    }
}

pub struct BessBlockHeader {
    magic: String,
    size: u32,
}

impl BessBlockHeader {
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

impl Serialize for BessBlockHeader {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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

impl Default for BessBlockHeader {
    fn default() -> Self {
        Self::new(String::from("    "), 0)
    }
}

pub struct BessBlock {
    header: BessBlockHeader,
    buffer: Vec<u8>,
}

impl BessBlock {
    pub fn new(header: BessBlockHeader, buffer: Vec<u8>) -> Self {
        Self { header, buffer }
    }

    pub fn from_magic(magic: String) -> Self {
        Self::new(BessBlockHeader::new(magic, 0), vec![])
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

impl Serialize for BessBlock {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
        self.header.write(buffer);
        buffer.write_all(&self.buffer).unwrap();
    }

    fn read(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.read(data);
        self.buffer.reserve_exact(self.header.size as usize);
        data.read_exact(&mut self.buffer).unwrap();
    }
}

impl Default for BessBlock {
    fn default() -> Self {
        Self::new(BessBlockHeader::default(), vec![])
    }
}

pub struct BessBuffer {
    size: u32,
    offset: u32,
    buffer: Vec<u8>,
}

impl BessBuffer {
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

impl Serialize for BessBuffer {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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

impl Default for BessBuffer {
    fn default() -> Self {
        Self::new(0, 0, vec![])
    }
}

pub struct BessFooter {
    start_offset: u32,
    magic: u32,
}

impl BessFooter {
    pub fn new(start_offset: u32, magic: u32) -> Self {
        Self {
            start_offset,
            magic,
        }
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.magic != BESS_MAGIC {
            return Err(Error::CustomError(String::from("Invalid magic")));
        }
        Ok(())
    }
}

impl Serialize for BessFooter {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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

impl Default for BessFooter {
    fn default() -> Self {
        Self::new(0x00, BESS_MAGIC)
    }
}

pub struct BessName {
    header: BessBlockHeader,
    name: String,
}

impl BessName {
    pub fn new(name: String) -> Self {
        Self {
            header: BessBlockHeader::new(String::from("NAME"), name.len() as u32),
            name,
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }
}

impl Serialize for BessName {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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

impl State for BessName {
    fn from_gb(_gb: &mut GameBoy) -> Result<Self, Error> {
        Ok(Self::new(format!("{} v{}", Info::name(), Info::version())))
    }

    fn to_gb(&self, _gb: &mut GameBoy) -> Result<(), Error> {
        Ok(())
    }
}

impl Default for BessName {
    fn default() -> Self {
        Self::new(String::from(""))
    }
}

pub struct BessInfo {
    header: BessBlockHeader,
    title: [u8; 16],
    checksum: [u8; 2],
}

impl BessInfo {
    pub fn new(title: &[u8], checksum: &[u8]) -> Self {
        Self {
            header: BessBlockHeader::new(
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
        for (offset, byte) in self.title.iter().enumerate() {
            if *byte == 0u8 {
                final_index = offset;
                break;
            }

            // in we're at the final byte of the title and the value
            // is one that is reserved for CGB compatibility testing
            // then we must ignore it for title processing purposes
            if offset > 14
                && (*byte == CgbMode::CgbCompatible as u8 || *byte == CgbMode::CgbOnly as u8)
            {
                final_index = offset;
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

impl Serialize for BessInfo {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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

impl State for BessInfo {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        Ok(Self::new(
            &gb.cartridge_i().rom_data()[0x0134..=0x0143],
            &gb.cartridge_i().rom_data()[0x014e..=0x014f],
        ))
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), Error> {
        if self.title() != gb.rom_i().title() {
            return Err(Error::CustomError(format!(
                "Invalid ROM loaded, expected '{}' (len {}) got '{}' (len {})",
                self.title(),
                self.title().len(),
                gb.rom_i().title(),
                gb.rom_i().title().len(),
            )));
        }
        Ok(())
    }
}

impl Default for BessInfo {
    fn default() -> Self {
        Self::new(&[0_u8; 16], &[0_u8; 2])
    }
}

pub struct BessCore {
    header: BessBlockHeader,

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

    ram: BessBuffer,
    vram: BessBuffer,
    mbc_ram: BessBuffer,
    oam: BessBuffer,
    hram: BessBuffer,
    background_palettes: BessBuffer,
    object_palettes: BessBuffer,
}

impl BessCore {
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
            header: BessBlockHeader::new(
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
            ram: BessBuffer::default(),
            vram: BessBuffer::default(),
            mbc_ram: BessBuffer::default(),
            oam: BessBuffer::default(),
            hram: BessBuffer::default(),
            background_palettes: BessBuffer::default(),
            object_palettes: BessBuffer::default(),
        }
    }

    pub fn from_data(data: &mut Cursor<Vec<u8>>) -> Self {
        let mut instance = Self::default();
        instance.read(data);
        instance
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.header.magic != "CORE" {
            return Err(Error::CustomError(String::from("Invalid magic")));
        }
        if self.oam.size != 0xa0 {
            return Err(Error::CustomError(String::from("Invalid OAM size")));
        }
        if self.hram.size != 0x7f {
            return Err(Error::CustomError(String::from("Invalid HRAM size")));
        }
        if (self.is_cgb() && self.background_palettes.size != 0x40)
            || (self.is_dmg() && self.background_palettes.size != 0x00)
        {
            return Err(Error::CustomError(String::from(
                "Invalid background palettes size",
            )));
        }
        if (self.is_cgb() && self.object_palettes.size != 0x40)
            || (self.is_dmg() && self.object_palettes.size != 0x00)
        {
            return Err(Error::CustomError(String::from(
                "Invalid object palettes size",
            )));
        }
        Ok(())
    }

    /// Obtains the BESS (Game Boy) model string using the
    /// provided `GameBoy` instance.
    fn bess_model(gb: &GameBoy) -> String {
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

impl Serialize for BessCore {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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

impl State for BessCore {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        let mut core = Self::new(
            Self::bess_model(gb),
            gb.cpu_i().pc(),
            gb.cpu_i().af(),
            gb.cpu_i().bc(),
            gb.cpu_i().de(),
            gb.cpu_i().hl(),
            gb.cpu_i().sp(),
            gb.cpu_i().ime(),
            gb.mmu_i().ie,
            u8::from(gb.cpu().halted()),
            // @TODO: these registers cannot be completely retrieved
            // and because of that some audio noise is played
            // The loading of the registers should be done in a much
            // more manual way like SameBoy does here
            // https://github.com/LIJI32/SameBoy/blob/7e6f1f866e89430adaa6be839aecc4a2ccabd69c/Core/save_state.c#L673
            gb.mmu().read_many_unsafe(0xff00, 128).try_into().unwrap(),
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
        Ok(core)
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), Error> {
        gb.cpu().set_pc(self.pc);
        gb.cpu().set_af(self.af);
        gb.cpu().set_bc(self.bc);
        gb.cpu().set_de(self.de);
        gb.cpu().set_hl(self.hl);
        gb.cpu().set_sp(self.sp);

        gb.cpu().set_ime(self.ime);
        gb.mmu().ie = self.ie;

        match self.execution_mode {
            0 => gb.cpu().set_halted(false),
            1 => gb.cpu().set_halted(true),
            2 => gb.cpu().stop(),
            _ => unimplemented!(),
        }

        // @TODO: we need to be careful about this writing and
        // should make this a bit more robust, to handle this
        // special case/situations
        // The registers should be handled in a more manual manner
        // to avoid unwanted side effects
        // https://github.com/LIJI32/SameBoy/blob/7e6f1f866e89430adaa6be839aecc4a2ccabd69c/Core/save_state.c#L1003
        gb.mmu().write_many_unsafe(0xff00, &self.io_registers);

        gb.mmu().set_ram(self.ram.buffer.to_vec());
        gb.ppu().set_vram(&self.vram.buffer);
        gb.rom().set_ram_data(&self.mbc_ram.buffer);
        gb.mmu().write_many(0xfe00, &self.oam.buffer);
        gb.mmu().write_many(0xff80, &self.hram.buffer);

        if gb.is_cgb() {
            // updates the internal palettes for the CGB with the values
            // stored in the BESS state
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

impl Default for BessCore {
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

pub struct BessMbrRegister {
    address: u16,
    value: u8,
}

impl BessMbrRegister {
    pub fn new(address: u16, value: u8) -> Self {
        Self { address, value }
    }
}

pub struct BessMbc {
    header: BessBlockHeader,
    registers: Vec<BessMbrRegister>,
}

impl BessMbc {
    pub fn new(registers: Vec<BessMbrRegister>) -> Self {
        Self {
            header: BessBlockHeader::new(
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

impl Serialize for BessMbc {
    fn write(&mut self, buffer: &mut Cursor<Vec<u8>>) {
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
            self.registers.push(BessMbrRegister::new(address, value));
        }
    }
}

impl State for BessMbc {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        let mut registers = vec![];
        match gb.cartridge().rom_type().mbc_type() {
            MbcType::NoMbc => (),
            MbcType::Mbc1 => {
                registers.push(BessMbrRegister::new(
                    0x0000,
                    if gb.rom().ram_enabled() {
                        0x0a_u8
                    } else {
                        0x00_u8
                    },
                ));
                registers.push(BessMbrRegister::new(
                    0x2000,
                    gb.rom().rom_bank() as u8 & 0x1f,
                ));
                registers.push(BessMbrRegister::new(0x4000, gb.rom().ram_bank()));
                registers.push(BessMbrRegister::new(0x6000, 0x00_u8));
            }
            MbcType::Mbc3 => {
                registers.push(BessMbrRegister::new(
                    0x0000,
                    if gb.rom().ram_enabled() {
                        0x0a_u8
                    } else {
                        0x00_u8
                    },
                ));
                registers.push(BessMbrRegister::new(0x2000, gb.rom().rom_bank() as u8));
                registers.push(BessMbrRegister::new(0x4000, gb.rom().ram_bank()));
            }
            MbcType::Mbc5 => {
                registers.push(BessMbrRegister::new(
                    0x0000,
                    if gb.rom().ram_enabled() {
                        0x0a_u8
                    } else {
                        0x00_u8
                    },
                ));
                registers.push(BessMbrRegister::new(0x2000, gb.rom().rom_bank() as u8));
                registers.push(BessMbrRegister::new(
                    0x3000,
                    (gb.rom().rom_bank() >> 8) as u8 & 0x01,
                ));
                registers.push(BessMbrRegister::new(0x4000, gb.rom().ram_bank()));
            }
            _ => unimplemented!(),
        }

        Ok(Self::new(registers))
    }

    fn to_gb(&self, gb: &mut GameBoy) -> Result<(), Error> {
        for register in self.registers.iter() {
            gb.mmu().write(register.address, register.value);
        }
        Ok(())
    }
}

impl Default for BessMbc {
    fn default() -> Self {
        Self::new(vec![])
    }
}

/// Top level manager structure containing the
/// entrypoint static methods for saving and loading
/// [BESS](https://github.com/LIJI32/SameBoy/blob/master/BESS.md) state
/// files and buffers for the Game Boy.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct StateManager;

impl StateManager {
    pub fn save_file(
        file_path: &str,
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
    ) -> Result<(), Error> {
        let mut file = File::create(file_path)
            .map_err(|_| Error::CustomError(format!("Failed to create file: {}", file_path)))?;
        let data = Self::save(gb, format)?;
        file.write_all(&data).unwrap();
        Ok(())
    }

    pub fn load_file(
        file_path: &str,
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
    ) -> Result<(), Error> {
        let mut file = File::open(file_path)
            .map_err(|_| Error::CustomError(format!("Failed to open file: {}", file_path)))?;
        let mut data = vec![];
        file.read_to_end(&mut data).unwrap();
        Self::load(&data, gb, format)?;
        Ok(())
    }
}

impl StateManager {
    pub fn save(gb: &mut GameBoy, format: Option<SaveStateFormat>) -> Result<Vec<u8>, Error> {
        let mut data = Cursor::new(vec![]);
        match format {
            Some(SaveStateFormat::Bos) | None => {
                let mut state = BosState::from_gb(gb)?;
                state.write(&mut data);
            }
            Some(SaveStateFormat::Bess) => {
                let mut state = BessState::from_gb(gb)?;
                state.write(&mut data);
            }
        }
        Ok(data.into_inner())
    }

    pub fn load(
        data: &[u8],
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
    ) -> Result<(), Error> {
        let data = &mut Cursor::new(data.to_vec());
        let format = match format {
            Some(format) => format,
            None => {
                if BosState::is_bos(data) {
                    SaveStateFormat::Bos
                } else if BessState::is_bess(data) {
                    SaveStateFormat::Bess
                } else {
                    return Err(Error::CustomError(String::from(
                        "Unknown save state file format",
                    )));
                }
            }
        };
        match format {
            SaveStateFormat::Bos => {
                let mut state = BosState::default();
                state.read(data);
                state.to_gb(gb)?;
            }
            SaveStateFormat::Bess => {
                let mut state = BessState::default();
                state.read(data);
                state.to_gb(gb)?;
            }
        }
        Ok(())
    }

    pub fn read_bos(data: &[u8]) -> Result<BosState, Error> {
        let data = &mut Cursor::new(data.to_vec());
        let mut state = BosState::default();
        state.read(data);
        Ok(state)
    }

    pub fn read_bess(data: &[u8]) -> Result<BessState, Error> {
        let data = &mut Cursor::new(data.to_vec());
        let mut state = BessState::default();
        state.read(data);
        Ok(state)
    }

    /// Obtains the thumbnail of the save state file, this thumbnail is
    /// stored in raw RGB format.
    /// This operation is currently only supported for the BOS format.
    pub fn thumbnail(data: &[u8], format: Option<SaveStateFormat>) -> Result<Vec<u8>, Error> {
        let data = &mut Cursor::new(data.to_vec());
        let format = match format {
            Some(format) => format,
            None => {
                if BosState::is_bos(data) {
                    SaveStateFormat::Bos
                } else if BessState::is_bess(data) {
                    SaveStateFormat::Bess
                } else {
                    return Err(Error::CustomError(String::from(
                        "Unknown save state file format",
                    )));
                }
            }
        };
        match format {
            SaveStateFormat::Bos => {
                let mut state = BosState::default();
                state.read(data);
                Ok(state.image_buffer.unwrap().image.to_vec())
            }
            SaveStateFormat::Bess => Err(Error::CustomError(String::from(
                "Format foes not support thumbnail",
            ))),
        }
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl StateManager {
    pub fn save_wa(gb: &mut GameBoy, format: Option<SaveStateFormat>) -> Result<Vec<u8>, String> {
        Self::save(gb, format).map_err(|e| e.to_string())
    }

    pub fn load_wa(
        data: &[u8],
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
    ) -> Result<(), String> {
        Self::load(data, gb, format).map_err(|e| e.to_string())
    }

    pub fn read_bos_wa(data: &[u8]) -> Result<BosState, String> {
        Self::read_bos(data).map_err(|e| e.to_string())
    }

    pub fn read_bess_wa(data: &[u8]) -> Result<BessState, String> {
        Self::read_bess(data).map_err(|e| e.to_string())
    }

    pub fn thumbnail_wa(data: &[u8], format: Option<SaveStateFormat>) -> Result<Vec<u8>, String> {
        Self::thumbnail(data, format).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gb::GameBoy;

    #[test]
    fn test_load_bos() {
        let mut gb = GameBoy::default();
        gb.load(true);
        gb.load_rom_file("res/roms/test/firstwhite.gb", None);
        let data = StateManager::save(&mut gb, Some(SaveStateFormat::Bos)).unwrap();
        StateManager::load(&data, &mut gb, Some(SaveStateFormat::Bos)).unwrap();
        StateManager::load(&data, &mut gb, None).unwrap();
    }

    #[test]
    fn test_load_bess() {
        let mut gb = GameBoy::default();
        gb.load(true);
        gb.load_rom_file("res/roms/test/firstwhite.gb", None);
        let data = StateManager::save(&mut gb, Some(SaveStateFormat::Bess)).unwrap();
        StateManager::load(&data, &mut gb, Some(SaveStateFormat::Bess)).unwrap();
        StateManager::load(&data, &mut gb, None).unwrap();
    }
}
