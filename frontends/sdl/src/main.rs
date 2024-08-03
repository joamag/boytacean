#![allow(clippy::uninlined_format_args)]

pub mod audio;
pub mod data;
pub mod sdl;
pub mod test;

use audio::Audio;
use boytacean::{
    devices::{printer::PrinterDevice, stdout::StdoutDevice},
    gb::{AudioProvider, GameBoy, GameBoyMode},
    info::Info,
    pad::PadKey,
    ppu::PaletteInfo,
    rom::Cartridge,
    serial::{NullDevice, SerialDevice},
    state::StateManager,
    util::{replace_ext, write_file},
};
use boytacean_common::error::Error;
use chrono::Utc;
use clap::Parser;
use image::{ColorType, ImageBuffer, Rgb};
use sdl::{surface_from_bytes, SdlSystem};
use sdl2::{
    event::Event,
    keyboard::{Keycode, Mod},
    pixels::PixelFormatEnum,
    Sdl,
};
use std::{
    cmp::max,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime},
};

/// The scale at which the screen is going to be drawn
/// meaning the ratio between Game Boy resolution and
/// the window size to be displayed.
const SCREEN_SCALE: f32 = 3.0;

/// Base audio volume to be used as the basis of the
/// amplification level of the volume
const VOLUME: f32 = 64.0;

/// The rate (in seconds) at which the current battery
/// backed RAM is going to be stored into the file system.
const STORE_RATE: u8 = 5;

/// The path to the default ROM file that is going to be
/// loaded in case no other ROM path is provided.
const DEFAULT_ROM_PATH: &str = "../../res/roms/demo/pocket.gb";

pub struct Benchmark {
    count: usize,
    cpu_only: Option<bool>,
}

impl Benchmark {
    pub fn new(count: usize, cpu_only: Option<bool>) -> Self {
        Self { count, cpu_only }
    }
}

impl Default for Benchmark {
    fn default() -> Self {
        Self::new(50000000, None)
    }
}

pub struct EmulatorOptions {
    auto_mode: Option<bool>,
    unlimited: Option<bool>,
    features: Option<Vec<&'static str>>,
}

/// Main structure used to control the logic execution of
/// an emulator in an SDL context.
///
/// The way the structure is defined should be as agnostic
/// as possible to the underlying system, meaning that it
/// should be able to run in different systems without any
/// major changes.
pub struct Emulator {
    /// Reference to the system that is going to be used to
    /// run the emulation.
    system: GameBoy,

    /// Flag that controls if the emulator should run in
    /// auto mode, meaning that the mode should be inferred
    /// from the ROM file.
    auto_mode: bool,

    /// Flag that controls if the emulator should run in an
    /// unlimited mode, meaning that no speed limit is imposed.
    unlimited: bool,

    /// Reference to the SDL system that is going to be used
    /// to render the graphics and handle the input.
    sdl: Option<SdlSystem>,

    /// Reference to the audio provider that is going to be used
    /// to handle the audio output.
    audio: Option<Audio>,

    /// The title of the emulator that is going to be displayed
    /// in the window title.
    title: String,

    /// The path to the ROM file that is going to be loaded into
    /// the emulator.
    rom_path: String,

    /// The path to the RAM file (save state) that is going to be
    /// to load state from.
    ram_path: String,

    /// Path to the directory where storage of files is located, this
    /// value is going to be used to save files.
    ///
    /// Example usage of this directory includes screenshots, save states
    /// and other files that are going to be saved to the file system.
    dir_path: String,

    /// The frequency at which the logic of the emulator is going to
    /// be executed, this value is going to be used to control the
    /// speed of the emulation.
    logic_frequency: u32,

    /// The frequency at which the visual part of the emulator is going
    /// to be executed, this value is going to be used to control the
    /// speed of the visual part of the emulation (eg: 60 FPS).
    visual_frequency: f32,

    /// The time at which the next tick is going to be executed, this
    /// value is expressed in milliseconds.
    next_tick_time: f32,

    /// Integer representation of the `next_tick_time` value.
    next_tick_time_i: u32,

    /// Flag that controls if the emulator is running above its reference
    /// speed.
    fast: bool,

    /// Set of features that are going to be enabled in the emulator, this
    /// value is going to be used to control the behavior of the emulator.
    features: Vec<&'static str>,

    /// Set of palettes that are going to be used to control the color
    /// of the emulator frame buffer.
    palettes: [PaletteInfo; 7],

