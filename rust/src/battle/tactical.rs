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

// ---------------------------------------------------------------------------
// P09: Tactical Transitions — Death + Explosion (tactrans.c)
// @plan PLAN-20260320-BATTLEPT2.P09
// @requirement REQ-DEATH-CHAIN, REQ-EXPLOSION, REQ-CLEANUP, REQ-REPLACEMENT,
//              REQ-SIMULTANEOUS, REQ-DITTY, REQ-ION-TRAIL
// ---------------------------------------------------------------------------

use super::element::{Element, ElementFlags, NORMAL_LIFE};

/// Explosion fragment spawn count per tick, derived from C switch schedule
/// (tactrans.c:548-575). Returns number of fragments to spawn for each tick.
///
/// Tick = (NUM_EXPLOSION_FRAMES * 3) - life_span
pub fn explosion_fragment_count(tick: u8) -> (u8, bool, bool) {
    // Returns: (fragment_count, hide_ship_prim, clear_preprocess)
    match tick {
        25 => (1, false, true), // preprocess_func = NULL, falls through to i=1
        0 | 1 | 2 | 20 | 21 | 22 | 23 | 24 => (1, false, false),
        3 | 4 | 5 | 18 | 19 => (2, false, false),
        15 => (3, true, false), // SetPrimType(NO_PRIM), CHANGING, falls through to i=3
        _ => (3, false, false), // default
    }
}

/// Compute the multi-step life_span for cleanup_dead_ship.
///
/// C reference: tactrans.c:358-371
///   life_span = MusicStarted ? MIN_DITTY_FRAME_COUNT : 1
///   if winner == dead_ship: life_span = MIN_DITTY_FRAME_COUNT + 1
///   ++life_span (unconditional)
pub fn compute_cleanup_life_span(music_started: bool, is_winner: bool) -> i16 {
    let mut life_span = if music_started {
        MIN_DITTY_FRAME_COUNT
    } else {
        1
    };
    if is_winner {
        life_span = MIN_DITTY_FRAME_COUNT + 1;
    }
    life_span += 1; // unconditional increment (preserves original framecount)
    life_span
}

/// Initialize explosion state on a ship element.
///
/// C reference: tactrans.c:702-727 StartShipExplosion
///
/// Sets life_span to EXPLOSION_LIFE (36), clears DISAPPEARING, sets
/// FINITE_LIFE|NONSOLID, zeros velocity, assigns explosion callbacks.
pub fn start_ship_explosion_state(element: &mut Element) {
    element.life_span = EXPLOSION_LIFE as u16;
    element.state_flags.remove(ElementFlags::DISAPPEARING);
    element
        .state_flags
        .insert(ElementFlags::FINITE_LIFE | ElementFlags::NONSOLID);
    element.velocity = super::velocity::VelocityDesc::default();
    // Callbacks assigned by caller (requires function pointer types from C bridge)
}

/// Check if a ship death record should decrement battle_counter.
///
/// C reference: tactrans.c:690-696
///
/// Returns false if the ship is fleeing (mass_points > MAX_SHIP_MASS),
/// because flee-ships are already counted in DoRunAway.
pub fn should_decrement_battle_counter(mass_points: u8) -> bool {
    mass_points <= super::element::MAX_SHIP_MASS
}

/// Set minimum life_span on a ship element that has finished exploding.
///
/// C reference: tactrans.c:376-386 setMinShipLifeSpan
///
/// Only applies if death_func == new_ship (element is in post-explosion state)
/// and element has FINITE_LIFE and not DISAPPEARING.
pub fn set_min_life_span(element: &mut Element, min_life: u16) {
    // In C, the check is `death_func == new_ship` which we can't check directly.
    // Caller must verify death phase. We just enforce the minimum.
    if element.state_flags.contains(ElementFlags::FINITE_LIFE)
        && !element.state_flags.contains(ElementFlags::DISAPPEARING)
        && element.life_span < min_life
    {
        element.life_span = min_life;
    }
}

/// Mark a dead ship's owned elements for deletion.
///
/// C reference: tactrans.c:307-336 (inside cleanup_dead_ship loop)
///
/// Sets element to: NO_PRIM display, life_span=0,
/// NONSOLID|DISAPPEARING|FINITE_LIFE, all callbacks zeroed.
pub fn mark_element_for_deletion(element: &mut Element) {
    element.life_span = 0;
    element.state_flags =
        ElementFlags::NONSOLID | ElementFlags::DISAPPEARING | ElementFlags::FINITE_LIFE;
    element.preprocess_func = None;
    element.postprocess_func = None;
    element.death_func = None;
    element.collision_func = None;
}

/// Ion trail color cycle table.
///
/// C reference: tactrans.c:758-769 colorTab[]
///
/// 12 colors cycling from bright orange through yellow to dark.
pub const ION_TRAIL_COLOR_TABLE: [(u8, u8, u8); 12] = [
    (0x1F, 0x15, 0x00), // START_ION_COLOR
    (0x1F, 0x11, 0x00),
    (0x1F, 0x0E, 0x00),
    (0x1F, 0x0A, 0x00),
    (0x1F, 0x07, 0x00),
    (0x1F, 0x03, 0x00),
    (0x1F, 0x00, 0x00),
    (0x1B, 0x00, 0x00),
    (0x17, 0x00, 0x00),
    (0x13, 0x00, 0x00),
    (0x10, 0x00, 0x00),
    (0x0C, 0x00, 0x00),
];

/// Number of ion trail color steps
pub const ION_TRAIL_LIFE: i16 = ION_TRAIL_COLOR_TABLE.len() as i16;

