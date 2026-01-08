//! Graphics core state and configuration.

use std::ffi::CStr;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Instant;

use crate::graphics::dcqueue::{DcqConfig, DrawCommandQueue, Screen as DcqScreen};
use crate::graphics::render_context::{RenderContext, ResourceType, ScreenType};
use crate::graphics::sdl::{
    DriverConfig, GraphicsDriver, GraphicsEvent, OpenGlDriver, RedrawMode as DriverRedrawMode,
    SdlDriver,
};
use crate::graphics::tfb_draw::Canvas;

/// Graphics driver backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GfxDriver {
    /// SDL OpenGL backend.
    SdlOpenGL = 0,
    /// SDL pure software backend.
    #[default]
    SdlPure = 1,
}

/// Forced redraw flags for buffer swapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedrawMode {
    /// No forced redraw.
    None = 0,
    /// Fading effect active - force redraw.
    Fading = 1,
    /// Exposure event - force redraw.
    Expose = 2,
    /// Full forced redraw.
    Full = 3,
}

/// Graphics scaling interpolation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleMode {
    /// Step mode (not truly a scaler).
    Step = 0,
    /// Nearest-neighbor scaling.
    Nearest = 1,
    /// Bilinear scaling.
    Bilinear = 2,
    /// Trilinear scaling.
    Trilinear = 3,
    /// HQ2x scaling (high-quality 2x magnification).
    Hq2x = 4,
    /// Biadaptive scaling (edge-adaptive bilinear).
    Biadaptive = 5,
    /// Triscan scaling (scanline-aware adaptive scaling).
    Triscan = 6,
}

/// Graphics initialization flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GfxFlags(u32);

impl GfxFlags {
    pub const FULLSCREEN: u32 = 1 << 0;
    pub const SHOW_FPS: u32 = 1 << 1;
    pub const SCANLINES: u32 = 1 << 2;
    pub const SCALE_BILINEAR: u32 = 1 << 3;
    pub const SCALE_BIADAPT: u32 = 1 << 4;
    pub const SCALE_BIADAPTADV: u32 = 1 << 5;
    pub const SCALE_TRISCAN: u32 = 1 << 6;
    pub const SCALE_HQXX: u32 = 1 << 7;
    pub const SCALE_ANY: u32 = Self::SCALE_BILINEAR
        | Self::SCALE_BIADAPT
        | Self::SCALE_BIADAPTADV
        | Self::SCALE_TRISCAN
        | Self::SCALE_HQXX;
    pub const SCALE_SOFT_ONLY: u32 = Self::SCALE_ANY & !Self::SCALE_BILINEAR;

    pub fn new(flags: u32) -> Self {
        Self(flags)
    }

    pub fn bits(self) -> u32 {
        self.0
    }

    pub fn contains(self, flag: u32) -> bool {
        (self.0 & flag) == flag
    }
}

/// Screen dimension state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenDimensions {
    /// Current screen width (logical).
    pub width: i32,
    /// Current screen height (logical).
    pub height: i32,
    /// Actual display width (may differ with scaling).
    pub actual_width: i32,
    /// Actual display height (may differ with scaling).
    pub actual_height: i32,
    /// Color depth in bits (e.g., 32 for RGBA).
    pub color_depth: i32,
}

impl Default for ScreenDimensions {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            actual_width: 0,
            actual_height: 0,
            color_depth: 32,
        }
    }
}

/// Scaling configuration state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleConfig {
    /// Graphic scale factor (256 = identity/1.0, 512 = 2.0, etc.).
    pub scale: i32,
    /// Interpolation mode.
    pub mode: ScaleMode,
}

impl Default for ScaleConfig {
    fn default() -> Self {
        Self {
            scale: 256,
            mode: ScaleMode::Nearest,
        }
    }
}

/// Frame rate tracking state.
#[derive(Debug, Clone, Copy)]
pub struct FrameRateState {
    /// Frame rate in FPS.
    pub rate: f32,
    /// Tick base time reference.
    pub tick_base: i32,
    /// Last frame timestamp in milliseconds.
    pub last_tick_ms: u64,
}