    /// Index of the current palette controlling the palette being used.
    palette_index: usize,
}

impl Emulator {
    pub fn new(system: GameBoy, options: EmulatorOptions) -> Self {
        Self {
            system,
            auto_mode: options.auto_mode.unwrap_or(true),
            unlimited: options.unlimited.unwrap_or(false),
            sdl: None,
            audio: None,
            title: format!("{} v{}", Info::name(), Info::version()),
            rom_path: String::from("invalid"),
            ram_path: String::from("invalid"),
            dir_path: String::from("invalid"),
            logic_frequency: GameBoy::CPU_FREQ,
            visual_frequency: GameBoy::VISUAL_FREQ,
            next_tick_time: 0.0,
            next_tick_time_i: 0,
            fast: false,
            features: options
                .features
                .unwrap_or_else(|| vec!["video", "audio", "no-vsync"]),
            palettes: [
                PaletteInfo::new(
                    "basic",
                    [
                        [0xff, 0xff, 0xff],
                        [0xc0, 0xc0, 0xc0],
                        [0x60, 0x60, 0x60],
                        [0x00, 0x00, 0x00],
                    ],
                ),
                PaletteInfo::new(
                    "hogwards",
                    [
                        [0xb6, 0xa5, 0x71],
                        [0x8b, 0x7e, 0x56],
                        [0x55, 0x4d, 0x35],
                        [0x20, 0x1d, 0x13],
                    ],
                ),
                PaletteInfo::new(
                    "christmas",
                    [
                        [0xe8, 0xe7, 0xdf],
                        [0x8b, 0xab, 0x95],
                        [0x9e, 0x5c, 0x5e],
                        [0x53, 0x4d, 0x57],
                    ],
                ),
                PaletteInfo::new(
                    "goldsilver",
                    [
                        [0xc5, 0xc6, 0x6d],
                        [0x97, 0xa1, 0xb0],
                        [0x58, 0x5e, 0x67],
                        [0x23, 0x52, 0x29],
                    ],
                ),
                PaletteInfo::new(
                    "pacman",
                    [
                        [0xff, 0xff, 0x00],
                        [0xff, 0xb8, 0x97],
                        [0x37, 0x32, 0xff],
                        [0x00, 0x00, 0x00],
                    ],
                ),
                PaletteInfo::new(
                    "mariobros",
                    [
                        [0xf7, 0xce, 0xc3],
                        [0xcc, 0x9e, 0x22],
                        [0x92, 0x34, 0x04],
                        [0x00, 0x00, 0x00],
                    ],
                ),
                PaletteInfo::new(
                    "pokemon",
                    [
                        [0xf8, 0x78, 0x00],
                        [0xb8, 0x60, 0x00],
                        [0x78, 0x38, 0x00],
                        [0x00, 0x00, 0x00],
                    ],
                ),
            ],
            palette_index: 0,
        }
    }

    pub fn start(&mut self, screen_scale: f32) {
        self.start_base();

        let sdl = sdl2::init().unwrap();

        if self.features.contains(&"video") {
            self.start_graphics(&sdl, screen_scale);
        }
        if self.features.contains(&"audio") {
            self.start_audio(&sdl);
        }
    }

    pub fn start_base(&mut self) {
        self.system.set_diag();
        #[cfg(feature = "slow")]
        {
            self.logic_frequency = 100;
        }
    }

    pub fn start_graphics(&mut self, sdl: &Sdl, screen_scale: f32) {
        self.sdl = Some(SdlSystem::new(
            sdl,
            &self.title,
            self.system.display_width() as u32,
            self.system.display_height() as u32,
            screen_scale,
            !self.features.contains(&"no-accelerated"),
            !self.features.contains(&"no-vsync"),
        ));
    }

    pub fn start_audio(&mut self, sdl: &Sdl) {
        self.audio = Some(Audio::new(
            sdl,
            self.system.audio_sampling_rate() as i32,
            self.system.audio_channels(),
            None,
        ));
    }

