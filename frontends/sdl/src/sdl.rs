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

use crate::shader;

/// Structure that provides the complete set of SDL Graphics
/// and Sound syb-system ready to be used by the overall
/// emulator infrastructure.
pub struct SdlSystem {
    pub canvas: Option<Canvas<Window>>,

    /// The window that is going to be used to display
    /// the graphics of the emulator.
    ///
    /// This value will be `None` in case the emulator is not running
    /// in OpenGL mode.
    pub window: Option<Window>,

    pub video_subsystem: VideoSubsystem,
    pub timer_subsystem: TimerSubsystem,
    pub audio_subsystem: AudioSubsystem,
    pub event_pump: EventPump,
    pub ttf_context: Sdl2TtfContext,

    pub gl_context: Option<GLContext>,
    pub shader_program: Option<u32>,
    pub gl_texture: Option<u32>,
    pub gl_vao: Option<u32>,
    pub gl_vbo: Option<u32>,

    pub uniform_locations: Option<(i32, i32, i32, i32)>,
    pub last_viewport_size: Option<(u32, u32)>,
    pub vao_bound: bool,
    pub texture_size: Option<(u32, u32)>,
}

impl SdlSystem {
    /// Start the SDL sub-system and all of its structure and returns
    /// a structure with all the needed stuff to handle SDL graphics
    /// and sound.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sdl: &Sdl,
        title: &str,
        width: u32,
        height: u32,
        scale: f32,
        opengl: bool,
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
        if opengl {
            video_subsystem
                .gl_attr()
                .set_context_profile(GLProfile::Core);
            video_subsystem.gl_attr().set_context_version(3, 3);
        }

        // creates the window that is going to be used to display
        // the graphics of the emulator, this is going to be used
        // to display the graphics of the emulator
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

