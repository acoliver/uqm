//! AI Dispatch Types (P14)
//!
//! Type definitions and constants for AI dispatch, evaluate descriptors,
//! control flags, AI range constants, and object tracking.
//! This is a type-only module — no orchestration logic.
//!
//! The AI dispatch orchestration (computer_intelligence, tactical_intelligence,
//! ship_intelligence) stays in C for Phase 1.

/// AI range constants (from intel.h)
///
/// Used to classify weapon ranges for AI decision-making
pub const CLOSE_RANGE_WEAPON: i32 = 50 << 2; // DISPLAY_TO_WORLD(50)
pub const LONG_RANGE_WEAPON: i32 = 1000 << 2; // DISPLAY_TO_WORLD(1000)

/// AI ship speed classification constants (from intel.h)
///
/// Used for maneuverability assessment
pub const FAST_SHIP: i32 = 150;
pub const MEDIUM_SHIP: i32 = 45;
pub const SLOW_SHIP: i32 = 25;

/// Object tracking indices (from intel.h)
///
/// Array indices for AI concern tracking system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectTrackingIndex {
    /// Enemy ship tracking index
    EnemyShip = 0,
    /// Crew object tracking index
    CrewObject = 1,
    /// Enemy weapon tracking index
    EnemyWeapon = 2,
    /// Gravity mass tracking index
    GravityMass = 3,
    /// First empty slot index
    FirstEmpty = 4,
}

/// Control flags (from intel.h)
///
/// Bitflags for player control types
pub const HUMAN_CONTROL: u8 = 1 << 0;
pub const CYBORG_CONTROL: u8 = 1 << 1; // Computer fights battles
pub const PSYTRON_CONTROL: u8 = 1 << 2; // Computer selects ships
pub const NETWORK_CONTROL: u8 = 1 << 3;
pub const COMPUTER_CONTROL: u8 = CYBORG_CONTROL | PSYTRON_CONTROL;
pub const CONTROL_MASK: u8 = HUMAN_CONTROL | COMPUTER_CONTROL | NETWORK_CONTROL;

/// AI difficulty rating flags
pub const STANDARD_RATING: u8 = 1 << 4;
pub const GOOD_RATING: u8 = 1 << 5;
pub const AWESOME_RATING: u8 = 1 << 6;

/// EvaluateDesc type placeholder
///
/// This type is used by the AI system to track objects of concern.
/// The full struct definition is in C (races.h). This placeholder
/// documents its existence for Phase 1 type definitions.
///
/// Fields (from C):
/// - which_turn: COUNT
/// - facing: COUNT  
/// - object_ptr: *const ELEMENT
/// - ObjectPtr: HELEMENT
#[derive(Debug, Clone, Copy, Default)]
pub struct EvaluateDesc {
    /// Turn counter for this evaluation
    pub which_turn: u16,
    /// Facing direction
    pub facing: u16,
    /// Distance to object (added for Rust clarity)
    pub distance: i32,
    /// Object handle (opaque in Phase 1)
    pub object_handle: usize,
}

// ---------------------------------------------------------------------------
// P11: AI Dispatch (intel.c)
// @plan PLAN-20260320-BATTLEPT2.P11
// @requirement REQ-AI-DISPATCH, REQ-AI-INPUT
// ---------------------------------------------------------------------------

/// AI dispatch path selection.
///
/// C reference: intel.c computer_intelligence
///
/// The four dispatch paths that the AI evaluates:
/// 1. Standard combat — evaluate target and engage
/// 2. Special weapon — race-specific ability use
/// 3. Flee consideration — low health retreat
/// 4. Missile evasion — incoming projectile dodge
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AiDispatchPath {
    /// Standard combat evaluation — turn/thrust/fire
    StandardCombat = 0,
    /// Special weapon handling — race-specific
    SpecialWeapon = 1,
    /// Flee consideration — health check
    FleeConsideration = 2,
    /// Missile evasion — dodge incoming projectiles
    MissileEvasion = 3,
}