    pub fn load_rom(&mut self, path: Option<&str>) -> Result<(), Error> {
        let rom_path: &str = path.unwrap_or(&self.rom_path);
        let ram_path = replace_ext(rom_path, "sav").unwrap_or_else(|| "invalid".to_string());
        let rom = self.system.load_rom_file(
            rom_path,
            if Path::new(&ram_path).exists() {
                Some(&ram_path)
            } else {
                None
            },
        )?;
        println!(
            "========= Cartridge =========\n{}\n=============================",
            rom
        );
        if let Some(ref mut sdl) = self.sdl {
            sdl.window_mut()
                .set_title(format!("{} [{}]", self.title, rom.title()).as_str())
                .unwrap();
        }
        self.rom_path = String::from(rom_path);
        self.ram_path = ram_path;
        self.dir_path = Path::new(&self.rom_path)
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.system.reset();
        self.system.load(true)?;
        self.load_rom(None)?;
        Ok(())
    }

    pub fn apply_cheats(&mut self, cheats: &Vec<String>) {
        for cheat in cheats {
            self.system.add_cheat_code(cheat).unwrap();
        }
    }

    pub fn benchmark(&mut self, params: &Benchmark) {
        println!("Going to run benchmark...");

        let count = params.count;
        let cpu_only = params.cpu_only.unwrap_or(false);
        let mut cycles = 0u64;

        if cpu_only {
            self.system.set_all_enabled(false);
        }

        let initial = SystemTime::now();

        for _ in 0..count {
            cycles += self.system.clock() as u64;
        }

        let delta = initial.elapsed().unwrap().as_millis() as f64 / 1000.0;
        let frequency_mhz = cycles as f64 / delta / 1000.0 / 1000.0;
        let speedup = cycles as f64
            / GameBoy::CPU_FREQ as f64
            / delta
            / self.system.speed().multiplier() as f64;
        let framerate = speedup * GameBoy::VISUAL_FREQ as f64;

        println!(
            "Took {:.2} seconds to run {} ticks ({} cycles) ({:.2} Mhz, {:.2} speedup, {:.2} FPS)!",
            delta, count, cycles, frequency_mhz, speedup, framerate
        );
    }

    fn save_state(&mut self, file_path: &str) {
        if let Err(message) = StateManager::save_file(file_path, &mut self.system, None, None) {
            println!("Error saving state: {}", message)
        } else {
            println!("Saved state into: {}", file_path)
        }
    }

    fn load_state(&mut self, file_path: &str) {
        if let Err(message) = StateManager::load_file(file_path, &mut self.system, None, None) {
            println!("Error loading state: {}", message)
        } else {
            println!("Loaded state from: {}", file_path)
        }
    }

    fn save_image(&mut self, file_path: &str) {
        let width = self.system.display_width() as u32;
        let height = self.system.display_height() as u32;
        let pixels = self.system.frame_buffer_raw();

        let mut image_buffer: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);

        for (x, y, pixel) in image_buffer.enumerate_pixels_mut() {
            let base = ((y * width + x) * 3) as usize;
            *pixel = Rgb([pixels[base], pixels[base + 1], pixels[base + 2]])
        }

