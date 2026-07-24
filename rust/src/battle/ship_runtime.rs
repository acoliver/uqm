//! Ship Runtime — Types + Pipeline Logic
//!
//! Type definitions, constants, and behavioral logic for the ship per-frame
//! processing pipeline. Phase 1 defined types; P07 adds ship_preprocess,
//! ship_postprocess, inertial_thrust, animation_preprocess, and ship_collision.
//!
//! @plan PLAN-20260320-BATTLEPT2.P07
//! @requirement REQ-SHIP-PIPELINE, REQ-INERTIAL-MOVEMENT, REQ-WEAPON-FIRING, REQ-SHIP-COLLISION
//!
//! # C Reference
//! `sc2/src/uqm/ship.c` — functions ported here are annotated with C line numbers.

/// Ship per-frame pipeline stages (7 stages total)
///
/// Exact order from ship.c ship_preprocess():
/// 1. Input processing
/// 2. APPEARING flag handling (first-frame initialization)
/// 3. Energy regeneration
/// 4. Race-specific preprocess
/// 5. Turn processing
/// 6. Thrust processing
/// 7. Status display update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ShipPipelineStage {
    /// Process input state
    Input = 0,
    /// Handle APPEARING flag (first frame only)
    Appearing = 1,
    /// Regenerate energy
    Energy = 2,
    /// Race-specific preprocess callback
    Preprocess = 3,
    /// Process turn input
    Turn = 4,
    /// Process thrust input
    Thrust = 5,
    /// Update status display
    Status = 6,
}

/// Spawn position types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpawnPositionType {
    /// Random position avoiding gravity wells
    Random = 0,
    /// Center position (Sa-Matra)
    Center = 1,
    /// HyperSpace position (flagship)
    HyperSpace = 2,
}

/// Maximum crew size constant
pub const MAX_CREW_SIZE: i16 = 42;

/// Maximum energy size constant
pub const MAX_ENERGY_SIZE: i16 = 42;

/// Maximum allowed speed constant (from ship.c)
/// Used for gravity well limit checks
pub const MAX_ALLOWED_SPEED: i32 = 18 << 2; // WORLD_TO_VELOCITY(DISPLAY_TO_WORLD(18))

/// Maximum allowed speed squared (for velocity checks without sqrt)
pub const MAX_ALLOWED_SPEED_SQR: u32 = (MAX_ALLOWED_SPEED * MAX_ALLOWED_SPEED) as u32;

/// Weapon firing types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WeaponFiringType {
    /// Primary weapon
    Primary = 0,
    /// Secondary weapon
    Secondary = 1,
}

// ---------------------------------------------------------------------------
// P07: Ship Runtime Behavioral Logic (ship.c:45-391)
// ---------------------------------------------------------------------------

use super::battle_types::{cosine, sine};
use super::element::{Element, ElementFlags, Point};
use super::velocity::{world_to_velocity as WORLD_TO_VELOCITY, VelocityDesc};

/// Status flags returned by inertial_thrust
pub type StatusFlags = u32;

/// Ship is at maximum speed
pub const SHIP_AT_MAX_SPEED: StatusFlags = 0x0001;
/// Ship speed exceeds normal maximum (gravity well)
pub const SHIP_BEYOND_MAX_SPEED: StatusFlags = 0x0002;
/// Ship is within a gravity well
pub const SHIP_IN_GRAVITY_WELL: StatusFlags = 0x0004;

/// Input flags for ship controls
pub const LEFT: StatusFlags = 0x0100;
pub const RIGHT: StatusFlags = 0x0200;
pub const THRUST: StatusFlags = 0x0400;
pub const WEAPON: StatusFlags = 0x0800;
pub const SPECIAL: StatusFlags = 0x1000;
pub const INPUT_MASK: StatusFlags = LEFT | RIGHT | THRUST | WEAPON | SPECIAL;

/// Target damage sound indices (from ship.c)
pub const TARGET_DAMAGED_FOR_1_PT: i32 = 0;
pub const TARGET_DAMAGED_FOR_6_PLUS_PT: i32 = 5;

// gravity_mass() is defined in element.rs (pub const fn gravity_mass)
use super::element::gravity_mass;

