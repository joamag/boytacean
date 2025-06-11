use gl;
use sdl2::{
    render::Canvas,
    rwops::RWops,
    surface::Surface,
    sys::image,
    ttf::Sdl2TtfContext,
    video::{GLContext, GLProfile, Window},
    AudioSubsystem, EventPump, Sdl, TimerSubsystem, VideoSubsystem,
};
use std::path::Path;

use crate::shader;

/// Structure that provides the complete set of SDL Graphics
/// and Sound syb-system ready to be used by the overall
/// emulator infrastructure.
pub struct SdlSystem {
    pub canvas: Option<Canvas<Window>>,
    pub window: Option<Window>,
    pub video_subsystem: VideoSubsystem,
    pub timer_subsystem: TimerSubsystem,
    pub audio_subsystem: AudioSubsystem,
    pub event_pump: EventPump,
    pub ttf_context: Sdl2TtfContext,
    pub gl_context: GLContext,
    pub shader_program: Option<u32>,
    pub gl_texture: Option<u32>,
    pub gl_vao: Option<u32>,
    pub gl_vbo: Option<u32>,
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
        video_subsystem
            .gl_attr()
            .set_context_profile(GLProfile::Core);
        video_subsystem.gl_attr().set_context_version(3, 3);

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
        /*let mut canvas_builder = window.into_canvas();
        if accelerated {
            canvas_builder = canvas_builder.accelerated();
        }
        if vsync {
            canvas_builder = canvas_builder.present_vsync();
        }
        let mut canvas = canvas_builder.build().unwrap();
        canvas.set_logical_size(width, height).unwrap();
        canvas.clear();*/

        let gl_context = window.gl_create_context().unwrap();
        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);

        Self {
            canvas: None,
            window: Some(window),
            video_subsystem,
            timer_subsystem,
            audio_subsystem,
            event_pump,
            ttf_context,
            gl_context,
            shader_program: None,
            gl_texture: None,
            gl_vao: None,
            gl_vbo: None,
        }
    }

    pub fn window(&self) -> &Window {
        if let Some(canvas) = &self.canvas {
            canvas.window()
        } else {
            self.window.as_ref().unwrap()
        }
    }

    pub fn window_mut(&mut self) -> &mut Window {
        if let Some(canvas) = &mut self.canvas {
            canvas.window_mut()
        } else {
            self.window.as_mut().unwrap()
        }
    }

    pub fn load_fragment_shader<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let program = shader::load_shader_program(path.as_ref().to_str().unwrap())?;
        unsafe {
            let mut vao = 0;
            let mut vbo = 0;
            let mut texture = 0;
            let vertices: [f32; 16] = [
                -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ];

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (std::mem::size_of_val(&vertices)) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            let stride = 4 * std::mem::size_of::<f32>() as i32;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (2 * std::mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);

            self.gl_texture = Some(texture);
            self.gl_vao = Some(vao);
            self.gl_vbo = Some(vbo);
        }
        self.shader_program = Some(program);
        Ok(())
    }

    pub fn render_frame_with_shader(&mut self, pixels: &[u8], width: u32, height: u32) {
        if self.shader_program.is_none() {
            return;
        }
        unsafe {
            let (dw, dh) = self.window.as_ref().unwrap().drawable_size();
            gl::Viewport(0, 0, dw as i32, dh as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.shader_program.unwrap());

            let texture = self.gl_texture.unwrap();
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as i32,
                width as i32,
                height as i32,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                pixels.as_ptr() as *const _,
            );

            let loc_image = gl::GetUniformLocation(
                self.shader_program.unwrap(),
                b"image\0".as_ptr() as *const _,
            );
            let loc_in = gl::GetUniformLocation(
                self.shader_program.unwrap(),
                b"input_resolution\0".as_ptr() as *const _,
            );
            let loc_out = gl::GetUniformLocation(
                self.shader_program.unwrap(),
                b"output_resolution\0".as_ptr() as *const _,
            );
            gl::Uniform1i(loc_image, 0);
            gl::Uniform2f(loc_in, width as f32, height as f32);
            gl::Uniform2f(loc_out, dw as f32, dh as f32);

            gl::BindVertexArray(self.gl_vao.unwrap());
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::BindVertexArray(0);
        }
        self.window().gl_make_current(&self.gl_context).unwrap();
        /*unsafe {
            gl::ClearColor(0.0, 1.0, 0.0, 1.0); // green
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }*/
        self.window().gl_swap_window();
    }
}

/// Creates an SDL2 Surface structure from the provided
/// bytes that represent an image (eg: a PNG image buffer).
pub fn surface_from_bytes(bytes: &[u8]) -> Surface<'_> {
    unsafe {
        let rw_ops = RWops::from_bytes(bytes).unwrap();
        let raw_surface = image::IMG_Load_RW(rw_ops.raw(), 0);
        Surface::from_ll(raw_surface)
    }
}