impl Default for FrameRateState {
    fn default() -> Self {
        Self {
            rate: 60.0,
            tick_base: 0,
            last_tick_ms: 0,
        }
    }
}

/// Graphics API errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphicsError {
    /// Graphics system not initialized.
    NotInitialized,
    /// Invalid operation for current state.
    InvalidOperation(String),
    /// Graphics driver error.
    DriverError(String),
}

impl std::fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "Graphics not initialized"),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Self::DriverError(msg) => write!(f, "Driver error: {}", msg),
        }
    }
}

impl std::error::Error for GraphicsError {}

/// Core graphics system state.
pub struct GraphicsState {
    driver_id: RwLock<GfxDriver>,
    flags: RwLock<GfxFlags>,
    dimensions: RwLock<ScreenDimensions>,
    scale_config: RwLock<ScaleConfig>,
    frame_rate: RwLock<FrameRateState>,
    gamma: RwLock<f32>,
    initialized: RwLock<bool>,

    driver: Option<Box<dyn GraphicsDriver>>,
    dcq: Arc<RwLock<DrawCommandQueue>>,
    render_context: Arc<RwLock<RenderContext>>,
    frame_timer: RwLock<Option<Instant>>,
}

impl GraphicsState {
    /// Create a new graphics state with default values.
    #[must_use]
    pub fn new() -> Self {
        let render_context = Arc::new(RwLock::new(RenderContext::new()));
        let dcq = Arc::new(RwLock::new(DrawCommandQueue::with_config(
            DcqConfig::standard(),
            Arc::clone(&render_context),
        )));

        Self {
            driver_id: RwLock::new(GfxDriver::default()),
            flags: RwLock::new(GfxFlags::new(0)),
            dimensions: RwLock::new(ScreenDimensions::default()),
            scale_config: RwLock::new(ScaleConfig::default()),
            frame_rate: RwLock::new(FrameRateState::default()),
            gamma: RwLock::new(1.0_f32),
            initialized: RwLock::new(false),
            driver: None,
            dcq,
            render_context,
            frame_timer: RwLock::new(None),
        }
    }

    /// Initialize the graphics subsystem.
    pub fn init(
        &mut self,
        driver: GfxDriver,
        flags: GfxFlags,
        _renderer: Option<&CStr>,
        width: i32,
        height: i32,
    ) -> Result<(), GraphicsError> {
        self.set_driver(driver);
        self.set_flags(flags);
        self.set_dimensions(ScreenDimensions {
            width,
            height,
            actual_width: width,
            actual_height: height,
            color_depth: self.get_dimensions().color_depth,
        });

        let mut new_driver: Box<dyn GraphicsDriver> = match driver {
            GfxDriver::SdlOpenGL => Box::new(OpenGlDriver::new()),
            GfxDriver::SdlPure => Box::new(SdlDriver::new()),
        };

        let config = DriverConfig::new(
            width as u32,
            height as u32,
            flags.contains(GfxFlags::FULLSCREEN),
        )
        .with_linear_scaling(flags.contains(GfxFlags::SCALE_BILINEAR));
        new_driver
            .init(&config)
            .map_err(|err| GraphicsError::DriverError(err.to_string()))?;

        let driver_dims = new_driver.get_dimensions();
        self.set_dimensions(ScreenDimensions {
            width: driver_dims.width as i32,
            height: driver_dims.height as i32,
            actual_width: driver_dims.actual_width as i32,
            actual_height: driver_dims.actual_height as i32,
            color_depth: self.get_dimensions().color_depth,
        });

        self.driver = Some(new_driver);

        if self.driver.is_some() {
            let logical_width = driver_dims.width as i32;
            let logical_height = driver_dims.height as i32;
            let main_canvas = Arc::new(RwLock::new(Canvas::new_for_screen(
                logical_width,
                logical_height,
            )));
            let extra_canvas = Arc::new(RwLock::new(Canvas::new_for_screen(
                logical_width,
                logical_height,
            )));
            let transition_canvas = Arc::new(RwLock::new(Canvas::new_for_screen(
                logical_width,
                logical_height,
            )));

            let mut ctx = self.render_context.write().unwrap();
            ctx.set_screen(ScreenType::Main, Arc::clone(&main_canvas));
            ctx.set_screen(ScreenType::Extra, Arc::clone(&extra_canvas));
            ctx.set_screen(ScreenType::Transition, Arc::clone(&transition_canvas));
        }

        *self.frame_timer.write().unwrap() = Some(Instant::now());
        *self.initialized.write().unwrap() = true;
        Ok(())
    }

