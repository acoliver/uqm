//! Ur-Quan live fusion and fighter callbacks, translated from ships/urquan/urquan.c.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_index, normalize_angle,
        normalize_facing, put_element, sine, starship, wrap_delta,
    };
    use std::ptr;
    const fn color(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }
    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE as i32 | SEEKING_SPECIAL as i32) as _;
        d.ship_info.ship_cost = 30;
        d.ship_info.crew_level = (MAX_CREW_SIZE as i32) as _;
        d.ship_info.max_crew = (MAX_CREW_SIZE as i32) as _;
        d.ship_info.energy_level = (MAX_ENERGY_SIZE as i32) as _;
        d.ship_info.max_energy = (MAX_ENERGY_SIZE as i32) as _;
        d.ship_info.race_strings_rsc = c"ship.urquan.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.urquan.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.urquan.meleeicons".as_ptr();
        d.fleet.strength = (2666 / SPHERE_RADIUS_INCREMENT as i32 * 2) as u16;
        d.fleet.known_loc = POINT { x: 5750, y: 6000 };
        d.characteristics.max_thrust = 30;
        d.characteristics.thrust_increment = 6;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 6;
        d.characteristics.special_energy_cost = 8;
        d.characteristics.energy_wait = 4;
        d.characteristics.turn_wait = 4;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 6;
        d.characteristics.special_wait = 9;
        d.characteristics.ship_mass = 10;
        d.ship_data.ship_rsc = [
            c"ship.urquan.graphics.dreadnought.large".as_ptr(),
            c"ship.urquan.graphics.dreadnought.medium".as_ptr(),
            c"ship.urquan.graphics.dreadnought.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.urquan.graphics.fusion.large".as_ptr(),
            c"ship.urquan.graphics.fusion.medium".as_ptr(),
            c"ship.urquan.graphics.fusion.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.urquan.graphics.fighter.large".as_ptr(),
            c"ship.urquan.graphics.fighter.medium".as_ptr(),
            c"ship.urquan.graphics.fighter.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.urquan.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.urquan.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.urquan.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (((20) << ONE_SHIFT as i32) * (20)) as u16;
        d
    }
    static mut DESC: RACE_DESC = descriptor();
    #[no_mangle]
    pub extern "C" fn init_urquan() -> *mut RACE_DESC {
        unsafe {
            DESC.postprocess_func = Some(urquan_postprocess);
            DESC.init_weapon_func = Some(initialize_fusion);
            DESC.cyborg_control.intelligence_func = Some(urquan_intelligence);
        }
        &raw mut DESC
    }
    pub(crate) unsafe extern "C" fn initialize_fusion(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
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
            pixoffs: 32,
            speed: 20 << ONE_SHIFT,
            hit_points: 10,
            damage: 6,
            life: 20,
            preprocess_func: None,
            blast_offs: 8,
        };
        *out = initialize_missile(&mut block);
        1
    }
    pub(crate) unsafe extern "C" fn fighter_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let face = COUNT::from((*e).__bindgen_anon_5.thrust_wait);
        let mut block = LASER_BLOCK {
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
            face,
            ex: cosine(facing_to_angle(i32::from(face)), 44 << ONE_SHIFT),
            ey: sine(facing_to_angle(i32::from(face)), 44 << ONE_SHIFT),
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 4,
            color: color(0x1f, 0x1f, 0x0a),
        };
        let h = initialize_laser(&mut block);
        if !h.is_null() {
            let laser = element(h);
            (*laser).pParent = ss.cast();
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 2),
                laser,
            );
            put_element(h);
        }
        (*e).postprocess_func = None;
        (*e).__bindgen_anon_5.thrust_wait = 8;
    }
    pub(crate) unsafe extern "C" fn fighter_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        (*(*ss).RaceDescPtr).characteristics.special_wait = (*(*ss).RaceDescPtr)
            .characteristics
            .special_wait
            .wrapping_add(1);
        if 400 - i32::from((*e).__bindgen_anon_1.life_span) > 6
            && (*e).state_flags & CHANGING as u16 == 0
        {
            let mut enroute = true;
            let mut dx = (*(*ss).RaceDescPtr).ship_info.crew_level as SIZE;
            let mut dy = (*e).__bindgen_anon_1.life_span as SIZE;
            let orig = frame_index((*e).current.image.frame);
            let mut face = orig;
            if (dy & 1 != 0 || !(*e).hTarget.is_null() || TrackShip(e, &mut face) >= 0)
                && (dx == 0 || dy >= 125)
            {
                (*e).state_flags |= IGNORE_SIMILAR as u16;
            } else if dx != 0 {
                let ship = element((*ss).hShip);
                dx = wrap_delta(
                    (*ship)
                        .current
                        .location
                        .x
                        .wrapping_sub((*e).current.location.x),
                    (ScreenWidth - 64) << (ONE_SHIFT + 3),
                );
                dy = wrap_delta(
                    (*ship)
                        .current
                        .location
                        .y
                        .wrapping_sub((*e).current.location.y),
                    ScreenHeight << (ONE_SHIFT + 3),
                );
                face = normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy))));
                (*e).state_flags &= !(IGNORE_SIMILAR as u16);
                enroute = false;
            }
            if (*e).__bindgen_anon_5.thrust_wait > 0 {
                (*e).__bindgen_anon_5.thrust_wait -= 1;
            }
            if !(*e).hTarget.is_null() {
                let target = element((*e).hTarget);
                dx = wrap_delta(
                    (*target)
                        .current
                        .location
                        .x
                        .wrapping_sub((*e).current.location.x),
                    (ScreenWidth - 64) << (ONE_SHIFT + 3),
                );
                dy = wrap_delta(
                    (*target)
                        .current
                        .location
                        .y
                        .wrapping_sub((*e).current.location.y),
                    ScreenHeight << (ONE_SHIFT + 3),
                );
                const RANGE: i32 = (44 << ONE_SHIFT) * 3 / 4;
                if (*e).__bindgen_anon_5.thrust_wait == 0
                    && i32::from(dx).abs() < RANGE
                    && i32::from(dy).abs() < RANGE
                    && i32::from(dx) * i32::from(dx) + i32::from(dy) * i32::from(dy) < RANGE * RANGE
                {
                    (*e).__bindgen_anon_5.thrust_wait =
                        normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy)))) as BYTE;
                    (*e).postprocess_func = Some(fighter_postprocess);
                }
                if enroute {
                    face = frame_index((*target).current.image.frame);
                    let offset_face = if (*e).__bindgen_anon_4.turn_wait & LEFT as u8 != 0 {
                        i32::from(face) - 4
                    } else {
                        i32::from(face) + 4
                    };
                    dx = dx.wrapping_add(cosine(facing_to_angle(offset_face), 30 << ONE_SHIFT));
                    dy = dy.wrapping_add(sine(facing_to_angle(offset_face), 30 << ONE_SHIFT));
                    face = normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy))));
                }
            }
            (*e).state_flags |= CHANGING as u16;
            if face != orig {
                image!(frame; (*e).next.image.frame, SetAbsFrameIndex((*e).next.image.frame, face));
            }
            SetVelocityVector(&raw mut (*e).velocity, 8 << ONE_SHIFT, face);
        }
    }
    pub(crate) unsafe extern "C" fn fighter_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        let ss = starship(a);
        if u32::from((*b).__bindgen_anon_3.mass_points) > MAX_SHIP_MASS * 10 {
            let h = AllocElement();
            if !h.is_null() {
                let fighter = element(h);
                let index = (*fighter).PrimIndex;
                image!(copy; *fighter, *a);
                (*fighter).PrimIndex = index;
                *GlobData.Game_state.DisplayArray.add(index as usize) = *GlobData
                    .Game_state
                    .DisplayArray
                    .add((*a).PrimIndex as usize);
                (*fighter).state_flags &= !(PRE_PROCESS as u16);
                (*fighter).state_flags |= CHANGING as u16;
                image!(copy; (*fighter).next, (*fighter).current);
                let mut travel = (*fighter).velocity.travel_angle;
                let delta = normalize_angle(
                    i32::from(ARCTAN(
                        (*pb).x.wrapping_sub((*pa).x),
                        (*pb).y.wrapping_sub((*pa).y),
                    )) - i32::from(travel),
                ) as SIZE;
                if (delta == 0 && (*fighter).__bindgen_anon_4.turn_wait & LEFT as u8 != 0)
                    || (delta != 0 && i32::from(delta) <= HALF_CIRCLE as i32)
                {
                    travel = travel.wrapping_sub(QUADRANT as COUNT);
                } else {
                    travel = travel.wrapping_add(QUADRANT as COUNT);
                }
                travel = normalize_facing(angle_to_facing(normalize_angle(i32::from(travel))));
                image!(frame; (*fighter).next.image.frame, SetAbsFrameIndex((*fighter).next.image.frame, travel));
                SetVelocityVector(&raw mut (*fighter).velocity, 8 << ONE_SHIFT, travel);
                put_element(h);
            }
            (*a).state_flags |= (DISAPPEARING | COLLISION) as u16;
        } else if (*a).pParent != (*b).pParent {
            (*a).__bindgen_anon_5.blast_offset = 0;
            weapon_collision(a, pa, b, pb);
            (*a).state_flags |= (DISAPPEARING | COLLISION) as u16;
        } else if (*b).state_flags & PLAYER_SHIP as u16 != 0 {
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 3),
                b,
            );
            DeltaCrew(b, 1);
            (*a).state_flags |= (DISAPPEARING | COLLISION) as u16;
        }
        if (*a).state_flags & DISAPPEARING as u16 != 0 {
            (*a).state_flags &= !(DISAPPEARING as u16);
            (*a).__bindgen_anon_2.hit_points = 0;
            (*a).__bindgen_anon_1.life_span = 0;
            (*a).state_flags |= NONSOLID as u16;
            (*(*ss).RaceDescPtr).characteristics.special_wait = (*(*ss).RaceDescPtr)
                .characteristics
                .special_wait
                .wrapping_sub(1);
        }
    }
    pub(crate) unsafe extern "C" fn spawn_fighters(e: *mut ELEMENT) {
        let ss = starship(e);
        let face = (i32::from((*ss).ShipFacing) + angle_to_facing(HALF_CIRCLE as i32)) as COUNT;
        let dx = cosine(facing_to_angle(i32::from(face)), 14 << ONE_SHIFT);
        let dy = sine(facing_to_angle(i32::from(face)), 14 << ONE_SHIFT);
        let mut i = if (*e).__bindgen_anon_2.crew_level > 2 {
            2
        } else {
            1
        };
        while i != 0 {
            i -= 1;
            let h = AllocElement();
            if h.is_null() {
                break;
            }
            DeltaCrew(e, -1);
            put_element(h);
            let fighter = element(h);
            (*fighter).__bindgen_anon_2.hit_points = 1;
            (*fighter).__bindgen_anon_3.mass_points = 0;
            (*fighter).__bindgen_anon_5.thrust_wait = 7;
            (*fighter).playerNr = (*e).playerNr;
            (*fighter).state_flags =
                (APPEARING | FINITE_LIFE | CREW_OBJECT | IGNORE_SIMILAR) as u16;
            (*fighter).__bindgen_anon_1.life_span = 400;
            (*GlobData
                .Game_state
                .DisplayArray
                .add((*fighter).PrimIndex as usize))
            .Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            (*fighter).preprocess_func = Some(fighter_preprocess);
            (*fighter).postprocess_func = None;
            (*fighter).collision_func = Some(fighter_collision);
            (*fighter).death_func = None;
            (*fighter).current.location = (*e).next.location;
            let fighter_face;
            if i == 1 {
                (*fighter).__bindgen_anon_4.turn_wait = LEFT as BYTE;
                fighter_face = normalize_facing(i32::from(face) + 2);
                (*fighter).current.location.x = (i32::from((*fighter).current.location.x)
                    + i32::from(dx)
                    - i32::from(dy)) as COORD;
                (*fighter).current.location.y = (i32::from((*fighter).current.location.y)
                    + i32::from(dy)
                    + i32::from(dx)) as COORD;
            } else {
                (*fighter).__bindgen_anon_4.turn_wait = RIGHT as BYTE;
                fighter_face = normalize_facing(i32::from(face) - 2);
                (*fighter).current.location.x = (i32::from((*fighter).current.location.x)
                    + i32::from(dx)
                    + i32::from(dy)) as COORD;
                (*fighter).current.location.y = (i32::from((*fighter).current.location.y)
                    + i32::from(dy)
                    - i32::from(dx)) as COORD;
            }
            let sx = cosine(
                facing_to_angle(i32::from(fighter_face)),
                (8 << ONE_SHIFT) << VELOCITY_SHIFT,
            );
            let sy = sine(
                facing_to_angle(i32::from(fighter_face)),
                (8 << ONE_SHIFT) << VELOCITY_SHIFT,
            );
            SetVelocityComponents(&raw mut (*fighter).velocity, sx, sy);
            (*fighter).current.location.x = (*fighter)
                .current
                .location
                .x
                .wrapping_sub(sx >> VELOCITY_SHIFT);
            (*fighter).current.location.y = (*fighter)
                .current
                .location
                .y
                .wrapping_sub(sy >> VELOCITY_SHIFT);
            image!(set; (*fighter).current.image.farray, (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast());
            image!(frame; (*fighter).current.image.frame, SetAbsFrameIndex((*(*ss).RaceDescPtr).ship_data.special[0], fighter_face));
            (*fighter).pParent = ss.cast();
        }
    }
    pub(crate) unsafe extern "C" fn urquan_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*weapon).ObjectPtr.is_null()
            && (*weapon).MoveState == MOVEMENT_STATE_ENTICE
            && ((*(*weapon).ObjectPtr).state_flags & CREW_OBJECT as u16 == 0
                || (*weapon).which_turn <= 8)
            && ((*(*weapon).ObjectPtr).state_flags & FINITE_LIFE as u16 == 0
                || ((*(*weapon).ObjectPtr).__bindgen_anon_3.mass_points >= 4
                    && (*weapon).which_turn == 2
                    && (*enemy).which_turn > 16))
        {
            (*weapon).MoveState = MOVEMENT_STATE_PURSUE;
        }
        ship_intelligence(ship, concerns, count);
        let other = if !(*enemy).ObjectPtr.is_null() {
            starship((*enemy).ObjectPtr)
        } else {
            ptr::null_mut()
        };
        if (*ss).special_counter == 0
            && !(*enemy).ObjectPtr.is_null()
            && (*(*ss).RaceDescPtr).ship_info.crew_level
                > (*(*ss).RaceDescPtr).ship_info.max_crew >> 2
            && (*(*other).RaceDescPtr).ship_info.ship_flags & POINT_DEFENSE as u16 == 0
            && ((*(*ss).RaceDescPtr).characteristics.special_wait < 6
                || ((*(*other).RaceDescPtr).cyborg_control.ManeuverabilityIndex
                    <= SLOW_SHIP as COUNT
                    && (*other).cur_status_flags & SHIP_BEYOND_MAX_SPEED as u16 == 0)
                || ((*enemy).which_turn <= 12
                    && (*ss).ship_input_state & (LEFT | RIGHT) as u8 != 0
                    && (*(*ss).RaceDescPtr).ship_info.energy_level
                        >= (*(*ss).RaceDescPtr).ship_info.max_energy >> 1))
        {
            (*ss).ship_input_state |= SPECIAL as u8;
        } else {
            (*ss).ship_input_state &= !(SPECIAL as u8);
        }
        (*(*ss).RaceDescPtr).characteristics.special_wait = 0;
    }
    pub(crate) unsafe extern "C" fn urquan_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*e).__bindgen_anon_2.crew_level > 1
            && (*ss).special_counter == 0
            && DeltaEnergy(e, -8) != 0
        {
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                e,
            );
            spawn_fighters(e);
            (*ss).special_counter = 9;
        }
    }
}
