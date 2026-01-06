//!
//! OpenGL graphics driver implementation.

use std::{ffi::CString, sync::atomic::{AtomicU8, Ordering}};

use sdl2::{
    event::Event,
    video::{GLContext, GLProfile},
    Sdl, VideoSubsystem,
};

use crate::graphics::sdl::common::{
    DriverConfig, DriverError, DriverResult, DriverState, GraphicsDriver, GraphicsEvent, RedrawMode,
    Screen, ScreenDims,
};

const LOGICAL_WIDTH: u32 = 320;
const LOGICAL_HEIGHT: u32 = 240;
const NUM_SCREENS: usize = 3;

#[derive(Copy, Clone)]
#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    tex: [f32; 2],
}

const QUAD_VERTICES: [Vertex; 4] = [
    Vertex {
        pos: [-1.0, -1.0],
        tex: [0.0, 0.0],
    },
    Vertex {
        pos: [1.0, -1.0],
        tex: [1.0, 0.0],
    },
    Vertex {
        pos: [-1.0, 1.0],
        tex: [0.0, 1.0],
    },
    Vertex {
        pos: [1.0, 1.0],
        tex: [1.0, 1.0],
    },
];

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

const VERT_SHADER_SRC: &str = "\
    attribute vec2 a_pos;\
    attribute vec2 a_tex;\
    varying vec2 v_tex;\
    void main() {\
        v_tex = a_tex;\
        gl_Position = vec4(a_pos, 0.0, 1.0);\
    }\
";

const FRAG_SHADER_SRC: &str = "\
    #ifdef GL_ES\
    precision mediump float;\
    #endif\
    varying vec2 v_tex;\
    uniform sampler2D u_tex;\
    void main() {\
        gl_FragColor = texture2D(u_tex, v_tex);\
    }\
";

pub struct OpenGlDriver {
    sdl_context: Option<Sdl>,
    video_subsystem: Option<VideoSubsystem>,
    window: Option<sdl2::video::Window>,
    gl_context: Option<GLContext>,
    textures: [u32; NUM_SCREENS],
    surfaces: [Option<Vec<u8>>; NUM_SCREENS],
    vertex_buffer: u32,
    index_buffer: u32,
    shader_program: u32,
    state: DriverState,
    pub keep_aspect_ratio: bool,
    active_screen: AtomicU8,
}