    /// Reinitialize graphics with new parameters.
    pub fn reinit(
        &mut self,
        driver: GfxDriver,
        flags: GfxFlags,
        renderer: Option<&CStr>,
        width: i32,
        height: i32,
    ) -> Result<(), GraphicsError> {
        if !self.is_initialized() {
            return Err(GraphicsError::NotInitialized);
        }

        self.driver = None;
        self.init(driver, flags, renderer, width, height)
    }

    /// Shutdown the graphics subsystem.
    pub fn uninit(&mut self) {
        if !self.is_initialized() {
            return;
        }

        if let Err(err) = self.purge_dangling() {
            log::warn!("Failed to purge dangling resources during uninit: {}", err);
        }

        if let Some(driver) = self.driver.as_mut() {
            if let Err(err) = driver.uninit() {
                log::warn!("Driver uninit failed: {}", err);
            }
        }
        self.driver = None;
        *self.initialized.write().unwrap() = false;
    }

    /// Check if graphics are initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        *self.initialized.read().unwrap()
    }

    /// Get reference to driver.
    pub fn driver(&self) -> Option<&dyn GraphicsDriver> {
        self.driver.as_deref()
    }

    /// Get DCQ.
    pub fn dcq(&self) -> Arc<RwLock<DrawCommandQueue>> {
        Arc::clone(&self.dcq)
    }

    /// Get render context.
    pub fn render_context(&self) -> Arc<RwLock<RenderContext>> {
        Arc::clone(&self.render_context)
    }

    /// Swap backbuffers and update display.
    pub fn swap_buffers(&mut self, force_full_redraw: bool) -> Result<(), GraphicsError> {
        if !self.is_initialized() {
            return Err(GraphicsError::NotInitialized);
        }

        self.dcq
            .write()
            .unwrap()
            .process_commands()
            .map_err(|err| {
                GraphicsError::InvalidOperation(format!("DCQ processing failed: {}", err))
            })?;

        let redraw_mode = if force_full_redraw {
            RedrawMode::Full
        } else {
            RedrawMode::None
        };

        {
            let driver = self
                .driver
                .as_mut()
                .ok_or(GraphicsError::NotInitialized)?;
            for screen in [DcqScreen::Main, DcqScreen::Extra, DcqScreen::Transition] {
                sync_canvases_to_driver(&self.render_context, &mut **driver, screen)?;
            }
        }

        let driver = self
            .driver
            .as_mut()
            .ok_or(GraphicsError::NotInitialized)?;
        driver
            .swap_buffers(map_redraw_mode(redraw_mode))
            .map_err(|err| GraphicsError::DriverError(err.to_string()))?;

        self.update_frame_rate_from_clock();

        Ok(())
    }

    /// Process pending graphics events.
    pub fn process_events(&mut self) -> Result<bool, GraphicsError> {
        let driver = self
            .driver
            .as_mut()
            .ok_or(GraphicsError::NotInitialized)?;

        let events = driver
            .poll_events()
            .map_err(|err| GraphicsError::DriverError(err.to_string()))?;
        let should_quit = events.iter().any(|event| matches!(event, GraphicsEvent::Quit));

        Ok(should_quit)
    }

    /// Flush pending graphics commands.
    pub fn flush(&mut self) -> Result<(), GraphicsError> {
        if !self.is_initialized() {
            return Err(GraphicsError::NotInitialized);
        }

        self.dcq
            .write()
            .unwrap()
            .process_commands()
            .map_err(|err| GraphicsError::InvalidOperation(format!("Flush failed: {}", err)))?;

        Ok(())
    }

    /// Purge dangling graphics resources.
    pub fn purge_dangling(&mut self) -> Result<usize, GraphicsError> {
        if !self.is_initialized() {
            return Ok(0);
        }

        let mut purged = 0;
        let orphans = {
            let ctx_guard = self.render_context.read().unwrap();
            ctx_guard.find_orphaned_resources()
        };

        if !orphans.is_empty() {
            let mut ctx_guard = self.render_context.write().unwrap();
            let mut removed_images = Vec::new();

            for id in orphans {
                let Some(meta) = ctx_guard.get_metadata(id).cloned() else {
                    continue;
                };

                if meta.resource_type == ResourceType::Screen {
                    continue;
                }

                if meta.resource_type == ResourceType::Image {
                    if let Some(image) = ctx_guard.remove_image(id) {
                        removed_images.push(image);
                    }
                }

                match meta.resource_type {
                    ResourceType::Image => {}
                    ResourceType::Canvas => {
                        ctx_guard.remove_canvas(id);
                    }
                    ResourceType::Font => {
                        ctx_guard.remove_font_page(id);
                    }
                    ResourceType::ColorMap => {
                        ctx_guard.remove_color_map(id);
                    }
                    ResourceType::DataPtr => {
                        continue;
                    }
                    ResourceType::Screen => {
                        continue;
                    }
                }

                purged += 1;
                log::debug!("Purged orphaned resource {} of type {:?}", id, meta.resource_type);
            }

            if let Some(driver) = self.driver.as_mut() {
                for image in removed_images {
                    driver.on_image_removed(image.as_ref());
                }
            }
        }

        purged += self.render_context.write().unwrap().purge_data_ptrs();

        Ok(purged)
    }


    /// Set the gamma correction value.
    pub fn set_gamma(&mut self, gamma: f32) -> Result<(), GraphicsError> {
        if gamma <= 0.0 || gamma.is_nan() {
            return Err(GraphicsError::InvalidOperation(format!(
                "Invalid gamma: {}",
                gamma
            )));
        }

        *self.gamma.write().unwrap() = gamma;

        if let Some(driver) = self.driver.as_mut() {
            driver
                .set_gamma(gamma)
                .map_err(|err| GraphicsError::DriverError(format!("set_gamma failed: {}", err)))?;
        }

        Ok(())
    }

    /// Get the current gamma value.
    #[must_use]
    pub fn get_gamma(&self) -> f32 {
        *self.gamma.read().unwrap()
    }

    /// Get the current frame rate.
    #[must_use]
    pub fn get_frame_rate(&self) -> f32 {
        self.frame_rate.read().unwrap().rate
    }

    /// Set the frame rate tick base.
    pub fn set_frame_rate_tick_base(&self, tick_base: i32) {
        let mut state = self.frame_rate.write().unwrap();
        state.tick_base = tick_base;
        state.last_tick_ms = tick_base.max(0) as u64;
    }

    /// Get the frame rate tick base.
    #[must_use]
    pub fn get_frame_rate_tick_base(&self) -> i32 {
        self.frame_rate.read().unwrap().tick_base
    }

    /// Update frame rate (called by driver each frame).
    pub fn update_frame_rate(&self, delta_time_ms: u32) {
        if delta_time_ms == 0 {
            return;
        }
        let fps = 1000.0 / delta_time_ms as f32;
        let mut frame_state = self.frame_rate.write().unwrap();
        frame_state.rate = fps;
        frame_state.last_tick_ms = delta_time_ms as u64;
    }

    fn update_frame_rate_from_clock(&self) {
        let now = Instant::now();
        let mut timer = self.frame_timer.write().unwrap();
        let mut frame_state = self.frame_rate.write().unwrap();

        if let Some(previous) = timer.replace(now) {
            let elapsed_ms = now.duration_since(previous).as_millis() as u64;
            if elapsed_ms > 0 {
                frame_state.rate = 1000.0 / elapsed_ms as f32;
                frame_state.last_tick_ms = elapsed_ms;
            }
        }
    }

    // Helper function to sync canvas pixels to driver (outside impl to avoid borrow conflicts)
} // Close impl block for Sync function
fn sync_canvases_to_driver(
    render_context: &Arc<RwLock<RenderContext>>,
    driver: &mut dyn GraphicsDriver,
    screen: DcqScreen,
) -> Result<(), GraphicsError> {
    let screen_type = ScreenType::from(screen);
    let canvas = render_context
        .read()
        .unwrap()
        .get_screen(screen_type)
        .ok_or_else(|| GraphicsError::InvalidOperation("Screen not found".to_string()))?;

    let canvas_guard = canvas.read().unwrap();
    let width = canvas_guard.width();
    let height = canvas_guard.height();
    let bytes_per_pixel = canvas_guard.format().bytes_per_pixel as usize;
    let stride = width as usize * bytes_per_pixel;
    let pixels = canvas_guard.pixels();

    let buffer_ptr = driver
        .get_screen_pixels_mut(screen as usize)
        .map_err(|err| GraphicsError::DriverError(err.to_string()))?;
    let pitch = driver
        .get_screen_pitch(screen as usize)
        .map_err(|err| GraphicsError::DriverError(err.to_string()))?;

    let buffer_len = pitch.saturating_mul(height as usize);
    let copy_len = stride.min(pitch);

    unsafe {
        let buffer = std::slice::from_raw_parts_mut(buffer_ptr, buffer_len);
        for row in 0..height as usize {
            let src_start = row * stride;
            let dst_start = row * pitch;
            if src_start + copy_len > pixels.len() || dst_start + copy_len > buffer.len() {
                break;
            }
            buffer[dst_start..dst_start + copy_len]
                .copy_from_slice(&pixels[src_start..src_start + copy_len]);
        }
    }

    Ok(())
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphicsState {
    /// Get screen dimensions (read-only snapshot).
    #[must_use]
    pub fn get_dimensions(&self) -> ScreenDimensions {
        *self.dimensions.read().unwrap()
    }

    /// Set screen dimensions.
    pub fn set_dimensions(&self, dims: ScreenDimensions) {
        *self.dimensions.write().unwrap() = dims;
    }

    /// Get screen logical width.
    #[must_use]
    pub fn get_width(&self) -> i32 {
        self.get_dimensions().width
    }

    /// Get screen logical height.
    #[must_use]
    pub fn get_height(&self) -> i32 {
        self.get_dimensions().height
    }

    /// Get actual display width.
    #[must_use]
    pub fn get_actual_width(&self) -> i32 {
        self.get_dimensions().actual_width
    }

    /// Get actual display height.
    #[must_use]
    pub fn get_actual_height(&self) -> i32 {
        self.get_dimensions().actual_height
    }

    /// Get current scale configuration.
    #[must_use]
    pub fn get_scale_config(&self) -> ScaleConfig {
        *self.scale_config.read().unwrap()
    }

    /// Get graphic scale factor.
    #[must_use]
    pub fn get_graphic_scale(&self) -> i32 {
        self.scale_config.read().unwrap().scale
    }

    /// Set graphic scale factor.
    pub fn set_graphic_scale(&self, scale: i32) -> i32 {
        let mut config = self.scale_config.write().unwrap();
        let previous = config.scale;
        config.scale = scale;
        previous
    }

    /// Get graphic scale mode.
    #[must_use]
    pub fn get_graphic_scale_mode(&self) -> ScaleMode {
        self.scale_config.read().unwrap().mode
    }

    /// Set graphic scale mode.
    pub fn set_graphic_scale_mode(&self, mode: ScaleMode) -> ScaleMode {
        let mut config = self.scale_config.write().unwrap();
        let previous = config.mode;
        config.mode = mode;
        previous
    }

    /// Get graphics flags.
    #[must_use]
    pub fn get_flags(&self) -> GfxFlags {
        *self.flags.read().unwrap()
    }

    /// Set graphics flags.
    pub fn set_flags(&self, flags: GfxFlags) {
        *self.flags.write().unwrap() = flags;
    }

    /// Check if FPS display is enabled.
    #[must_use]
    pub fn show_fps(&self) -> bool {
        self.get_flags().contains(GfxFlags::SHOW_FPS)
    }

    /// Check if fullscreen mode is enabled.
    #[must_use]
    pub fn is_fullscreen(&self) -> bool {
        self.get_flags().contains(GfxFlags::FULLSCREEN)
    }

    /// Get current graphics driver.
    #[must_use]
    pub fn get_driver(&self) -> GfxDriver {
        *self.driver_id.read().unwrap()
    }

    /// Set graphics driver.
    pub fn set_driver(&self, driver: GfxDriver) {
        *self.driver_id.write().unwrap() = driver;
    }

    /// Check if hardware scaling is supported.
    #[must_use]
    pub fn supports_hardware_scaling(&self) -> Option<bool> {
        if !self.is_initialized() {
            return None;
        }
        if let Some(driver) = self.driver.as_ref() {
            return Some(driver.supports_hardware_scaling());
        }
        Some(self.get_driver() == GfxDriver::SdlOpenGL)
    }
}

fn map_redraw_mode(mode: RedrawMode) -> DriverRedrawMode {
    match mode {
        RedrawMode::None => DriverRedrawMode::None,
        RedrawMode::Fading => DriverRedrawMode::Fading,
        RedrawMode::Expose => DriverRedrawMode::Expose,
        RedrawMode::Full => DriverRedrawMode::Full,
    }
}

static GLOBAL_STATE: OnceLock<Mutex<GraphicsState>> = OnceLock::new();

/// Initialize global graphics state.
pub fn init_global_state() -> &'static Mutex<GraphicsState> {
    GLOBAL_STATE.get_or_init(|| Mutex::new(GraphicsState::new()))
}

