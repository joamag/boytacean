#![allow(clippy::uninlined_format_args)]

pub mod consts;
pub mod palettes;
pub mod structs;

use boytacean::{
    color::XRGB8888_SIZE,
    debugln,
    gb::{AudioProvider, GameBoy},
    info::Info,
    infoln,
    pad::PadKey,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAME_BUFFER_SIZE},
    rom::Cartridge,
    state::{SaveStateFormat, StateManager},
    warnln,
};
use consts::{
    REGION_NTSC, RETRO_API_VERSION, RETRO_DEVICE_ID_JOYPAD_A, RETRO_DEVICE_ID_JOYPAD_B,
    RETRO_DEVICE_ID_JOYPAD_DOWN, RETRO_DEVICE_ID_JOYPAD_L, RETRO_DEVICE_ID_JOYPAD_L2,
    RETRO_DEVICE_ID_JOYPAD_L3, RETRO_DEVICE_ID_JOYPAD_LEFT, RETRO_DEVICE_ID_JOYPAD_R,
    RETRO_DEVICE_ID_JOYPAD_R2, RETRO_DEVICE_ID_JOYPAD_R3, RETRO_DEVICE_ID_JOYPAD_RIGHT,
    RETRO_DEVICE_ID_JOYPAD_SELECT, RETRO_DEVICE_ID_JOYPAD_START, RETRO_DEVICE_ID_JOYPAD_UP,
    RETRO_DEVICE_ID_JOYPAD_X, RETRO_DEVICE_ID_JOYPAD_Y, RETRO_DEVICE_JOYPAD,
    RETRO_ENVIRONMENT_GET_GAME_INFO_EXT, RETRO_ENVIRONMENT_GET_VARIABLE,
    RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE, RETRO_ENVIRONMENT_SET_CONTENT_INFO_OVERRIDE,
    RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, RETRO_ENVIRONMENT_SET_VARIABLES,
    RETRO_PIXEL_FORMAT_XRGB8888,
};
use palettes::get_palette;
use std::{
    collections::HashMap,
    ffi::CStr,
    fmt::{self, Display, Formatter},
    os::raw::{c_char, c_uint, c_void},
    ptr::{self, addr_of},
    slice::from_raw_parts,
};
use structs::{
    RetroGameInfo, RetroGameInfoExt, RetroSystemAvInfo, RetroSystemContentInfoOverride,
    RetroSystemInfo, RetroVariable,
};

/// Represents the information about the LibRetro extension
/// to be used in a static context. Handles strings at the
/// byte buffer level and also at the C string level.
struct LibRetroInfo {
    name: &'static str,
    version: &'static str,
    name_s: String,
    version_s: String,
}

static mut EMULATOR: Option<GameBoy> = None;
static mut KEY_STATES: Option<HashMap<RetroJoypad, bool>> = None;
static mut FRAME_BUFFER: [u32; FRAME_BUFFER_SIZE] = [0x00; FRAME_BUFFER_SIZE];
static mut INFO: LibRetroInfo = LibRetroInfo {
    name: "",
    version: "",
    name_s: String::new(),
    version_s: String::new(),
};
static mut GAME_INFO_EXT: RetroGameInfoExt = RetroGameInfoExt {
    full_path: std::ptr::null(),
    archive_path: std::ptr::null(),
    archive_file: std::ptr::null(),
    dir: std::ptr::null(),
    name: std::ptr::null(),
    ext: std::ptr::null(),
    meta: std::ptr::null(),
    data: std::ptr::null(),
    size: 0,
    file_in_archive: 0,
    persistent_data: 0,
};

static mut PENDING_CYCLES: u32 = 0_u32;

static mut ENVIRONMENT_CALLBACK: Option<extern "C" fn(u32, *const c_void) -> bool> = None;
static mut VIDEO_REFRESH_CALLBACK: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)> = None;
static mut AUDIO_SAMPLE_CALLBACK: Option<extern "C" fn(i16, i16)> = None;
static mut AUDIO_SAMPLE_BATCH_CALLBACK: Option<extern "C" fn(*const i16, usize)> = None;
static mut INPUT_POLL_CALLBACK: Option<extern "C" fn()> = None;
static mut INPUT_STATE_CALLBACK: Option<
    extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16,
> = None;
static mut UPDATED: bool = false;
static mut VARIABLE: RetroVariable = RetroVariable {
    key: "palette\0".as_ptr() as *const c_char,
    value: std::ptr::null(),
};

