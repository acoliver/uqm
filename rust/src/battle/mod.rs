// Battle Engine Subsystem — Phase 1 Rust Port
// @plan PLAN-20260320-BATTLE.P03, P04, P06, P08, P09, P10, P11, P12, P13, P14, P15, P16
// @requirement Foundation module for shared types extracted from ships

pub mod ai_types;
pub mod battle_types;
pub mod collision;
pub mod display_list;
pub mod element;
pub mod ffi;
pub mod integration;
pub mod lifecycle;
pub mod netplay;
pub mod process_types;
pub mod ship_runtime_types;
pub mod tactical;
pub mod velocity;
pub mod weapon;

pub use ai_types::*;
pub use battle_types::*;
pub use collision::*;
pub use display_list::*;
pub use element::*;
pub use integration::*;
pub use lifecycle::*;
pub use netplay::*;
pub use process_types::*;
pub use ship_runtime_types::*;
pub use tactical::*;
pub use velocity::*;
pub use weapon::*;

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

    #[test]
    fn test_velocity_element_collision_chain() {
        // Create two elements with opposing velocities
        let mut elem0 = Element::new();
        let mut elem1 = Element::new();

        // Element 0: moving right (+X) at speed 10
        elem0.velocity.set_components(10, 0);
        elem0.mass_points = 5;
        elem0.current.location = Point::new(100, 100);
        elem0.next.location = Point::new(110, 100);

        // Element 1: moving left (-X) at speed 10
        elem1.velocity.set_components(-10, 0);
        elem1.mass_points = 5;
        elem1.current.location = Point::new(200, 100);
        elem1.next.location = Point::new(190, 100);

        // Calculate initial momentum (total should be 0 since equal and opposite)
        let (dx0_before, dy0_before) = elem0.velocity.get_current_components();
        let (dx1_before, dy1_before) = elem1.velocity.get_current_components();
        let total_momentum_x_before =
            (dx0_before * elem0.mass_points as i32) + (dx1_before * elem1.mass_points as i32);
        let total_momentum_y_before =
            (dy0_before * elem0.mass_points as i32) + (dy1_before * elem1.mass_points as i32);

        // Collide the elements
        elastic_collide(&mut elem0, &mut elem1);

        // Calculate final momentum
        let (dx0_after, dy0_after) = elem0.velocity.get_current_components();
        let (dx1_after, dy1_after) = elem1.velocity.get_current_components();
        let total_momentum_x_after =
            (dx0_after * elem0.mass_points as i32) + (dx1_after * elem1.mass_points as i32);
        let total_momentum_y_after =
            (dy0_after * elem0.mass_points as i32) + (dy1_after * elem1.mass_points as i32);

        // Verify momentum conservation (within tolerance of 1 unit due to integer math)
        assert!(
            (total_momentum_x_before - total_momentum_x_after).abs() <= 1,
            "X momentum not conserved: before={}, after={}",
            total_momentum_x_before,
            total_momentum_x_after
        );
        assert!(
            (total_momentum_y_before - total_momentum_y_after).abs() <= 1,
            "Y momentum not conserved: before={}, after={}",
            total_momentum_y_before,
            total_momentum_y_after
        );

        // Verify velocities changed (elastic collision should reverse velocities for equal mass)
        assert_ne!(
            dx0_before, dx0_after,
            "Element 0 velocity should have changed"
        );
        assert_ne!(
            dx1_before, dx1_after,
            "Element 1 velocity should have changed"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Weapon → Damage → Blast Chain
    // -----------------------------------------------------------------------

    #[test]
    fn test_weapon_damage_blast_chain() {
        // Create weapon element
        let mut weapon = Element::new();
        weapon.current.location = Point::new(100, 100);
        weapon.next.location = Point::new(110, 100);
        weapon.mass_points = 5; // Damage amount
        weapon.crew_or_hp = 5; // Weapon hit points
        weapon.state_flags = ElementFlags::empty();

        // Create target element
        let mut target = Element::new();
        target.current.location = Point::new(115, 100);
        target.next.location = Point::new(115, 100);
        target.mass_points = 10;
        target.crew_or_hp = 20; // Initial HP
        target.life_span = NORMAL_LIFE; // Required for damage application
        target.state_flags = ElementFlags::PLAYER_SHIP; // Make it a player ship

        let initial_hp = target.crew_or_hp;

        // Apply weapon collision (needs collision points)
        let weapon_pt = Point::new(110, 100);
        let target_pt = Point::new(115, 100);
        weapon_collision(&mut weapon, &weapon_pt, &mut target, &target_pt);

        // Verify damage was applied
        assert!(
            target.crew_or_hp < initial_hp,
            "Target HP should decrease: before={}, after={}",
            initial_hp,
            target.crew_or_hp
        );

        // Verify weapon was marked as collided or destroyed
        assert!(
            weapon.state_flags.contains(ElementFlags::COLLISION)
                || weapon.state_flags.contains(ElementFlags::DISAPPEARING),
            "Weapon should be marked as collided or destroyed"
        );

        // Compute blast direction (takes angle from weapon to target)
        let dx = target.current.location.x - weapon.current.location.x;
        let dy = target.current.location.y - weapon.current.location.y;
        let angle = arctan(dx as i32, dy as i32) as u8;
        let blast_dir = compute_blast_direction(angle);
        assert!(
            (blast_dir as u16) < NUM_FACINGS,
            "Blast direction {} should be < NUM_FACINGS ({})",
            blast_dir,
            NUM_FACINGS
        );
    }

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

    #[test]
    fn test_ffi_roundtrip() {
        let mut vel = VelocityDesc::new();
        let magnitude = 100;
        let facing = 8; // 45 degrees
        let direction = 0;

        // Call through FFI
        let result = ffi::rust_velocity_set_vector(&mut vel, magnitude, facing, direction);
        assert_eq!(result, 0, "FFI call should succeed");

        // Verify through direct Rust call
        let mut dx: i32 = 0;
        let mut dy: i32 = 0;
        let result = ffi::rust_velocity_get_current_components(&vel, &mut dx, &mut dy);
        assert_eq!(result, 0, "FFI call should succeed");

        // Compare with direct method call
        let (direct_dx, direct_dy) = vel.get_current_components();
        assert_eq!(
            dx, direct_dx,
            "FFI dx should match direct call: ffi={}, direct={}",
            dx, direct_dx
        );
        assert_eq!(
            dy, direct_dy,
            "FFI dy should match direct call: ffi={}, direct={}",
            dy, direct_dy
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Display List Stress Test
    // -----------------------------------------------------------------------

    #[test]
    fn test_display_list_stress() {
        let capacity = 10;
        let mut list = DisplayList::new(capacity);

        // Allocate to capacity and add to active list
        let mut handles = Vec::new();
        for i in 0..capacity {
            let handle = list.alloc().expect("Should allocate successfully");
            if let Some(elem) = list.get_mut(handle) {
                elem.mass_points = i as u8;
            }
            // Add to active list
            list.push_back(handle);
            handles.push(handle);
        }

        assert_eq!(list.count(), capacity, "Should be at capacity");

        // Verify we can't allocate more
        assert!(
            list.alloc().is_none(),
            "Should fail to allocate beyond capacity"
        );

        // Free all
        for handle in handles.iter() {
            list.free(*handle);
        }

        assert_eq!(list.count(), 0, "All elements should be freed");

        // Reallocate and verify old handles are invalid
        let new_handle = list.alloc().expect("Should allocate after freeing");
        assert!(
            list.get(handles[0]).is_none(),
            "Old handle should be invalidated by generation counter"
        );
        assert!(list.get(new_handle).is_some(), "New handle should be valid");
    }

    // -----------------------------------------------------------------------
    // Test 6: Type Relocation Verification
    // -----------------------------------------------------------------------

    #[test]
    fn test_type_relocation_verification() {
        // Verify constants are accessible from both battle_types and ship_runtime_types
        use super::battle_types::NUM_FACINGS as BT_NUM_FACINGS;
        use super::ship_runtime_types::MAX_CREW_SIZE;

        // NUM_FACINGS should be 16
        assert_eq!(BT_NUM_FACINGS, 16, "NUM_FACINGS should be 16");

        // MAX_CREW_SIZE should be 42
        assert_eq!(MAX_CREW_SIZE, 42, "MAX_CREW_SIZE should be 42");

        // Verify ShipPipelineStage enum exists
        use super::ship_runtime_types::ShipPipelineStage;
        let stage = ShipPipelineStage::Input;
        assert_eq!(stage as u8, 0, "ShipPipelineStage::Input should be stage 0");

        // Verify SpawnPositionType enum exists
        use super::ship_runtime_types::SpawnPositionType;
        let spawn_type = SpawnPositionType::Random;
        assert_eq!(spawn_type as u8, 0, "SpawnPositionType::Random should be 0");
    }

    // -----------------------------------------------------------------------
    // Test 7: Full Battle Types Instantiation
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_battle_types_instantiation() {
        // Element
        let _elem = Element::new();

        // VelocityDesc
        let _vel = VelocityDesc::new();

        // DisplayList
        let _list = DisplayList::new(10);

        // CrcState
        let _crc = CrcState::new();

        // LaserBlock
        let _laser = LaserBlock::new(Point::new(0, 0), Point::new(10, 10), 0, 0, 0);

        // MissileBlock
        let _missile = MissileBlock::new(Point::new(0, 0), 0, 0, 0, 0, 0, 0, 0, 0, 0);

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

    #[test]
    fn test_collision_with_gravity_mass() {
        use super::collision::is_gravity_mass;

        // Normal mass
        assert!(!is_gravity_mass(50), "Mass 50 should not be gravity mass");

        // Gravity mass threshold (100)
        assert!(is_gravity_mass(100), "Mass 100 should be gravity mass");
        assert!(is_gravity_mass(150), "Mass 150 should be gravity mass");

        // Create collision with gravity mass
        let mut light_elem = Element::new();
        light_elem.mass_points = 5;
        light_elem.velocity.set_components(10, 0);
        light_elem.current.location = Point::new(100, 100);

        let mut heavy_elem = Element::new();
        heavy_elem.mass_points = 100; // Gravity mass
        heavy_elem.velocity.set_components(0, 0);
        heavy_elem.current.location = Point::new(110, 100);

        // Collide
        elastic_collide(&mut light_elem, &mut heavy_elem);

        // Gravity mass should not move
        let (heavy_dx, heavy_dy) = heavy_elem.velocity.get_current_components();
        assert_eq!(
            heavy_dx, 0,
            "Gravity mass should not have X velocity after collision"
        );
        assert_eq!(
            heavy_dy, 0,
            "Gravity mass should not have Y velocity after collision"
        );

        // Light mass should have changed velocity
        let (light_dx, _light_dy) = light_elem.velocity.get_current_components();
        assert_ne!(
            light_dx, 10,
            "Light mass velocity should have changed after collision with gravity mass"
        );
    }

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

    #[test]
    fn test_weapon_tracking() {
        let weapon_pos = Point::new(100, 100);
        let target_pos = Point::new(100, 200); // Target to the south
        let current_facing = 0; // Currently facing north (UQM: 0=North)

        // Compute tracking facing (should turn one step toward target)
        let facing = compute_track_facing(weapon_pos, target_pos, current_facing);

        // Target is south (facing 8), current is north (facing 0)
        // Distance is 8 steps clockwise or 8 steps counter-clockwise (half circle)
        // In this case, function turns one step in the deterministic direction (right/clockwise)
        // So facing should be 1 (one step clockwise from 0)
        assert_eq!(
            facing, 1,
            "Should turn one step clockwise from north toward south, got {}",
            facing
        );
    }
}
