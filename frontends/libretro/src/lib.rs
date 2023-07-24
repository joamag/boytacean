#![allow(clippy::uninlined_format_args)]

use std::{
    collections::HashMap,
    ffi::CStr,
    fmt::{self, Display, Formatter},
    os::raw::{c_char, c_float, c_uint, c_void},
    slice::from_raw_parts,
};

use boytacean::{
    gb::{AudioProvider, GameBoy},
    pad::PadKey,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_RGB155_SIZE, RGB1555_SIZE},
    rom::Cartridge,
};

const RETRO_API_VERSION: u32 = 1;
const REGION_NTSC: u32 = 0;

//const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 10;

//const RETRO_PIXEL_FORMAT_0RGB1555: usize = 0;
//const RETRO_PIXEL_FORMAT_XRGB8888: usize = 1;
//const RETRO_PIXEL_FORMAT_RGB565: usize = 2;

//const RETRO_MEMORY_SAVE_RAM: u32 = 0;
//const RETRO_MEMORY_SYSTEM_RAM: u32 = 0;

const RETRO_DEVICE_JOYPAD: usize = 1;

static mut EMULATOR: Option<GameBoy> = None;
static mut KEY_STATES: Option<HashMap<RetroJoypad, bool>> = None;
static mut FRAME_BUFFER: [u8; FRAME_BUFFER_RGB155_SIZE] = [0x00; FRAME_BUFFER_RGB155_SIZE];

static mut ENVIRONMENT_CALLBACK: Option<extern "C" fn(u32, *const c_void) -> bool> = None;
static mut VIDEO_REFRESH_CALLBACK: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)> = None;
static mut AUDIO_SAMPLE_CALLBACK: Option<extern "C" fn(i16, i16)> = None;
static mut AUDIO_SAMPLE_BATCH_CALLBACK: Option<extern "C" fn(*const i16, usize)> = None;
static mut INPUT_POLL_CALLBACK: Option<extern "C" fn()> = None;
static mut INPUT_STATE_CALLBACK: Option<
    extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16,
> = None;

const RETRO_DEVICE_ID_JOYPAD_B: isize = 0;
const RETRO_DEVICE_ID_JOYPAD_Y: isize = 1;
const RETRO_DEVICE_ID_JOYPAD_SELECT: isize = 2;
const RETRO_DEVICE_ID_JOYPAD_START: isize = 3;
const RETRO_DEVICE_ID_JOYPAD_UP: isize = 4;
const RETRO_DEVICE_ID_JOYPAD_DOWN: isize = 5;
const RETRO_DEVICE_ID_JOYPAD_LEFT: isize = 6;
const RETRO_DEVICE_ID_JOYPAD_RIGHT: isize = 7;
const RETRO_DEVICE_ID_JOYPAD_A: isize = 8;
const RETRO_DEVICE_ID_JOYPAD_X: isize = 9;
const RETRO_DEVICE_ID_JOYPAD_L: isize = 10;
const RETRO_DEVICE_ID_JOYPAD_R: isize = 11;
const RETRO_DEVICE_ID_JOYPAD_L2: isize = 12;
const RETRO_DEVICE_ID_JOYPAD_R2: isize = 13;
const RETRO_DEVICE_ID_JOYPAD_L3: isize = 14;
const RETRO_DEVICE_ID_JOYPAD_R3: isize = 15;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetroJoypad {
    RetroDeviceIdJoypadY = RETRO_DEVICE_ID_JOYPAD_B,
    RetroDeviceIdJoypadB = RETRO_DEVICE_ID_JOYPAD_Y,
    RetroDeviceIdJoypadSelect = RETRO_DEVICE_ID_JOYPAD_SELECT,
    RetroDeviceIdJoypadStart = RETRO_DEVICE_ID_JOYPAD_START,
    RetroDeviceIdJoypadUp = RETRO_DEVICE_ID_JOYPAD_UP,
    RetroDeviceIdJoypadDown = RETRO_DEVICE_ID_JOYPAD_DOWN,
    RetroDeviceIdJoypadLeft = RETRO_DEVICE_ID_JOYPAD_LEFT,
    RetroDeviceIdJoypadRight = RETRO_DEVICE_ID_JOYPAD_RIGHT,
    RetroDeviceIdJoypadA = RETRO_DEVICE_ID_JOYPAD_A,
    RetroDeviceIdJoypadX = RETRO_DEVICE_ID_JOYPAD_X,
    RetroDeviceIdJoypadL = RETRO_DEVICE_ID_JOYPAD_L,
    RetroDeviceIdJoypadR = RETRO_DEVICE_ID_JOYPAD_R,
    RetroDeviceIdJoypadL2 = RETRO_DEVICE_ID_JOYPAD_L2,
    RetroDeviceIdJoypadR2 = RETRO_DEVICE_ID_JOYPAD_R2,
    RetroDeviceIdJoypadL3 = RETRO_DEVICE_ID_JOYPAD_L3,
    RetroDeviceIdJoypadR3 = RETRO_DEVICE_ID_JOYPAD_R3,
}

