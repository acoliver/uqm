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
pub mod memory;
pub mod network;
pub mod observe_port;
pub mod platform;
pub mod resource;
pub mod sound;
pub mod strings;
pub mod threading;
pub mod time;
pub mod video;

pub mod battle;
pub mod clock;
pub mod observe;

pub mod handoff;
pub mod input;
pub mod kernel;
mod owner;
pub mod world;
pub use kernel::Kernel;
pub use owner::Owner;
pub use owner::Projection;
pub use world::StoryKey;
pub use world::{Game, StarId, World, WorldData};

pub mod save;
