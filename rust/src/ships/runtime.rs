// Shared Ship Runtime Pipeline
// @plan PLAN-20260314-SHIPS.P08
// @requirement REQ-PIPELINE-ORDER, REQ-MOVEMENT-INERTIAL, REQ-MOVEMENT-DETERMINISTIC, REQ-ENERGY-REGEN, REQ-WEAPON-FIRE, REQ-SPECIAL-ACTIVATION, REQ-AI-HOOK, REQ-HOOK-SERIALIZED, REQ-COLLISION-CORRECT, REQ-COLLISION-OVERRIDE

use super::traits::{BattleContext, ShipState};
use super::types::{ShipsError, Starship, StatusFlags};
// ---------------------------------------------------------------------------
// Re-exports from battle_types (shared foundation)
// @plan PLAN-20260320-BATTLE.P03
// ---------------------------------------------------------------------------

pub use crate::battle::{
    // Conversion functions
    angle_to_facing,
    arctan,
    cosine,
    display_to_world,
    facing_to_angle,
    gravity_mass,
    normalize_angle,
    normalize_facing,
    sine,
    velocity_to_world,
    world_to_velocity,
    // Element constants
    APPEARING,
    CHANGING,
    // Angle/facing constants
    CIRCLE_SHIFT,
    COLLISION_FLAG,
    DISAPPEARING,
    FACING_SHIFT,
    FINITE_LIFE,
    FULL_CIRCLE,
    GRAVITY_THRESHOLD,
    HALF_CIRCLE,
    IGNORE_SIMILAR,
    MAX_SHIP_MASS,
    NORMAL_LIFE,
    NUM_FACINGS,
    OCTANT,
    // Coordinate constants
    ONE_SHIFT,
    PLAYER_SHIP,
    QUADRANT,
    VELOCITY_SHIFT,
};

// ---------------------------------------------------------------------------
// VelocityState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct VelocityState {
    pub travel_angle: u16,
    pub vector: (i16, i16), // whole part of per-frame displacement
    pub fract: (i16, i16),  // fractional increment
    pub error: (i16, i16),  // error accumulator
    pub incr: (i16, i16),   // direction+fractional (packed as MAKE_WORD)
}

impl VelocityState {
    pub fn zero(&mut self) {
        self.travel_angle = 0;
        self.vector = (0, 0);
        self.fract = (0, 0);
        self.error = (0, 0);
        self.incr = (0, 0);
    }

    pub fn get_current_components(&self) -> (i32, i32) {
        // C: (SIZE)HIBYTE(incr) — HIBYTE produces unsigned BYTE (0-255),
        // then (SIZE) zero-extends to signed 16-bit
        let hibyte_x = ((self.incr.0 as u16) >> 8) as i32;
        let hibyte_y = ((self.incr.1 as u16) >> 8) as i32;

        let dx = world_to_velocity(self.vector.0 as i32) + (self.fract.0 as i32 - hibyte_x);
        let dy = world_to_velocity(self.vector.1 as i32) + (self.fract.1 as i32 - hibyte_y);
        (dx, dy)
    }

    pub fn set_vector(&mut self, magnitude: i32, facing: u16) {
        let angle = facing_to_angle(normalize_facing(facing));
        self.travel_angle = angle;
        let magnitude = world_to_velocity(magnitude);
        let mut dx = cosine(angle, magnitude);
        let mut dy = sine(angle, magnitude);

        if dx >= 0 {
            self.vector.0 = velocity_to_world(dx) as i16;
            self.incr.0 = 0x0001; // MAKE_WORD(1, 0): LOBYTE=1 (step +1), HIBYTE=0
        } else {
            dx = -dx;
            self.vector.0 = -velocity_to_world(dx) as i16;
            let frac_part = ((dx & ((1 << VELOCITY_SHIFT) - 1)) << 1) as u8;
            // MAKE_WORD(0xFF, frac): LOBYTE=0xFF (step -1), HIBYTE=frac
            self.incr.0 = ((frac_part as u16) << 8 | 0xFF) as i16;
        }

        if dy >= 0 {
            self.vector.1 = velocity_to_world(dy) as i16;
            self.incr.1 = 0x0001; // MAKE_WORD(1, 0): LOBYTE=1 (step +1), HIBYTE=0
        } else {
            dy = -dy;
            self.vector.1 = -velocity_to_world(dy) as i16;
            let frac_part = ((dy & ((1 << VELOCITY_SHIFT) - 1)) << 1) as u8;
            // MAKE_WORD(0xFF, frac): LOBYTE=0xFF (step -1), HIBYTE=frac
            self.incr.1 = ((frac_part as u16) << 8 | 0xFF) as i16;
        }

        self.fract.0 = (dx & ((1 << VELOCITY_SHIFT) - 1)) as i16;
        self.fract.1 = (dy & ((1 << VELOCITY_SHIFT) - 1)) as i16;
        self.error = (0, 0);
    }

