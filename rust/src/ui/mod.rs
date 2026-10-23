//! In-game menus and flagship status display.

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod border;
pub mod util;

pub mod flash;
pub(crate) mod flash_math;
pub mod settings;

pub mod confirm;
pub mod getchar;
pub mod menu;
pub mod menu_state;
#[cfg(any(not(test), feature = "linked_c_archive"))]
mod palette;

pub mod shipstat;
pub mod sis;
pub(crate) mod sis_capacity_math;

pub mod outfit;
pub mod shipyard;
pub(crate) mod shipyard_layout;
pub mod starbase;
