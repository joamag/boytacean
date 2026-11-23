//! Implementation of the libretro API, the core loaded by frontends
//! such as RetroArch, running either a Game Boy or a Game Boy Advance
//! through the unified [`System`] of the emulator.

use std::{
    collections::HashMap,
    ffi::CStr,
    fmt::{self, Display, Formatter},
    fs::read,
    os::raw::{c_char, c_uint, c_void},
    path::Path,
    ptr::{self, addr_of},
    slice::from_raw_parts,
};

use boytacean::{
    color::{RGB888_SIZE, XRGB8888_SIZE},
    debugln,
    gb::GameBoy,
    gba::{flash::SaveType, GameBoyAdvance},
    info::Info,
    infoln,
    pad::PadKey,
    ppu::{DISPLAY_HEIGHT, DISPLAY_SIZE, DISPLAY_WIDTH},
    state::{SaveStateFormat, StateManager},
    system::System,
    warnln,
};

use crate::{
    consts::{
        REGION_NTSC, RETRO_API_VERSION, RETRO_DEVICE_ID_JOYPAD_A, RETRO_DEVICE_ID_JOYPAD_B,
        RETRO_DEVICE_ID_JOYPAD_DOWN, RETRO_DEVICE_ID_JOYPAD_L, RETRO_DEVICE_ID_JOYPAD_L2,
        RETRO_DEVICE_ID_JOYPAD_L3, RETRO_DEVICE_ID_JOYPAD_LEFT, RETRO_DEVICE_ID_JOYPAD_R,
        RETRO_DEVICE_ID_JOYPAD_R2, RETRO_DEVICE_ID_JOYPAD_R3, RETRO_DEVICE_ID_JOYPAD_RIGHT,
        RETRO_DEVICE_ID_JOYPAD_SELECT, RETRO_DEVICE_ID_JOYPAD_START, RETRO_DEVICE_ID_JOYPAD_UP,
        RETRO_DEVICE_ID_JOYPAD_X, RETRO_DEVICE_ID_JOYPAD_Y, RETRO_DEVICE_JOYPAD,
        RETRO_ENVIRONMENT_GET_GAME_INFO_EXT, RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY,
        RETRO_ENVIRONMENT_GET_VARIABLE, RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE,
        RETRO_ENVIRONMENT_SET_CONTENT_INFO_OVERRIDE, RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
        RETRO_ENVIRONMENT_SET_VARIABLES, RETRO_MEMORY_SAVE_RAM, RETRO_PIXEL_FORMAT_XRGB8888,
    },
    palettes::get_palette,
    structs::{
        RetroGameInfo, RetroGameInfoExt, RetroSystemAvInfo, RetroSystemContentInfoOverride,
        RetroSystemInfo, RetroVariable,
    },
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

/// The size of the frame buffer, large enough for the display
/// of any of the systems (the 240x160 of the GBA).
const FRAME_BUFFER_SIZE: usize = GameBoyAdvance::DISPLAY_WIDTH * GameBoyAdvance::DISPLAY_HEIGHT;

/// Audio normalization factor for the GBA, whose samples use the
/// complete range of the 16 bits, being the level that matches the
/// one of the Game Boy samples (64), as in the SDL frontend.
const VOLUME_GBA: i32 = 20972;

/// The size of the largest EEPROM of the GBA (64 Kbit), the size of
/// the save RAM of the EEPROM cartridges until the game sets it.
const EEPROM_SIZE: usize = 8192;

/// The name of the file of the BIOS of the GBA, optional, loaded
/// from the system directory of the frontend when present.
const BIOS_GBA: &str = "gba_bios.bin";

/// The emulator of the loaded game, either a Game Boy or a GBA,
/// created for the system of its ROM.
static mut EMULATOR: Option<System> = None;
/// The pressed state of each key of the joypad, as last polled,
/// so that only the keys whose state changed are pressed or lifted.
static mut KEY_STATES: Option<HashMap<RetroJoypad, bool>> = None;
/// The buffer of the frame sent to the frontend, with its pixels
/// in the XRGB8888 format (the pixel format set for the core).
static mut FRAME_BUFFER: [u32; FRAME_BUFFER_SIZE] = [0x00; FRAME_BUFFER_SIZE];
/// The name and version of the core, created only once, as the
/// pointers to them must remain valid for the frontend.
static mut INFO: LibRetroInfo = LibRetroInfo {
    name: "",
    version: "",
    name_s: String::new(),
    version_s: String::new(),
};
/// The extended information of the loaded game, filled by the
/// frontend, used for the size of its ROM file.
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

/// The system directory of the frontend, filled by it, where the
/// BIOS of the GBA is looked for.
static mut SYSTEM_DIRECTORY: *const c_char = std::ptr::null();

/// The cycles run beyond the limit of the previous frame, that are
/// counted in the next one, keeping the timing of the system.
static mut PENDING_CYCLES: u32 = 0_u32;

/// The callback of the frontend for its environment commands.
static mut ENVIRONMENT_CALLBACK: Option<extern "C" fn(u32, *const c_void) -> bool> = None;
/// The callback of the frontend that receives each new frame.
static mut VIDEO_REFRESH_CALLBACK: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)> = None;
/// The callback of the frontend for a single audio sample, unused,
/// as the samples are sent in batches.
static mut AUDIO_SAMPLE_CALLBACK: Option<extern "C" fn(i16, i16)> = None;
/// The callback of the frontend that receives the audio samples.
static mut AUDIO_SAMPLE_BATCH_CALLBACK: Option<extern "C" fn(*const i16, usize)> = None;
/// The callback of the frontend that polls its input devices.
static mut INPUT_POLL_CALLBACK: Option<extern "C" fn()> = None;
/// The callback of the frontend for the state of each input key.
static mut INPUT_STATE_CALLBACK: Option<
    extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16,
> = None;
/// If any of the core options changed, as reported by the frontend,
/// so that their values are read again.
static mut UPDATED: bool = false;
/// The palette core option, whose value is read from the frontend.
static mut VARIABLE: RetroVariable = RetroVariable {
    key: "palette\0".as_ptr() as *const c_char,
    value: std::ptr::null(),
};

/// The core options declared to the frontend, the palette of the
/// DMG, terminated by an empty entry.
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

/// The content information override declared to the frontend, for
/// the extensions of the core, whose ROMs are loaded from memory (no
/// full path needed), terminated by an empty entry.
const INFO_OVERRIDE: [RetroSystemContentInfoOverride; 2] = [
    RetroSystemContentInfoOverride {
        extensions: "gb|gbc|gba\0".as_ptr() as *const c_char,
        need_fullpath: 0,
        persistent_data: 0,
    },
    RetroSystemContentInfoOverride {
        extensions: std::ptr::null(),
        need_fullpath: 0,
        persistent_data: 0,
    },
];

