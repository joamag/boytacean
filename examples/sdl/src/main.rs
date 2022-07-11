pub mod data;
pub mod util;

use boytacean::{
    gb::GameBoy,
    pad::PadKey,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
};
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum};
use util::Graphics;

use crate::util::surface_from_bytes;

const VISUAL_HZ: u32 = 60;
const SCREEN_SCALE: f32 = 2.0;

/// The base title to be used in the window.
static TITLE: &'static str = "Boytacean";

pub struct Emulator {
    system: GameBoy,
    graphics: Graphics,
    visual_frequency: u32,
    next_tick_time: f32,
    next_tick_time_i: u32,
}

impl Emulator {
    pub fn new(system: GameBoy, screen_scale: f32) -> Self {
        Self {
            system: system,
            graphics: Graphics::new(
                TITLE,
                DISPLAY_WIDTH as u32,
                DISPLAY_HEIGHT as u32,
                screen_scale,
            ),
            visual_frequency: VISUAL_HZ,
            next_tick_time: 0.0,
            next_tick_time_i: 0,
        }
    }

    pub fn load_rom(&mut self, path: &str) {
        let rom = self.system.load_rom_file(path);
        println!("==== Cartridge ====\n{}\n===================", rom);
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

            let mut counter_ticks = 0u32;

            if current_time >= self.next_tick_time_i {
                loop {
                    // limits the number of ticks to the typical number
                    // of ticks required to do a complete PPU draw
                    if counter_ticks >= 70224 {
                        break;
                    }

                    // runs the Game Boy clock, this operations should
                    // include the advance of both the CPU and the PPU
                    counter_ticks += self.system.clock() as u32;
                }

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

                // updates the next update time reference to the current
                // time so that it can be used from game loop control
                self.next_tick_time += 1000.0 / self.visual_frequency as f32;
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

    let mut emulator = Emulator::new(game_boy, SCREEN_SCALE);
    emulator.load_rom("../../res/roms.prop/dr_mario.gb");
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
