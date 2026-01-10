//!
//! Minimal SDL2 software driver implementation.
//!
//! **Supported:**
//! - Window creation and management
//! - Three-screen surface management (Main, Extra, Transition)
//! - Per-frame rendering of active screen
//! - Keyboard, mouse, and window event handling
//! - Fullscreen toggle
//! - Pixel Read/Write access to screen surfaces
//!
//! **NOT Supported (by design):**
//! - Hardware-accelerated rendering (use OpenGL driver instead)
//! - Screen layering/compositing (only renders active screen)
//! - Transition blending effects
//! - Gamma correction (use OpenGL driver instead)
//! - Dirty-rectangle optimization (uploads full frame every time)
//! - Dynamic window resizing (window size is fixed at initialization)
//! - Scanline effects
//!
//! This driver provides a software-only rendering path compatible with
//! systems that don't have OpenGL support. For hardware-accelerated
//! rendering with additional features, use the OpenGL driver.
//!

use std::cell::Cell;

use sdl2::{
    event::Event,
    pixels::PixelFormatEnum,
    render::{Canvas, TextureAccess},
    Sdl, VideoSubsystem,
};

use crate::graphics::sdl::common::{
    DriverConfig, DriverError, DriverResult, DriverState, GraphicsDriver, GraphicsEvent,
    RedrawMode, Screen,
};

/// Pixel format for textures.
#[cfg(target_endian = "big")]
const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBA8888;

#[cfg(target_endian = "little")]
const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBX8888;

/// Logical screen resolution (always 320x240 internally).
const LOGICAL_WIDTH: u32 = 320;
const LOGICAL_HEIGHT: u32 = 240;

/// Minimal SDL2 graphics driver.
///
/// This driver uses SDL2's software rendering backend via SDL_Renderer.
/// It provides compatibility with SDL2 without requiring OpenGL support.
///
/// # Simplified Architecture
///
/// - 3 pixel buffers for MAIN, EXTRA, TRANSITION screens
/// - 320x240 logical resolution always
/// - Full texture upload on each frame (no dirty rectangle tracking)
/// - Textures created on-demand per frame to avoid lifetime issues
/// - Basic event handling
/// - Gamma correction unsupported (use OpenGL driver)
///
/// # Thread Safety
///
/// SDL2 must be initialized on the main thread. Methods that interact with
/// SDL should be called from the same thread that created the driver.
pub struct SdlDriver {
    /// SDL2 context.
    sdl_context: Option<Sdl>,
    /// Video subsystem.
    video_subsystem: Option<VideoSubsystem>,
    /// Window handle.
    window: Option<sdl2::video::Window>,
    /// Canvas for rendering.
    canvas: Option<Canvas<sdl2::video::Window>>,
    /// Event pump for input handling.
    event_pump: Option<sdl2::EventPump>,
    /// Pixel buffers for each screen.
    pixel_buffers: [Option<Vec<u8>>; 3],
    /// Currently active screen to display.
    active_screen: Cell<Screen>,
    /// Shared driver state.
    state: DriverState,
}

