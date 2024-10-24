//! System save state (BOS and [BESS](https://github.com/LIJI32/SameBoy/blob/master/BESS.md) formats) functions and structures.
//!
//! The BOS (Boytacean Save) format is a custom save state format that contains the emulator state and the frame buffer.
//! Its serialization includes header, info, image buffer and then a BESS (Best Effort Save State) footer with the state itself.
//!
//! The [BESS](https://github.com/LIJI32/SameBoy/blob/master/BESS.md) format is a format developed by the [SameBoy](https://sameboy.github.io/) emulator and is used to store the emulator state
//! in agnostic and compatible way.

use boytacean_common::{
    data::{
        read_bytes, read_into, read_u16, read_u32, read_u64, read_u8, write_bytes, write_u16,
        write_u32, write_u64, write_u8,
    },
    error::Error,
    util::{save_bmp, timestamp},
};
use boytacean_encoding::zippy::{decode_zippy, encode_zippy};
use std::{
    convert::TryInto,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::size_of,
    vec,
};

use crate::{
    disable_pedantic, enable_pedantic,
    gb::{GameBoy, GameBoyDevice, GameBoyMode, GameBoySpeed},
    info::Info,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_SIZE},
    rom::{CgbMode, MbcType},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Magic string for the BOSC (Boytacean Save Compressed) format.
pub const BOSC_MAGIC: &str = "BOSC";

/// Magic string ("BOSC") in little endian unsigned 32 bit format.
pub const BOSC_MAGIC_UINT: u32 = 0x43534f42;

/// Magic string for the BOS (Boytacean Save) format.
pub const BOS_MAGIC: &str = "BOS\0";

/// Magic string ("BOS\0") in little endian unsigned 32 bit format.
pub const BOS_MAGIC_UINT: u32 = 0x00534f42;

/// Current version of the BOS (Boytacean Save Compressed) format.
pub const BOSC_VERSION: u8 = 1;

/// Current version of the BOS (Boytacean Save) format.
pub const BOS_VERSION: u8 = 1;

/// Magic number for the BESS file format.
pub const BESS_MAGIC: u32 = 0x53534542;

/// Represents the different formats for the state storage
/// and retrieval.
///
/// Different formats will have different levels of detail
/// and will require different amounts of data to be
/// stored and retrieved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum StateFormat {
    /// Minimal state format, meaning that only the most basic
    /// elements of the component will be stored and retrieved.
    Minimal = 1,

    /// Partial state format, meaning that only the essential
    /// elements of the component will be stored and retrieved.
    /// All the remaining data, should inferred or computed.
    Partial = 2,

    /// Full state format, meaning that every single element
    /// of the component will be stored and retrieved. This
    /// should included redundant and calculated data.
    Full = 3,
}

impl From<u8> for StateFormat {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Minimal,
            2 => Self::Partial,
            3 => Self::Full,
            _ => Self::Partial,
        }
    }
}

/// Represents a component that is able to store and retrieve
/// the state of its internal structure.
///
/// This trait is used to define the behavior of the state
/// components that are used to store the emulator state.
///
/// Ideally each of Game Boy's components should implement
/// this trait to allow the state to be saved and restored
/// in a consistent way.
pub trait StateComponent {
    fn state(&self, format: Option<StateFormat>) -> Result<Vec<u8>, Error>;
    fn set_state(&mut self, data: &[u8], format: Option<StateFormat>) -> Result<(), Error>;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub enum SaveStateFormat {
    /// Boytacean Save Compressed format (BOSC).
    /// This format uses the Zippy compression algorithm
    /// to compress the underlying BOS contents.
    Bosc = 1,

    /// Boytacean Save format (uncompressed) (BOS).
    Bos = 2,

    /// Best Effort Save State format (BESS).
    Bess = 3,
}

impl SaveStateFormat {
    pub fn description(&self) -> String {
        match self {
            Self::Bosc => String::from("BOSC"),
            Self::Bos => String::from("BOS"),
            Self::Bess => String::from("BESS"),
        }
    }

