//! Live utwig callbacks translated from ships/utwig/utwig.c.
#[cfg(not(test))]
mod engine {
    use crate::ships::native::{abi::*, cosine, facing_to_angle, sine, starship};
    use std::{mem, ptr};
    const fn color(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }
    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.ship_flags =
            (FIRES_FORE as i32 | POINT_DEFENSE as i32 | SHIELD_DEFENSE as i32) as _;
        d.ship_info.ship_cost = 22;
        d.ship_info.crew_level = 20;
        d.ship_info.max_crew = 20;
        d.ship_info.energy_level = ((20) >> 1) as _;
        d.ship_info.max_energy = 20;
        d.ship_info.race_strings_rsc = c"ship.utwig.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.utwig.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.utwig.meleeicons".as_ptr();
        d.fleet.strength = (666 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet.known_loc = POINT { x: 8534, y: 8797 };
        d.characteristics.max_thrust = 36;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 0;
        d.characteristics.weapon_energy_cost = 0;
        d.characteristics.special_energy_cost = 1;
        d.characteristics.energy_wait = 255;
        d.characteristics.turn_wait = 1;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 7;
        d.characteristics.special_wait = 12;
        d.characteristics.ship_mass = 8;
        d.ship_data.ship_rsc = [
            c"ship.utwig.graphics.jugger.large".as_ptr(),
            c"ship.utwig.graphics.jugger.medium".as_ptr(),
            c"ship.utwig.graphics.jugger.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.utwig.graphics.lance.large".as_ptr(),
            c"ship.utwig.graphics.lance.medium".as_ptr(),
            c"ship.utwig.graphics.lance.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [std::ptr::null(), std::ptr::null(), std::ptr::null()];
        d.ship_data.captain_control.captain_rsc = c"ship.utwig.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.utwig.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.utwig.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (50 << ONE_SHIFT) as u16;
        d
    }
    static mut DESC: RACE_DESC = descriptor();
    #[no_mangle]
    pub extern "C" fn init_utwig() -> *mut RACE_DESC {
        unsafe {
            DESC.preprocess_func = Some(utwig_preprocess);
            DESC.init_weapon_func = Some(initialize_lance);
            DESC.cyborg_control.intelligence_func = Some(utwig_intelligence);
        }
        &raw mut DESC
    }
    unsafe extern "C" fn initialize_lance(ship: *mut ELEMENT, out: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            speed: 30 << ONE_SHIFT,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess_func: None,
            blast_offs: 1,
            pixoffs: 0,
            cx: 0,
            cy: 0,
        };
        for (i, (x, y)) in [(5, -18), (13, -9), (17, -4)].into_iter().enumerate() {
            let angle = (facing_to_angle(i32::from(block.face)) + QUADRANT as i32) as COUNT;
            let cc = cosine(i32::from(angle), x << ONE_SHIFT);
            let ssin = sine(i32::from(angle), y << ONE_SHIFT);
            let sc = sine(i32::from(angle), x << ONE_SHIFT);
            let cs = cosine(i32::from(angle), y << ONE_SHIFT);
            let cos0 = cc.wrapping_sub(ssin);
            let sin0 = sc.wrapping_add(cs);
            block.cx = (*ship).next.location.x.wrapping_add(cos0);
            block.cy = (*ship).next.location.y.wrapping_add(sin0);
            *out.add(i * 2) = initialize_missile(&mut block);
            let cos0 = (-(i32::from(cc)) - i32::from(ssin)) as SIZE;
            let sin0 = (-(i32::from(sc)) + i32::from(cs)) as SIZE;
            block.cx = (*ship).next.location.x.wrapping_add(cos0);
            block.cy = (*ship).next.location.y.wrapping_add(sin0);
            *out.add(i * 2 + 1) = initialize_missile(&mut block);
        }
        6
    }

