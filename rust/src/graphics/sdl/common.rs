//!
//! Common types, traits, and errors for SDL/OpenGL drivers.
//!
//! This module provides shared infrastructure for both SDL2 pure software
//! and OpenGL-accelerated drivers, mirroring the common code in
//! `sc2/src/libs/graphics/sdl/sdl_common.c`.
//!

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use crate::graphics::tfb_draw::TFImage;

/// Error types for driver operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverError {
    /// Driver not initialized.
    NotInitialized,
    /// Invalid driver type specified.
    InvalidDriver,
    /// Video mode configuration failed.
    VideoModeFailed(String),
    /// Window creation failed.
    WindowCreationFailed(String),
    /// Renderer creation failed.
    RendererCreationFailed(String),
    /// OpenGL context creation failed.
    GlContextFailed(String),
    /// Invalid operation for current state.
    InvalidOperation(String),
    /// Gamma correction not supported by this driver.
    GammaNotSupported,
    /// Fullscreen toggle failed.
    FullscreenFailed(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "Graphics driver not initialized"),
            Self::InvalidDriver => write!(f, "Invalid graphics driver type"),
            Self::VideoModeFailed(msg) => write!(f, "Video mode configuration failed: {}", msg),
            Self::WindowCreationFailed(msg) => write!(f, "Window creation failed: {}", msg),
            Self::RendererCreationFailed(msg) => write!(f, "Renderer creation failed: {}", msg),
            Self::GlContextFailed(msg) => write!(f, "OpenGL context creation failed: {}", msg),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Self::GammaNotSupported => write!(f, "Gamma correction not supported by this driver"),
            Self::FullscreenFailed(msg) => write!(f, "Fullscreen toggle failed: {}", msg),
        }
    }
}

impl std::error::Error for DriverError {}

/// Result type for driver operations.
pub type DriverResult<T> = Result<T, DriverError>;

/// Redraw mode flags for buffer swapping, mirroring TFB_REDRAW_* constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RedrawMode {
    /// No forced redraw (TFB_REDRAW_NO).
    #[default]
    None = 0,
    /// Fading effect active (TFB_REDRAW_FADING).
    Fading = 1,
    /// Exposure event (TFB_REDRAW_EXPOSE).
    Expose = 2,
    /// Full forced redraw (TFB_REDRAW_YES).
    Full = 3,
}

impl RedrawMode {
    /// Create from integer value.
    #[must_use]
    pub const fn from_int(value: i32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Fading,
            2 => Self::Expose,
            3 => Self::Full,
            _ => Self::None,
        }
    }

    /// Convert to integer value.
    #[must_use]
    pub const fn to_int(self) -> i32 {
        self as i32
    }

    /// Check if any redraw is forced.
    #[must_use]
    pub fn should_redraw(self) -> bool {
        self != Self::None
    }
}

/// Screen types, mirroring TFB_SCREEN_* constants in C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Screen {
    /// Main screen (TFB_SCREEN_MAIN).
    Main = 0,
    /// Extra screen (TFB_SCREEN_EXTRA).
    Extra = 1,
    /// Transition screen (TFB_SCREEN_TRANSITION).
    Transition = 2,
}

impl Screen {
    /// Get screen index.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Get screen name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Extra => "Extra",
            Self::Transition => "Transition",
        }
    }
}

const NUM_SCREENS: usize = 3;

/// Update region for tracking dirty rectangles, mirroring SDL_Rect in C.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpdateRect {
    /// Top-left x coordinate.
    pub x: i32,
    /// Top-left y coordinate.
    pub y: i32,
    /// Width of the region.
    pub w: u32,
    /// Height of the region.
    pub h: u32,
}

impl UpdateRect {
    /// Create a new update rectangle.
    #[must_use]
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Check if the rectangle is empty (zero-area).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }
}

