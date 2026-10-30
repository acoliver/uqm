//! Spathi Eluder callbacks and descriptor for the live engine.
//!
//! Ported from `sc2/src/uqm/ships/spathi/spathi.c`. Macro arithmetic from
//! `units.h` is translated here with the same `int` promotion the C macros
//! get, so facing math wraps exactly as the original does.

use crate::ships::native::abi::{CIRCLE_SHIFT, FACING_SHIFT, HALF_CIRCLE, QUADRANT};

/// `ANGLE_TO_FACING`: rounds an angle to the nearest of sixteen facings.
const fn angle_to_facing(angle: i32) -> i32 {
    (angle + (1 << (CIRCLE_SHIFT - FACING_SHIFT - 1))) >> (CIRCLE_SHIFT - FACING_SHIFT)
}

/// `NORMALIZE_FACING`: the macro masks an `int` and casts the result to COUNT.
const fn normalize_facing(facing: i32) -> u16 {
    (facing & ((1 << FACING_SHIFT) - 1)) as u16
}

/// `ANGLE_TO_FACING (HALF_CIRCLE)`, the facing directly behind a ship.
const HALF_FACING: i32 = angle_to_facing(HALF_CIRCLE as i32);
/// `ANGLE_TO_FACING (QUADRANT)`.
const QUARTER_FACING: i32 = angle_to_facing(QUADRANT as i32);

