//! Graphics core state and configuration.

use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

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
}

impl Default for FrameRateState {
    fn default() -> Self {
        Self {
            rate: 60.0,
            tick_base: 0,
        }
    }
}

/// Core graphics system state.
#[derive(Debug)]
pub struct GraphicsState {
    driver: AtomicI32,
    flags: AtomicU32,
    dimensions: Mutex<ScreenDimensions>,
    scale_config: Mutex<ScaleConfig>,
    frame_rate: Mutex<FrameRateState>,

    gamma: AtomicU32,
    initialized: AtomicBool,
}

impl GraphicsState {
    /// Create a new graphics state with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            driver: AtomicI32::new(GfxDriver::default() as i32),
            flags: AtomicU32::new(0),
            dimensions: Mutex::new(ScreenDimensions::default()),
            scale_config: Mutex::new(ScaleConfig::default()),
            frame_rate: Mutex::new(FrameRateState::default()),
            gamma: AtomicU32::new(1.0_f32.to_bits()),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the graphics subsystem.
    pub fn init(
        &self,
        driver: GfxDriver,
        flags: GfxFlags,

        _renderer: Option<&CStr>,
        width: i32,
        height: i32,
    ) -> Result<(), String> {
        self.set_driver(driver);
        self.set_flags(flags);
        self.set_dimensions(ScreenDimensions {
            width,
            height,
            actual_width: width,
            actual_height: height,
            color_depth: self.get_dimensions().color_depth,
        });
        self.initialized.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Reinitialize graphics with new parameters.
    pub fn reinit(
        &self,
        driver: GfxDriver,
        flags: GfxFlags,
        width: i32,
        height: i32,
    ) -> Result<(), String> {
        if !self.is_initialized() {
            return Err("Graphics not initialized".to_string());
        }
        self.init(driver, flags, None, width, height)
    }

    /// Shutdown the graphics subsystem.
    pub fn uninit(&self) {
        self.initialized.store(false, Ordering::Relaxed);
    }

    /// Check if graphics are initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// Swap backbuffers and update display.
    pub fn swap_buffers(&self, _force_redraw: RedrawMode) {
        debug_assert!(self.is_initialized(), "graphics not initialized");
    }

    /// Process pending graphics events.
    pub fn process_events(&self) {
        debug_assert!(self.is_initialized(), "graphics not initialized");
    }

    /// Flush pending graphics commands.
    pub fn flush(&self) {
        debug_assert!(self.is_initialized(), "graphics not initialized");
    }

    /// Purge dangling graphics resources.
    pub fn purge_dangling(&self) {
        debug_assert!(self.is_initialized(), "graphics not initialized");
    }

    /// Set the gamma correction value.
    pub fn set_gamma(&self, gamma: f32) -> bool {
        self.gamma.store(gamma.to_bits(), Ordering::Relaxed);
        true
    }

    /// Get the current gamma value.
    #[must_use]
    pub fn get_gamma(&self) -> f32 {
        f32::from_bits(self.gamma.load(Ordering::Relaxed))
    }

    /// Get the current frame rate.
    #[must_use]
    pub fn get_frame_rate(&self) -> f32 {
        self.frame_rate.lock().unwrap().rate
    }

    /// Set the frame rate tick base.
    pub fn set_frame_rate_tick_base(&self, tick_base: i32) {
        self.frame_rate.lock().unwrap().tick_base = tick_base;
    }

    /// Get the frame rate tick base.
    #[must_use]
    pub fn get_frame_rate_tick_base(&self) -> i32 {
        self.frame_rate.lock().unwrap().tick_base
    }

    /// Update frame rate (called by driver each frame).
    pub fn update_frame_rate(&self, delta_time_ms: u32) {
        if delta_time_ms == 0 {
            return;
        }
        let fps = 1000.0 / delta_time_ms as f32;
        self.frame_rate.lock().unwrap().rate = fps;
    }
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
        *self.dimensions.lock().unwrap()
    }

