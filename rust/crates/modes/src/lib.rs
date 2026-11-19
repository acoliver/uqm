//! Game modes: battle, melee setup, navigation, solar system, orbit, planet
//! side, communication, starbase and the menus.
#![forbid(unsafe_code)]

// The same names the root package exports, so `crate::…` paths inside the
// moved modules keep resolving (A7, until U8-C).
pub use uqm_content::ships;
pub use uqm_engine as engine;
pub use uqm_engine::input_port as input;
pub use uqm_engine::{
    callback, collections, graphics, io, log_abi, logging, math, platform, resource, sound, state,
    strings, threading, time, video,
};
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
