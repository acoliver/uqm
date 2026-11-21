//! The SDL2 display: window, renderer and event pump.
mod display;
pub mod input;
#[cfg(all(feature = "debug-process", target_os = "macos"))]
mod macos;

pub use display::SdlDisplay;
