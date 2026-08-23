//! Planet-side lander gameplay.
//!
//! The deterministic core in this module owns lander rules independently of
//! rendering, audio, input polling, resource handles, and C global state.

pub mod adapters;
pub mod assembly;
pub mod assets;
pub mod automation_fixture;
pub mod batch_guard;
pub mod cargo;
pub mod collision;
pub mod collision_adapter;
pub mod controller;
pub mod creatures;
pub mod entities;
pub mod ffi;
pub mod generation;
pub mod generation_adapter;
pub mod geometry;
pub mod graphics_adapter;
pub mod hazards;
pub mod init_lander;
pub mod lifecycle;
pub mod mask_adapter;
pub mod menu_sounds;
pub mod model;
pub mod orbit_music;
pub mod orbit_scan;
pub mod report_adapter;
pub mod resources;
pub mod runtime;
pub mod selection;
pub mod session;
pub mod session_factory;
pub mod simulation;
pub mod special_effects;
pub mod telemetry;
pub mod visual_adapter;
pub mod world;

pub use cargo::{BioCargo, CargoPickup, MineralCargo, NUM_ELEMENT_CATEGORIES};
pub use creatures::{CreatureCatalog, CreatureKind, CreatureStats, CREATURE_COUNT};
pub use hazards::{
    apply_crew_damage, hazard_chance, thermal_hazard_rating, CrewDamage, HazardKind, SoundCue,
};
pub use model::{CrewCount, LanderUpgrades, ShieldSet, SurfacePoint};
