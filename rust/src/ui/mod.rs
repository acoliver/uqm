//! In-game menus and flagship status display.

#[cfg(not(test))]
pub mod border;
pub mod util;

pub mod flash;
pub(crate) mod flash_math;
pub mod settings;

pub mod confirm;
pub mod getchar;
pub mod menu;
pub mod menu_state;
#[cfg(not(test))]
mod palette;

pub mod shipstat;
pub mod sis;
pub(crate) mod sis_capacity_math;

mod flagship_stats;
pub mod outfit;
mod outfit_state;
pub mod shipyard;
#[path = "shipyard/hangar.rs"]
mod shipyard_hangar;
pub(crate) mod shipyard_layout;
pub mod starbase;