    pub fn from_string(value: &str) -> Self {
        match value {
            "BOSC" => Self::Bosc,
            "BOS" => Self::Bos,
            "BESS" => Self::Bess,
            _ => Self::Bos,
        }
    }
}

impl From<&str> for SaveStateFormat {
    fn from(value: &str) -> Self {
        Self::from_string(value)
    }
}

impl Display for SaveStateFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[derive(Clone, Copy)]
pub enum BosBlockKind {
    Info = 0x01,
    ImageBuffer = 0x02,
    DeviceState = 0x03,
    Unknown = 0xff,
}

impl BosBlockKind {
    fn from_u8(value: u8) -> Self {
        match value {
            0x01 => Self::Info,
            0x02 => Self::ImageBuffer,
            0x03 => Self::DeviceState,
            _ => Self::Unknown,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Info => String::from("Info"),
            Self::ImageBuffer => String::from("ImageBuffer"),
            Self::DeviceState => String::from("DeviceState"),
            Self::Unknown => String::from("Unknown"),
        }
    }
}

impl Display for BosBlockKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u8> for BosBlockKind {
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct FromGbOptions {
    thumbnail: bool,
    state_format: Option<StateFormat>,
    agent: Option<String>,
    agent_version: Option<String>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl FromGbOptions {
    pub fn new(
        thumbnail: bool,
        state_format: Option<StateFormat>,
        agent: Option<String>,
        agent_version: Option<String>,
    ) -> Self {
        Self {
            thumbnail,
            state_format,
            agent,
            agent_version,
        }
    }
}

impl Default for FromGbOptions {
    fn default() -> Self {
        Self {
            thumbnail: true,
            state_format: None,
            agent: None,
            agent_version: None,
        }
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct ToGbOptions {
    reload: bool,
}

impl ToGbOptions {
    pub fn new(reload: bool) -> Self {
        Self { reload }
    }
}

impl Default for ToGbOptions {
    fn default() -> Self {
        Self { reload: true }
    }
}

pub trait Serialize {
    /// Writes the data from the internal structure into the
    /// provided buffer.
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error>;

    /// Reads the data from the provided buffer and populates
    /// the internal structure with it.
    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error>;
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

pub trait StateBox {
    /// Obtains a new instance of the state from the provided
    /// `GameBoy` instance and returns it as a boxed value.
    fn from_gb(gb: &mut GameBoy, options: &FromGbOptions) -> Result<Box<Self>, Error>
    where
        Self: Sized;

    /// Applies the state to the provided `GameBoy` instance.
    fn to_gb(&self, gb: &mut GameBoy, options: &ToGbOptions) -> Result<(), Error>;
}

pub trait StateConfig {
    /// Obtains the Game Boy execution mode expected by the
    /// state instance.
    fn mode(&self) -> Result<GameBoyMode, Error>;
}

pub trait StateInfo {
    fn timestamp(&self) -> Result<u64, Error>;
    fn agent(&self) -> Result<String, Error>;
    fn model(&self) -> Result<String, Error>;
    fn title(&self) -> Result<String, Error>;
    fn image_eager(&self) -> Result<Vec<u8>, Error>;
    fn has_image(&self) -> bool;
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Default)]
pub struct BoscState {
    magic: u32,
    version: u8,
    bos: BosState,
}

impl BoscState {
    /// Checks if the data contained in the provided
    /// buffer represents a valid BOSC (Boytacean Save
    /// Compressed) file structure, thought magic
    /// string validation.
    pub fn is_bosc<R: Read + Seek>(reader: &mut R) -> Result<bool, Error> {
        let magic = read_u32(reader)?;
        reader.rewind()?;
        Ok(magic == BOSC_MAGIC_UINT)
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.magic != BOSC_MAGIC_UINT {
            return Err(Error::DataError(String::from("Invalid magic")));
        }
        if self.version != BOSC_VERSION {
            return Err(Error::DataError(format!(
                "Invalid version, expected {BOS_VERSION}, got {}",
                self.version
            )));
        }
        self.bos.verify()?;
        Ok(())
    }
}

impl Serialize for BoscState {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        write_u32(writer, self.magic)?;
        write_u8(writer, self.version)?;

        let mut cursor = Cursor::new(vec![]);
        self.bos.write(&mut cursor)?;

        let bos_compressed = encode_zippy(&cursor.into_inner(), None, None)?;
        write_bytes(writer, &bos_compressed)?;

        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.magic = read_u32(reader)?;
        self.version = read_u8(reader)?;

        let mut bos_compressed = vec![];
        reader.read_to_end(&mut bos_compressed)?;
        let bos_buffer = decode_zippy(&bos_compressed, None)?;
        let mut bos_cursor = Cursor::new(bos_buffer);

        self.bos.read(&mut bos_cursor)?;

        Ok(())
    }
}

impl StateBox for BoscState {
    fn from_gb(gb: &mut GameBoy, options: &FromGbOptions) -> Result<Box<Self>, Error> {
        Ok(Box::new(Self {
            magic: BOSC_MAGIC_UINT,
            version: BOSC_VERSION,
            bos: *BosState::from_gb(gb, options)?,
        }))
    }

    fn to_gb(&self, gb: &mut GameBoy, options: &ToGbOptions) -> Result<(), Error> {
        self.verify()?;
        self.bos.to_gb(gb, options)?;
        Ok(())
    }
}

impl StateConfig for BoscState {
    fn mode(&self) -> Result<GameBoyMode, Error> {
        self.bos.mode()
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Default)]
pub struct BosState {
    magic: u32,
    version: u8,
    block_count: u8,
    info: Option<BosInfo>,
    image_buffer: Option<BosImageBuffer>,
    device_states: Vec<BosDeviceState>,
    bess: BessState,
}

impl BosState {
    /// Checks if the data contained in the provided
    /// buffer represents a valid BOS (Boytacean Save)
    /// file structure, thought magic string validation.
    pub fn is_bos<R: Read + Seek>(reader: &mut R) -> Result<bool, Error> {
        let magic = read_u32(reader)?;
        reader.rewind()?;
        Ok(magic == BOS_MAGIC_UINT)
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.magic != BOS_MAGIC_UINT {
            return Err(Error::CustomError(String::from("Invalid magic")));
        }
        if self.version != BOS_VERSION {
            return Err(Error::CustomError(format!(
                "Invalid version, expected {BOS_VERSION}, got {}",
                self.version
            )));
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
        count += self.device_states.len() as u8;
        count
    }
}

impl StateInfo for BosState {
    fn timestamp(&self) -> Result<u64, Error> {
        if let Some(info) = &self.info {
            Ok(info.timestamp)
        } else {
            Err(Error::CustomError(String::from("No timestamp available")))
        }
    }

    fn agent(&self) -> Result<String, Error> {
        if let Some(info) = &self.info {
            Ok(format!("{}/{}", info.agent, info.agent_version))
        } else {
            Err(Error::CustomError(String::from("No agent available")))
        }
    }

    fn model(&self) -> Result<String, Error> {
        if let Some(info) = &self.info {
            Ok(info.model.clone())
        } else {
            Err(Error::CustomError(String::from("No model available")))
        }
    }

    fn title(&self) -> Result<String, Error> {
        self.bess.title()
    }

    fn image_eager(&self) -> Result<Vec<u8>, Error> {
        if let Some(image_buffer) = &self.image_buffer {
            Ok(image_buffer.image.to_vec())
        } else {
            Err(Error::CustomError(String::from("No image available")))
        }
    }

    fn has_image(&self) -> bool {
        self.image_buffer.is_some()
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl BosState {
    pub fn timestamp_wa(&self) -> Result<u64, String> {
        Ok(Self::timestamp(self)?)
    }

    pub fn agent_wa(&self) -> Result<String, String> {
        Ok(Self::agent(self)?)
    }

    pub fn model_wa(&self) -> Result<String, String> {
        Ok(Self::model(self)?)
    }

    pub fn title_wa(&self) -> Result<String, String> {
        Ok(Self::title(self)?)
    }

    pub fn image_eager_wa(&self) -> Result<Vec<u8>, String> {
        Ok(Self::image_eager(self)?)
    }

    pub fn has_image_wa(&self) -> bool {
        self.has_image()
    }
}

impl Serialize for BosState {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.block_count = self.build_block_count();

        write_u32(writer, self.magic)?;
        write_u8(writer, self.version)?;
        write_u8(writer, self.block_count)?;

        if let Some(info) = &mut self.info {
            info.write(writer)?;
        }
        if let Some(image_buffer) = &mut self.image_buffer {
            image_buffer.write(writer)?;
        }
        for device_state in &mut self.device_states {
            device_state.write(writer)?;
        }

        self.bess.write(writer)?;

        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.magic = read_u32(reader)?;
        self.version = read_u8(reader)?;
        self.block_count = read_u8(reader)?;

        for _ in 0..self.block_count {
            let block = BosBlock::from_data(reader)?;
            let offset = -((size_of::<u8>() + size_of::<u16>() + size_of::<u32>()) as i64);
            reader.seek(SeekFrom::Current(offset))?;

            match block.kind {
                BosBlockKind::Info => {
                    self.info = Some(BosInfo::from_data(reader)?);
                }
                BosBlockKind::ImageBuffer => {
                    self.image_buffer = Some(BosImageBuffer::from_data(reader)?);
                }
                BosBlockKind::DeviceState => {
                    self.device_states.push(BosDeviceState::from_data(reader)?);
                }
                _ => {
                    reader.seek(SeekFrom::Current(-offset))?;
                    reader.seek(SeekFrom::Current(block.size as i64))?;
                }
            }
        }

        self.block_count = self.build_block_count();

        self.bess.read(reader)?;

        Ok(())
    }
}

impl StateBox for BosState {
    fn from_gb(gb: &mut GameBoy, options: &FromGbOptions) -> Result<Box<Self>, Error> {
        Ok(Box::new(Self {
            magic: BOS_MAGIC_UINT,
            version: BOS_VERSION,
            block_count: 2,
            info: Some(*<BosInfo as StateBox>::from_gb(gb, options)?),
            image_buffer: if options.thumbnail {
                Some(BosImageBuffer::from_gb(gb)?)
            } else {
                None
            },
            device_states: vec![
                BosDeviceState::from_gb(gb, GameBoyDevice::Cpu, options)?,
                BosDeviceState::from_gb(gb, GameBoyDevice::Ppu, options)?,
                BosDeviceState::from_gb(gb, GameBoyDevice::Apu, options)?,
                BosDeviceState::from_gb(gb, GameBoyDevice::Dma, options)?,
                BosDeviceState::from_gb(gb, GameBoyDevice::Pad, options)?,
                BosDeviceState::from_gb(gb, GameBoyDevice::Timer, options)?,
            ],
            bess: *BessState::from_gb(gb, options)?,
        }))
    }

    fn to_gb(&self, gb: &mut GameBoy, options: &ToGbOptions) -> Result<(), Error> {
        self.verify()?;
        self.bess.to_gb(gb, options)?;
        for device_state in &self.device_states {
            device_state.to_gb(gb, options)?;
        }
        Ok(())
    }
}

impl StateConfig for BosState {
    fn mode(&self) -> Result<GameBoyMode, Error> {
        self.bess.mode()
    }
}

pub struct BosBlock {
    kind: BosBlockKind,
    version: u16,
    size: u32,
}

impl BosBlock {
    pub fn new(kind: BosBlockKind, version: u16, size: u32) -> Self {
        Self {
            kind,
            version,
            size,
        }
    }

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }

    pub fn description(&self) -> String {
        format!("{} version={} size={}", self.kind, self.version, self.size)
    }
}

impl Serialize for BosBlock {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        write_u8(writer, self.kind as u8)?;
        write_u16(writer, self.version)?;
        write_u32(writer, self.size)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        let check = self.version != 0;
        let expected_version = self.version;

        self.kind = read_u8(reader)?.into();
        self.version = read_u16(reader)?;
        self.size = read_u32(reader)?;

        if check && self.version != expected_version {
            return Err(Error::DataError(format!(
                "Invalid version, expected {expected_version}, got {} for block ({})",
                self.version, self
            )));
        }
        Ok(())
    }
}

impl Default for BosBlock {
    fn default() -> Self {
        Self::new(BosBlockKind::Info, 0, 0)
    }
}

impl Display for BosBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
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
                1,
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

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }
}

impl Serialize for BosInfo {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;

