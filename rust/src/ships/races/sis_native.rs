//! Live flagship callbacks translated from ships/sis_ship/sis_ship.c.
#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, element, normalize_facing, object_cloaked, put_element, starship,
    };
    use crate::state::game_state_keys::{get_game_state, set_game_state};
    use std::{mem, ptr};

    #[derive(Clone, Copy)]
    struct SisData {
        num_trackers: COUNT,
        num_blasters: COUNT,
        missiles: [MISSILE_BLOCK; 6],
    }

    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.ship_cost = 16;
        d.ship_info.crew_level = MAX_CREW_SIZE as COUNT;
        d.ship_info.max_crew = MAX_CREW_SIZE as COUNT;
        d.ship_info.energy_level = MAX_ENERGY_SIZE as u8;
        d.ship_info.max_energy = MAX_ENERGY_SIZE as u8;
        d.ship_info.icons_rsc = c"ship.flagship.icons".as_ptr();
        d.characteristics.max_thrust = 10;
        d.characteristics.thrust_increment = 4;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 1;
        d.characteristics.special_energy_cost = 0;
        d.characteristics.energy_wait = 10;
        d.characteristics.turn_wait = 17;
        d.characteristics.thrust_wait = 6;
        d.characteristics.weapon_wait = 6;
        d.characteristics.special_wait = 9;
        d.characteristics.ship_mass = MAX_SHIP_MASS as u8;
        d.ship_data.ship_rsc = [
            c"ship.flagship.graphics.flagship.large".as_ptr(),
            c"ship.flagship.graphics.flagship.medium".as_ptr(),
            c"ship.flagship.graphics.flagship.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.flagship.graphics.blaster.large".as_ptr(),
            c"ship.flagship.graphics.blaster.medium".as_ptr(),
            c"ship.flagship.graphics.blaster.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.flagship.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.flagship.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.flagship.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (24 << ONE_SHIFT) * 12;
        d
    }
    static mut DESC: RACE_DESC = descriptor();

    unsafe fn data(d: *mut RACE_DESC) -> *mut SisData {
        (*d).data.cast()
    }

    unsafe fn clear_data(d: *mut RACE_DESC) {
        if !(*d).data.is_null() {
            drop(Box::from_raw(data(d)));
            (*d).data = ptr::null_mut();
        }
    }

    unsafe extern "C" fn sis_hyper_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*e).state_flags & APPEARING as u16 != 0 {
            (*e).velocity = GlobData.Game_state.velocity;
        }
        (*ss).weapon_counter = (*ss).weapon_counter.wrapping_add(1);
        let (mut udx, mut udy): (SIZE, SIZE) = (0, 0);
        let (mut dx, mut dy): (SIZE, SIZE) = (0, 0);
        let mut accelerate = 0;
        let mut leave = GlobData.Game_state.autopilot.x == -1
            || GlobData.Game_state.autopilot.y == -1
            || (*ss).cur_status_flags & (LEFT | RIGHT | THRUST) as u16 != 0;
        if !leave {
            // units.h uses the original, resolution-dependent hyperspace factors.
            let log_units_x = (((ScreenWidth - 64) << (ONE_SHIFT + 3)) >> 4) * 195;
            let log_units_y = ((ScreenHeight << (ONE_SHIFT + 3)) >> 4) * 25;
            let ux = (GlobData
                .SIS_state
                .log_x
                .wrapping_mul(6250)
                .wrapping_add(log_units_x / 2)
                / log_units_x) as i16;
            let uy = (9999
                - GlobData
                    .SIS_state
                    .log_y
                    .wrapping_mul(625)
                    .wrapping_add(log_units_y / 2)
                    / log_units_y) as i16;
            udx = GlobData.Game_state.autopilot.x.wrapping_sub(ux);
            udy = GlobData
                .Game_state
                .autopilot
                .y
                .wrapping_sub(uy)
                .wrapping_neg();
            dx = udx.wrapping_abs();
            dy = udy.wrapping_abs();
            leave = dx <= 1 && dy <= 1;
            if !leave {
                let mut facing = normalize_facing(angle_to_facing(ARCTAN(udx, udy) as i32));
                if (*ss).cur_status_flags & SHIP_AT_MAX_SPEED as u16 != 0 {
                    if normalize_facing(
                        i32::from((*ss).ShipFacing) + angle_to_facing(QUADRANT as i32)
                            - i32::from(facing),
                    ) > angle_to_facing(HALF_CIRCLE as i32) as u16
                    {
                        leave = true;
                    }
                    facing = (*ss).ShipFacing;
                } else if facing != (*ss).ShipFacing && (*e).__bindgen_anon_4.turn_wait == 0 {
                    if normalize_facing(i32::from((*ss).ShipFacing) - i32::from(facing))
                        >= angle_to_facing(HALF_CIRCLE as i32) as u16
                    {
                        facing = normalize_facing(i32::from(facing) - 1);
                        (*ss).cur_status_flags |= RIGHT as u16;
                    } else if (*ss).ShipFacing != facing {
                        facing = normalize_facing(i32::from(facing) + 1);
                        (*ss).cur_status_flags |= LEFT as u16;
                    }
                    if facing == (*ss).ShipFacing {
                        (*e).velocity = mem::zeroed();
                    }
                }
                if !leave {
                    GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut dx, &mut dy);
                    if (GlobData.SIS_state.FuelOnBoard != 0
                        || get_game_state("ARILOU_SPACE_SIDE") > 1)
                        && facing == (*ss).ShipFacing
                    {
                        (*ss).cur_status_flags |= SHIP_AT_MAX_SPEED as u16;
                        accelerate = 1;
                    } else {
                        accelerate = -1;
                        udx = dx.wrapping_shl(4);
                        udy = dy.wrapping_shl(4);
                    }
                }
            }
        }
        if leave {
            GlobData.Game_state.autopilot = POINT { x: -1, y: -1 };
            if (*ss).cur_status_flags & THRUST as u16 == 0
                || (GlobData.SIS_state.FuelOnBoard == 0 && get_game_state("ARILOU_SPACE_SIDE") <= 1)
            {
                accelerate = -1;
                GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut dx, &mut dy);
                udx = dx.wrapping_shl(4);
                udy = dy.wrapping_shl(4);
                (*ss).cur_status_flags &= !(THRUST as u16);
            }
        }
        if (*e).__bindgen_anon_5.thrust_wait == 0 && accelerate != 0 {
            let c = &(*(*ss).RaceDescPtr).characteristics;
            let increment = (i32::from(c.thrust_increment) << VELOCITY_SHIFT) as i16;
            let dist = square_root(
                (i64::from(udx) * i64::from(udx) + i64::from(udy) * i64::from(udy)) as u32,
            )
            .max(1);
            let mut speed =
                square_root((i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32)
                    as i16;
            if accelerate < 0 {
                dy = ((i32::from(speed) / i32::from(increment) - 1) * i32::from(increment)) as i16;
                if i32::from(dy) < i32::from(speed) - i32::from(increment) {
                    dy = speed.wrapping_sub(increment);
                }
                speed = dy.max(0);
                (*ss).cur_status_flags &= !(SHIP_AT_MAX_SPEED as u16);
            } else {
                let max_velocity = (i32::from(c.max_thrust) << VELOCITY_SHIFT) as i16;
                dy = ((i32::from(speed) / i32::from(increment) + 1) * i32::from(increment)) as i16;
                if i32::from(dy) < i32::from(speed) + i32::from(increment) {
                    dy = speed.wrapping_add(increment);
                }
                speed = dy;
                if speed > max_velocity {
                    speed = max_velocity;
                    (*ss).cur_status_flags |= SHIP_AT_MAX_SPEED as u16;
                }
            }
            dx = (i64::from(udx) * i64::from(speed) / i64::from(dist)) as i16;
            dy = (i64::from(udy) * i64::from(speed) / i64::from(dist)) as i16;
            SetVelocityComponents(&raw mut (*e).velocity, dx, dy);
            (*e).__bindgen_anon_5.thrust_wait = c.thrust_wait;
        }
    }

    unsafe extern "C" fn sis_hyper_postprocess(e: *mut ELEMENT) {
        GlobData.Game_state.velocity = (*e).velocity;
        let ss = starship(e);
        if ((*ss).cur_status_flags & WEAPON as u16 != 0
            || PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0)
            && (*ss).special_counter == 0
        {
            HyperspaceMenu();
            (*ss).cur_status_flags &= !(SHIP_AT_MAX_SPEED as u16);
            (*ss).special_counter = 10;
        }
    }

    unsafe extern "C" fn spawn_point_defense(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*e).state_flags & PLAYER_SHIP as u16 != 0 {
            let h = AllocElement();
            if !h.is_null() {
                let d = element(h);
                (*d).playerNr = (*e).playerNr;
                (*d).state_flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
                (*d).death_func = Some(spawn_point_defense);
                (*d).pParent = ss.cast();
                put_element(h);
            }
        } else {
            let colors = [3, 7, 10, 14, 17, 21, 24, 28];
            let cost = (*(*ss).RaceDescPtr).characteristics.special_energy_cost;
            let g = colors[cost as usize];
            let color = Color {
                r: 255,
                g: (g << 3) | (g >> 2),
                b: 0,
                a: 255,
            };
            let ship = element((*ss).hShip);
            let mut paid = false;
            let mut h = disp_q.tail;
            while !h.is_null() {
                let o = element(h);
                h = (*o).pred;
                if o != ship
                    && (*o).state_flags & (NONSOLID | DISAPPEARING) as u16 == 0
                    && !object_cloaked(o)
                {
                    let dx = (*o)
                        .next
                        .location
                        .x
                        .wrapping_sub((*ship).next.location.x)
                        .wrapping_abs()
                        >> ONE_SHIFT;
                    let dy = (*o)
                        .next
                        .location
                        .y
                        .wrapping_sub((*ship).next.location.y)
                        .wrapping_abs()
                        >> ONE_SHIFT;
                    if dx as u16 <= 100
                        && dy as u16 <= 100
                        && u32::from(dx as u16) * u32::from(dx as u16)
                            + u32::from(dy as u16) * u32::from(dy as u16)
                            <= 10000
                    {
                        if !paid {
                            if DeltaEnergy(ship, -(i16::from(cost) << 2)) == 0 {
                                break;
                            }
                            ProcessSound(
                                SetAbsStringTableIndex(
                                    (*(*ss).RaceDescPtr).ship_data.ship_sounds,
                                    1,
                                ),
                                e,
                            );
                            (*ss).special_counter =
                                (*(*ss).RaceDescPtr).characteristics.special_wait;
                            paid = true;
                        }
                        let mut b = LASER_BLOCK {
                            cx: (*ship).next.location.x,
                            cy: (*ship).next.location.y,
                            face: 0,
                            ex: (*o).next.location.x.wrapping_sub((*ship).next.location.x),
                            ey: (*o).next.location.y.wrapping_sub((*ship).next.location.y),
                            sender: (*ship).playerNr,
                            flags: IGNORE_SIMILAR as u16,
                            pixoffs: 0,
                            color,
                        };
                        let beam = initialize_laser(&mut b);
                        if !beam.is_null() {
                            let p = element(beam);
                            (*p).__bindgen_anon_3.mass_points = cost;
                            (*p).pParent = ss.cast();
                            (*p).hTarget = ptr::null_mut();
                            put_element(beam);
                        }
                    }
                }
            }
        }
    }

    unsafe extern "C" fn sis_battle_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*(*ss).RaceDescPtr).characteristics.special_energy_cost == 0 {
            (*ss).cur_status_flags &= !(SPECIAL as u16);
            (*ss).special_counter = 2;
        }
        if (*(*ss).RaceDescPtr).ship_info.ship_flags
            & (FIRES_FORE | FIRES_RIGHT | FIRES_AFT | FIRES_LEFT) as u16
            == 0
        {
            (*ss).cur_status_flags &= !(WEAPON as u16);
            (*ss).weapon_counter = 2;
        }
    }
    unsafe extern "C" fn sis_battle_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && (*(*ss).RaceDescPtr).characteristics.special_energy_cost != 0
        {
            spawn_point_defense(e);
        }
    }
    unsafe extern "C" fn blaster_collision(
        e: *mut ELEMENT,
        p: *mut POINT,
        o: *mut ELEMENT,
        q: *mut POINT,
    ) {
        let h = weapon_collision(e, p, o, q);
        if !h.is_null() {
            let b = element(h);
            match (*e).__bindgen_anon_3.mass_points {
                2 => {
                    (*b).__bindgen_anon_1.life_span = 2;
                    (*b).current.image.frame = SetAbsFrameIndex((*e).current.image.frame, 0);
                    (*b).preprocess_func = None;
                }
                4 => {
                    (*b).__bindgen_anon_1.life_span = 6;
                    (*b).current.image.frame = IncFrameIndex((*e).current.image.frame);
                }
                6 => {
                    (*b).__bindgen_anon_1.life_span = 7;
                    (*b).current.image.frame = SetAbsFrameIndex((*e).current.image.frame, 20);
                }
                _ => {}
            }
        }
    }
    unsafe extern "C" fn blaster_preprocess(e: *mut ELEMENT) {
        let frame = crate::ships::native::frame_index((*e).current.image.frame);
        match (*e).__bindgen_anon_3.mass_points {
            2 if frame < 8 => {
                (*e).next.image.frame = IncFrameIndex((*e).current.image.frame);
                (*e).state_flags |= CHANGING as u16;
            }
            6 => {
                (*e).next.image.frame = if frame < 19 {
                    IncFrameIndex((*e).current.image.frame)
                } else {
                    SetAbsFrameIndex((*e).current.image.frame, 16)
                };
                (*e).state_flags |= CHANGING as u16;
            }
            _ => {}
        }
        if (*e).__bindgen_anon_4.turn_wait & 15 != 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            let wait = (*e).__bindgen_anon_4.turn_wait >> 4;
            if wait != 0 {
                let mut face = normalize_facing(angle_to_facing((*e).velocity.TravelAngle as i32));
                if TrackShip(e, &mut face) > 0 {
                    SetVelocityVector(&raw mut (*e).velocity, 24 << ONE_SHIFT, face);
                }
                (*e).__bindgen_anon_4.turn_wait = (wait << 4) | wait;
            }
        }
    }
    unsafe extern "C" fn initialize_blasters(e: *mut ELEMENT, out: *mut HELEMENT) -> COUNT {
        let ss = starship(e);
        let data = data((*ss).RaceDescPtr);
        let nt = (4 - (*data).num_trackers) as u8 & 3;
        for i in 0..(*data).num_blasters as usize {
            let mut b = (*data).missiles[i];
            b.cx = (*e).next.location.x;
            b.cy = (*e).next.location.y;
            b.farray = (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast();
            b.sender = (*e).playerNr;
            b.face = normalize_facing(i32::from((*ss).ShipFacing) + i32::from(b.face));
            let h = initialize_missile(&mut b);
            *out.add(i) = h;
            if !h.is_null() {
                let m = element(h);
                (*m).collision_func = Some(blaster_collision);
                (*m).__bindgen_anon_4.turn_wait = (nt << 4) | nt;
            }
        }
        (*data).num_blasters
    }
    unsafe extern "C" fn sis_intelligence(
        e: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(e);
        let d = (*ss).RaceDescPtr;
        let data = data(d);
        let w = concerns.add(ENEMY_WEAPON_INDEX as usize);
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if !(*w).ObjectPtr.is_null() {
            if (*d).characteristics.special_energy_cost != 0 {
                if (*ss).special_counter == 0
                    && ((*w).which_turn <= 2
                        || (!(*enemy).ObjectPtr.is_null() && (*enemy).which_turn <= 4))
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                } else {
                    (*ss).ship_input_state &= !(SPECIAL as u8);
                }
                (*w).ObjectPtr = ptr::null_mut();
            } else if (*d).cyborg_control.ManeuverabilityIndex < MEDIUM_SHIP as u16
                && (*w).MoveState == MOVEMENT_STATE_ENTICE
                && ((*(*w).ObjectPtr).state_flags & CREW_OBJECT as u16 == 0 || (*w).which_turn <= 8)
                && ((*(*w).ObjectPtr).state_flags & FINITE_LIFE as u16 == 0
                    || ((*(*w).ObjectPtr).__bindgen_anon_3.mass_points >= 4
                        && (*w).which_turn == 2
                        && (*enemy).which_turn > 16))
            {
                (*w).MoveState = MOVEMENT_STATE_PURSUE;
            }
        }
        ship_intelligence(e, concerns, count);
        if (*data).num_trackers != 0
            && (*ss).weapon_counter == 0
            && (*ss).ship_input_state & WEAPON as u8 == 0
            && !(*enemy).ObjectPtr.is_null()
            && (*enemy).which_turn <= 16
        {
            let dx = (*(*enemy).ObjectPtr)
                .current
                .location
                .x
                .wrapping_sub((*e).current.location.x);
            let dy = (*(*enemy).ObjectPtr)
                .current
                .location
                .y
                .wrapping_sub((*e).current.location.y);
            let direction = normalize_facing(angle_to_facing(ARCTAN(dx, dy) as i32));
            let mut flags = FIRES_FORE as u16;
            let mut facing = (*ss).ShipFacing;
            while flags <= FIRES_LEFT as u16 {
                if (*d).ship_info.ship_flags & flags != 0
                    && normalize_facing(
                        i32::from(direction) - i32::from(facing) + angle_to_facing(OCTANT as i32),
                    ) <= angle_to_facing(QUADRANT as i32) as u16
                {
                    (*ss).ship_input_state |= WEAPON as u8;
                    break;
                }
                flags <<= 1;
                facing = facing.wrapping_add(QUADRANT as u16);
            }
        }
    }

    unsafe fn init_weapon_slots(d: *mut RACE_DESC, slots: &[u8; 16]) {
        let data = data(d);
        (*data).num_blasters = 0;
        for i in [0, 1, 2, 15] {
            let gun = slots[15 - i];
            if !(GUN_WEAPON as u8..=CANNON_WEAPON as u8).contains(&gun) {
                continue;
            }
            let gun = gun - (GUN_WEAPON as u8 - 1);
            (*d).characteristics.weapon_energy_cost = (*d)
                .characteristics
                .weapon_energy_cost
                .wrapping_add(gun * 2);
            let mut b: MISSILE_BLOCK = mem::zeroed();
            b.flags = IGNORE_SIMILAR as u16;
            b.blast_offs = 8;
            b.speed = 24 << ONE_SHIFT;
            b.preprocess_func = Some(blaster_preprocess);
            b.hit_points = i16::from(2 * gun);
            b.damage = i16::from(2 * gun);
            b.life = 12 + 3 * (u16::from(gun) - 1);
            b.index = match gun {
                1 => 0,
                2 => 9,
                _ => 16,
            };
            match i {
                0 => {
                    (*d).ship_info.ship_flags |= FIRES_FORE as u16;
                    b.pixoffs = 28;
                    b.face = 0;
                }
                1 => {
                    (*d).ship_info.ship_flags |= FIRES_FORE as u16;
                    b.pixoffs = 28;
                    b.face = 1;
                }
                2 => {
                    (*d).ship_info.ship_flags |= (FIRES_LEFT | FIRES_RIGHT) as u16;
                    b.pixoffs = 20;
                    b.face = angle_to_facing(QUADRANT as i32) as u16;
                }
                _ => {
                    (*d).ship_info.ship_flags |= FIRES_AFT as u16;
                    b.pixoffs = 28;
                    b.face = angle_to_facing(HALF_CIRCLE as i32) as u16;
                }
            }
            (*data).missiles[(*data).num_blasters as usize] = b;
            (*data).num_blasters += 1;
            if i == 1 || i == 2 {
                b.face = normalize_facing(-i32::from(b.face));
                (*data).missiles[(*data).num_blasters as usize] = b;
                (*data).num_blasters += 1;
            }
        }
    }
    unsafe fn init_module_slots(d: *mut RACE_DESC, slots: &[u8; 16]) {
        (*d).ship_info.max_crew = 0;
        let mut trackers: u16 = 0;
        for &m in slots.iter().rev() {
            match u32::from(m) {
                CREW_POD => (*d).ship_info.max_crew += CREW_POD_CAPACITY as COUNT,
                TRACKING_SYSTEM => trackers += 1,
                ANTIMISSILE_DEFENSE => (*d).characteristics.special_energy_cost += 1,
                SHIVA_FURNACE => (*d).characteristics.energy_regeneration += 1,
                DYNAMO_UNIT => {
                    (*d).characteristics.energy_wait = ((*d).characteristics.energy_wait - 2).max(4)
                }
                _ => {}
            }
        }
        trackers = trackers.min(3);
        (*d).characteristics.weapon_energy_cost += (trackers * 3) as u8;
        (*data(d)).num_trackers = trackers;
        if (*d).characteristics.special_energy_cost != 0 {
            (*d).ship_info.ship_flags |= POINT_DEFENSE as u16;
            (*d).characteristics.special_energy_cost =
                (*d).characteristics.special_energy_cost.min(8);
        }
    }
    #[no_mangle]
    pub unsafe extern "C" fn init_sis() -> *mut RACE_DESC {
        let d = &raw mut DESC;
        *d = descriptor();
        (*d).uninit_func = Some(uninit_sis);
        if inHQSpace() != 0 {
            (*d).ship_data.ship_rsc = [ptr::null(); 3];
            (*d).ship_data.weapon_rsc = [ptr::null(); 3];
            (*d).ship_data.special_rsc = [ptr::null(); 3];
            (*d).ship_info.icons_rsc = ptr::null();
            (*d).ship_data.captain_control.captain_rsc = ptr::null();
            (*d).ship_data.victory_ditty_rsc = ptr::null();
            (*d).ship_data.ship_sounds_rsc = ptr::null();
            (*d).ship_data.ship_rsc[0] = c"ship.flagship.graphics.hyperspace".as_ptr();
            (*d).preprocess_func = Some(sis_hyper_preprocess);
            (*d).postprocess_func = Some(sis_hyper_postprocess);
            (*d).characteristics.max_thrust -= 4;
        } else {
            (*d).preprocess_func = Some(sis_battle_preprocess);
            (*d).postprocess_func = Some(sis_battle_postprocess);
            (*d).init_weapon_func = Some(initialize_blasters);
            (*d).cyborg_control.intelligence_func = Some(sis_intelligence);
            if get_game_state("CHMMR_BOMB_STATE") == 3 {
                set_game_state("BOMB_CARRIER", 1);
            }
        }
        (*d).data = Box::into_raw(Box::new(mem::zeroed::<SisData>())).cast();
        let slots = GlobData.SIS_state.ModuleSlots;
        init_module_slots(d, &slots);
        init_weapon_slots(d, &slots);
        (*d).characteristics.thrust_wait = 0;
        for drive in GlobData.SIS_state.DriveSlots {
            if u32::from(drive) == FUSION_THRUSTER {
                (*d).characteristics.max_thrust += 2;
                (*d).characteristics.thrust_wait += 1;
            }
        }
        (*d).characteristics.thrust_wait = 6 - ((*d).characteristics.thrust_wait >> 1);
        (*d).characteristics.max_thrust =
            ((*d).characteristics.max_thrust / (*d).characteristics.thrust_increment + 1)
                * (*d).characteristics.thrust_increment;
        for jet in GlobData.SIS_state.JetSlots {
            if u32::from(jet) == TURNING_JETS {
                (*d).characteristics.turn_wait -= 2;
            }
        }
        if GlobData.Game_state.CurrentActivity & 255 == SUPER_MELEE as u16 {
            (*d).ship_info.crew_level = (*d).ship_info.max_crew;
        } else {
            (*d).ship_info.max_crew += 1;
            (*d).ship_info.crew_level = GlobData.SIS_state.CrewEnlisted + 1;
            (*d).ship_info.ship_flags |= PLAYER_CAPTAIN as u16;
        }
        (*d).ship_info.energy_level = (*d).ship_info.max_energy;
        d
    }
    unsafe extern "C" fn uninit_sis(d: *mut RACE_DESC) {
        if inHQSpace() == 0 {
            GlobData.SIS_state.CrewEnlisted = (*d).ship_info.crew_level;
            if (*d).ship_info.ship_flags & PLAYER_CAPTAIN as u16 != 0 {
                GlobData.SIS_state.CrewEnlisted = GlobData.SIS_state.CrewEnlisted.wrapping_sub(1);
            }
        }
        clear_data(d);
    }
}
