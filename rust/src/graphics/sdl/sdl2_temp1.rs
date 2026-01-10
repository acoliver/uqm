//!
//! SDL2 pure software driver implementation.
//!
//! This module provides a Rust wrapper for the SDL2 pure software graphics driver,
//! mirroring the C implementation in `sc2/src/libs/graphics/sdl/sdl2_pure.c` and
//! `sc2/src/libs/graphics/sdl/sdl2_common.c`.
//!
//! # Implementation Status
//!
//! This implementation provides:
//! - Full SDL2 initialization with video subsystem
//! - Window creation with resizable support
//! - SDL2 renderer creation (software backend)
//! - Three screen surfaces ( MAIN, EXTRA, TRANSITION)
//! - Texture management for each screen
//! - Fullscreen toggle with cursor visibility
//! - Delta update optimization (dirty rectangles)
//! - Buffer swapping via SDL_RenderPresent
//! - Event processing for keyboard, mouse, and window events
//! - Render quality hints (linear vs nearest-neighbor scaling)
//!
//! Corresponds to the TFB_GFXDRIVER_SDL_PURE backend in C.
//!
//! # Architecture
//!
//! The driver maintains:
//! - `sdl_context`: SDL2 context handle (main thread only)
//! - `video_subsystem`: SDL2 video subsystem
//! - `window`: SDL window handle
//! - `renderer`: SDL renderer handle
//! - `screens`: Array of screen surfaces with texture tracking
//! - State tracking via DriverState
//!
//! # Screen Management
//!
//! Three screens are managed:
//! - Screen 0 (Main): Primary rendering surface
//! - Screen 1 (Extra): Secondary surface for overlays (initially inactive)
//! - Screen 2 (Transition): Transition effects surface
//!
//! Each screen has:
//! - Pixel surface for direct drawing access
//! - Optional scaled surface (if scaling is enabled)
//! - SDL texture for rendering
//! - Dirty rectangle tracking

use std::sync::Mutex;

use sdl2::{
    event::Event, key::Keycode,
    pixels::PixelFormatEnum,
    rect::Rect,
    render::{Texture, TextureAccess, WindowCanvas},
    Sdl, VideoSubsystem,
};

use crate::graphics::sdl::common::{
    DriverConfig, DriverError, DriverResult, DriverState, GraphicsDriver, RedrawMode,
    Screen, ScreenDims, UpdateRect, GraphicsEvent,
};

use crate::graphics::sdl::common::{
    DriverConfig, DriverError, DriverResult, DriverState, GraphicsDriver, RedrawMode,
    Screen, ScreenDims, UpdateRect, GraphicsEvent,
};

/// Maximum number of screens (mirrors TFB_GFX_NUMSCREENS).
const NUM_SCREENS: usize = 3;

/// Pixel format masks for 32-bit RGBA surfaces.
#[cfg(target_endian = "big")]
const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBA8888;

#[cfg(target_endian = "little")]
const PIXEL_FORMAT: PixelFormatEnum = PixelFormatEnum::RGBX8888;

/// Screen surface information, mirroring TFB_SDL2_SCREENINFO in C.
#[derive(Debug)]
struct ScreenInfo {
    /// Pixel surface for direct drawing.
    surface: Option<sdl2::surface::Surface<'static>>,
    /// Scaled surface (for software scaling, optional).
    scaled_surface: Option<sdl2::surface::Surface<'static>>,
    /// SDL texture for rendering.
    texture: Option<Texture<'static>>,
    /// Dirty flag - true if texture needs update.
    dirty: bool,
    /// Active flag - controls screen visibility.
    active: bool,
    /// Dirty rectangle for partial updates.
    updated: UpdateRect,
    /// Locked state for raw pixel access.
    locked: bool,
}

impl Default for ScreenInfo {
    fn default() -> Self {
        Self {
            surface: None,
            scaled_surface: None,
            texture: None,
            dirty: true,
            active: true,
            updated: UpdateRect::new(0, 0, 0, 0),
            locked: false,
        }
    }
}

impl ScreenInfo {
    fn new() -> Self {
        Self::default()
    }

    /// Mark the entire screen as dirty.
    fn mark_full_dirty(&mut self, width: u32, height: u32) {
        self.updated = UpdateRect::new(0, 0, width, height);
        self.dirty = true;
    }

    /// Mark a specific region as dirty.
    fn mark_rect_dirty(&mut self, rect: UpdateRect) {
        self.updated = rect;
        self.dirty = true;
    }
}

