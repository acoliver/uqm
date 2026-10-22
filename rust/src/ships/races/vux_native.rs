//! Live vux callbacks translated from ships/vux/vux.c.
#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, init_intersect_points,
        normalize_facing, put_element, sine, starship,
    };
    use std::{mem, ptr};
    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.ship_flags =
            (FIRES_FORE as i32 | SEEKING_SPECIAL as i32 | IMMEDIATE_WEAPON as i32) as _;
        d.ship_info.ship_cost = 12;
        d.ship_info.crew_level = 20;
        d.ship_info.max_crew = 20;
        d.ship_info.energy_level = 40;
        d.ship_info.max_energy = 40;
        d.ship_info.race_strings_rsc = c"ship.vux.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.vux.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.vux.meleeicons".as_ptr();
        d.fleet.strength = (900 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet.known_loc = POINT { x: 4412, y: 1558 };
        d.characteristics.max_thrust = 21;
        d.characteristics.thrust_increment = 7;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = 2;
        d.characteristics.energy_wait = 8;
        d.characteristics.turn_wait = 6;
        d.characteristics.thrust_wait = 4;
        d.characteristics.weapon_wait = 0;
        d.characteristics.special_wait = 7;
        d.characteristics.ship_mass = 6;
        d.ship_data.ship_rsc = [
            c"ship.vux.graphics.intruder.large".as_ptr(),
            c"ship.vux.graphics.intruder.medium".as_ptr(),
            c"ship.vux.graphics.intruder.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.vux.graphics.slime".as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        ];
        d.ship_data.special_rsc = [
            c"ship.vux.graphics.limpets.large".as_ptr(),
            c"ship.vux.graphics.limpets.medium".as_ptr(),
            c"ship.vux.graphics.limpets.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.vux.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.vux.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.vux.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (50 << ONE_SHIFT) as u16;
        d
    }
    static mut DESC: RACE_DESC = descriptor();
    #[no_mangle]
    pub extern "C" fn init_vux() -> *mut RACE_DESC {
        unsafe {
            DESC.preprocess_func = Some(vux_preprocess);
            DESC.postprocess_func = Some(vux_postprocess);
            DESC.init_weapon_func = Some(initialize_horrific_laser);
            DESC.cyborg_control.intelligence_func = Some(vux_intelligence);
        }
        &raw mut DESC
    }
    unsafe extern "C" fn limpet_preprocess(e: *mut ELEMENT) {
        let original = normalize_facing(angle_to_facing(i32::from((*e).velocity.TravelAngle)));
        let mut face = original;
        let delta = TrackShip(e, &mut face);
        if delta > 0 {
            face = (i32::from(original) + i32::from(delta)) as COUNT;
            SetVelocityVector(&raw mut (*e).velocity, 25, face);
        }
        (*e).next.image.frame = IncFrameIndex((*e).next.image.frame);
        (*e).state_flags |= CHANGING as u16;
    }
    unsafe extern "C" fn limpet_collision(
        a: *mut ELEMENT,
        _pa: *mut POINT,
        b: *mut ELEMENT,
        _pb: *mut POINT,
    ) {
        if (*b).state_flags & PLAYER_SHIP as u16 != 0 {
            let other = starship(b);
            let rd = (*other).RaceDescPtr;
            (*rd).characteristics.turn_wait = (*rd).characteristics.turn_wait.wrapping_add(1);
            if (*rd).characteristics.turn_wait == 0 {
                (*rd).characteristics.turn_wait = (*rd).characteristics.turn_wait.wrapping_sub(1);
            }
            (*rd).characteristics.thrust_wait = (*rd).characteristics.thrust_wait.wrapping_add(1);
            if (*rd).characteristics.thrust_wait == 0 {
                (*rd).characteristics.thrust_wait =
                    (*rd).characteristics.thrust_wait.wrapping_sub(1);
            }
            if (*rd).characteristics.thrust_increment <= 1 << ONE_SHIFT {
                (*rd).characteristics.max_thrust =
                    (*rd).characteristics.thrust_increment.wrapping_shl(1);
            } else {
                let n = (*rd).characteristics.max_thrust / (*rd).characteristics.thrust_increment;
                (*rd).characteristics.thrust_increment -= 1;
                (*rd).characteristics.max_thrust =
                    (*rd).characteristics.thrust_increment.wrapping_mul(n);
            }
            (*rd).cyborg_control.ManeuverabilityIndex = 0;
            let ss = starship(a);
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 2),
                b,
            );
            let mut stamp: STAMP = mem::zeroed();
            stamp.frame = SetAbsFrameIndex(
                (*(*ss).RaceDescPtr).ship_data.weapon[0],
                TFB_Random() as COUNT,
            );
            ModifySilhouette(b, &mut stamp, MODIFY_IMAGE as BYTE);
        }
        (*a).__bindgen_anon_2.hit_points = 0;
        (*a).__bindgen_anon_1.life_span = 0;
        (*a).state_flags |= (COLLISION | DISAPPEARING) as u16;
    }
    unsafe extern "C" fn spawn_limpets(e: *mut ELEMENT) {
        let ss = starship(e);
        let mut block = MISSILE_BLOCK {
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast(),
            // The C expression adds HALF_CIRCLE directly to the facing.
            face: (i32::from((*ss).ShipFacing) + HALF_CIRCLE as i32) as COUNT,
            index: 0,
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 8,
            speed: 25,
            hit_points: 1,
            damage: 0,
            life: 80,
            preprocess_func: Some(limpet_preprocess),
            blast_offs: 0,
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
        };
        let h = initialize_missile(&mut block);
        if !h.is_null() {
            let limpet = element(h);
            (*limpet).collision_func = Some(limpet_collision);
            (*limpet).pParent = ss.cast();
            put_element(h);
        }
    }
    unsafe extern "C" fn initialize_horrific_laser(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let face = (*ss).ShipFacing;
        let mut block = LASER_BLOCK {
            face,
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            ex: cosine(facing_to_angle(i32::from(face)), 162 << ONE_SHIFT),
            ey: sine(facing_to_angle(i32::from(face)), 162 << ONE_SHIFT),
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 12,
            color: Color {
                r: 82,
                g: 255,
                b: 82,
                a: 255,
            },
        };
        *out = initialize_laser(&mut block);
        1
    }
    unsafe extern "C" fn vux_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*weapon).ObjectPtr.is_null() && (*weapon).MoveState == MOVEMENT_STATE_ENTICE {
            (*weapon).MoveState = if (*(*weapon).ObjectPtr).state_flags & FINITE_LIFE as u16 != 0
                && (*(*weapon).ObjectPtr).state_flags & CREW_OBJECT as u16 == 0
            {
                MOVEMENT_STATE_AVOID
            } else {
                MOVEMENT_STATE_PURSUE
            };
        }
        ship_intelligence(ship, concerns, count);
        let ss = starship(ship);
        if (*ss).special_counter == 0
            && !(*enemy).ObjectPtr.is_null()
            && (*enemy).which_turn <= 12
            && (*ss).ship_input_state & (LEFT | RIGHT) as u8 != 0
            && (*(*ss).RaceDescPtr).ship_info.energy_level
                >= (*(*ss).RaceDescPtr).ship_info.max_energy >> 1
        {
            (*ss).ship_input_state |= SPECIAL as u8;
        } else {
            (*ss).ship_input_state &= !(SPECIAL as u8);
        }
    }
    unsafe extern "C" fn vux_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && DeltaEnergy(e, -2) != 0
        {
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                e,
            );
            spawn_limpets(e);
            (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
        }
    }
    unsafe extern "C" fn vux_preprocess(e: *mut ELEMENT) {
        if (*e).state_flags & APPEARING as u16 != 0 {
            let ss = starship(e);
            let mut face = (*ss).ShipFacing;
            if GlobData.Game_state.CurrentActivity as u8 != IN_ENCOUNTER as u8
                && TrackShip(e, &mut face) >= 0
            {
                let other = element((*e).hTarget);
                const ENTRY: i32 = ((150 + 12 + 46) * 2) << ONE_SHIFT;
                loop {
                    (*e).current.location.x = (i32::from((*other).current.location.x)
                        - (ENTRY >> 1)
                        + i32::from(TFB_Random() as COUNT) % ENTRY)
                        as COORD;
                    (*e).current.location.y = (i32::from((*other).current.location.y)
                        - (ENTRY >> 1)
                        + i32::from(TFB_Random() as COUNT) % ENTRY)
                        as COORD;
                    let dx = (*other)
                        .current
                        .location
                        .x
                        .wrapping_sub((*e).current.location.x);
                    let dy = (*other)
                        .current
                        .location
                        .y
                        .wrapping_sub((*e).current.location.y);
                    face = normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy))));
                    (*e).current.image.frame = SetAbsFrameIndex((*e).current.image.frame, face);
                    let x = i32::from((*e).current.location.x) & !((1 << ONE_SHIFT) - 1);
                    let y = i32::from((*e).current.location.y) & !((1 << ONE_SHIFT) - 1);
                    let width = (ScreenWidth - 64) << (ONE_SHIFT + 3);
                    let height = ScreenHeight << (ONE_SHIFT + 3);
                    (*e).current.location.x = (if x < 0 {
                        x + width
                    } else if x >= width {
                        x - width
                    } else {
                        x
                    }) as COUNT as COORD;
                    (*e).current.location.y = (if y < 0 {
                        y + height
                    } else if y >= height {
                        y - height
                    } else {
                        y
                    }) as COUNT as COORD;
                    if CalculateGravity(e) == 0 && TimeSpaceMatterConflict(e) == 0 {
                        break;
                    }
                }
                (*e).hTarget = ptr::null_mut();
                (*e).next = (*e).current;
                init_intersect_points(e);
                (*e).IntersectControl.IntersectStamp.frame =
                    SetEquFrameIndex(*(*e).next.image.farray, (*e).next.image.frame);
                (*ss).ShipFacing = face;
            }
            (*(*ss).RaceDescPtr).preprocess_func = None;
        }
    }
}