        image_buffer
            .save_with_format(file_path, image::ImageFormat::Png)
            .unwrap();
    }

    pub fn toggle_audio(&mut self) {
        let apu_enabled = self.system.apu_enabled();
        self.system.set_apu_enabled(!apu_enabled);
    }

    pub fn toggle_palette(&mut self) {
        self.system
            .ppu()
            .set_palette_colors(self.palettes[self.palette_index].colors());
        self.palette_index = (self.palette_index + 1) % self.palettes.len();
    }

    pub fn toggle_fullscreen(&mut self) {
        let window = self.sdl.as_mut().unwrap().window_mut();
        if window.fullscreen_state() == sdl2::video::FullscreenType::Off {
            window
                .set_fullscreen(sdl2::video::FullscreenType::Desktop)
                .unwrap()
        } else {
            window
                .set_fullscreen(sdl2::video::FullscreenType::Off)
                .unwrap()
        }
    }

    pub fn print_debug(&mut self) {
        println!("{}", self.system.description_debug());
    }

    pub fn limited(&self) -> bool {
        !self.unlimited
    }

    pub fn run(&mut self) {
        // obtains the dimensions of the display that are going
        // to be used for the graphics rendering
        let (width, height) = (self.system.display_width(), self.system.display_height());

        // updates the icon of the window to reflect the image
        // and style of the emulator
        let surface = surface_from_bytes(&data::ICON);
        self.sdl.as_mut().unwrap().window_mut().set_icon(&surface);

        // creates an accelerated canvas to be used in the drawing
        // then clears it and presents it
        self.sdl.as_mut().unwrap().canvas.present();

        // creates a texture creator for the current canvas, required
        // for the creation of dynamic and static textures
        let texture_creator = self.sdl.as_mut().unwrap().canvas.texture_creator();

        // creates the texture streaming that is going to be used
        // as the target for the pixel buffer
        let mut texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, width as u32, height as u32)
            .unwrap();

        // calculates the rate as visual cycles that will take from
        // the current visual frequency to re-save the battery backed RAM
        let store_count = (self.visual_frequency * STORE_RATE as f32).round() as u32;

        // starts the variable that will control the number of cycles that
        // are going to move (because of overflow) from one tick to another
        let mut pending_cycles = 0u32;

        // allocates space for the loop ticks counter to be used in each
        // iteration cycle
        let mut counter = 0u32;

        // the main loop to execute the multiple machine clocks, in
        // theory the emulator should keep an infinite loop here
        'main: loop {
            // increments the counter that will keep track
            // on the number of visual ticks since beginning
            counter = counter.wrapping_add(1);

            // in case the current counter is a multiple of the store rate
            // then we've reached the time to re-save the battery backed RAM
            // into a *.sav file in the file system
            if counter % store_count == 0 && self.system.rom().has_battery() {
                let ram_data = self.system.rom().ram_data();
                write_file(&self.ram_path, ram_data, None).unwrap();
            }

            // obtains an event from the SDL sub-system to be
            // processed under the current emulation context
            while let Some(event) = self.sdl.as_mut().unwrap().event_pump.poll_event() {
                match event {
                    Event::Quit { .. } => break 'main,
                    Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'main,
                    Event::KeyDown {
                        keycode: Some(Keycode::R),
                        ..
                    } => self.reset().unwrap(),
                    Event::KeyDown {
                        keycode: Some(Keycode::B),
                        ..
                    } => self.benchmark(&Benchmark::default()),
                    Event::KeyDown {
                        keycode: Some(Keycode::I),
                        ..
                    } => self.save_image(&self.image_name(Some("png"), Some(&self.dir_path))),
                    Event::KeyDown {
                        keycode: Some(Keycode::T),
                        ..
                    } => self.toggle_audio(),
                    Event::KeyDown {
                        keycode: Some(Keycode::P),
                        ..
                    } => self.toggle_palette(),
                    Event::KeyDown {
                        keycode: Some(Keycode::C),
                        ..
                    } => self.print_debug(),
                    Event::KeyDown {
                        keycode: Some(Keycode::E),
                        keymod,
                        ..
                    } => {
                        if !self.fast && (keymod & (Mod::LCTRLMOD | Mod::RCTRLMOD)) != Mod::NOMOD {
                            self.fast = true;
                            self.logic_frequency *= 8;
                        }
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::E),
                        ..
                    } => {
                        if self.fast {
                            self.fast = false;
                            self.logic_frequency /= 8;
                        }
                    }
                    Event::KeyUp {
                        keycode: Some(Keycode::LCtrl) | Some(Keycode::RCtrl),
                        ..
                    } => {
                        if self.fast {
                            self.fast = false;
                            self.logic_frequency /= 8;
                        }
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::F),
                        keymod,
                        ..
                    } => {
                        if (keymod & (Mod::LCTRLMOD | Mod::RCTRLMOD)) != Mod::NOMOD {
                            self.toggle_fullscreen()
                        }
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Plus),
                        ..
                    } => self.logic_frequency = self.logic_frequency.saturating_add(400000),
                    Event::KeyDown {
                        keycode: Some(Keycode::Minus),
                        ..
                    } => self.logic_frequency = self.logic_frequency.saturating_sub(400000),
                    Event::KeyDown {
                        keycode: Some(keycode),
                        keymod,
                        ..
                    } => {
                        match keycode {
                            Keycode::Num0
                            | Keycode::Num1
                            | Keycode::Num2
                            | Keycode::Num3
                            | Keycode::Num4
                            | Keycode::Num5
                            | Keycode::Num6
                            | Keycode::Num7
                            | Keycode::Num8
                            | Keycode::Num9 => {
                                let file_path = self.save_name(
                                    keycode as u8 - Keycode::Num0 as u8,
                                    None,
                                    Some(&self.dir_path),
                                );
                                if (keymod & (Mod::LCTRLMOD | Mod::RCTRLMOD)) != Mod::NOMOD {
                                    self.save_state(&file_path);
                                } else {
                                    self.load_state(&file_path);
                                }
                            }
                            _ => {}
                        }
                        if let Some(key) = key_to_pad(keycode) {
                            self.system.key_press(key)
                        }
                    }
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => {
                        if let Some(key) = key_to_pad(keycode) {
                            self.system.key_lift(key)
                        }
                    }
                    Event::DropFile { filename, .. } => {
                        if self.auto_mode {
                            let mode = Cartridge::from_file(&filename).unwrap().gb_mode();
                            self.system.set_mode(mode);
                        }
                        self.system.reset();
                        self.system.load(true).unwrap();
                        self.load_rom(Some(&filename)).unwrap();
                    }
                    _ => (),
                }
            }

            let current_time = self.sdl.as_mut().unwrap().timer_subsystem.ticks();

            if current_time >= self.next_tick_time_i {
                // re-starts the counter cycles with the number of pending cycles
                // from the previous tick and the last frame with the system PPU
                // frame index to be overridden in case there's at least one new frame
                // being drawn in the current tick
                let mut counter_cycles = pending_cycles;
                let mut last_frame = self.system.ppu_frame();
                let mut frame_dirty = false;

                // calculates the number of cycles that are meant to be the target
                // for the current "tick" operation this is basically the current
                // logic frequency divided by the visual one, this operation also
                // takes into account the current Game Boy speed multiplier (GBC)
                let cycle_limit = (self.logic_frequency as f32 * self.system.multiplier() as f32
                    / self.visual_frequency)
                    .round() as u32;

                loop {
                    // limits the number of ticks to the typical number
                    // of cycles expected for the current logic cycle
                    if counter_cycles >= cycle_limit {
                        pending_cycles = counter_cycles - cycle_limit;
                        break;
                    }

                    // runs the Game Boy clock, this operation should
                    // include the advance of both the CPU, PPU, APU
                    // and any other frequency based component of the system
                    counter_cycles += self.system.clock() as u32;

                    // in case a new frame is available from the emulator
                    // then the frame must be pushed into SDL for display
                    if self.system.ppu_frame() != last_frame {
                        // obtains the frame buffer of the Game Boy PPU and uses it
                        // to update the stream texture, that will latter be copied
                        // to the canvas
                        let frame_buffer = self.system.frame_buffer().as_ref();
                        texture.update(None, frame_buffer, width * 3).unwrap();

                        // obtains the index of the current PPU frame, this value
                        // is going to be used to detect for new frame presence
                        last_frame = self.system.ppu_frame();
                        frame_dirty = true;
                    }
                }

                // in case there's new audio data available in the emulator we must
                // handle it, sending it to the audio queue nad clearing the buffer
                if !self.system.audio_buffer().is_empty() {
                    if let Some(audio) = self.audio.as_mut() {
                        let audio_buffer = self
                            .system
                            .audio_buffer()
                            .iter()
                            .map(|v| *v as f32 / VOLUME)
                            .collect::<Vec<f32>>();
                        audio.device.queue_audio(&audio_buffer).unwrap();
                    }
                    self.system.clear_audio_buffer();
                }

                // in case there's at least one new frame that was drawn during
                // during the current tick, then we need to flush it to the canvas,
                // this separation between texture creation and canvas flush prevents
                // resources from being over-used in situations where multiple frames
                // are generated during the same tick cycle
                if frame_dirty {
                    // clears the graphics canvas, making sure that no garbage
                    // pixel data remaining in the pixel buffer, not doing this would
                    // create visual glitches in OSs like Mac OS X
                    self.sdl.as_mut().unwrap().canvas.clear();

                    // copies the texture that was created for the frame (during
                    // the loop part of the tick) to the canvas
                    self.sdl
                        .as_mut()
                        .unwrap()
                        .canvas
                        .copy(&texture, None, None)
                        .unwrap();

                    // presents the canvas effectively updating the screen
                    // information presented to the user
                    self.sdl.as_mut().unwrap().canvas.present();
                }

                // calculates the number of ticks that have elapsed since the
                // last draw operation, this is critical to be able to properly
                // operate the clock of the CPU in frame drop situations, meaning
                // a situation where the system resources are no able to emulate
                // the system on time and frames must be skipped (ticks > 1)
                if self.next_tick_time == 0.0 {
                    self.next_tick_time = current_time as f32;
                }
                let mut ticks = ((current_time as f32 - self.next_tick_time)
                    / ((1.0 / self.visual_frequency) * 1000.0))
                    .ceil() as u8;
                ticks = max(ticks, 1);

                // in case the limited (speed) mode is set then we must calculate
                // a new next tick time reference, this is required to prevent the
                // machine from running too fast (eg: 50x)
                if self.limited() {
                    // updates the next update time reference to the current
                    // time so that it can be used from game loop control
                    self.next_tick_time += (1000.0 / self.visual_frequency) * ticks as f32;
                    self.next_tick_time_i = self.next_tick_time.ceil() as u32;
                }
            }

            let current_time = self.sdl.as_mut().unwrap().timer_subsystem.ticks();
            let pending_time = self.next_tick_time_i.saturating_sub(current_time);
            self.sdl
                .as_mut()
                .unwrap()
                .timer_subsystem
                .delay(pending_time);
        }
    }

    pub fn run_benchmark(&mut self, params: &Benchmark) {
        let count = params.count;
        let cpu_only = params.cpu_only.unwrap_or(false);
        let mut cycles = 0u64;

        if cpu_only {
            self.system.set_all_enabled(false);
        }

        let initial = SystemTime::now();

        for _ in 0..count {
            cycles += self.system.clock() as u64;
        }

        let delta = initial.elapsed().unwrap().as_millis() as f64 / 1000.0;
        let frequency_mhz = cycles as f64 / delta / 1000.0 / 1000.0;

        println!(
            "Took {:.2} seconds to run {} ticks ({} cycles) ({:.2} Mhz)!",
            delta, count, cycles, frequency_mhz
        );
    }

    pub fn run_headless(&mut self, allowed_cycles: Option<u64>) {
        let allowed_cycles = allowed_cycles.unwrap_or(u64::MAX);

        // starts the variable that will control the number of cycles that
        // are going to move (because of overflow) from one tick to another
        let mut pending_cycles = 0u32;

        // allocates space for the loop ticks counter to be used in each
        // iteration cycle
        let mut counter = 0u32;

        // creates the reference instant that is going to be used to
        // calculate the elapsed time
        let reference = Instant::now();

        // creates the total cycles counter that is going to be used
        // to control the number of cycles that have been executed
        let mut total_cycles = 0u64;

        // the main loop to execute the multiple machine clocks, in
        // theory the emulator should keep an infinite loop here
        loop {
            // increments the counter that will keep track
            // on the number of visual ticks since beginning
            counter = counter.wrapping_add(1);

            let current_time = reference.elapsed().as_millis() as u32;

            if current_time >= self.next_tick_time_i {
                // re-starts the counter cycles with the number of pending cycles
                // from the previous tick
                let mut counter_cycles = pending_cycles;

                // calculates the number of cycles that are meant to be the target
                // for the current "tick" operation this is basically the current
                // logic frequency divided by the visual one, this operation also
                // takes into account the current Game Boy speed multiplier (GBC)
                let cycle_limit = (self.logic_frequency as f32 * self.system.multiplier() as f32
                    / self.visual_frequency)
                    .round() as u32;

                loop {
                    // limits the number of ticks to the typical number
                    // of cycles expected for the current logic cycle
                    if counter_cycles >= cycle_limit {
                        pending_cycles = counter_cycles - cycle_limit;
                        break;
                    }

                    // runs the Game Boy clock, this operation should
                    // include the advance of both the CPU, PPU, APU
                    // and any other frequency based component of the system
                    counter_cycles += self.system.clock() as u32;
                }

                // increments the total number of cycles with the cycle limit
                // fot the current tick an in case the total number of cycles
                // exceeds the allowed cycles then the loop is broken
                total_cycles += cycle_limit as u64;
                if total_cycles >= allowed_cycles {
                    break;
                }

                // calculates the number of ticks that have elapsed since the
                // last draw operation, this is critical to be able to properly
                // operate the clock of the CPU in frame drop situations, meaning
                // a situation where the system resources are no able to emulate
                // the system on time and frames must be skipped (ticks > 1)
                if self.next_tick_time == 0.0 {
                    self.next_tick_time = current_time as f32;
                }
                let mut ticks = ((current_time as f32 - self.next_tick_time)
                    / ((1.0 / self.visual_frequency) * 1000.0))
                    .ceil() as u8;
                ticks = max(ticks, 1);

                // in case the limited (speed) mode is set then we must calculate
                // a new next tick time reference, this is required to prevent the
                // machine from running too fast (eg: 50x)
                if self.limited() {
                    // updates the next update time reference to the current
                    // time so that it can be used from game loop control
                    self.next_tick_time += (1000.0 / self.visual_frequency) * ticks as f32;
                    self.next_tick_time_i = self.next_tick_time.ceil() as u32;
                }
            }

            let current_time = reference.elapsed().as_millis() as u32;
            let pending_time = self.next_tick_time_i.saturating_sub(current_time);
            let ten_millis = Duration::from_millis(pending_time as u64);
            thread::sleep(ten_millis);
        }
    }

    /// Obtains the ROM name (file name without extension) so that
    /// it can be used for derivate file names (eg: save files, screenshots).
    fn rom_name(&self) -> &str {
        Path::new(&self.rom_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
    }

    fn image_name(&self, ext: Option<&str>, dir_path: Option<&str>) -> String {
        let ext = ext.unwrap_or("png");
        let dir_path = dir_path.unwrap_or(".");
        Self::best_name(self.rom_name(), ext, dir_path)
    }

    /// Obtains the best possible save file name (ex: `{ROM_NAME}.s0`) taking
    /// into consideration the current directory path and the index of the save.
    fn save_name(&self, index: u8, suffix: Option<&str>, dir_path: Option<&str>) -> String {
        let suffix = suffix.unwrap_or("s");
        let dir_path = dir_path.unwrap_or(".");
        Self::sequence_name(self.rom_name(), index, suffix, dir_path)
    }

    /// Generates a file name for the provided base name, index and suffix
    /// taking into consideration the provided directory path.
    /// The generated file name is `{base}.{suffix}{index}`.
    fn sequence_name(base: &str, index: u8, suffix: &str, dir_path: &str) -> String {
        let file_name = format!("{}.{}{}", base, suffix, index);
        let mut file_buf = PathBuf::from(dir_path);
        file_buf.push(file_name);
        file_buf.to_str().unwrap().to_string()
    }

    /// Tries to obtain the best possible file name for the provided base name
    /// and extension avoiding name collisions with existing files in the
    /// same directory.
    fn best_name(base: &str, ext: &str, dir_path: &str) -> String {
        let mut index = 0_usize;
        let mut name = format!("{}.{}", base, ext);

        let mut path_buf = PathBuf::from(dir_path);
        path_buf.push(&name);

        while path_buf.exists() {
            index += 1;
            name = format!("{}-{}.{}", base, index, ext);
            path_buf = PathBuf::from(dir_path);
            path_buf.push(&name);
        }

        path_buf.to_str().unwrap().to_string()
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("auto"), help = "GB execution mode (ex: dmg, cgb, sgb) to be used")]
    mode: String,

    #[arg(short, long, default_value_t = String::from("printer"), help = "Serial device to be used")]
    device: String,

    #[arg(
        long,
        default_value_t = false,
        help = "If set no boot ROM will be loaded"
    )]
    no_boot: bool,

    #[arg(
        long,
        default_value_t = String::from(""),
        help = "Path to Game Boy boot ROM file to be used in loading stage"
    )]
    boot_rom_path: String,

    #[arg(long, default_value_t = false, help = "If set no PPU will be used")]
    no_ppu: bool,

    #[arg(long, default_value_t = false, help = "If set no APU will be used")]
    no_apu: bool,

    #[arg(long, default_value_t = false, help = "If set no DMA will be used")]
    no_dma: bool,

    #[arg(long, default_value_t = false, help = "If set no timer will be used")]
    no_timer: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Run in benchmark mode, with no UI"
    )]
    benchmark: bool,

    #[arg(
        long,
        default_value_t = 500000000,
        help = "The size of the benchmark in clock ticks"
    )]
    benchmark_count: usize,

    #[arg(long, default_value_t = false, help = "Run benchmark only for the CPU")]
    benchmark_cpu: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Run in headless mode, with no UI"
    )]
    headless: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "If set no CPU speed limit will be imposed"
    )]
    unlimited: bool,

    #[arg(
        long,
        default_value_t = 0,
        help = "Number of CPU cycles to run in headless mode"
    )]
    cycles: u64,

    #[arg(
        long,
        help = "Cheat codes to be applied to the ROM, supports both Game Genie and GameShark"
    )]
    cheats: Vec<String>,

    #[arg(default_value_t = String::from(DEFAULT_ROM_PATH), help = "Path to the ROM file to be loaded")]
    rom_path: String,
}

