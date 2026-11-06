pub(crate) mod access;
pub mod context;
pub(crate) mod display;
pub use display::DisplayPrimitive;
pub mod frames;
pub mod hooks;
pub mod input;
pub use access::link_queue;
pub mod net;
pub mod race;
pub use access::record;

pub use access::table;
pub use context::{Battle, Cx, World};
pub use hooks::{CollisionFn, CollisionHook, ElementFn, ElementHook, ElementHooks};
pub use race::{
    Concern, Concerns, Race, RaceDesc, RaceState, StoryKey, Subject, WeaponOrigin, WeaponSpawn,
};
pub use table::{ElementId, ElementTable, Parent, ShipId};

pub use access::views;

pub use race::{Callbacks, Selection};

#[cfg(not(test))]
pub(crate) use access::androsynth_template;
#[cfg(not(test))]
pub(crate) use access::vux_template;

pub(crate) mod kernel;
pub use access::template;
pub use template::{RaceResources, RaceTemplate};

pub use access::schema;