impl RetroJoypad {
    pub fn description(&self) -> &'static str {
        match self {
            RetroJoypad::RetroDeviceIdJoypadY => "Y",
            RetroJoypad::RetroDeviceIdJoypadB => "B",
            RetroJoypad::RetroDeviceIdJoypadSelect => "Select",
            RetroJoypad::RetroDeviceIdJoypadStart => "Start",
            RetroJoypad::RetroDeviceIdJoypadUp => "Up",
            RetroJoypad::RetroDeviceIdJoypadDown => "Down",
            RetroJoypad::RetroDeviceIdJoypadLeft => "Left",
            RetroJoypad::RetroDeviceIdJoypadRight => "Right",
            RetroJoypad::RetroDeviceIdJoypadA => "A",
            RetroJoypad::RetroDeviceIdJoypadX => "X",
            RetroJoypad::RetroDeviceIdJoypadL => "L",
            RetroJoypad::RetroDeviceIdJoypadR => "R",
            RetroJoypad::RetroDeviceIdJoypadL2 => "L2",
            RetroJoypad::RetroDeviceIdJoypadR2 => "R2",
            RetroJoypad::RetroDeviceIdJoypadL3 => "L3",
            RetroJoypad::RetroDeviceIdJoypadR3 => "R3",
        }
    }
}

