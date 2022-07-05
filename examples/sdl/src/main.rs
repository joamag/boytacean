use boytacean::{
    gb::GameBoy,
    ppu::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
};
use sdl2::{
    event::Event, image::LoadSurface, pixels::PixelFormatEnum, surface::Surface, video::Window,
    AudioSubsystem, EventPump, TimerSubsystem, VideoSubsystem,
};

/// The base title to be used in the window.
static TITLE: &'static str = "Boytacean";

pub struct Graphics {
    window: Window,
    video_subsystem: VideoSubsystem,
    timer_subsystem: TimerSubsystem,
    audio_subsystem: AudioSubsystem,
    event_pump: EventPump,
}

fn start_sdl() -> Graphics {
    // initializes the SDL sub-system, making it ready to be
    // used for display of graphics and audio
    let sdl = sdl2::init().unwrap();
    let video_subsystem = sdl.video().unwrap();
    let timer_subsystem = sdl.timer().unwrap();
    let audio_subsystem = sdl.audio().unwrap();
    let event_pump = sdl.event_pump().unwrap();

    // initialized the fonts context to be used
    // in the loading of fonts
    let ttf_context = sdl2::ttf::init().unwrap();

    // creates the system window that is going to be used to
    // show the emulator and sets it to the central are o screen
    let mut window = video_subsystem
        .window(
            TITLE,
            2 as u32 * DISPLAY_WIDTH as u32, //@todo check screen scale
            2 as u32 * DISPLAY_HEIGHT as u32, //@todo check screen scale
        )
        .resizable()
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    Graphics {
        window: window,
        video_subsystem: video_subsystem,
        timer_subsystem: timer_subsystem,
        audio_subsystem: audio_subsystem,
        event_pump: event_pump,
    }
}

fn main() {
    let mut graphics = start_sdl();

    // updates the icon of the window to reflect the image
    // and style of the emulator
    let surface = Surface::from_file("./res/icon.png").unwrap();
    graphics.window.set_icon(&surface);

    let mut canvas = graphics.window.into_canvas().accelerated().build().unwrap();
    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();

    // creates the texture streaming that is going to be used
    // as the target for the pixel buffer
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGB24,
            DISPLAY_WIDTH as u32,
            DISPLAY_HEIGHT as u32,
        )
        .unwrap();

    // creates a new Game Boy instance and loads both the boot ROM
    // and the initial game ROM to "start the engine"
    let mut game_boy = GameBoy::new();
    game_boy.load_boot_dmg_bootix();

    //game_boy.load_rom_file("../../res/roms.prop/tetris.gb");
    //game_boy.load_rom_file("../../res/roms.prop/alleyway.gb");

    //game_boy.load_rom_file("../../res/roms/firstwhite.gb");
    //game_boy.load_rom_file("../../res/roms/opus5.gb");

    //game_boy.load_rom_file("../../res/roms/paradius/cpu/01-special.gb"); // PASSED
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/02-interrupts.gb"); // NO FINISH
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/03-op sp,hl.gb"); // PASSED
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/04-op r,imm.gb"); // PASSED
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/05-op rp.gb"); // PASSED
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/06-ld r,r.gb"); // PASSED
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/07-jr,jp,call,ret,rst.gb"); // PASSED
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/08-misc instrs.gb");  // PASSED
    game_boy.load_rom_file("../../res/roms/paradius/cpu/09-op r,r.gb"); // NO FINISH
    //game_boy.load_rom_file("../../res/roms/paradius/cpu/11-op a,(hl).gb"); // NO FINISH

    'main: loop {
        while let Some(event) = graphics.event_pump.poll_event() {
            match event {
                Event::Quit { .. } => break 'main,
                _ => (),
            }
        }

        let mut counter_ticks = 0u32;

        loop {
            // limits the number of ticks to the typical number
            // of ticks required to do a complete PPU draw
            if counter_ticks >= 70224 {
                break;
            }

            // runs the Game Boy clock, this operations should
            // include the advance of both the CPU and the PPU
            counter_ticks += game_boy.clock() as u32;
        }

        // obtains the frame buffer of the Game Boy PPU and uses it
        // to update the stream texture, copying it then to the canvas
        let frame_buffer = game_boy.frame_buffer().as_ref();
        texture
            .update(None, frame_buffer, DISPLAY_WIDTH as usize * 3)
            .unwrap();
        canvas.copy(&texture, None, None).unwrap();

        // presents the canvas effectively updating the screen
        // information presented to the user
        canvas.present();

        // @todo this must be improved with proper timestamps
        graphics.timer_subsystem.delay(17);
    }
}
