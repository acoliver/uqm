//! Slylandro live callbacks translated from `sc2/src/uqm/ships/slylandr/slylandr.c`.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, normalize_facing, put_element,
        sine, starship,
    };
    use std::ptr;

    const MAX_CREW: u16 = 12;
    const MAX_ENERGY: u8 = 20;
    const WEAPON_WAIT: u8 = 17;
    const LASER_LENGTH: u32 = 32;
    const HARVEST_RANGE: i32 = 208 * 3 / 8;
    const CLOSE_RANGE_WEAPON: u16 = 50 << ONE_SHIFT;

    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (SEEKING_WEAPON | CREW_IMMUNE) as u16;
        d.ship_info.ship_cost = 17;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.slylandro.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.slylandro.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.slylandro.meleeicons".as_ptr();
        // races.h: INFINITE_RADIUS is (~0), stored in COUNT.
        d.fleet.strength = u16::MAX;
        d.fleet.known_loc = POINT { x: 333, y: 9812 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: 60,
            thrust_increment: 60,
            energy_regeneration: 0,
            weapon_energy_cost: 2,
            special_energy_cost: 0,
            energy_wait: 10,
            turn_wait: 0,
            thrust_wait: 0,
            weapon_wait: WEAPON_WAIT,
            special_wait: 20,
            ship_mass: 1,
        };
        d.ship_data.ship_rsc = [
            c"ship.slylandro.graphics.probe.large".as_ptr(),
            c"ship.slylandro.graphics.probe.medium".as_ptr(),
            c"ship.slylandro.graphics.probe.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.slylandro.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.slylandro.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.slylandro.sounds".as_ptr();
        d.cyborg_control.WeaponRange = CLOSE_RANGE_WEAPON << 1;
        d
    }
    static mut SLYLANDRO_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_slylandro() -> *mut RACE_DESC {
        unsafe {
            SLYLANDRO_DESC.preprocess_func = Some(slylandro_preprocess);
            SLYLANDRO_DESC.postprocess_func = Some(slylandro_postprocess);
            SLYLANDRO_DESC.init_weapon_func = Some(initialize_lightning);
            SLYLANDRO_DESC.cyborg_control.intelligence_func = Some(slylandro_intelligence);
        }
        &raw mut SLYLANDRO_DESC
    }

    unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
        GlobData
            .Game_state
            .DisplayArray
            .add((*e).PrimIndex as usize)
    }

    const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    pub(crate) unsafe extern "C" fn lightning_postprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait != 0 && (*e).state_flags & COLLISION as u16 == 0 {
            let mut h = ptr::null_mut();
            initialize_lightning(e, &mut h);
            if !h.is_null() {
                put_element(h);
            }
        }
    }

    pub(crate) unsafe extern "C" fn lightning_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        let ss = starship(a);
        if (*ss).weapon_counter > WEAPON_WAIT >> 1 {
            (*ss).weapon_counter = WEAPON_WAIT.wrapping_sub((*ss).weapon_counter);
        }
        (*ss).weapon_counter = (*ss)
            .weapon_counter
            .wrapping_sub((*a).__bindgen_anon_4.turn_wait);
        (*a).__bindgen_anon_4.turn_wait = 0;
        weapon_collision(a, pa, b, pb);
    }

    pub(crate) unsafe extern "C" fn initialize_lightning(
        e: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let mut block = LASER_BLOCK {
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
            ex: 0,
            ey: 0,
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            face: 0,
            pixoffs: 0,
            // The C block leaves color uninitialized; it is overwritten below
            // before the new element can be drawn.
            color: rgb15(0, 0, 0),
        };
        *out = initialize_laser(&mut block);
        if !(*out).is_null() {
            let ss = starship(e);
            let laser = element(*out);
            (*laser).postprocess_func = Some(lightning_postprocess);
            (*laser).collision_func = Some(lightning_collision);
            // This draw precedes TrackShip's optional opposite-facing draw.
            let random = TFB_Random();
            let mut angle: COUNT;
            let delta: SIZE;
            if (*e).state_flags & PLAYER_SHIP as u16 == 0 {
                angle = (*e).velocity.travel_angle;
                let mut facing = normalize_facing(angle_to_facing(i32::from(angle)));
                delta = TrackShip(e, &mut facing);
                (*laser).__bindgen_anon_4.turn_wait =
                    (*e).__bindgen_anon_4.turn_wait.wrapping_sub(1);
                (*prim(laser)).color = (*prim(e)).color;
                // Child segments keep their travel angle, not the rounded and
                // turned facing returned by TrackShip.
            } else {
                let mut facing = (*ss).ShipFacing;
                (*e).hTarget = ptr::null_mut();
                delta = TrackShip(e, &mut facing);
                (*e).hTarget = ptr::null_mut();
                angle = facing_to_angle(i32::from(facing)) as COUNT;
                let mut wait = (*ss).weapon_counter;
                if wait == 0 {
                    wait = WEAPON_WAIT;
                }
                if wait > WEAPON_WAIT >> 1 {
                    wait = WEAPON_WAIT.wrapping_sub(wait);
                }
                (*laser).__bindgen_anon_4.turn_wait = wait;
                (*prim(laser)).color = match (random >> 8) & 3 {
                    0 => rgb15(31, 31, 31),
                    1 => rgb15(22, 23, 31),
                    2 => rgb15(6, 7, 31),
                    _ => rgb15(0, 0, 24),
                };
            }
            if delta == -1 || i32::from(delta) == angle_to_facing(HALF_CIRCLE as i32) {
                angle = angle.wrapping_add(random as COUNT);
            } else if delta == 0 {
                angle = if random as COUNT & 1 != 0 {
                    angle.wrapping_sub(1)
                } else {
                    angle.wrapping_add(1)
                };
            } else if i32::from(delta) < angle_to_facing(HALF_CIRCLE as i32) {
                angle = angle.wrapping_add(random as COUNT & (QUADRANT as COUNT - 1));
            } else {
                angle = angle.wrapping_sub(random as COUNT & (QUADRANT as COUNT - 1));
            }
            let length = ((((random >> 16) & (LASER_LENGTH - 1)) + 4)
                << (ONE_SHIFT + VELOCITY_SHIFT)) as SIZE;
            SetVelocityComponents(
                &raw mut (*laser).velocity,
                cosine(i32::from(angle), i32::from(length)),
                sine(i32::from(angle), i32::from(length)),
            );
            (*laser).pParent = ss.cast();
        }
        1
    }

    pub(crate) unsafe extern "C" fn slylandro_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        mut count: COUNT,
    ) {
        if GlobData.Game_state.CurrentActivity as u8 == IN_ENCOUNTER as u8 {
            (*concerns.add(ENEMY_WEAPON_INDEX as usize)).ObjectPtr = ptr::null_mut();
        }
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        let ss = starship(ship);
        if (*ss).special_counter == 0
            && (*(*ss).RaceDescPtr).ship_info.energy_level == 0
            && (*concerns.add(GRAVITY_MASS_INDEX as usize))
                .ObjectPtr
                .is_null()
        {
            count = FIRST_EMPTY_INDEX as COUNT + 1;
        }
        if !(*enemy).ObjectPtr.is_null()
            && (*enemy).MoveState == MOVEMENT_STATE_PURSUE
            && (*enemy).which_turn <= 6
        {
            (*enemy).MoveState = MOVEMENT_STATE_ENTICE;
        }
        (*ship).__bindgen_anon_5.thrust_wait = (*ship).__bindgen_anon_5.thrust_wait.wrapping_add(1);
        ship_intelligence(ship, concerns, count);
        (*ship).__bindgen_anon_5.thrust_wait = (*ship).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
        if !(*enemy).ObjectPtr.is_null() && (*enemy).which_turn <= 14 {
            (*ss).ship_input_state |= WEAPON as u8;
        } else {
            (*ss).ship_input_state &= !(WEAPON as u8);
        }
        (*ss).ship_input_state &= !(SPECIAL as u8);
        if (*(*ss).RaceDescPtr).ship_info.energy_level < (*(*ss).RaceDescPtr).ship_info.max_energy {
            let junk = concerns.add(FIRST_EMPTY_INDEX as usize);
            if !(*junk).ObjectPtr.is_null() && (*junk).which_turn <= 14 {
                (*ss).ship_input_state |= SPECIAL as u8;
            }
        }
    }

    // CollisionPossible checks solidity only on the first argument.
    unsafe fn collision_possible(a: *mut ELEMENT, b: *mut ELEMENT) -> bool {
        let shared = (*a).state_flags & (*b).state_flags;
        (*a).state_flags & (NONSOLID | DISAPPEARING) as u16 == 0
            && shared & COLLISION as u16 == 0
            && (shared & IGNORE_SIMILAR as u16 == 0 || (*a).pParent != (*b).pParent)
            && ((*a).__bindgen_anon_3.mass_points != 0 || (*b).__bindgen_anon_3.mass_points != 0)
    }

    unsafe fn harvest_space_junk(ship: *mut ELEMENT) -> bool {
        let mut found = false;
        let mut h = disp_q.head;
        while !h.is_null() {
            let obj = element(h);
            let next = (*obj).succ;
            if (*obj).state_flags & (APPEARING | PLAYER_SHIP | FINITE_LIFE) as u16 == 0
                && (*obj).playerNr == NEUTRAL_PLAYER_NUM as SIZE
                && u32::from((*obj).__bindgen_anon_3.mass_points) <= MAX_SHIP_MASS * 10
                && collision_possible(obj, ship)
            {
                let mut dx = (i32::from((*obj).next.location.x)
                    - i32::from((*ship).next.location.x)) as SIZE;
                let mut dy = (i32::from((*obj).next.location.y)
                    - i32::from((*ship).next.location.y)) as SIZE;
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                if i32::from(dx) <= HARVEST_RANGE
                    && i32::from(dy) <= HARVEST_RANGE
                    && i32::from(dx) * i32::from(dx) + i32::from(dy) * i32::from(dy)
                        <= HARVEST_RANGE * HARVEST_RANGE
                {
                    (*obj).__bindgen_anon_1.life_span = 0;
                    (*obj).state_flags |= NONSOLID as u16;
                    if !found {
                        found = true;
                        let ss = starship(ship);
                        ProcessSound(
                            SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                            ship,
                        );
                        DeltaEnergy(ship, i16::from(MAX_ENERGY));
                    }
                }
            }
            h = next;
        }
        found
    }

    pub(crate) unsafe extern "C" fn slylandro_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).weapon_counter != 0 && (*ss).weapon_counter < WEAPON_WAIT {
            let mut h = ptr::null_mut();
            initialize_lightning(ship, &mut h);
            if !h.is_null() {
                put_element(h);
            }
        }
        if (*ss).special_counter == 0
            && (*ss).cur_status_flags & SPECIAL as u16 != 0
            && harvest_space_junk(ship)
        {
            (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
        }
    }

    pub(crate) unsafe extern "C" fn slylandro_preprocess(ship: *mut ELEMENT) {
        if (*ship).state_flags & (APPEARING | NONSOLID) as u16 == 0 {
            let ss = starship(ship);
            if (*ss).cur_status_flags & THRUST as u16 != 0
                && (*ss).old_status_flags & THRUST as u16 == 0
            {
                (*ss).ShipFacing = (*ss)
                    .ShipFacing
                    .wrapping_add(angle_to_facing(HALF_CIRCLE as i32) as COUNT);
            }
            if (*ship).__bindgen_anon_4.turn_wait == 0 {
                (*ship).__bindgen_anon_4.turn_wait = (u16::from((*ship).__bindgen_anon_4.turn_wait)
                    + u16::from((*(*ss).RaceDescPtr).characteristics.turn_wait)
                    + 1) as BYTE;
                if (*ss).cur_status_flags & LEFT as u16 != 0 {
                    (*ss).ShipFacing = (*ss).ShipFacing.wrapping_sub(1);
                } else if (*ss).cur_status_flags & RIGHT as u16 != 0 {
                    (*ss).ShipFacing = (*ss).ShipFacing.wrapping_add(1);
                }
            }
            (*ss).ShipFacing = normalize_facing(i32::from((*ss).ShipFacing));
            if (*ship).__bindgen_anon_5.thrust_wait == 0 {
                (*ship).__bindgen_anon_5.thrust_wait =
                    (u16::from((*ship).__bindgen_anon_5.thrust_wait)
                        + u16::from((*(*ss).RaceDescPtr).characteristics.thrust_wait)
                        + 1) as BYTE;
                SetVelocityVector(
                    &raw mut (*ship).velocity,
                    (*(*ss).RaceDescPtr).characteristics.max_thrust as SIZE,
                    (*ss).ShipFacing,
                );
                (*ss).cur_status_flags |= SHIP_AT_MAX_SPEED as u16;
                (*ss).cur_status_flags &= !(SHIP_IN_GRAVITY_WELL as u16);
            }
            image!(frame; (*ship).next.image.frame, IncFrameIndex((*ship).next.image.frame));
            (*ship).state_flags |= CHANGING as u16;
        }
    }
}