const VARIABLES: [RetroVariable; 2] = [
    RetroVariable {
        key: "palette\0".as_ptr() as *const c_char,
        value: "DMG color palette; basic|hogwards|christmas|goldsilver|pacman|mariobros|pokemon\0"
            .as_ptr() as *const c_char,
    },
    RetroVariable {
        key: std::ptr::null(),
        value: std::ptr::null(),
    },
];
const INFO_OVERRIDE: [RetroSystemContentInfoOverride; 2] = [
    RetroSystemContentInfoOverride {
        extensions: "gb|gbc\0".as_ptr() as *const c_char,
        need_fullpath: 0,
        persistent_data: 0,
    },
    RetroSystemContentInfoOverride {
        extensions: std::ptr::null(),
        need_fullpath: 0,
        persistent_data: 0,
    },
];

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
/// This function should be called only within Libretro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut RetroSystemInfo) {
    debugln!("retro_get_system_info()");

    INFO.name_s = format!("{}\0", Info::name());
    INFO.name = INFO.name_s.as_str();
    INFO.version_s = format!("v{}\0", Info::version());
    INFO.version = INFO.version_s.as_str();

    (*info).library_name = INFO.name.as_ptr() as *const c_char;
    (*info).library_version = INFO.version.as_ptr() as *const c_char;
    (*info).valid_extensions = "gb|gbc\0".as_ptr() as *const c_char;
    (*info).need_fullpath = u8::from(false);
    (*info).block_extract = u8::from(false);
}

/// # Safety
///
/// This function should be called only within Libretro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut RetroSystemAvInfo) {
    debugln!("retro_get_system_av_info()");

    let emulator = EMULATOR.as_ref().unwrap();
    let environment_cb = ENVIRONMENT_CALLBACK.as_ref().unwrap();

    (*info).geometry.base_width = DISPLAY_WIDTH as u32;
    (*info).geometry.base_height = DISPLAY_HEIGHT as u32;
    (*info).geometry.max_width = DISPLAY_WIDTH as u32;
    (*info).geometry.max_height = DISPLAY_HEIGHT as u32;
    (*info).geometry.aspect_ratio = DISPLAY_WIDTH as f32 / DISPLAY_HEIGHT as f32;
    (*info).timing.fps = GameBoy::VISUAL_FREQ as f64;
    (*info).timing.sample_rate = emulator.audio_sampling_rate() as f64;

    if !environment_cb(
        RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
        &RETRO_PIXEL_FORMAT_XRGB8888 as *const _ as *const c_void,
    ) {
        warnln!("Failed to set pixel format");
    }
}

