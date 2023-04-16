#![allow(clippy::uninlined_format_args)]

pub mod audio;
pub mod data;
pub mod graphics;

use audio::Audio;
use boytacean::{
    gb::{AudioProvider, GameBoy},
    pad::PadKey,
    ppu::{PaletteInfo, PpuMode, DISPLAY_HEIGHT, DISPLAY_WIDTH},
};
use graphics::{surface_from_bytes, Graphics};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, Sdl};
use std::{cmp::max, time::SystemTime};

/// The scale at which the screen is going to be drawn
/// meaning the ratio between Game Boy resolution and
/// the window size to be displayed.
const SCREEN_SCALE: f32 = 2.0;

/// The base title to be used in the window.
static TITLE: &str = "Boytacean";

/// Base audio volume to be used as the basis of the
/// amplification level of the volume
static VOLUME: f32 = 64.0;

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

pub struct Emulator {
    system: GameBoy,
    graphics: Option<Graphics>,
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
    pub fn new(system: GameBoy) -> Self {
        Self {
            system,
            graphics: None,
            audio: None,
            title: TITLE,
            rom_path: String::from("invalid"),
            logic_frequency: GameBoy::CPU_FREQ,
            visual_frequency: GameBoy::VISUAL_FREQ,
            next_tick_time: 0.0,
            next_tick_time_i: 0,
            features: vec!["video", "audio", "no-vsync"],
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
        let sdl = sdl2::init().unwrap();
        if self.features.contains(&"video") {
            self.start_graphics(&sdl, screen_scale);
        }
        if self.features.contains(&"audio") {
            self.start_audio(&sdl);
        }
    }

    pub fn start_graphics(&mut self, sdl: &Sdl, screen_scale: f32) {
        self.graphics = Some(Graphics::new(
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
        let rom = self.system.load_rom_file(path_res);
        println!(
            "========= Cartridge =========\n{}\n=============================",
            rom
        );
        self.graphics
            .as_mut()
            .unwrap()
            .window_mut()
            .set_title(format!("{} [{}]", self.title, rom.title()).as_str())
            .unwrap();
        self.rom_path = String::from(path_res);
    }

    pub fn reset(&mut self) {
        self.system.reset();
        self.system.load_boot_default();
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
            "Took {:.2} seconds to run {} ticks ({:.2} Mhz)!",
            delta, count, frequency_mhz
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

    pub fn run(&mut self) {
        // updates the icon of the window to reflect the image
        // and style of the emulator
        let surface = surface_from_bytes(&data::ICON);
        self.graphics
            .as_mut()
            .unwrap()
            .window_mut()
            .set_icon(&surface);

        // creates an accelerated canvas to be used in the drawing
        // then clears it and presents it
        self.graphics.as_mut().unwrap().canvas.present();

        // creates a texture creator for the current canvas, required
        // for the creation of dynamic and static textures
        let texture_creator = self.graphics.as_mut().unwrap().canvas.texture_creator();

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
            while let Some(event) = self.graphics.as_mut().unwrap().event_pump.poll_event() {
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
                        self.system.reset();
                        self.system.load_boot_default();
                        self.load_rom(Some(&filename));
                    }
                    _ => (),
                }
            }

            let current_time = self.graphics.as_mut().unwrap().timer_subsystem.ticks();

            if current_time >= self.next_tick_time_i {
                // re-starts the counter cycles with the number of pending cycles
                // from the previous tick and the last frame with a dummy value
                // meant to be overridden in case there's at least one new frame
                // being drawn in the current tick
                let mut counter_cycles = pending_cycles;
                let mut last_frame = 0xffffu16;

                // calculates the number of cycles that are meant to be the target
                // for the current "tick" operation this is basically the current
                // logic frequency divided by the visual one
                let cycle_limit =
                    (self.logic_frequency as f32 / self.visual_frequency).round() as u32;

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

                    if let Some(audio) = self.audio.as_mut() {
                        // obtains the new audio buffer and queues it into the audio
                        // subsystem ready to be processed
                        let audio_buffer = self
                            .system
                            .audio_buffer()
                            .iter()
                            .map(|v| *v as f32 / VOLUME)
                            .collect::<Vec<f32>>();
                        audio.device.queue_audio(&audio_buffer).unwrap();
                    }

                    // clears the audio buffer to prevent it from
                    // "exploding" in size
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
                    self.graphics.as_mut().unwrap().canvas.clear();

                    // copies the texture that was created for the frame (during
                    // the loop part of the tick) to the canvas
                    self.graphics
                        .as_mut()
                        .unwrap()
                        .canvas
                        .copy(&texture, None, None)
                        .unwrap();

                    // presents the canvas effectively updating the screen
                    // information presented to the user
                    self.graphics.as_mut().unwrap().canvas.present();
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

                // updates the next update time reference to the current
                // time so that it can be used from game loop control
                self.next_tick_time += (1000.0 / self.visual_frequency) * ticks as f32;
                self.next_tick_time_i = self.next_tick_time.ceil() as u32;
            }

            let current_time = self.graphics.as_mut().unwrap().timer_subsystem.ticks();
            let pending_time = self.next_tick_time_i.saturating_sub(current_time);
            self.graphics
                .as_mut()
                .unwrap()
                .timer_subsystem
                .delay(pending_time);
        }
    }
}

fn main() {
    // creates a new Game Boy instance and loads both the boot ROM
    // and the initial game ROM to "start the engine"
    let mut game_boy = GameBoy::new();
    game_boy.attach_printer_serial();
    game_boy.load_boot_default();

    // creates a new generic emulator structure then starts
    // both the video and audio sub-systems, loads default
    // ROM file and starts running it
    let mut emulator = Emulator::new(game_boy);
    emulator.start(SCREEN_SCALE);
    emulator.load_rom(Some("../../res/roms/test/gbprinter.gb"));
    emulator.toggle_palette();
    emulator.run();
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