        write_u32(writer, size_of::<u64>() as u32)?;
        write_u64(writer, self.timestamp)?;

        write_u32(writer, self.agent.as_bytes().len() as u32)?;
        write_bytes(writer, self.agent.as_bytes())?;

        write_u32(writer, self.agent_version.as_bytes().len() as u32)?;
        write_bytes(writer, self.agent_version.as_bytes())?;

        write_u32(writer, self.model.as_bytes().len() as u32)?;
        write_bytes(writer, self.model.as_bytes())?;

        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;

        read_u32(reader)?;
        self.timestamp = read_u64(reader)?;

        let buffer_len = read_u32(reader)? as usize;
        self.agent = String::from_utf8(read_bytes(reader, buffer_len)?)?;

        let buffer_len = read_u32(reader)? as usize;
        self.agent_version = String::from_utf8(read_bytes(reader, buffer_len)?)?;

        let buffer_len = read_u32(reader)? as usize;
        self.model = String::from_utf8(read_bytes(reader, buffer_len)?)?;

        Ok(())
    }
}

impl State for BosInfo {
    fn from_gb(gb: &mut GameBoy) -> Result<Self, Error> {
        let timestamp = timestamp();
        Ok(Self::new(
            gb.mode().to_string(Some(true)),
            timestamp,
            Info::name_lower(),
            Info::version(),
        ))
    }

    fn to_gb(&self, _gb: &mut GameBoy) -> Result<(), Error> {
        Ok(())
    }
}

impl StateBox for BosInfo {
    fn from_gb(gb: &mut GameBoy, options: &FromGbOptions) -> Result<Box<Self>, Error>
    where
        Self: Sized,
    {
        let timestamp = timestamp();
        Ok(Box::new(Self::new(
            gb.mode().to_string(Some(true)),
            timestamp,
            options.agent.clone().unwrap_or(Info::name_lower()),
            options.agent_version.clone().unwrap_or(Info::version()),
        )))
    }

    fn to_gb(&self, _gb: &mut GameBoy, _options: &ToGbOptions) -> Result<(), Error> {
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
                1,
                (size_of::<u8>() * FRAME_BUFFER_SIZE) as u32,
            ),
            image,
        }
    }

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
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
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;
        write_bytes(writer, &self.image)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;
        read_into(reader, &mut self.image)?;
        Ok(())
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

