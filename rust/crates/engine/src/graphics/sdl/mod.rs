//! The SDL2 display: window, renderer and event pump.
mod display;
mod events;
#[cfg(all(feature = "debug-process", target_os = "macos"))]
mod macos;

pub(crate) use display::display_sample;
pub use display::SdlDisplay;
pub use events::process_events;