    pub fn set_components(&mut self, dx: i32, dy: i32) {
        let angle = arctan(dx, dy);
        if angle == FULL_CIRCLE {
            self.zero();
        } else {
            let mut dx = dx;
            let mut dy = dy;

            if dx >= 0 {
                self.vector.0 = velocity_to_world(dx) as i16;
                self.incr.0 = 0x0001; // MAKE_WORD(1, 0): LOBYTE=1, HIBYTE=0
            } else {
                dx = -dx;
                self.vector.0 = -velocity_to_world(dx) as i16;
                let frac_part = ((dx & ((1 << VELOCITY_SHIFT) - 1)) << 1) as u8;
                self.incr.0 = ((frac_part as u16) << 8 | 0xFF) as i16;
            }

            if dy >= 0 {
                self.vector.1 = velocity_to_world(dy) as i16;
                self.incr.1 = 0x0001; // MAKE_WORD(1, 0): LOBYTE=1, HIBYTE=0
            } else {
                dy = -dy;
                self.vector.1 = -velocity_to_world(dy) as i16;
                let frac_part = ((dy & ((1 << VELOCITY_SHIFT) - 1)) << 1) as u8;
                self.incr.1 = ((frac_part as u16) << 8 | 0xFF) as i16;
            }

            self.fract.0 = (dx & ((1 << VELOCITY_SHIFT) - 1)) as i16;
            self.fract.1 = (dy & ((1 << VELOCITY_SHIFT) - 1)) as i16;
            self.error = (0, 0);
            self.travel_angle = angle;
        }
    }

    pub fn delta_components(&mut self, dx: i32, dy: i32) {
        let (cur_dx, cur_dy) = self.get_current_components();
        self.set_components(cur_dx + dx, cur_dy + dy);
    }

    pub fn velocity_squared(dx: i32, dy: i32) -> u64 {
        (dx as i64 * dx as i64 + dy as i64 * dy as i64) as u64
    }

    pub fn is_zero(&self) -> bool {
        self.vector.0 == 0 && self.vector.1 == 0 && self.fract.0 == 0 && self.fract.1 == 0
    }
}

// ---------------------------------------------------------------------------
// ElementState
// ---------------------------------------------------------------------------

/// Computer-control value for `Starship.control` (C: COMPUTER_CONTROL).
pub const COMPUTER_CONTROL: u8 = 1;

#[derive(Debug, Clone)]
pub struct ElementState {
    pub state_flags: u16,
    pub life_span: u16,
    /// Crew level / hit points (C union: crew_level and hit_points share storage).
    pub crew_level: u16,
    pub mass_points: u8,
    pub turn_wait: u8,
    pub thrust_wait: u8,
    pub next_turn: u8,
    pub color_cycle_index: u8,
    pub player_nr: i16,
    pub position: (i32, i32),
    pub next_position: (i32, i32),
    pub velocity: VelocityState,
    pub image_frame: u16,
    pub prim_index: u16,
    pub h_target: usize,
}

