//! Shared engine services and their SDL/OS backends.

pub mod bridge_log;
pub mod callback;
pub mod collections;
pub mod configuration;
pub mod graphics;
pub mod host;
pub mod input_port;
pub mod io;
pub mod log_abi;
pub mod logging;
pub mod math;
pub mod network;
pub mod observe_port;
pub mod platform;
pub mod resource;
pub mod sound;
pub mod state;
pub mod strings;
pub mod sync;
pub mod threading;
pub mod time;
pub mod video;

pub mod battle;
pub mod clock;
pub mod observe;

pub mod input;
pub mod kernel;
pub mod world;
pub use kernel::Kernel;
pub use world::StoryKey;
pub use world::{Game, StarId, World, WorldData};

pub mod save;
pub mod species;

// The engine's own unit tests build a Kernel without the font installers
// these fixtures call.
#[cfg(all(feature = "test-support", not(test)))]
pub mod test_support;