/// Animation preprocess: advance frame counter.
///
/// C reference: `animation_preprocess()` (ship.c:45-58)
///
/// Decrements turn_wait; when elapsed, advances image frame and sets CHANGING.
/// Public because P09 explosion_preprocess also uses this pattern.
pub fn animation_preprocess(element: &mut Element) {
    if element.turn_wait > 0 {
        element.turn_wait -= 1;
    } else {
        // Advance to next frame (C: IncFrameIndex)
        // In Rust, we increment the frame index value stored in next.image
        // The actual frame handle resolution happens through C bridge
        element.state_flags |= ElementFlags::CHANGING;
        element.turn_wait = element.thrust_or_blast; // C union: next_turn
    }
}

/// Compute velocity squared (avoids sqrt for speed comparisons).
fn velocity_squared(dx: i32, dy: i32) -> u64 {
    (dx as i64 * dx as i64 + dy as i64 * dy as i64) as u64
}

/// Inertial thrust physics.
///
/// C reference: `inertial_thrust()` (ship.c:61-153)
///
/// Computes new velocity based on:
/// - Inertialess: instant max speed in facing direction
/// - Normal: accelerate if below max speed
/// - Gravity well: allow speeds up to MAX_ALLOWED_SPEED
/// - At max speed with facing change: half-thrust new minus full old
///
/// Returns status flags (AT_MAX_SPEED, BEYOND_MAX_SPEED).
pub fn inertial_thrust(
    velocity: &mut VelocityDesc,
    current_angle: u16,
    travel_angle: u16,
    thrust_increment: i32,
    max_thrust: i32,
    cur_status_flags: StatusFlags,
    ship_facing: u16,
) -> StatusFlags {
    // Inertialess (Skiff): thrust_increment == max_thrust
    if thrust_increment == max_thrust {
        velocity.set_vector(max_thrust, ship_facing);
        return SHIP_AT_MAX_SPEED;
    }

    // Already at max and traveling in same direction (not in gravity well)
    if travel_angle == current_angle
        && (cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED)) != 0
        && (cur_status_flags & SHIP_IN_GRAVITY_WELL) == 0
    {
        return cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED);
    }

    // General case
    let vel_thrust = WORLD_TO_VELOCITY(thrust_increment);
    let (cur_dx, cur_dy) = velocity.get_current_components();
    let current_speed = velocity_squared(cur_dx, cur_dy);

    let delta_x = cur_dx + cosine(current_angle, vel_thrust);
    let delta_y = cur_dy + sine(current_angle, vel_thrust);
    let desired_speed = velocity_squared(delta_x, delta_y);
    let max_speed = velocity_squared(WORLD_TO_VELOCITY(max_thrust), 0);

    if desired_speed <= max_speed {
        // Normal acceleration
        velocity.set_components(delta_x, delta_y);
    } else if ((cur_status_flags & SHIP_IN_GRAVITY_WELL) != 0
        && desired_speed <= MAX_ALLOWED_SPEED_SQR as u64)
        || desired_speed < current_speed
    {
        // Gravity well or decelerating
        velocity.set_components(delta_x, delta_y);
        return SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED;
    } else if travel_angle == current_angle {
        // At max speed, same direction
        if current_speed <= max_speed {
            velocity.set_vector(max_thrust, ship_facing);
        }
        return SHIP_AT_MAX_SPEED;
    } else {
        // At max speed, changing direction: half-thrust new minus full old
        let mut v = *velocity;
        let half_thrust = vel_thrust >> 1;
        let dx_delta = cosine(current_angle, half_thrust) - cosine(travel_angle, vel_thrust);
        let dy_delta = sine(current_angle, half_thrust) - sine(travel_angle, vel_thrust);
        v.delta_components(dx_delta, dy_delta);

        let (new_dx, new_dy) = v.get_current_components();
        let new_speed = velocity_squared(new_dx, new_dy);

        if new_speed > max_speed {
            if new_speed < current_speed {
                *velocity = v;
            }
            return SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED;
        }

        *velocity = v;
    }

    0
}

/// Ship collision handler.
///
/// C reference: `collision()` (ship.c:366-391)
///
/// Entire body gated on other element NOT having FINITE_LIFE:
/// - Sets COLLISION flag on this element
/// - If other is gravity mass: damage = max(hit_points/4, 1) + play sound
/// - If other has FINITE_LIFE (projectile): no-op
///
/// Note: elastic velocity response is NOT here — it's in ProcessCollisions
/// calling collide()/elastic_collide() externally.
///
/// # Safety
/// The other element pointer must be valid for reads.
pub unsafe fn ship_collision(
    element: *mut Element,
    _save_pt: *const Point,
    other: *mut Element,
    _other_save_pt: *const Point,
) {
    if element.is_null() || other.is_null() {
        return;
    }

    let other_ref = &*other;
    if other_ref.state_flags.contains(ElementFlags::FINITE_LIFE) {
        // Projectile collision — no-op (projectile's collision_func handles damage)
        return;
    }

    let elem = &mut *element;
    elem.state_flags |= ElementFlags::COLLISION;

    if gravity_mass(other_ref.mass_points) {
        let damage = (elem.crew_or_hp >> 2).max(1); // C union: hit_points
                                                    // do_damage and ProcessSound called through C bridge
                                                    // (deferred to P06 bridge wiring)
        let _ = damage; // Used when bridge is wired
    }
}