#[no_mangle]
pub extern "C" fn retro_set_environment(
    callback: Option<extern "C" fn(u32, *const c_void) -> bool>,
) {
    debugln!("retro_set_environment()");
    unsafe {
        ENVIRONMENT_CALLBACK = callback;
        let environment_cb = ENVIRONMENT_CALLBACK.as_ref().unwrap();
        environment_cb(
            RETRO_ENVIRONMENT_SET_VARIABLES,
            &VARIABLES as *const _ as *const c_void,
        );
        environment_cb(
            RETRO_ENVIRONMENT_SET_CONTENT_INFO_OVERRIDE,
            &INFO_OVERRIDE as *const _ as *const c_void,
        );
    }
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device() {
    debugln!("retro_set_controller_port_device()");
}

#[no_mangle]
pub extern "C" fn retro_run() {
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    let environment_cb = unsafe { ENVIRONMENT_CALLBACK.as_ref().unwrap() };
    let video_refresh_cb = unsafe { VIDEO_REFRESH_CALLBACK.as_ref().unwrap() };
    let sample_batch_cb = unsafe { AUDIO_SAMPLE_BATCH_CALLBACK.as_ref().unwrap() };
    let input_poll_cb = unsafe { INPUT_POLL_CALLBACK.as_ref().unwrap() };
    let input_state_cb = unsafe { INPUT_STATE_CALLBACK.as_ref().unwrap() };
    let key_states = unsafe { KEY_STATES.as_mut().unwrap() };

    let mut last_frame = emulator.ppu_frame();

    let mut counter_cycles = unsafe { PENDING_CYCLES };
    let cycle_limit = (GameBoy::CPU_FREQ as f32 * emulator.multiplier() as f32
        / GameBoy::VISUAL_FREQ)
        .round() as u32;

    // determines if any of the variable has changed value
    // if that's the case all of them must be polled for
    // update, and if needed action is triggered
    unsafe {
        if !environment_cb(
            RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE,
            addr_of!(UPDATED) as *const _ as *const c_void,
        ) {
            warnln!("Failed to get variable update");
        }
        if UPDATED {
            update_vars();
            UPDATED = false;
        }
    }

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
            let frame_buffer = emulator.frame_buffer_xrgb8888_u32();
            unsafe {
                FRAME_BUFFER.copy_from_slice(&frame_buffer);
                video_refresh_cb(
                    FRAME_BUFFER.as_ptr() as *const u8,
                    DISPLAY_WIDTH as u32,
                    DISPLAY_HEIGHT as u32,
                    DISPLAY_WIDTH * XRGB8888_SIZE,
                );
            }

            // obtains the index of the current PPU frame, this value
            // is going to be used to detect for new frame presence
            last_frame = emulator.ppu_frame();
        }
    }

    // in case there's new audio data available in the emulator
    // we must handle it by sending it to the audio callback and
    // clearing the audio buffer
    if !emulator.audio_buffer().is_empty() {
        let audio_buffer = emulator
            .audio_buffer()
            .iter()
            .map(|v| *v as i16 * 256)
            .collect::<Vec<i16>>();
        sample_batch_cb(audio_buffer.as_ptr(), audio_buffer.len() / 2_usize);
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
/// This function should be called only within Libretro context.
#[no_mangle]
pub unsafe extern "C" fn retro_load_game(game: *const RetroGameInfo) -> bool {
    debugln!("retro_load_game()");
    let environment_cb = ENVIRONMENT_CALLBACK.as_ref().unwrap();
    if !environment_cb(
        RETRO_ENVIRONMENT_GET_GAME_INFO_EXT,
        addr_of!(GAME_INFO_EXT) as *const _ as *const c_void,
    ) {
        warnln!("Failed to get extended game info");
    }
    infoln!(
        "Loading ROM file in Boytacean from '{}' ({} bytes)...",
        String::from(CStr::from_ptr((*game).path).to_str().unwrap()),
        if GAME_INFO_EXT.size > 0 {
            GAME_INFO_EXT.size
        } else {
            (*game).size
        },
    );
    let instance = EMULATOR.as_mut().unwrap();
    let data_buffer = from_raw_parts((*game).data as *const u8, (*game).size);
    let rom = Cartridge::from_data(data_buffer).unwrap();
    let mode = rom.gb_mode();
    instance.set_mode(mode);
    instance.reset();
    instance.load(true).unwrap();
    instance.load_cartridge(rom).unwrap();
    update_vars();
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
    let instance = unsafe { EMULATOR.as_mut().unwrap() };
    instance.reset();
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(_memory_id: u32) -> usize {
    debugln!("retro_get_memory_size()");
    0
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(_memory_id: u32) -> *mut c_void {
    debugln!("retro_get_memory_data()");
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    debugln!("retro_serialize_size()");
    let instance = unsafe { EMULATOR.as_mut().unwrap() };

    // uses BESS file format for its static nature, meaning that the final
    // size of the serialized state is known in advance
    StateManager::save(instance, Some(SaveStateFormat::Bess), None)
        .unwrap()
        .len()
}

#[no_mangle]
pub extern "C" fn retro_serialize(data: *mut c_void, size: usize) -> bool {
    debugln!("retro_serialize()");
    let instance = unsafe { EMULATOR.as_mut().unwrap() };
    let state = match StateManager::save(instance, Some(SaveStateFormat::Bess), None) {
        Ok(state) => state,
        Err(err) => {
            warnln!("Failed to save state: {}", err);
            #[allow(unreachable_code)]
            {
                return false;
            }
        }
    };
    if state.len() > size {
        warnln!(
            "Invalid state size needed {} bytes, got {} bytes",
            state.len(),
            size
        );
        #[allow(unreachable_code)]
        {
            return false;
        }
    }
    unsafe {
        ptr::copy_nonoverlapping(state.as_ptr(), data as *mut u8, state.len());
    }
    true
}

#[no_mangle]
pub extern "C" fn retro_unserialize(data: *const c_void, size: usize) -> bool {
    debugln!("retro_unserialize()");
    let instance = unsafe { EMULATOR.as_mut().unwrap() };
    let state = unsafe { from_raw_parts(data as *const u8, size) };
    if let Err(err) = StateManager::load(state, instance, None, None) {
        warnln!("Failed to load state: {}", err);
        #[allow(unreachable_code)]
        {
            return false;
        }
    }
    true
}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {
    debugln!("retro_cheat_reset()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    emulator.reset_cheats();
}

/// # Safety
///
/// This function should be called only within Libretro context.
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
    if let Err(err) = emulator.add_cheat_code(&code_s) {
        warnln!("Failed to add cheat code ({}): {}", code_s, err);
    }
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

unsafe fn update_vars() {
    update_palette();
}

unsafe fn update_palette() {
    let emulator = EMULATOR.as_mut().unwrap();
    let environment_cb = ENVIRONMENT_CALLBACK.as_ref().unwrap();
    if !environment_cb(
        RETRO_ENVIRONMENT_GET_VARIABLE,
        addr_of!(VARIABLE) as *const _ as *const c_void,
    ) {
        warnln!("Failed to get variable");
    }
    if VARIABLE.value.is_null() {
        return;
    }
    let palette_name = String::from(CStr::from_ptr(VARIABLE.value).to_str().unwrap());
    let palette_info: boytacean::ppu::PaletteInfo = get_palette(palette_name);
    emulator.ppu().set_palette_colors(palette_info.colors());
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