impl OpenGlDriver {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sdl_context: None,
            video_subsystem: None,
            window: None,
            gl_context: None,
            textures: [0; NUM_SCREENS],
            surfaces: [None, None, None],
            vertex_buffer: 0,
            index_buffer: 0,
            shader_program: 0,
            state: DriverState::new(),
            keep_aspect_ratio: true,
            active_screen: AtomicU8::new(Screen::Main as u8),
        }
    }

    pub fn set_keep_aspect_ratio(&mut self, keep_aspect_ratio: bool) {
        self.keep_aspect_ratio = keep_aspect_ratio;
    }

    fn validate_screen_index(screen: usize) -> DriverResult<()> {
        if screen < NUM_SCREENS {
            Ok(())
        } else {
            Err(DriverError::InvalidOperation(format!(
                "Invalid screen index: {}",
                screen
            )))
        }
    }

    fn init_surfaces(&mut self) {
        let buffer_size = (LOGICAL_WIDTH * LOGICAL_HEIGHT * 4) as usize;
        for surface in &mut self.surfaces {
            let mut buffer = vec![0u8; buffer_size];
            for pixel in buffer.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
            *surface = Some(buffer);
        }
    }

    fn init_shaders(&mut self) -> DriverResult<()> {
        let vertex_shader = Self::compile_shader(gl::VERTEX_SHADER, VERT_SHADER_SRC)?;
        let fragment_shader = Self::compile_shader(gl::FRAGMENT_SHADER, FRAG_SHADER_SRC)?;
        let program = unsafe { gl::CreateProgram() };

        unsafe {
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::BindAttribLocation(program, 0, b"a_pos\0".as_ptr().cast());
            gl::BindAttribLocation(program, 1, b"a_tex\0".as_ptr().cast());
            gl::LinkProgram(program);
        }

        let mut link_status = 0;
        unsafe {
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut link_status);
        }
        if link_status == 0 {
            let log = Self::program_info_log(program);
            unsafe {
                gl::DeleteProgram(program);
                gl::DeleteShader(vertex_shader);
                gl::DeleteShader(fragment_shader);
            }
            return Err(DriverError::InvalidOperation(format!(
                "shader link failed: {}",
                log
            )));
        }

        unsafe {
            gl::DetachShader(program, vertex_shader);
            gl::DetachShader(program, fragment_shader);
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            gl::UseProgram(program);
            let sampler_location = gl::GetUniformLocation(program, b"u_tex\0".as_ptr().cast());
            if sampler_location >= 0 {
                gl::Uniform1i(sampler_location, 0);
            }
        }

        self.shader_program = program;
        Ok(())
    }

    fn init_buffers(&mut self) {
        unsafe {
            gl::GenBuffers(1, &mut self.vertex_buffer);
            gl::GenBuffers(1, &mut self.index_buffer);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(&QUAD_VERTICES) as isize,
                QUAD_VERTICES.as_ptr().cast(),
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.index_buffer);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                std::mem::size_of_val(&QUAD_INDICES) as isize,
                QUAD_INDICES.as_ptr().cast(),
                gl::STATIC_DRAW,
            );
        }
    }

    fn compile_shader(shader_type: u32, source: &str) -> DriverResult<u32> {
        let shader = unsafe { gl::CreateShader(shader_type) };
        let c_str = CString::new(source).map_err(|e| {
            DriverError::InvalidOperation(format!("shader source contains null: {}", e))
        })?;

        unsafe {
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);
        }

        let mut status = 0;
        unsafe {
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        }
        if status == 0 {
            let log = Self::shader_info_log(shader);
            unsafe {
                gl::DeleteShader(shader);
            }
            return Err(DriverError::InvalidOperation(format!(
                "shader compile failed: {}",
                log
            )));
        }

        Ok(shader)
    }

    fn shader_info_log(shader: u32) -> String {
        let mut len = 0;
        unsafe {
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        }
        if len <= 1 {
            return String::new();
        }

        let mut buffer = vec![0u8; len as usize];
        unsafe {
            gl::GetShaderInfoLog(
                shader,
                len,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
            );
        }
        String::from_utf8_lossy(&buffer)
            .trim_end_matches('\0')
            .to_string()
    }

    fn program_info_log(program: u32) -> String {
        let mut len = 0;
        unsafe {
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        }
        if len <= 1 {
            return String::new();
        }

        let mut buffer = vec![0u8; len as usize];
        unsafe {
            gl::GetProgramInfoLog(
                program,
                len,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
            );
        }
        String::from_utf8_lossy(&buffer)
            .trim_end_matches('\0')
            .to_string()
    }


    fn init_textures(&mut self, config: &DriverConfig) {
        let filter = if config.linear_scaling {
            gl::LINEAR
        } else {
            gl::NEAREST
        };

        unsafe {
            gl::GenTextures(NUM_SCREENS as i32, self.textures.as_mut_ptr());
            for texture in &self.textures {
                gl::BindTexture(gl::TEXTURE_2D, *texture);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter as i32);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    LOGICAL_WIDTH as i32,
                    LOGICAL_HEIGHT as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
            }
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    fn init_sdl(&mut self, config: &DriverConfig) -> DriverResult<()> {
        let sdl_context = sdl2::init()
            .map_err(|e| DriverError::VideoModeFailed(format!("SDL2 init: {}", e)))?;
        let video_subsystem = sdl_context
            .video()
            .map_err(|e| DriverError::VideoModeFailed(format!("video subsystem: {}", e)))?;

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_profile(GLProfile::GLES);
        gl_attr.set_context_version(2, 0);
        gl_attr.set_depth_size(0);
        gl_attr.set_double_buffer(true);

        let title = format!(
            "The Ur-Quan Masters v{} (OpenGL)",
            env!("CARGO_PKG_VERSION")
        );

        let mut window_builder = video_subsystem.window(&title, config.width, config.height);
        let mut window = window_builder
            .opengl()
            .position_centered()
            .build()
            .map_err(|e| DriverError::WindowCreationFailed(e.to_string()))?;

        if config.fullscreen {
            window
                .set_fullscreen(sdl2::video::FullscreenType::Desktop)
                .map_err(|e| DriverError::WindowCreationFailed(format!("set fullscreen: {}", e)))?;
        }

        let gl_context = window
            .gl_create_context()
            .map_err(|e| DriverError::GlContextFailed(e.to_string()))?;
        window
            .gl_make_current(&gl_context)
            .map_err(|e| DriverError::GlContextFailed(format!("make current: {}", e)))?;

        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::DITHER);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        }

        self.sdl_context = Some(sdl_context);
        self.video_subsystem = Some(video_subsystem);
        self.window = Some(window);
        self.gl_context = Some(gl_context);

        Ok(())
    }

    fn cleanup(&mut self) {
        if self.textures != [0; NUM_SCREENS] {
            unsafe {
                gl::DeleteTextures(NUM_SCREENS as i32, self.textures.as_ptr());
            }
            self.textures = [0; NUM_SCREENS];
        }

        if self.vertex_buffer != 0 {
            unsafe {
                gl::DeleteBuffers(1, &self.vertex_buffer);
            }
            self.vertex_buffer = 0;
        }

        if self.index_buffer != 0 {
            unsafe {
                gl::DeleteBuffers(1, &self.index_buffer);
            }
            self.index_buffer = 0;
        }

        if self.shader_program != 0 {
            unsafe {
                gl::DeleteProgram(self.shader_program);
            }
            self.shader_program = 0;
        }

        for surface in &mut self.surfaces {
            *surface = None;
        }

        self.gl_context = None;
        self.window = None;
        self.video_subsystem = None;
        self.sdl_context = None;
    }

    fn get_pitch_internal() -> usize {
        LOGICAL_WIDTH as usize * 4
    }

    fn viewport_for_window(&self, window_width: u32, window_height: u32) -> (i32, i32, i32, i32) {
        if !self.keep_aspect_ratio || window_width == 0 || window_height == 0 {
            return (
                0,
                0,
                window_width.max(1) as i32,
                window_height.max(1) as i32,
            );
        }

        let target_ratio = LOGICAL_WIDTH as f32 / LOGICAL_HEIGHT as f32;
        let window_ratio = window_width as f32 / window_height as f32;

        if (window_ratio - target_ratio).abs() < f32::EPSILON {
            return (0, 0, window_width as i32, window_height as i32);
        }

        if window_ratio > target_ratio {
            let view_width = (window_height as f32 * target_ratio).round() as u32;
            let x = ((window_width - view_width) / 2) as i32;
            (x, 0, view_width as i32, window_height as i32)
        } else {
            let view_height = (window_width as f32 / target_ratio).round() as u32;
            let y = ((window_height - view_height) / 2) as i32;
            (0, y, window_width as i32, view_height as i32)
        }
    }

    fn draw_fullscreen_quad(&self) {
        unsafe {
            gl::UseProgram(self.shader_program);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.index_buffer);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Vertex>() as i32,
                std::ptr::null(),
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                std::mem::size_of::<Vertex>() as i32,
                (std::mem::size_of::<f32>() * 2) as *const _,
            );
            gl::DrawElements(
                gl::TRIANGLES,
                QUAD_INDICES.len() as i32,
                gl::UNSIGNED_SHORT,
                std::ptr::null(),
            );
            gl::DisableVertexAttribArray(0);
            gl::DisableVertexAttribArray(1);
        }
    }
}

