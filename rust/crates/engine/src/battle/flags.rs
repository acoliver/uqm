//! Fixed-width flags used by engine battle records.

pub const PLAYER_SHIP: u32 = 4;
pub const APPEARING: u32 = 8;
pub const DISAPPEARING: u32 = 16;
pub const NONSOLID: u32 = 64;
pub const FINITE_LIFE: u32 = 1024;
pub const PRE_PROCESS: u32 = 2048;
pub const MAX_TIME_VALUE: u32 = 257;
pub const DEFY_PHYSICS: u32 = 512;
pub const POST_PROCESS: u32 = 4096;
pub const IGNORE_VELOCITY: u32 = 8192;
pub const BACKGROUND_OBJECT: u32 = 32768;
pub const HYPERJUMP_LIFE: u32 = 15;
pub const NUM_EXPLOSION_FRAMES: u32 = 12;

pub const NORMAL_LIFE: u32 = 1;
pub const CHANGING: u32 = 32;
pub const COLLISION: u32 = 128;
pub const CREW_OBJECT: u32 = 16384;
pub const CREW_IMMUNE: u32 = 64;
pub const LOW_ON_ENERGY: u32 = 32;
pub const SHIP_BEYOND_MAX_SPEED: u32 = 64;
pub const SHIP_AT_MAX_SPEED: u32 = 128;
pub const SHIP_IN_GRAVITY_WELL: u32 = 256;
pub const RPG_PLAYER_NUM: u32 = 0;
pub const NPC_PLAYER_NUM: u32 = 1;
pub const NEUTRAL_PLAYER_NUM: i32 = -1;
pub const IN_LAST_BATTLE: std::os::raw::c_uint = 1;

/// Primitive kinds that `STAMP_PRIMITIVE` and `LINE_PRIMITIVE` leave out.
pub const POINT_PRIMITIVE: u8 = 0;
pub const STAMPFILL_PRIMITIVE: u8 = 2;

/// Status bits replaced together after a ship thrust update.
pub const SHIP_SPEED_FLAGS: u16 =
    (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16;