        let (canvas, gl_context, window) = if opengl {
            let gl_context = window.gl_create_context().unwrap();
            gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);
            (None, Some(gl_context), Some(window))
        } else {
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
            (Some(canvas), None, None)
        };

        Self {
            canvas,
            window,
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
            uniform_locations: None,
            last_viewport_size: None,
            vao_bound: false,
            texture_size: None,
        }
    }

    /// Returns the window that is going to be used to display
    /// the graphics of the emulator.
    pub fn window(&self) -> &Window {
        if let Some(canvas) = &self.canvas {
            canvas.window()
        } else {
            self.window.as_ref().unwrap()
        }
    }

    /// Returns a mutable reference to the window that is going to be used
    /// to display the graphics of the emulator.
    pub fn window_mut(&mut self) -> &mut Window {
        if let Some(canvas) = &mut self.canvas {
            canvas.window_mut()
        } else {
            self.window.as_mut().unwrap()
        }
    }

    /// Loads a (fragment) shader into the SDL system.
    ///
    /// This function creates all the needed OpenGL objects to
    /// render the shader, this function is used to apply effects
    /// to the graphics of the emulator.
    ///
    /// # Arguments
    /// * `name` - The name of the shader to load.
    ///
    /// # Returns
    /// * `Ok(())` - If the shader was loaded successfully.
    /// * `Err(String)` - If the shader was not loaded successfully.
    pub fn load_shader(&mut self, name: &str) -> Result<(), String> {
        let program = shader::load_shader_program(name)?;

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
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);

            self.gl_texture = Some(texture);
            self.gl_vao = Some(vao);
            self.gl_vbo = Some(vbo);
        }

        self.shader_program = Some(program);

        self.init_shader_variables()?;

        Ok(())
    }

    /// Renders a frame using the currently loaded shader program.
    ///
    /// Arguments:
    /// * `pixels` - The buffer of pixels to render.
    /// * `width` - The width of the frame.
    /// * `height` - The height of the frame.
    /// * `window_width` - The width of the window (viewport).
    /// * `window_height` - The height of the window (viewport).
    pub fn render_frame_with_shader(
        &mut self,
        pixels: &[u8],
        width: u32,
        height: u32,
        window_width: u32,
        window_height: u32,
    ) {
        if self.shader_program.is_none() {
            return;
        }

        unsafe {
            let (dw, dh) = self.window.as_ref().unwrap().drawable_size();
            if self.last_viewport_size != Some((dw, dh)) {
                gl::Viewport(0, 0, dw as i32, dh as i32);
                self.last_viewport_size = Some((dw, dh));
            }

            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.shader_program.unwrap());

            let texture = self.gl_texture.unwrap();
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture);

            // checks if we need to resize the texture, this is going to be used
            // to avoid unnecessary texture updates
            if self.texture_size != Some((width, height)) {
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
                self.texture_size = Some((width, height));
            } else {
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    gl::RGB,
                    gl::UNSIGNED_BYTE,
                    pixels.as_ptr() as *const _,
                );
            }

            // calculate the viewport dimensions to maintain the aspect ratio
            // this is going to be used to maintain the aspect ratio of the game
            // and the window, this is going to be used to avoid distortion of the
            // game graphics when the window is resized
            let game_aspect = width as f32 / height as f32;
            let window_aspect = window_width as f32 / window_height as f32;
            let (vp_width, vp_height, vp_x, vp_y) = if window_aspect > game_aspect {
                let vp_height = window_height;
                let vp_width = ((vp_height as f32 * game_aspect).round()) as u32;
                let vp_x = (window_width - vp_width) / 2;
                let vp_y = 0;
                (vp_width, vp_height, vp_x, vp_y)
            } else {
                let vp_width = window_width;
                let vp_height = ((vp_width as f32 / game_aspect).round()) as u32;
                let vp_x = 0;
                let vp_y = (window_height - vp_height) / 2;
                (vp_width, vp_height, vp_x, vp_y)
            };

            // sets the viewport to maintain aspect ratio
            gl::Viewport(vp_x as i32, vp_y as i32, vp_width as i32, vp_height as i32);

            let (loc_image, loc_in, loc_out, origin) = self.uniform_locations.unwrap();
            gl::Uniform1i(loc_image, 0);
            gl::Uniform2f(loc_in, width as f32, height as f32);
            gl::Uniform2f(loc_out, vp_width as f32, vp_height as f32);
            gl::Uniform2f(origin, vp_x as f32, vp_y as f32);

            // keeps the VAO bound throughout the frame
            if !self.vao_bound {
                gl::BindVertexArray(self.gl_vao.unwrap());
                self.vao_bound = true;
            }

            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        }

        self.window().gl_swap_window();
    }

    fn init_shader_variables(&mut self) -> Result<(), String> {
        // initializes the texture, making sure that we have a valid texture
        // to be used in the shader program
        unsafe {
            let mut texture = 0;
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            // initializes texture with null data, actual size will be set on first frame
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as i32,
                1,
                1,
                0,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );

            self.gl_texture = Some(texture);
            self.texture_size = None; // Will be set on first frame
        }

        // caches the uniform locations, this is going to be used to
        // update the shader program with the correct values
        unsafe {
            let program = self.shader_program.unwrap();
            let loc_image = gl::GetUniformLocation(program, c"image".as_ptr() as *const _);
            let loc_in = gl::GetUniformLocation(program, c"input_resolution".as_ptr() as *const _);
            let loc_out =
                gl::GetUniformLocation(program, c"output_resolution".as_ptr() as *const _);
            let origin = gl::GetUniformLocation(program, c"origin".as_ptr() as *const _);
            self.uniform_locations = Some((loc_image, loc_in, loc_out, origin));
        }

        // initializes the viewport and clears the color, this is going
        // to be used to clear the screen before rendering the frame
        unsafe {
            let (dw, dh) = self.window.as_ref().unwrap().drawable_size();
            gl::Viewport(0, 0, dw as i32, dh as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        }

        Ok(())
    }

    pub fn cleanup(&mut self) {
        unsafe {
            if self.vao_bound {
                gl::BindVertexArray(0);
                self.vao_bound = false;
            }
            // ... rest of cleanup code ...
        }
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