pub struct BosDeviceState {
    header: BosBlock,
    device: GameBoyDevice,
    format: StateFormat,
    state: Vec<u8>,
}

impl BosDeviceState {
    pub fn new(device: GameBoyDevice, format: StateFormat, state: Vec<u8>) -> Self {
        Self {
            header: BosBlock::new(
                BosBlockKind::DeviceState,
                1,
                (size_of::<u8>() + size_of::<u8>() + state.len()) as u32,
            ),
            device,
            format,
            state,
        }
    }

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }

    fn from_gb(
        gb: &mut GameBoy,
        device: GameBoyDevice,
        options: &FromGbOptions,
    ) -> Result<Self, Error> {
        let format: StateFormat = options.state_format.unwrap_or(StateFormat::Partial);
        match device {
            GameBoyDevice::Cpu => Ok(Self::new(device, format, gb.cpu_i().state(Some(format))?)),
            GameBoyDevice::Ppu => Ok(Self::new(device, format, gb.ppu_i().state(Some(format))?)),
            GameBoyDevice::Apu => Ok(Self::new(device, format, gb.apu_i().state(Some(format))?)),
            GameBoyDevice::Dma => Ok(Self::new(device, format, gb.dma_i().state(Some(format))?)),
            GameBoyDevice::Pad => Ok(Self::new(device, format, gb.pad_i().state(Some(format))?)),
            GameBoyDevice::Timer => {
                Ok(Self::new(device, format, gb.timer_i().state(Some(format))?))
            }
            _ => Err(Error::NotImplemented),
        }
    }

    fn to_gb(&self, gb: &mut GameBoy, _options: &ToGbOptions) -> Result<(), Error> {
        match self.device {
            GameBoyDevice::Cpu => gb.cpu().set_state(&self.state, Some(self.format))?,
            GameBoyDevice::Ppu => gb.ppu().set_state(&self.state, Some(self.format))?,
            GameBoyDevice::Apu => gb.apu().set_state(&self.state, Some(self.format))?,
            GameBoyDevice::Dma => gb.dma().set_state(&self.state, Some(self.format))?,
            GameBoyDevice::Pad => gb.pad().set_state(&self.state, Some(self.format))?,
            GameBoyDevice::Timer => gb.timer().set_state(&self.state, Some(self.format))?,
            _ => return Err(Error::NotImplemented),
        }
        Ok(())
    }
}

impl Serialize for BosDeviceState {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;
        write_u8(writer, self.device as u8)?;
        write_u8(writer, self.format as u8)?;
        write_bytes(writer, &self.state)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;
        self.device = read_u8(reader)?.into();
        self.format = read_u8(reader)?.into();
        let state_len = self.header.size as usize - size_of::<u8>() - size_of::<u8>();
        self.state.append(&mut read_bytes(reader, state_len)?);
        Ok(())
    }
}

impl Default for BosDeviceState {
    fn default() -> Self {
        Self::new(GameBoyDevice::Unknown, StateFormat::Partial, vec![])
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
    pub fn is_bess<R: Read + Seek>(reader: &mut R) -> Result<bool, Error> {
        reader.seek(SeekFrom::End(-4))?;
        let magic = read_u32(reader)?;
        reader.rewind()?;
        Ok(magic == BESS_MAGIC)
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
    fn dump_core<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        let mut buffers = [
            &mut self.core.ram,
            &mut self.core.vram,
            &mut self.core.mbc_ram,
            &mut self.core.oam,
            &mut self.core.hram,
            &mut self.core.background_palettes,
            &mut self.core.object_palettes,
        ];

        for item in buffers.iter_mut() {
            item.offset = writer.stream_position()? as u32;
            write_bytes(writer, &item.buffer)?;
        }

        Ok(())
    }
}

impl StateInfo for BessState {
    fn timestamp(&self) -> Result<u64, Error> {
        Ok(0)
    }

    fn agent(&self) -> Result<String, Error> {
        Ok(self.name.name.clone())
    }

    fn model(&self) -> Result<String, Error> {
        Ok(self.core.mode().into())
    }

    fn title(&self) -> Result<String, Error> {
        Ok(self.info.title())
    }

    fn image_eager(&self) -> Result<Vec<u8>, Error> {
        Err(Error::NotImplemented)
    }

    fn has_image(&self) -> bool {
        false
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl BessState {
    pub fn timestamp_wa(&self) -> Result<u64, String> {
        Ok(Self::timestamp(self)?)
    }

    pub fn agent_wa(&self) -> Result<String, String> {
        Ok(Self::agent(self)?)
    }

    pub fn model_wa(&self) -> Result<String, String> {
        Ok(Self::model(self)?)
    }

    pub fn title_wa(&self) -> Result<String, String> {
        Ok(Self::title(self)?)
    }

    pub fn image_eager_wa(&self) -> Result<Vec<u8>, String> {
        Ok(Self::image_eager(self)?)
    }

    pub fn has_image_wa(&self) -> bool {
        self.has_image()
    }
}

impl Serialize for BessState {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.dump_core(writer)?;
        self.footer.start_offset = writer.stream_position()? as u32;
        self.name.write(writer)?;
        self.info.write(writer)?;
        self.core.write(writer)?;
        self.mbc.write(writer)?;
        self.end.write(writer)?;
        self.footer.write(writer)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        // moves the cursor to the end of the file
        // to read the footer, and then places the
        // the cursor in the start of the BESS data
        // according to the footer information
        reader.seek(SeekFrom::End(-8))?;
        self.footer.read(reader)?;
        reader.seek(SeekFrom::Start(self.footer.start_offset as u64))?;

        loop {
            // reads the block header information and then moves the
            // cursor back to the original position to be able to
            // re-read the block data
            let block = BessBlockHeader::from_data(reader)?;
            let offset = -((size_of::<u32>() * 2) as i64);
            reader.seek(SeekFrom::Current(offset))?;

            match block.magic.as_str() {
                "NAME" => self.name = BessName::from_data(reader)?,
                "INFO" => self.info = BessInfo::from_data(reader)?,
                "CORE" => self.core = BessCore::from_data(reader)?,
                "MBC " => self.mbc = BessMbc::from_data(reader)?,
                "END " => self.end = BessBlock::from_data(reader)?,
                _ => {
                    BessBlock::from_data(reader)?;
                }
            }

            if block.is_end() {
                break;
            }
        }

        Ok(())
    }
}

impl StateBox for BessState {
    fn from_gb(gb: &mut GameBoy, options: &FromGbOptions) -> Result<Box<Self>, Error> {
        Ok(Box::new(Self {
            footer: BessFooter::default(),
            name: *<BessName as StateBox>::from_gb(gb, options)?,
            info: BessInfo::from_gb(gb)?,
            core: BessCore::from_gb(gb)?,
            mbc: BessMbc::from_gb(gb)?,
            end: BessBlock::from_magic(String::from("END ")),
        }))
    }

