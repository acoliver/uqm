//! Live yehat callbacks translated from ships/yehat/yehat.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{abi::*, cosine, facing_to_angle, sine, starship};
    use std::ptr;
    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE as i32 | SHIELD_DEFENSE as i32) as _;
        d.ship_info.ship_cost = 23;
        d.ship_info.crew_level = 20;
        d.ship_info.max_crew = 20;
        d.ship_info.energy_level = 10;
        d.ship_info.max_energy = 10;
        d.ship_info.race_strings_rsc = c"ship.yehat.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.yehat.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.yehat.meleeicons".as_ptr();
        d.fleet.strength = (750 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet.known_loc = POINT { x: 4970, y: 40 };
        d.characteristics.max_thrust = 30;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 2;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = 3;
        d.characteristics.energy_wait = 6;
        d.characteristics.turn_wait = 2;
        d.characteristics.thrust_wait = 2;
        d.characteristics.weapon_wait = 0;
        d.characteristics.special_wait = 2;
        d.characteristics.ship_mass = 3;
        d.ship_data.ship_rsc = [
            c"ship.yehat.graphics.terminator.large".as_ptr(),
            c"ship.yehat.graphics.terminator.medium".as_ptr(),
            c"ship.yehat.graphics.terminator.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.yehat.graphics.missile.large".as_ptr(),
            c"ship.yehat.graphics.missile.medium".as_ptr(),
            c"ship.yehat.graphics.missile.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.yehat.graphics.shield.large".as_ptr(),
            c"ship.yehat.graphics.shield.medium".as_ptr(),
            c"ship.yehat.graphics.shield.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.yehat.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.yehat.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.yehat.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (((20) << ONE_SHIFT as i32) * (10) / 3) as u16;
        d
    }
    static mut DESC: RACE_DESC = descriptor();
    #[no_mangle]
    pub extern "C" fn init_yehat() -> *mut RACE_DESC {
        unsafe {
            DESC.preprocess_func = Some(yehat_preprocess);
            DESC.postprocess_func = Some(yehat_postprocess);
            DESC.init_weapon_func = Some(initialize_standard_missiles);
            DESC.cyborg_control.intelligence_func = Some(yehat_intelligence);
        }
        &raw mut DESC
    }
    pub(crate) unsafe extern "C" fn initialize_standard_missiles(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 16,
            speed: 20 << ONE_SHIFT,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess_func: None,
            blast_offs: 1,
            cx: 0,
            cy: 0,
        };
        let x = sine(facing_to_angle(i32::from(block.face)), 8 << ONE_SHIFT).wrapping_neg();
        let y = cosine(facing_to_angle(i32::from(block.face)), 8 << ONE_SHIFT);
        block.cx = (*ship).next.location.x.wrapping_add(x);
        block.cy = (*ship).next.location.y.wrapping_add(y);
        *out = initialize_missile(&mut block);
        block.cx = (*ship).next.location.x.wrapping_sub(x);
        block.cy = (*ship).next.location.y.wrapping_sub(y);
        *out.add(1) = initialize_missile(&mut block);
        2
    }
    pub(crate) unsafe extern "C" fn yehat_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let mut shield: SIZE = -1;
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*weapon).ObjectPtr.is_null() && (*weapon).MoveState == MOVEMENT_STATE_ENTICE {
            shield = 0;
            let object = (*weapon).ObjectPtr;
            if (*object).state_flags & (FINITE_LIFE | CREW_OBJECT) as u16 == 0 {
                (*weapon).MoveState = MOVEMENT_STATE_PURSUE;
            } else if (*object).__bindgen_anon_3.mass_points != 0
                || (*object).state_flags & CREW_OBJECT as u16 != 0
            {
                if (*object).state_flags & FINITE_LIFE as u16 == 0 {
                    (*weapon).which_turn = (*weapon).which_turn.wrapping_shl(1);
                } else {
                    (*weapon).which_turn >>= 1;
                    if (*weapon).which_turn == 0 {
                        (*weapon).which_turn = 1;
                    }
                    if (*object).__bindgen_anon_3.mass_points != 0 {
                        (*weapon).ObjectPtr = ptr::null_mut();
                    } else {
                        (*weapon).MoveState = MOVEMENT_STATE_PURSUE;
                    }
                }
                shield = 1;
            }
        }
        let ss = starship(ship);
        if (*ss).special_counter == 0 {
            (*ss).ship_input_state &= !(SPECIAL as u8);
            if shield != 0 {
                if u32::from((*ship).__bindgen_anon_1.life_span) <= NORMAL_LIFE + 1
                    && (shield > 0 || !(*weapon).ObjectPtr.is_null())
                    && (*weapon).which_turn <= 2
                    && (shield > 0
                        || (*(*weapon).ObjectPtr).state_flags & PLAYER_SHIP as u16 != 0
                        || PlotIntercept((*weapon).ObjectPtr, ship, 2, 0) != 0)
                    && TFB_Random() & 3 != 0
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
                if !(*weapon).ObjectPtr.is_null()
                    && (*(*weapon).ObjectPtr).state_flags & CREW_OBJECT as u16 == 0
                {
                    (*weapon).ObjectPtr = ptr::null_mut();
                }
            }
        }
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if !(*enemy).ObjectPtr.is_null() {
            let other = starship((*enemy).ObjectPtr);
            if (*(*other).RaceDescPtr).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 == 0 {
                (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
            }
        }
        ship_intelligence(ship, concerns, count);
    }
    pub(crate) unsafe extern "C" fn yehat_postprocess(e: *mut ELEMENT) {
        if (*e).state_flags & NONSOLID as u16 == 0 {
            let ss = starship(e);
            if (*ss).special_counter > 0 {
                if u32::from((*e).__bindgen_anon_1.life_span) == NORMAL_LIFE {
                    (*ss).special_counter = 0;
                } else {
                    ProcessSound(
                        SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                        e,
                    );
                    DeltaEnergy(e, -3);
                }
            }
        }
    }
    pub(crate) unsafe extern "C" fn yehat_preprocess(e: *mut ELEMENT) {
        if (*e).state_flags & APPEARING as u16 == 0 {
            let ss = starship(e);
            let expired = if u32::from((*e).__bindgen_anon_1.life_span) > NORMAL_LIFE {
                (*e).__bindgen_anon_1.life_span -= 1;
                u32::from((*e).__bindgen_anon_1.life_span) == NORMAL_LIFE
            } else {
                false
            };
            if expired
                || (u32::from((*e).__bindgen_anon_1.life_span) == NORMAL_LIFE
                    && (*e).next.image.farray
                        == (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast())
            {
                image!(set; (*e).next.image.farray, (&raw mut (*(*ss).RaceDescPtr).ship_data.ship).cast());
                image!(frame; (*e).next.image.frame, SetEquFrameIndex(
                    (*(*ss).RaceDescPtr).ship_data.ship[0],
                    (*e).next.image.frame,
                ));
                (*e).state_flags |= CHANGING as u16;
            }
            if (*ss).cur_status_flags & SPECIAL as u16 != 0 && (*ss).special_counter == 0 {
                if (*(*ss).RaceDescPtr).ship_info.energy_level < 3 {
                    DeltaEnergy(e, -3);
                } else {
                    (*e).__bindgen_anon_1.life_span = (10 + NORMAL_LIFE) as COUNT;
                    image!(set; (*e).next.image.farray, (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast());
                    image!(frame; (*e).next.image.frame, SetEquFrameIndex(
                        (*(*ss).RaceDescPtr).ship_data.special[0],
                        (*e).next.image.frame,
                    ));
                    (*e).state_flags |= CHANGING as u16;
                    (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
                }
            }
        }
    }
}