/// Result of AI dispatch — input flags for the ship.
///
/// These flags are written to cur_status_flags and consumed by ship_preprocess.
#[derive(Debug, Clone, Copy, Default)]
pub struct AiInput {
    pub turn_left: bool,
    pub turn_right: bool,
    pub thrust: bool,
    pub fire_weapon: bool,
    pub fire_special: bool,
}

impl AiInput {
    /// Convert to status flags bitmask matching C ship input constants.
    pub fn to_status_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.turn_left {
            flags |= super::ship_runtime::LEFT;
        }
        if self.turn_right {
            flags |= super::ship_runtime::RIGHT;
        }
        if self.thrust {
            flags |= super::ship_runtime::THRUST;
        }
        if self.fire_weapon {
            flags |= super::ship_runtime::WEAPON;
        }
        if self.fire_special {
            flags |= super::ship_runtime::SPECIAL;
        }
        flags
    }
}

/// Select which AI dispatch path should be taken.
///
/// C reference: intel.c computer_intelligence dispatch selection
///
/// Evaluates ship state to determine which of the 4 paths to take.
/// In C, this involves checking:
/// - Is there an incoming missile? → MissileEvasion
/// - Is health low enough to flee? → FleeConsideration
/// - Is special weapon available and useful? → SpecialWeapon
/// - Default → StandardCombat
pub fn select_dispatch_path(
    has_incoming_missile: bool,
    health_fraction_low: bool,
    special_available: bool,
) -> AiDispatchPath {
    if has_incoming_missile {
        AiDispatchPath::MissileEvasion
    } else if health_fraction_low {
        AiDispatchPath::FleeConsideration
    } else if special_available {
        AiDispatchPath::SpecialWeapon
    } else {
        AiDispatchPath::StandardCombat
    }
}

/// Compute the desired turn direction to face a target.
///
/// C reference: intel.c ship_intelligence turn computation
///
/// Returns: negative = turn left, positive = turn right, 0 = on target
pub fn compute_turn_direction(current_facing: u16, desired_facing: u16, num_facings: u16) -> i16 {
    let diff = (desired_facing as i32) - (current_facing as i32);
    let half = (num_facings / 2) as i32;

    if diff == 0 || diff.unsigned_abs() as u16 == num_facings {
        0
    } else if diff > 0 {
        if diff <= half {
            1 // turn right (clockwise)
        } else {
            -1 // turn left (shorter path)
        }
    } else {
        // diff < 0
        if (-diff) <= half {
            -1 // turn left (counter-clockwise)
        } else {
            1 // turn right (shorter path)
        }
    }
}