/// Screen dimension state, mirroring ScreenWidth/ScreenHeight and actual dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenDims {
    /// Logical screen width (e.g., 320).
    pub width: u32,
    /// Logical screen height (e.g., 240).
    pub height: u32,
    /// Actual display width (may differ with scaling).
    pub actual_width: u32,
    /// Actual display height (may differ with scaling).
    pub actual_height: u32,
}

/// Wrapper for SDL2 Event to avoid exposing sdl2's event type directly.
#[derive(Debug, Clone)]
pub enum GraphicsEvent {
    /// Quit event (window close, quit command, etc.).
    Quit,
    /// Key press event with keycode.
    KeyDown(i32),
    /// Key release event with keycode.
    KeyUp(i32),
    /// Mouse button press event.
    MouseButtonDown(u8),
    /// Mouse button release event.
    MouseButtonUp(u8),
    /// Mouse motion event with coordinates.
    MouseMotion(i32, i32),
    /// Window event (resize, expose, etc.).
    WindowEvent,
    /// Unknown/other event.
    Unknown,
}

impl Default for ScreenDims {
    fn default() -> Self {
        Self {
            width: 320,
            height: 240,
            actual_width: 320,
            actual_height: 240,
        }
    }
}

/// Driver configuration state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverConfig {
    /// Screen width in pixels.
    pub width: u32,
    /// Screen height in pixels.
    pub height: u32,
    /// Fullscreen mode flag.
    pub fullscreen: bool,
    /// Linear scaling flag (smooth vs pixelated).
    pub linear_scaling: bool,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            width: 320,
            height: 240,
            fullscreen: false,
            linear_scaling: false,
        }
    }
}

impl DriverConfig {
    /// Create a new driver configuration.
    #[must_use]
    pub const fn new(width: u32, height: u32, fullscreen: bool) -> Self {
        Self {
            width,
            height,
            fullscreen,
            linear_scaling: false,
        }
    }

    /// Create configuration for fullscreen mode.
    #[must_use]
    pub const fn fullscreen(width: u32, height: u32) -> Self {
        Self::new(width, height, true)
    }

    /// Create configuration for windowed mode.
    #[must_use]
    pub const fn windowed(width: u32, height: u32) -> Self {
        Self::new(width, height, false)
    }

    /// Set linear scaling mode.
    #[must_use]
    pub const fn with_linear_scaling(mut self, linear: bool) -> Self {
        self.linear_scaling = linear;
        self
    }

    /// Check if fullscreen is enabled.
    #[must_use]
    pub const fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    /// Check if linear scaling is enabled.
    #[must_use]
    pub const fn is_linear_scaling(&self) -> bool {
        self.linear_scaling
    }
}

/// Trait for graphics drivers.
///
/// This trait defines the common interface that all graphics drivers
/// (SDL2 pure, OpenGL, etc.) must implement.
///
/// This mirrors the driver structure in C code where different backends
/// (sdl2/opengl) implement similar function tables.
pub trait GraphicsDriver {
    /// Initialize the driver with the given configuration.
    ///
    /// Corresponds to `TFB_Pure_InitGraphics` / `TFB_GL_InitGraphics` in C.
    fn init(&mut self, config: &DriverConfig) -> DriverResult<()>;

    /// Shutdown the driver and release resources.
    ///
    /// Corresponds to `TFB_Pure_UninitGraphics` / `TFB_GL_UninitGraphics` in C.
    fn uninit(&mut self) -> DriverResult<()>;

    /// Swap buffers (display the rendered frame).
    ///
    /// This corresponds to `TFB_SwapBuffers` in C, with the redraw mode
    /// indicating whether a full or partial refresh is needed.
    ///
    /// # Safety
    ///
    /// This function must only be called from the graphics/rendering thread.
    fn swap_buffers(&mut self, mode: RedrawMode) -> DriverResult<()>;