impl Default for OpenGlDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for OpenGlDriver {
    fn drop(&mut self) {
        if self.state.is_initialized() {
            self.cleanup();
        }
    }
}

impl GraphicsDriver for OpenGlDriver {
    fn init(&mut self, config: &DriverConfig) -> DriverResult<()> {
        if self.state.is_initialized() {
            return Err(DriverError::VideoModeFailed(
                "Already initialized".to_string(),
            ));
        }

        self.init_sdl(config)?;
        self.init_surfaces();
        self.init_textures(config);
        self.init_shaders()?;
        self.init_buffers();

        self.state.mark_initialized(*config);
        self.active_screen.store(Screen::Main as u8, Ordering::Relaxed);
        Ok(())
    }

    fn uninit(&mut self) -> DriverResult<()> {
        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        self.cleanup();
        self.state.mark_uninitialized();
        Ok(())
    }

    fn swap_buffers(&mut self, _mode: RedrawMode) -> DriverResult<()> {
        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let window = self.window.as_ref().ok_or(DriverError::NotInitialized)?;
        let (window_width, window_height) = window.size();
        let (x, y, width, height) = self.viewport_for_window(window_width, window_height);

        unsafe {
            gl::Viewport(0, 0, window_width.max(1) as i32, window_height.max(1) as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Viewport(x, y, width, height);
        }

        let screen_index = self.active_screen.load(Ordering::Relaxed) as usize;
        if let Some(surface) = &self.surfaces[screen_index] {
            unsafe {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.textures[screen_index]);
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    LOGICAL_WIDTH as i32,
                    LOGICAL_HEIGHT as i32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    surface.as_ptr() as *const _,
                );
                gl::Disable(gl::BLEND);
            }

            self.draw_fullscreen_quad();
        }