impl Display for RetroJoypad {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

const KEYS: [RetroJoypad; 8] = [
    RetroJoypad::RetroDeviceIdJoypadUp,
    RetroJoypad::RetroDeviceIdJoypadDown,
    RetroJoypad::RetroDeviceIdJoypadLeft,
    RetroJoypad::RetroDeviceIdJoypadRight,
    RetroJoypad::RetroDeviceIdJoypadStart,
    RetroJoypad::RetroDeviceIdJoypadSelect,
    RetroJoypad::RetroDeviceIdJoypadA,
    RetroJoypad::RetroDeviceIdJoypadB,
];

#[repr(C)]
pub struct RetroGameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
pub struct RetroSystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
pub struct RetroGameGeometry {
    pub base_width: c_uint,
    pub base_height: c_uint,
    pub max_width: c_uint,
    pub max_height: c_uint,
    pub aspect_ratio: c_float,
}

#[repr(C)]
pub struct RetroSystemAvInfo {
    geometry: RetroGameGeometry,
    timing: RetroSystemTiming,
}

#[repr(C)]
pub struct RetroSystemTiming {
    fps: f64,
    sample_rate: f64,
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_init() {
    println!("retro_init()");
    unsafe {
        EMULATOR = Some(GameBoy::new(None));
        KEY_STATES = Some(HashMap::new());
    }
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    println!("retro_deinit()");
}

#[no_mangle]
pub extern "C" fn retro_reset() {
    println!("retro_reset()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    emulator.reload();
}

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    println!("retro_api_version()");
    RETRO_API_VERSION
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut RetroSystemInfo) {
    println!("retro_get_system_info()");
    unsafe {
        (*info).library_name = "Boytacean\0".as_ptr() as *const c_char;
        (*info).library_version = "v0.9.6\0".as_ptr() as *const c_char;
        (*info).valid_extensions = "gb|gbc\0".as_ptr() as *const c_char;
        (*info).need_fullpath = false;
        (*info).block_extract = false;
    }
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut RetroSystemAvInfo) {
    println!("retro_get_system_av_info()");
    unsafe {
        (*info).geometry.base_width = DISPLAY_WIDTH as u32;
        (*info).geometry.base_height = DISPLAY_HEIGHT as u32;
        (*info).geometry.max_width = DISPLAY_WIDTH as u32 * 64;
        (*info).geometry.max_height = DISPLAY_HEIGHT as u32 * 64;
        (*info).geometry.aspect_ratio = DISPLAY_WIDTH as f32 / DISPLAY_HEIGHT as f32;
        (*info).timing.fps = GameBoy::VISUAL_FREQ as f64;
        (*info).timing.sample_rate = EMULATOR.as_ref().unwrap().audio_sampling_rate() as f64;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_environment(
    callback: Option<extern "C" fn(u32, *const c_void) -> bool>,
) {
    println!("retro_set_environment()");
    unsafe {
        ENVIRONMENT_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_video_refresh(
    callback: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)>,
) {
    println!("retro_set_video_refresh()");
    unsafe {
        VIDEO_REFRESH_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample(callback: Option<extern "C" fn(i16, i16)>) {
    println!("retro_set_audio_sample()");
    unsafe {
        AUDIO_SAMPLE_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(callback: Option<extern "C" fn(*const i16, usize)>) {
    println!("retro_set_audio_sample_batch()");
    unsafe {
        AUDIO_SAMPLE_BATCH_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_input_poll(callback: Option<extern "C" fn()>) {
    println!("retro_set_input_poll()");
    unsafe {
        INPUT_POLL_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_input_state(
    callback: Option<extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16>,
) {
    println!("retro_set_input_state()");
    unsafe {
        INPUT_STATE_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _system: u32,
    _info: *const RetroGameInfo,
    _num_info: usize,
) -> bool {
    println!("retro_load_game_special()");
    false
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device() {
    println!("retro_set_controller_port_device()");
}

#[no_mangle]
pub extern "C" fn retro_run() {
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    let sample_batch_cb = unsafe { AUDIO_SAMPLE_BATCH_CALLBACK.as_ref().unwrap() };
    let channels = emulator.audio_channels();

    let mut counter_cycles = 0_u32;
    let cycle_limit = (GameBoy::CPU_FREQ as f32 * emulator.multiplier() as f32
        / GameBoy::VISUAL_FREQ)
        .round() as u32;

    loop {
        // limits the number of ticks to the typical number
        // of cycles expected for the current logic cycle
        if counter_cycles >= cycle_limit {
            //pending_cycles = counter_cycles - cycle_limit;
            break;
        }

        // runs the Game Boy clock, this operation should
        // include the advance of both the CPU, PPU, APU
        // and any other frequency based component of the system
        counter_cycles += emulator.clock() as u32;

        // obtains the audio buffer reference and queues it
        // in a batch manner using the audio callback at the
        // the end of the operation clears the buffer
        let audio_buffer = emulator
            .audio_buffer()
            .iter()
            .map(|v| *v as i16 * 256)
            .collect::<Vec<i16>>();

        sample_batch_cb(
            audio_buffer.as_ptr(),
            audio_buffer.len() / channels as usize,
        );
        emulator.clear_audio_buffer();
    }

    unsafe {
        INPUT_POLL_CALLBACK.as_ref().unwrap()();
        let key_states = KEY_STATES.as_mut().unwrap();
        for key in KEYS {
            let key_pad = retro_key_to_pad(key).unwrap();
            let current = INPUT_STATE_CALLBACK.as_ref().unwrap()(
                0,
                RETRO_DEVICE_JOYPAD as u32,
                0,
                key as u32,
            ) > 0;
            let previous = key_states.get(&key).unwrap_or(&false);
            if current != *previous {
                if current {
                    emulator.key_press(key_pad);
                } else {
                    emulator.key_lift(key_pad);
                }
            }
            key_states.insert(key, current);
        }
    }

    let frame_buffer = emulator.frame_buffer_rgb1555();
    unsafe {
        FRAME_BUFFER.copy_from_slice(&frame_buffer);
        VIDEO_REFRESH_CALLBACK.unwrap()(
            FRAME_BUFFER.as_ptr(),
            DISPLAY_WIDTH as u32,
            DISPLAY_HEIGHT as u32,
            DISPLAY_WIDTH * RGB1555_SIZE,
        );
    }
}

#[no_mangle]
pub extern "C" fn retro_get_region() -> u32 {
    println!("retro_get_region()");
    REGION_NTSC
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_load_game(game: *const RetroGameInfo) -> bool {
    println!("retro_load_game()");
    unsafe {
        let instance = EMULATOR.as_mut().unwrap();
        let data_buffer = from_raw_parts((*game).data as *const u8, (*game).size);
        let file_path_c = CStr::from_ptr((*game).path);
        let file_path = file_path_c.to_str().unwrap();
        let mode = Cartridge::from_file(file_path).gb_mode();
        instance.set_mode(mode);
        instance.reset();
        instance.load(true);
        instance.load_rom(data_buffer, None);
    }
    true
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    println!("retro_unload_game()");
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(_memory_id: u32) -> *mut c_void {
    println!("retro_get_memory_data()");
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(_memory_id: u32) -> usize {
    println!("retro_get_memory_size()");
    0
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() {
    println!("retro_serialize_size()");
}

#[no_mangle]
pub extern "C" fn retro_serialize() {
    println!("retro_serialize()");
}

#[no_mangle]
pub extern "C" fn retro_unserialize() {
    println!("retro_unserialize()");
}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {
    println!("retro_cheat_reset()");
}

#[no_mangle]
pub extern "C" fn retro_cheat_set() {
    println!("retro_cheat_set()");
}

fn retro_key_to_pad(retro_key: RetroJoypad) -> Option<PadKey> {
    match retro_key {
        RetroJoypad::RetroDeviceIdJoypadUp => Some(PadKey::Up),
        RetroJoypad::RetroDeviceIdJoypadDown => Some(PadKey::Down),
        RetroJoypad::RetroDeviceIdJoypadLeft => Some(PadKey::Left),
        RetroJoypad::RetroDeviceIdJoypadRight => Some(PadKey::Right),
        RetroJoypad::RetroDeviceIdJoypadStart => Some(PadKey::Start),
        RetroJoypad::RetroDeviceIdJoypadSelect => Some(PadKey::Select),
        RetroJoypad::RetroDeviceIdJoypadA => Some(PadKey::A),
        RetroJoypad::RetroDeviceIdJoypadB => Some(PadKey::B),
        _ => None,
    }
}