impl SdlDriver {
    /// Create a new SDL2 driver instance.
    ///
    /// The driver is not initialized until `init()` is called.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sdl_context: None,
            video_subsystem: None,
            window: None,
            canvas: None,
            event_pump: None,
            pixel_buffers: [None, None, None],
            active_screen: Cell::new(Screen::Main),
            state: DriverState::new(),
        }
    }

    /// Get the underlying driver state.
    #[must_use]
    pub const fn state(&self) -> &DriverState {
        &self.state
    }

    /// Get pixel buffer index validation helper.
    fn validate_screen_index(screen: usize) -> DriverResult<()> {
        if screen < 3 {
            Ok(())
        } else {
            Err(DriverError::InvalidOperation(format!(
                "Invalid screen index: {}",
                screen
            )))
        }
    }

    /// Convert a screen index to the Screen enum.
    #[must_use]
    fn screen_from_index(screen: usize) -> Screen {
        match screen {
            0 => Screen::Main,
            1 => Screen::Extra,
            _ => Screen::Transition,
        }
    }

    /// Get pitch (bytes per row) for screen.
    #[must_use]
    fn get_pitch_internal() -> usize {
        LOGICAL_WIDTH as usize * 4 // 4 bytes per pixel (RGBX)
    }

    /// Initialize SDL2 and create the window/renderer.
    fn init_sdl(&mut self, config: &DriverConfig) -> DriverResult<()> {
        log::info!("Initializing SDL2");

        // Initialize SDL2 video subsystem
        let sdl_context =
            sdl2::init().map_err(|e| DriverError::VideoModeFailed(format!("SDL2 init: {}", e)))?;

        let video_subsystem = sdl_context
            .video()
            .map_err(|e| DriverError::VideoModeFailed(format!("video subsystem: {}", e)))?;

        let version = video_subsystem.current_video_driver();
        log::info!("SDL2 video driver: {}", version);

        self.sdl_context = Some(sdl_context);
        self.video_subsystem = Some(video_subsystem);

        // Create window
        let title = format!(
            "The Ur-Quan Masters v{} (SDL2 Pure)",
            env!("CARGO_PKG_VERSION")
        );

        log::info!("Creating window: {}x{}", config.width, config.height);

        let mut window_builder = self
            .video_subsystem
            .as_ref()
            .ok_or(DriverError::NotInitialized)?
            .window(&title, config.width, config.height);

        let window = window_builder
            .position_centered()
            .build()
            .map_err(|e| DriverError::WindowCreationFailed(e.to_string()))?;

        // Store window reference for fullscreen toggles
        self.window = Some(window.clone());

        // Set fullscreen if requested before taking ownership
        if config.fullscreen {
            self.window
                .as_mut()
                .ok_or(DriverError::NotInitialized)?
                .set_fullscreen(sdl2::video::FullscreenType::Desktop)
                .map_err(|e| DriverError::WindowCreationFailed(format!("set fullscreen: {}", e)))?;
        }

        let event_pump = self
            .sdl_context
            .as_ref()
            .ok_or(DriverError::NotInitialized)?
            .event_pump()
            .map_err(|e| DriverError::InvalidOperation(format!("event pump: {}", e)))?;

        self.event_pump = Some(event_pump);

        // Create renderer with ACCELERATED flag
        let mut canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| DriverError::RendererCreationFailed(e.to_string()))?;

        let info = canvas.info();
        log::info!("SDL2 renderer: {}", info.name);

        // Set logical size for consistent rendering (always 320x240)
        canvas
            .set_logical_size(LOGICAL_WIDTH, LOGICAL_HEIGHT)
            .map_err(|e| DriverError::RendererCreationFailed(format!("set logical size: {}", e)))?;

        // Set render hints for quality based on scaling preference
        let scale_hint = if config.linear_scaling {
            "linear"
        } else {
            "nearest"
        };
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", scale_hint);

        self.canvas = Some(canvas);

        // Initialize pixel buffers for all screens
        self.init_pixel_buffers()?;

        Ok(())
    }

    /// Initialize pixel buffers for all screens.
    fn init_pixel_buffers(&mut self) -> DriverResult<()> {
        log::info!("Initializing pixel buffers");

        for i in 0..3 {
            // Create pixel buffer
            let buffer_size = LOGICAL_WIDTH as usize * LOGICAL_HEIGHT as usize * 4; // RGBX = 4 bytes per pixel
            let mut pixel_buffer = vec![0u8; buffer_size];

            // Initialize with black
            for y in 0..LOGICAL_HEIGHT as usize {
                for x in 0..LOGICAL_WIDTH as usize {
                    let offset = (y * LOGICAL_WIDTH as usize + x) * 4;
                    pixel_buffer[offset] = 0; // R
                    pixel_buffer[offset + 1] = 0; // G
                    pixel_buffer[offset + 2] = 0; // B
                    pixel_buffer[offset + 3] = 255; // X (unused)
                }
            }

            self.pixel_buffers[i] = Some(pixel_buffer);

            log::debug!(
                "Initialized screen {}: {}x{} (logical), pitch: {}",
                i,
                LOGICAL_WIDTH,
                LOGICAL_HEIGHT,
                Self::get_pitch_internal()
            );
        }

        log::info!("Pixel buffers initialized successfully");
        Ok(())
    }

    /// Clean up SDL2 resources.
    fn cleanup(&mut self) {
        log::info!("Cleaning up SDL2 driver");

        for buffer in &mut self.pixel_buffers {
            *buffer = None;
        }

        self.canvas = None;
        self.window = None;
        self.event_pump = None;
        self.video_subsystem = None;
        self.sdl_context = None;
    }
}

impl Default for SdlDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Drop implementation for SdlDriver.
impl Drop for SdlDriver {
    fn drop(&mut self) {
        // Clean up if not already done
        if self.window.is_some() || self.canvas.is_some() {
            log::debug!("SdlDriver dropping, cleaning up resources");
            self.cleanup();
        }
    }
}

