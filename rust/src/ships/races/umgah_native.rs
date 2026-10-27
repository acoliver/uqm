//! Umgah callbacks and descriptor translated from `umgah.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::memory::{rust_hfree, rust_hmalloc};
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_index, normalize_angle,
        rel_frame_index, sine, starship,
    };
    use std::mem;

    const MAX_CREW: u16 = 10;
    const MAX_ENERGY: u8 = 30;
    const MAX_THRUST: u16 = 18;
    const THRUST_INCREMENT: u16 = 6;
    const JUMP_DIST: i32 = 40 << ONE_SHIFT;
    const CLOSE_RANGE_WEAPON: u16 = 50 << ONE_SHIFT;
    const LONG_RANGE_WEAPON: u32 = 1000 << ONE_SHIFT;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub(crate) struct UmgahData {
        pub(crate) prev_facing: u16,
    }

    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE | IMMEDIATE_WEAPON) as u16;
        d.ship_info.ship_cost = 7;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.umgah.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.umgah.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.umgah.meleeicons".as_ptr();
        d.fleet.strength = 833 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet.known_loc = POINT { x: 1798, y: 6000 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: MAX_ENERGY,
            weapon_energy_cost: 0,
            special_energy_cost: 1,
            energy_wait: 150,
            turn_wait: 4,
            thrust_wait: 3,
            weapon_wait: 0,
            special_wait: 2,
            ship_mass: 1,
        };
        d.ship_data.ship_rsc = [
            c"ship.umgah.graphics.drone.large".as_ptr(),
            c"ship.umgah.graphics.drone.medium".as_ptr(),
            c"ship.umgah.graphics.drone.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.umgah.graphics.spritz".as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        ];
        d.ship_data.special_rsc = [
            c"ship.umgah.graphics.cone.large".as_ptr(),
            c"ship.umgah.graphics.cone.medium".as_ptr(),
            c"ship.umgah.graphics.cone.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.umgah.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.umgah.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.umgah.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (LONG_RANGE_WEAPON << 2) as u16;
        d
    }

    static mut UMGAH_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_umgah() -> *mut RACE_DESC {
        unsafe {
            // GetCodeResData copies this return value into its owned descriptor.
            // Rebuild the template so each capture starts with private, empty data.
            UMGAH_DESC = descriptor();
            UMGAH_DESC.uninit_func = Some(uninit_umgah);
            UMGAH_DESC.preprocess_func = Some(umgah_preprocess);
            UMGAH_DESC.postprocess_func = Some(umgah_postprocess);
            UMGAH_DESC.init_weapon_func = Some(initialize_cone);
            UMGAH_DESC.cyborg_control.intelligence_func = Some(umgah_intelligence);
        }
        &raw mut UMGAH_DESC
    }

    unsafe fn set_data(desc: *mut RACE_DESC, value: Option<UmgahData>) {
        if !(*desc).data.is_null() {
            rust_hfree((*desc).data);
            (*desc).data = std::ptr::null_mut();
        }
        if let Some(v) = value {
            let p = rust_hmalloc(mem::size_of::<UmgahData>()).cast::<UmgahData>();
            *p = v;
            (*desc).data = p.cast();
        }
    }

    pub(crate) unsafe extern "C" fn cone_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        (*(*ss).RaceDescPtr).ship_data.special[0] = rel_frame_index(
            (*(*ss).RaceDescPtr).ship_data.special[0],
            angle_to_facing(FULL_CIRCLE as i32) as i16,
        );
        (*e).state_flags |= APPEARING as u16;
    }

    pub(crate) unsafe extern "C" fn cone_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        let h = weapon_collision(a, pa, b, pb);
        if !h.is_null() {
            RemoveElement(h);
            FreeElement(h);
            (*a).state_flags &= !(DISAPPEARING as u16);
        }
    }

    pub(crate) unsafe extern "C" fn initialize_cone(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let desc = (*ss).RaceDescPtr;
        let facing = (*ss).ShipFacing;
        let mut b = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*desc).ship_data.special).cast(),
            face: facing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 100,
            damage: 1,
            life: 1,
            preprocess_func: Some(cone_preprocess),
            blast_offs: 0,
            index: 0,
        };
        let data = (*desc).data.cast::<UmgahData>();
        if data.is_null() || facing != (*data).prev_facing {
            set_data(
                desc,
                Some(UmgahData {
                    prev_facing: facing,
                }),
            );
            (*desc).ship_data.special[0] = SetAbsFrameIndex((*desc).ship_data.special[0], facing);
        }
        b.index = frame_index((*desc).ship_data.special[0]);
        *out = initialize_missile(&mut b);
        if !(*out).is_null() {
            let cone = element(*out);
            (*cone).collision_func = Some(cone_collision);
            (*cone).state_flags &= !(APPEARING as u16);
            (*cone).next = (*cone).current;
            (*cone).IntersectControl.IntersectStamp.origin = POINT {
                x: (*cone).current.location.x >> ONE_SHIFT,
                y: (*cone).current.location.y >> ONE_SHIFT,
            };
            (*cone).IntersectControl.EndPoint = POINT {
                x: (*cone).next.location.x >> ONE_SHIFT,
                y: (*cone).next.location.y >> ONE_SHIFT,
            };
            (*cone).IntersectControl.IntersectStamp.frame = (*desc).ship_data.special[0];
        }
        1
    }

    pub(crate) unsafe extern "C" fn umgah_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*e).state_flags & APPEARING as u16 != 0 {
            set_data((*ss).RaceDescPtr, None);
        } else if (*e).__bindgen_anon_5.thrust_wait == 0
            && (*ss).cur_status_flags & SPECIAL as u16 != 0
            && DeltaEnergy(e, -1) != 0
        {
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                e,
            );
            let facing = facing_to_angle(i32::from((*ss).ShipFacing)) + HALF_CIRCLE as i32;
            let speed = JUMP_DIST << VELOCITY_SHIFT;
            DeltaVelocityComponents(
                &raw mut (*e).velocity,
                cosine(facing, speed),
                sine(facing, speed),
            );
            (*ss).cur_status_flags &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
            (*ss).special_counter = 2;
        }
    }

    pub(crate) unsafe extern "C" fn umgah_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).special_counter > 0 {
            (*ss).special_counter = 0;
            (*e).velocity = mem::zeroed();
        }
    }

    pub(crate) unsafe extern "C" fn uninit_umgah(desc: *mut RACE_DESC) {
        set_data(desc, None);
    }

    pub(crate) unsafe extern "C" fn umgah_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*weapon).ObjectPtr.is_null() && (*weapon).MoveState == MOVEMENT_STATE_ENTICE {
            if (*weapon).which_turn > 3 || (*ss).old_status_flags & SPECIAL as u16 != 0 {
                (*weapon).ObjectPtr = std::ptr::null_mut();
            } else if (*(*weapon).ObjectPtr).state_flags & FINITE_LIFE as u16 != 0
                && (*(*weapon).ObjectPtr).state_flags & CREW_OBJECT as u16 == 0
            {
                (*weapon).MoveState = MOVEMENT_STATE_AVOID;
            } else {
                (*weapon).MoveState = MOVEMENT_STATE_PURSUE;
            }
        }
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        let gravity = concerns.add(GRAVITY_MASS_INDEX as usize);
        if (*ss).special_counter != 0
            || !(*gravity).ObjectPtr.is_null()
            || (*enemy).ObjectPtr.is_null()
        {
            (*(*ss).RaceDescPtr).cyborg_control.WeaponRange = CLOSE_RANGE_WEAPON;
            ship_intelligence(ship, concerns, count);
            if (*enemy).which_turn < 16 {
                (*ss).ship_input_state |= WEAPON as u8;
            }
            (*ss).ship_input_state &= !(SPECIAL as u8);
        } else {
            let turn = (*enemy).which_turn.min(255) as u8;
            let enough = ((JUMP_DIST * i32::from((*(*ss).RaceDescPtr).ship_info.energy_level))
                >> 6)
                > i32::from(turn);
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
            let behind = normalize_angle(
                ARCTAN(dx, dy) as i32
                    - (facing_to_angle(i32::from((*ss).ShipFacing)) + HALF_CIRCLE as i32)
                    + OCTANT as i32
                    + (OCTANT >> 2) as i32,
            ) <= ((OCTANT as i32 + (OCTANT >> 2) as i32) << 1);
            let enemy_ss = starship((*enemy).ObjectPtr);
            let desc = (*ss).RaceDescPtr;
            (*desc).cyborg_control.WeaponRange = if enough
                && ((*ss).old_status_flags & SPECIAL as u16 != 0
                    || behind
                    || (turn > 6
                        && (*(*enemy_ss).RaceDescPtr)
                            .cyborg_control
                            .ManeuverabilityIndex
                            <= SLOW_SHIP as u16)
                    || (16..=24).contains(&turn))
            {
                (LONG_RANGE_WEAPON << 3) as u16
            } else {
                CLOSE_RANGE_WEAPON
            };
            ship_intelligence(ship, concerns, count);
            if (*desc).cyborg_control.WeaponRange == CLOSE_RANGE_WEAPON {
                (*ss).ship_input_state &= !(SPECIAL as u8);
            } else {
                (*ss).ship_input_state &= !(THRUST as u8);
                let lined = (*ship).__bindgen_anon_4.turn_wait == 0
                    && (*ss).old_status_flags & (LEFT | RIGHT) as u16 == 0;
                if ((*ss).old_status_flags & SPECIAL as u16 != 0
                    && turn <= (*desc).characteristics.special_wait)
                    || ((*ss).old_status_flags & SPECIAL as u16 == 0
                        && behind
                        && (lined || turn < 16))
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                    (*desc).characteristics.special_wait = turn;
                    if turn <= 8 && lined {
                        if TFB_Random() & 1 != 0 {
                            (*ss).ship_input_state |= LEFT as u8;
                        } else {
                            (*ss).ship_input_state |= RIGHT as u8;
                        }
                    }
                } else if (*ss).old_status_flags & SPECIAL as u16 != 0 {
                    (*ss).ship_input_state &= !((SPECIAL | LEFT | RIGHT) as u8);
                    (*ss).ship_input_state |= THRUST as u8;
                }
            }
            if turn < 16 && !behind {
                (*ss).ship_input_state |= WEAPON as u8;
            }
        }
        if (*ss).ship_input_state & SPECIAL as u8 == 0 {
            (*(*ss).RaceDescPtr).characteristics.special_wait = 255;
        }
    }
}
