//! In-game menus and flagship status display.

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod border;
pub mod util;

pub mod flash;
pub(crate) mod flash_math;
pub mod settings;

pub mod confirm;
#[cfg(any(not(test), feature = "linked_c_archive"))]
mod palette;
pub mod getchar;