/// Get a reference to the global graphics state.
#[must_use]
pub fn global_state() -> &'static Mutex<GraphicsState> {
    GLOBAL_STATE.get().expect("graphics state not initialized")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::cmap::ColorMapInner;
    use crate::graphics::font::FontPage;
    use crate::graphics::tfb_draw::TFImage;

    #[test]
    fn test_graphics_state_default() {
        let state = GraphicsState::new();
        assert!(!state.is_initialized());
        assert_eq!(state.get_gamma(), 1.0);
        assert_eq!(state.get_driver(), GfxDriver::SdlPure);
    }

    #[test]
    fn test_screen_dimensions_default() {
        let dims = ScreenDimensions::default();
        assert_eq!(dims.width, 0);
        assert_eq!(dims.height, 0);
        assert_eq!(dims.color_depth, 32);
    }

    #[test]
    fn test_scale_config_default() {
        let config = ScaleConfig::default();
        assert_eq!(config.scale, 256);
        assert_eq!(config.mode, ScaleMode::Nearest);
    }

    #[test]
    fn test_frame_rate_state_default() {
        let state = FrameRateState::default();
        assert_eq!(state.rate, 60.0);
        assert_eq!(state.tick_base, 0);
        assert_eq!(state.last_tick_ms, 0);
    }

    #[test]
    fn test_gfx_driver_values() {
        assert_eq!(GfxDriver::SdlOpenGL as i32, 0);
        assert_eq!(GfxDriver::SdlPure as i32, 1);
    }

    #[test]
    fn test_reinit_fails_when_uninitialized() {
        let mut state = GraphicsState::new();
        let result = state.reinit(GfxDriver::SdlPure, GfxFlags::new(0), None, 640, 480);
        assert!(matches!(result, Err(GraphicsError::NotInitialized)));
    }

    #[test]
    fn test_redraw_mode_values() {
        assert_eq!(RedrawMode::None as i32, 0);
        assert_eq!(RedrawMode::Fading as i32, 1);
        assert_eq!(RedrawMode::Expose as i32, 2);
        assert_eq!(RedrawMode::Full as i32, 3);
    }

    #[test]
    fn test_scale_mode_values() {
        assert_eq!(ScaleMode::Step as i32, 0);
        assert_eq!(ScaleMode::Nearest as i32, 1);
        assert_eq!(ScaleMode::Bilinear as i32, 2);
        assert_eq!(ScaleMode::Trilinear as i32, 3);
        assert_eq!(ScaleMode::Hq2x as i32, 4);
        assert_eq!(ScaleMode::Biadaptive as i32, 5);
    }

    #[test]
    fn test_gfx_flags_flag_values() {
        assert_eq!(GfxFlags::FULLSCREEN, 1 << 0);
        assert_eq!(GfxFlags::SHOW_FPS, 1 << 1);
        assert_eq!(GfxFlags::SCANLINES, 1 << 2);
    }

    #[test]
    fn test_gfx_flags_contains() {
        let flags = GfxFlags::new(GfxFlags::FULLSCREEN | GfxFlags::SHOW_FPS);
        assert!(flags.contains(GfxFlags::FULLSCREEN));
        assert!(flags.contains(GfxFlags::SHOW_FPS));
        assert!(!flags.contains(GfxFlags::SCANLINES));
    }

    #[test]
    fn test_gfx_flags_scale_any() {
        let scale_flags = GfxFlags::new(GfxFlags::SCALE_ANY);
        assert!(scale_flags.contains(GfxFlags::SCALE_BILINEAR));
        assert!(scale_flags.contains(GfxFlags::SCALE_TRISCAN));
    }

    #[test]
    fn test_set_dimensions() {
        let state = GraphicsState::new();
        let dims = ScreenDimensions {
            width: 640,
            height: 480,
            actual_width: 640,
            actual_height: 480,
            color_depth: 32,
        };
        state.set_dimensions(dims);
        let retrieved = state.get_dimensions();
        assert_eq!(retrieved.width, 640);
        assert_eq!(retrieved.height, 480);
    }

    #[test]
    fn test_set_gamma() {
        let mut state = GraphicsState::new();
        assert_eq!(state.get_gamma(), 1.0);
        let result = state.set_gamma(1.5);
        assert!(result.is_ok());
        assert_eq!(state.get_gamma(), 1.5);
    }

    #[test]
    fn test_purge_dangling_no_orphans() {
        let mut state = GraphicsState::new();
        let render_context = state.render_context();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(6, 6)));

        {
            let mut ctx = render_context.write().unwrap();
            ctx.set_screen(ScreenType::Main, Arc::clone(&canvas));
        }

        *state.initialized.write().unwrap() = true;

        let result = state.purge_dangling().unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_purge_dangling_cleans_resources() {
        let mut state = GraphicsState::new();
        let render_context = state.render_context();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(6, 6)));
        let image = Arc::new(TFImage::new_rgba(8, 8));
        let page = Arc::new(FontPage::new(0x0000, 0x0020, 1));
        let cmap = Arc::new(ColorMapInner::new(0));

        let ids = {
            let mut ctx = render_context.write().unwrap();
            let canvas_id = ctx.register_canvas(Arc::clone(&canvas));
            let image_id = ctx.register_image(Arc::clone(&image));
            let font_id = ctx.register_font_page(Arc::clone(&page));
            let cmap_id = ctx.register_color_map(Arc::clone(&cmap));
            ctx.register_data_ptr(0x1000);
            let data_id = ctx.get_data_ptr_id(0x1000);
            assert_eq!(ctx.data_ptr_count(), 1);

            ctx.decrement_ref(canvas_id);
            ctx.decrement_ref(image_id);
            ctx.decrement_ref(font_id);
            ctx.decrement_ref(cmap_id);
            if let Some(id) = data_id {
                ctx.decrement_ref(id);
            }

            (canvas_id, image_id, font_id, cmap_id, data_id)
        };

        *state.initialized.write().unwrap() = true;

        let purged = state.purge_dangling().unwrap();
        assert!(purged >= 4);

        let ctx_guard = render_context.read().unwrap();
        assert!(!ctx_guard.has_resource(ids.0));
        assert!(!ctx_guard.has_resource(ids.1));
        assert!(!ctx_guard.has_resource(ids.2));
        assert!(!ctx_guard.has_resource(ids.3));
        assert_eq!(ctx_guard.data_ptr_count(), 0);
        if let Some(data_id) = ids.4 {
            assert!(!ctx_guard.has_resource(data_id));
        }
    }

    #[test]
    fn test_purge_dangling_preserves_screens() {
        let mut state = GraphicsState::new();
        let render_context = state.render_context();
        let canvas = Arc::new(RwLock::new(Canvas::new_rgba(6, 6)));

        {
            let mut ctx = render_context.write().unwrap();
            ctx.set_screen(ScreenType::Main, Arc::clone(&canvas));
            if let Some(meta) = ctx.get_metadata(ScreenType::Main as u32) {
                assert_eq!(meta.resource_type, ResourceType::Screen);
            }
            ctx.decrement_ref(ScreenType::Main as u32);
        }

        *state.initialized.write().unwrap() = true;

        let purged = state.purge_dangling().unwrap();
        assert_eq!(purged, 0);
        let ctx_guard = render_context.read().unwrap();
        assert!(ctx_guard.get_screen(ScreenType::Main).is_some());
    }

    #[test]
    fn test_set_frame_rate_tick_base() {
        let state = GraphicsState::new();
        state.set_frame_rate_tick_base(100);
        assert_eq!(state.get_frame_rate_tick_base(), 100);
    }

    #[test]
    fn test_set_frame_rate_tick_base_updates_last_tick() {
        let state = GraphicsState::new();
        state.set_frame_rate_tick_base(120);
        assert_eq!(state.get_frame_rate_tick_base(), 120);
        assert_eq!(state.frame_rate.read().unwrap().last_tick_ms, 120);
    }

    #[test]
    fn test_set_graphic_scale() {
        let state = GraphicsState::new();
        let old_scale = state.set_graphic_scale(512);
        assert_eq!(old_scale, 256);
        assert_eq!(state.get_graphic_scale(), 512);
    }

    #[test]
    fn test_set_graphic_scale_mode() {
        let state = GraphicsState::new();
        let old_mode = state.set_graphic_scale_mode(ScaleMode::Bilinear);
        assert_eq!(old_mode, ScaleMode::Nearest);
        assert_eq!(state.get_graphic_scale_mode(), ScaleMode::Bilinear);
    }

    #[test]
    fn test_set_flags() {
        let state = GraphicsState::new();
        let flags = GfxFlags::new(GfxFlags::SHOW_FPS | GfxFlags::FULLSCREEN);
        state.set_flags(flags);
        assert!(state.show_fps());
        assert!(state.is_fullscreen());
    }

    #[test]
    fn test_no_show_fps_by_default() {
        let state = GraphicsState::new();
        assert!(!state.show_fps());
    }

    #[test]
    fn test_set_driver() {
        let state = GraphicsState::new();
        state.set_driver(GfxDriver::SdlOpenGL);
        assert_eq!(state.get_driver(), GfxDriver::SdlOpenGL);
    }

    #[test]
    fn test_update_frame_rate() {
        let state = GraphicsState::new();
        for _ in 0..60 {
            state.update_frame_rate(17);
        }
        let fps = state.get_frame_rate();
        assert!((55.0..=65.0).contains(&fps), "FPS: {}", fps);
        assert_eq!(state.frame_rate.read().unwrap().last_tick_ms, 17);
    }

    #[test]
    fn test_update_frame_rate_skips_zero_delta() {
        let state = GraphicsState::new();
        state.update_frame_rate(20);
        let previous = state.get_frame_rate();
        state.update_frame_rate(0);
        assert_eq!(state.get_frame_rate(), previous);
        assert_eq!(state.frame_rate.read().unwrap().last_tick_ms, 20);
    }

    #[test]
    fn test_get_frame_rate_initial() {
        let state = GraphicsState::new();
        assert_eq!(state.get_frame_rate(), 60.0);
    }

    #[test]
    fn test_show_fps_flag_default_false() {
        let state = GraphicsState::new();
        assert!(!state.show_fps());
    }

    #[test]
    fn test_supports_hardware_scaling_default() {
        let state = GraphicsState::new();
        assert_eq!(state.supports_hardware_scaling(), None);
    }

    #[test]
    fn test_init_global_state() {
        let state = init_global_state();
        let guard = state.lock().unwrap();
        assert!(!guard.is_initialized());
    }
}
// SAFETY: GraphicsState contains a GraphicsDriver trait object that we only access
// from the main thread per SDL2 requirements. The actual drivers implement Send/Sync
// via unsafe impls. This is safe because:
// 1. All graphics operations happen on the main thread
// 2. SDL2 requires main-thread-only usage
// 3. The Mutex ensures only one thread accesses the state at a time
unsafe impl Send for GraphicsState {}
unsafe impl Sync for GraphicsState {}