/// The keys of the joypad polled from the frontend, the ones with
/// a key in the emulated systems.
const KEYS: [RetroJoypad; 10] = [
    RetroJoypad::RetroDeviceIdJoypadUp,
    RetroJoypad::RetroDeviceIdJoypadDown,
    RetroJoypad::RetroDeviceIdJoypadLeft,
    RetroJoypad::RetroDeviceIdJoypadRight,
    RetroJoypad::RetroDeviceIdJoypadStart,
    RetroJoypad::RetroDeviceIdJoypadSelect,
    RetroJoypad::RetroDeviceIdJoypadA,
    RetroJoypad::RetroDeviceIdJoypadB,
    RetroJoypad::RetroDeviceIdJoypadL,
    RetroJoypad::RetroDeviceIdJoypadR,
];

/// The keys of the libretro joypad, with the ids used by the
/// frontend for them.
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
    /// Obtains the readable name of the key.
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

/// Obtains the version of the libretro API implemented by the core.
#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    debugln!("retro_api_version()");
    RETRO_API_VERSION
}

/// Initializes the core, with a Game Boy until a game is loaded.
#[no_mangle]
pub extern "C" fn retro_init() {
    debugln!("retro_init()");
    unsafe {
        EMULATOR = Some(System::Gb(GameBoy::new(None)));
        KEY_STATES = Some(HashMap::new());
    }
}

/// Deinitializes the core, with nothing to release.
#[no_mangle]
pub extern "C" fn retro_deinit() {
    debugln!("retro_deinit()");
}

/// Resets the loaded game, as the reset of the system does.
#[no_mangle]
pub extern "C" fn retro_reset() {
    debugln!("retro_reset()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };

    // the Game Boy is reloaded with its cartridge, while the GBA
    // is reset, keeping its ROM (and the save RAM of both)
    match emulator {
        System::Gb(gb) => gb.reload().unwrap(),
        System::Gba(gba) => gba.reset(),
    }
}

/// Fills the information of the core, its name, version and the
/// extensions of the ROMs it loads (from memory).
///
/// # Safety
///
/// This function should be called only within Libretro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut RetroSystemInfo) {
    debugln!("retro_get_system_info()");

    // the strings are only created once, as the pointers to them must
    // remain valid for the frontend after any following call
    if INFO.name_s.is_empty() {
        INFO.name_s = format!("{}\0", Info::name());
        INFO.name = INFO.name_s.as_str();
        INFO.version_s = format!("v{}\0", Info::version());
        INFO.version = INFO.version_s.as_str();
    }

    (*info).library_name = INFO.name.as_ptr() as *const c_char;
    (*info).library_version = INFO.version.as_ptr() as *const c_char;
    (*info).valid_extensions = "gb|gbc|gba\0".as_ptr() as *const c_char;
    (*info).need_fullpath = u8::from(false);
    (*info).block_extract = u8::from(false);
}

/// Fills the audio and video information of the loaded system,
/// setting the pixel format of its frames (XRGB8888).
///
/// # Safety
///
/// This function should be called only within Libretro context.
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut RetroSystemAvInfo) {
    debugln!("retro_get_system_av_info()");

    let emulator = EMULATOR.as_ref().unwrap();
    let environment_cb = ENVIRONMENT_CALLBACK.as_ref().unwrap();

    // the geometry and the timing are the ones of the loaded system,
    // with the maximum geometry being the largest display (the GBA)
    (*info).geometry.base_width = emulator.display_width() as u32;
    (*info).geometry.base_height = emulator.display_height() as u32;
    (*info).geometry.max_width = GameBoyAdvance::DISPLAY_WIDTH as u32;
    (*info).geometry.max_height = GameBoyAdvance::DISPLAY_HEIGHT as u32;
    (*info).geometry.aspect_ratio =
        emulator.display_width() as f32 / emulator.display_height() as f32;
    (*info).timing.fps = emulator.visual_freq() as f64;
    (*info).timing.sample_rate = emulator.audio_sampling_rate() as f64;

    if !environment_cb(
        RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
        &RETRO_PIXEL_FORMAT_XRGB8888 as *const _ as *const c_void,
    ) {
        warnln!("Failed to set pixel format");
    }
}

/// Sets the environment callback of the frontend, declaring to it
/// the core options and the content information override.
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

/// Sets the device of a controller port, ignored, as the joypad
/// is the only device of the core.
#[no_mangle]
pub extern "C" fn retro_set_controller_port_device() {
    debugln!("retro_set_controller_port_device()");
}

/// Runs the loaded system for the cycles of a frame, sending its
/// frames and audio to the frontend and polling its input.
#[no_mangle]
pub extern "C" fn retro_run() {
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    let environment_cb = unsafe { ENVIRONMENT_CALLBACK.as_ref().unwrap() };
    let video_refresh_cb = unsafe { VIDEO_REFRESH_CALLBACK.as_ref().unwrap() };
    let sample_batch_cb = unsafe { AUDIO_SAMPLE_BATCH_CALLBACK.as_ref().unwrap() };
    let input_poll_cb = unsafe { INPUT_POLL_CALLBACK.as_ref().unwrap() };
    let input_state_cb = unsafe { INPUT_STATE_CALLBACK.as_ref().unwrap() };
    let key_states = unsafe { KEY_STATES.as_mut().unwrap() };

    // the system runs for the cycles of a frame, at its current speed
    // (eg: the double speed of the Game Boy Color), starting with the
    // cycles run beyond the limit of the previous frame
    let mut counter_cycles = unsafe { PENDING_CYCLES };
    let cycle_limit = (emulator.cpu_freq() as f32 * emulator.multiplier() as f32
        / emulator.visual_freq())
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

    // the system is matched once, outside of the loop (the hot path of
    // the core), so that each clock (an instruction) runs without the
    // dispatch of the system, with the loop of the GBA in `run_gba()`
    match emulator {
        System::Gb(gb) => {
            let mut last_frame = gb.ppu_frame();

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
                counter_cycles += gb.clock() as u32;

                // in case a new frame is available in the emulator
                // then the frame must be pushed into display
                if gb.ppu_frame() != last_frame {
                    let frame_buffer = gb.frame_buffer_xrgb8888_u32();
                    unsafe {
                        FRAME_BUFFER[..DISPLAY_SIZE].copy_from_slice(&frame_buffer);
                        video_refresh_cb(
                            FRAME_BUFFER.as_ptr() as *const u8,
                            DISPLAY_WIDTH as u32,
                            DISPLAY_HEIGHT as u32,
                            DISPLAY_WIDTH * XRGB8888_SIZE,
                        );
                    }

                    // obtains the index of the current PPU frame, this value
                    // is going to be used to detect for new frame presence
                    last_frame = gb.ppu_frame();
                }
            }
        }
        System::Gba(gba) => run_gba(gba, counter_cycles, cycle_limit),
    }

    // in case there's new audio data available in the emulator
    // we must handle it by sending it to the audio callback and
    // clearing the audio buffer
    if !emulator.audio_buffer().is_empty() {
        // the samples are scaled into the 16 bits range of the frontend,
        // the ones of the Game Boy multiplied by 256 and the ones of the
        // GBA normalized to the same level
        let audio_buffer = if emulator.is_gba() {
            emulator
                .audio_buffer()
                .iter()
                .map(|v| (*v as i32 * 64 * 256 / VOLUME_GBA) as i16)
                .collect::<Vec<i16>>()
        } else {
            emulator
                .audio_buffer()
                .iter()
                .map(|v| *v * 256)
                .collect::<Vec<i16>>()
        };
        sample_batch_cb(audio_buffer.as_ptr(), audio_buffer.len() / 2_usize);
        emulator.clear_audio_buffer();
    }

    // polls the input of the frontend, pressing and lifting the keys
    // whose state changed since the previous frame
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