impl GraphicsDriver for SdlDriver {
    /// Initialize the SDL2 driver.
    ///
    /// This creates the window, renderer, and textures.
    ///
    /// Errors:
    /// - `VideoModeFailed`: If SDL2 initialization fails
    /// - `WindowCreationFailed`: If window creation fails
    /// - `RendererCreationFailed`: If renderer creation fails
    fn init(&mut self, config: &DriverConfig) -> DriverResult<()> {
        if self.state.is_initialized() {
            return Err(DriverError::VideoModeFailed(
                "Already initialized".to_string(),
            ));
        }

        // Initialize SDL2 and create window/renderer
        self.init_sdl(config)?;

        // Mark as initialized
        self.state.mark_initialized(*config);
        self.active_screen.set(Screen::Main);

        log::info!("SDL2 driver initialized successfully");
        Ok(())
    }

    /// Shutdown the SDL2 driver.
    ///
    /// This releases all SDL2 resources.
    ///
    /// Errors:
    /// - `NotInitialized`: If the driver is not initialized
    fn uninit(&mut self) -> DriverResult<()> {
        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        self.cleanup();
        self.state.mark_uninitialized();
        log::info!("SDL2 driver shut down");
        Ok(())
    }

    /// Swap buffers and display the rendered frame.
    ///
    /// This creates a temporary texture for the active screen, uploads pixel data,
    /// renders to canvas, and presents. Textures are dropped automatically
    /// at the end of the function, avoiding lifetime complexity.
    ///
    /// Arguments:
    /// - `mode`: Redraw mode (currently ignored in minimal driver)
    ///
    /// Errors:
    /// - `NotInitialized`: If the driver is not initialized
    fn swap_buffers(&mut self, _mode: RedrawMode) -> DriverResult<()> {
        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let canvas = self.canvas.as_mut().ok_or(DriverError::NotInitialized)?;
        let texture_creator = canvas.texture_creator();

        // Clear canvas
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        canvas.clear();

        let active_index = self.active_screen.get().index();
        if let Some(buffer) = &self.pixel_buffers[active_index] {
            let mut texture = texture_creator
                .create_texture(
                    PIXEL_FORMAT,
                    TextureAccess::Streaming,
                    LOGICAL_WIDTH,
                    LOGICAL_HEIGHT,
                )
                .map_err(|e| {
                    DriverError::RendererCreationFailed(format!(
                        "screen {} texture: {}",
                        active_index, e
                    ))
                })?;

            // Upload pixel data to texture
            let pitch = Self::get_pitch_internal();
            texture
                .update(None, buffer, pitch)
                .map_err(|e| DriverError::InvalidOperation(format!("texture update: {}", e)))?;

            // Render texture to full logical size
            canvas
                .copy(&texture, None, None)
                .map_err(|e| DriverError::InvalidOperation(format!("render copy: {}", e)))?;

            // Texture is dropped here at end of scope, no cleanup needed
        }

        // Present frame
        canvas.present();

        Ok(())
    }

    /// Set gamma correction.
    ///
    /// Arguments:
    /// - `gamma`: Gamma value (1.0 = no correction, >1.0 = brighter, <1.0 = darker)
    ///
    /// Returns:
    /// - `Ok(())` when stored locally, `Err` if not initialized
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

        if let Some(window) = self.window.as_mut() {
            if let Err(err) = window.set_brightness(gamma as f64) {
                log::warn!("SDL2 set brightness failed: {}", err);
            }
        }