    fn to_gb(&self, gb: &mut GameBoy, options: &ToGbOptions) -> Result<(), Error> {
        self.verify()?;
        StateBox::to_gb(&self.name, gb, options)?;
        self.info.to_gb(gb)?;
        self.core.to_gb(gb)?;
        self.mbc.to_gb(gb)?;
        Ok(())
    }
}

impl StateConfig for BessState {
    fn mode(&self) -> Result<GameBoyMode, Error> {
        match self.core.model.chars().next() {
            Some('G') => Ok(GameBoyMode::Dmg),
            Some('S') => Ok(GameBoyMode::Sgb),
            Some('C') => Ok(GameBoyMode::Cgb),
            None | Some(_) => Err(Error::InvalidData),
        }
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

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }

    pub fn is_end(&self) -> bool {
        self.magic == "END "
    }
}

impl Serialize for BessBlockHeader {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        write_bytes(writer, self.magic.as_bytes())?;
        write_u32(writer, self.size)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.magic = String::from_utf8(read_bytes(reader, 4)?)?;
        self.size = read_u32(reader)?;
        Ok(())
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

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }

    pub fn magic(&self) -> &String {
        &self.header.magic
    }

    pub fn is_end(&self) -> bool {
        self.header.is_end()
    }
}

impl Serialize for BessBlock {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;
        write_bytes(writer, &self.buffer)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;
        self.buffer.reserve_exact(self.header.size as usize);
        read_into(reader, &mut self.buffer)?;
        Ok(())
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
    fn load_buffer<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![0x00; self.size as usize];
        let position = reader.stream_position()?;
        reader.seek(SeekFrom::Start(self.offset as u64))?;
        read_into(reader, &mut buffer)?;
        reader.seek(SeekFrom::Start(position))?;
        Ok(buffer)
    }
}

impl Serialize for BessBuffer {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        write_u32(writer, self.size)?;
        write_u32(writer, self.offset)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.size = read_u32(reader)?;
        self.offset = read_u32(reader)?;
        self.buffer = self.load_buffer(reader)?;
        Ok(())
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
            return Err(Error::DataError(String::from("Invalid magic")));
        }
        Ok(())
    }
}

impl Serialize for BessFooter {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        write_u32(writer, self.start_offset)?;
        write_u32(writer, self.magic)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.start_offset = read_u32(reader)?;
        self.magic = read_u32(reader)?;
        Ok(())
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

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }

    pub fn format_name(name: &str, version: &str) -> String {
        format!("{name} v{version}")
    }

    pub fn build_name(&mut self, name: &str, version: &str) {
        self.name = Self::format_name(name, version);
    }
}

impl Serialize for BessName {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;
        write_bytes(writer, self.name.as_bytes())?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;
        self.name = String::from_utf8(read_bytes(reader, self.header.size as usize)?)?;
        Ok(())
    }
}

impl State for BessName {
    fn from_gb(_gb: &mut GameBoy) -> Result<Self, Error> {
        Ok(Self::new(Self::format_name(
            &Info::name(),
            &Info::version(),
        )))
    }

    fn to_gb(&self, _gb: &mut GameBoy) -> Result<(), Error> {
        Ok(())
    }
}

impl StateBox for BessName {
    fn from_gb(_gb: &mut GameBoy, options: &FromGbOptions) -> Result<Box<Self>, Error>
    where
        Self: Sized,
    {
        Ok(Box::new(Self::new(Self::format_name(
            &options.agent.clone().unwrap_or(Info::name()),
            &options.agent_version.clone().unwrap_or(Info::version()),
        ))))
    }

