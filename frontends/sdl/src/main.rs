#![allow(clippy::uninlined_format_args)]

pub mod audio;
pub mod data;
pub mod sdl;

use audio::Audio;
use boytacean::{
    devices::{printer::PrinterDevice, stdout::StdoutDevice},
    gb::{AudioProvider, GameBoy, GameBoyMode},
    pad::PadKey,
    ppu::{PaletteInfo, PpuMode, DISPLAY_HEIGHT, DISPLAY_WIDTH},
    rom::Cartridge,
    serial::{NullDevice, SerialDevice},
};
use chrono::Utc;
use clap::Parser;
use image::ColorType;
use sdl::{surface_from_bytes, SdlSystem};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, Sdl};
use std::{
    cmp::max,
    path::Path,
    thread,
    time::{Duration, Instant, SystemTime},
};

/// The scale at which the screen is going to be drawn
/// meaning the ratio between Game Boy resolution and
/// the window size to be displayed.
const SCREEN_SCALE: f32 = 2.0;

/// The base title to be used in the window.
const TITLE: &str = "Boytacean";

/// Base audio volume to be used as the basis of the
/// amplification level of the volume
const VOLUME: f32 = 64.0;

pub struct Benchmark {
    count: usize,
}

impl Benchmark {
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

impl Default for Benchmark {
    fn default() -> Self {
        Self::new(50000000)
    }
}

pub struct EmulatorOptions {
    auto_mode: Option<bool>,
    unlimited: Option<bool>,
    features: Option<Vec<&'static str>>,
}

pub struct Emulator {
    system: GameBoy,
    auto_mode: bool,
    unlimited: bool,
    sdl: Option<SdlSystem>,
    audio: Option<Audio>,
    title: &'static str,
    rom_path: String,
    logic_frequency: u32,
    visual_frequency: f32,
    next_tick_time: f32,
    next_tick_time_i: u32,
    features: Vec<&'static str>,
    palettes: [PaletteInfo; 7],
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
            title: TITLE,
            rom_path: String::from("invalid"),
            logic_frequency: GameBoy::CPU_FREQ,
            visual_frequency: GameBoy::VISUAL_FREQ,
            next_tick_time: 0.0,
            next_tick_time_i: 0,
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

    #[cfg(not(feature = "slow"))]
    pub fn start_base(&mut self) {}

    #[cfg(feature = "slow")]
    pub fn start_base(&mut self) {
        self.logic_frequency = 100;
    }

    pub fn start_graphics(&mut self, sdl: &Sdl, screen_scale: f32) {
        self.sdl = Some(SdlSystem::new(
            sdl,
            self.title,
            DISPLAY_WIDTH as u32,
            DISPLAY_HEIGHT as u32,
            screen_scale,
            !self.features.contains(&"no-accelerated"),
            !self.features.contains(&"no-vsync"),
        ));
    }

    pub fn start_audio(&mut self, sdl: &Sdl) {
        self.audio = Some(Audio::new(sdl));
    }

    pub fn load_rom(&mut self, path: Option<&str>) {
        let path_res = path.unwrap_or(&self.rom_path);
        let rom: &boytacean::rom::Cartridge = self.system.load_rom_file(path_res);
        println!(
            "========= Cartridge =========\n{}\n=============================",
            rom
        );
        if let Some(ref mut sdl) = self.sdl {
            sdl.window_mut()
                .set_title(format!("{} [{}]", self.title, rom.title()).as_str())
                .unwrap();
        }
        self.rom_path = String::from(path_res);
    }

    pub fn reset(&mut self) {
        self.system.reset();
        self.system.load(true);
        self.load_rom(None);
    }

