//! Supox callbacks used by the live battle engine.
//!
//! Every value and expression below mirrors `sc2/src/uqm/ships/supox/supox.c`,
//! including C integer promotion and truncation, so battles replay identically.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::{abi::*, starship};

    // Core characteristics
    const MAX_CREW: u16 = 12;
    const MAX_ENERGY: u8 = 16;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const MAX_THRUST: u16 = 40;
    const THRUST_INCREMENT: u16 = 8;
    const THRUST_WAIT: u8 = 0;
    const TURN_WAIT: u8 = 1;
    const SHIP_MASS: u8 = 4;

    // Gob launcher
    const WEAPON_ENERGY_COST: u8 = 1;
    const WEAPON_WAIT: u8 = 2;
    const SUPOX_OFFSET: i16 = 23;
    const MISSILE_OFFSET: i16 = 2;
    /// DISPLAY_TO_WORLD (30)
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 10;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 1;

    // Lateral/reverse thrust
    const SPECIAL_ENERGY_COST: u8 = 1;
    const SPECIAL_WAIT: u8 = 0;

    const FULL_CIRCLE_ANGLE: i32 = FULL_CIRCLE as i32;
    const HALF_CIRCLE_ANGLE: i32 = HALF_CIRCLE as i32;
    const QUADRANT_ANGLE: i32 = QUADRANT as i32;
    const OCTANT_ANGLE: i32 = OCTANT as i32;
    const FACING_MASK: i32 = (1 << FACING_SHIFT) - 1;

    /// units.h ANGLE_TO_FACING; evaluated in `int` like the C macro.
    const fn angle_to_facing(angle: i32) -> i32 {
        (angle + (1 << (CIRCLE_SHIFT - FACING_SHIFT - 1))) >> (CIRCLE_SHIFT - FACING_SHIFT)
    }

    /// units.h FACING_TO_ANGLE
    const fn facing_to_angle(facing: i32) -> i32 {
        facing << (CIRCLE_SHIFT - FACING_SHIFT)
    }

    /// units.h NORMALIZE_ANGLE
    const fn normalize_angle(angle: i32) -> i32 {
        angle & (FULL_CIRCLE_ANGLE - 1)
    }

    /// units.h NORMALIZE_FACING
    const fn normalize_facing(facing: i32) -> u16 {
        (facing & FACING_MASK) as u16
    }

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut desc = RACE_DESC::EMPTY;
        desc.ship_info.ship_flags = FIRES_FORE as u16;
        desc.ship_info.ship_cost = 16;
        desc.ship_info.crew_level = MAX_CREW;
        desc.ship_info.max_crew = MAX_CREW;
        desc.ship_info.energy_level = MAX_ENERGY;
        desc.ship_info.max_energy = MAX_ENERGY;
        desc.ship_info.race_strings_rsc = c"ship.supox.text".as_ptr();
        desc.ship_info.icons_rsc = c"ship.supox.icons".as_ptr();
        desc.ship_info.melee_icon_rsc = c"ship.supox.meleeicons".as_ptr();
        // Initial SoI radius: 333 / SPHERE_RADIUS_INCREMENT * 2
        desc.fleet.strength = (333 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        desc.fleet.known_loc = POINT { x: 7468, y: 9246 };
        desc.characteristics = CHARACTERISTIC_STUFF {
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
        desc.ship_data.ship_rsc = [
            c"ship.supox.graphics.blade.large".as_ptr(),
            c"ship.supox.graphics.blade.medium".as_ptr(),
            c"ship.supox.graphics.blade.small".as_ptr(),
        ];
        desc.ship_data.weapon_rsc = [
            c"ship.supox.graphics.glob.large".as_ptr(),
            c"ship.supox.graphics.glob.medium".as_ptr(),
            c"ship.supox.graphics.glob.small".as_ptr(),
        ];
        // special_rsc stays NULL_RESOURCE for all three views.
        desc.ship_data.captain_control.captain_rsc = c"ship.supox.graphics.captain".as_ptr();
        desc.ship_data.victory_ditty_rsc = c"ship.supox.ditty".as_ptr();
        desc.ship_data.ship_sounds_rsc = c"ship.supox.sounds".as_ptr();
        desc.cyborg_control.ManeuverabilityIndex = 0;
        desc.cyborg_control.WeaponRange =
            ((MISSILE_SPEED as i32 * MISSILE_LIFE as i32) >> 1) as u16;
        desc.cyborg_control.intelligence_func = Some(supox_intelligence);
        desc.preprocess_func = Some(supox_preprocess);
        desc.init_weapon_func = Some(initialize_horn);
        desc
    }

    static mut SUPOX_DESC: RACE_DESC = descriptor();

    /// Returns the static template the engine copies for each Supox ship.
    #[no_mangle]
    pub extern "C" fn init_supox() -> *mut RACE_DESC {
        &raw mut SUPOX_DESC
    }

    pub(crate) unsafe extern "C" fn supox_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);

        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if (*ss).special_counter != 0 || (*enemy).ObjectPtr.is_null() {
            (*ss).ship_input_state &= !(SPECIAL as u8);
        } else {
            let target = (*enemy).ObjectPtr;
            let delta_x =
                (i32::from((*target).next.location.x) - i32::from((*ship).next.location.x)) as SIZE;
            let delta_y =
                (i32::from((*target).next.location.y) - i32::from((*ship).next.location.y)) as SIZE;
            let direction_angle = ARCTAN(delta_x, delta_y);

            let ship_angle = facing_to_angle(i32::from((*ss).ShipFacing));
            let lined_up = normalize_angle(
                normalize_angle(i32::from(direction_angle) - ship_angle) + QUADRANT_ANGLE,
            ) <= HALF_CIRCLE_ANGLE;

            if !lined_up
                || (*enemy).which_turn > 20
                || normalize_angle(
                    i32::from((*enemy).facing) - (ship_angle + HALF_CIRCLE_ANGLE) + OCTANT_ANGLE,
                ) > QUADRANT_ANGLE
            {
                (*ss).ship_input_state &= !(SPECIAL as u8);
            } else if lined_up && (*enemy).which_turn <= 12 {
                (*ss).ship_input_state |= SPECIAL as u8;
            }

            if (*ss).ship_input_state & SPECIAL as u8 != 0 {
                (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
            }
        }

        ship_intelligence(ship, concerns, count);

        if (*ss).ship_input_state & SPECIAL as u8 != 0 {
            (*ss).ship_input_state |= (THRUST | WEAPON) as u8;
        }

        let enemy_weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if (*ss).special_counter == 0
            && !(*enemy_weapon).ObjectPtr.is_null()
            && (*enemy_weapon).MoveState == MOVEMENT_STATE_AVOID
            && (*ship).__bindgen_anon_4.turn_wait == 0
        {
            (*ss).ship_input_state &= !(THRUST as u8);
            (*ss).ship_input_state |= SPECIAL as u8;
            if (*ss).cur_status_flags & (LEFT | RIGHT) as u16 == 0 {
                (*ss).ship_input_state |= 1u8 << (TFB_Random() as u8 & 1);
            } else {
                (*ss).ship_input_state |= ((*ss).cur_status_flags & (LEFT | RIGHT) as u16) as u8;
            }
        }
    }

    pub(crate) unsafe extern "C" fn initialize_horn(
        ship: *mut ELEMENT,
        horns: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: SUPOX_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess_func: None,
            blast_offs: MISSILE_OFFSET,
        };
        *horns = initialize_missile(&mut block);
        1
    }

    pub(crate) unsafe extern "C" fn supox_preprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & SPECIAL as u16 == 0 {
            return;
        }

        let mut add_facing: i32 = 0;
        if (*ss).cur_status_flags & THRUST as u16 != 0 {
            if (*ship).__bindgen_anon_5.thrust_wait == 0 {
                (*ship).__bindgen_anon_5.thrust_wait += 1;
            }

            add_facing = angle_to_facing(HALF_CIRCLE_ANGLE);
        }
        if (*ss).cur_status_flags & LEFT as u16 != 0 {
            if (*ship).__bindgen_anon_4.turn_wait == 0 {
                (*ship).__bindgen_anon_4.turn_wait += 1;
            }

            if add_facing != 0 {
                add_facing += angle_to_facing(OCTANT_ANGLE);
            } else {
                add_facing = -angle_to_facing(QUADRANT_ANGLE);
            }
        } else if (*ss).cur_status_flags & RIGHT as u16 != 0 {
            if (*ship).__bindgen_anon_4.turn_wait == 0 {
                (*ship).__bindgen_anon_4.turn_wait += 1;
            }

            if add_facing != 0 {
                add_facing -= angle_to_facing(OCTANT_ANGLE);
            } else {
                add_facing = angle_to_facing(QUADRANT_ANGLE);
            }
        }

        if add_facing != 0 {
            let facing = (*ss).ShipFacing;
            (*ss).ShipFacing = normalize_facing(i32::from(facing) + add_facing);
            let thrust_status = inertial_thrust(ship);
            (*ss).cur_status_flags &=
                !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16);
            (*ss).cur_status_flags |= thrust_status;
            (*ss).ShipFacing = facing;
        }
    }
}
