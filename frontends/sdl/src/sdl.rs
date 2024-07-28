use sdl2::{
    render::Canvas, rwops::RWops, surface::Surface, sys::image, ttf::Sdl2TtfContext, video::Window,
    AudioSubsystem, EventPump, Sdl, TimerSubsystem, VideoSubsystem,
};

/// Structure that provides the complete set of SDL Graphics
/// and Sound syb-system ready to be used by the overall
/// emulator infrastructure.
pub struct SdlSystem {
    pub canvas: Canvas<Window>,
    pub video_subsystem: VideoSubsystem,
    pub timer_subsystem: TimerSubsystem,
    pub audio_subsystem: AudioSubsystem,
    pub event_pump: EventPump,
    pub ttf_context: Sdl2TtfContext,
}

impl SdlSystem {
    /// Start the SDL sub-system and all of its structure and returns
    /// a structure with all the needed stuff to handle SDL graphics
    /// and sound.
    pub fn new(
        sdl: &Sdl,
        title: &str,
        width: u32,
        height: u32,
        scale: f32,
        accelerated: bool,
        vsync: bool,
    ) -> Self {
        // initializes the SDL sub-system, making it ready to be
        // used for display of graphics and audio
        let video_subsystem = sdl.video().unwrap();
        let timer_subsystem = sdl.timer().unwrap();
        let audio_subsystem = sdl.audio().unwrap();
        let event_pump = sdl.event_pump().unwrap();

        // initializes the fonts context to be used
        // in the loading of fonts (supports TTF fonts)
        let ttf_context = sdl2::ttf::init().unwrap();

        // creates the system window that is going to be used to
        // show the emulator and sets it to the central are o screen
        let window = video_subsystem
            .window(
                title,
                (scale * width as f32) as u32,
                (scale * height as f32) as u32,
            )
            .resizable()
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        // creates a canvas (according to spec) to be used in the drawing
        // then clears it so that is can be presented empty initially
        let mut canvas_builder = window.into_canvas();
        if accelerated {
            canvas_builder = canvas_builder.accelerated();
        }
        if vsync {
            canvas_builder = canvas_builder.present_vsync();
        }
        let mut canvas = canvas_builder.build().unwrap();
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
/// bytes that represent an image (eg: a PNG image buffer).
pub fn surface_from_bytes(bytes: &[u8]) -> Surface {
    unsafe {
        let rw_ops = RWops::from_bytes(bytes).unwrap();
        let raw_surface = image::IMG_Load_RW(rw_ops.raw(), 0);
        Surface::from_ll(raw_surface)
    }
}
