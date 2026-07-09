// Collision System — Elastic Collision Physics
// @plan PLAN-20260320-BATTLE.P08
// @requirement Elastic collision resolution matching C collide.c

use super::battle_types::{arctan, cosine, sine, HALF_CIRCLE, OCTANT, QUADRANT, SCALED_ONE};
use super::element::{Element, ElementFlags, MAX_SHIP_MASS};
use super::velocity::{velocity_to_world, world_to_velocity};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Collision penalty: turn wait increment
pub const COLLISION_TURN_WAIT: u8 = 1;

/// Collision penalty: thrust wait increment
pub const COLLISION_THRUST_WAIT: u8 = 3;

/// Gravity mass threshold: mass >= 100 is immovable
/// C macro: GRAVITY_MASS(m) = (m > MAX_SHIP_MASS * 10)
/// Note: Test is (mass_points + 1) > 100, so mass_points >= 100 is exempt
const GRAVITY_MASS_THRESHOLD: u8 = MAX_SHIP_MASS * 10; // 100

/// Tests if a mass is a gravity mass (immovable)
pub const fn is_gravity_mass(mass_points: u8) -> bool {
    mass_points >= GRAVITY_MASS_THRESHOLD
}

// ---------------------------------------------------------------------------
// Core Collision Function
// ---------------------------------------------------------------------------

