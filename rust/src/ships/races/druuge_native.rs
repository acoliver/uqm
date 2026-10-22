//! Druuge callbacks registered with the live battle engine.

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::{
        abi::*, cosine, element, facing_to_angle, frame_index, sine, starship,
    };
    use std::{mem, ptr};

    const MAX_CREW: u16 = 14;
    const MAX_ENERGY: u8 = 32;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 50;
    const MAX_THRUST: u16 = 20;
    const THRUST_INCREMENT: u16 = 2;
    const THRUST_WAIT: u8 = 1;
    const TURN_WAIT: u8 = 4;
    const SHIP_MASS: u8 = 5;
    const WEAPON_ENERGY_COST: u8 = 4;
    const WEAPON_WAIT: u8 = 10;
    const DRUUGE_OFFSET: i16 = 24;
    const MISSILE_OFFSET: i16 = 6;
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 20;
    const MISSILE_RANGE: i32 = MISSILE_SPEED as i32 * MISSILE_LIFE as i32;
    const MISSILE_HITS: i16 = 4;
    const MISSILE_DAMAGE: i16 = 6;
    const RECOIL_VELOCITY: i32 = (6 << ONE_SHIFT) << VELOCITY_SHIFT;
    const MAX_RECOIL_VELOCITY: i32 = RECOIL_VELOCITY * 4;
    const SPECIAL_ENERGY_COST: u8 = 16;
    const SPECIAL_WAIT: u8 = 30;

    const fn descriptor() -> RACE_DESC {
        let mut desc: RACE_DESC = unsafe { mem::zeroed() };
        desc.ship_info.ship_flags = FIRES_FORE as u16;
        desc.ship_info.ship_cost = 17;
        desc.ship_info.crew_level = MAX_CREW;
        desc.ship_info.max_crew = MAX_CREW;
        desc.ship_info.energy_level = MAX_ENERGY;
        desc.ship_info.max_energy = MAX_ENERGY;
        desc.ship_info.race_strings_rsc = c"ship.druuge.text".as_ptr();
        desc.ship_info.icons_rsc = c"ship.druuge.icons".as_ptr();
        desc.ship_info.melee_icon_rsc = c"ship.druuge.meleeicons".as_ptr();
        desc.fleet.strength = (1400 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        desc.fleet.known_loc = POINT { x: 9500, y: 2792 };
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
            c"ship.druuge.graphics.mauler.large".as_ptr(),
            c"ship.druuge.graphics.mauler.medium".as_ptr(),
            c"ship.druuge.graphics.mauler.small".as_ptr(),
        ];
        desc.ship_data.weapon_rsc = [
            c"ship.druuge.graphics.cannon.large".as_ptr(),
            c"ship.druuge.graphics.cannon.medium".as_ptr(),
            c"ship.druuge.graphics.cannon.small".as_ptr(),
        ];
        desc.ship_data.captain_control.captain_rsc = c"ship.druuge.graphics.captain".as_ptr();
        desc.ship_data.victory_ditty_rsc = c"ship.druuge.ditty".as_ptr();
        desc.ship_data.ship_sounds_rsc = c"ship.druuge.sounds".as_ptr();
        desc.cyborg_control.WeaponRange = MISSILE_RANGE as u16;
        desc.cyborg_control.intelligence_func = Some(druuge_intelligence);
        desc.preprocess_func = Some(druuge_preprocess);
        desc.postprocess_func = Some(druuge_postprocess);
        desc.init_weapon_func = Some(initialize_cannon);
        desc
    }

    static mut DRUUGE_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_druuge() -> *mut RACE_DESC {
        &raw mut DRUUGE_DESC
    }

    unsafe extern "C" fn initialize_cannon(ship: *mut ELEMENT, cannons: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: DRUUGE_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess_func: None,
            blast_offs: MISSILE_OFFSET,
        };
        *cannons = initialize_missile(&mut block);
        if !(*cannons).is_null() {
            let cannon = element(*cannons);
            (*cannon).collision_func = Some(cannon_collision);
        }
        1
    }

    unsafe extern "C" fn cannon_collision(
        cannon: *mut ELEMENT,
        point0: *mut POINT,
        other: *mut ELEMENT,
        point1: *mut POINT,
    ) {
        weapon_collision(cannon, point0, other, point1);
        let collision_mass = i32::from((*other).__bindgen_anon_3.mass_points) + 1;
        if (*other).state_flags & PLAYER_SHIP as u16 != 0
            && (*other).__bindgen_anon_2.crew_level != 0
            && collision_mass <= MAX_SHIP_MASS as i32 * 10
        {
            let ss = starship(other);
            (*ss).cur_status_flags &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
            let angle = facing_to_angle(i32::from(frame_index((*cannon).next.image.frame)));
            DeltaVelocityComponents(
                &raw mut (*other).velocity,
                cosine(angle, RECOIL_VELOCITY) as SIZE,
                sine(angle, RECOIL_VELOCITY) as SIZE,
            );
            let mut dx: SIZE = 0;
            let mut dy: SIZE = 0;
            GetCurrentVelocityComponents(&raw mut (*other).velocity, &mut dx, &mut dy);
            let x = i64::from(dx);
            let y = i64::from(dy);
            let limit = i64::from(MAX_RECOIL_VELOCITY);
            if x * x + y * y > limit * limit {
                let a = ARCTAN(dx, dy);
                SetVelocityComponents(
                    &raw mut (*other).velocity,
                    cosine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE,
                    sine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE,
                );
            }
        }
    }

    unsafe extern "C" fn druuge_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        let mut old_flags = 0u16;
        let mut enemy_ss = ptr::null_mut();
        if (*ss).cur_status_flags & SHIP_BEYOND_MAX_SPEED as u16 != 0 {
            (*enemy).MoveState = MOVEMENT_STATE_ENTICE;
        } else if !(*enemy).ObjectPtr.is_null()
            && (*enemy).which_turn <= ((MISSILE_RANGE * 3 / 4) >> 6) as u16
        {
            enemy_ss = starship((*enemy).ObjectPtr);
            old_flags = (*(*enemy_ss).RaceDescPtr).ship_info.ship_flags;
            (*(*enemy_ss).RaceDescPtr).ship_info.ship_flags &=
                !((FIRES_FORE | FIRES_RIGHT | FIRES_AFT | FIRES_LEFT) as u16);
            (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
            if (*ship).__bindgen_anon_5.thrust_wait == 0 {
                (*ship).__bindgen_anon_5.thrust_wait += 1;
            }
        }
        ship_intelligence(ship, concerns, count);
        if !enemy_ss.is_null() {
            (*(*enemy_ss).RaceDescPtr).ship_info.ship_flags = old_flags;
        }
        if (*ss).cur_status_flags & SHIP_BEYOND_MAX_SPEED as u16 == 0
            && ((*enemy).which_turn <= 12
                || (!(*concerns.add(ENEMY_WEAPON_INDEX as usize))
                    .ObjectPtr
                    .is_null()
                    && (*concerns.add(ENEMY_WEAPON_INDEX as usize)).which_turn <= 6))
        {
            (*ss).ship_input_state |= WEAPON as u8;
            if (*ship).__bindgen_anon_5.thrust_wait < WEAPON_WAIT + 1 {
                (*ship).__bindgen_anon_5.thrust_wait = WEAPON_WAIT + 1;
            }
        }
        if (*ss).ship_input_state & WEAPON as u8 != 0
            && (*(*ss).RaceDescPtr).ship_info.energy_level < WEAPON_ENERGY_COST
            && (*ship).__bindgen_anon_2.crew_level > 1
        {
            (*ss).ship_input_state |= SPECIAL as u8;
        } else {
            (*ss).ship_input_state &= !(SPECIAL as u8);
        }
    }

    unsafe extern "C" fn druuge_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & WEAPON as u16 == 0
            || (*ss).weapon_counter != (*(*ss).RaceDescPtr).characteristics.weapon_wait
        {
            return;
        }
        (*ss).cur_status_flags &= !(SHIP_AT_MAX_SPEED as u16);
        let angle = facing_to_angle(i32::from((*ss).ShipFacing)) + HALF_CIRCLE as i32;
        DeltaVelocityComponents(
            &raw mut (*ship).velocity,
            cosine(angle, RECOIL_VELOCITY) as SIZE,
            sine(angle, RECOIL_VELOCITY) as SIZE,
        );
        let mut dx: SIZE = 0;
        let mut dy: SIZE = 0;
        GetCurrentVelocityComponents(&raw mut (*ship).velocity, &mut dx, &mut dy);
        let x = i64::from(dx);
        let y = i64::from(dy);
        let limit = i64::from(MAX_RECOIL_VELOCITY);
        if x * x + y * y > limit * limit {
            let a = ARCTAN(dx, dy);
            SetVelocityComponents(
                &raw mut (*ship).velocity,
                cosine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE,
                sine(i32::from(a), MAX_RECOIL_VELOCITY) as SIZE,
            );
        }
    }

    unsafe extern "C" fn druuge_preprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & SPECIAL as u16 == 0 {
            return;
        }
        if (*ss).special_counter != 0
            || (*ship).__bindgen_anon_2.crew_level == 1
            || (*(*ss).RaceDescPtr).ship_info.energy_level
                == (*(*ss).RaceDescPtr).ship_info.max_energy
        {
            (*ss).cur_status_flags &= !(SPECIAL as u16);
            return;
        }
        ProcessSound(
            SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
            ship,
        );
        DeltaCrew(ship, -1);
        DeltaEnergy(ship, i16::from(SPECIAL_ENERGY_COST));
        (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
    }
}
