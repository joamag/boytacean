#![allow(clippy::uninlined_format_args)]

pub mod data;
pub mod util;

use boytacean::{
    gb::GameBoy,
    pad::PadKey,
    ppu::{PpuMode, DISPLAY_HEIGHT, DISPLAY_WIDTH},
};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum};
use util::Graphics;

use crate::util::surface_from_bytes;

/// The ration at which the logic of the Game Boy is
/// going to be run, increasing this value will provide
/// better emulator accuracy, please keep in mind that
/// the PPU will keep running at the same speed.
const LOGIC_RATIO: f32 = 2.0;

/// The scale at which the screen is going to be drawn
/// meaning the ratio between Game Boy resolution and
/// the window size to be displayed.
const SCREEN_SCALE: f32 = 2.0;

/// The base title to be used in the window.
static TITLE: &str = "Boytacean";

pub struct Emulator {
    system: GameBoy,
    graphics: Graphics,
    logic_ratio: f32,
    next_tick_time: f32,
    next_tick_time_i: u32,
}

impl Emulator {
    pub fn new(system: GameBoy, screen_scale: f32) -> Self {
        Self {
            system,
            graphics: Graphics::new(
                TITLE,
                DISPLAY_WIDTH as u32,
                DISPLAY_HEIGHT as u32,
                screen_scale,
            ),
            logic_ratio: LOGIC_RATIO,
            next_tick_time: 0.0,
            next_tick_time_i: 0,
        }
    }

    pub fn load_rom(&mut self, path: &str) {
        let rom = self.system.load_rom_file(path);
        println!(
            "========= Cartridge =========\n{}\n=============================\n",
            rom
        );
        self.graphics
            .window_mut()
            .set_title(format!("{} - {}", TITLE, rom.title()).as_str())
            .unwrap();
    }

    pub fn run(&mut self) {
        // updates the icon of the window to reflect the image
        // and style of the emulator
        let surface = surface_from_bytes(&data::ICON);
        self.graphics.window_mut().set_icon(&surface);

        // creates an accelerated canvas to be used in the drawing
        // then clears it and presents it
        self.graphics.canvas.present();

        // creates a texture creator for the current canvas, required
        // for the creation of dynamic and static textures
        let texture_creator = self.graphics.canvas.texture_creator();

        // creates the texture streaming that is going to be used
        // as the target for the pixel buffer
        let mut texture = texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGB24,
                DISPLAY_WIDTH as u32,
                DISPLAY_HEIGHT as u32,
            )
            .unwrap();

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
            while let Some(event) = self.graphics.event_pump.poll_event() {
                match event {
                    Event::Quit { .. } => break 'main,
                    Event::KeyDown {
                        keycode: Some(keycode),
                        ..
                    } => match key_to_pad(keycode) {
                        Some(key) => self.system.key_press(key),
                        None => (),
                    },
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => match key_to_pad(keycode) {
                        Some(key) => self.system.key_lift(key),
                        None => (),
                    },
                    Event::DropFile { filename, .. } => {
                        self.system.reset();
                        self.system.load_boot_default();
                        self.load_rom(&filename);
                    }
                    _ => (),
                }
            }

            let current_time = self.graphics.timer_subsystem.ticks();

            let mut counter_cycles = 0u32;
            let mut last_frame = 0xffffu16;

            if current_time >= self.next_tick_time_i {
                let cycle_limit = (GameBoy::LCD_CYCLES as f32 / self.logic_ratio as f32) as u32;

                loop {
                    // limits the number of ticks to the typical number
                    // of cycles expected for the current logic cycle
                    if counter_cycles >= cycle_limit {
                        break;
                    }

                    // runs the Game Boy clock, this operations should
                    // include the advance of both the CPU and the PPU
                    counter_cycles += self.system.clock() as u32;

                    if self.system.ppu_mode() == PpuMode::VBlank
                        && self.system.ppu_frame() != last_frame
                    {
                        // clears the graphics canvas, making sure that no garbage
                        // pixel data remaining in the pixel buffer, not doing this would
                        // create visual glitches in OSs like Mac OS X
                        self.graphics.canvas.clear();

                        // obtains the frame buffer of the Game Boy PPU and uses it
                        // to update the stream texture, copying it then to the canvas
                        let frame_buffer = self.system.frame_buffer().as_ref();
                        texture
                            .update(None, frame_buffer, DISPLAY_WIDTH as usize * 3)
                            .unwrap();
                        self.graphics.canvas.copy(&texture, None, None).unwrap();

                        // presents the canvas effectively updating the screen
                        // information presented to the user
                        self.graphics.canvas.present();

                        // obtains the index of the current PPU frame, this value
                        // is going to be used to detect for new frame presence
                        last_frame = self.system.ppu_frame();
                    }
                }

                let logic_frequency =
                    GameBoy::CPU_FREQ as f32 / GameBoy::LCD_CYCLES as f32 * self.logic_ratio;

                // updates the next update time reference to the current
                // time so that it can be used from game loop control
                self.next_tick_time += 1000.0 / logic_frequency as f32;
                self.next_tick_time_i = self.next_tick_time.ceil() as u32;
            }

            let current_time = self.graphics.timer_subsystem.ticks();
            let pending_time = self.next_tick_time_i.saturating_sub(current_time);
            self.graphics.timer_subsystem.delay(pending_time);
        }
    }
}

fn main() {
    // creates a new Game Boy instance and loads both the boot ROM
    // and the initial game ROM to "start the engine"
    let mut game_boy = GameBoy::new();
    game_boy.load_boot_default();

    // creates a new generic emulator structure loads the default
    // ROM file and starts running it
    let mut emulator = Emulator::new(game_boy, SCREEN_SCALE);
    emulator.load_rom("../../res/roms/pocket.gb");
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
