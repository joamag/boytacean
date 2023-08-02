#![allow(clippy::uninlined_format_args)]

pub mod consts;

use std::{
    collections::HashMap,
    ffi::CStr,
    fmt::{self, Display, Formatter},
    os::raw::{c_char, c_float, c_uint, c_void},
    slice::from_raw_parts,
};

use boytacean::{
    debugln,
    gb::{AudioProvider, GameBoy},
    pad::PadKey,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_RGB155_SIZE, RGB1555_SIZE},
    rom::Cartridge,
};
use consts::{
    RETRO_DEVICE_ID_JOYPAD_A, RETRO_DEVICE_ID_JOYPAD_B, RETRO_DEVICE_ID_JOYPAD_DOWN,
    RETRO_DEVICE_ID_JOYPAD_L, RETRO_DEVICE_ID_JOYPAD_L2, RETRO_DEVICE_ID_JOYPAD_L3,
    RETRO_DEVICE_ID_JOYPAD_LEFT, RETRO_DEVICE_ID_JOYPAD_R, RETRO_DEVICE_ID_JOYPAD_R2,
    RETRO_DEVICE_ID_JOYPAD_R3, RETRO_DEVICE_ID_JOYPAD_RIGHT, RETRO_DEVICE_ID_JOYPAD_SELECT,
    RETRO_DEVICE_ID_JOYPAD_START, RETRO_DEVICE_ID_JOYPAD_UP, RETRO_DEVICE_ID_JOYPAD_X,
    RETRO_DEVICE_ID_JOYPAD_Y, RETRO_DEVICE_JOYPAD,
};

use crate::consts::{REGION_NTSC, RETRO_API_VERSION};

static mut EMULATOR: Option<GameBoy> = None;
static mut KEY_STATES: Option<HashMap<RetroJoypad, bool>> = None;
static mut FRAME_BUFFER: [u8; FRAME_BUFFER_RGB155_SIZE] = [0x00; FRAME_BUFFER_RGB155_SIZE];
static mut PENDING_CYCLES: u32 = 0_u32;

static mut ENVIRONMENT_CALLBACK: Option<extern "C" fn(u32, *const c_void) -> bool> = None;
static mut VIDEO_REFRESH_CALLBACK: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)> = None;
static mut AUDIO_SAMPLE_CALLBACK: Option<extern "C" fn(i16, i16)> = None;
static mut AUDIO_SAMPLE_BATCH_CALLBACK: Option<extern "C" fn(*const i16, usize)> = None;
static mut INPUT_POLL_CALLBACK: Option<extern "C" fn()> = None;
static mut INPUT_STATE_CALLBACK: Option<
    extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16,
> = None;

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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetroJoypad {
    RetroDeviceIdJoypadB = RETRO_DEVICE_ID_JOYPAD_B,
    RetroDeviceIdJoypadY = RETRO_DEVICE_ID_JOYPAD_Y,
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

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    debugln!("retro_api_version()");
    RETRO_API_VERSION
}

#[no_mangle]
pub extern "C" fn retro_init() {
    debugln!("retro_init()");
    unsafe {
        EMULATOR = Some(GameBoy::new(None));
        KEY_STATES = Some(HashMap::new());
    }
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    debugln!("retro_deinit()");
}