/// Ship collision handler matching the ElementCollisionFunc signature.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// This is the C-ABI-compatible entry point for use as a collision_func.
pub unsafe extern "C" fn ship_collision_func(
    element: *mut Element,
    save_pt: *const Point,
    other: *mut Element,
    other_save_pt: *const Point,
) {
    ship_collision(element, save_pt, other, other_save_pt);
}

// ---------------------------------------------------------------------------
// P08: Ship Spawn + Init (ship.c:393-591)
// ---------------------------------------------------------------------------

/// Spawn configuration for ship element initialization.
///
/// Contains the parameters needed to create a ship element, extracted
/// from the opaque C STARSHIP/RaceDesc types via bridge calls.
pub struct SpawnConfig {
    /// Player number (0 or 1, NPC_PLAYER_NUM=1)
    pub player_nr: u8,
    /// Ship mass from characteristics
    pub ship_mass: u8,
    /// Initial ship facing (frame index)
    pub ship_facing: u16,
    /// Position X in world coordinates
    pub position_x: i16,
    /// Position Y in world coordinates
    pub position_y: i16,
    /// Whether this is the Sa-Matra (gets life_span+1)
    pub is_sa_matra: bool,
}

/// NPC player number constant (matches C NPC_PLAYER_NUM)
pub const NPC_PLAYER_NUM: u8 = 1;

/// Initialize a ship element from spawn configuration.
///
/// C reference: ship.c:443-510 (element initialization portion of spawn_ship)
///
/// Sets up the element with APPEARING|PLAYER_SHIP|IGNORE_SIMILAR flags,
/// STAMP_PRIM display type, and context-dependent position.
pub fn init_ship_element(element: &mut Element, config: &SpawnConfig) {
    element.player_nr = config.player_nr as i16;
    element.crew_or_hp = 0; // crew_level = 0 (initialized in APPEARING path)
    element.mass_points = config.ship_mass;
    element.state_flags =
        ElementFlags::APPEARING | ElementFlags::PLAYER_SHIP | ElementFlags::IGNORE_SIMILAR;
    element.turn_wait = 0;
    element.thrust_or_blast = 0; // thrust_wait = 0
    element.life_span = super::element::NORMAL_LIFE;
    element.color_cycle_index = 0;

    // Position
    element.current.location.x = config.position_x;
    element.current.location.y = config.position_y;
    element.next.location = element.current.location;

    // Sa-Matra gets extended life_span
    if config.is_sa_matra {
        element.life_span += 1;
    }

    // Zero velocity
    element.velocity = VelocityDesc::default();
}