fn run(args: Args, emulator: &mut Emulator) {
    // determines if the emulator should run in headless mode or
    // not and runs it accordingly, note that if running in headless
    // mode the number of cycles to be run may be specified
    if args.benchmark {
        emulator.run_benchmark(&Benchmark::new(
            args.benchmark_count,
            Some(args.benchmark_cpu),
        ));
    } else if args.headless {
        emulator.run_headless(if args.cycles > 0 {
            Some(args.cycles)
        } else {
            None
        });
    } else {
        emulator.run();
    }
}

fn main() {
    // parses the provided command line arguments and uses them to
    // obtain structured values
    let args = Args::parse();

    // in case the default ROM path is provided and the file does not
    // exist then fails gracefully
    let path = Path::new(&args.rom_path);
    if args.rom_path == DEFAULT_ROM_PATH && !path.exists() {
        println!("No ROM file provided, please provide one using the [ROM_PATH] argument");
        return;
    }

    // tries to build the target mode from the mode argument
    // parsing it if it does not contain the "auto" value
    let mode = if args.mode == "auto" {
        GameBoyMode::Dmg
    } else {
        GameBoyMode::from_string(&args.mode)
    };
    let auto_mode = args.mode == "auto";

    // creates a new Game Boy instance and loads both the boot ROM
    // and the initial game ROM to "start the engine"
    let mut game_boy = GameBoy::new(Some(mode));
    if auto_mode {
        let mode = Cartridge::from_file(&args.rom_path).unwrap().gb_mode();
        game_boy.set_mode(mode);
    }
    let device: Box<dyn SerialDevice> = build_device(&args.device).unwrap();
    game_boy.set_ppu_enabled(!args.no_ppu);
    game_boy.set_apu_enabled(!args.no_apu);
    game_boy.set_dma_enabled(!args.no_dma);
    game_boy.set_timer_enabled(!args.no_timer);
    game_boy.attach_serial(device);
    game_boy
        .load(!args.no_boot && args.boot_rom_path.is_empty())
        .unwrap();
    if args.no_boot {
        game_boy.load_boot_state();
    }
    if !args.boot_rom_path.is_empty() {
        game_boy.load_boot_path(&args.boot_rom_path).unwrap();
    }

    // prints the current version of the emulator (informational message)
    println!("========= {} =========\n{}", Info::name(), game_boy);

    // creates a new generic emulator structure then starts
    // both the video and audio sub-systems, loads default
    // ROM file and starts running it
    let options = EmulatorOptions {
        auto_mode: Some(auto_mode),
        unlimited: Some(args.unlimited),
        features: if args.headless || args.benchmark {
            Some(vec![])
        } else {
            Some(vec!["video", "audio", "no-vsync"])
        },
    };
    let mut emulator = Emulator::new(game_boy, options);
    emulator.start(SCREEN_SCALE);
    emulator.load_rom(Some(&args.rom_path)).unwrap();
    emulator.apply_cheats(&args.cheats);
    emulator.toggle_palette();

    run(args, &mut emulator);
}