#[no_mangle]
pub extern "C" fn retro_reset() {
    debugln!("retro_reset()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    emulator.reload();
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut RetroSystemInfo) {
    debugln!("retro_get_system_info()");
    (*info).library_name = "Boytacean\0".as_ptr() as *const c_char;
    (*info).library_version = "v0.9.13\0".as_ptr() as *const c_char;
    (*info).valid_extensions = "gb|gbc\0".as_ptr() as *const c_char;
    (*info).need_fullpath = false;
    (*info).block_extract = false;
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut RetroSystemAvInfo) {
    debugln!("retro_get_system_av_info()");
    (*info).geometry.base_width = DISPLAY_WIDTH as u32;
    (*info).geometry.base_height = DISPLAY_HEIGHT as u32;
    (*info).geometry.max_width = DISPLAY_WIDTH as u32 * 64;
    (*info).geometry.max_height = DISPLAY_HEIGHT as u32 * 64;
    (*info).geometry.aspect_ratio = DISPLAY_WIDTH as f32 / DISPLAY_HEIGHT as f32;
    (*info).timing.fps = GameBoy::VISUAL_FREQ as f64;
    (*info).timing.sample_rate = EMULATOR.as_ref().unwrap().audio_sampling_rate() as f64;
}

#[no_mangle]
pub extern "C" fn retro_set_environment(
    callback: Option<extern "C" fn(u32, *const c_void) -> bool>,
) {
    debugln!("retro_set_environment()");
    unsafe {
        ENVIRONMENT_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device() {
    debugln!("retro_set_controller_port_device()");
}

#[no_mangle]
pub extern "C" fn retro_run() {
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    let video_refresh_cb = unsafe { VIDEO_REFRESH_CALLBACK.as_ref().unwrap() };
    let sample_batch_cb = unsafe { AUDIO_SAMPLE_BATCH_CALLBACK.as_ref().unwrap() };
    let input_poll_cb = unsafe { INPUT_POLL_CALLBACK.as_ref().unwrap() };
    let input_state_cb = unsafe { INPUT_STATE_CALLBACK.as_ref().unwrap() };
    let key_states = unsafe { KEY_STATES.as_mut().unwrap() };
    let channels = emulator.audio_channels();

    let mut last_frame = emulator.ppu_frame();

    let mut counter_cycles = unsafe { PENDING_CYCLES };
    let cycle_limit = (GameBoy::CPU_FREQ as f32 * emulator.multiplier() as f32
        / GameBoy::VISUAL_FREQ)
        .round() as u32;

    loop {
        // limits the number of ticks to the typical number
        // of cycles expected for the current logic cycle
        if counter_cycles >= cycle_limit {
            unsafe { PENDING_CYCLES = counter_cycles - cycle_limit };
            break;
        }

        // runs the Game Boy clock, this operation should
        // include the advance of both the CPU, PPU, APU
        // and any other frequency based component of the system
        counter_cycles += emulator.clock() as u32;

        // in case a new frame is available in the emulator
        // then the frame must be pushed into display
        if emulator.ppu_frame() != last_frame {
            let frame_buffer = emulator.frame_buffer_rgb1555();
            unsafe {
                FRAME_BUFFER.copy_from_slice(&frame_buffer);
                video_refresh_cb(
                    FRAME_BUFFER.as_ptr(),
                    DISPLAY_WIDTH as u32,
                    DISPLAY_HEIGHT as u32,
                    DISPLAY_WIDTH * RGB1555_SIZE,
                );
            }

            // obtains the index of the current PPU frame, this value
            // is going to be used to detect for new frame presence
            last_frame = emulator.ppu_frame();
        }

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

    input_poll_cb();

    for key in KEYS {
        let key_pad = retro_key_to_pad(key).unwrap();
        let current = input_state_cb(0, RETRO_DEVICE_JOYPAD as u32, 0, key as u32) > 0;
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

#[no_mangle]
pub extern "C" fn retro_get_region() -> u32 {
    debugln!("retro_get_region()");
    REGION_NTSC
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_load_game(game: *const RetroGameInfo) -> bool {
    debugln!("retro_load_game()");
    let instance = EMULATOR.as_mut().unwrap();
    let data_buffer = from_raw_parts((*game).data as *const u8, (*game).size);
    let rom = Cartridge::from_data(data_buffer);
    let mode = rom.gb_mode();
    instance.set_mode(mode);
    instance.reset();
    instance.load(true);
    instance.load_cartridge(rom);
    true
}

#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _system: u32,
    _info: *const RetroGameInfo,
    _num_info: usize,
) -> bool {
    debugln!("retro_load_game_special()");
    false
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    debugln!("retro_unload_game()");
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(_memory_id: u32) -> *mut c_void {
    debugln!("retro_get_memory_data()");
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(_memory_id: u32) -> usize {
    debugln!("retro_get_memory_size()");
    0
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() {
    debugln!("retro_serialize_size()");
}

#[no_mangle]
pub extern "C" fn retro_serialize() {
    debugln!("retro_serialize()");
}

#[no_mangle]
pub extern "C" fn retro_unserialize() {
    debugln!("retro_unserialize()");
}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {
    debugln!("retro_cheat_reset()");
    println!("retro_cheat_reset()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    emulator.reset_cheats();
}

/// # Safety
///
/// This function should not be called only within Lib Retro context.
#[no_mangle]
pub unsafe extern "C" fn retro_cheat_set(_index: c_uint, enabled: bool, code: *const c_char) {
    debugln!("retro_cheat_set()");
    // we'll just ignore cheats that are not enabled, Boytacean
    // does not support pre-loading cheats
    if !enabled {
        return;
    }
    let emulator = EMULATOR.as_mut().unwrap();
    let code_c = CStr::from_ptr(code);
    let code_s = code_c.to_string_lossy().into_owned();
    emulator.add_cheat_code(&code_s).unwrap();
}

#[no_mangle]
pub extern "C" fn retro_set_video_refresh(
    callback: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)>,
) {
    debugln!("retro_set_video_refresh()");
    unsafe {
        VIDEO_REFRESH_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample(callback: Option<extern "C" fn(i16, i16)>) {
    debugln!("retro_set_audio_sample()");
    unsafe {
        AUDIO_SAMPLE_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(callback: Option<extern "C" fn(*const i16, usize)>) {
    debugln!("retro_set_audio_sample_batch()");
    unsafe {
        AUDIO_SAMPLE_BATCH_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_input_poll(callback: Option<extern "C" fn()>) {
    debugln!("retro_set_input_poll()");
    unsafe {
        INPUT_POLL_CALLBACK = callback;
    }
}

#[no_mangle]
pub extern "C" fn retro_set_input_state(
    callback: Option<extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16>,
) {
    debugln!("retro_set_input_state()");
    unsafe {
        INPUT_STATE_CALLBACK = callback;
    }
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
