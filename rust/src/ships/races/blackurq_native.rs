//! Live black_urquan callbacks translated from ships/blackurq/blackurq.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{
        abi::*, angle_to_facing, element, frame_count, frame_index, init_intersect_points,
        normalize_facing, put_element, starship, wrap_delta,
    };
    use std::{mem, ptr};

    use crate::collections::queue::InsertQueue;
    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE as i32) as _;
        d.ship_info.ship_cost = 30;
        d.ship_info.crew_level = (MAX_CREW_SIZE as i32) as _;
        d.ship_info.max_crew = (MAX_CREW_SIZE as i32) as _;
        d.ship_info.energy_level = (MAX_ENERGY_SIZE as i32) as _;
        d.ship_info.max_energy = (MAX_ENERGY_SIZE as i32) as _;
        d.ship_info.race_strings_rsc = c"ship.kohrah.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.kohrah.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.kohrah.meleeicons".as_ptr();
        d.fleet.strength = (2666 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet.known_loc = POINT { x: 6000, y: 6250 };
        d.characteristics.max_thrust = 30;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 6;
        d.characteristics.special_energy_cost = (MAX_ENERGY_SIZE as i32 / 2) as _;
        d.characteristics.energy_wait = 4;
        d.characteristics.turn_wait = 4;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 6;
        d.characteristics.special_wait = 9;
        d.characteristics.ship_mass = 10;
        d.ship_data.ship_rsc = [
            c"ship.kohrah.graphics.marauder.large".as_ptr(),
            c"ship.kohrah.graphics.marauder.medium".as_ptr(),
            c"ship.kohrah.graphics.marauder.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.kohrah.graphics.buzzsaw.large".as_ptr(),
            c"ship.kohrah.graphics.buzzsaw.medium".as_ptr(),
            c"ship.kohrah.graphics.buzzsaw.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.kohrah.graphics.gas.large".as_ptr(),
            c"ship.kohrah.graphics.gas.medium".as_ptr(),
            c"ship.kohrah.graphics.gas.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.kohrah.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.kohrah.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.kohrah.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (50 << ONE_SHIFT) as u16;
        d
    }
    static mut DESC: RACE_DESC = descriptor();
    #[no_mangle]
    pub extern "C" fn init_black_urquan() -> *mut RACE_DESC {
        unsafe {
            DESC.preprocess_func = Some(black_urquan_preprocess);
            DESC.postprocess_func = Some(black_urquan_postprocess);
            DESC.init_weapon_func = Some(initialize_buzzsaw);
            DESC.cyborg_control.intelligence_func = Some(black_urquan_intelligence);
        }
        &raw mut DESC
    }
    pub(crate) unsafe extern "C" fn spin_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let ship = element((*ss).hShip);
        if (*ship).__bindgen_anon_2.crew_level != 0 && {
            let wait = &raw mut (*(*ss).RaceDescPtr).characteristics.special_wait;
            *wait = (*wait).wrapping_add(1);
            *wait > 8
        } {
            (*e).__bindgen_anon_1.life_span = 1;
            (*e).state_flags |= DISAPPEARING as u16;
        } else {
            (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
            if (*e).__bindgen_anon_4.turn_wait != 0 {
                (*e).__bindgen_anon_4.turn_wait -= 1;
            } else {
                image!(frame; (*e).next.image.frame, if frame_index((*e).current.image.frame) < 1 {
                    IncFrameIndex((*e).current.image.frame)
                } else {
                    SetAbsFrameIndex((*e).current.image.frame, 0)
                });
                (*e).state_flags |= CHANGING as u16;
                (*e).__bindgen_anon_4.turn_wait = 0;
            }
        }
    }

    pub(crate) unsafe extern "C" fn buzztrack_preprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_5.thrust_wait != 0 {
            (*e).__bindgen_anon_5.thrust_wait -= 1;
        } else {
            let mut facing = 0;
            if (*e).hTarget.is_null() && TrackShip(e, &mut facing) < 0 {
                (*e).velocity = mem::zeroed();
            } else {
                let target = element((*e).hTarget);
                let mut dx = wrap_delta(
                    (*target)
                        .current
                        .location
                        .x
                        .wrapping_sub((*e).current.location.x),
                    (ScreenWidth - 64) << (ONE_SHIFT + 3),
                );
                let mut dy = wrap_delta(
                    (*target)
                        .current
                        .location
                        .y
                        .wrapping_sub((*e).current.location.y),
                    ScreenHeight << (ONE_SHIFT + 3),
                );
                facing = normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy))));
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                if dx >= 224
                    || dy >= 224
                    || (i32::from(dx as u16) * i32::from(dx)) as u32
                        + (i32::from(dy as u16) * i32::from(dy)) as u32
                        >= 224 * 224
                {
                    (*e).velocity = mem::zeroed();
                } else {
                    (*e).__bindgen_anon_5.thrust_wait = 4;
                    SetVelocityVector(&raw mut (*e).velocity, 2 << ONE_SHIFT, facing);
                }
            }
        }
        spin_preprocess(e);
    }

    pub(crate) unsafe extern "C" fn decelerate_preprocess(e: *mut ELEMENT) {
        let (mut dx, mut dy) = (0, 0);
        GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut dx, &mut dy);
        dx /= 2;
        dy /= 2;
        SetVelocityComponents(&raw mut (*e).velocity, dx, dy);
        if dx == 0 && dy == 0 {
            (*e).preprocess_func = Some(buzztrack_preprocess);
        }
        spin_preprocess(e);
    }
    pub(crate) unsafe extern "C" fn splinter_preprocess(e: *mut ELEMENT) {
        image!(frame; (*e).next.image.frame, IncFrameIndex((*e).current.image.frame));
        (*e).state_flags |= CHANGING as u16;
    }
    pub(crate) unsafe extern "C" fn buzzsaw_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        weapon_collision(a, pa, b, pb);
        if (*a).state_flags & DISAPPEARING as u16 != 0 {
            (*a).state_flags &= !(DISAPPEARING as u16);
            (*a).state_flags |= (NONSOLID | CHANGING) as u16;
            (*a).__bindgen_anon_1.life_span = 5;
            image!(frame; (*a).next.image.frame, SetAbsFrameIndex((*a).current.image.frame, 2));
            (*a).preprocess_func = Some(splinter_preprocess);
        }
    }
    pub(crate) unsafe extern "C" fn buzzsaw_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & WEAPON as u16 == 0 {
            (*e).__bindgen_anon_1.life_span >>= 1;
            (*e).preprocess_func = Some(decelerate_preprocess);
        }
        spin_preprocess(e);
    }
    pub(crate) unsafe extern "C" fn buzzsaw_postprocess(e: *mut ELEMENT) {
        (*e).postprocess_func = None;
        let h = AllocElement();
        if !h.is_null() {
            let list = element(h);
            let index = (*list).PrimIndex;
            *list = *e;
            (*list).PrimIndex = index;
            *GlobData.Game_state.DisplayArray.add(index as usize) = *GlobData
                .Game_state
                .DisplayArray
                .add((*e).PrimIndex as usize);
            image!(copy; (*list).current, (*list).next);
            init_intersect_points(list);
            (*list).state_flags = ((*list).state_flags
                & !((PRE_PROCESS | CHANGING | APPEARING) as u16))
                | POST_PROCESS as u16;
            let ss = starship(e);
            InsertQueue((&raw mut disp_q).cast(), h, (*element((*ss).hShip)).succ);
            (*e).__bindgen_anon_1.life_span = 0;
        }
    }
    pub(crate) unsafe extern "C" fn initialize_buzzsaw(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: 0,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 28,
            speed: 64,
            hit_points: 10,
            damage: 4,
            life: 64,
            preprocess_func: Some(buzzsaw_preprocess),
            blast_offs: 9,
        };
        *out = initialize_missile(&mut block);
        if !(*out).is_null() {
            let e = element(*out);
            (*e).__bindgen_anon_4.turn_wait = 0;
            (*e).__bindgen_anon_5.thrust_wait = 0;
            (*e).postprocess_func = Some(buzzsaw_postprocess);
            (*e).collision_func = Some(buzzsaw_collision);
        }
        1
    }
    pub(crate) unsafe extern "C" fn black_urquan_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*weapon).ObjectPtr.is_null()
            && (*weapon).MoveState == MOVEMENT_STATE_ENTICE
            && (*(*weapon).ObjectPtr).state_flags & CREW_OBJECT as u16 != 0
            && (*weapon).which_turn <= 8
        {
            (*weapon).MoveState = MOVEMENT_STATE_PURSUE;
        }
        ship_intelligence(ship, concerns, count);
        let ss = starship(ship);
        (*ss).ship_input_state &= !(SPECIAL as u8);
        if (*ss).special_counter == 0
            && (*(*ss).RaceDescPtr).ship_info.energy_level >= (MAX_ENERGY_SIZE / 2) as u8
            && !(*weapon).ObjectPtr.is_null()
            && (*weapon).which_turn <= 8
        {
            (*ss).ship_input_state |= SPECIAL as u8;
        }
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if !(*enemy).ObjectPtr.is_null() {
            let mut h = if (*ss).old_status_flags & WEAPON as u16 != 0 {
                (*ship).succ
            } else {
                ptr::null_mut()
            };
            while !h.is_null() {
                let saw = element(h);
                let next = (*saw).succ;
                if (*saw).state_flags & NONSOLID as u16 == 0
                    && (*saw).next.image.farray
                        == (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast()
                    && (*saw).__bindgen_anon_1.life_span > 64 * 3 / 4
                    && (*saw).playerNr == (*ship).playerNr
                {
                    if PlotIntercept(
                        saw,
                        (*enemy).ObjectPtr,
                        (*saw).__bindgen_anon_1.life_span,
                        640 / 2,
                    ) == 0
                    {
                        (*ss).ship_input_state &= !(WEAPON as u8);
                    } else if (*ss).weapon_counter == 0 {
                        (*ss).ship_input_state |= WEAPON as u8;
                    }
                    break;
                }
                h = next;
            }
            if h.is_null() {
                if (*ss).old_status_flags & WEAPON as u16 != 0 {
                    (*ss).ship_input_state &= !(WEAPON as u8);
                } else if (*ss).weapon_counter == 0
                    && ship_weapons(ship, (*enemy).ObjectPtr, 640 / 2) != 0
                {
                    (*ss).ship_input_state |= WEAPON as u8;
                }
                if (*ss).special_counter == 0
                    && (*ss).ship_input_state & WEAPON as u8 == 0
                    && (*enemy).which_turn <= 8
                    && (*ss).ship_input_state & (LEFT | RIGHT) as u8 != 0
                    && (*(*ss).RaceDescPtr).ship_info.energy_level >= (MAX_ENERGY_SIZE / 2) as u8
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            }
        }
    }
    pub(crate) unsafe extern "C" fn gas_cloud_preprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait != 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            image!(frame; (*e).next.image.frame, IncFrameIndex((*e).current.image.frame));
            (*e).state_flags |= CHANGING as u16;
            (*e).__bindgen_anon_4.turn_wait = 2;
        }
    }
    pub(crate) unsafe extern "C" fn gas_cloud_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        (*a).__bindgen_anon_3.mass_points = if (*b).state_flags & PLAYER_SHIP as u16 != 0 {
            3
        } else {
            50
        };
        weapon_collision(a, pa, b, pb);
    }
    pub(crate) unsafe extern "C" fn spawn_gas_cloud(e: *mut ELEMENT) {
        let ss = starship(e);
        let mut block = MISSILE_BLOCK {
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast(),
            face: 0,
            index: 0,
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 20,
            speed: 16,
            hit_points: 100,
            damage: 3,
            life: (i32::from(frame_count((*(*ss).RaceDescPtr).ship_data.special[0])) * 3 - 1)
                as COUNT,
            preprocess_func: Some(gas_cloud_preprocess),
            blast_offs: 2,
        };
        let (mut dx, mut dy) = (0, 0);
        GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut dx, &mut dy);
        while i32::from(block.face) < angle_to_facing(FULL_CIRCLE as i32) {
            let h = initialize_missile(&mut block);
            if !h.is_null() {
                let gas = element(h);
                (*gas).pParent = ss.cast();
                (*gas).hTarget = ptr::null_mut();
                (*gas).__bindgen_anon_4.turn_wait = 1;
                (*gas).collision_func = Some(gas_cloud_collision);
                DeltaVelocityComponents(&raw mut (*gas).velocity, dx, dy);
                put_element(h);
            }
            block.face =
                (i32::from(block.face) + angle_to_facing(FULL_CIRCLE as i32) / 16) as COUNT;
        }
    }
    pub(crate) unsafe extern "C" fn black_urquan_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && DeltaEnergy(e, -((MAX_ENERGY_SIZE / 2) as SIZE)) != 0
        {
            spawn_gas_cloud(e);
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                e,
            );
            (*ss).special_counter = 9;
        }
    }
    pub(crate) unsafe extern "C" fn black_urquan_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        (*(*ss).RaceDescPtr).characteristics.special_wait = 0;
        if (*ss).weapon_counter == 0
            && (*ss).cur_status_flags & (*ss).old_status_flags & WEAPON as u16 != 0
        {
            (*ss).weapon_counter += 1;
        }
    }
}