fn build_device(device: &str) -> Result<Box<dyn SerialDevice>, Error> {
    match device {
        "null" => Ok(Box::<NullDevice>::default()),
        "stdout" => Ok(Box::<StdoutDevice>::default()),
        "printer" => {
            let mut printer = Box::<PrinterDevice>::default();
            printer.set_callback(|image_buffer| {
                let file_name = format!("printer-{}.png", Utc::now().format("%Y%m%d-%H%M%S"));
                image::save_buffer(
                    Path::new(&file_name),
                    image_buffer,
                    160,
                    (image_buffer.len() / 4 / 160) as u32,
                    ColorType::Rgba8,
                )
                .unwrap();
            });
            Ok(printer)
        }
        _ => Err(Error::InvalidParameter(format!(
            "Unsupported device: {}",
            device
        ))),
    }
}

fn key_to_pad(keycode: Keycode) -> Option<PadKey> {
    match keycode {
        Keycode::Up => Some(PadKey::Up),
        Keycode::Down => Some(PadKey::Down),
        Keycode::Left => Some(PadKey::Left),
        Keycode::Right => Some(PadKey::Right),
        Keycode::Return => Some(PadKey::Start),
        Keycode::Return2 => Some(PadKey::Start),
        Keycode::Space => Some(PadKey::Select),
        Keycode::A => Some(PadKey::A),
        Keycode::S => Some(PadKey::B),
        _ => None,
    }
}