    fn to_gb(&self, _gb: &mut GameBoy, _options: &ToGbOptions) -> Result<(), Error> {
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

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
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
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;
        write_bytes(writer, &self.title)?;
        write_bytes(writer, &self.checksum)?;
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;
        read_into(reader, &mut self.title)?;
        read_into(reader, &mut self.checksum)?;
        Ok(())
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
            return Err(Error::DataError(format!(
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

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }

    pub fn verify(&self) -> Result<(), Error> {
        if self.header.magic != "CORE" {
            return Err(Error::DataError(String::from("Invalid magic")));
        }
        if self.major != 1 {
            return Err(Error::DataError(String::from("Invalid major version")));
        }
        if self.minor != 1 {
            return Err(Error::DataError(String::from("Invalid minor version")));
        }
        if self.oam.size != 0xa0 {
            return Err(Error::DataError(String::from("Invalid OAM size")));
        }
        if self.hram.size != 0x7f {
            return Err(Error::DataError(String::from("Invalid HRAM size")));
        }
        if (self.is_cgb() && self.background_palettes.size != 0x40)
            || (self.is_dmg() && self.background_palettes.size != 0x00)
        {
            return Err(Error::DataError(String::from(
                "Invalid background palettes size",
            )));
        }
        if (self.is_cgb() && self.object_palettes.size != 0x40)
            || (self.is_dmg() && self.object_palettes.size != 0x00)
        {
            return Err(Error::DataError(String::from(
                "Invalid object palettes size",
            )));
        }
        Ok(())
    }

    pub fn mode(&self) -> GameBoyMode {
        if self.is_dmg() {
            return GameBoyMode::Dmg;
        }
        if self.is_cgb() {
            return GameBoyMode::Cgb;
        }
        if self.is_sgb() {
            return GameBoyMode::Sgb;
        }
        GameBoyMode::Dmg
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

    fn is_sgb(&self) -> bool {
        if let Some(first_char) = self.model.chars().next() {
            return first_char == 'S';
        }
        false
    }
}

impl Serialize for BessCore {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;

        write_u16(writer, self.major)?;
        write_u16(writer, self.minor)?;

        write_bytes(writer, self.model.as_bytes())?;

        write_u16(writer, self.pc)?;
        write_u16(writer, self.af)?;
        write_u16(writer, self.bc)?;
        write_u16(writer, self.de)?;
        write_u16(writer, self.hl)?;
        write_u16(writer, self.sp)?;

        write_u8(writer, self.ime as u8)?;
        write_u8(writer, self.ie)?;
        write_u8(writer, self.execution_mode)?;
        write_u8(writer, self._padding)?;

        write_bytes(writer, &self.io_registers)?;

        self.ram.write(writer)?;
        self.vram.write(writer)?;
        self.mbc_ram.write(writer)?;
        self.oam.write(writer)?;
        self.hram.write(writer)?;
        self.background_palettes.write(writer)?;
        self.object_palettes.write(writer)?;

        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;

        self.major = read_u16(reader)?;
        self.minor = read_u16(reader)?;

        self.model = String::from_utf8(read_bytes(reader, 4)?)?;

        self.pc = read_u16(reader)?;
        self.af = read_u16(reader)?;
        self.bc = read_u16(reader)?;
        self.de = read_u16(reader)?;
        self.hl = read_u16(reader)?;
        self.sp = read_u16(reader)?;

        self.ime = read_u8(reader)? != 0;
        self.ie = read_u8(reader)?;
        self.execution_mode = read_u8(reader)?;
        self._padding = read_u8(reader)?;

        read_into(reader, &mut self.io_registers)?;

        self.ram.read(reader)?;
        self.vram.read(reader)?;
        self.mbc_ram.read(reader)?;
        self.oam.read(reader)?;
        self.hram.read(reader)?;
        self.background_palettes.read(reader)?;
        self.object_palettes.read(reader)?;

        Ok(())
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
            {
                // @TODO: These registers cannot be completely retrieved
                // and because of that some audio noise is played when loading state.
                // The loading of the registers should be done in a much
                // more manual way like SameBoy does here:
                // https://github.com/LIJI32/SameBoy/blob/7e6f1f866e89430adaa6be839aecc4a2ccabd69c/Core/save_state.c#L673
                disable_pedantic!();
                let io_registers = gb.mmu().read_many_raw(0xff00, 128).try_into().unwrap();
                enable_pedantic!();
                io_registers
            },
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
        disable_pedantic!();
        gb.mmu().write_many(0xff00, &self.io_registers);
        enable_pedantic!();

        gb.mmu().set_ram(self.ram.buffer.to_vec());
        gb.ppu().set_vram(&self.vram.buffer);
        gb.ppu().set_oam(&self.oam.buffer);
        gb.ppu().set_hram(&self.hram.buffer);
        gb.rom().set_ram_data(&self.mbc_ram.buffer);

        // disables a series of operations that would otherwise be
        // triggered by the writing of associated registers
        gb.dma().set_active_dma(false);
        gb.serial().set_transferring(false);

        // clears the PPU screen resetting the mode cycle clock
        // and other PPU cycle control structures
        gb.ppu().clear_screen(false);

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

            // need to disable HDMA transfer to avoid unwanted
            // DMA transfers when loading the state
            gb.dma().set_active_hdma(false);
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

    pub fn from_data<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut instance = Self::default();
        instance.read(reader)?;
        Ok(instance)
    }
}

impl Serialize for BessMbc {
    fn write<W: Write + Seek>(&mut self, writer: &mut W) -> Result<(), Error> {
        self.header.write(writer)?;
        for register in self.registers.iter() {
            write_u16(writer, register.address)?;
            write_u8(writer, register.value)?;
        }
        Ok(())
    }

    fn read<R: Read + Seek>(&mut self, reader: &mut R) -> Result<(), Error> {
        self.header.read(reader)?;
        for _ in 0..(self.header.size / 3) {
            let address = read_u16(reader)?;
            let value = read_u8(reader)?;
            self.registers.push(BessMbrRegister::new(address, value));
        }
        Ok(())
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
        options: Option<FromGbOptions>,
    ) -> Result<(), Error> {
        let mut file = File::create(file_path)
            .map_err(|_| Error::IoError(format!("Failed to create file: {file_path}")))?;
        let data = Self::save(gb, format, options)?;
        file.write_all(&data)
            .map_err(|_| Error::IoError(format!("Failed to write to file: {file_path}")))?;
        file.flush()
            .map_err(|_| Error::IoError(format!("Failed to flush file: {file_path}")))?;
        Ok(())
    }

    pub fn load_file(
        file_path: &str,
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
        options: Option<ToGbOptions>,
    ) -> Result<(), Error> {
        let mut file = File::open(file_path)
            .map_err(|_| Error::IoError(format!("Failed to open file: {file_path}")))?;
        let mut data = vec![];
        file.read_to_end(&mut data)
            .map_err(|_| Error::IoError(format!("Failed to read from file: {file_path}")))?;
        Self::load(&data, gb, format, options)?;
        Ok(())
    }
}

impl StateManager {
    pub fn save(
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
        options: Option<FromGbOptions>,
    ) -> Result<Vec<u8>, Error> {
        let options = options.unwrap_or_default();
        let mut data = Cursor::new(vec![]);
        match format {
            Some(SaveStateFormat::Bosc) | None => {
                let mut state = BoscState::from_gb(gb, &options)?;
                state.write(&mut data)?;
            }
            Some(SaveStateFormat::Bos) => {
                let mut state = BosState::from_gb(gb, &options)?;
                state.write(&mut data)?;
            }
            Some(SaveStateFormat::Bess) => {
                let mut state = BessState::from_gb(gb, &options)?;
                state.write(&mut data)?;
            }
        }
        Ok(data.into_inner())
    }

    pub fn load(
        data: &[u8],
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
        options: Option<ToGbOptions>,
    ) -> Result<(), Error> {
        let options = options.unwrap_or_default();
        let data = &mut Cursor::new(data.to_vec());
        let format = match format {
            Some(format) => format,
            None => {
                if BoscState::is_bosc(data)? {
                    SaveStateFormat::Bosc
                } else if BosState::is_bos(data)? {
                    SaveStateFormat::Bos
                } else if BessState::is_bess(data)? {
                    SaveStateFormat::Bess
                } else {
                    return Err(Error::InvalidParameter(String::from(
                        "Unknown save state file format",
                    )));
                }
            }
        };
        match format {
            SaveStateFormat::Bosc => {
                let mut state = BoscState::default();
                Self::load_inner(&mut state, data, gb, &options)?;
            }
            SaveStateFormat::Bos => {
                let mut state = BosState::default();
                Self::load_inner(&mut state, data, gb, &options)?;
            }
            SaveStateFormat::Bess => {
                let mut state = BessState::default();
                Self::load_inner(&mut state, data, gb, &options)?;
            }
        }
        Ok(())
    }

    pub fn read_bos_auto(data: &[u8]) -> Result<BosState, Error> {
        match Self::format(data)? {
            SaveStateFormat::Bosc => {
                let mut state = BoscState::default();
                let data = &mut Cursor::new(data.to_vec());
                state.read(data)?;
                Ok(state.bos)
            }
            SaveStateFormat::Bos => {
                let mut state = BosState::default();
                let data = &mut Cursor::new(data.to_vec());
                state.read(data)?;
                Ok(state)
            }
            SaveStateFormat::Bess => Err(Error::InvalidParameter(String::from(
                "Incompatible save state file format (BESS)",
            ))),
        }
    }

    pub fn read_bosc(data: &[u8]) -> Result<BoscState, Error> {
        let data = &mut Cursor::new(data.to_vec());
        let mut state = BoscState::default();
        state.read(data)?;
        Ok(state)
    }

    pub fn read_bos(data: &[u8]) -> Result<BosState, Error> {
        let data = &mut Cursor::new(data.to_vec());
        let mut state = BosState::default();
        state.read(data)?;
        Ok(state)
    }

    pub fn read_bess(data: &[u8]) -> Result<BessState, Error> {
        let data = &mut Cursor::new(data.to_vec());
        let mut state = BessState::default();
        state.read(data)?;
        Ok(state)
    }

    pub fn format(data: &[u8]) -> Result<SaveStateFormat, Error> {
        let data = &mut Cursor::new(data.to_vec());
        if BoscState::is_bosc(data)? {
            Ok(SaveStateFormat::Bosc)
        } else if BosState::is_bos(data)? {
            Ok(SaveStateFormat::Bos)
        } else if BessState::is_bess(data)? {
            Ok(SaveStateFormat::Bess)
        } else {
            Err(Error::InvalidData)
        }
    }

    /// Validates the provided state data and runs a series of simple
    /// validations according to the provided params.
    pub fn validate(data: &[u8], title: Option<String>) -> Result<(), Error> {
        match Self::format(data)? {
            SaveStateFormat::Bosc | SaveStateFormat::Bos => {
                let state = Self::read_bos_auto(data)?;
                if let Some(title) = title {
                    if state.title()? != title {
                        return Err(Error::InvalidData);
                    }
                }
            }
            SaveStateFormat::Bess => {
                let state = Self::read_bess(data)?;
                if let Some(title) = title {
                    if state.title()? != title {
                        return Err(Error::InvalidData);
                    }
                }
            }
        }
        Ok(())
    }

    /// Obtains the thumbnail of the save state file, this thumbnail is
    /// stored in raw RGB format.
    ///
    /// This operation is currently only supported for the BOS format.
    pub fn thumbnail(data: &[u8], format: Option<SaveStateFormat>) -> Result<Vec<u8>, Error> {
        let format = match format {
            Some(format) => format,
            None => Self::format(data)?,
        };
        match format {
            SaveStateFormat::Bosc => {
                let mut state = BoscState::default();
                let data = &mut Cursor::new(data.to_vec());
                state.read(data)?;
                Ok(state
                    .bos
                    .image_buffer
                    .ok_or(Error::InvalidData)?
                    .image
                    .to_vec())
            }
            SaveStateFormat::Bos => {
                let mut state = BosState::default();
                let data = &mut Cursor::new(data.to_vec());
                state.read(data)?;
                Ok(state.image_buffer.ok_or(Error::InvalidData)?.image.to_vec())
            }
            SaveStateFormat::Bess => Err(Error::InvalidParameter(String::from(
                "Format foes not support thumbnail",
            ))),
        }
    }

    fn load_inner<T: Serialize + StateBox + StateConfig + Default, R: Read + Seek>(
        state: &mut T,
        reader: &mut R,
        gb: &mut GameBoy,
        options: &ToGbOptions,
    ) -> Result<(), Error> {
        state.read(reader)?;

        // in case the hardware model in the (saved) state is
        // different from the current hardware model, we need
        // to set the hardware model
        if state.mode()? != gb.mode() {
            gb.set_mode(state.mode()?);
        }

        // reload the Game Boy machine to make sure we're in
        // a clean state before loading the state
        if options.reload {
            gb.reload();
        }

        state.to_gb(gb, options)?;

        Ok(())
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl StateManager {
    pub fn save_wa(
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
        options: Option<FromGbOptions>,
    ) -> Result<Vec<u8>, String> {
        Ok(Self::save(gb, format, options)?)
    }

    pub fn load_wa(
        data: &[u8],
        gb: &mut GameBoy,
        format: Option<SaveStateFormat>,
        options: Option<ToGbOptions>,
    ) -> Result<(), String> {
        Ok(Self::load(data, gb, format, options)?)
    }

    pub fn read_bos_auto_wa(data: &[u8]) -> Result<BosState, String> {
        Ok(Self::read_bos_auto(data)?)
    }

    pub fn read_bosc_wa(data: &[u8]) -> Result<BoscState, String> {
        Ok(Self::read_bosc(data)?)
    }

    pub fn read_bos_wa(data: &[u8]) -> Result<BosState, String> {
        Ok(Self::read_bos(data)?)
    }

    pub fn read_bess_wa(data: &[u8]) -> Result<BessState, String> {
        Ok(Self::read_bess(data)?)
    }

    pub fn format_wa(data: &[u8]) -> Result<SaveStateFormat, String> {
        Ok(Self::format(data)?)
    }

    pub fn format_str_wa(data: &[u8]) -> Result<String, String> {
        Ok(Self::format(data)?.to_string())
    }

    pub fn validate_wa(data: &[u8], title: Option<String>) -> Result<(), String> {
        Ok(Self::validate(data, title)?)
    }

    pub fn thumbnail_wa(data: &[u8], format: Option<SaveStateFormat>) -> Result<Vec<u8>, String> {
        Ok(Self::thumbnail(data, format)?)
    }
}

#[cfg(test)]
mod tests {
    use boytacean_encoding::zippy::{decode_zippy, encode_zippy};

    use crate::{
        gb::GameBoy,
        state::{FromGbOptions, State},
    };

    use super::{BessCore, SaveStateFormat, StateManager};

    #[test]
    fn test_bess_core() {
        let mut gb = GameBoy::default();
        gb.load(true).unwrap();
        gb.load_rom_file("res/roms/test/firstwhite.gb", None)
            .unwrap();
        let bess_core = BessCore::from_gb(&mut gb).unwrap();
        assert_eq!(bess_core.model, "GDB ");
        assert_eq!(bess_core.pc, 0x0000);
        assert_eq!(bess_core.af, 0x0000);
        assert_eq!(bess_core.bc, 0x0000);
        assert_eq!(bess_core.de, 0x0000);
        assert_eq!(bess_core.hl, 0x0000);
        assert_eq!(bess_core.sp, 0x0000);
        assert!(!bess_core.ime);
        assert_eq!(bess_core.ie, 0x00);
        assert_eq!(bess_core.execution_mode, 0);
        assert_eq!(bess_core.io_registers.len(), 128);
        assert_eq!(
            bess_core.io_registers,
            [
                63, 0, 0, 255, 0, 0, 0, 248, 255, 255, 255, 255, 255, 255, 255, 224, 128, 0, 0,
                255, 56, 255, 0, 0, 255, 56, 127, 255, 159, 255, 56, 255, 0, 0, 0, 63, 0, 0, 240,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 134, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 126, 255, 254, 0, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255, 249, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255
            ]
        );
        assert_eq!(bess_core.ram.size, 0x2000);
        assert_eq!(bess_core.vram.size, 0x2000);
        assert_eq!(bess_core.mbc_ram.size, 0x2000);
        assert_eq!(bess_core.oam.size, 0x00a0);
        assert_eq!(bess_core.hram.size, 0x007f);
        assert_eq!(bess_core.background_palettes.size, 0x0000);
        assert_eq!(bess_core.object_palettes.size, 0x0000);
    }

    #[test]
    fn test_load_bosc() {
        let mut gb = GameBoy::default();
        gb.load(true).unwrap();
        gb.load_rom_file("res/roms/test/firstwhite.gb", None)
            .unwrap();
        let data = StateManager::save(&mut gb, Some(SaveStateFormat::Bosc), None).unwrap();
        StateManager::load(&data, &mut gb, Some(SaveStateFormat::Bosc), None).unwrap();
        StateManager::load(&data, &mut gb, None, None).unwrap();
    }

    #[test]
    fn test_load_bos() {
        let mut gb = GameBoy::default();
        gb.load(true).unwrap();
        gb.load_rom_file("res/roms/test/firstwhite.gb", None)
            .unwrap();
        let data = StateManager::save(&mut gb, Some(SaveStateFormat::Bos), None).unwrap();
        StateManager::load(&data, &mut gb, Some(SaveStateFormat::Bos), None).unwrap();
        StateManager::load(&data, &mut gb, None, None).unwrap();
    }

    #[test]
    fn test_load_bess() {
        let mut gb = GameBoy::default();
        gb.load(true).unwrap();
        gb.load_rom_file("res/roms/test/firstwhite.gb", None)
            .unwrap();
        let data = StateManager::save(&mut gb, Some(SaveStateFormat::Bess), None).unwrap();
        StateManager::load(&data, &mut gb, Some(SaveStateFormat::Bess), None).unwrap();
        StateManager::load(&data, &mut gb, None, None).unwrap();
    }

    #[test]
    fn test_bos_agent_version() {
        let mut gb = GameBoy::default();
        gb.load(true).unwrap();
        gb.load_rom_file("res/roms/test/firstwhite.gb", None)
            .unwrap();
        let data = StateManager::save(
            &mut gb,
            Some(SaveStateFormat::Bos),
            Some(FromGbOptions {
                agent: Some(String::from("test-agent")),
                agent_version: Some(String::from("1.2.3")),
                ..Default::default()
            }),
        )
        .unwrap();
        let loaded_state = StateManager::read_bos(&data).unwrap();
        let info = loaded_state.info.unwrap();
        assert_eq!(info.agent, "test-agent");
        assert_eq!(info.agent_version, "1.2.3");
    }

    #[test]
    fn test_bess_agent_version() {
        let mut gb = GameBoy::default();
        gb.load(true).unwrap();
        gb.load_rom_file("res/roms/test/firstwhite.gb", None)
            .unwrap();
        let data = StateManager::save(
            &mut gb,
            Some(SaveStateFormat::Bess),
            Some(FromGbOptions {
                agent: Some(String::from("TestAgent")),
                agent_version: Some(String::from("1.2.3")),
                ..Default::default()
            }),
        )
        .unwrap();
        let loaded_state = StateManager::read_bess(&data).unwrap();
        assert_eq!(loaded_state.name.name, "TestAgent v1.2.3");
    }

    #[test]
    fn test_compression() {
        let mut gb = GameBoy::default();
        gb.load(true).unwrap();
        gb.load_rom_file("res/roms/test/firstwhite.gb", None)
            .unwrap();
        gb.step_to(0x0100);
        let data = StateManager::save(
            &mut gb,
            Some(SaveStateFormat::Bess),
            Some(FromGbOptions {
                agent_version: Some(String::from("0.0.0")),
                ..Default::default()
            }),
        )
        .unwrap();
        let encoded = encode_zippy(&data, None, None).unwrap();
        let decoded = decode_zippy(&encoded, None).unwrap();
        assert_eq!(data, decoded);
        assert_eq!(encoded.len(), 841);
        assert_eq!(decoded.len(), 25153);
    }
}
