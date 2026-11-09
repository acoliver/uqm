//! In-game menus and flagship status display.

pub mod border;
pub mod util;

pub mod settings;

pub mod confirm;
pub mod getchar;
pub mod menu;
pub mod menu_state;
mod palette;

pub mod shipstat;
pub mod sis;
pub(crate) mod sis_capacity_math;

mod flagship_stats;
pub mod outfit;
mod outfit_state;
mod session;
pub use session::MenuServices;
pub mod shipyard;
#[path = "shipyard/catalog.rs"]
mod shipyard_catalog;
#[cfg(test)]
#[path = "shipyard/catalog_tests.rs"]
mod shipyard_catalog_tests;
#[path = "shipyard/crew.rs"]
mod shipyard_crew;
#[cfg(test)]
#[path = "shipyard/crew_tests.rs"]
mod shipyard_crew_tests;
#[path = "shipyard/frame.rs"]
mod shipyard_frame;
#[path = "shipyard/hangar.rs"]
mod shipyard_hangar;
pub(crate) mod shipyard_layout;
#[path = "shipyard/selection.rs"]
mod shipyard_selection;
pub mod starbase;

pub mod save_problem;

pub(crate) const fn rgb15(r: u8, g: u8, b: u8) -> crate::graphics::native_abi::Color {
    crate::graphics::native_abi::Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
