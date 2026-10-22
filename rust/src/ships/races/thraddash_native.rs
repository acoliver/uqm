//! Thraddash callbacks and descriptor translated from `thradd.c`.

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, element, facing_to_angle, frame_index, normalize_angle,
        normalize_facing, starship,
    };
    use std::{ffi::c_void, mem};

    extern "C" {
        fn InsertQueue(queue: *mut c_void, handle: *mut c_void, before: *mut c_void);
    }

    const MAX_CREW: u16 = 8;
    const MAX_ENERGY: u8 = 24;
    const MAX_THRUST: u16 = 28;
    const THRUST_INCREMENT: u16 = 7;
    const SPECIAL_THRUST_INCREMENT: u16 = 12;
    const SPECIAL_MAX_THRUST: u16 = 72;
    const LONG_RANGE_WEAPON: i32 = (30 << ONE_SHIFT) * 15 / 2;
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 15;
    const NAPALM_LIFE: u16 = 48;
    const NAPALM_DECAY_RATE: u8 = 5;
    const NUM_NAPALM_FADES: u16 = 6;

    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.ship_flags = FIRES_FORE as u16;
        d.ship_info.ship_cost = 10;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.thraddash.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.thraddash.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.thraddash.meleeicons".as_ptr();
        d.fleet.strength = 833 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet.known_loc = POINT { x: 2535, y: 8358 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: 1,
            weapon_energy_cost: 2,
            special_energy_cost: 1,
            energy_wait: 6,
            turn_wait: 1,
            thrust_wait: 0,
            weapon_wait: 12,
            special_wait: 0,
            ship_mass: 7,
        };
        d.ship_data.ship_rsc = [
            c"ship.thraddash.graphics.torch.large".as_ptr(),
            c"ship.thraddash.graphics.torch.medium".as_ptr(),
            c"ship.thraddash.graphics.torch.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.thraddash.graphics.horn.large".as_ptr(),
            c"ship.thraddash.graphics.horn.medium".as_ptr(),
            c"ship.thraddash.graphics.horn.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.thraddash.graphics.napalm.large".as_ptr(),
            c"ship.thraddash.graphics.napalm.medium".as_ptr(),
            c"ship.thraddash.graphics.napalm.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.thraddash.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.thraddash.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.thraddash.sounds".as_ptr();
        d.cyborg_control.WeaponRange = LONG_RANGE_WEAPON as u16;
        d
    }

    static mut THRADDASH_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_thraddash() -> *mut RACE_DESC {
        unsafe {
            THRADDASH_DESC.preprocess_func = Some(thraddash_preprocess);
            THRADDASH_DESC.init_weapon_func = Some(initialize_horn);
            THRADDASH_DESC.cyborg_control.intelligence_func = Some(thraddash_intelligence);
        }
        &raw mut THRADDASH_DESC
    }

    unsafe extern "C" fn flame_napalm_preprocess(e: *mut ELEMENT) {
        (*e).velocity = mem::zeroed();
        if (*e).state_flags & NONSOLID as u16 != 0 {
            (*e).state_flags &= !(NONSOLID as u16);
            (*e).state_flags |= APPEARING as u16;
            (*GlobData
                .Game_state
                .DisplayArray
                .add(usize::from((*e).PrimIndex)))
            .Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            (*e).IntersectControl.IntersectStamp.origin = POINT {
                x: (*e).current.location.x >> ONE_SHIFT,
                y: (*e).current.location.y >> ONE_SHIFT,
            };
            (*e).IntersectControl.EndPoint = POINT {
                x: (*e).next.location.x >> ONE_SHIFT,
                y: (*e).next.location.y >> ONE_SHIFT,
            };
            (*e).IntersectControl.IntersectStamp.frame =
                SetEquFrameIndex(*(*e).next.image.farray, (*e).next.image.frame);
        } else if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            if (*e).__bindgen_anon_1.life_span <= NUM_NAPALM_FADES * (NAPALM_DECAY_RATE as u16 + 1)
                || frame_index((*e).current.image.frame) != NUM_NAPALM_FADES
            {
                (*e).next.image.frame = DecFrameIndex((*e).current.image.frame);
            } else if (*e).__bindgen_anon_1.life_span
                > NUM_NAPALM_FADES * (NAPALM_DECAY_RATE as u16 + 1)
            {
                (*e).next.image.frame = SetAbsFrameIndex(
                    (*e).current.image.frame,
                    GetFrameCount((*e).current.image.frame).wrapping_sub(1),
                );
            }
            (*e).__bindgen_anon_4.turn_wait = NAPALM_DECAY_RATE;
            (*e).state_flags |= CHANGING as u16;
        }
    }

    unsafe extern "C" fn initialize_horn(ship: *mut ELEMENT, out: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut b = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 9,
            speed: MISSILE_SPEED,
            hit_points: 2,
            damage: 1,
            life: MISSILE_LIFE,
            preprocess_func: None,
            blast_offs: 3,
        };
        *out = initialize_missile(&mut b);
        1
    }

    unsafe extern "C" fn thraddash_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 == 0 {
            if (*ss).old_status_flags & SPECIAL as u16 != 0
                && (*ss).cur_status_flags & SHIP_AT_MAX_SPEED as u16 != 0
            {
                (*ss).cur_status_flags |= SHIP_BEYOND_MAX_SPEED as u16;
            }
            return;
        }
        if DeltaEnergy(e, -1) == 0 {
            return;
        }
        if (*ss).old_status_flags & SPECIAL as u16 == 0 {
            (*ss).cur_status_flags &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
        }
        if (*e).__bindgen_anon_5.thrust_wait == 0 {
            (*e).__bindgen_anon_5.thrust_wait += 1;
        }
        let desc = (*ss).RaceDescPtr;
        let inc = (*desc).characteristics.thrust_increment;
        let max = (*desc).characteristics.max_thrust;
        (*desc).characteristics.thrust_increment = SPECIAL_THRUST_INCREMENT;
        (*desc).characteristics.max_thrust = SPECIAL_MAX_THRUST;
        let status = inertial_thrust(e);
        (*ss).cur_status_flags &=
            !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16);
        (*ss).cur_status_flags |= status as u16;
        (*desc).characteristics.thrust_increment = inc;
        (*desc).characteristics.max_thrust = max;
        let mut b = MISSILE_BLOCK {
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
            farray: (&raw mut (*desc).ship_data.special).cast(),
            face: 0,
            index: GetFrameCount((*desc).ship_data.special[0]).wrapping_sub(1),
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 1,
            damage: 2,
            life: NAPALM_LIFE,
            preprocess_func: Some(flame_napalm_preprocess),
            blast_offs: 0,
        };
        let h = initialize_missile(&mut b);
        if !h.is_null() {
            let trail = element(h);
            (*trail).pParent = ss.cast();
            (*trail).hTarget = std::ptr::null_mut();
            (*trail).__bindgen_anon_4.turn_wait = NAPALM_DECAY_RATE;
            (*trail).state_flags |= NONSOLID as u16;
            (*GlobData
                .Game_state
                .DisplayArray
                .add(usize::from((*trail).PrimIndex)))
            .Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
            (*trail).next = (*trail).current;
            (*trail).state_flags |= PRE_PROCESS as u16;
            InsertQueue((&raw mut disp_q).cast(), h, disp_q.head);
            ProcessSound(SetAbsStringTableIndex((*desc).ship_data.ship_sounds, 1), e);
        }
    }

    unsafe extern "C" fn thraddash_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if !(*enemy).ObjectPtr.is_null() {
            let mut dx: SIZE = 0;
            let mut dy: SIZE = 0;
            GetCurrentVelocityComponents(&raw mut (*(*enemy).ObjectPtr).velocity, &mut dx, &mut dy);
            let stationary = (4 << ONE_SHIFT) << VELOCITY_SHIFT;
            let speed2 = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
            (*enemy).MoveState = if (*enemy).which_turn > 8
                || speed2 <= i64::from(stationary) * i64::from(stationary)
            {
                MOVEMENT_STATE_PURSUE
            } else {
                MOVEMENT_STATE_ENTICE
            };
        }
        ship_intelligence(ship, concerns, count);
        let ss = starship(ship);
        if (*ss).special_counter != 0 {
            return;
        }
        (*ss).ship_input_state &= !(SPECIAL as u8);
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*weapon).ObjectPtr.is_null() && (*weapon).MoveState == MOVEMENT_STATE_ENTICE {
            if (*ss).ship_input_state & THRUST as u8 != 0
                || ((*ship).__bindgen_anon_4.turn_wait == 0
                    && (*ss).ship_input_state & (LEFT | RIGHT) as u8 == 0)
                || normalize_facing(
                    angle_to_facing(
                        i32::from((*(*weapon).ObjectPtr).velocity.TravelAngle)
                            + HALF_CIRCLE as i32
                            + OCTANT as i32,
                    ) - i32::from((*ss).ShipFacing),
                ) > angle_to_facing(QUADRANT as i32) as u16
            {
                (*ss).ship_input_state |= SPECIAL as u8;
            }
        } else if !(*enemy).ObjectPtr.is_null() {
            if (*enemy).MoveState == MOVEMENT_STATE_PURSUE {
                if (*(*ss).RaceDescPtr).ship_info.energy_level >= 3
                    && (*ship).__bindgen_anon_4.turn_wait == 0
                    && (*ss).ship_input_state & (LEFT | RIGHT) as u8 == 0
                    && ((*ss).cur_status_flags & SPECIAL as u16 == 0
                        || (*ss).cur_status_flags
                            & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
                            == 0)
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            } else if (*enemy).MoveState == MOVEMENT_STATE_ENTICE {
                let dx = (*(*enemy).ObjectPtr)
                    .next
                    .location
                    .x
                    .wrapping_sub((*ship).next.location.x);
                let dy = (*(*enemy).ObjectPtr)
                    .next
                    .location
                    .y
                    .wrapping_sub((*ship).next.location.y);
                let dir = ARCTAN(dx, dy);
                if ((*enemy).which_turn > 24 && (*ss).ship_input_state & (LEFT | RIGHT) as u8 == 0)
                    || ((*enemy).which_turn <= 16
                        && normalize_angle(
                            dir as i32
                                - (facing_to_angle(i32::from((*ss).ShipFacing))
                                    + HALF_CIRCLE as i32)
                                + QUADRANT as i32,
                        ) <= HALF_CIRCLE as i32
                        && ((*enemy).which_turn < 12
                            || normalize_angle(
                                dir as i32
                                    - ((*(*enemy).ObjectPtr).velocity.TravelAngle as i32
                                        + HALF_CIRCLE as i32)
                                    + OCTANT as i32
                                    + 2,
                            ) <= (OCTANT as i32 + 2) * 2))
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            }
        }
        if (*ss).ship_input_state & SPECIAL as u8 != 0
            && (*(*ss).RaceDescPtr).ship_info.energy_level >= 1
        {
            (*ss).ship_input_state &= !(THRUST as u8);
        }
    }
}
