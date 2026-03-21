//! Tactical Transitions Types (P13)
//!
//! Type definitions and constants for ship death pipeline, explosion animation,
//! cleanup, flee sequence, warp transitions, and winner determination.
//! This is a type-only module — no orchestration logic.
//!
//! The tactical transition orchestration (ship_death, explosion_preprocess,
//! cleanup_dead_ship, new_ship, flee_preprocess, ship_transition) stays
//! in C for Phase 1.

/// Ship death pipeline phases (4 phases)
///
/// From tactrans.c:
/// 1. ship_death() -> StartShipExplosion
/// 2. explosion_preprocess() -> 36 frames of debris spawning
/// 3. cleanup_dead_ship() -> clear elements, preserve crew objects
/// 4. new_ship() -> spawn replacement ship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeathPipelinePhase {
    /// Ship death initiated, start explosion
    ShipDeath = 0,
    /// Explosion animation in progress
    Explosion = 1,
    /// Cleanup dead ship elements
    Cleanup = 2,
    /// Spawn new ship
    NewShip = 3,
}

/// Explosion animation constants
///
/// From element.h and tactrans.c:
/// - NUM_EXPLOSION_FRAMES = 12
/// - life_span = NUM_EXPLOSION_FRAMES * 3 = 36
/// - Frame 15: hide primitive
/// - Frame 25: clear preprocess
pub const NUM_EXPLOSION_FRAMES: i16 = 12;
pub const EXPLOSION_LIFE: i16 = NUM_EXPLOSION_FRAMES * 3; // 36 frames

/// Frame milestones during explosion
pub const EXPLOSION_HIDE_PRIM_FRAME: u8 = 15;
pub const EXPLOSION_CLEAR_PREPROCESS_FRAME: u8 = 25;

/// Minimum ditty frame count (from tactrans.c)
pub const MIN_DITTY_FRAME_COUNT: i16 = (24 * 3); // (ONE_SECOND * 3) / BATTLE_FRAME_RATE

/// Hyperjump (warp transition) life constant
pub const HYPERJUMP_LIFE: i16 = 15;

/// Ion trail 12-color palette
///
/// From tactrans.c spawn_ion_trail():
/// Color cycle for warp-in ghost images
pub const ION_TRAIL_COLORS: [u8; 12] = [
    0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
];

/// Flee 20-color red pulse palette
///
/// From tactrans.c flee_preprocess():
/// Accelerating red pulse animation during flee sequence
pub const FLEE_PULSE_COLORS: [u8; 20] = [
    0x2E, 0x2D, 0x2C, 0x2B, 0x2A, // Dark red -> bright red
    0x29, 0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x20, 0x1F, 0x1E, 0x1D, 0x1C, 0x1B,
];

/// Flee mass constant (from tactrans.c and battle.c)
///
/// When a ship flees, mass_points is set to MAX_SHIP_MASS * 10
pub const FLEE_MASS: u8 = 100; // MAX_SHIP_MASS (10) * 10

/// Pkunk reincarnation mass constant
///
/// From tactrans.c: Reincarnating Pkunk has mass = MAX_SHIP_MASS + 1
pub const PKUNK_REINCARNATION_MASS: u8 = 11; // MAX_SHIP_MASS + 1

/// Winner determination types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WinnerDeterminationType {
    /// Iterate display list in order
    DisplayListOrder = 0,
    /// Check PLAYER_SHIP flag
    PlayerShipFlag = 1,
    /// Break on first alive ship found
    BreakFirst = 2,
}

/// OpponentAlive return cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpponentAliveResult {
    /// Opponent is alive
    Alive = 0,
    /// Opponent is dead (crew_level == 0)
    Dead = 1,
    /// No opponent found
    NoOpponent = 2,
}

/// Debris spawn rates during explosion (from tactrans.c)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExplosionDebrisRate {
    /// Spawn 1 debris per frame
    One = 1,
    /// Spawn 2 debris per frame
    Two = 2,
    /// Spawn 3 debris per frame
    Three = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_death_pipeline_phase_count() {
        let phases = [
            DeathPipelinePhase::ShipDeath,
            DeathPipelinePhase::Explosion,
            DeathPipelinePhase::Cleanup,
            DeathPipelinePhase::NewShip,
        ];
        assert_eq!(phases.len(), 4);
    }

    #[test]
    fn test_explosion_constants() {
        assert_eq!(NUM_EXPLOSION_FRAMES, 12);
        assert_eq!(EXPLOSION_LIFE, 36); // 12 * 3
        assert_eq!(EXPLOSION_HIDE_PRIM_FRAME, 15);
        assert_eq!(EXPLOSION_CLEAR_PREPROCESS_FRAME, 25);
    }

    #[test]
    fn test_min_ditty_frame_count() {
        assert_eq!(MIN_DITTY_FRAME_COUNT, 72); // (24 * 3)
    }

    #[test]
    fn test_hyperjump_life() {
        assert_eq!(HYPERJUMP_LIFE, 15);
    }

    #[test]
    fn test_ion_trail_colors_length() {
        assert_eq!(ION_TRAIL_COLORS.len(), 12);
    }

    #[test]
    fn test_flee_pulse_colors_length() {
        assert_eq!(FLEE_PULSE_COLORS.len(), 20);
    }

    #[test]
    fn test_flee_mass_constant() {
        assert_eq!(FLEE_MASS, 100); // 10 * 10
    }

    #[test]
    fn test_pkunk_reincarnation_mass() {
        assert_eq!(PKUNK_REINCARNATION_MASS, 11); // MAX_SHIP_MASS + 1
    }

    #[test]
    fn test_winner_determination_variants() {
        assert_eq!(WinnerDeterminationType::DisplayListOrder as u8, 0);
        assert_eq!(WinnerDeterminationType::PlayerShipFlag as u8, 1);
        assert_eq!(WinnerDeterminationType::BreakFirst as u8, 2);
    }

    #[test]
    fn test_opponent_alive_result_variants() {
        assert_eq!(OpponentAliveResult::Alive as u8, 0);
        assert_eq!(OpponentAliveResult::Dead as u8, 1);
        assert_eq!(OpponentAliveResult::NoOpponent as u8, 2);
    }

    #[test]
    fn test_explosion_debris_rates() {
        assert_eq!(ExplosionDebrisRate::One as u8, 1);
        assert_eq!(ExplosionDebrisRate::Two as u8, 2);
        assert_eq!(ExplosionDebrisRate::Three as u8, 3);
    }
}
