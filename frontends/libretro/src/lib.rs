#![allow(clippy::uninlined_format_args)]

use std::os::raw::{c_char, c_void, c_uint};

const RETRO_API_VERSION: u32 = 1;

#[repr(C)]
pub struct retro_system_info {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
pub struct retro_game_geometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}

#[repr(C)]
pub struct retro_system_timing {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
pub struct retro_system_api {
    pub retro_api_version: u32,
    pub retro_get_system_info: extern "C" fn(*mut retro_system_info),
    pub retro_set_environment: extern "C" fn(extern "C" fn(u32, *const c_void), *const c_void),
    pub retro_set_video_refresh: extern "C" fn(extern "C" fn()),
    pub retro_set_audio_sample: extern "C" fn(extern "C" fn(i16)),
    pub retro_set_audio_sample_batch: extern "C" fn(extern "C" fn(*const i16, usize)),
    pub retro_set_input_poll: extern "C" fn(extern "C" fn()),
    pub retro_set_input_state: extern "C" fn(extern "C" fn(u32, u32, u16, i16) -> i16),
    // Add other Libretro core functions here as needed
}

#[no_mangle]
extern "C" fn retro_get_system_info(info: *mut retro_system_info) {
    println!("retro_get_system_info");
    unsafe {
        (*info).library_name = "Boytacean\0".as_ptr() as *const c_char;
        (*info).library_version = "v0.9.6\0".as_ptr() as *const c_char;
        (*info).valid_extensions = "gb|gbc\0".as_ptr() as *const c_char;
        (*info).need_fullpath = false;
        (*info).block_extract = false;
    }
}

#[no_mangle]
extern "C" fn retro_set_environment(
    _callback: extern "C" fn(u32, *const c_void),
    _data: *const c_void,
) {
    // Set any environment variables or configuration options here if needed
    // For example, you might handle system RAM allocation using this function
}

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    println!("retro_api_version()");
    RETRO_API_VERSION
}

#[no_mangle]
pub extern "C" fn retro_init() {
    println!("retro_init()");
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    println!("retro_deinit()");
}