    /// Set gamma correction level.
    ///
    /// Corresponds to `TFB_SetGamma` in C.
    ///
    /// # Arguments
    ///
    /// * `gamma` - Gamma value (1.0 = no correction, >1.0 = brighter, <1.0 = darker)
    ///
    /// # Returns
    ///
    /// `Ok(())` if gamma was set successfully, `Err` if an error occurred.
    fn set_gamma(&mut self, gamma: f32) -> DriverResult<()>;

    /// Get current gamma correction level.
    ///
    /// # Returns
    ///
    /// The current gamma value.
    #[must_use]
    fn get_gamma(&self) -> f32;

    /// Toggle fullscreen mode.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if mode was toggled, `Ok(false)` if mode is unchanged,
    /// `Err` if toggling failed.
    fn toggle_fullscreen(&mut self) -> DriverResult<bool>;

    /// Check if currently in fullscreen mode.
    #[must_use]
    fn is_fullscreen(&self) -> bool;

    /// Check if the driver is initialized.
    #[must_use]
    fn is_initialized(&self) -> bool;

    /// Check if hardware scaling is supported.
    ///
    /// SDL2 pure driver always returns `false`.
    /// OpenGL driver returns `true`.
    #[must_use]
    fn supports_hardware_scaling(&self) -> bool;

    /// Get current screen dimensions.
    #[must_use]
    fn get_dimensions(&self) -> ScreenDims;

    /// Get screen surface for direct pixel access (readonly).
    ///
    /// This provides access to the pixel buffer for the specified screen.
    ///
    /// # Arguments
    ///
    /// * `screen` - Screen index (0=Main, 1=Extra, 2=Transition)
    ///
    /// # Returns
    ///
    /// Pointer to pixel data if available, error otherwise.
    fn get_screen_pixels(&self, screen: usize) -> DriverResult<*const u8>;

    /// Get screen surface for direct pixel access (mutable).
    ///
    /// This provides mutable access to the pixel buffer for the specified screen.
    ///
    /// # Arguments
    ///
    /// * `screen` - Screen index (0=Main, 1=Extra, 2=Transition)
    ///
    /// # Returns
    ///
    /// Mutable pointer to pixel data if available, error otherwise.
    fn get_screen_pixels_mut(&mut self, screen: usize) -> DriverResult<*mut u8>;

    /// Get screen pitch (bytes per row).
    ///
    /// # Arguments
    ///
    /// * `screen` - Screen index (0=Main, 1=Extra, 2=Transition)
    ///
    /// # Returns
    ///
    /// The pitch in bytes if available, error otherwise.
    fn get_screen_pitch(&self, screen: usize) -> DriverResult<usize>;

    /// Poll for pending events.
    ///
    /// Returns a vector of pending graphics events suitable for further processing
    /// by the higher-level graphics system.
    ///
    /// # Returns
    ///
    /// Vector of pending events, empty if none available.
    fn poll_events(&mut self) -> DriverResult<Vec<GraphicsEvent>>;

    /// Notify the driver that an image resource was removed.
    ///
    /// Drivers can override this to purge any cached textures.
    fn on_image_removed(&mut self, _image: &TFImage) {}
}

/// State container for tracking driver configuration.
#[derive(Debug)]
pub struct DriverState {
    /// Whether the driver is initialized.
    initialized: AtomicBool,
    /// Current configuration.
    config: Mutex<DriverConfig>,
    /// Current gamma value.
    gamma: AtomicU32,
}

impl Default for DriverState {
    fn default() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            config: Mutex::new(DriverConfig::default()),
            gamma: AtomicU32::new(1.0_f32.to_bits()),
        }
    }
}

