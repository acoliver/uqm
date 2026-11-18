// The names `navigation::hyperspace` still reads from the retired navigation
// ABI. Each resolves to its owner; the lane that converts that importer
// deletes this file (D10).
pub use crate::graphics::native_abi::POINT;
pub const IN_BATTLE: u32 = crate::engine::world::activity::activity_flags::IN_BATTLE as u32;
pub const MELNORME_SHIP: u32 = crate::engine::species::ship::MELNORME_SHIP as u32;
pub const SLYLANDRO_SHIP: u32 = crate::engine::species::ship::SLYLANDRO_SHIP as u32;
pub const ONE_SHOT_ENCOUNTER: u32 = crate::comm::dispatch::ONE_SHOT_ENCOUNTER as u32;
pub const PLAYER_SHIP: u32 = crate::battle::element::PLAYER_SHIP as u32;
pub const SIN_SHIFT: u32 = 14;