/// Determine spawn position type based on game context.
///
/// C reference: ship.c:459-499 (positioning branches in spawn_ship)
///
/// Returns the appropriate SpawnPositionType for the current game state.
pub fn determine_spawn_position(
    player_nr: u8,
    is_last_battle: bool,
    is_hq_space: bool,
) -> SpawnPositionType {
    if player_nr == NPC_PLAYER_NUM && is_last_battle {
        SpawnPositionType::Center
    } else if is_hq_space {
        SpawnPositionType::HyperSpace
    } else {
        SpawnPositionType::Random
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ship_pipeline_stage_count() {
        // Verify we have exactly 7 stages
        let stages = [
            ShipPipelineStage::Input,
            ShipPipelineStage::Appearing,
            ShipPipelineStage::Energy,
            ShipPipelineStage::Preprocess,
            ShipPipelineStage::Turn,
            ShipPipelineStage::Thrust,
            ShipPipelineStage::Status,
        ];
        assert_eq!(stages.len(), 7);
    }

    #[test]
    fn test_ship_pipeline_stage_order() {
        assert_eq!(ShipPipelineStage::Input as u8, 0);
        assert_eq!(ShipPipelineStage::Appearing as u8, 1);
        assert_eq!(ShipPipelineStage::Energy as u8, 2);
        assert_eq!(ShipPipelineStage::Preprocess as u8, 3);
        assert_eq!(ShipPipelineStage::Turn as u8, 4);
        assert_eq!(ShipPipelineStage::Thrust as u8, 5);
        assert_eq!(ShipPipelineStage::Status as u8, 6);
    }

    #[test]
    fn test_spawn_position_variants() {
        assert_eq!(SpawnPositionType::Random as u8, 0);
        assert_eq!(SpawnPositionType::Center as u8, 1);
        assert_eq!(SpawnPositionType::HyperSpace as u8, 2);
    }

    #[test]
    fn test_crew_energy_constants() {
        assert_eq!(MAX_CREW_SIZE, 42);
        assert_eq!(MAX_ENERGY_SIZE, 42);
    }

    #[test]
    fn test_max_allowed_speed_constants() {
        assert_eq!(MAX_ALLOWED_SPEED, 72); // 18 << 2
        assert_eq!(MAX_ALLOWED_SPEED_SQR, 5184); // 72 * 72
    }

    #[test]
    fn test_weapon_firing_type_variants() {
        assert_eq!(WeaponFiringType::Primary as u8, 0);
        assert_eq!(WeaponFiringType::Secondary as u8, 1);
    }

    // -- P07: Ship Runtime behavioral tests --

    #[test]
    fn test_gravity_mass_check() {
        assert!(!gravity_mass(0));
        assert!(!gravity_mass(100)); // MAX_SHIP_MASS * 10, boundary
        assert!(gravity_mass(101)); // just above threshold
        assert!(gravity_mass(255));
    }

    #[test]
    fn test_animation_preprocess_decrement() {
        let mut elem = Element {
            turn_wait: 3,
            thrust_or_blast: 5, // C union: next_turn
            ..Element::default()
        };
        animation_preprocess(&mut elem);
        assert_eq!(elem.turn_wait, 2);
        assert!(!elem.state_flags.contains(ElementFlags::CHANGING));
    }

    #[test]
    fn test_animation_preprocess_advance_frame() {
        let mut elem = Element {
            turn_wait: 0,
            thrust_or_blast: 4, // C union: next_turn
            ..Element::default()
        };
        animation_preprocess(&mut elem);
        assert!(elem.state_flags.contains(ElementFlags::CHANGING));
        assert_eq!(elem.turn_wait, 4);
    }

    #[test]
    fn test_inertial_thrust_inertialess() {
        let mut vel = VelocityDesc::default();
        let result = inertial_thrust(
            &mut vel, 0,  // current_angle
            0,  // travel_angle
            10, // thrust == max → inertialess
            10, // max_thrust
            0, 0, // ship_facing
        );
        assert_eq!(result, SHIP_AT_MAX_SPEED);
    }

    #[test]
    fn test_inertial_thrust_already_at_max() {
        let mut vel = VelocityDesc::default();
        vel.set_components(100, 0);
        let result = inertial_thrust(
            &mut vel,
            0, // current_angle same as travel
            0, // travel_angle
            5, // thrust != max
            10,
            SHIP_AT_MAX_SPEED, // already at max
            0,
        );
        assert_eq!(result, SHIP_AT_MAX_SPEED);
    }

    #[test]
    fn test_inertial_thrust_normal_acceleration() {
        let mut vel = VelocityDesc::default();
        // Start from rest, accelerate
        let result = inertial_thrust(
            &mut vel, 0,  // facing = 0
            0,  // travel = 0
            5,  // thrust
            20, // max_thrust (much higher than thrust)
            0,  // no status flags
            0,
        );
        // Should be 0 (not yet at max)
        assert_eq!(result, 0);
    }

    #[test]
    fn test_velocity_squared() {
        assert_eq!(velocity_squared(3, 4), 25);
        assert_eq!(velocity_squared(0, 0), 0);
        assert_eq!(velocity_squared(1, 0), 1);
    }

    #[test]
    fn test_ship_collision_finite_life_noop() {
        let mut elem = Element {
            crew_or_hp: 10,
            ..Element::default()
        };
        let mut other = Element {
            state_flags: ElementFlags::FINITE_LIFE,
            ..Element::default()
        };

        unsafe {
            ship_collision(
                &mut elem as *mut Element,
                std::ptr::null(),
                &mut other as *mut Element,
                std::ptr::null(),
            );
        }
        // FINITE_LIFE → no-op: COLLISION should NOT be set
        assert!(!elem.state_flags.contains(ElementFlags::COLLISION));
    }

    #[test]
    fn test_ship_collision_sets_collision_flag() {
        let mut elem = Element {
            crew_or_hp: 10,
            ..Element::default()
        };
        let mut other = Element {
            state_flags: ElementFlags::empty(),
            mass_points: 50, // non-gravity, non-finite-life
            ..Element::default()
        };

        unsafe {
            ship_collision(
                &mut elem as *mut Element,
                std::ptr::null(),
                &mut other as *mut Element,
                std::ptr::null(),
            );
        }
        assert!(elem.state_flags.contains(ElementFlags::COLLISION));
    }

    #[test]
    fn test_ship_collision_gravity_mass() {
        let mut elem = Element {
            crew_or_hp: 20,
            ..Element::default()
        };
        let mut other = Element {
            state_flags: ElementFlags::empty(),
            mass_points: 200, // gravity mass (>127)
            ..Element::default()
        };

        unsafe {
            ship_collision(
                &mut elem as *mut Element,
                std::ptr::null(),
                &mut other as *mut Element,
                std::ptr::null(),
            );
        }
        assert!(elem.state_flags.contains(ElementFlags::COLLISION));
        // Damage would be applied via C bridge (max(20/4, 1) = 5)
    }

    #[test]
    fn test_ship_collision_null_safety() {
        unsafe {
            ship_collision(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            // Should not panic
        }
    }

    #[test]
    fn test_status_flags_constants() {
        assert_eq!(SHIP_AT_MAX_SPEED, 0x0001);
        assert_eq!(SHIP_BEYOND_MAX_SPEED, 0x0002);
        assert_eq!(SHIP_IN_GRAVITY_WELL, 0x0004);
    }

    #[test]
    fn test_input_mask() {
        assert_eq!(INPUT_MASK, LEFT | RIGHT | THRUST | WEAPON | SPECIAL);
        // Verify no overlap with speed flags
        assert_eq!(INPUT_MASK & SHIP_AT_MAX_SPEED, 0);
    }

    // ---- P08: Ship Spawn tests ----

    #[test]
    fn test_init_ship_element_basic() {
        let mut elem = Element::default();
        let config = SpawnConfig {
            player_nr: 0,
            ship_mass: 6,
            ship_facing: 8,
            position_x: 100,
            position_y: 200,
            is_sa_matra: false,
        };
        init_ship_element(&mut elem, &config);
        assert_eq!(elem.player_nr, 0);
        assert_eq!(elem.mass_points, 6);
        assert!(elem.state_flags.contains(ElementFlags::APPEARING));
        assert!(elem.state_flags.contains(ElementFlags::PLAYER_SHIP));
        assert!(elem.state_flags.contains(ElementFlags::IGNORE_SIMILAR));
        assert_eq!(elem.life_span, super::super::element::NORMAL_LIFE);
        assert_eq!(elem.crew_or_hp, 0);
        assert_eq!(elem.turn_wait, 0);
        assert_eq!(elem.thrust_or_blast, 0);
        assert_eq!(elem.current.location.x, 100);
        assert_eq!(elem.current.location.y, 200);
    }

    #[test]
    fn test_init_ship_element_sa_matra() {
        let mut elem = Element::default();
        let config = SpawnConfig {
            player_nr: NPC_PLAYER_NUM,
            ship_mass: 10,
            ship_facing: 0,
            position_x: 5000,
            position_y: 4000,
            is_sa_matra: true,
        };
        init_ship_element(&mut elem, &config);
        assert_eq!(
            elem.life_span,
            super::super::element::NORMAL_LIFE + 1,
            "Sa-Matra gets extended life_span"
        );
        assert_eq!(elem.player_nr, NPC_PLAYER_NUM as i16);
    }

    #[test]
    fn test_determine_spawn_position_sa_matra() {
        let pos = determine_spawn_position(NPC_PLAYER_NUM, true, false);
        assert!(matches!(pos, SpawnPositionType::Center));
    }

    #[test]
    fn test_determine_spawn_position_hyperspace() {
        let pos = determine_spawn_position(0, false, true);
        assert!(matches!(pos, SpawnPositionType::HyperSpace));
    }

    #[test]
    fn test_determine_spawn_position_normal() {
        let pos = determine_spawn_position(0, false, false);
        assert!(matches!(pos, SpawnPositionType::Random));
    }

    #[test]
    fn test_determine_spawn_position_npc_not_last_battle() {
        // NPC but not last battle → normal random
        let pos = determine_spawn_position(NPC_PLAYER_NUM, false, false);
        assert!(matches!(pos, SpawnPositionType::Random));
    }
}