/// Check if an object is within weapon range.
///
/// C reference: intel.c range checks throughout
pub fn in_weapon_range(distance: i32, weapon_range: i32) -> bool {
    distance <= weapon_range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_range_constants() {
        assert_eq!(CLOSE_RANGE_WEAPON, 200); // 50 << 2
        assert_eq!(LONG_RANGE_WEAPON, 4000); // 1000 << 2
    }

    #[test]
    fn test_ai_ship_speed_constants() {
        assert_eq!(FAST_SHIP, 150);
        assert_eq!(MEDIUM_SHIP, 45);
        assert_eq!(SLOW_SHIP, 25);
    }

    #[test]
    fn test_object_tracking_indices() {
        assert_eq!(ObjectTrackingIndex::EnemyShip as u8, 0);
        assert_eq!(ObjectTrackingIndex::CrewObject as u8, 1);
        assert_eq!(ObjectTrackingIndex::EnemyWeapon as u8, 2);
        assert_eq!(ObjectTrackingIndex::GravityMass as u8, 3);
        assert_eq!(ObjectTrackingIndex::FirstEmpty as u8, 4);
    }

    #[test]
    fn test_control_flags() {
        assert_eq!(HUMAN_CONTROL, 1);
        assert_eq!(CYBORG_CONTROL, 2);
        assert_eq!(PSYTRON_CONTROL, 4);
        assert_eq!(NETWORK_CONTROL, 8);
        assert_eq!(COMPUTER_CONTROL, 6); // CYBORG | PSYTRON
        assert_eq!(CONTROL_MASK, 15); // HUMAN | COMPUTER | NETWORK
    }

    #[test]
    fn test_difficulty_rating_flags() {
        assert_eq!(STANDARD_RATING, 16); // 1 << 4
        assert_eq!(GOOD_RATING, 32); // 1 << 5
        assert_eq!(AWESOME_RATING, 64); // 1 << 6
    }

    #[test]
    fn test_evaluate_desc_default() {
        let desc = EvaluateDesc::default();
        assert_eq!(desc.which_turn, 0);
        assert_eq!(desc.facing, 0);
        assert_eq!(desc.distance, 0);
        assert_eq!(desc.object_handle, 0);
    }

    // ---- P11: AI Dispatch tests ----

    #[test]
    fn test_dispatch_path_priority() {
        // Missile evasion takes highest priority
        assert_eq!(
            select_dispatch_path(true, true, true),
            AiDispatchPath::MissileEvasion
        );
        // Flee is next
        assert_eq!(
            select_dispatch_path(false, true, true),
            AiDispatchPath::FleeConsideration
        );
        // Special weapon next
        assert_eq!(
            select_dispatch_path(false, false, true),
            AiDispatchPath::SpecialWeapon
        );
        // Default: standard combat
        assert_eq!(
            select_dispatch_path(false, false, false),
            AiDispatchPath::StandardCombat
        );
    }

    #[test]
    fn test_compute_turn_direction() {
        // Facing target: no turn
        assert_eq!(compute_turn_direction(4, 4, 16), 0);
        // Target slightly right
        assert_eq!(compute_turn_direction(4, 6, 16), 1);
        // Target slightly left
        assert_eq!(compute_turn_direction(6, 4, 16), -1);
        // Target across 180° (ambiguous) — arbitrary choice, turn right
        assert_eq!(compute_turn_direction(0, 8, 16), 1); // exactly half = arbitrary
                                                         // Wrap-around: facing 14, target 2 (16 facings) — shorter to turn right
        assert_eq!(compute_turn_direction(14, 2, 16), 1);
        // Wrap-around: facing 2, target 14 — shorter to turn left
        assert_eq!(compute_turn_direction(2, 14, 16), -1);
    }

    #[test]
    fn test_in_weapon_range() {
        assert!(in_weapon_range(100, CLOSE_RANGE_WEAPON));
        assert!(!in_weapon_range(CLOSE_RANGE_WEAPON + 1, CLOSE_RANGE_WEAPON));
        assert!(in_weapon_range(500, LONG_RANGE_WEAPON));
    }

    #[test]
    fn test_ai_input_to_flags() {
        let input = AiInput {
            turn_left: true,
            thrust: true,
            fire_weapon: true,
            ..Default::default()
        };
        let flags = input.to_status_flags();
        use super::super::ship_runtime::{LEFT, RIGHT, SPECIAL, THRUST, WEAPON};
        assert!(flags & (LEFT as u32) != 0);
        assert!(flags & (THRUST as u32) != 0);
        assert!(flags & (WEAPON as u32) != 0);
        assert!(flags & (RIGHT as u32) == 0);
        assert!(flags & (SPECIAL as u32) == 0);
    }

    #[test]
    fn test_ai_dispatch_path_values() {
        assert_eq!(AiDispatchPath::StandardCombat as u8, 0);
        assert_eq!(AiDispatchPath::SpecialWeapon as u8, 1);
        assert_eq!(AiDispatchPath::FleeConsideration as u8, 2);
        assert_eq!(AiDispatchPath::MissileEvasion as u8, 3);
    }
}