/// The shared shape of the three terms in the original intelligence test:
/// `NORMALIZE_FACING (direction_facing - subtracted + ANGLE_TO_FACING (QUADRANT))
/// <= ANGLE_TO_FACING (HALF_CIRCLE)`.
const fn within_half_circle(direction_facing: u16, subtracted: i32) -> bool {
    normalize_facing(direction_facing as i32 - subtracted + QUARTER_FACING) as i32 <= HALF_FACING
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::ships::native::abi::*;

    // Core characteristics
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static SPATHI: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: Some(spathi_postprocess),
        init_weapon: Some(initialize_standard_missile),
        intelligence: Some(spathi_intelligence),
        uninit: None,
    };
    const MAX_CREW: u16 = 30;
    const MAX_ENERGY: u8 = 10;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 10;
    const MAX_THRUST: u16 = 48;
    const THRUST_INCREMENT: u16 = 12;
    const THRUST_WAIT: u8 = 1;
    const TURN_WAIT: u8 = 1;
    const SHIP_MASS: u8 = 5;

    // Forward gun
    const WEAPON_ENERGY_COST: u8 = 2;
    const WEAPON_WAIT: u8 = 0;
    const SPATHI_FORWARD_OFFSET: i16 = 16;
    const MISSILE_SPEED: i16 = display_to_world(30) as i16;
    const MISSILE_LIFE: u16 = 10;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 1;
    const MISSILE_OFFSET: i16 = 1;
    /// Cyborg-only weapon range, `MISSILE_SPEED * MISSILE_LIFE`.
    const MISSILE_RANGE: u16 = MISSILE_SPEED as u16 * MISSILE_LIFE;

    // B.U.T.T.
    const SPECIAL_ENERGY_COST: u8 = 3;
    const SPECIAL_WAIT: u8 = 7;
    const SPATHI_REAR_OFFSET: i16 = 20;
    const DISCRIMINATOR_SPEED: i16 = display_to_world(8) as i16;
    const DISCRIMINATOR_LIFE: u16 = 30;
    const DISCRIMINATOR_HITS: i16 = 1;
    const DISCRIMINATOR_DAMAGE: i16 = 2;
    const DISCRIMINATOR_OFFSET: i16 = 4;
    const TRACK_WAIT: u8 = 1;

    /// `DISPLAY_TO_WORLD`.
    const fn display_to_world(x: i32) -> i32 {
        x << ONE_SHIFT
    }

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (FIRES_FORE | FIRES_AFT | SEEKING_SPECIAL | DONT_CHASE) as u16;
        d.info.ship_cost = 18;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.spathi.text");
        d.resources.icons = Some(c"ship.spathi.icons");
        d.resources.melee_icon = Some(c"ship.spathi.meleeicons");
        d.fleet_strength = 1000 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet_location = POINT { x: 2549, y: 3600 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: ENERGY_REGENERATION,
            weapon_energy_cost: WEAPON_ENERGY_COST,
            special_energy_cost: SPECIAL_ENERGY_COST,
            energy_wait: ENERGY_WAIT,
            turn_wait: TURN_WAIT,
            thrust_wait: THRUST_WAIT,
            weapon_wait: WEAPON_WAIT,
            special_wait: SPECIAL_WAIT,
            ship_mass: SHIP_MASS,
        };
        d.resources.ship = [
            Some(c"ship.spathi.graphics.eluder.large"),
            Some(c"ship.spathi.graphics.eluder.medium"),
            Some(c"ship.spathi.graphics.eluder.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.spathi.graphics.missile.large"),
            Some(c"ship.spathi.graphics.missile.medium"),
            Some(c"ship.spathi.graphics.missile.small"),
        ];
        d.resources.special = [
            Some(c"ship.spathi.graphics.butt.large"),
            Some(c"ship.spathi.graphics.butt.medium"),
            Some(c"ship.spathi.graphics.butt.small"),
        ];
        d.resources.captain = Some(c"ship.spathi.graphics.captain");
        d.resources.victory = Some(c"ship.spathi.ditty");
        d.resources.sounds = Some(c"ship.spathi.sounds");
        d.intelligence.weapon_range = MISSILE_RANGE;
        d.descriptor(&SPATHI)
    }
    pub(crate) fn butt_missile_preprocess(cx: &mut Cx<'_>, missile: ElementId) {
        if cx.element(missile).turn > 0 {
            cx.element_mut(missile).turn -= 1;
        } else {
            let mut facing = cx.frame_number(cx.element(missile).next.image.frame);
            if cx.track_ship(missile, &mut facing) > 0 {
                cx.set_frame(
                    missile,
                    true,
                    cx.absolute_frame(cx.element(missile).next.image.frame, facing),
                );
                cx.element_mut(missile).flags |= CHANGING as u16;

                cx.element_mut(missile)
                    .velocity
                    .set_vector(i32::from((DISCRIMINATOR_SPEED) as SIZE), facing);
            }

            cx.element_mut(missile).turn = TRACK_WAIT;
        }
    }

    pub(crate) fn spawn_butt_missile(cx: &mut Cx<'_>, ship: ElementId) {
        let ss = cx.owner(ship);
        let face = normalize_facing(cx.ship_state(ss).facing as i32 + HALF_FACING);
        let mut block = Missile {
            cx: cx.element(ship).next.location.x,
            cy: cx.element(ship).next.location.y,
            flags: 0,
            sender: cx.element(ship).player,
            pixoffs: SPATHI_REAR_OFFSET,
            speed: DISCRIMINATOR_SPEED,
            hit_points: DISCRIMINATOR_HITS,
            damage: DISCRIMINATOR_DAMAGE,
            face,
            index: face,
            life: DISCRIMINATOR_LIFE,
            farray: cx.ship_frame_set(ss, 2),
            preprocess: Some(butt_missile_preprocess),
            blast_offs: DISCRIMINATOR_OFFSET,
        };
        if let Some(butt_ptr) = cx.missile(&mut block) {
            cx.element_mut(butt_ptr).turn = TRACK_WAIT;
            cx.set_parent(butt_ptr, Some(ss));

            // LAUNCH_BUTT_MISSILE
            cx.process_ship_sound(ss, 1, butt_ptr);

            cx.put(butt_ptr);
        }
    }

    pub(crate) fn spathi_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.ship_intelligence(ss, concerns);

        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);

        let eval = ENEMY_SHIP_INDEX as usize;
        if cx.ship_state(ss).special_counter == 0
            && concerns.objects[eval].object.is_some()
            && concerns.objects[eval].which_turn <= 24
        {
            let target = concerns.objects[eval]
                .object
                .unwrap_or_else(|| panic!("enemy ship concern"));
            let travel_facing = normalize_facing(angle_to_facing(
                i32::from(concerns.subject.velocity.travel_angle) + HALF_CIRCLE as i32,
            ));
            // SIZE locals: the int difference is truncated to 16 bits.
            let delta_x = (i32::from(cx.element(target).current.location.x)
                - i32::from(concerns.subject.current.location.x)) as SIZE;
            let delta_y = (i32::from(cx.element(target).current.location.y)
                - i32::from(concerns.subject.current.location.y)) as SIZE;
            let direction_facing = normalize_facing(angle_to_facing(i32::from(
                crate::math::arctan(delta_x, delta_y),
            )));

            if within_half_circle(
                direction_facing,
                i32::from(cx.ship_state(ss).facing) + HALF_FACING,
            ) && (concerns.objects[eval].which_turn <= 8
                || within_half_circle(
                    direction_facing,
                    angle_to_facing(i32::from(cx.element(target).velocity.travel_angle))
                        - HALF_FACING,
                ))
                && (cx.ship_state(ss).status
                    & (SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16
                    == 0
                    || within_half_circle(direction_facing, i32::from(travel_facing)))
            {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
        }
    }

    pub(crate) fn initialize_standard_missile(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            flags: IGNORE_SIMILAR as u16,
            sender: origin.player,
            pixoffs: SPATHI_FORWARD_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            face: cx.ship_state(ss).facing,
            index: cx.ship_state(ss).facing,
            life: MISSILE_LIFE,
            farray: cx.ship_frame_set(ss, 1),
            preprocess: None,
            blast_offs: MISSILE_OFFSET,
        };
        vec![cx.missile(&mut block)]
    }

    pub(crate) fn spathi_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(ship, -(SPECIAL_ENERGY_COST as SIZE))
        {
            spawn_butt_missile(cx, ship);

            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rear_facing_is_half_a_turn_of_sixteen_facings() {
        assert_eq!(HALF_FACING, 8);
        assert_eq!(QUARTER_FACING, 4);
        assert_eq!(normalize_facing(3 + HALF_FACING), 11);
        assert_eq!(normalize_facing(12 + HALF_FACING), 4);
    }

    #[test]
    fn angle_to_facing_rounds_to_nearest_facing() {
        assert_eq!(angle_to_facing(0), 0);
        assert_eq!(angle_to_facing(1), 0);
        assert_eq!(angle_to_facing(2), 1);
        assert_eq!(angle_to_facing(63), 16);
        assert_eq!(normalize_facing(angle_to_facing(63)), 0);
    }

    #[test]
    fn normalize_facing_wraps_negative_values_like_the_c_macro() {
        assert_eq!(normalize_facing(-1), 15);
        assert_eq!(normalize_facing(-17), 15);
    }

    #[test]
    fn butt_gate_accepts_targets_behind_the_ship() {
        // Ship facing 0: the rear is facing 8, so a target at 8 is in range.
        assert!(within_half_circle(8, HALF_FACING));
        // Directly ahead lies outside the half circle behind the ship.
        assert!(!within_half_circle(0, HALF_FACING));
    }
}