/// Advance ion trail color cycle and determine if trail should disappear.
///
/// C reference: tactrans.c:755-796 cycle_ion_trail
///
/// Returns the next color index, or None if trail has expired.
pub fn advance_ion_trail(color_cycle_index: u8) -> Option<u8> {
    let next = color_cycle_index + 1;
    if (next as usize) < ION_TRAIL_COLOR_TABLE.len() {
        Some(next)
    } else {
        None // Trail expired
    }
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

    // ---- P09: Death + Explosion tests ----

    #[test]
    fn test_explosion_fragment_schedule() {
        // Tick 0,1,2: 1 fragment
        assert_eq!(explosion_fragment_count(0), (1, false, false));
        assert_eq!(explosion_fragment_count(1), (1, false, false));
        assert_eq!(explosion_fragment_count(2), (1, false, false));
        // Tick 3,4,5: 2 fragments
        assert_eq!(explosion_fragment_count(3), (2, false, false));
        assert_eq!(explosion_fragment_count(5), (2, false, false));
        // Tick 15: 3 fragments + hide prim
        assert_eq!(explosion_fragment_count(15), (3, true, false));
        // Tick 18,19: 2 fragments
        assert_eq!(explosion_fragment_count(18), (2, false, false));
        // Tick 20-24: 1 fragment
        assert_eq!(explosion_fragment_count(20), (1, false, false));
        assert_eq!(explosion_fragment_count(24), (1, false, false));
        // Tick 25: 1 fragment + clear preprocess
        assert_eq!(explosion_fragment_count(25), (1, false, true));
        // Default (e.g., tick 10): 3 fragments
        assert_eq!(explosion_fragment_count(10), (3, false, false));
    }

    #[test]
    fn test_cleanup_life_span_no_music_not_winner() {
        // No music, not winner: 1 + 1 = 2
        assert_eq!(compute_cleanup_life_span(false, false), 2);
    }

    #[test]
    fn test_cleanup_life_span_music_not_winner() {
        // Music started, not winner: MIN_DITTY_FRAME_COUNT + 1
        assert_eq!(
            compute_cleanup_life_span(true, false),
            MIN_DITTY_FRAME_COUNT + 1
        );
    }

    #[test]
    fn test_cleanup_life_span_winner() {
        // Winner (regardless of music): MIN_DITTY_FRAME_COUNT + 1 + 1
        assert_eq!(
            compute_cleanup_life_span(true, true),
            MIN_DITTY_FRAME_COUNT + 2
        );
        assert_eq!(
            compute_cleanup_life_span(false, true),
            MIN_DITTY_FRAME_COUNT + 2
        );
    }

    #[test]
    fn test_start_ship_explosion_state() {
        let mut elem = Element::default();
        elem.state_flags = ElementFlags::DISAPPEARING | ElementFlags::PLAYER_SHIP;
        start_ship_explosion_state(&mut elem);
        assert_eq!(elem.life_span, EXPLOSION_LIFE as u16);
        assert!(!elem.state_flags.contains(ElementFlags::DISAPPEARING));
        assert!(elem.state_flags.contains(ElementFlags::FINITE_LIFE));
        assert!(elem.state_flags.contains(ElementFlags::NONSOLID));
        assert!(elem.state_flags.contains(ElementFlags::PLAYER_SHIP)); // preserved
    }

    #[test]
    fn test_should_decrement_battle_counter() {
        assert!(should_decrement_battle_counter(5)); // normal ship
        assert!(should_decrement_battle_counter(10)); // MAX_SHIP_MASS
        assert!(!should_decrement_battle_counter(11)); // fleeing
        assert!(!should_decrement_battle_counter(100)); // FLEE_MASS
    }

    #[test]
    fn test_set_min_life_span() {
        let mut elem = Element::default();
        elem.state_flags = ElementFlags::FINITE_LIFE;
        elem.life_span = 5;
        set_min_life_span(&mut elem, 10);
        assert_eq!(elem.life_span, 10);
        // Already above minimum — no change
        set_min_life_span(&mut elem, 3);
        assert_eq!(elem.life_span, 10);
    }

    #[test]
    fn test_set_min_life_span_disappearing_noop() {
        let mut elem = Element::default();
        elem.state_flags = ElementFlags::FINITE_LIFE | ElementFlags::DISAPPEARING;
        elem.life_span = 1;
        set_min_life_span(&mut elem, 10);
        assert_eq!(elem.life_span, 1, "DISAPPEARING elements not adjusted");
    }

    #[test]
    fn test_mark_element_for_deletion() {
        let mut elem = Element::default();
        elem.life_span = 50;
        elem.state_flags = ElementFlags::PLAYER_SHIP;
        mark_element_for_deletion(&mut elem);
        assert_eq!(elem.life_span, 0);
        assert!(elem.state_flags.contains(ElementFlags::NONSOLID));
        assert!(elem.state_flags.contains(ElementFlags::DISAPPEARING));
        assert!(elem.state_flags.contains(ElementFlags::FINITE_LIFE));
        assert!(!elem.state_flags.contains(ElementFlags::PLAYER_SHIP));
        assert!(elem.preprocess_func.is_none());
        assert!(elem.postprocess_func.is_none());
        assert!(elem.death_func.is_none());
        assert!(elem.collision_func.is_none());
    }

    #[test]
    fn test_ion_trail_advance() {
        assert_eq!(advance_ion_trail(0), Some(1));
        assert_eq!(advance_ion_trail(10), Some(11));
        assert_eq!(advance_ion_trail(11), None); // 12 colors, index 11 is last
    }

    #[test]
    fn test_ion_trail_color_table_length() {
        assert_eq!(ION_TRAIL_COLOR_TABLE.len(), 12);
        assert_eq!(ION_TRAIL_LIFE, 12);
    }
}