/// Obtains the region of the core (NTSC), the one of its timing.
#[no_mangle]
pub extern "C" fn retro_get_region() -> u32 {
    debugln!("retro_get_region()");
    REGION_NTSC
}

/// Loads the game with the ROM data provided by the frontend,
/// creating the emulator for its system (Game Boy or GBA).
///
/// # Safety
///
/// This function should be called only within Libretro context.
#[no_mangle]
pub unsafe extern "C" fn retro_load_game(game: *const RetroGameInfo) -> bool {
    debugln!("retro_load_game()");
    let environment_cb = ENVIRONMENT_CALLBACK.as_ref().unwrap();

    // obtains the extended information of the game, used for the
    // size of its ROM file, printed when it is loaded
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
    // creates the emulator for the system of the ROM (Game Boy or GBA),
    // ready to run, from the ROM data loaded by the frontend
    let data_buffer = from_raw_parts((*game).data as *const u8, (*game).size);
    let mut instance = System::from_rom(data_buffer).unwrap();

    // the size of the EEPROM of the GBA (512 bytes or 8 KiB) is only known
    // once the game accesses it, so its save RAM starts with the largest
    // one, loading the saves of both, and only shrinks in place, so that
    // the pointer to it, kept by the frontend, remains valid
    if let System::Gba(gba) = &mut instance {
        if gba.cpu.bus.save.save_type() == SaveType::Eeprom {
            gba.set_ram_data(vec![0xff; EEPROM_SIZE]);
        }

        // the BIOS of the GBA is optional, as its calls are emulated (HLE),
        // being loaded from the system directory of the frontend if present
        if environment_cb(
            RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY,
            addr_of!(SYSTEM_DIRECTORY) as *const _ as *const c_void,
        ) && !SYSTEM_DIRECTORY.is_null()
        {
            let directory = CStr::from_ptr(SYSTEM_DIRECTORY).to_str().unwrap();
            let path = Path::new(directory).join(BIOS_GBA);
            if let Ok(data) = read(&path) {
                infoln!(
                    "Loading GBA BIOS in Boytacean from '{}' ({} bytes)...",
                    path.display(),
                    data.len()
                );
                gba.load_bios(&data);
            }
        }
    }
    EMULATOR = Some(instance);

    // applies the values of the core options to the new emulator
    update_vars();
    true
}

/// Loads a special game (eg: made of several ROMs), unsupported.
#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _system: u32,
    _info: *const RetroGameInfo,
    _num_info: usize,
) -> bool {
    debugln!("retro_load_game_special()");
    false
}

/// Unloads the game, resetting the emulator.
#[no_mangle]
pub extern "C" fn retro_unload_game() {
    debugln!("retro_unload_game()");
    let instance = unsafe { EMULATOR.as_mut().unwrap() };
    instance.reset();
}

/// Obtains the size of the memory of the provided type, the save
/// RAM of the cartridges with a battery being the only one exposed.
#[no_mangle]
pub extern "C" fn retro_get_memory_size(memory_id: u32) -> usize {
    debugln!("retro_get_memory_size()");
    if memory_id != RETRO_MEMORY_SAVE_RAM {
        return 0;
    }
    match unsafe { EMULATOR.as_ref().unwrap() } {
        System::Gb(gb) if gb.rom_i().has_battery() => gb.rom_i().ram_data().len(),
        System::Gba(gba) if gba.has_battery() => gba.cpu.bus.save.data.len(),
        _ => 0,
    }
}

/// Obtains the pointer to the memory of the provided type, the
/// buffer of the save RAM itself, read and written by the frontend
/// (eg: for its saves), or null when there is none.
#[no_mangle]
pub extern "C" fn retro_get_memory_data(memory_id: u32) -> *mut c_void {
    debugln!("retro_get_memory_data()");
    if retro_get_memory_size(memory_id) == 0 {
        return std::ptr::null_mut();
    }
    match unsafe { EMULATOR.as_mut().unwrap() } {
        System::Gb(gb) => gb.rom().ram_data_mut().as_mut_ptr() as *mut c_void,
        System::Gba(gba) => gba.cpu.bus.save.data.as_mut_ptr() as *mut c_void,
    }
}

/// Obtains the size of the save states of the loaded system.
#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    debugln!("retro_serialize_size()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };

    // save states are only supported for the Game Boy, a zero size
    // tells the frontend that the GBA has none
    let System::Gb(instance) = emulator else {
        return 0;
    };

    // uses BESS file format for its static nature, meaning that the final
    // size of the serialized state is known in advance
    StateManager::save(instance, Some(SaveStateFormat::Bess), None)
        .unwrap()
        .len()
}

/// Saves the state of the loaded system into the provided buffer,
/// in the BESS format, failing when the buffer is too small.
#[no_mangle]
pub extern "C" fn retro_serialize(data: *mut c_void, size: usize) -> bool {
    debugln!("retro_serialize()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    let System::Gb(instance) = emulator else {
        return false;
    };
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

    // copies the state into the buffer of the frontend, that fits it
    unsafe {
        ptr::copy_nonoverlapping(state.as_ptr(), data as *mut u8, state.len());
    }
    true
}

