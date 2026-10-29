pub(crate) mod access;
pub mod context;
pub mod hooks;
pub mod legacy;
pub mod net;
pub mod race;
pub mod record;

pub mod table;
pub use context::{Battle, Cx, World};
pub use hooks::{CollisionFn, CollisionHook, ElementFn, ElementHook, ElementHooks};
pub use race::{
    Concern, Concerns, Race, RaceDesc, RaceState, StoryKey, Subject, WeaponOrigin, WeaponSpawn,
};
pub use table::{ElementId, ElementTable, Parent, ShipId};

pub mod views;

pub use race::{Callbacks, Selection};

pub(crate) mod androsynth_template;
pub(crate) mod vux_template;

pub(crate) mod kernel;
pub mod template;
pub use template::{RaceResources, RaceTemplate};
