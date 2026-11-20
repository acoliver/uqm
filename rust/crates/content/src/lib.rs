//! Game content: race behaviour and descriptors, system generators, dialogue
//! scripts and data tables.
#![forbid(unsafe_code)]

// The same names the root package exports, so `crate::…` paths inside the
// moved modules keep resolving (A7, until U8-C).
pub use uqm_engine as engine;
pub use uqm_engine::{callback, collections, graphics, math, sound, state, threading};
pub use uqm_engine::{log_debug, log_error, log_fatal, log_info, log_warning};

pub mod comm {
    pub mod races;
}
pub mod planets {
    pub mod generate;
    pub mod tables;
}
pub mod ships;