/// SDL2 pure software graphics driver.
///
/// This driver uses SDL2's software rendering backend via SDL_Renderer.
/// It provides compatibility with SDL2 without requiring OpenGL support.
///
/// Corresponds to the TFB_GFXDRIVER_SDL_PURE backend in C.
///
/// # Thread Safety
///
/// SDL2 must be initialized on the main thread. Methods that interact with
/// SDL should be called from the same thread that created the driver.
///
/// # Example
///
/// ```rust,ignore
/// use crate::graphics::sdl::{SdlDriver, DriverConfig, RedrawMode};
///
/// let mut driver = SdlDriver::new()?;
/// let config = DriverConfig::new(640, 480, false);
/// driver.init(&config)?;
/// driver.swap_buffers(RedrawMode::None)?;
/// driver.uninit()?;
/// ```
pub struct SdlDriver {
    /// SDL2 context.
    sdl_context: Option<Sdl>,
    /// Video subsystem.
    video_subsystem: Option<VideoSubsystem>,
    /// Window handle.
    window: Option<sdl2::video::Window>,
    /// Canvas for rendering.
    canvas: Option<Mutex<WindowCanvas>>,
    /// Screen surfaces and textures.
    screens: [ScreenInfo; NUM_SCREENS],
    /// Shared driver state.
    state: DriverState,
    /// Preferred renderer backend (optional).
    renderer_backend: Option<String>,
}

