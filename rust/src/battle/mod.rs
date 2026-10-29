// Battle Engine Subsystem — Phase 1 Rust Port
// @plan PLAN-20260320-BATTLE.P03, P04, P06, P08, P09, P10, P11, P12, P13, P14, P15, P16
// @requirement Foundation module for shared types extracted from ships

pub mod ai;
pub mod ai_entry;
pub mod battle_controls;
pub mod battle_native;
pub mod battle_types;
pub mod build_native;
pub mod c_bridge;
pub(crate) mod context_access;
pub mod cyborg_native;
pub mod element;
pub mod encount_native;
pub mod ffi;
pub mod galaxy;
pub mod gendef;
pub mod gravity;
mod integration_tests_p14;
pub mod lifecycle;
pub mod netplay;
pub mod process_native;
pub mod ship_native;
pub mod sounds;
pub mod status_native;
pub mod tactrans_native;
pub mod velocity;
pub mod weapon_native;
pub mod world_elements;

pub use ai::*;
pub use battle_types::*;
pub use element::*;
pub use lifecycle::*;
pub use netplay::*;
pub use velocity::*;

// Explicit re-exports to resolve ambiguous glob re-exports. These duplicate
// definitions exist across modules (some intentionally for type-layered
// migration); the canonical source is named explicitly here so the glob
// re-exports above remain unambiguous.
pub use battle_types::{BATTLE_FRAME_RATE, MAX_REDUCTION, MAX_VIS_REDUCTION};
pub use element::{IntersectControl, MAX_CREW_SIZE, MAX_ENERGY_SIZE};
pub use lifecycle::BattleInputState;

// ---------------------------------------------------------------------------
// E2E Integration Tests — Phase 1 Battle Engine
// @plan PLAN-20260320-BATTLE.P18
// ---------------------------------------------------------------------------

#[cfg(test)]
mod integration_tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Test 1: Velocity → Element → Collision Chain
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Test 2: Weapon → Damage → Blast Chain
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Test 3: CRC Consistency
    // -----------------------------------------------------------------------

    #[test]
    fn test_crc_consistency() {
        // Create identical elements
        let elem1 = Element::new();
        let mut elem2 = Element::new();
        // CRC both (should be identical for identical elements)
        let mut crc1 = CrcState::new();
        crc1.process_element(&elem1);
        let hash1 = crc1.finish();
        let mut crc2 = CrcState::new();
        crc2.process_element(&elem2);
        let hash2 = crc2.finish();
        assert_eq!(hash1, hash2, "Identical elements should have identical CRC");
        // Modify one field
        elem2.mass_points = 42;
        // CRC again
        let mut crc3 = CrcState::new();
        crc3.process_element(&elem2);
        let hash3 = crc3.finish();
        assert_ne!(hash1, hash3, "Modified element should have different CRC");
    }

    // -----------------------------------------------------------------------
    // Test 4: FFI Round-Trip
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Test 6: Type Relocation Verification
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Test 7: Full Battle Types Instantiation
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_battle_types_instantiation() {
        // Element
        let _elem = Element::new();
        // VelocityDesc
        let _vel = VelocityDesc::new();
        // CrcState
        let _crc = CrcState::new();
        // Extent
        let _extent = Extent::new(10, 20);
        // Point
        let _point = Point::new(5, 10);
        // ElementFlags
        let _flags = ElementFlags::PLAYER_SHIP | ElementFlags::COLLISION;
        // ElementVisualState
        let _state = ElementVisualState::new();
        // IntersectControl (use element version)
        let _intersect = element::IntersectControl {
            last_time_val: 0,
            intersect_stamp: Stamp {
                origin: Point::new(0, 0),
                frame: std::ptr::null_mut(),
            },
            end_point: Point::new(10, 10),
        };
        // All types constructed successfully (no panic)
    }

    // -----------------------------------------------------------------------
    // Test 8: Velocity Zero Detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_velocity_zero_detection() {
        let mut vel = VelocityDesc::new();
        assert!(vel.is_zero(), "New velocity should be zero");
        vel.set_components(10, 20);
        assert!(!vel.is_zero(), "Non-zero velocity should not be zero");
        vel.zero();
        assert!(vel.is_zero(), "Zeroed velocity should be zero");
    }

    // -----------------------------------------------------------------------
    // Test 9: Element Lifecycle States
    // -----------------------------------------------------------------------

    #[test]
    fn test_element_lifecycle_states() {
        let mut elem = Element::new();
        // Initially no lifecycle flags
        assert!(
            !elem.state_flags.contains(ElementFlags::APPEARING),
            "New element should not be APPEARING"
        );
        assert!(
            !elem.state_flags.contains(ElementFlags::DISAPPEARING),
            "New element should not be DISAPPEARING"
        );
        // Set APPEARING
        elem.state_flags.insert(ElementFlags::APPEARING);
        assert!(
            elem.state_flags.contains(ElementFlags::APPEARING),
            "Should have APPEARING flag"
        );
        // Transition to DISAPPEARING
        elem.state_flags.remove(ElementFlags::APPEARING);
        elem.state_flags.insert(ElementFlags::DISAPPEARING);
        assert!(
            !elem.state_flags.contains(ElementFlags::APPEARING),
            "Should not have APPEARING flag"
        );
        assert!(
            elem.state_flags.contains(ElementFlags::DISAPPEARING),
            "Should have DISAPPEARING flag"
        );
        // Set life_span for FINITE_LIFE
        elem.life_span = 100;
        elem.state_flags.insert(ElementFlags::FINITE_LIFE);
        assert!(
            elem.state_flags.contains(ElementFlags::FINITE_LIFE),
            "Should have FINITE_LIFE flag"
        );
        assert_eq!(elem.life_span, 100, "Life span should be 100");
    }

    // -----------------------------------------------------------------------
    // Test 10: Collision Detection with Gravity Mass
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Test 11: Battle Types Trigonometry Functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_battle_types_trig_functions() {
        // Test arctan (UQM coordinate system: 0=North(-Y), 16=East(+X), 32=South(+Y), 48=West(-X))
        let angle = arctan(100, 0);
        assert_eq!(angle, 16, "arctan(100, 0) should be 16 (east, +X)");
        let angle = arctan(0, 100);
        assert_eq!(angle, 32, "arctan(0, 100) should be 32 (south, +Y)");
        let angle = arctan(0, -100);
        assert_eq!(angle, 0, "arctan(0, -100) should be 0 (north, -Y)");
        // Test sine/cosine (both take angle and magnitude)
        // angle 0 = North = -Y, so sine should be negative
        let sin_val = sine(0, 100);
        assert!(sin_val < 0, "sine(0, 100) should be negative (north is -Y)");
        // angle 16 = East = +X, cosine(0) = sine(0+16) should be 0
        let cos_val = cosine(0, 100);
        assert_eq!(cos_val, 0, "cosine(0, 100) should be 0");
        // angle 16 = East, sine should be 0
        let sin_val = sine(16, 100);
        assert_eq!(sin_val, 0, "sine(16, 100) should be 0 (east)");
        // Test angle normalization
        let normalized = normalize_angle(64);
        assert_eq!(normalized, 0, "64 should normalize to 0");
        let normalized = normalize_angle(65);
        assert_eq!(normalized, 1, "65 should normalize to 1");
        // Test facing normalization
        let facing = normalize_facing(16);
        assert_eq!(facing, 0, "16 should normalize to 0");
        let facing = normalize_facing(17);
        assert_eq!(facing, 1, "17 should normalize to 1");
    }

    // -----------------------------------------------------------------------
    // Test 12: Weapon Tracking
    // -----------------------------------------------------------------------
}