        window.gl_swap_window();
        Ok(())
    }

    fn set_gamma(&mut self, gamma: f32) -> DriverResult<()> {
        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        if gamma <= 0.0 || gamma.is_nan() {
            return Err(DriverError::InvalidOperation(format!(
                "invalid gamma: {}",
                gamma
            )));
        }

        self.state.set_gamma(gamma);
        Ok(())
    }

    fn get_gamma(&self) -> f32 {
        self.state.gamma()
    }

    fn toggle_fullscreen(&mut self) -> DriverResult<bool> {
        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let window = self
            .window
            .as_mut()
            .ok_or(DriverError::NotInitialized)?;

        let mut config = self.state.config();

        if config.fullscreen {
            window
                .set_fullscreen(sdl2::video::FullscreenType::Off)
                .map_err(|e| DriverError::FullscreenFailed(format!("unset fullscreen: {}", e)))?;
            window
                .set_size(config.width, config.height)
                .map_err(|e| DriverError::FullscreenFailed(format!("set size: {}", e)))?;
            config.fullscreen = false;
        } else {
            window
                .set_fullscreen(sdl2::video::FullscreenType::Desktop)
                .map_err(|e| DriverError::FullscreenFailed(format!("set fullscreen: {}", e)))?;
            config.fullscreen = true;
        }

        self.state.update_config(config);
        Ok(true)
    }

    fn is_fullscreen(&self) -> bool {
        self.state.config().is_fullscreen()
    }

    fn is_initialized(&self) -> bool {
        self.state.is_initialized()
    }

    fn supports_hardware_scaling(&self) -> bool {
        true
    }

    fn get_dimensions(&self) -> ScreenDims {
        let config = self.state.config();
        ScreenDims {
            width: LOGICAL_WIDTH,
            height: LOGICAL_HEIGHT,
            actual_width: config.width,
            actual_height: config.height,
        }
    }

    fn get_screen_pixels(&self, screen: usize) -> DriverResult<*const u8> {
        Self::validate_screen_index(screen)?;

        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let surface = self.surfaces[screen]
            .as_ref()
            .ok_or(DriverError::NotInitialized)?;

        self.active_screen.store(match screen {
            0 => Screen::Main,
            1 => Screen::Extra,
            _ => Screen::Transition,
        } as u8, Ordering::Relaxed);

        Ok(surface.as_ptr())
    }

    fn get_screen_pixels_mut(&mut self, screen: usize) -> DriverResult<*mut u8> {
        Self::validate_screen_index(screen)?;

        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let surface = self.surfaces[screen]
            .as_mut()
            .ok_or(DriverError::NotInitialized)?;

        self.active_screen.store(match screen {
            0 => Screen::Main,
            1 => Screen::Extra,
            _ => Screen::Transition,
        } as u8, Ordering::Relaxed);

        Ok(surface.as_mut_ptr())
    }

    fn get_screen_pitch(&self, screen: usize) -> DriverResult<usize> {
        Self::validate_screen_index(screen)?;

        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        Ok(Self::get_pitch_internal())
    }

    fn poll_events(&mut self) -> DriverResult<Vec<GraphicsEvent>> {
        let sdl_context = self
            .sdl_context
            .as_ref()
            .ok_or(DriverError::NotInitialized)?;

        let mut event_pump = sdl_context
            .event_pump()
            .map_err(|e| DriverError::InvalidOperation(format!("event pump: {}", e)))?;

        let mut events = Vec::new();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => events.push(GraphicsEvent::Quit),
                Event::KeyDown { .. } => {}
                Event::KeyUp { .. } => {}
                Event::MouseButtonDown { mouse_btn, .. } => {
                    events.push(GraphicsEvent::MouseButtonDown(mouse_btn as u8));
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    events.push(GraphicsEvent::MouseButtonUp(mouse_btn as u8));
                }
                Event::MouseMotion { x, y, .. } => events.push(GraphicsEvent::MouseMotion(x, y)),
                Event::Window { .. } => events.push(GraphicsEvent::WindowEvent),
                _ => events.push(GraphicsEvent::Unknown),
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opengl_driver_new() {
        let driver = OpenGlDriver::new();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
        assert!(driver.keep_aspect_ratio);
    }

    #[test]
    fn test_opengl_driver_default() {
        let driver = OpenGlDriver::default();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
    }

    #[test]
    fn test_opengl_driver_set_keep_aspect_ratio() {
        let mut driver = OpenGlDriver::new();
        driver.set_keep_aspect_ratio(false);
        assert!(!driver.keep_aspect_ratio);
        driver.set_keep_aspect_ratio(true);
        assert!(driver.keep_aspect_ratio);
    }
}


// SAFETY: We only access the driver from the main thread per SDL2 requirements.
unsafe impl Send for OpenGlDriver {}
// SAFETY: We only access the driver from the main thread per SDL2 requirements.
unsafe impl Sync for OpenGlDriver {}