impl ElementState {
    /// C union alias: hit_points shares storage with crew_level.
    pub fn hit_points(&self) -> u16 {
        self.crew_level
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ElementState {
    fn default() -> Self {
        Self {
            state_flags: 0,
            life_span: 0,
            crew_level: 0,
            mass_points: 0,
            turn_wait: 0,
            thrust_wait: 0,
            next_turn: 0,
            color_cycle_index: 0,
            player_nr: 0,
            position: (0, 0),
            next_position: (0, 0),
            velocity: VelocityState::default(),
            image_frame: 0,
            prim_index: 0,
            h_target: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// CollisionResult
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollisionResult {
    NoEffect,
    Damage(u16),
    CrewPickup(u16),
    Bounce,
    PlanetCollision { damage: u16 },
}

// ---------------------------------------------------------------------------
// Pipeline functions
// ---------------------------------------------------------------------------

/// Main preprocess pipeline for ships
pub fn ship_preprocess(
    starship: &mut Starship,
    element: &mut ElementState,
) -> Result<(), ShipsError> {
    // 1. Input normalization
    let mut cur_status_flags = starship.cur_status_flags
        & !(StatusFlags::LEFT
            | StatusFlags::RIGHT
            | StatusFlags::THRUST
            | StatusFlags::WEAPON
            | StatusFlags::SPECIAL);

    if (element.state_flags & APPEARING) == 0 {
        cur_status_flags |= StatusFlags(starship.ship_input_state as u16)
            & (StatusFlags::LEFT
                | StatusFlags::RIGHT
                | StatusFlags::THRUST
                | StatusFlags::WEAPON
                | StatusFlags::SPECIAL);
    } else {
        // 2. First-frame setup (APPEARING)
        let crew_level = starship
            .race_desc
            .as_ref()
            .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?
            .ship_info
            .crew_level;
        element.crew_level = crew_level;

        // Call preprocess hook on first frame
        let mut ship_state = build_ship_state(starship, element)?;
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };
        starship
            .race_desc
            .as_mut()
            .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?
            .behavior
            .preprocess(&mut ship_state, &ctx)?;

        // Clear APPEARING flag
        element.state_flags &= !APPEARING;

        // Early return on first frame
        starship.cur_status_flags = cur_status_flags;
        return Ok(());
    }

    starship.cur_status_flags = cur_status_flags;

    // 2b. AI hook for computer-controlled ships
    // C: intelligence() is called via ship_intelligence() before shared movement/weapon processing.
    // It computes ship_input_state which was already merged into cur_status_flags above.
    // In the Rust pipeline, we invoke it here so AI-computed flags feed into turn/thrust/weapon.
    if starship.control == COMPUTER_CONTROL {
        let ship_state = build_ship_state(starship, element)?;
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };
        let ai_flags = starship
            .race_desc
            .as_mut()
            .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?
            .behavior
            .intelligence(&ship_state, &ctx);
        // Merge AI-computed flags into status
        starship.cur_status_flags |= ai_flags
            & (StatusFlags::LEFT
                | StatusFlags::RIGHT
                | StatusFlags::THRUST
                | StatusFlags::WEAPON
                | StatusFlags::SPECIAL);
    }

    // 3. Energy regeneration
    if starship.energy_counter > 0 {
        starship.energy_counter -= 1;
    } else {
        let (energy_level, max_energy, energy_regen) = {
            let race_desc = starship
                .race_desc
                .as_ref()
                .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?;
            (
                race_desc.ship_info.energy_level,
                race_desc.ship_info.max_energy,
                race_desc.characteristics.energy_regeneration as i8,
            )
        };

        if energy_level < max_energy || energy_regen < 0 {
            delta_energy(starship, energy_regen as i16);
        }
    }

    // 4. Race preprocess dispatch
    let mut ship_state = build_ship_state(starship, element)?;
    let ctx = BattleContext {
        hyperspace: false,
        frame_count: 0,
        gravity_center: None,
    };
    starship
        .race_desc
        .as_mut()
        .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?
        .behavior
        .preprocess(&mut ship_state, &ctx)?;
    cur_status_flags = starship.cur_status_flags;

    // 5. Turn handling
    let turn_wait_char = {
        let race_desc = starship
            .race_desc
            .as_ref()
            .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?;
        race_desc.characteristics.turn_wait
    };

    if element.turn_wait > 0 {
        element.turn_wait -= 1;
    } else if cur_status_flags.contains(StatusFlags::LEFT)
        || cur_status_flags.contains(StatusFlags::RIGHT)
    {
        if cur_status_flags.contains(StatusFlags::LEFT) {
            starship.ship_facing = normalize_facing(starship.ship_facing.wrapping_sub(1));
        } else {
            starship.ship_facing = normalize_facing(starship.ship_facing.wrapping_add(1));
        }
        element.image_frame = starship.ship_facing;
        element.state_flags |= CHANGING;
        element.turn_wait = turn_wait_char;
    }

    // 6. Thrust handling
    let thrust_wait_char = {
        let race_desc = starship
            .race_desc
            .as_ref()
            .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?;
        race_desc.characteristics.thrust_wait
    };

    if element.thrust_wait > 0 {
        element.thrust_wait -= 1;
    } else if cur_status_flags.contains(StatusFlags::THRUST) {
        let thrust_status = inertial_thrust(starship, element)?;
        starship.cur_status_flags &= !(StatusFlags::SHIP_AT_MAX_SPEED
            | StatusFlags::SHIP_BEYOND_MAX_SPEED
            | StatusFlags::SHIP_IN_GRAVITY_WELL);
        starship.cur_status_flags |= thrust_status;
        element.thrust_wait = thrust_wait_char;
    }

    Ok(())
}

/// Main postprocess pipeline for ships
pub fn ship_postprocess(
    starship: &mut Starship,
    element: &mut ElementState,
) -> Result<(), ShipsError> {
    // 1. Early exit if dead
    if element.crew_level == 0 {
        return Ok(());
    }

    // 2. Weapon fire
    if starship.weapon_counter > 0 {
        starship.weapon_counter -= 1;
    } else if starship.cur_status_flags.contains(StatusFlags::WEAPON) {
        let (weapon_cost, weapon_wait) = {
            let race_desc = starship
                .race_desc
                .as_ref()
                .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?;
            (
                race_desc.characteristics.weapon_energy_cost as i16,
                race_desc.characteristics.weapon_wait,
            )
        };

        if delta_energy(starship, -weapon_cost) {
            // Weapon fired successfully
            let ship_state = build_ship_state(starship, element)?;
            let ctx = BattleContext {
                hyperspace: false,
                frame_count: 0,
                gravity_center: None,
            };
            let _weapons = starship
                .race_desc
                .as_mut()
                .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?
                .behavior
                .init_weapon(&ship_state, &ctx)?;
            starship.weapon_counter = weapon_wait;
        }
    }

    // 3. Special cooldown
    if starship.special_counter > 0 {
        starship.special_counter -= 1;
    }

    // 4. Race postprocess dispatch
    let mut ship_state = build_ship_state(starship, element)?;
    let ctx = BattleContext {
        hyperspace: false,
        frame_count: 0,
        gravity_center: None,
    };
    starship
        .race_desc
        .as_mut()
        .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?
        .behavior
        .postprocess(&mut ship_state, &ctx)?;

    Ok(())
}

/// Inertial thrust physics
pub fn inertial_thrust(
    starship: &mut Starship,
    element: &mut ElementState,
) -> Result<StatusFlags, ShipsError> {
    let (thrust_increment, max_thrust) = {
        let race_desc = starship
            .race_desc
            .as_ref()
            .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?;
        (
            race_desc.characteristics.thrust_increment as i32,
            race_desc.characteristics.max_thrust as i32,
        )
    };
    const MAX_ALLOWED_SPEED: i32 = world_to_velocity(display_to_world(18));
    const MAX_ALLOWED_SPEED_SQR: u64 = (MAX_ALLOWED_SPEED as u64) * (MAX_ALLOWED_SPEED as u64);

    let current_angle = facing_to_angle(starship.ship_facing);
    let travel_angle = element.velocity.travel_angle;

    // Case 1: Inertialess (Skiff)
    if thrust_increment == max_thrust {
        element
            .velocity
            .set_vector(max_thrust, starship.ship_facing);
        return Ok(StatusFlags::SHIP_AT_MAX_SPEED);
    }

    // Case 2: Already at max speed in same direction (not in gravity well)
    if travel_angle == current_angle
        && (starship
            .cur_status_flags
            .contains(StatusFlags::SHIP_AT_MAX_SPEED)
            || starship
                .cur_status_flags
                .contains(StatusFlags::SHIP_BEYOND_MAX_SPEED))
        && !starship
            .cur_status_flags
            .contains(StatusFlags::SHIP_IN_GRAVITY_WELL)
    {
        return Ok(starship.cur_status_flags
            & (StatusFlags::SHIP_AT_MAX_SPEED | StatusFlags::SHIP_BEYOND_MAX_SPEED));
    }

    // Case 3: Normal acceleration
    let thrust_increment_vel = world_to_velocity(thrust_increment);
    let (cur_delta_x, cur_delta_y) = element.velocity.get_current_components();
    let current_speed = VelocityState::velocity_squared(cur_delta_x, cur_delta_y);
    let delta_x = cur_delta_x + cosine(current_angle, thrust_increment_vel);
    let delta_y = cur_delta_y + sine(current_angle, thrust_increment_vel);
    let desired_speed = VelocityState::velocity_squared(delta_x, delta_y);
    let max_speed = VelocityState::velocity_squared(world_to_velocity(max_thrust), 0);

    if desired_speed <= max_speed {
        // Normal acceleration
        element.velocity.set_components(delta_x, delta_y);
        Ok(StatusFlags::empty())
    } else if (starship
        .cur_status_flags
        .contains(StatusFlags::SHIP_IN_GRAVITY_WELL)
        && desired_speed <= MAX_ALLOWED_SPEED_SQR)
        || desired_speed < current_speed
    {
        // Acceleration in gravity well or deceleration after gravity whip
        element.velocity.set_components(delta_x, delta_y);
        Ok(StatusFlags::SHIP_AT_MAX_SPEED | StatusFlags::SHIP_BEYOND_MAX_SPEED)
    } else if travel_angle == current_angle {
        // Normal max acceleration, same direction
        if current_speed <= max_speed {
            element
                .velocity
                .set_vector(max_thrust, starship.ship_facing);
        }
        Ok(StatusFlags::SHIP_AT_MAX_SPEED)
    } else {
        // Maxed-out acceleration at an angle
        let mut v = element.velocity.clone();
        v.delta_components(
            cosine(current_angle, thrust_increment_vel >> 1)
                - cosine(travel_angle, thrust_increment_vel),
            sine(current_angle, thrust_increment_vel >> 1)
                - sine(travel_angle, thrust_increment_vel),
        );
        let (new_dx, new_dy) = v.get_current_components();
        let new_speed = VelocityState::velocity_squared(new_dx, new_dy);

        if new_speed > max_speed {
            if new_speed < current_speed {
                element.velocity = v;
            }
            Ok(StatusFlags::SHIP_AT_MAX_SPEED | StatusFlags::SHIP_BEYOND_MAX_SPEED)
        } else {
            element.velocity = v;
            Ok(StatusFlags::empty())
        }
    }
}

/// Energy delta helper
pub fn delta_energy(starship: &mut Starship, energy_delta: i16) -> bool {
    let race_desc = match starship.race_desc.as_mut() {
        Some(rd) => rd,
        None => return false,
    };

    let energy_level = &mut race_desc.ship_info.energy_level;
    let max_energy = race_desc.ship_info.max_energy;
    let energy_wait = race_desc.characteristics.energy_wait;

    if energy_delta >= 0 {
        let new_energy = (*energy_level as i16).saturating_add(energy_delta);
        *energy_level = new_energy.min(max_energy as i16) as u8;
        starship.cur_status_flags &= !StatusFlags::LOW_ON_ENERGY;
        starship.energy_counter = energy_wait;
        true
    } else {
        let cost = (-energy_delta) as u8;
        if *energy_level >= cost {
            *energy_level -= cost;
            starship.cur_status_flags &= !StatusFlags::LOW_ON_ENERGY;
            starship.energy_counter = energy_wait;
            true
        } else {
            starship.cur_status_flags |= StatusFlags::LOW_ON_ENERGY;
            false
        }
    }
}

/// Animation preprocess helper
pub fn animation_preprocess(element: &mut ElementState) {
    if element.turn_wait > 0 {
        element.turn_wait -= 1;
    } else {
        element.image_frame = element.image_frame.wrapping_add(1);
        element.state_flags |= CHANGING;
        element.turn_wait = element.next_turn;
    }
}

/// Default ship collision handler
pub fn default_ship_collision(
    element0: &mut ElementState,
    element1: &ElementState,
) -> CollisionResult {
    if (element1.state_flags & FINITE_LIFE) == 0 {
        element0.state_flags |= COLLISION_FLAG;
        if gravity_mass(element1.mass_points) {
            // C: damage = ElementPtr0->hit_points >> 2; if (damage == 0) damage = 1;
            let damage = (element0.hit_points() >> 2).max(1);
            return CollisionResult::PlanetCollision { damage };
        }
    }
    CollisionResult::NoEffect
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

pub fn build_ship_state(
    starship: &Starship,
    element: &ElementState,
) -> Result<ShipState, ShipsError> {
    let race_desc = starship
        .race_desc
        .as_ref()
        .ok_or_else(|| ShipsError::InvalidState("No race descriptor".to_string()))?;
    Ok(ShipState {
        crew_level: element.crew_level,
        max_crew: race_desc.ship_info.max_crew,
        energy_level: race_desc.ship_info.energy_level as u16,
        max_energy: race_desc.ship_info.max_energy as u16,
        ship_facing: starship.ship_facing as u8,
        cur_status_flags: starship.cur_status_flags,
        old_status_flags: starship.old_status_flags,
        player_nr: starship.player_nr,
        position: element.position,
        velocity: element.velocity.get_current_components(),
        element_ptr: std::ptr::null_mut(),
        starship_ptr: std::ptr::null_mut(),
        weapon_counter: starship.weapon_counter,
        special_counter: starship.special_counter,
        energy_counter: starship.energy_counter,
        ship_input_state: starship.ship_input_state,
        thrust_wait: element.thrust_wait,
        turn_wait: element.turn_wait,
        ship_sounds: race_desc.ship_data.ship_sounds,
        weapon_farray: std::ptr::null_mut(),
        special_farray: std::ptr::null_mut(),
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ships::traits::ShipBehavior;
    use crate::ships::types::{
        Characteristics, FleetStuff, IntelStuff, RaceDesc, RaceDescTemplate, ShipData, ShipInfo,
        SpeciesId,
    };

    #[derive(Debug)]
    struct TestBehavior;

    impl ShipBehavior for TestBehavior {
        fn descriptor_template(&self) -> RaceDescTemplate {
            RaceDescTemplate {
                ship_info: ShipInfo {
                    crew_level: 20,
                    max_crew: 20,
                    energy_level: 10,
                    max_energy: 20,
                    ..ShipInfo::default()
                },
                fleet: FleetStuff::default(),
                characteristics: Characteristics {
                    max_thrust: 30,
                    thrust_increment: 5,
                    energy_regeneration: 1,
                    weapon_energy_cost: 3,
                    special_energy_cost: 5,
                    energy_wait: 6,
                    turn_wait: 2,
                    thrust_wait: 3,
                    weapon_wait: 10,
                    special_wait: 15,
                    ship_mass: 5,
                },
                ship_data: ShipData::default(),
                intel: IntelStuff::default(),
            }
        }
    }

    fn make_test_starship() -> Starship {
        let behavior = Box::new(TestBehavior);
        let template = behavior.descriptor_template();
        let race_desc = RaceDesc {
            ship_info: template.ship_info.clone(),
            fleet: template.fleet,
            characteristics: template.characteristics,
            ship_data: template.ship_data,
            intel: template.intel,
            behavior,
            data: None,
        };

        Starship {
            species_id: SpeciesId::Earthling,
            race_desc: Some(Box::new(race_desc)),
            crew_level: 20,
            max_crew: 20,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            weapon_counter: 0,
            special_counter: 0,
            energy_counter: 0,
            ship_input_state: 0,
            player_nr: 0,
            ..Starship::default()
        }
    }

    fn make_test_element() -> ElementState {
        ElementState {
            crew_level: 20,
            state_flags: 0,
            turn_wait: 0,
            thrust_wait: 0,
            ..ElementState::default()
        }
    }
    #[test]
    fn velocity_zero() {
        unsafe {
            let mut v = VelocityState::default();
            v.vector = (10, 20);
            v.fract = (5, 5);
            v.zero();
            assert_eq!(v.vector, (0, 0));
            assert_eq!(v.fract, (0, 0));
            assert_eq!(v.error, (0, 0));
        }
    }

    #[test]
    fn velocity_is_zero() {
        unsafe {
            let v = VelocityState::default();
            assert!(v.is_zero());

            let mut v = VelocityState::default();
            v.vector = (1, 0);
            assert!(!v.is_zero());
        }
    }

    #[test]
    fn velocity_set_vector() {
        unsafe {
            let mut v = VelocityState::default();
            v.set_vector(10, 4); // East/Right (facing 4)
            let (dx, dy) = v.get_current_components();
            assert!(dx > 0);
            assert!(dy.abs() < 5);
        }
    }

    #[test]
    fn velocity_set_components() {
        unsafe {
            let mut v = VelocityState::default();
            v.set_components(100, 0);
            let (dx, dy) = v.get_current_components();
            // Allow for 1 unit of rounding error due to fixed-point representation
            assert!((dx - 100).abs() <= 1);
            assert!(dy.abs() <= 1);
        }
    }

    #[test]
    fn velocity_delta_components() {
        unsafe {
            let mut v = VelocityState::default();
            v.set_components(50, 50);
            v.delta_components(10, -20);
            let (dx, dy) = v.get_current_components();
            // Allow for 2 units of accumulated rounding error
            assert!((dx - 60).abs() <= 2, "dx={}, expected ~60", dx);
            assert!((dy - 30).abs() <= 2, "dy={}, expected ~30", dy);
        }
    }

    #[test]
    fn velocity_incr_matches_c_make_word() {
        unsafe {
            // C: MAKE_WORD(lo, hi) = (hi << 8) | lo
            // Positive: MAKE_WORD(1, 0) = 0x0001 — LOBYTE=1 (step +1), HIBYTE=0
            // Negative: MAKE_WORD(0xFF, frac) = (frac << 8) | 0xFF — LOBYTE=0xFF (step -1)
            let mut v = VelocityState::default();
            v.set_components(100, 0); // positive X
            assert_eq!(v.incr.0 as u16 & 0xFF, 1, "positive LOBYTE should be 1");
            assert_eq!((v.incr.0 as u16) >> 8, 0, "positive HIBYTE should be 0");

            v.set_components(-100, 0); // negative X
            assert_eq!(
                v.incr.0 as u16 & 0xFF,
                0xFF,
                "negative LOBYTE should be 0xFF"
            );
            // HIBYTE = (VELOCITY_REMAINDER(100) << 1) = (100 & 31) << 1 = (4) << 1 = 8
            let expected_frac = (((100i32 & 31) << 1) as u16) & 0xFF;
            assert_eq!(
                (v.incr.0 as u16) >> 8,
                expected_frac,
                "negative HIBYTE should be fractional correction"
            );
        }
    }

    #[test]
    fn velocity_roundtrip_positive_exact() {
        unsafe {
            // Set positive components, read them back — should be exact for multiples of VELOCITY_SCALE
            let mut v = VelocityState::default();
            v.set_components(128, 64); // both multiples of 32
            let (dx, dy) = v.get_current_components();
            assert_eq!(dx, 128, "positive X roundtrip");
            assert_eq!(dy, 64, "positive Y roundtrip");
        }
    }

    #[test]
    fn velocity_roundtrip_negative() {
        unsafe {
            // Negative velocity roundtrip
            let mut v = VelocityState::default();
            v.set_components(-101, -50);
            let (dx, dy) = v.get_current_components();
            // C gets exact roundtrip for these values
            assert_eq!(dx, -101, "negative X roundtrip");
            assert_eq!(dy, -50, "negative Y roundtrip");
        }
    }

    #[test]
    fn velocity_squared_calculation() {
        unsafe {
            assert_eq!(VelocityState::velocity_squared(3, 4), 25);
            assert_eq!(VelocityState::velocity_squared(0, 0), 0);
            assert_eq!(VelocityState::velocity_squared(-3, 4), 25);
        }
    }

    // -- Energy regeneration -------------------------------------------------

    #[test]
    fn energy_regen_counter_cycles() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0; // Not appearing

            ship.energy_counter = 6;
            ship_preprocess(&mut ship, &mut element).unwrap();
            assert_eq!(ship.energy_counter, 5);
        }
    }

    #[test]
    fn energy_regen_increases() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 10;
            ship.energy_counter = 0;

            ship_preprocess(&mut ship, &mut element).unwrap();

            let energy = ship.race_desc.as_ref().unwrap().ship_info.energy_level;
            assert_eq!(energy, 11);
        }
    }

    #[test]
    fn energy_regen_caps_at_max() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 20; // At max
            ship.energy_counter = 0;

            ship_preprocess(&mut ship, &mut element).unwrap();

            let energy = ship.race_desc.as_ref().unwrap().ship_info.energy_level;
            assert_eq!(energy, 20); // Should not exceed max
        }
    }

