#![allow(clippy::uninlined_format_args)]

use std::os::raw::{c_char, c_uint};

const RETRO_API_VERSION: u32 = 1;

#[repr(C)]
pub struct RetroSystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
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
pub extern "C" fn retro_get_system_av_info() {
    println!("retro_get_system_av_info()");
}

#[no_mangle]
pub extern "C" fn retro_set_environment() {
    println!("retro_set_environment()");
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
pub extern "C" fn retro_load_game() {
    println!("retro_load_game()");
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    println!("retro_unload_game()");
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data() {
    println!("retro_get_memory_data()");
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size() {
    println!("retro_get_memory_size()");
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