impl DriverState {
    /// Create a new driver state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            config: Mutex::new(DriverConfig {
                width: 320,
                height: 240,
                fullscreen: false,
                linear_scaling: false,
            }),
            gamma: AtomicU32::new(1.0_f32.to_bits()),
        }
    }

    /// Mark the driver as initialized.
    pub fn mark_initialized(&self, config: DriverConfig) {
        *self.config.lock().unwrap() = config;
        self.initialized.store(true, Ordering::Relaxed);
    }

    /// Mark the driver as uninitialized.
    pub fn mark_uninitialized(&self) {
        self.initialized.store(false, Ordering::Relaxed);
    }

    /// Check if initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Relaxed)
    }

    /// Get current configuration.
    #[must_use]
    pub fn config(&self) -> DriverConfig {
        *self.config.lock().unwrap()
    }

    /// Get gamma value.
    #[must_use]
    pub fn gamma(&self) -> f32 {
        f32::from_bits(self.gamma.load(Ordering::Relaxed))
    }

    /// Set gamma value.
    pub fn set_gamma(&self, gamma: f32) {
        self.gamma.store(gamma.to_bits(), Ordering::Relaxed);
    }

    /// Update configuration.
    pub fn update_config(&self, config: DriverConfig) {
        *self.config.lock().unwrap() = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redraw_mode_values() {
        assert_eq!(RedrawMode::None as i32, 0);
        assert_eq!(RedrawMode::Fading as i32, 1);
        assert_eq!(RedrawMode::Expose as i32, 2);
        assert_eq!(RedrawMode::Full as i32, 3);
    }

    #[test]
    fn test_redraw_mode_should_redraw() {
        assert!(!RedrawMode::None.should_redraw());
        assert!(RedrawMode::Fading.should_redraw());
        assert!(RedrawMode::Expose.should_redraw());
        assert!(RedrawMode::Full.should_redraw());
    }

    #[test]
    fn test_driver_config_default() {
        let config = DriverConfig::default();
        assert_eq!(config.width, 320);
        assert_eq!(config.height, 240);
        assert!(!config.fullscreen);
    }

    #[test]
    fn test_driver_config_new() {
        let config = DriverConfig::new(640, 480, true);
        assert_eq!(config.width, 640);
        assert_eq!(config.height, 480);
        assert!(config.fullscreen);
    }

    #[test]
    fn test_driver_config_fullscreen() {
        let config = DriverConfig::fullscreen(800, 600);
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(config.is_fullscreen());
    }

    #[test]
    fn test_driver_config_windowed() {
        let config = DriverConfig::windowed(1024, 768);
        assert_eq!(config.width, 1024);
        assert_eq!(config.height, 768);
        assert!(!config.is_fullscreen());
    }

    #[test]
    fn test_screen_dims_default() {
        let dims = ScreenDims::default();
        assert_eq!(dims.width, 320);
        assert_eq!(dims.height, 240);
        assert_eq!(dims.actual_width, 320);
        assert_eq!(dims.actual_height, 240);
    }

    #[test]
    fn test_driver_state_default() {
        let state = DriverState::default();
        assert!(!state.is_initialized());
        assert_eq!(state.gamma(), 1.0);
        assert_eq!(state.config().width, 320);
    }

    #[test]
    fn test_driver_state_mark_initialized() {
        let state = DriverState::default();
        assert!(!state.is_initialized());

        let config = DriverConfig::new(640, 480, false);
        state.mark_initialized(config);

        assert!(state.is_initialized());
        assert_eq!(state.config().width, 640);
    }

    #[test]
    fn test_driver_state_mark_uninitialized() {
        let state = DriverState::default();
        state.mark_initialized(DriverConfig::default());
        assert!(state.is_initialized());

        state.mark_uninitialized();
        assert!(!state.is_initialized());
    }

    #[test]
    fn test_driver_state_set_gamma() {
        let state = DriverState::default();
        assert_eq!(state.gamma(), 1.0);

        state.set_gamma(1.5);
        assert_eq!(state.gamma(), 1.5);
    }

    #[test]
    fn test_driver_state_update_config() {
        let state = DriverState::default();
        assert_eq!(state.config().width, 320);

        let config = DriverConfig::new(800, 600, true);
        state.update_config(config);

        assert_eq!(state.config().width, 800);
        assert!(state.config().is_fullscreen());
    }
}
