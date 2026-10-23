//! Chmmr callbacks for the live engine, ported from `ships/chmmr/chmmr.c`.

#[cfg(not(test))]
mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_index, normalize_angle,
        object_cloaked, put_element, sine, starship,
    };
    use std::{ffi::c_void, mem, ptr};

    extern "C" {
        fn InsertQueue(queue: *mut c_void, handle: *mut c_void, before: *mut c_void);
    }

    const CHMMR_OFFSET: i32 = 18;
    const LASER_RANGE: i32 = 150 << ONE_SHIFT;
    const SATELLITE_OFFSET: i32 = 64 << ONE_SHIFT;
    const DEFENSE_RANGE: u16 = 64;

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
            (FIRES_FORE | IMMEDIATE_WEAPON | SEEKING_SPECIAL | POINT_DEFENSE) as u16;
        d.ship_info.ship_cost = 30;
        d.ship_info.crew_level = MAX_CREW_SIZE as u16;
        d.ship_info.max_crew = MAX_CREW_SIZE as u16;
        d.ship_info.energy_level = MAX_ENERGY_SIZE as u8;
        d.ship_info.max_energy = MAX_ENERGY_SIZE as u8;
        d.ship_info.race_strings_rsc = c"ship.chmmr.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.chmmr.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.chmmr.meleeicons".as_ptr();
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: 35,
            thrust_increment: 7,
            energy_regeneration: 1,
            weapon_energy_cost: 2,
            special_energy_cost: 1,
            energy_wait: 1,
            turn_wait: 3,
            thrust_wait: 5,
            weapon_wait: 0,
            special_wait: 0,
            ship_mass: 10,
        };
        d.ship_data.ship_rsc = [
            c"ship.chmmr.graphics.avatar.large".as_ptr(),
            c"ship.chmmr.graphics.avatar.medium".as_ptr(),
            c"ship.chmmr.graphics.avatar.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.chmmr.graphics.muzzle.large".as_ptr(),
            c"ship.chmmr.graphics.muzzle.medium".as_ptr(),
            c"ship.chmmr.graphics.muzzle.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.chmmr.graphics.satellite.large".as_ptr(),
            c"ship.chmmr.graphics.satellite.medium".as_ptr(),
            c"ship.chmmr.graphics.satellite.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.chmmr.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.chmmr.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.chmmr.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (50 << ONE_SHIFT) as u16;
        d
    }

    static mut CHMMR_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub unsafe extern "C" fn init_chmmr() -> *mut RACE_DESC {
        CHMMR_DESC.preprocess_func = Some(chmmr_preprocess);
        CHMMR_DESC.postprocess_func = Some(chmmr_postprocess);
        CHMMR_DESC.init_weapon_func = Some(initialize_megawatt_laser);
        CHMMR_DESC.cyborg_control.intelligence_func = Some(chmmr_intelligence);
        &raw mut CHMMR_DESC
    }

    unsafe extern "C" fn animate(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            (*e).next.image.frame = IncFrameIndex((*e).current.image.frame);
            (*e).state_flags |= CHANGING as u16;
            (*e).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_5.next_turn;
        }
    }

    unsafe extern "C" fn laser_death(e: *mut ELEMENT) {
        let ss = starship(e);
        (*ss).special_counter = (*e).__bindgen_anon_4.turn_wait;
        if !(*ss).hShip.is_null() {
            let ship = element((*ss).hShip);
            let dx = (*e)
                .current
                .location
                .x
                .wrapping_sub((*ship).current.location.x);
            let dy = (*e)
                .current
                .location
                .y
                .wrapping_sub((*ship).current.location.y);
            if TFB_Random() as u8 & 7 != 0 {
                let dist = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
                let minimum = (CHMMR_OFFSET + 10) << ONE_SHIFT;
                if dist >= i64::from(minimum) * i64::from(minimum) {
                    let h = AllocElement();
                    if !h.is_null() {
                        let spot = element(h);
                        (*spot).playerNr = (*e).playerNr;
                        (*spot).state_flags =
                            (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
                        (*spot).__bindgen_anon_4.turn_wait = 0;
                        (*spot).__bindgen_anon_5.next_turn = 0;
                        (*spot).__bindgen_anon_1.life_span = 9;
                        let angle = i32::from(ARCTAN(dx, dy));
                        let magnitude = (i32::from(TFB_Random() as COUNT)
                            % (i32::from(square_root(dist as DWORD)) + 1 - minimum)
                            + minimum) as COUNT;
                        (*spot).current.location.x = (*ship)
                            .current
                            .location
                            .x
                            .wrapping_add(cosine(angle, i32::from(magnitude)));
                        (*spot).current.location.y = (*ship)
                            .current
                            .location
                            .y
                            .wrapping_add(sine(angle, i32::from(magnitude)));
                        (*spot).current.image.farray =
                            (*(*ss).RaceDescPtr).ship_data.weapon.as_mut_ptr();
                        (*spot).current.image.frame = SetAbsFrameIndex(
                            (*(*ss).RaceDescPtr).ship_data.weapon[0],
                            (angle_to_facing(FULL_CIRCLE as i32) << 1) as COUNT,
                        );
                        (*spot).preprocess_func = Some(animate);
                        (*spot).pParent = ss.cast();
                        (*GlobData
                            .Game_state
                            .DisplayArray
                            .add((*spot).PrimIndex as usize))
                        .Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                        put_element(h);
                    }
                }
            }
        }
    }

    unsafe extern "C" fn initialize_megawatt_laser(
        ship: *mut ELEMENT,
        lasers: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let face = (*ss).ShipFacing;
        let mut rect: RECT = mem::zeroed();
        GetFrameRect(
            SetAbsFrameIndex((*(*ss).RaceDescPtr).ship_data.weapon[0], face),
            &mut rect,
        );
        let mut block = LASER_BLOCK {
            cx: ((i32::from((*ship).next.location.x) & !((1 << ONE_SHIFT) - 1))
                + (i32::from(rect.corner.x) << ONE_SHIFT)) as COORD,
            cy: ((i32::from((*ship).next.location.y) & !((1 << ONE_SHIFT) - 1))
                + (i32::from(rect.corner.y) << ONE_SHIFT)) as COORD,
            ex: cosine(facing_to_angle(i32::from(face)), LASER_RANGE),
            ey: sine(facing_to_angle(i32::from(face)), LASER_RANGE),
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            face,
            color: [
                color(0x17, 0, 0),
                color(0x1f, 3, 0),
                color(0x1f, 0x11, 0),
                color(0x1f, 3, 0),
            ][(*ss).special_counter as usize],
        };
        *lasers = initialize_laser(&mut block);
        if !(*lasers).is_null() {
            let laser = element(*lasers);
            (*laser).__bindgen_anon_3.mass_points = 2;
            (*laser).death_func = Some(laser_death);
            (*laser).__bindgen_anon_4.turn_wait =
                ((u32::from((*ss).special_counter) + 1) % 4) as u8;
        }
        1
    }

    unsafe extern "C" fn chmmr_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        ship_intelligence(ship, concerns, count);
        let ss = starship(ship);
        (*ss).ship_input_state &= !(SPECIAL as u8);
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if (*ss).special_counter == 0
            && !(*enemy).ObjectPtr.is_null()
            && (*ss).ship_input_state & WEAPON as u8 == 0
            && (*enemy).which_turn > 12
            && normalize_angle(
                i32::from((*ship).velocity.TravelAngle)
                    - (i32::from((*(*enemy).ObjectPtr).velocity.TravelAngle) + HALF_CIRCLE as i32)
                    + QUADRANT as i32,
            ) > HALF_CIRCLE as i32
        {
            (*ss).ship_input_state |= SPECIAL as u8;
        }
    }

    unsafe extern "C" fn chmmr_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & WEAPON as u16 != 0 {
            let last = element(disp_q.tail);
            if last != ship
                && (*last).playerNr == (*ship).playerNr
                && (*last).state_flags & APPEARING as u16 != 0
                && (*GlobData
                    .Game_state
                    .DisplayArray
                    .add((*last).PrimIndex as usize))
                .Type
                    == gfx_object_LINE_PRIM as GRAPHICS_PRIM
                && (*ss).special_counter & 1 == 0
            {
                let h = AllocElement();
                if !h.is_null() {
                    let flash = element(h);
                    (*flash).playerNr = (*ship).playerNr;
                    (*flash).state_flags =
                        (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
                    (*flash).__bindgen_anon_1.life_span = 1;
                    (*flash).current = (*ship).next;
                    (*flash).current.image.farray =
                        (*(*ss).RaceDescPtr).ship_data.weapon.as_mut_ptr();
                    (*flash).current.image.frame = SetAbsFrameIndex(
                        (*(*ss).RaceDescPtr).ship_data.weapon[0],
                        ((*ss).ShipFacing as i32 + angle_to_facing(FULL_CIRCLE as i32)) as COUNT,
                    );
                    (*flash).pParent = ss.cast();
                    (*GlobData
                        .Game_state
                        .DisplayArray
                        .add((*flash).PrimIndex as usize))
                    .Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                    put_element(h);
                }
            }
        }
        if (*ss).cur_status_flags & SPECIAL as u16 != 0 && DeltaEnergy(ship, -1) != 0 {
            // C plays the sound at the old target before TrackShip selects one.
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                element((*ship).hTarget),
            );
            let mut facing: COUNT = 0;
            if TrackShip(ship, &mut facing) >= 0 {
                let target = element((*ship).hTarget);
                let collision_mass = i32::from((*target).__bindgen_anon_3.mass_points) + 1;
                if collision_mass <= MAX_SHIP_MASS as i32 * 10 {
                    let a = facing_to_angle(i32::from((*ss).ShipFacing));
                    let dx = (i32::from((*ship).next.location.x)
                        + i32::from(cosine(a, LASER_RANGE / 3 + (CHMMR_OFFSET << ONE_SHIFT)))
                        - i32::from((*target).next.location.x))
                        as SIZE;
                    let dy = (i32::from((*ship).next.location.y)
                        + i32::from(sine(a, LASER_RANGE / 3 + (CHMMR_OFFSET << ONE_SHIFT)))
                        - i32::from((*target).next.location.y))
                        as SIZE;
                    let angle = i32::from(ARCTAN(dx, dy));
                    let magnitude = ((12 << VELOCITY_SHIFT)
                        / i32::from((*target).__bindgen_anon_3.mass_points))
                        as COUNT;
                    DeltaVelocityComponents(
                        &mut (*target).velocity,
                        cosine(angle, i32::from(magnitude)),
                        sine(angle, i32::from(magnitude)),
                    );
                    let mut vx: SIZE = 0;
                    let mut vy: SIZE = 0;
                    GetCurrentVelocityComponents(&mut (*target).velocity, &mut vx, &mut vy);
                    let enemy_ss = starship(target);
                    let speed =
                        (i64::from(vx) * i64::from(vx) + i64::from(vy) * i64::from(vy)) as DWORD;
                    let max_velocity =
                        (i32::from((*(*enemy_ss).RaceDescPtr).characteristics.max_thrust)
                            << VELOCITY_SHIFT) as SIZE;
                    let max_speed = (i64::from(max_velocity) * i64::from(max_velocity)) as DWORD;
                    (*enemy_ss).cur_status_flags &=
                        !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                    if speed > max_speed {
                        (*enemy_ss).cur_status_flags |=
                            (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
                    } else if speed == max_speed {
                        (*enemy_ss).cur_status_flags |= SHIP_AT_MAX_SPEED as u16;
                    }
                    let offsets = [8, 17, 28, 42, 60];
                    let colors = [0x10, 0x0e, 0x0c, 9, 7];
                    for i in 0..5 {
                        let h = AllocElement();
                        if !h.is_null() {
                            let shadow = element(h);
                            (*shadow).playerNr = (*target).playerNr;
                            (*shadow).state_flags =
                                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | POST_PROCESS) as u16;
                            (*shadow).__bindgen_anon_1.life_span = 1;
                            (*shadow).current = (*target).next;
                            (*shadow).current.location.x = (*shadow)
                                .current
                                .location
                                .x
                                .wrapping_add(cosine(angle, offsets[i] << ONE_SHIFT));
                            (*shadow).current.location.y = (*shadow)
                                .current
                                .location
                                .y
                                .wrapping_add(sine(angle, offsets[i] << ONE_SHIFT));
                            (*shadow).next = (*shadow).current;
                            (*shadow).pParent = enemy_ss.cast();
                            SetVelocityComponents(&mut (*shadow).velocity, vx, vy);
                            let prim = GlobData
                                .Game_state
                                .DisplayArray
                                .add((*shadow).PrimIndex as usize);
                            (*prim).Type = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
                            (*prim).color = color(0, 0, colors[i]);
                            InsertQueue((&raw mut disp_q).cast(), h, disp_q.head);
                        }
                    }
                }
            }
        }
        (*ss).special_counter = 0;
    }

    unsafe extern "C" fn satellite_preprocess(e: *mut ELEMENT) {
        (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
        (*e).next.image.frame = SetAbsFrameIndex(
            (*e).current.image.frame,
            (frame_index((*e).current.image.frame).wrapping_add(1)) & 7,
        );
        (*e).state_flags |= CHANGING as u16;
        (*e).__bindgen_anon_4.turn_wait =
            normalize_angle(i32::from((*e).__bindgen_anon_4.turn_wait) + 1) as u8;
        let ss = starship(e);
        if !(*ss).hShip.is_null() {
            (*(*ss).RaceDescPtr).ship_info.ship_flags |= POINT_DEFENSE as u16;
            let ship = element((*ss).hShip);
            let mut dx = (i32::from((*ship).next.location.x)
                + i32::from(cosine(
                    i32::from((*e).__bindgen_anon_4.turn_wait),
                    SATELLITE_OFFSET,
                ))
                - i32::from((*e).current.location.x)) as SIZE;
            let mut dy = (i32::from((*ship).next.location.y)
                + i32::from(sine(
                    i32::from((*e).__bindgen_anon_4.turn_wait),
                    SATELLITE_OFFSET,
                ))
                - i32::from((*e).current.location.y)) as SIZE;
            let width = (ScreenWidth - 64) << (ONE_SHIFT + 3);
            let height = ScreenHeight << (ONE_SHIFT + 3);
            // WRAP_DELTA performs one adjustment, after the SIZE assignment.
            dx = if i32::from(dx).abs() <= width >> 1 {
                dx
            } else if dx < 0 {
                (width + i32::from(dx)) as SIZE
            } else {
                (i32::from(dx) - width) as SIZE
            };
            dy = if i32::from(dy).abs() <= height >> 1 {
                dy
            } else if dy < 0 {
                (height + i32::from(dy)) as SIZE
            } else {
                (i32::from(dy) - height) as SIZE
            };
            if i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
                <= i64::from(20 << ONE_SHIFT).pow(2)
            {
                SetVelocityComponents(
                    &mut (*e).velocity,
                    (i32::from(dx) << VELOCITY_SHIFT) as SIZE,
                    (i32::from(dy) << VELOCITY_SHIFT) as SIZE,
                );
            } else {
                let angle = i32::from(ARCTAN(dx, dy));
                SetVelocityComponents(
                    &mut (*e).velocity,
                    cosine(angle, (20 << ONE_SHIFT) << VELOCITY_SHIFT),
                    sine(angle, (20 << ONE_SHIFT) << VELOCITY_SHIFT),
                );
            }
        }
    }

    unsafe extern "C" fn spawn_point_defense(e: *mut ELEMENT) {
        let ss = starship(e);
        let ship = element((*ss).hShip);
        let satellite = element((*e).hTarget);
        let mut best: HELEMENT = ptr::null_mut();
        let mut best_dist: u16 = DEFENSE_RANGE + 1;
        let mut weakest: u8 = 255;
        let mut h = (*e).pred;
        while !h.is_null() {
            let object = element(h);
            let previous = (*object).pred;
            let shared_flags = (*ship).state_flags & (*object).state_flags;
            // CollisionPossible is a macro, including its asymmetric solidity test.
            if (*object).playerNr != (*ship).playerNr
                && i32::from((*object).playerNr) != NEUTRAL_PLAYER_NUM
                && (*object).state_flags & (NONSOLID | DISAPPEARING) as u16 == 0
                && shared_flags & COLLISION as u16 == 0
                && (shared_flags & IGNORE_SIMILAR as u16 == 0
                    || (*ship).pParent != (*object).pParent)
                && ((*ship).__bindgen_anon_3.mass_points != 0
                    || (*object).__bindgen_anon_3.mass_points != 0)
                && !object_cloaked(object)
            {
                let mut dx = (*object)
                    .next
                    .location
                    .x
                    .wrapping_sub((*satellite).next.location.x);
                let mut dy = (*object)
                    .next
                    .location
                    .y
                    .wrapping_sub((*satellite).next.location.y);
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                let x = (dx >> ONE_SHIFT) as UWORD;
                let y = (dy >> ONE_SHIFT) as UWORD;
                if x <= DEFENSE_RANGE && y <= DEFENSE_RANGE {
                    let dist = (u32::from(x) * u32::from(x) + u32::from(y) * u32::from(y)) as UWORD;
                    let hp = (*object).__bindgen_anon_2.hit_points;
                    if dist <= DEFENSE_RANGE * DEFENSE_RANGE
                        && (hp < u16::from(weakest)
                            || (hp == u16::from(weakest) && dist < best_dist))
                    {
                        best = h;
                        best_dist = dist;
                        weakest = hp as u8;
                    }
                }
            }
            h = previous;
        }
        if !best.is_null() {
            let object = element(best);
            let mut block = LASER_BLOCK {
                cx: (*satellite).next.location.x,
                cy: (*satellite).next.location.y,
                face: 0,
                ex: (*object)
                    .next
                    .location
                    .x
                    .wrapping_sub((*satellite).next.location.x),
                ey: (*object)
                    .next
                    .location
                    .y
                    .wrapping_sub((*satellite).next.location.y),
                sender: (*satellite).playerNr,
                flags: IGNORE_SIMILAR as u16,
                pixoffs: 0,
                color: color(0, 1, 0x1f),
            };
            let beam = initialize_laser(&mut block);
            if !beam.is_null() {
                let p = element(beam);
                (*p).pParent = ss.cast();
                (*p).hTarget = ptr::null_mut();
                put_element(beam);
                (*satellite).__bindgen_anon_5.thrust_wait = 2;
            }
        }
    }

    unsafe extern "C" fn satellite_postprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_5.thrust_wait != 0 || (*e).__bindgen_anon_1.life_span == 0 {
            (*e).__bindgen_anon_5.thrust_wait = (*e).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
        } else {
            let h = AllocElement();
            if !h.is_null() {
                put_element(h);
                let defense = element(h);
                (*defense).playerNr = (*e).playerNr;
                (*defense).state_flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
                let successor = element((*e).succ);
                (*defense).hTarget = (*successor).pred;
                (*defense).death_func = Some(spawn_point_defense);
                (*defense).pParent = starship(e).cast();
            }
        }
    }

    unsafe extern "C" fn satellite_collision(
        _: *mut ELEMENT,
        _: *mut POINT,
        _: *mut ELEMENT,
        _: *mut POINT,
    ) {
    }

    unsafe extern "C" fn satellite_death(e: *mut ELEMENT) {
        let ss = starship(e);
        (*(*ss).RaceDescPtr).ship_info.ship_flags &= !(POINT_DEFENSE as u16);
        (*e).state_flags &= !(DISAPPEARING as u16);
        (*e).state_flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
        (*e).__bindgen_anon_1.life_span = 4;
        (*e).__bindgen_anon_4.turn_wait = 1;
        (*e).__bindgen_anon_5.next_turn = 0;
        (*e).next.image.frame = SetAbsFrameIndex((*e).current.image.frame, 8);
        (*e).preprocess_func = Some(animate);
        (*e).death_func = None;
        (*e).postprocess_func = None;
        (*e).collision_func = None;
    }

    unsafe extern "C" fn spawn_satellites(e: *mut ELEMENT) {
        let ss = starship(e);
        if !(*ss).hShip.is_null() {
            let ship = element((*ss).hShip);
            for i in 0..3 {
                let h = AllocElement();
                if !h.is_null() {
                    let satellite = element(h);
                    (*satellite).playerNr = (*ship).playerNr;
                    (*satellite).state_flags = (IGNORE_SIMILAR | APPEARING | FINITE_LIFE) as u16;
                    (*satellite).__bindgen_anon_1.life_span = (NORMAL_LIFE + 1) as COUNT;
                    (*satellite).__bindgen_anon_2.hit_points = 10;
                    (*satellite).__bindgen_anon_3.mass_points = 10;
                    let angle = (i * FULL_CIRCLE as i32 + 1) / 3;
                    (*satellite).__bindgen_anon_4.turn_wait = angle as u8;
                    (*satellite).current.location.x = (*ship)
                        .next
                        .location
                        .x
                        .wrapping_add(cosine(angle, SATELLITE_OFFSET));
                    (*satellite).current.location.y = (*ship)
                        .next
                        .location
                        .y
                        .wrapping_add(sine(angle, SATELLITE_OFFSET));
                    (*satellite).current.image.farray =
                        (*(*ss).RaceDescPtr).ship_data.special.as_mut_ptr();
                    (*satellite).current.image.frame = SetAbsFrameIndex(
                        (*(*ss).RaceDescPtr).ship_data.special[0],
                        TFB_Random() as COUNT & 7,
                    );
                    (*satellite).preprocess_func = Some(satellite_preprocess);
                    (*satellite).postprocess_func = Some(satellite_postprocess);
                    (*satellite).death_func = Some(satellite_death);
                    (*satellite).collision_func = Some(satellite_collision);
                    (*satellite).pParent = ss.cast();
                    (*GlobData
                        .Game_state
                        .DisplayArray
                        .add((*satellite).PrimIndex as usize))
                    .Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                    put_element(h);
                }
            }
        }
    }

    unsafe extern "C" fn chmmr_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let h = AllocElement();
        if !h.is_null() {
            let satellite = element(h);
            (*satellite).playerNr = (*e).playerNr;
            (*satellite).state_flags = (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
            (*satellite).__bindgen_anon_1.life_span = (HYPERJUMP_LIFE + 1) as COUNT;
            (*satellite).death_func = Some(spawn_satellites);
            (*satellite).pParent = ss.cast();
            (*GlobData
                .Game_state
                .DisplayArray
                .add((*satellite).PrimIndex as usize))
            .Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
            put_element(h);
        }
        (*(*ss).RaceDescPtr).preprocess_func = None;
    }
}
