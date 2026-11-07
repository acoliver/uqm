//! Values published by a lander trip at observation boundaries.

pub mod terminal {
    pub const RUNNING: u32 = 0;
    pub const RETURNED: u32 = 1;
    pub const DESTROYED: u32 = 2;
    pub const ABORTED: u32 = 3;
    pub const ADAPTER_FAILURE: u32 = 4;
    pub const FRAME_BUDGET: u32 = 5;
}

/// A mineral deposit that is still on the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MineralDeposit {
    pub x: i32,
    pub y: i32,
    pub category: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanetSideObservation {
    pub generation: u64,
    pub active: bool,
    pub frames: u64,
    pub start_x: i32,
    pub start_y: i32,
    pub current_x: i32,
    pub current_y: i32,
    pub start_crew: u8,
    pub current_crew: u8,
    /// Lander facing, one of the sixteen.
    pub facing: u8,
    /// Mineral units in the lander hold, per element category.
    pub mineral_units: [u16; 8],
    pub sound_count: u64,
    pub last_sound: u32,
    pub terminal: u32,
    pub returned_crew: i16,
    pub returned_minerals: u16,
    /// Live lifecycle phase code. See [`phase_name`].
    pub phase: u32,
    /// Graphics batch levels restored after a native callback during this trip.
    pub batch_depth_corrections: u64,
    /// Mineral deposits collected while surface frames were presented.
    pub mineral_pickups: u64,
    /// Stun-bolt impacts on live creatures while surface frames were presented.
    pub creature_hits: u64,
    /// Collisions that connected only across the wrapped horizontal seam.
    pub seam_hits: u64,
}

/// Human-readable name for an observed phase code.
#[must_use]
pub const fn phase_name(code: u32) -> &'static str {
    match code {
        1 => "Warmup",
        2 => "Launch",
        3 => "Landing",
        4 => "Active",
        5 => "TakingOff",
        6 => "Explosion",
        7 => "Return",
        8 => "Complete",
        9 => "Aborted",
        _ => "None",
    }
}
