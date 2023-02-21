use sdl2::{
    render::Canvas, rwops::RWops, surface::Surface, sys::image, ttf::Sdl2TtfContext, video::Window,
    AudioSubsystem, EventPump, TimerSubsystem, VideoSubsystem,
};

/// Structure that provides the complete set of Graphics
/// and Sound syb-system ready to be used by the overall
/// emulator infrastructure.
pub struct Graphics {
    pub canvas: Canvas<Window>,
    pub video_subsystem: VideoSubsystem,
    pub timer_subsystem: TimerSubsystem,
    pub audio_subsystem: AudioSubsystem,
    pub event_pump: EventPump,
    pub ttf_context: Sdl2TtfContext,
}

impl Graphics {
    /// Start the SDL sub-system and all of its structure and returns
    /// a structure with all the needed stuff to handle SDL graphics
    /// and sound.
    pub fn new(title: &str, width: u32, height: u32, scale: f32) -> Self {
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
        let window = video_subsystem
            .window(title, scale as u32 * width, scale as u32 * height)
            .resizable()
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        // creates an accelerated canvas to be used in the drawing
        // then clears it so that is can be presented empty initially
        let mut canvas = window.into_canvas().accelerated().build().unwrap();
        canvas.set_logical_size(width, height).unwrap();
        canvas.clear();

        Self {
            canvas,
            video_subsystem,
            timer_subsystem,
            audio_subsystem,
            event_pump,
            ttf_context,
        }
    }

    pub fn window(&self) -> &Window {
        self.canvas.window()
    }

    pub fn window_mut(&mut self) -> &mut Window {
        self.canvas.window_mut()
    }
}

/// Creates an SDL2 Surface structure from the provided
/// bytes that represent an image (eg: an PNG image buffer).
pub fn surface_from_bytes(bytes: &[u8]) -> Surface {
    unsafe {
        let rw_ops = RWops::from_bytes(bytes).unwrap();
        let raw_surface = image::IMG_Load_RW(rw_ops.raw(), 0);
        Surface::from_ll(raw_surface)
    }
}
