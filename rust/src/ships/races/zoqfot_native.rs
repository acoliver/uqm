//! Live zoqfotpik callbacks translated from ships/zoqfot/zoqfot.c.
#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_count, frame_index,
        normalize_facing, put_element, sine, starship,
    };
    use std::mem;
    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.ship_flags = (FIRES_FORE as i32) as _;
        d.ship_info.ship_cost = 6;
        d.ship_info.crew_level = 10;
        d.ship_info.max_crew = 10;
        d.ship_info.energy_level = 10;
        d.ship_info.max_energy = 10;
        d.ship_info.race_strings_rsc = c"ship.zoqfotpik.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.zoqfotpik.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.zoqfotpik.meleeicons".as_ptr();
        d.fleet.strength = (320 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet.known_loc = POINT { x: 3761, y: 5333 };
        d.characteristics.max_thrust = 40;
        d.characteristics.thrust_increment = 10;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = ((10) * 3 / 4) as _;
        d.characteristics.energy_wait = 4;
        d.characteristics.turn_wait = 1;
        d.characteristics.thrust_wait = 0;
        d.characteristics.weapon_wait = 0;
        d.characteristics.special_wait = 6;
        d.characteristics.ship_mass = 5;
        d.ship_data.ship_rsc = [
            c"ship.zoqfotpik.graphics.stinger.large".as_ptr(),
            c"ship.zoqfotpik.graphics.stinger.medium".as_ptr(),
            c"ship.zoqfotpik.graphics.stinger.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.zoqfotpik.graphics.spit.large".as_ptr(),
            c"ship.zoqfotpik.graphics.spit.medium".as_ptr(),
            c"ship.zoqfotpik.graphics.spit.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.zoqfotpik.graphics.proboscis.large".as_ptr(),
            c"ship.zoqfotpik.graphics.proboscis.medium".as_ptr(),
            c"ship.zoqfotpik.graphics.proboscis.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.zoqfotpik.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.zoqfotpik.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.zoqfotpik.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (((10) << ONE_SHIFT as i32) * (10)) as u16;
        d
    }
    static mut DESC: RACE_DESC = descriptor();
    #[no_mangle]
    pub extern "C" fn init_zoqfotpik() -> *mut RACE_DESC {
        unsafe {
            DESC.postprocess_func = Some(zoqfotpik_postprocess);
            DESC.init_weapon_func = Some(initialize_spit);
            DESC.cyborg_control.intelligence_func = Some(zoqfotpik_intelligence);
        }
        &raw mut DESC
    }
    unsafe extern "C" fn spit_preprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            (*e).next.image.frame = IncFrameIndex((*e).next.image.frame);
            let mut angle = (*e).velocity.TravelAngle;
            let index = frame_index((*e).next.image.frame);
            if index == 1 {
                angle = (i32::from(angle) + (i32::from(TFB_Random() as COUNT) % 3) - 1) as COUNT;
            }
            let speed = (((i32::from(frame_count((*e).next.image.frame)) - i32::from(index))
                << ONE_SHIFT)
                << 1)
                << VELOCITY_SHIFT;
            let speed = speed as COUNT;
            SetVelocityComponents(
                &raw mut (*e).velocity,
                cosine(i32::from(angle), i32::from(speed)),
                sine(i32::from(angle), i32::from(speed)),
            );
            (*e).__bindgen_anon_4.turn_wait = 2;
            (*e).state_flags |= CHANGING as u16;
        }
    }
    unsafe extern "C" fn initialize_spit(ship: *mut ELEMENT, out: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: 0,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 13,
            speed: ((i32::from(frame_count((*(*ss).RaceDescPtr).ship_data.weapon[0])) << ONE_SHIFT)
                << 1) as SIZE,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess_func: Some(spit_preprocess),
            blast_offs: 0,
        };
        *out = initialize_missile(&mut block);
        1
    }
    unsafe extern "C" fn tongue_postprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait != 0 {
            spawn_tongue(e);
        }
    }
    unsafe extern "C" fn tongue_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        let ss = starship(a);
        if (*ss).special_counter == (*(*ss).RaceDescPtr).characteristics.special_wait {
            weapon_collision(a, pa, b, pb);
        }
        (*ss).special_counter = (*ss)
            .special_counter
            .wrapping_sub((*a).__bindgen_anon_4.turn_wait);
        (*a).__bindgen_anon_4.turn_wait = 0;
        (*a).state_flags |= NONSOLID as u16;
    }
    unsafe extern "C" fn spawn_tongue(e: *mut ELEMENT) {
        let ss = starship(e);
        let mut block = MISSILE_BLOCK {
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 1,
            damage: 12,
            life: 1,
            preprocess_func: None,
            blast_offs: 4,
        };
        let h = initialize_missile(&mut block);
        if !h.is_null() {
            let tongue = element(h);
            (*tongue).postprocess_func = Some(tongue_postprocess);
            (*tongue).collision_func = Some(tongue_collision);
            if (*e).state_flags & PLAYER_SHIP as u16 != 0 {
                (*tongue).__bindgen_anon_4.turn_wait = (*ss).special_counter;
            } else {
                (*tongue).__bindgen_anon_4.turn_wait =
                    (*e).__bindgen_anon_4.turn_wait.wrapping_sub(1);
                let mut r: RECT = mem::zeroed();
                GetFrameRect((*tongue).current.image.frame, &mut r);
                let x = ((i32::from(r.extent.width) >> 1) << ONE_SHIFT) as SIZE;
                let y = ((i32::from(r.extent.height) >> 1) << ONE_SHIFT) as SIZE;
                let angle = facing_to_angle(i32::from((*ss).ShipFacing)) as COUNT;
                if u32::from(angle) > HALF_CIRCLE && u32::from(angle) < FULL_CIRCLE {
                    (*tongue).current.location.x = (*tongue).current.location.x.wrapping_sub(x);
                } else if angle > 0 && u32::from(angle) < HALF_CIRCLE {
                    (*tongue).current.location.x = (*tongue).current.location.x.wrapping_add(x);
                }
                if u32::from(angle) < QUADRANT || u32::from(angle) > FULL_CIRCLE - QUADRANT {
                    (*tongue).current.location.y = (*tongue).current.location.y.wrapping_sub(y);
                } else if u32::from(angle) > QUADRANT && u32::from(angle) < FULL_CIRCLE - QUADRANT {
                    (*tongue).current.location.y = (*tongue).current.location.y.wrapping_add(y);
                }
            }
            (*tongue).pParent = ss.cast();
            put_element(h);
        }
    }
    unsafe extern "C" fn zoqfotpik_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        mut count: COUNT,
    ) {
        let ss = starship(ship);
        let mut tongue = false;
        if (*ss).special_counter == 0 {
            (*ss).ship_input_state &= !(SPECIAL as u8);
            let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
            if !(*enemy).ObjectPtr.is_null() && (*enemy).which_turn <= 4 {
                tongue = true;
                (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
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
                if (*ss).ShipFacing == normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy))))
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            }
        }
        (*ss).weapon_counter = (*ss).weapon_counter.wrapping_add(1);
        ship_intelligence(ship, concerns, count);
        (*ss).weapon_counter = (*ss).weapon_counter.wrapping_sub(1);
        if (*ss).weapon_counter == 0 {
            (*ss).ship_input_state &= !(WEAPON as u8);
            if !tongue {
                let mut eval = concerns.add(count as usize);
                while count != 0 {
                    count -= 1;
                    eval = eval.sub(1);
                    if !(*eval).ObjectPtr.is_null()
                        && (u32::from(count) == ENEMY_SHIP_INDEX
                            || (u32::from(count) == ENEMY_WEAPON_INDEX
                                && (*eval).MoveState != MOVEMENT_STATE_AVOID))
                        && ship_weapons(ship, (*eval).ObjectPtr, 20 << ONE_SHIFT) != 0
                    {
                        (*ss).ship_input_state |= WEAPON as u8;
                        break;
                    }
                }
            }
        }
    }
    unsafe extern "C" fn zoqfotpik_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && DeltaEnergy(e, -7) != 0
        {
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                e,
            );
            (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
        }
        if (*ss).special_counter != 0 {
            spawn_tongue(e);
        }
    }
}
