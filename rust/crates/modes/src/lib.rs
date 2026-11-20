//! Game modes: battle, melee setup, navigation, solar system, orbit, planet
//! side, communication, starbase and the menus.
#![forbid(unsafe_code)]

// The same names the root package exports, so `crate::…` paths inside the
// moved modules keep resolving (A7, until U8-C).
pub use uqm_content::ships;
pub use uqm_engine as engine;
pub use uqm_engine::callback;
pub use uqm_engine::collections;
pub use uqm_engine::graphics;
pub use uqm_engine::input_port as input;
pub use uqm_engine::io;
pub use uqm_engine::log_abi;
pub use uqm_engine::logging;
pub use uqm_engine::math;
pub use uqm_engine::resource;
pub use uqm_engine::sound;
pub use uqm_engine::state;
pub use uqm_engine::strings;
pub use uqm_engine::threading;
pub use uqm_engine::time;
pub use uqm_engine::video;
pub use uqm_engine::{log_debug, log_error, log_fatal, log_info, log_warning};

pub mod battle;
pub mod comm;
pub mod front_end;
pub mod mainloop {
    pub mod restart_menu;
}
pub mod navigation;
pub mod pickship;
pub mod planet_side;
pub mod planets;
pub mod starbase;
pub mod supermelee;
pub mod ui;