    /// Set screen dimensions.
    pub fn set_dimensions(&self, dims: ScreenDimensions) {
        *self.dimensions.lock().unwrap() = dims;
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
        *self.scale_config.lock().unwrap()
    }

    /// Get graphic scale factor.
    #[must_use]
    pub fn get_graphic_scale(&self) -> i32 {
        self.scale_config.lock().unwrap().scale
    }

    /// Set graphic scale factor.
    pub fn set_graphic_scale(&self, scale: i32) -> i32 {
        let mut config = self.scale_config.lock().unwrap();
        let previous = config.scale;
        config.scale = scale;
        previous
    }

    /// Get graphic scale mode.
    #[must_use]
    pub fn get_graphic_scale_mode(&self) -> ScaleMode {
        self.scale_config.lock().unwrap().mode
    }

    /// Set graphic scale mode.
    pub fn set_graphic_scale_mode(&self, mode: ScaleMode) -> ScaleMode {
        let mut config = self.scale_config.lock().unwrap();
        let previous = config.mode;
        config.mode = mode;
        previous
    }

    /// Get graphics flags.
    #[must_use]
    pub fn get_flags(&self) -> GfxFlags {
        GfxFlags::new(self.flags.load(Ordering::Relaxed))
    }

    /// Set graphics flags.
    pub fn set_flags(&self, flags: GfxFlags) {
        self.flags.store(flags.bits(), Ordering::Relaxed);
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
        match self.driver.load(Ordering::Relaxed) {
            0 => GfxDriver::SdlOpenGL,
            _ => GfxDriver::SdlPure,
        }
    }

    /// Set graphics driver.
    pub fn set_driver(&self, driver: GfxDriver) {
        self.driver.store(driver as i32, Ordering::Relaxed);
    }

    /// Check if hardware scaling is supported.
    #[must_use]
    pub fn supports_hardware_scaling(&self) -> Option<bool> {
        if !self.is_initialized() {
            return None;
        }
        Some(self.get_driver() == GfxDriver::SdlOpenGL)
    }
}

static GLOBAL_STATE: OnceLock<GraphicsState> = OnceLock::new();

/// Initialize global graphics state.
pub fn init_global_state() -> &'static GraphicsState {
    GLOBAL_STATE.get_or_init(GraphicsState::new)
}

/// Get a reference to the global graphics state.
#[must_use]
pub fn global_state() -> &'static GraphicsState {
    GLOBAL_STATE.get().expect("graphics state not initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn test_gfx_driver_values() {
        assert_eq!(GfxDriver::SdlOpenGL as i32, 0);
        assert_eq!(GfxDriver::SdlPure as i32, 1);
    }

    #[test]
    fn test_reinit_fails_when_uninitialized() {
        let state = GraphicsState::new();
        let result = state.reinit(GfxDriver::SdlPure, GfxFlags::new(0), 640, 480);
        assert!(result.is_err());
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
        let state = GraphicsState::new();
        assert_eq!(state.get_gamma(), 1.0);
        let success = state.set_gamma(1.5);
        assert!(success);
        assert_eq!(state.get_gamma(), 1.5);
    }

    #[test]
    fn test_set_frame_rate_tick_base() {
        let state = GraphicsState::new();
        state.set_frame_rate_tick_base(100);
        assert_eq!(state.get_frame_rate_tick_base(), 100);
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
    fn test_supports_hardware_scaling() {
        let state = GraphicsState::new();
        assert_eq!(state.supports_hardware_scaling(), None);
        state
            .init(GfxDriver::SdlOpenGL, GfxFlags::new(0), None, 640, 480)
            .unwrap();
        assert_eq!(state.supports_hardware_scaling(), Some(true));
    }

    #[test]
    fn test_init_global_state() {
        let state = init_global_state();
        assert!(!state.is_initialized());
    }
}