impl SdlDriver {
    /// Create a new SDL2 pure driver instance.
    ///
    /// The driver is not initialized until `init()` is called.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sdl_context: None,
            video_subsystem: None,
            window: None,
            canvas: None,
            screens: [
                ScreenInfo::new(),
                ScreenInfo::new(),
                ScreenInfo::new(),
            ],
            state: DriverState::new(),
            renderer_backend: None,
        }
    }

    /// Get the underlying driver state.
    #[must_use]
    pub const fn state(&self) -> &DriverState {
        &self.state
    }

    /// Set the preferred renderer backend.
    ///
    /// If set to `None`, SDL2 will use the default renderer.
    ///
    /// Note: This must be called before `init()`.
    pub fn set_renderer_backend(&mut self, backend: Option<String>) {
        self.renderer_backend = backend;
    }

    /// Check if hardware scaling is supported.
    ///
    /// SDL2 pure driver supports hardware scaling via the renderer.
    #[must_use]
    pub const fn supports_hardware_scaling() -> bool {
        true
    }

    /// Initialize SDL2 and create the window/renderer.
    ///
    /// This is the internal initialization method used by `init()`.
    fn init_sdl(&mut self, config: &DriverConfig) -> DriverResult<()> {
        log::info!("Initializing SDL2");

        // Initialize SDL2 video subsystem
        let sdl_context = sdl2::init()
            .map_err(|e| DriverError::VideoModeFailed(format!("SDL2 init: {}", e)))?;

        let video_subsystem = sdl_context
            .video()
            .map_err(|e| DriverError::VideoModeFailed(format!("video subsystem: {}", e)))?;

        let version = video_subsystem.current_video_driver().unwrap_or("unknown");
        log::info!("SDL2 video driver: {}", version);

        self.sdl_context = Some(sdl_context);
        self.video_subsystem = Some(video_subsystem);
        // Create window
        let title = format!(
            "The Ur-Quan Masters v{} (SDL2 Pure)",
            env!("CARGO_PKG_VERSION")
        );

        log::info!("Creating window: {}x{}", config.width, config.height);

        let window_builder = self.video_subsystem.as_ref().unwrap().window(
            &title,
            config.width,
            config.height,
        );

        let mut window = window_builder
            .position_centered()
            .build()
            .map_err(|e| DriverError::WindowCreationFailed(e.to_string()))?;

        // Set fullscreen if requested
        if config.fullscreen {
            window
                .set_fullscreen(sdl2::video::FullscreenType::Desktop)
                .map_err(|e| DriverError::FullscreenFailed(format!("set fullscreen: {}", e)))?;
            window.hide_cursor();
        } else {
            window.show_cursor();
        }

        self.window = Some(window);

        // Create renderer
        let window = self.window.as_ref().unwrap();
        let mut canvas = window
            .into_canvas()
            .build()
            .map_err(|e| DriverError::RendererCreationFailed(e.to_string()))?;

        let info = canvas
            .info()
            .ok_or_else(|| DriverError::RendererCreationFailed("no renderer info".into()))?;
        log::info!("SDL2 renderer: {}", info.name);

        // Set render hints for quality
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "1"); // "1" = linear, "0" = nearest

        // Set logical size for consistent rendering
        canvas
            .set_logical_size(320, 240)
            .map_err(|e| DriverError::RendererCreationFailed(format!("set logical size: {}", e)))?;

        self.canvas = Some(Mutex::new(canvas));

        // Store window reference for texture creation
        // We'll access texture creator on-demand instead of storing it
        self.texture_creator = None;

        // Initialize screens
        self.init_screens(config)?;

        Ok(())
    }

    /// Initialize screen surfaces and textures.
    fn init_screens(&mut self, config: &DriverConfig) -> DriverResult<()> {
        // Get texture creator from canvas
        let canvas = self.canvas.as_ref().ok_or(DriverError::NotInitialized)?;
        let canvas_locked = canvas.lock().unwrap();
        let texture_creator = canvas_locked.texture_creator();

        for i in 0..NUM_SCREENS {
            self.screens[i] = ScreenInfo::new();
            self.screens[i].active = true;

            // Create pixel surface
            let surface = sdl2::surface::Surface::new(
                config.width,
                config.height,
                PIXEL_FORMAT,
            ).map_err(|e| {
                DriverError::WindowCreationFailed(format!("screen {} surface: {}", i, e))
            })?;

            self.screens[i].surface = Some(surface);
            self.screens[i].mark_full_dirty(config.width, config.height);

            // Create texture
            let texture = texture_creator
                .create_texture(
                    PixelFormatEnum::RGBX8888,
                    TextureAccess::Streaming,
                    config.width,
                    config.height,
                )
                .map_err(|e| {
                    DriverError::RendererCreationFailed(format!("screen {} texture: {}", i, e))
                })?;

            self.screens[i].texture = Some(texture);
            log::debug!("Initialized screen {}: {}x{}", i, config.width, config.height);
        }

        // Extra screen is initially inactive
        self.screens[Screen::Extra.index()].active = false;

        // Release lock
        drop(canvas_locked);
        Ok(())
    }

        // Extra screen is initially inactive
        self.screens[Screen::Extra.index()].active = false;

        Ok(())
    }

    /// Clean up SDL2 resources.
    fn cleanup(&mut self) {
        log::info!("Cleaning up SDL2 driver");

        // Drop textures first
        for screen in &mut self.screens {
            screen.texture = None;
            screen.scaled_surface = None;
            screen.surface = None;
        }

        self.canvas = None;
        self.window = None;
        self.video_subsystem = None;
        self.sdl_context = None;
    }

    /// Update texture from surface (with delta rect optimization).
    fn update_texture(
        &mut self,
        screen_index: usize,
        rect: Option<UpdateRect>,
    ) -> DriverResult<()> {
        let screen = &mut self.screens[screen_index];
        let surface = screen
            .surface
            .as_ref()
            .ok_or(DriverError::NotInitialized)?;
        let texture = screen
            .texture
            .as_mut()
            .ok_or(DriverError::NotInitialized)?;

        // Lock surface for pixel access
        let rect = if let Some(r) = rect {
            if r.is_empty() {
                return Ok(());
            }
            Rect::new(r.x, r.y, r.w, r.h)
        } else {
            Rect::new(0, 0, surface.width(), surface.height())
        };

        surface.with_lock(|pixels| {
            // Calculate pixel offset for partial update
            let pixel_format = surface.pixel_format_enum();
            let bytes_per_pixel = pixel_format.byte_size_of_pixels() as usize;
            let pitch = surface.pitch() as usize;
            let offset = rect.y() as usize * pitch + rect.x() as usize * bytes_per_pixel;

            let pixel_data = &pixels[offset..];

            texture
                .update(rect, pixel_data, pitch)
                .map_err(|e| DriverError::InvalidOperation(format!("texture update: {}", e)))?;

            Ok(())
        })?;

        screen.dirty = false;
        Ok(())
    }
            Rect::new(r.x, r.y, r.w, r.h)
        } else {
            Rect::new(0, 0, surface.width(), surface.height())
        };

        surface
            .with_lock(|pixels| {
                // Calculate pixel offset for partial update
                let pixel_format = surface.pixel_format();
                let bytes_per_pixel = pixel_format.byte_size_per_pixel() as usize;
                let pitch = surface.pitch() as usize;
                let offset = rect.y() as usize * pitch + rect.x() as usize * bytes_per_pixel;
