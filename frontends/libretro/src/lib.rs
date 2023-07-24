#![allow(clippy::uninlined_format_args)]

use std::os::raw::{c_char, c_float, c_uint, c_void};

use boytacean::{
    gb::GameBoy,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH, RGB1555_SIZE},
};

const RETRO_API_VERSION: u32 = 1;
const REGION_NTSC: u32 = 0;

//const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 10;

//const RETRO_PIXEL_FORMAT_0RGB1555: usize = 0;
//const RETRO_PIXEL_FORMAT_XRGB8888: usize = 1;
//const RETRO_PIXEL_FORMAT_RGB565: usize = 2;

//const RETRO_MEMORY_SAVE_RAM: u32 = 0;
//const RETRO_MEMORY_SYSTEM_RAM: u32 = 0;

static mut EMULATOR: Option<GameBoy> = None;

static mut ENVIRONMENT_CALLBACK: Option<extern "C" fn(u32, *const c_void) -> bool> = None;
static mut VIDEO_REFRESH_CALLBACK: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)> = None;
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
    unsafe {
        EMULATOR = Some(GameBoy::new(None));
        EMULATOR.as_mut().unwrap().load(true);
    }
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

    let mut counter_cycles = 0_u32;
    let cycle_limit = 4194304 / 60; //@TODO this is super tricky

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
    }

    let frame_buffer = emulator.frame_buffer_rgb1555();

    unsafe {
        VIDEO_REFRESH_CALLBACK.unwrap()(
            frame_buffer.as_ptr(),
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

#[no_mangle]
pub extern "C" fn retro_load_game(game: *const RetroGameInfo) -> bool {
    println!("retro_load_game()");
    unsafe {
        let data_buffer = std::slice::from_raw_parts((*game).data as *const u8, (*game).size);
        EMULATOR.as_mut().unwrap().load_rom(data_buffer, None);
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
