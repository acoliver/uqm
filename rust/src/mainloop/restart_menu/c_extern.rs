//! Remaining communication imports pending Part 10's caller migration.
pub use crate::graphics::cmap_abi::{FadeScreen, FADE_ALL_TO_COLOR};
#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::SetMenuSounds;
