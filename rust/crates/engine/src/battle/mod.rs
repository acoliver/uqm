pub mod access;
pub mod context;
pub mod display;
pub use display::DisplayPrimitive;
pub mod frames;
pub mod hooks;
pub mod input;
pub use access::link_queue;
pub mod ai_constants;
pub mod cyborg;
pub mod net;
pub mod observation;
pub mod race;
mod race_ops;
pub mod race_services;
pub use access::androsynth_template;
pub use access::record;
pub use access::table;
pub use access::views;
pub use access::vux_template;
pub use context::{Battle, Cx, World};
pub use hooks::{CollisionFn, CollisionHook, ElementFn, ElementHook, ElementHooks};
pub use race::{Callbacks, Selection};
pub use race::{
    Concern, Concerns, Race, RaceDesc, RaceState, StoryKey, Subject, WeaponOrigin, WeaponSpawn,
};
pub use table::{ElementId, ElementTable, Parent, ShipId};

pub mod kernel;
pub use access::schema;
pub use access::template;
pub use template::{RaceResources, RaceTemplate};

mod descriptor;

pub mod hyperspace;

pub mod flags;
pub mod gravity;
pub mod intercept;
pub mod tracking;

pub mod motion;
mod projectile;
pub mod services;
pub mod statistics;
pub mod weapon;
pub mod world_elements;

/// Display primitive kinds (C: `gfx_object`).
pub const STAMP_PRIMITIVE: u8 = 1;
pub const LINE_PRIMITIVE: u8 = 3;
pub const EMPTY_PRIMITIVE: u8 = 7;