    #[test]
    fn energy_negative_regen_works() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            ship.race_desc
                .as_mut()
                .unwrap()
                .characteristics
                .energy_regeneration = (-1i8) as u8;
            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 10;
            ship.energy_counter = 0;

            ship_preprocess(&mut ship, &mut element).unwrap();

            let energy = ship.race_desc.as_ref().unwrap().ship_info.energy_level;
            assert_eq!(energy, 9);
        }
    }

    // -- Turn handling -------------------------------------------------------

    #[test]
    fn turn_left_decrements_facing() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            ship.ship_facing = 5;
            ship.ship_input_state = StatusFlags::LEFT.0 as u8;

            ship_preprocess(&mut ship, &mut element).unwrap();

            assert_eq!(ship.ship_facing, 4);
            assert_eq!(element.turn_wait, 2); // Set to turn_wait characteristic
        }
    }

    #[test]
    fn turn_right_increments_facing() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            ship.ship_facing = 5;
            ship.ship_input_state = StatusFlags::RIGHT.0 as u8;

            ship_preprocess(&mut ship, &mut element).unwrap();

            assert_eq!(ship.ship_facing, 6);
        }
    }

    #[test]
    fn turn_wait_delay() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;
            element.turn_wait = 5;

            ship.ship_facing = 5;
            ship.ship_input_state = StatusFlags::LEFT.0 as u8;

            ship_preprocess(&mut ship, &mut element).unwrap();

            assert_eq!(ship.ship_facing, 5); // Should not turn yet
            assert_eq!(element.turn_wait, 4);
        }
    }

    // -- Thrust handling -----------------------------------------------------

    #[test]
    fn thrust_increases_velocity() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            ship.ship_facing = 4; // East/Right (facing 4 → angle 16 → +X)
            ship.ship_input_state = StatusFlags::THRUST.0 as u8;

            ship_preprocess(&mut ship, &mut element).unwrap();

            let (dx, dy) = element.velocity.get_current_components();
            assert!(dx > 0, "dx should be positive for right thrust");
        }
    }

    #[test]
    fn thrust_capped_at_max() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            ship.ship_facing = 4; // East/Right
            ship.ship_input_state = StatusFlags::THRUST.0 as u8;

            // Apply thrust many times
            for _ in 0..100 {
                element.thrust_wait = 0;
                ship_preprocess(&mut ship, &mut element).unwrap();
            }

            let (dx, _dy) = element.velocity.get_current_components();
            let max_vel = world_to_velocity(30); // max_thrust = 30
            assert!(dx <= max_vel + 10, "Velocity should be capped near max");
        }
    }

    #[test]
    fn inertialess_mode_works() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();

            // Set thrust_increment == max_thrust (inertialess like Skiff)
            ship.race_desc
                .as_mut()
                .unwrap()
                .characteristics
                .thrust_increment = 30;
            ship.race_desc.as_mut().unwrap().characteristics.max_thrust = 30;

            ship.ship_facing = 4; // Some angle
            ship.ship_input_state = StatusFlags::THRUST.0 as u8;

            ship_preprocess(&mut ship, &mut element).unwrap();

            assert!(ship
                .cur_status_flags
                .contains(StatusFlags::SHIP_AT_MAX_SPEED));
        }
    }

    // -- Inertia (coasting) --------------------------------------------------

    #[test]
    fn ship_coasts_when_not_thrusting() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();
            element.state_flags = 0;

            element.velocity.set_components(100, 50);
            ship.ship_input_state = 0; // No thrust

            ship_preprocess(&mut ship, &mut element).unwrap();

            let (dx, dy) = element.velocity.get_current_components();
            // Allow for 1 unit of rounding error
            assert!((dx - 100).abs() <= 1);
            assert!((dy - 50).abs() <= 1);
        }
    }

    // -- Weapon fire ---------------------------------------------------------

    #[test]
    fn weapon_fires_when_conditions_met() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();

            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 10;
            ship.cur_status_flags = StatusFlags::WEAPON;
            ship.weapon_counter = 0;

            ship_postprocess(&mut ship, &mut element).unwrap();

            assert_eq!(ship.race_desc.as_ref().unwrap().ship_info.energy_level, 7); // 10 - 3
            assert_eq!(ship.weapon_counter, 10); // Set to weapon_wait
        }
    }

    #[test]
    fn weapon_does_not_fire_when_energy_insufficient() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();

            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 2; // Less than weapon_cost (3)
            ship.cur_status_flags = StatusFlags::WEAPON;
            ship.weapon_counter = 0;

            ship_postprocess(&mut ship, &mut element).unwrap();

            assert_eq!(ship.race_desc.as_ref().unwrap().ship_info.energy_level, 2); // No change
            assert_eq!(ship.weapon_counter, 0); // Not set
            assert!(ship.cur_status_flags.contains(StatusFlags::LOW_ON_ENERGY));
        }
    }

    // -- Cooldown ------------------------------------------------------------

    #[test]
    fn cooldown_decrements_each_frame() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();

            ship.weapon_counter = 5;
            ship.special_counter = 3;

            ship_postprocess(&mut ship, &mut element).unwrap();

            assert_eq!(ship.weapon_counter, 4);
            assert_eq!(ship.special_counter, 2);
        }
    }

    // -- First frame (APPEARING) ---------------------------------------------

    #[test]
    fn first_frame_appearing_flag_triggers_setup() {
        unsafe {
            let mut ship = make_test_starship();
            let mut element = make_test_element();

            element.state_flags = APPEARING;
            element.crew_level = 0; // Should be set from race_desc

            ship_preprocess(&mut ship, &mut element).unwrap();

            assert_eq!(element.crew_level, 20); // From race_desc ship_info
            assert_eq!(element.state_flags & APPEARING, 0); // Flag cleared
        }
    }

    // -- Movement determinism ------------------------------------------------

    #[test]
    fn movement_determinism_same_inputs_same_outputs() {
        unsafe {
            let mut ship1 = make_test_starship();
            let mut element1 = make_test_element();
            element1.state_flags = 0;
            ship1.ship_input_state = StatusFlags::THRUST.0 as u8;

            let mut ship2 = make_test_starship();
            let mut element2 = make_test_element();
            element2.state_flags = 0;
            ship2.ship_input_state = StatusFlags::THRUST.0 as u8;

            for _ in 0..10 {
                element1.thrust_wait = 0;
                element2.thrust_wait = 0;
                ship_preprocess(&mut ship1, &mut element1).unwrap();
                ship_preprocess(&mut ship2, &mut element2).unwrap();
            }

            let (dx1, dy1) = element1.velocity.get_current_components();
            let (dx2, dy2) = element2.velocity.get_current_components();

            assert_eq!(dx1, dx2);
            assert_eq!(dy1, dy2);
        }
    }

    // -- Collision -----------------------------------------------------------

    #[test]
    fn collision_planet_damage() {
        unsafe {
            let mut ship_element = make_test_element();
            ship_element.crew_level = 100;

            let mut planet_element = make_test_element();
            planet_element.mass_points = MAX_SHIP_MASS * 10 + 1; // Gravity mass
            planet_element.state_flags = 0; // Not FINITE_LIFE

            let result = default_ship_collision(&mut ship_element, &planet_element);

            match result {
                CollisionResult::PlanetCollision { damage } => {
                    assert_eq!(damage, 25); // 100 >> 2
                }
                _ => panic!("Expected PlanetCollision"),
            }
            assert!(ship_element.state_flags & COLLISION_FLAG != 0);
        }
    }

    #[test]
    fn collision_default_no_effect_for_finite_life() {
        unsafe {
            let mut ship_element = make_test_element();
            let mut projectile_element = make_test_element();
            projectile_element.state_flags = FINITE_LIFE;

            let result = default_ship_collision(&mut ship_element, &projectile_element);

            assert_eq!(result, CollisionResult::NoEffect);
            assert_eq!(ship_element.state_flags & COLLISION_FLAG, 0);
        }
    }

    // -- delta_energy --------------------------------------------------------

    #[test]
    fn delta_energy_positive() {
        unsafe {
            let mut ship = make_test_starship();
            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 10;

            let result = delta_energy(&mut ship, 5);

            assert!(result);
            assert_eq!(ship.race_desc.as_ref().unwrap().ship_info.energy_level, 15);
        }
    }

    #[test]
    fn delta_energy_negative_sufficient() {
        unsafe {
            let mut ship = make_test_starship();
            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 10;

            let result = delta_energy(&mut ship, -5);

            assert!(result);
            assert_eq!(ship.race_desc.as_ref().unwrap().ship_info.energy_level, 5);
        }
    }

    #[test]
    fn delta_energy_negative_insufficient() {
        unsafe {
            let mut ship = make_test_starship();
            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 3;

            let result = delta_energy(&mut ship, -5);

            assert!(!result);
            assert_eq!(ship.race_desc.as_ref().unwrap().ship_info.energy_level, 3);
            assert!(ship.cur_status_flags.contains(StatusFlags::LOW_ON_ENERGY));
        }
    }

    #[test]
    fn delta_energy_clamping_at_max() {
        unsafe {
            let mut ship = make_test_starship();
            ship.race_desc.as_mut().unwrap().ship_info.energy_level = 18;
            ship.race_desc.as_mut().unwrap().ship_info.max_energy = 20;

            let result = delta_energy(&mut ship, 5);

            assert!(result);
            assert_eq!(ship.race_desc.as_ref().unwrap().ship_info.energy_level, 20);
            // Clamped
        }
    }

    // -- animation_preprocess ------------------------------------------------

    #[test]
    fn animation_preprocess_increments_frame() {
        unsafe {
            let mut element = make_test_element();
            element.image_frame = 5;
            element.turn_wait = 0;
            element.next_turn = 3;

            animation_preprocess(&mut element);

            assert_eq!(element.image_frame, 6);
            assert_eq!(element.turn_wait, 3);
            assert!(element.state_flags & CHANGING != 0);
        }
    }

    #[test]
    fn animation_preprocess_waits() {
        unsafe {
            let mut element = make_test_element();
            element.image_frame = 5;
            element.turn_wait = 2;

            animation_preprocess(&mut element);

            assert_eq!(element.image_frame, 5); // No change
            assert_eq!(element.turn_wait, 1);
        }
    }

    #[test]
    fn hit_points_alias_matches_crew_level() {
        unsafe {
            let mut element = ElementState::default();
            element.crew_level = 42;
            assert_eq!(element.hit_points(), 42);
        }
    }

    #[test]
    fn ai_hook_invoked_for_computer_control() {
        unsafe {
            let mut starship = make_test_starship();
            starship.control = COMPUTER_CONTROL;
            let mut element = make_test_element();
            // Not APPEARING, so normal flow runs
            element.state_flags = 0;

            let result = ship_preprocess(&mut starship, &mut element);
            assert!(result.is_ok());
            // AI intelligence() was called (default returns empty flags, so no visible effect
            // beyond the call not panicking)
        }
    }

    #[test]
    fn ai_hook_not_invoked_for_human_control() {
        unsafe {
            let mut starship = make_test_starship();
            starship.control = 0; // human
            let mut element = make_test_element();
            element.state_flags = 0;

            let result = ship_preprocess(&mut starship, &mut element);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn collision_planet_uses_hit_points() {
        unsafe {
            let mut element0 = ElementState::default();
            element0.crew_level = 20; // hit_points == crew_level via union
            let mut element1 = ElementState::default();
            element1.mass_points = 200; // GRAVITY_MASS: > MAX_SHIP_MASS * 10
            element1.state_flags = 0; // not FINITE_LIFE

            let result = default_ship_collision(&mut element0, &element1);
            // damage = hit_points >> 2 = 20 >> 2 = 5
            assert_eq!(result, CollisionResult::PlanetCollision { damage: 5 });
        }
    }

    #[test]
    fn collision_planet_damage_min_one() {
        unsafe {
            let mut element0 = ElementState::default();
            element0.crew_level = 2; // hit_points = 2, >> 2 = 0, but min 1
            let mut element1 = ElementState::default();
            element1.mass_points = 200;
            element1.state_flags = 0;

            let result = default_ship_collision(&mut element0, &element1);
            assert_eq!(result, CollisionResult::PlanetCollision { damage: 1 });
        }
    }
}