    unsafe extern "C" fn utwig_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        let mut shield: SIZE = 0;
        if (*(*ss).RaceDescPtr).ship_info.energy_level != 0 {
            shield = -1;
            if !(*weapon).ObjectPtr.is_null() && (*weapon).MoveState == MOVEMENT_STATE_ENTICE {
                shield = 0;
                let object = (*weapon).ObjectPtr;
                if (*object).state_flags & FINITE_LIFE as u16 == 0 {
                    (*weapon).MoveState = MOVEMENT_STATE_PURSUE;
                } else if (*object).__bindgen_anon_3.mass_points != 0
                    || (*object).state_flags & CREW_OBJECT as u16 != 0
                {
                    (*weapon).which_turn >>= 1;
                    if (*weapon).which_turn == 0 {
                        (*weapon).which_turn = 1;
                    }
                    if (*object).__bindgen_anon_3.mass_points != 0 {
                        (*weapon).ObjectPtr = ptr::null_mut();
                    } else {
                        (*weapon).MoveState = MOVEMENT_STATE_PURSUE;
                    }
                    shield = 1;
                }
            }
        }
        if (*ss).special_counter == 0 {
            (*ss).ship_input_state &= !(SPECIAL as u8);
            if shield != 0 {
                if (shield > 0 || !(*weapon).ObjectPtr.is_null())
                    && (*weapon).which_turn <= 2
                    && (shield > 0
                        || (*(*weapon).ObjectPtr).state_flags & PLAYER_SHIP as u16 != 0
                        || PlotIntercept((*weapon).ObjectPtr, ship, 2, 0) != 0)
                    && TFB_Random() & 3 != 0
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                    (*ss).ship_input_state &= !(WEAPON as u8);
                }
                (*weapon).ObjectPtr = ptr::null_mut();
            }
        }
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if (*(*ss).RaceDescPtr).ship_info.energy_level != 0 && !(*enemy).ObjectPtr.is_null() {
            let other = starship((*enemy).ObjectPtr);
            if (*(*other).RaceDescPtr).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 == 0 {
                (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
            }
        }
        ship_intelligence(ship, concerns, count);
    }

    unsafe extern "C" fn utwig_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        if u32::from((*a).__bindgen_anon_1.life_span) > NORMAL_LIFE
            && (*b).state_flags & FINITE_LIFE as u16 != 0
            && (*b).__bindgen_anon_3.mass_points != 0
        {
            (*a).__bindgen_anon_1.life_span = (*a)
                .__bindgen_anon_1
                .life_span
                .wrapping_add(u16::from((*b).__bindgen_anon_3.mass_points));
        }
        collision(a, pa, b, pb);
    }

    unsafe extern "C" fn utwig_preprocess(e: *mut ELEMENT) {
        if (*e).state_flags & APPEARING as u16 != 0 {
            (*e).collision_func = Some(utwig_collision);
        }
        let ss = starship(e);
        if u32::from((*e).__bindgen_anon_1.life_span) > NORMAL_LIFE + 1 {
            DeltaEnergy(
                e,
                (u32::from((*e).__bindgen_anon_1.life_span) - (NORMAL_LIFE + 1)) as SIZE,
            );
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                e,
            );
        }
        if (*ss).cur_status_flags & SPECIAL as u16 == 0 {
            (*ss).special_counter = 0;
        } else if (*ss).special_counter.is_multiple_of(6) {
            if DeltaEnergy(e, -1) == 0 {
                (*(*ss).RaceDescPtr).ship_info.ship_flags &=
                    !((POINT_DEFENSE | SHIELD_DEFENSE) as u16);
            } else if (*ss).special_counter == 0 {
                (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
                ProcessSound(
                    SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 2),
                    e,
                );
            }
        }
        let prim = GlobData
            .Game_state
            .DisplayArray
            .add((*e).PrimIndex as usize);
        if (*ss).special_counter == 0 {
            (*prim).color = color(0x1f, 0x1c, 0);
            (*e).colorCycleIndex = 0;
            (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
            (*prim).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        } else {
            const COLORS: [Color; 7] = [
                color(0x1f, 0x15, 0),
                color(0x1f, 0x0e, 0),
                color(0x1f, 7, 0),
                color(0x1f, 0, 0),
                color(0x17, 0, 0),
                color(0x13, 0, 0),
                color(0x0f, 0, 0),
            ];
            if (*ss).weapon_counter == 0 {
                (*ss).weapon_counter += 1;
            }
            (*e).colorCycleIndex = (*e).colorCycleIndex.wrapping_add(1);
            if usize::from((*e).colorCycleIndex) == COLORS.len() + 1 {
                (*e).colorCycleIndex = 1;
            }
            (*prim).color = COLORS[usize::from((*e).colorCycleIndex) - 1];
            (*e).__bindgen_anon_1.life_span = (NORMAL_LIFE + 1) as COUNT;
            (*prim).Type = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
        }
    }
}
