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
#[cfg(test)]
#[path = "shipyard/catalog_tests.rs"]
mod shipyard_catalog_tests;
#[path = "shipyard/hangar.rs"]
mod shipyard_hangar;
pub(crate) mod shipyard_layout;
pub mod starbase;

#[cfg(not(test))]
pub(crate) mod save_problem;

pub(crate) const fn rgb15(r: u8, g: u8, b: u8) -> crate::navigation::abi::Color {
    crate::navigation::abi::Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
