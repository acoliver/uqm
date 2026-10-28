#[cfg(not(test))]
mod access;
pub mod context;
pub mod hooks;
pub mod legacy;
pub mod net;
pub mod race;
pub mod record;

pub mod table;
pub use context::{Battle, Cx, World};
pub use hooks::{CollisionFn, CollisionHook, ElementFn, ElementHook, ElementHooks};
pub use race::{Concern, Concerns, Race, RaceDesc, RaceState, WeaponOrigin, WeaponSpawn};
pub use table::{ElementId, ElementTable, Parent, ShipId};
