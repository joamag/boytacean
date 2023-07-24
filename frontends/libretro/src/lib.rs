#![allow(clippy::uninlined_format_args)]

use std::os::raw::{c_char, c_float, c_uint, c_void};

use boytacean::{
    gb::GameBoy,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
};

const RETRO_API_VERSION: u32 = 1;
const REGION_NTSC: u32 = 0;

//const RETRO_MEMORY_SAVE_RAM: u32 = 0;
//const RETRO_MEMORY_SYSTEM_RAM: u32 = 0;

static mut ENVIRONMENT_CALLBACK: Option<extern "C" fn(u32, *const c_void) -> bool> = None;
static mut VIDEO_REFRESH_CALLBACK: Option<extern "C" fn(*const u8)> = None;
static mut AUDIO_SAMPLE_CALLBACK: Option<extern "C" fn(i16, i16)> = None;
static mut AUDIO_SAMPLE_BATCH_CALLBACK: Option<extern "C" fn(*const i16, usize)> = None;
static mut INPUT_POLL_CALLBACK: Option<extern "C" fn()> = None;
static mut INPUT_STATE_CALLBACK: Option<
    extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16,
> = None;

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
pub extern "C" fn retro_init() {
    println!("retro_init()");
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    println!("retro_deinit()");
}

#[no_mangle]
pub extern "C" fn retro_reset() {
    println!("retro_reset()");
}

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    println!("retro_api_version()");
    RETRO_API_VERSION
}

#[no_mangle]
pub extern "C" fn retro_get_system_info(info: *mut RetroSystemInfo) {
    println!("retro_get_system_info()");
    unsafe {
        (*info).library_name = "Boytacean\0".as_ptr() as *const c_char;
        (*info).library_version = "v0.9.6\0".as_ptr() as *const c_char;
        (*info).valid_extensions = "gb|gbc\0".as_ptr() as *const c_char;
        (*info).need_fullpath = false;
        (*info).block_extract = false;
    }
}

#[no_mangle]
pub extern "C" fn retro_get_system_av_info(info: *mut RetroSystemAvInfo) {
    println!("retro_get_system_av_info()");
    unsafe {
        (*info).geometry.base_width = DISPLAY_WIDTH as u32;
        (*info).geometry.base_height = DISPLAY_HEIGHT as u32;
        (*info).geometry.max_width = DISPLAY_WIDTH as u32 * 32;
        (*info).geometry.max_height = DISPLAY_HEIGHT as u32 * 32;
        (*info).geometry.aspect_ratio = DISPLAY_WIDTH as f32 / DISPLAY_HEIGHT as f32;
        (*info).timing.fps = GameBoy::VISUAL_FREQ as f64;
        (*info).timing.sample_rate = 44100.0;
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
pub extern "C" fn retro_set_video_refresh(callback: Option<extern "C" fn(*const u8)>) {
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
    println!("retro_run()");
}

#[no_mangle]
pub extern "C" fn retro_get_region() -> u32 {
    println!("retro_get_region()");
    REGION_NTSC
}

#[no_mangle]
pub extern "C" fn retro_load_game(_game: *const RetroGameInfo) -> bool {
    println!("retro_load_game()");
    return true;
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    println!("retro_unload_game()");
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(memory_id: u32) -> *mut c_void {
    println!("retro_get_memory_data()");
    match memory_id {
        //RETRO_MEMORY_SAVE_RAM => SAVE_RAM.as_mut_ptr() as *mut c_void,
        //RETRO_MEMORY_SYSTEM_RAM => SYSTEM_RAM.as_mut_ptr() as *mut c_void,
        _ => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(memory_id: u32) -> usize {
    println!("retro_get_memory_size()");
    match memory_id {
        //RETRO_MEMORY_SAVE_RAM => 0,
        //RETRO_MEMORY_SYSTEM_RAM => 0,
        _ => 0,
    }
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
