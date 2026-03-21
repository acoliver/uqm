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
#[derive(Debug, Clone, Copy)]
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

impl Default for EvaluateDesc {
    fn default() -> Self {
        Self {
            which_turn: 0,
            facing: 0,
            distance: 0,
            object_handle: 0,
        }
    }
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
}