    pub fn benchmark(&mut self, params: Benchmark) {
        println!("Going to run benchmark...");

        let count = params.count;
        let mut cycles = 0;

        let initial = SystemTime::now();

        for _ in 0..count {
            cycles += self.system.clock() as u32;
        }

        let delta = initial.elapsed().unwrap().as_millis() as f32 / 1000.0;
        let frequency_mhz = cycles as f32 / delta / 1000.0 / 1000.0;

        println!(
            "Took {:.2} seconds to run {} ticks ({} cycles) ({:.2} Mhz)!",
            delta, count, cycles, frequency_mhz
        );
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

    pub fn limited(&self) -> bool {
        !self.unlimited
    }

    pub fn run(&mut self) {
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
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                DISPLAY_WIDTH as u32,
                DISPLAY_HEIGHT as u32,
            )
            .unwrap();

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
                    } => self.reset(),
                    Event::KeyDown {
                        keycode: Some(Keycode::B),
                        ..
                    } => self.benchmark(Benchmark::default()),
                    Event::KeyDown {
                        keycode: Some(Keycode::T),
                        ..
                    } => self.toggle_audio(),
                    Event::KeyDown {
                        keycode: Some(Keycode::P),
                        ..
                    } => self.toggle_palette(),
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
                        ..
                    } => {
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
                            let mode = Cartridge::from_file(&filename).gb_mode();
                            self.system.set_mode(mode);
                        }
                        self.system.reset();
                        self.system.load(true);
                        self.load_rom(Some(&filename));
                    }
                    _ => (),
                }
            }

            let current_time = self.sdl.as_mut().unwrap().timer_subsystem.ticks();

            if current_time >= self.next_tick_time_i {
                // re-starts the counter cycles with the number of pending cycles
                // from the previous tick and the last frame with a dummy value
                // meant to be overridden in case there's at least one new frame
                // being drawn in the current tick
                let mut counter_cycles = pending_cycles;
                let mut last_frame = 0xffffu16;

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

                    // in case a V-Blank state has been reached a new frame is available
                    // then the frame must be pushed into SDL for display
                    if self.system.ppu_mode() == PpuMode::VBlank
                        && self.system.ppu_frame() != last_frame
                    {
                        // obtains the frame buffer of the Game Boy PPU and uses it
                        // to update the stream texture, that will latter be copied
                        // to the canvas
                        let frame_buffer = self.system.frame_buffer().as_ref();
                        texture
                            .update(None, frame_buffer, DISPLAY_WIDTH * 3)
                            .unwrap();

                        // obtains the index of the current PPU frame, this value
                        // is going to be used to detect for new frame presence
                        last_frame = self.system.ppu_frame();
                    }

                    // in case the audio subsystem is enabled, then the audio buffer
                    // must be queued into the SDL audio subsystem
                    if let Some(audio) = self.audio.as_mut() {
                        let audio_buffer = self
                            .system
                            .audio_buffer()
                            .iter()
                            .map(|v| *v as f32 / VOLUME)
                            .collect::<Vec<f32>>();
                        audio.device.queue_audio(&audio_buffer).unwrap();
                    }

                    // clears the audio buffer to prevent it from
                    // "exploding" in size, this is required GC operation
                    self.system.clear_audio_buffer();
                }

                // in case there's at least one new frame that was drawn during
                // during the current tick, then we need to flush it to the canvas,
                // this separation between texture creation and canvas flush prevents
                // resources from being over-used in situations where multiple frames
                // are generated during the same tick cycle
                if last_frame != 0xffffu16 {
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

    pub fn run_headless(&mut self) {
        // starts the variable that will control the number of cycles that
        // are going to move (because of overflow) from one tick to another
        let mut pending_cycles = 0u32;

        // allocates space for the loop ticks counter to be used in each
        // iteration cycle
        let mut counter = 0u32;

        let reference = Instant::now();

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
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("auto"))]
    mode: String,

    #[arg(short, long, default_value_t = String::from("printer"))]
    device: String,

    #[arg(long, default_value_t = false)]
    no_ppu: bool,

    #[arg(long, default_value_t = false)]
    no_apu: bool,

    #[arg(long, default_value_t = false)]
    no_dma: bool,

    #[arg(long, default_value_t = false)]
    no_timer: bool,

    #[arg(long, default_value_t = false)]
    headless: bool,

    #[arg(long, default_value_t = false)]
    unlimited: bool,

    #[arg(short, long, default_value_t = String::from("../../res/roms/demo/pocket.gb"))]
    rom_path: String,
}

fn main() {
    // parses the provided command line arguments and uses them to
    // obtain structured values
    let args = Args::parse();
    let mode: GameBoyMode = if args.mode == "auto" {
        GameBoyMode::Dmg
    } else {
        GameBoyMode::from_string(&args.mode)
    };
    let auto_mode = args.mode == "auto";

    // creates a new Game Boy instance and loads both the boot ROM
    // and the initial game ROM to "start the engine"
    let mut game_boy = GameBoy::new(mode);
    let device = build_device(&args.device);
    game_boy.set_ppu_enabled(!args.no_ppu);
    game_boy.set_apu_enabled(!args.no_apu);
    game_boy.set_dma_enabled(!args.no_dma);
    game_boy.set_timer_enabled(!args.no_timer);
    game_boy.attach_serial(device);
    game_boy.load(true);

    // prints the current version of the emulator (informational message)
    println!("========= Boytacean =========\n{}", game_boy);

    // creates a new generic emulator structure then starts
    // both the video and audio sub-systems, loads default
    // ROM file and starts running it
    let options = EmulatorOptions {
        auto_mode: Some(auto_mode),
        unlimited: Some(args.unlimited),
        features: if args.headless {
            Some(vec![])
        } else {
            Some(vec!["video", "audio", "no-vsync"])
        },
    };
    let mut emulator = Emulator::new(game_boy, options);
    emulator.start(SCREEN_SCALE);
    emulator.load_rom(Some(&args.rom_path));
    emulator.toggle_palette();
    if args.headless {
        emulator.run_headless();
    } else {
        emulator.run();
    }
}

fn build_device(device: &str) -> Box<dyn SerialDevice> {
    match device {
        "null" => Box::<NullDevice>::default(),
        "stdout" => Box::<StdoutDevice>::default(),
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
            printer
        }
        _ => panic!("Unsupported device: {}", device),
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
