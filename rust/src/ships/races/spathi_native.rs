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
    use crate::collections::observation::image;
    use crate::ships::native::{abi::*, element, frame_index, put_element, starship};

    // Core characteristics
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

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE | FIRES_AFT | SEEKING_SPECIAL | DONT_CHASE) as u16;
        d.ship_info.ship_cost = 18;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.spathi.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.spathi.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.spathi.meleeicons".as_ptr();
        d.fleet.strength = 1000 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet.known_loc = POINT { x: 2549, y: 3600 };
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
        d.ship_data.ship_rsc = [
            c"ship.spathi.graphics.eluder.large".as_ptr(),
            c"ship.spathi.graphics.eluder.medium".as_ptr(),
            c"ship.spathi.graphics.eluder.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.spathi.graphics.missile.large".as_ptr(),
            c"ship.spathi.graphics.missile.medium".as_ptr(),
            c"ship.spathi.graphics.missile.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.spathi.graphics.butt.large".as_ptr(),
            c"ship.spathi.graphics.butt.medium".as_ptr(),
            c"ship.spathi.graphics.butt.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.spathi.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.spathi.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.spathi.sounds".as_ptr();
        d.cyborg_control.WeaponRange = MISSILE_RANGE;
        d.cyborg_control.intelligence_func = Some(spathi_intelligence);
        d.postprocess_func = Some(spathi_postprocess);
        d.init_weapon_func = Some(initialize_standard_missile);
        d
    }

    static mut SPATHI_DESC: RACE_DESC = descriptor();

    /// Returns the static template copied by CaptureCodeRes for each ship.
    #[no_mangle]
    pub extern "C" fn init_spathi() -> *mut RACE_DESC {
        &raw mut SPATHI_DESC
    }

    pub(crate) unsafe extern "C" fn butt_missile_preprocess(missile: *mut ELEMENT) {
        if (*missile).__bindgen_anon_4.turn_wait > 0 {
            (*missile).__bindgen_anon_4.turn_wait -= 1;
        } else {
            let mut facing = frame_index((*missile).next.image.frame);
            if TrackShip(missile, &mut facing) > 0 {
                image!(frame; (*missile).next.image.frame, SetAbsFrameIndex((*missile).next.image.frame, facing));
                (*missile).state_flags |= CHANGING as u16;

                SetVelocityVector(&raw mut (*missile).velocity, DISCRIMINATOR_SPEED, facing);
            }

            (*missile).__bindgen_anon_4.turn_wait = TRACK_WAIT;
        }
    }

    pub(crate) unsafe extern "C" fn spawn_butt_missile(ship: *mut ELEMENT) {
        let ss = starship(ship);
        let race = (*ss).RaceDescPtr;
        let face = normalize_facing((*ss).ShipFacing as i32 + HALF_FACING);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            flags: 0,
            sender: (*ship).playerNr,
            pixoffs: SPATHI_REAR_OFFSET,
            speed: DISCRIMINATOR_SPEED,
            hit_points: DISCRIMINATOR_HITS,
            damage: DISCRIMINATOR_DAMAGE,
            face,
            index: face,
            life: DISCRIMINATOR_LIFE,
            farray: (&raw mut (*race).ship_data.special).cast::<FRAME>(),
            preprocess_func: Some(ElementHook::Legacy(butt_missile_preprocess)),
            blast_offs: DISCRIMINATOR_OFFSET,
        };
        let butt = initialize_missile(&mut block);
        if !butt.is_null() {
            let butt_ptr = element(butt);
            (*butt_ptr).__bindgen_anon_4.turn_wait = TRACK_WAIT;
            (*butt_ptr).pParent = ss.cast();

            // LAUNCH_BUTT_MISSILE
            ProcessSound(
                SetAbsStringTableIndex((*race).ship_data.ship_sounds, 1),
                butt_ptr,
            );

            put_element(butt);
        }
    }

    pub(crate) unsafe extern "C" fn spathi_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        ship_intelligence(ship, concerns, count);

        let ss = starship(ship);
        (*ss).ship_input_state &= !(SPECIAL as u8);

        let eval = concerns.add(ENEMY_SHIP_INDEX as usize);
        if (*ss).special_counter == 0 && !(*eval).ObjectPtr.is_null() && (*eval).which_turn <= 24 {
            let target = (*eval).ObjectPtr;
            let travel_facing = normalize_facing(angle_to_facing(
                i32::from((*ship).velocity.travel_angle) + HALF_CIRCLE as i32,
            ));
            // SIZE locals: the int difference is truncated to 16 bits.
            let delta_x = (i32::from((*target).current.location.x)
                - i32::from((*ship).current.location.x)) as SIZE;
            let delta_y = (i32::from((*target).current.location.y)
                - i32::from((*ship).current.location.y)) as SIZE;
            let direction_facing =
                normalize_facing(angle_to_facing(i32::from(ARCTAN(delta_x, delta_y))));

            if within_half_circle(direction_facing, i32::from((*ss).ShipFacing) + HALF_FACING)
                && ((*eval).which_turn <= 8
                    || within_half_circle(
                        direction_facing,
                        angle_to_facing(i32::from((*target).velocity.travel_angle)) - HALF_FACING,
                    ))
                && ((*ss).cur_status_flags & (SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16
                    == 0
                    || within_half_circle(direction_facing, i32::from(travel_facing)))
            {
                (*ss).ship_input_state |= SPECIAL as u8;
            }
        }
    }

    pub(crate) unsafe extern "C" fn initialize_standard_missile(
        ship: *mut ELEMENT,
        weapons: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            flags: IGNORE_SIMILAR as u16,
            sender: (*ship).playerNr,
            pixoffs: SPATHI_FORWARD_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            life: MISSILE_LIFE,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast::<FRAME>(),
            preprocess_func: None,
            blast_offs: MISSILE_OFFSET,
        };
        *weapons = initialize_missile(&mut block);

        1
    }

    pub(crate) unsafe extern "C" fn spathi_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && DeltaEnergy(ship, -(SPECIAL_ENERGY_COST as SIZE)) != 0
        {
            spawn_butt_missile(ship);

            (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
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