        self.state.set_gamma(gamma);
        Ok(())
    }

    /// Get current gamma value.
    ///
    /// Returns the tracked gamma value.
    fn get_gamma(&self) -> f32 {
        self.state.gamma()
    }

    /// Toggle fullscreen mode.
    ///
    /// Returns:
    /// - `Ok(true)`: Mode was toggled
    /// - `Err`: If toggle failed
    fn toggle_fullscreen(&mut self) -> DriverResult<bool> {
        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let window = self.window.as_mut().ok_or(DriverError::NotInitialized)?;

        let mut config = self.state.config();

        if config.fullscreen {
            // Switch to windowed mode
            window
                .set_fullscreen(sdl2::video::FullscreenType::Off)
                .map_err(|e| DriverError::FullscreenFailed(format!("unset fullscreen: {}", e)))?;

            // Restore original size
            window
                .set_size(config.width, config.height)
                .map_err(|e| DriverError::FullscreenFailed(format!("set size: {}", e)))?;

            config.fullscreen = false;
            log::info!(
                "Switched to windowed mode: {}x{}",
                config.width,
                config.height
            );
        } else {
            // Switch to fullscreen mode
            window
                .set_fullscreen(sdl2::video::FullscreenType::Desktop)
                .map_err(|e| DriverError::FullscreenFailed(format!("set fullscreen: {}", e)))?;

            config.fullscreen = true;
            log::info!("Switched to fullscreen mode");
        }

        self.state.update_config(config);
        Ok(true)
    }

    /// Check if currently in fullscreen mode.
    fn is_fullscreen(&self) -> bool {
        self.state.config().is_fullscreen()
    }

    /// Check if the driver is initialized.
    fn is_initialized(&self) -> bool {
        self.state.is_initialized()
    }

    /// Check if hardware scaling is supported.
    ///
    /// SDL2 supports hardware scaling via the renderer.
    fn supports_hardware_scaling(&self) -> bool {
        true
    }

    /// Get current screen dimensions.
    ///
    /// Note: In the minimal driver, logical resolution is always 320x240.
    fn get_dimensions(&self) -> crate::graphics::sdl::common::ScreenDims {
        use crate::graphics::sdl::common::ScreenDims;
        let config = self.state.config();
        ScreenDims {
            width: LOGICAL_WIDTH,
            height: LOGICAL_HEIGHT,
            actual_width: config.width,
            actual_height: config.height,
        }
    }

    /// Get screen pixels for direct access (readonly).
    ///
    /// Arguments:
    /// - `screen`: Screen index (0=Main, 1=Extra, 2=Transition)
    ///
    /// Returns:
    /// - Pointer to pixel data if available, error otherwise
    fn get_screen_pixels(&self, screen: usize) -> DriverResult<*const u8> {
        Self::validate_screen_index(screen)?;

        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let pixel_buffer = self.pixel_buffers[screen]
            .as_ref()
            .ok_or(DriverError::NotInitialized)?;

        self.active_screen.set(Self::screen_from_index(screen));

        Ok(pixel_buffer.as_ptr())
    }

    /// Get screen pixels for direct access (mutable).
    ///
    /// Arguments:
    /// - `screen`: Screen index (0=Main, 1=Extra, 2=Transition)
    ///
    /// Returns:
    /// - Mutable pointer to pixel data if available, error otherwise
    fn get_screen_pixels_mut(&mut self, screen: usize) -> DriverResult<*mut u8> {
        Self::validate_screen_index(screen)?;

        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        let pixel_buffer = self.pixel_buffers[screen]
            .as_mut()
            .ok_or(DriverError::NotInitialized)?;

        self.active_screen.set(Self::screen_from_index(screen));

        Ok(pixel_buffer.as_mut_ptr())
    }

    /// Get screen pitch (bytes per row).
    ///
    /// Arguments:
    /// - `screen`: Screen index (0=Main, 1=Extra, 2=Transition)
    ///
    /// Returns:
    /// - The pitch in bytes if available, error otherwise
    fn get_screen_pitch(&self, screen: usize) -> DriverResult<usize> {
        Self::validate_screen_index(screen)?;

        if !self.state.is_initialized() {
            return Err(DriverError::NotInitialized);
        }

        Ok(Self::get_pitch_internal())
    }

    /// Poll for pending events.
    ///
    /// Returns a vector of pending graphics events suitable for further processing.
    fn poll_events(&mut self) -> DriverResult<Vec<GraphicsEvent>> {
        let event_pump = self
            .event_pump
            .as_mut()
            .ok_or(DriverError::NotInitialized)?;

        let mut events = Vec::new();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    events.push(GraphicsEvent::Quit);
                }
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    events.push(GraphicsEvent::KeyDown(scancode as i32));
                }
                Event::KeyDown { .. } => {}
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    events.push(GraphicsEvent::KeyUp(scancode as i32));
                }
                Event::KeyUp { .. } => {}
                Event::MouseButtonDown { mouse_btn, .. } => {
                    events.push(GraphicsEvent::MouseButtonDown(mouse_btn as u8));
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    events.push(GraphicsEvent::MouseButtonUp(mouse_btn as u8));
                }
                Event::MouseMotion { x, y, .. } => {
                    events.push(GraphicsEvent::MouseMotion(x, y));
                }
                Event::Window { win_event, .. } => {
                    match win_event {
                        sdl2::event::WindowEvent::Resized(w, h) => {
                            log::info!("Window resized to {}x{}", w, h);
                        }
                        sdl2::event::WindowEvent::FocusGained => {
                            log::debug!("Window focus gained");
                        }
                        sdl2::event::WindowEvent::FocusLost => {
                            log::debug!("Window focus lost");
                        }
                        _ => {}
                    }
                    events.push(GraphicsEvent::WindowEvent);
                }
                _ => {
                    events.push(GraphicsEvent::Unknown);
                }
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::sdl::common::ScreenDims;

    #[test]
    fn test_sdl_driver_default() {
        let driver = SdlDriver::default();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
        assert!(!driver.is_fullscreen());
        assert!(driver.supports_hardware_scaling());
    }

    #[test]
    fn test_sdl_driver_new() {
        let driver = SdlDriver::new();
        assert!(!driver.is_initialized());
        assert_eq!(driver.get_gamma(), 1.0);
    }

    #[test]
    fn test_constants() {
        assert_eq!(LOGICAL_WIDTH, 320);
        assert_eq!(LOGICAL_HEIGHT, 240);
        assert_eq!(SdlDriver::get_pitch_internal(), 1280); // 320 * 4
    }

    #[test]
    fn test_validate_screen_index() {
        assert!(SdlDriver::validate_screen_index(0).is_ok());
        assert!(SdlDriver::validate_screen_index(1).is_ok());
        assert!(SdlDriver::validate_screen_index(2).is_ok());
        assert!(SdlDriver::validate_screen_index(3).is_err());
        assert!(SdlDriver::validate_screen_index(100).is_err());
    }

    #[test]
    fn test_get_dimensions_before_init() {
        let driver = SdlDriver::new();
        let dims = driver.get_dimensions();
        assert_eq!(dims.width, 320);
        assert_eq!(dims.height, 240);
        assert_eq!(dims.actual_width, 320);
        assert_eq!(dims.actual_height, 240);
    }

    #[test]
    fn test_get_screen_pixels_fails_uninitialized() {
        let driver = SdlDriver::new();
        assert!(
            driver.get_screen_pixels(0).is_err(),
            "Should fail when not initialized"
        );
    }

    #[test]
    fn test_get_screen_pixels_mut_fails_uninitialized() {
        let mut driver = SdlDriver::new();
        assert!(
            driver.get_screen_pixels_mut(0).is_err(),
            "Should fail when not initialized"
        );
    }

    #[test]
    fn test_get_screen_pitch_fails_uninitialized() {
        let driver = SdlDriver::new();
        assert!(
            driver.get_screen_pitch(0).is_err(),
            "Should fail when not initialized"
        );
    }

    #[test]
    fn test_get_screen_pixels_invalid_index() {
        let driver = SdlDriver::new();
        assert!(
            driver.get_screen_pixels(3).is_err(),
            "Should fail for invalid index"
        );
    }

    #[test]
    fn test_set_gamma_before_init() {
        let mut driver = SdlDriver::new();
        // Should fail because not initialized
        assert!(
            driver.set_gamma(1.5).is_err(),
            "set_gamma should fail when not initialized"
        );
    }

    #[test]
    fn test_toggle_fullscreen_fails_uninitialized() {
        let mut driver = SdlDriver::new();
        assert!(
            driver.toggle_fullscreen().is_err(),
            "toggle_fullscreen should fail when not initialized"
        );
    }

    #[test]
    fn test_poll_events_fails_uninitialized() {
        let mut driver = SdlDriver::new();
        assert!(
            driver.poll_events().is_err(),
            "poll_events should fail when not initialized"
        );
    }

    #[test]
    fn test_swap_buffers_fails_uninitialized() {
        let mut driver = SdlDriver::new();
        assert!(
            driver.swap_buffers(RedrawMode::None).is_err(),
            "swap_buffers should fail when not initialized"
        );
    }

    #[test]
    fn test_screen_dims_structure() {
        let dims = ScreenDims::default();
        assert_eq!(dims.width, 320);
        assert_eq!(dims.height, 240);
        assert_eq!(dims.actual_width, 320);
        assert_eq!(dims.actual_height, 240);
    }
}

// SAFETY: We only access the driver from the main thread per SDL2 requirements.
unsafe impl Send for SdlDriver {}
// SAFETY: We only access the driver from the main thread per SDL2 requirements.
unsafe impl Sync for SdlDriver {}