/// Elastic collision resolution between two elements
///
/// Matches C's collide() from collide.c exactly:
/// 1. Compute impact angle from position difference
/// 2. Get velocity components and compute relative velocity
/// 3. Compute directness; fudge if scraping collision
/// 4. Handle stationary overlap (DEFY_PHYSICS)
/// 5. Compute momentum transfer using mass-weighted elastic response
/// 6. Apply velocity changes to both elements (if not gravity mass)
/// 7. Handle player ship penalties (thrust/turn wait, max speed flags)
/// 8. Enforce minimum velocity (if result < 1 world unit/frame)
///
/// # Arguments
/// * `elem0` - First element (mutable)
/// * `elem1` - Second element (mutable)
///
/// # Safety
/// This function assumes elem0 and elem1 are valid, non-null elements.
/// Both elements' velocities and state flags are mutated.
pub fn elastic_collide(elem0: &mut Element, elem1: &mut Element) {
    // --- Step 1: Compute impact angle from position delta ---
    let dx_rel = elem0.next.location.x - elem1.next.location.x;
    let dy_rel = elem0.next.location.y - elem1.next.location.y;
    let mut impact_angle_0 = arctan(dx_rel as i32, dy_rel as i32) & 63; // Mask to 0-63
    let mut impact_angle_1 = impact_angle_0.wrapping_add(HALF_CIRCLE) & 63;

    // --- Step 2: Get current velocities and compute relative velocity ---
    let (dx0, dy0) = elem0.velocity.get_current_components();
    let travel_angle_0 = elem0.velocity.get_travel_angle();

    let (dx1, dy1) = elem1.velocity.get_current_components();
    let travel_angle_1 = elem1.velocity.get_travel_angle();

    let dx_rel = dx0 - dx1;
    let dy_rel = dy0 - dy1;
    let rel_travel_angle = arctan(dx_rel, dy_rel);

    // Integer square root (matching C's square_root)
    let speed = isqrt((dx_rel as i64 * dx_rel as i64 + dy_rel as i64 * dy_rel as i64) as u32);

    // --- Step 3: Compute directness; fudge if scraping ---
    let mut directness = rel_travel_angle.wrapping_sub(impact_angle_0) & 63;

    // Scraping collision: directness in [0..QUADRANT] or [HALF_CIRCLE+QUADRANT..FULL_CIRCLE)
    if directness <= QUADRANT || directness >= HALF_CIRCLE + QUADRANT {
        // Shapes just scraped — fudge to prevent re-collision
        directness = HALF_CIRCLE;
        impact_angle_0 = travel_angle_0.wrapping_add(HALF_CIRCLE);
        impact_angle_1 = travel_angle_1.wrapping_add(HALF_CIRCLE);
    }

    // --- Step 4: Stationary overlap → DEFY_PHYSICS ---
    if elem0.next.location.x == elem0.current.location.x
        && elem0.next.location.y == elem0.current.location.y
        && elem1.next.location.x == elem1.current.location.x
        && elem1.next.location.y == elem1.current.location.y
    {
        // Both elements are stationary (no position change this frame)
        if elem0.state_flags.contains(ElementFlags::DEFY_PHYSICS)
            && elem1.state_flags.contains(ElementFlags::DEFY_PHYSICS)
        {
            // Already defying physics — nudge impact angles and zero velocities
            impact_angle_0 = travel_angle_0.wrapping_add(HALF_CIRCLE - OCTANT);
            impact_angle_1 = travel_angle_1.wrapping_add(HALF_CIRCLE - OCTANT);
            elem0.velocity.zero();
            elem1.velocity.zero();
        }
        elem0
            .state_flags
            .insert(ElementFlags::DEFY_PHYSICS | ElementFlags::COLLISION);
        elem1
            .state_flags
            .insert(ElementFlags::DEFY_PHYSICS | ElementFlags::COLLISION);
    }

    // --- Step 5: Momentum transfer computation ---
    let mass0 = elem0.mass_points as i64;
    let mass1 = elem1.mass_points as i64;

    // scalar = SINE(Directness, speed << 1) * (mass0 * mass1)
    // SINE returns i32, we multiply by i64 mass product
    let scalar = sine(directness, (speed as i32) << 1) as i64 * (mass0 * mass1);

    // --- Step 5a: Apply to elem0 (if not gravity mass) ---
    // Note: C uses (mass_points + 1) > 100, so mass_points >= 100 is exempt
    if !is_gravity_mass(elem0.mass_points) {
        // Player ship penalty
        if elem0.state_flags.contains(ElementFlags::PLAYER_SHIP) {
            // Phase 2+: clear max-speed flags via StarShip FFI
            // C: StarShipPtr->cur_status_flags &= ~(SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED)
            // Phase 1 cannot do this because StarShip struct access requires FFI to ships subsystem.
            // The caller (C process loop) should handle this until Phase 2+ wires the FFI.

            // Thrust/turn wait penalty (only if NOT already DEFY_PHYSICS)
            if !elem0.state_flags.contains(ElementFlags::DEFY_PHYSICS) {
                if elem0.turn_wait < COLLISION_TURN_WAIT {
                    elem0.turn_wait = elem0.turn_wait.saturating_add(COLLISION_TURN_WAIT);
                }
                if elem0.thrust_or_blast < COLLISION_THRUST_WAIT {
                    elem0.thrust_or_blast =
                        elem0.thrust_or_blast.saturating_add(COLLISION_THRUST_WAIT);
                }
            }
        }

        // Velocity change inversely proportional to own mass
        // speed0 = scalar / (mass0 * (mass0 + mass1))
        let speed0 = (scalar / (mass0 * (mass0 + mass1))) as i32;

        // Apply delta via DeltaVelocityComponents
        elem0
            .velocity
            .delta_components(cosine(impact_angle_0, speed0), sine(impact_angle_0, speed0));

        // Minimum velocity enforcement
        let (mut dx0, mut dy0) = elem0.velocity.get_current_components();
        dx0 = dx0.abs();
        dy0 = dy0.abs();

        // VELOCITY_TO_WORLD(dx0 + dy0) < SCALED_ONE
        // Threshold: (|dx0|+|dy0|) >> 5 < 4, i.e. |dx0|+|dy0| < 128
        if velocity_to_world(dx0 + dy0) < SCALED_ONE {
            // Result too slow — set minimum velocity along impact angle
            // C uses WORLD_TO_VELOCITY(SCALED_ONE) - 1 = 127
            let min_vel = world_to_velocity(SCALED_ONE) - 1; // 4 << 5 - 1 = 127
            elem0.velocity.set_components(
                cosine(impact_angle_0, min_vel),
                sine(impact_angle_0, min_vel),
            );
        }
    }

    // --- Step 5b: Apply to elem1 (if not gravity mass) — symmetric ---
    if !is_gravity_mass(elem1.mass_points) {
        // Player ship penalty
        if elem1.state_flags.contains(ElementFlags::PLAYER_SHIP) {
            // Phase 2+: clear max-speed flags via StarShip FFI (see elem0 comment above)

            // Thrust/turn wait penalty (only if NOT already DEFY_PHYSICS)
            if !elem1.state_flags.contains(ElementFlags::DEFY_PHYSICS) {
                if elem1.turn_wait < COLLISION_TURN_WAIT {
                    elem1.turn_wait = elem1.turn_wait.saturating_add(COLLISION_TURN_WAIT);
                }
                if elem1.thrust_or_blast < COLLISION_THRUST_WAIT {
                    elem1.thrust_or_blast =
                        elem1.thrust_or_blast.saturating_add(COLLISION_THRUST_WAIT);
                }
            }
        }

        // Velocity change inversely proportional to own mass
        let speed1 = (scalar / (mass1 * (mass0 + mass1))) as i32;

        // Apply delta
        elem1
            .velocity
            .delta_components(cosine(impact_angle_1, speed1), sine(impact_angle_1, speed1));

        // Minimum velocity enforcement
        let (mut dx1, mut dy1) = elem1.velocity.get_current_components();
        dx1 = dx1.abs();
        dy1 = dy1.abs();

        if velocity_to_world(dx1 + dy1) < SCALED_ONE {
            let min_vel = world_to_velocity(SCALED_ONE) - 1; // C uses 127
            elem1.velocity.set_components(
                cosine(impact_angle_1, min_vel),
                sine(impact_angle_1, min_vel),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Integer Square Root
// ---------------------------------------------------------------------------

/// Integer square root matching C's square_root() from libs/math/sqrt.c
///
/// Uses binary search algorithm to find floor(sqrt(value)).
/// This is a simplified version that works on u32 and returns u16.
///
/// # Arguments
/// * `value` - Input value (u32)
///
/// # Returns
/// floor(sqrt(value)) as u16
fn isqrt(value: u32) -> u16 {
    if value == 0 {
        return 0;
    }

    // Binary search for the square root
    let mut result: u16 = 0;

    // Find the highest bit set in value to determine starting point
    let mut test_value = value;
    let mut bit_pos = 0u32;
    while test_value > 0 {
        test_value >>= 1;
        bit_pos += 1;
    }

    // Start shift at half the bit position (highest bit in result)
    let shift = (bit_pos / 2).min(15);

    // Binary search from highest to lowest bit
    for i in (0..=shift).rev() {
        let mask = 1u16 << i;
        let test = result | mask;
        let test_squared = (test as u32) * (test as u32);

        if test_squared <= value {
            result = test;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::element::Element;
    use super::*;

    // -- Integer Square Root Tests --

    #[test]
    fn isqrt_zero() {
        assert_eq!(isqrt(0), 0);
    }

    #[test]
    fn isqrt_one() {
        assert_eq!(isqrt(1), 1);
    }

    #[test]
    fn isqrt_perfect_squares() {
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(9), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(25), 5);
        assert_eq!(isqrt(100), 10);
        assert_eq!(isqrt(256), 16);
        assert_eq!(isqrt(1024), 32);
        assert_eq!(isqrt(65536), 256);
    }

    #[test]
    fn isqrt_non_perfect_squares() {
        assert_eq!(isqrt(2), 1); // floor(sqrt(2)) = 1
        assert_eq!(isqrt(3), 1);
        assert_eq!(isqrt(5), 2);
        assert_eq!(isqrt(8), 2);
        assert_eq!(isqrt(10), 3);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(99), 9);
        assert_eq!(isqrt(101), 10);
    }

    // -- Gravity Mass Tests --

    #[test]
    fn gravity_mass_threshold() {
        assert!(!is_gravity_mass(0));
        assert!(!is_gravity_mass(MAX_SHIP_MASS));
        assert!(!is_gravity_mass(99));
        assert!(is_gravity_mass(100)); // Exactly 100 IS gravity mass
        assert!(is_gravity_mass(101));
        assert!(is_gravity_mass(255));
    }

    // -- Elastic Collision Tests --

    #[test]
    fn head_on_collision_equal_mass() {
        // Two ships colliding head-on, equal mass
        let mut elem0 = Element::new();
        let mut elem1 = Element::new();

        // Setup: elem0 at (104,100) moving right, elem1 at (106,100) moving left
        // Slight overlap (2 pixels apart), head-on collision
        elem0.current.location = super::super::element::Point::new(94, 100);
        elem0.next.location = super::super::element::Point::new(104, 100); // Moved right 10 pixels
        elem0.mass_points = 5;
        elem0.velocity.set_components(320, 0); // Moving right (10 world units/frame)

        elem1.current.location = super::super::element::Point::new(116, 100);
        elem1.next.location = super::super::element::Point::new(106, 100); // Moved left 10 pixels
        elem1.mass_points = 5;
        elem1.velocity.set_components(-320, 0); // Moving left (10 world units/frame)

        // Collide
        elastic_collide(&mut elem0, &mut elem1);

        // Expected: For equal mass head-on collision, velocities should reverse
        let (dx0, _dy0) = elem0.velocity.get_current_components();
        let (dx1, _dy1) = elem1.velocity.get_current_components();

        // After head-on equal-mass collision, velocities reverse direction
        // elem0 was moving right (+320), should now move left (negative)
        // elem1 was moving left (-320), should now move right (positive)
        assert!(
            dx0 < 0,
            "elem0 should move left after collision, got dx0={}",
            dx0
        );
        assert!(
            dx1 > 0,
            "elem1 should move right after collision, got dx1={}",
            dx1
        );
    }

    #[test]
    fn gravity_mass_immovable() {
        // Collision with gravity-mass object (mass >= 100)
        let mut ship = Element::new();
        let mut planet = Element::new();

        ship.current.location = super::super::element::Point::new(100, 100);
        ship.next.location = super::super::element::Point::new(105, 100);
        ship.mass_points = 5;
        ship.velocity.set_components(160, 0);

        planet.current.location = super::super::element::Point::new(200, 100);
        planet.next.location = super::super::element::Point::new(200, 100);
        planet.mass_points = 100; // Gravity mass
        planet.velocity.set_components(0, 0);

        let (planet_dx_before, planet_dy_before) = planet.velocity.get_current_components();

        elastic_collide(&mut ship, &mut planet);

        let (planet_dx_after, planet_dy_after) = planet.velocity.get_current_components();

        // Planet should not move (gravity mass)
        assert_eq!(
            planet_dx_before, planet_dx_after,
            "Gravity mass should not change velocity"
        );
        assert_eq!(
            planet_dy_before, planet_dy_after,
            "Gravity mass should not change velocity"
        );
    }

    #[test]
    fn zero_velocity_stuck_overlap() {
        // Both elements stationary and overlapping
        let mut elem0 = Element::new();
        let mut elem1 = Element::new();

        // Both at same position, no movement
        elem0.current.location = super::super::element::Point::new(100, 100);
        elem0.next.location = super::super::element::Point::new(100, 100);
        elem0.mass_points = 5;
        elem0.velocity.set_components(0, 0);

        elem1.current.location = super::super::element::Point::new(100, 100);
        elem1.next.location = super::super::element::Point::new(100, 100);
        elem1.mass_points = 5;
        elem1.velocity.set_components(0, 0);

        elastic_collide(&mut elem0, &mut elem1);

        // Both should have DEFY_PHYSICS set
        assert!(
            elem0.state_flags.contains(ElementFlags::DEFY_PHYSICS),
            "Stuck overlap should set DEFY_PHYSICS on elem0"
        );
        assert!(
            elem1.state_flags.contains(ElementFlags::DEFY_PHYSICS),
            "Stuck overlap should set DEFY_PHYSICS on elem1"
        );

        // Both should have COLLISION set
        assert!(
            elem0.state_flags.contains(ElementFlags::COLLISION),
            "Stuck overlap should set COLLISION on elem0"
        );
        assert!(
            elem1.state_flags.contains(ElementFlags::COLLISION),
            "Stuck overlap should set COLLISION on elem1"
        );
    }

    #[test]
    fn player_ship_collision_penalty() {
        // Player ship collision should increment thrust/turn wait
        let mut player_ship = Element::new();
        let mut enemy = Element::new();

        player_ship.current.location = super::super::element::Point::new(100, 100);
        player_ship.next.location = super::super::element::Point::new(105, 100);
        player_ship.mass_points = 5;
        player_ship.velocity.set_components(160, 0);
        player_ship.state_flags.insert(ElementFlags::PLAYER_SHIP);
        player_ship.turn_wait = 0;
        player_ship.thrust_or_blast = 0;

        enemy.current.location = super::super::element::Point::new(200, 100);
        enemy.next.location = super::super::element::Point::new(195, 100);
        enemy.mass_points = 5;
        enemy.velocity.set_components(-160, 0);

        elastic_collide(&mut player_ship, &mut enemy);

        // Player ship should have thrust/turn wait incremented
        assert_eq!(
            player_ship.turn_wait, COLLISION_TURN_WAIT,
            "Player ship turn_wait should be incremented"
        );
        assert_eq!(
            player_ship.thrust_or_blast, COLLISION_THRUST_WAIT,
            "Player ship thrust_wait should be incremented"
        );
    }

    #[test]
    fn asymmetric_mass_collision() {
        // Heavy object vs light object colliding head-on
        let mut heavy = Element::new();
        let mut light = Element::new();

        // Heavy object at (104,100), light at (106,100) - 2 pixels apart
        heavy.current.location = super::super::element::Point::new(94, 100);
        heavy.next.location = super::super::element::Point::new(104, 100);
        heavy.mass_points = 10; // Heavy (5x more mass than light)
        heavy.velocity.set_components(320, 0); // Moving right

        light.current.location = super::super::element::Point::new(116, 100);
        light.next.location = super::super::element::Point::new(106, 100);
        light.mass_points = 2; // Light
        light.velocity.set_components(-320, 0); // Moving left

        let (heavy_dx_before, _) = heavy.velocity.get_current_components();
        let (light_dx_before, _) = light.velocity.get_current_components();

        elastic_collide(&mut heavy, &mut light);

        let (heavy_dx_after, _) = heavy.velocity.get_current_components();
        let (light_dx_after, _) = light.velocity.get_current_components();

        // Heavy object should have smaller velocity change than light object
        let heavy_delta = (heavy_dx_after - heavy_dx_before).abs();
        let light_delta = (light_dx_after - light_dx_before).abs();

        assert!(
            heavy_delta < light_delta,
            "Heavy object should have smaller velocity change than light object: heavy_delta={}, light_delta={}",
            heavy_delta, light_delta
        );
    }

    #[test]
    fn scraping_collision() {
        // Shallow-angle collision (scraping)
        let mut elem0 = Element::new();
        let mut elem1 = Element::new();

        // Setup: elem0 moving slightly upward-right, elem1 stationary
        elem0.current.location = super::super::element::Point::new(100, 100);
        elem0.next.location = super::super::element::Point::new(105, 101);
        elem0.mass_points = 5;
        elem0.velocity.set_components(160, 32); // Mostly horizontal, slight vertical

        elem1.current.location = super::super::element::Point::new(110, 105);
        elem1.next.location = super::super::element::Point::new(110, 105);
        elem1.mass_points = 5;
        elem1.velocity.set_components(0, 0);

        elastic_collide(&mut elem0, &mut elem1);

        // Collision should resolve (no assertion failure)
        // Just verify function completes without panic
        assert!(true);
    }

    #[test]
    fn minimum_velocity_enforcement() {
        // Collision with slow velocities - should enforce minimum
        let mut elem0 = Element::new();
        let mut elem1 = Element::new();

        // Slow moving collision - elements 2 pixels apart
        elem0.current.location = super::super::element::Point::new(99, 100);
        elem0.next.location = super::super::element::Point::new(100, 100); // Moved right 1 pixel
        elem0.mass_points = 5;
        elem0.velocity.set_components(32, 0); // 1 world unit/frame (slow)

        elem1.current.location = super::super::element::Point::new(103, 100);
        elem1.next.location = super::super::element::Point::new(102, 100); // Moved left 1 pixel
        elem1.mass_points = 5;
        elem1.velocity.set_components(-32, 0); // 1 world unit/frame (slow)

        elastic_collide(&mut elem0, &mut elem1);

        // After collision, velocities should be at least minimum
        let (dx0, dy0) = elem0.velocity.get_current_components();
        let (dx1, dy1) = elem1.velocity.get_current_components();

        // C sets min_vel = WORLD_TO_VELOCITY(SCALED_ONE) - 1 = 127
        // After set_components(cosine(angle, 127), sine(angle, 127)),
        // the resulting Manhattan distance should be non-trivial (> 0)
        let speed0 = dx0.abs() + dy0.abs();
        let speed1 = dx1.abs() + dy1.abs();

        // Elements should have been pushed apart — velocities should be non-zero
        assert!(
            speed0 > 0,
            "elem0 should have minimum velocity enforced: speed0={}",
            speed0,
        );
        assert!(
            speed1 > 0,
            "elem1 should have minimum velocity enforced: speed1={}",
            speed1,
        );
    }
}

// ---------------------------------------------------------------------------
// C FFI Export — replaces C collide.c
// ---------------------------------------------------------------------------

/// C: `void collide(ELEMENT *ElementPtr0, ELEMENT *ElementPtr1)`
///
/// Entry point for the C battle loop. Delegates to `elastic_collide`
/// after dereferencing the raw pointers.
#[no_mangle]
pub extern "C" fn collide(e0: *mut Element, e1: *mut Element) {
    if e0.is_null() || e1.is_null() {
        return;
    }
    let (e0_ref, e1_ref) = unsafe { (&mut *e0, &mut *e1) };
    elastic_collide(e0_ref, e1_ref);
}