/// Loads the state of the loaded system from the provided buffer.
#[no_mangle]
pub extern "C" fn retro_unserialize(data: *const c_void, size: usize) -> bool {
    debugln!("retro_unserialize()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    let System::Gb(instance) = emulator else {
        return false;
    };
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

/// Removes all the cheats of the loaded system.
#[no_mangle]
pub extern "C" fn retro_cheat_reset() {
    debugln!("retro_cheat_reset()");
    let emulator = unsafe { EMULATOR.as_mut().unwrap() };
    if let System::Gb(gb) = emulator {
        gb.reset_cheats();
    }
}

/// Adds the provided cheat code (Game Genie or GameShark) to the
/// loaded system.
///
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
    let code_c = CStr::from_ptr(code);
    let code_s = code_c.to_string_lossy().into_owned();

    // cheats are only supported for the Game Boy
    let emulator = EMULATOR.as_mut().unwrap();
    let System::Gb(gb) = emulator else {
        warnln!("Failed to add cheat code ({}): not supported", code_s);
        #[allow(unreachable_code)]
        {
            return;
        }
    };
    if let Err(err) = gb.add_cheat_code(&code_s) {
        warnln!("Failed to add cheat code ({}): {}", code_s, err);
    }
}

/// Sets the video refresh callback of the frontend, that receives
/// the frames of the core.
#[no_mangle]
pub extern "C" fn retro_set_video_refresh(
    callback: Option<extern "C" fn(*const u8, c_uint, c_uint, usize)>,
) {
    debugln!("retro_set_video_refresh()");
    unsafe {
        VIDEO_REFRESH_CALLBACK = callback;
    }
}

/// Sets the audio sample callback of the frontend, for a single
/// sample (unused, as the samples are sent in batches).
#[no_mangle]
pub extern "C" fn retro_set_audio_sample(callback: Option<extern "C" fn(i16, i16)>) {
    debugln!("retro_set_audio_sample()");
    unsafe {
        AUDIO_SAMPLE_CALLBACK = callback;
    }
}

/// Sets the audio sample batch callback of the frontend, that
/// receives the audio samples of the core.
#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(callback: Option<extern "C" fn(*const i16, usize)>) {
    debugln!("retro_set_audio_sample_batch()");
    unsafe {
        AUDIO_SAMPLE_BATCH_CALLBACK = callback;
    }
}

/// Sets the input poll callback of the frontend, called before
/// reading the state of the keys.
#[no_mangle]
pub extern "C" fn retro_set_input_poll(callback: Option<extern "C" fn()>) {
    debugln!("retro_set_input_poll()");
    unsafe {
        INPUT_POLL_CALLBACK = callback;
    }
}

/// Sets the input state callback of the frontend, that reports the
/// state of each key of the joypad.
#[no_mangle]
pub extern "C" fn retro_set_input_state(
    callback: Option<extern "C" fn(port: u32, device: u32, index: u32, id: u32) -> i16>,
) {
    debugln!("retro_set_input_state()");
    unsafe {
        INPUT_STATE_CALLBACK = callback;
    }
}

/// Runs the GBA for the cycles of a frame, starting with the provided
/// ones, pushing its new frames into display, as `retro_run()` does
/// for the Game Boy.
///
/// Kept out of line so that the (hotter) loop of the Game Boy, in
/// `retro_run()`, is optimized as when it was the only one.
#[inline(never)]
fn run_gba(gba: &mut GameBoyAdvance, mut counter_cycles: u32, cycle_limit: u32) {
    let video_refresh_cb = unsafe { VIDEO_REFRESH_CALLBACK.as_ref().unwrap() };
    let mut last_frame = gba.ppu_frame();

    loop {
        // limits the number of ticks to the typical number
        // of cycles expected for the current logic cycle
        if counter_cycles >= cycle_limit {
            unsafe { PENDING_CYCLES = counter_cycles - cycle_limit };
            break;
        }

        // runs the GBA clock, advancing the CPU and the
        // other frequency based components of the system
        counter_cycles += gba.clock();

        // in case a new frame is available in the emulator
        // then the frame must be pushed into display, with
        // its RGB888 pixels converted into XRGB8888
        if gba.ppu_frame() != last_frame {
            unsafe {
                for (pixel, color) in FRAME_BUFFER
                    .iter_mut()
                    .zip(gba.frame_buffer().chunks_exact(RGB888_SIZE))
                {
                    *pixel = ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32;
                }
                video_refresh_cb(
                    FRAME_BUFFER.as_ptr() as *const u8,
                    GameBoyAdvance::DISPLAY_WIDTH as u32,
                    GameBoyAdvance::DISPLAY_HEIGHT as u32,
                    GameBoyAdvance::DISPLAY_WIDTH * XRGB8888_SIZE,
                );
            }

            // obtains the index of the current PPU frame, this value
            // is going to be used to detect for new frame presence
            last_frame = gba.ppu_frame();
        }
    }
}

/// Reads the values of the core options from the frontend, applying
/// them to the emulator.
unsafe fn update_vars() {
    update_palette();
}

/// Reads the palette core option from the frontend, applying its
/// colors to the DMG palette of the Game Boy.
unsafe fn update_palette() {
    let emulator = EMULATOR.as_mut().unwrap();
    let environment_cb = ENVIRONMENT_CALLBACK.as_ref().unwrap();
    if !environment_cb(
        RETRO_ENVIRONMENT_GET_VARIABLE,
        addr_of!(VARIABLE) as *const _ as *const c_void,
    ) {
        warnln!("Failed to get variable");
    }

    // without a value for the option the palette is kept as is
    if VARIABLE.value.is_null() {
        return;
    }
    let palette_name = String::from(CStr::from_ptr(VARIABLE.value).to_str().unwrap());
    let palette_info: boytacean::ppu::PaletteInfo = get_palette(palette_name);

    // the palette is only used by the Game Boy (DMG)
    if let System::Gb(gb) = emulator {
        gb.ppu().set_palette_colors(palette_info.colors());
    }
}

/// Converts a key of the libretro joypad into the key of the
/// emulated systems, if it has one.
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
        RetroJoypad::RetroDeviceIdJoypadL => Some(PadKey::L),
        RetroJoypad::RetroDeviceIdJoypadR => Some(PadKey::R),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        ffi::{CStr, CString},
        fs::{create_dir_all, remove_dir_all, write},
        os::raw::{c_char, c_uint, c_void},
        ptr,
        slice::from_raw_parts,
        sync::Mutex,
    };

    use boytacean::{
        gb::{GameBoy, GameBoyMode},
        gba::{flash::SaveType, GameBoyAdvance},
        info::Info,
        pad::PadKey,
        system::System,
    };

    use super::{
        retro_cheat_reset, retro_cheat_set, retro_get_memory_data, retro_get_memory_size,
        retro_get_system_av_info, retro_get_system_info, retro_init, retro_key_to_pad,
        retro_load_game, retro_reset, retro_run, retro_serialize, retro_serialize_size,
        retro_set_audio_sample_batch, retro_set_environment, retro_set_input_poll,
        retro_set_input_state, retro_set_video_refresh, retro_unserialize, update_palette,
        RetroJoypad, BIOS_GBA, EMULATOR, VOLUME_GBA,
    };
    use crate::{
        consts::{
            RETRO_DEVICE_ID_JOYPAD_L, RETRO_DEVICE_ID_JOYPAD_R,
            RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY, RETRO_ENVIRONMENT_GET_VARIABLE,
            RETRO_MEMORY_SAVE_RAM, RETRO_MEMORY_SYSTEM_RAM,
        },
        palettes::get_palette,
        structs::{
            RetroGameGeometry, RetroGameInfo, RetroSystemAvInfo, RetroSystemInfo,
            RetroSystemTiming, RetroVariable,
        },
    };

    static EMULATOR_LOCK: Mutex<()> = Mutex::new(());

    /// The last frame sent by the core, with its width, height
    /// and pitch, as kept by the video refresh callback.
    static mut FRAME: (Vec<u32>, c_uint, c_uint, usize) = (Vec::new(), 0, 0, 0);

    /// The audio samples sent by the core, as kept by the audio
    /// sample batch callback.
    static mut AUDIO: Vec<i16> = Vec::new();

    /// The keys pressed in the frontend, as a mask of their ids,
    /// returned by the input state callback.
    static mut PRESSED: u16 = 0;

    /// The system directory of the frontend, as returned by the
    /// environment callback that supports it.
    static mut SYSTEM_DIRECTORY: Option<CString> = None;

    /// The ROM of the jsmolka test suite used to run the GBA,
    /// which draws a static image.
    const GBA_ROM: &[u8] =
        include_bytes!("../../../res/roms.gba/test/jsmolka_gba-tests/ppu_stripes.gba");

    /// The ROM of the jsmolka test suite with a GBA SRAM.
    const GBA_SRAM_ROM: &[u8] =
        include_bytes!("../../../res/roms.gba/test/jsmolka_gba-tests/sram.gba");

    /// The info file of the core, used by RetroArch to describe
    /// the core, that must agree with the core itself.
    const INFO_FILE: &str = include_str!("../res/boytacean_libretro.info");

    /// Obtains the value of the field with the provided key
    /// in the info file of the core.
    fn info_value(key: &str) -> &'static str {
        INFO_FILE
            .lines()
            .filter_map(|line| line.split_once('='))
            .find(|(name, _)| name.trim() == key)
            .map(|(_, value)| value.trim().trim_matches('"'))
            .unwrap()
    }

    /// Environment callback of a frontend that supports no
    /// command, as the tests run without a frontend.
    extern "C" fn environment(_command: u32, _data: *const c_void) -> bool {
        false
    }

    /// Environment callback of a frontend that only supports the
    /// command to get a variable, returning the "christmas" palette.
    extern "C" fn environment_palette(command: u32, data: *const c_void) -> bool {
        if command != RETRO_ENVIRONMENT_GET_VARIABLE {
            return false;
        }
        let variable = data as *mut RetroVariable;
        unsafe { (*variable).value = "christmas\0".as_ptr() as *const c_char };
        true
    }

    /// Environment callback of a frontend that only supports the
    /// command to get its system directory.
    extern "C" fn environment_system(command: u32, data: *const c_void) -> bool {
        if command != RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY {
            return false;
        }
        let directory = data as *mut *const c_char;
        unsafe { *directory = SYSTEM_DIRECTORY.as_ref().unwrap().as_ptr() };
        true
    }

    /// Video refresh callback of a frontend, that keeps the last
    /// frame sent by the core.
    extern "C" fn video_refresh(data: *const u8, width: c_uint, height: c_uint, pitch: usize) {
        let frame = unsafe { from_raw_parts(data as *const u32, pitch / 4 * height as usize) };
        unsafe { FRAME = (frame.to_vec(), width, height, pitch) };
    }

    /// Audio sample batch callback of a frontend, that keeps the
    /// audio samples sent by the core.
    extern "C" fn audio_sample_batch(data: *const i16, frames: usize) {
        let samples = unsafe { from_raw_parts(data, frames * 2) };
        unsafe { AUDIO.extend_from_slice(samples) };
    }

    /// Input poll callback of a frontend, with nothing to poll.
    extern "C" fn input_poll() {}

    /// Input state callback of a frontend, with the keys of the
    /// mask of pressed keys.
    extern "C" fn input_state(_port: u32, _device: u32, _index: u32, id: u32) -> i16 {
        unsafe { (PRESSED >> id & 0x1) as i16 }
    }

    /// Sets the callbacks of a frontend needed to run the core,
    /// keeping its frames and audio, and with no key pressed.
    fn set_callbacks() {
        retro_set_video_refresh(Some(video_refresh));
        retro_set_audio_sample_batch(Some(audio_sample_batch));
        retro_set_input_poll(Some(input_poll));
        retro_set_input_state(Some(input_state));
        unsafe { PRESSED = 0 };
    }

    /// Builds the ROM of a GBA cartridge with an EEPROM, from a test
    /// ROM without a save, with the EEPROM signature appended.
    fn eeprom_rom() -> Vec<u8> {
        let mut data =
            include_bytes!("../../../res/roms.gba/test/jsmolka_gba-tests/none.gba").to_vec();
        data.extend_from_slice(b"EEPROM_V");
        data
    }

    /// Initializes the core and loads the game with the provided
    /// ROM data, as a frontend does.
    fn load_game(data: &[u8]) {
        load_game_environment(data, environment);
    }

    /// Initializes the core and loads the game with the provided
    /// ROM data, as a frontend with the provided environment does.
    fn load_game_environment(data: &[u8], environment: extern "C" fn(u32, *const c_void) -> bool) {
        retro_init();
        retro_set_environment(Some(environment));
        let game = RetroGameInfo {
            path: "test.gb\0".as_ptr() as *const c_char,
            data: data.as_ptr() as *const c_void,
            size: data.len(),
            meta: ptr::null(),
        };
        assert!(unsafe { retro_load_game(&game) });
    }

    #[test]
    fn test_retro_reset() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        let data = retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8;
        assert!(!data.is_null());
        unsafe { *data = 0x42 };
        retro_reset();

        // the save RAM must stay in the same buffer, as the frontend
        // keeps its pointer, with its contents kept across the reset
        assert_eq!(
            retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8,
            data
        );
        if let System::Gb(gb) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert_eq!(gb.rom_i().ram_data()[0], 0x42);
        } else {
            panic!("Expected Game Boy system");
        }
    }

    #[test]
    fn test_retro_reset_gba() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(GBA_SRAM_ROM);
        let data = retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8;
        assert!(!data.is_null());
        unsafe { *data = 0x42 };
        let emulator = unsafe { EMULATOR.as_mut().unwrap() };
        emulator.next_frame();
        assert_eq!(emulator.ppu_frame(), 1);
        retro_reset();

        // the GBA is reset keeping its ROM, ready to run again, and its
        // save RAM in the same buffer, with its contents
        let emulator = unsafe { EMULATOR.as_mut().unwrap() };
        assert!(emulator.is_gba());
        assert_eq!(emulator.ppu_frame(), 0);
        assert_eq!(emulator.rom_title(), "GBA Tests");
        assert!(emulator.next_frame() > 0);
        assert_eq!(
            retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8,
            data
        );
        assert_eq!(unsafe { *data }, 0x42);
    }

    #[test]
    fn test_retro_get_system_info() {
        let mut info = RetroSystemInfo {
            library_name: ptr::null(),
            library_version: ptr::null(),
            valid_extensions: ptr::null(),
            need_fullpath: 1,
            block_extract: 1,
        };
        unsafe { retro_get_system_info(&mut info) };
        let (library_name, library_version, valid_extensions) = unsafe {
            (
                CStr::from_ptr(info.library_name).to_str().unwrap(),
                CStr::from_ptr(info.library_version).to_str().unwrap(),
                CStr::from_ptr(info.valid_extensions).to_str().unwrap(),
            )
        };
        assert_eq!(library_name, Info::name());
        assert_eq!(library_version, format!("v{}", Info::version()));
        assert_eq!(valid_extensions, "gb|gbc|gba");
        assert_eq!(info.need_fullpath, 0);
        assert_eq!(info.block_extract, 0);

        // the info file must describe the core as it reports itself
        assert_eq!(info_value("corename"), library_name);
        assert_eq!(info_value("display_version"), Info::version());
        assert_eq!(info_value("supported_extensions"), valid_extensions);
        assert_eq!(info_value("license"), env!("CARGO_PKG_LICENSE"));
        assert_eq!(info_value("firmware0_path"), BIOS_GBA);
        assert_eq!(info_value("firmware0_opt"), "true");

        // the pointers must be statically allocated, so the ones of
        // a previous call must remain valid after a new call
        let mut other = RetroSystemInfo {
            library_name: ptr::null(),
            library_version: ptr::null(),
            valid_extensions: ptr::null(),
            need_fullpath: 1,
            block_extract: 1,
        };
        unsafe { retro_get_system_info(&mut other) };
        assert_eq!(other.library_name, info.library_name);
        assert_eq!(other.library_version, info.library_version);
        assert_eq!(other.valid_extensions, info.valid_extensions);
    }

    #[test]
    fn test_retro_get_system_av_info() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        for (data, width, height, fps, sample_rate) in [
            (
                include_bytes!("../../../res/roms/demo/pocket.gb").as_slice(),
                160,
                144,
                GameBoy::VISUAL_FREQ,
                44100.0,
            ),
            (GBA_ROM, 240, 160, GameBoyAdvance::VISUAL_FREQ, 32768.0),
        ] {
            load_game(data);
            let mut info = RetroSystemAvInfo {
                geometry: RetroGameGeometry {
                    base_width: 0,
                    base_height: 0,
                    max_width: 0,
                    max_height: 0,
                    aspect_ratio: 0.0,
                },
                timing: RetroSystemTiming {
                    fps: 0.0,
                    sample_rate: 0.0,
                },
            };
            unsafe { retro_get_system_av_info(&mut info) };
            assert_eq!(info.geometry.base_width, width);
            assert_eq!(info.geometry.base_height, height);
            assert_eq!(info.geometry.aspect_ratio, width as f32 / height as f32);
            assert_eq!(info.timing.fps, fps as f64);
            assert_eq!(info.timing.sample_rate, sample_rate);

            // the maximum geometry is the one of the largest display, so
            // that the frontend fits the frames of any of the systems
            assert_eq!(info.geometry.max_width, 240);
            assert_eq!(info.geometry.max_height, 160);
        }
    }

    #[test]
    fn test_retro_run() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        for (data, width, height) in [
            (
                include_bytes!("../../../res/roms/demo/pocket.gb").as_slice(),
                160,
                144,
            ),
            (GBA_ROM, 240, 160),
        ] {
            load_game(data);
            set_callbacks();
            for _ in 0..10 {
                retro_run();
            }

            // the frames are sent with the size of the display of the
            // running system, with its pixels in the XRGB8888 format
            let (frame, frame_width, frame_height, pitch) = unsafe { FRAME.clone() };
            assert_eq!(frame_width, width);
            assert_eq!(frame_height, height);
            assert_eq!(pitch, width as usize * 4);
            let emulator = unsafe { EMULATOR.as_mut().unwrap() };
            if emulator.is_gba() {
                let expected = emulator
                    .frame_buffer()
                    .chunks_exact(3)
                    .map(|color| {
                        ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | color[2] as u32
                    })
                    .collect::<Vec<u32>>();
                assert_eq!(frame, expected);
                assert!(frame.iter().any(|pixel| *pixel != frame[0]));
            }

            // plays a loud square wave in the GBA (channel 1 at its maximum
            // volume), whose samples would overflow with the Game Boy scale
            if let System::Gba(gba) = emulator {
                gba.cpu.bus.apu.set_soundcnt_x(0x80);
                gba.cpu.bus.write16(0x04000080, 0xff77);
                gba.cpu.bus.write16(0x04000082, 0x0002);
                gba.cpu.bus.write16(0x04000062, 0xf080);
                gba.cpu.bus.write16(0x04000064, 0x8700);
            }

            // the samples of a frame are sent first by the next run, scaled
            // into the range of the frontend, at the level of the Game Boy
            emulator.next_frame();
            let samples = emulator
                .audio_buffer()
                .iter()
                .copied()
                .collect::<Vec<i16>>();
            unsafe { AUDIO = Vec::new() };
            retro_run();
            let audio = unsafe { AUDIO.clone() };
            assert!(!samples.is_empty());
            assert!(audio.len() > samples.len());
            let expected = samples
                .iter()
                .map(|v| {
                    if emulator.is_gba() {
                        (*v as i32 * 64 * 256 / VOLUME_GBA) as i16
                    } else {
                        *v * 256
                    }
                })
                .collect::<Vec<i16>>();
            assert_eq!(audio[..samples.len()], expected);
            if emulator.is_gba() {
                assert!(samples.iter().any(|v| v.abs() > 128));
            }
        }
    }

    #[test]
    fn test_retro_run_keys() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(GBA_ROM);
        set_callbacks();

        // the shoulder buttons of the frontend are the L and R of the
        // GBA, pressed and lifted as the other keys (active low)
        unsafe { PRESSED = 1 << RETRO_DEVICE_ID_JOYPAD_L | 1 << RETRO_DEVICE_ID_JOYPAD_R };
        retro_run();
        if let System::Gba(gba) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert_eq!(gba.cpu.bus.pad.keyinput() & 0x0300, 0x0000);
        } else {
            panic!("Expected GBA system");
        }
        unsafe { PRESSED = 0 };
        retro_run();
        if let System::Gba(gba) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert_eq!(gba.cpu.bus.pad.keyinput() & 0x0300, 0x0300);
        } else {
            panic!("Expected GBA system");
        }

        // while the Game Boy, that has no shoulder buttons, ignores them
        load_game(include_bytes!("../../../res/roms/demo/pocket.gb"));
        set_callbacks();
        unsafe { PRESSED = 1 << RETRO_DEVICE_ID_JOYPAD_L | 1 << RETRO_DEVICE_ID_JOYPAD_R };
        retro_run();
        assert!(unsafe { EMULATOR.as_ref().unwrap() }.is_gb());
    }

    #[test]
    fn test_retro_load_game() {
        let _guard = EMULATOR_LOCK.lock().unwrap();

        // the system is created for the loaded ROM, either a Game Boy
        // (in the mode of its cartridge) or a GBA, switching between them
        load_game(include_bytes!(
            "../../../res/roms/test/blargg/cpu/cpu_instrs.gb"
        ));
        if let System::Gb(gb) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert_eq!(gb.mode(), GameBoyMode::Cgb);
        } else {
            panic!("Expected Game Boy system");
        }
        load_game(GBA_ROM);
        let emulator = unsafe { EMULATOR.as_ref().unwrap() };
        assert!(emulator.is_gba());
        assert_eq!(emulator.rom_title(), "GBA Tests");

        // the EEPROM of the GBA starts with the largest size (8 KiB), as
        // its size is unknown until the game accesses it
        load_game(&eeprom_rom());
        if let System::Gba(gba) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert_eq!(gba.cpu.bus.save.save_type(), SaveType::Eeprom);
            assert_eq!(gba.cpu.bus.save.data, vec![0xff; 8192]);
        } else {
            panic!("Expected GBA system");
        }
        load_game(include_bytes!("../../../res/roms/demo/pocket.gb"));
        if let System::Gb(gb) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert_eq!(gb.mode(), GameBoyMode::Dmg);
            assert_eq!(gb.rom_i().title(), "POCKET-DEMO");
        } else {
            panic!("Expected Game Boy system");
        }
    }

    #[test]
    fn test_retro_load_game_bios() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let directory = temp_dir().join("boytacean_libretro_bios_test");
        create_dir_all(&directory).unwrap();
        unsafe { SYSTEM_DIRECTORY = Some(CString::new(directory.to_str().unwrap()).unwrap()) };

        // without the BIOS in the system directory of the frontend, the BIOS
        // calls of the GBA are emulated, with the ROM booted directly
        load_game_environment(GBA_ROM, environment_system);
        if let System::Gba(gba) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert!(!gba.cpu.bus.use_real_bios);
            assert_eq!(gba.cpu.pc(), 0x0800_0000);
        } else {
            panic!("Expected GBA system");
        }

        // while with it the GBA boots through the BIOS
        write(directory.join(BIOS_GBA), vec![0u8; 0x4000]).unwrap();
        load_game_environment(GBA_ROM, environment_system);
        if let System::Gba(gba) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert!(gba.cpu.bus.use_real_bios);
            assert_eq!(gba.cpu.pc(), 0x0000_0000);
        } else {
            panic!("Expected GBA system");
        }

        // that is not loaded by the Game Boy, nor without a system directory
        load_game_environment(
            include_bytes!("../../../res/roms/demo/pocket.gb"),
            environment_system,
        );
        assert!(unsafe { EMULATOR.as_ref().unwrap() }.is_gb());
        load_game(GBA_ROM);
        if let System::Gba(gba) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert!(!gba.cpu.bus.use_real_bios);
        } else {
            panic!("Expected GBA system");
        }
        remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn test_retro_get_memory_size() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        assert_eq!(retro_get_memory_size(RETRO_MEMORY_SAVE_RAM), 8192);
        assert_eq!(retro_get_memory_size(RETRO_MEMORY_SYSTEM_RAM), 0);

        // a cartridge with RAM but without battery has nothing to save
        load_game(include_bytes!(
            "../../../res/roms/test/blargg/cgb_sound/cgb_sound.gb"
        ));
        assert_eq!(retro_get_memory_size(RETRO_MEMORY_SAVE_RAM), 0);

        // while a cartridge with battery and without RAM in its header
        // keeps the RAM bank allocated for it (as the built-in RAM of the
        // MBC2), saved as the other frontends do in `save_ram_file()`
        load_game(include_bytes!("../../../res/roms/test/rtc3test.gb"));
        assert_eq!(retro_get_memory_size(RETRO_MEMORY_SAVE_RAM), 8192);
    }

    #[test]
    fn test_retro_get_memory_size_gba() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        for (data, size) in [
            (GBA_SRAM_ROM.to_vec(), 0x8000),
            (
                include_bytes!("../../../res/roms.gba/test/jsmolka_gba-tests/flash64.gba").to_vec(),
                0x10000,
            ),
            (
                include_bytes!("../../../res/roms.gba/test/jsmolka_gba-tests/flash128.gba")
                    .to_vec(),
                0x20000,
            ),
            (eeprom_rom(), 0x2000),
            (
                include_bytes!("../../../res/roms.gba/test/jsmolka_gba-tests/none.gba").to_vec(),
                0,
            ),
        ] {
            load_game(&data);
            assert_eq!(retro_get_memory_size(RETRO_MEMORY_SAVE_RAM), size);
            assert_eq!(retro_get_memory_size(RETRO_MEMORY_SYSTEM_RAM), 0);
        }
    }

    #[test]
    fn test_retro_get_memory_data() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        let data = retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8;
        assert!(!data.is_null());
        assert!(retro_get_memory_data(RETRO_MEMORY_SYSTEM_RAM).is_null());

        // the pointer must be the RAM of the cartridge itself, so that the
        // game reads the saves loaded by the frontend into it, and the
        // frontend reads the writes of the game
        if let System::Gb(gb) = unsafe { EMULATOR.as_mut().unwrap() } {
            gb.write_memory(0x0000, 0x0a);
            unsafe { *data.add(1) = 0x42 };
            assert_eq!(gb.read_memory(0xa001), 0x42);
            gb.write_memory(0xa002, 0x24);
            assert_eq!(unsafe { *data.add(2) }, 0x24);
        } else {
            panic!("Expected Game Boy system");
        }

        load_game(include_bytes!(
            "../../../res/roms/test/blargg/cgb_sound/cgb_sound.gb"
        ));
        assert!(retro_get_memory_data(RETRO_MEMORY_SAVE_RAM).is_null());
        load_game(include_bytes!("../../../res/roms/test/rtc3test.gb"));
        assert!(!retro_get_memory_data(RETRO_MEMORY_SAVE_RAM).is_null());
    }

    #[test]
    fn test_retro_get_memory_data_gba() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(GBA_SRAM_ROM);
        let data = retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8;
        assert!(!data.is_null());
        assert!(retro_get_memory_data(RETRO_MEMORY_SYSTEM_RAM).is_null());

        // the pointer must be the save RAM of the cartridge itself, read
        // and written by the game, as the one of the Game Boy
        if let System::Gba(gba) = unsafe { EMULATOR.as_mut().unwrap() } {
            unsafe { *data.add(1) = 0x42 };
            assert_eq!(gba.cpu.bus.read8(0x0e000001), 0x42);
            gba.cpu.bus.write8(0x0e000002, 0x24);
            assert_eq!(unsafe { *data.add(2) }, 0x24);
        } else {
            panic!("Expected GBA system");
        }

        // the EEPROM keeps its buffer when the game sets its size, either
        // the largest one (as loaded) or the smallest one (shrunk in place),
        // as it starts with the largest size, never growing afterwards
        for (count, size) in [(17, 8192), (9, 512)] {
            load_game(&eeprom_rom());
            assert_eq!(retro_get_memory_size(RETRO_MEMORY_SAVE_RAM), 8192);
            let data = retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8;
            unsafe { *data = 0x42 };
            if let System::Gba(gba) = unsafe { EMULATOR.as_mut().unwrap() } {
                gba.cpu.bus.save.start_eeprom_transfer(count);
            }
            assert_eq!(
                retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8,
                data
            );
            assert_eq!(retro_get_memory_size(RETRO_MEMORY_SAVE_RAM), size);
            assert_eq!(unsafe { *data }, 0x42);
        }
        load_game(include_bytes!(
            "../../../res/roms.gba/test/jsmolka_gba-tests/none.gba"
        ));
        assert!(retro_get_memory_data(RETRO_MEMORY_SAVE_RAM).is_null());
    }

    #[test]
    fn test_retro_serialize_size() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        assert!(retro_serialize_size() > 0);

        // the GBA has no save states, which a zero size tells
        load_game(GBA_ROM);
        assert_eq!(retro_serialize_size(), 0);
    }

    #[test]
    fn test_retro_serialize() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        let mut state = vec![0u8; retro_serialize_size() - 1];
        assert!(!retro_serialize(
            state.as_mut_ptr() as *mut c_void,
            state.len()
        ));
        load_game(GBA_ROM);
        let mut state = vec![0u8; 1024];
        assert!(!retro_serialize(
            state.as_mut_ptr() as *mut c_void,
            state.len()
        ));
    }

    #[test]
    fn test_retro_unserialize() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        let data = retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8;
        assert!(!data.is_null());
        unsafe { *data = 0x42 };
        let mut state = vec![0u8; retro_serialize_size()];
        assert!(retro_serialize(
            state.as_mut_ptr() as *mut c_void,
            state.len()
        ));
        unsafe { *data = 0x24 };
        assert!(retro_unserialize(
            state.as_ptr() as *const c_void,
            state.len()
        ));

        // the save RAM of the state must be loaded into the same buffer,
        // as the frontend keeps its pointer (eg: for its autosave)
        assert_eq!(
            retro_get_memory_data(RETRO_MEMORY_SAVE_RAM) as *mut u8,
            data
        );
        assert_eq!(unsafe { *data }, 0x42);
    }

    #[test]
    fn test_retro_unserialize_gba() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(GBA_ROM);
        let state = vec![0u8; 1024];
        assert!(!retro_unserialize(
            state.as_ptr() as *const c_void,
            state.len()
        ));
    }

    #[test]
    fn test_retro_cheat_reset() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        unsafe { retro_cheat_set(0, true, "00A-17B-C49\0".as_ptr() as *const c_char) };
        retro_cheat_reset();
        if let System::Gb(gb) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert!(!gb
                .rom_i()
                .game_genie()
                .as_ref()
                .unwrap()
                .contains_addr(0x4a17));
        } else {
            panic!("Expected Game Boy system");
        }

        // the GBA has no cheats to reset
        load_game(GBA_ROM);
        retro_cheat_reset();
        assert!(unsafe { EMULATOR.as_ref().unwrap() }.is_gba());
    }

    #[test]
    fn test_retro_cheat_set() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        load_game(include_bytes!("../../../res/roms/game/shocklobster.gb"));
        unsafe { retro_cheat_set(0, false, "00A-17B-C49\0".as_ptr() as *const c_char) };
        if let System::Gb(gb) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert!(gb.rom_i().game_genie().is_none());
        } else {
            panic!("Expected Game Boy system");
        }
        unsafe { retro_cheat_set(0, true, "00A-17B-C49\0".as_ptr() as *const c_char) };
        if let System::Gb(gb) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert!(gb
                .rom_i()
                .game_genie()
                .as_ref()
                .unwrap()
                .contains_addr(0x4a17));
        } else {
            panic!("Expected Game Boy system");
        }

        // the cheats are not supported by the GBA, so they are ignored
        load_game(GBA_ROM);
        unsafe { retro_cheat_set(0, true, "00A-17B-C49\0".as_ptr() as *const c_char) };
        assert!(unsafe { EMULATOR.as_ref().unwrap() }.is_gba());
    }

    #[test]
    fn test_update_palette() {
        let _guard = EMULATOR_LOCK.lock().unwrap();
        let palette = get_palette(String::from("christmas"));
        load_game(include_bytes!("../../../res/roms/demo/pocket.gb"));
        retro_set_environment(Some(environment_palette));
        unsafe { update_palette() };
        if let System::Gb(gb) = unsafe { EMULATOR.as_ref().unwrap() } {
            assert_eq!(gb.ppu_i().palette_bg()[0], palette.colors()[0]);
        } else {
            panic!("Expected Game Boy system");
        }

        // the palette is only used by the Game Boy, not by the GBA
        load_game(GBA_ROM);
        retro_set_environment(Some(environment_palette));
        unsafe { update_palette() };
        assert!(unsafe { EMULATOR.as_ref().unwrap() }.is_gba());
    }

    #[test]
    fn test_retro_key_to_pad() {
        assert_eq!(
            retro_key_to_pad(RetroJoypad::RetroDeviceIdJoypadA),
            Some(PadKey::A)
        );
        assert_eq!(
            retro_key_to_pad(RetroJoypad::RetroDeviceIdJoypadL),
            Some(PadKey::L)
        );
        assert_eq!(
            retro_key_to_pad(RetroJoypad::RetroDeviceIdJoypadR),
            Some(PadKey::R)
        );
        assert_eq!(retro_key_to_pad(RetroJoypad::RetroDeviceIdJoypadX), None);
        assert_eq!(retro_key_to_pad(RetroJoypad::RetroDeviceIdJoypadL2), None);
    }
}
