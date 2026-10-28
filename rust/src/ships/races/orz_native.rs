//! Orz callbacks and descriptor translated from `sc2/src/uqm/ships/orz/orz.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_count, normalize_angle,
        normalize_facing, object_cloaked, put_element, sine, starship,
    };
    use std::ptr;

    use crate::collections::queue::InsertQueue;

    use crate::battle::weapon_native::engine::ModifySilhouette;

    const MODIFY_SWAP: BYTE = 1 << 1;
    // The cast in element.h prevents bindgen from emitting this macro.
    const GRAVITY_THRESHOLD: COUNT = 255;
    const MAX_CREW: u16 = 16;
    const MAX_ENERGY: u8 = 20;
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 12;
    const TURRET_OFFSET: i32 = 14;
    const TURRET_WAIT: u8 = 3;
    const MAX_MARINES: u8 = 8;
    const MARINE_WAIT: u8 = 12;
    const MARINE_MAX_THRUST: u16 = 32;
    const MARINE_THRUST_INCREMENT: u16 = 8;
    const ION_LIFE: u8 = 1;

    const fn color(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    const ION_COLORS: [Color; 5] = [
        color(0x1f, 0x15, 0),
        color(0x1f, 0x0e, 0),
        color(0x1f, 0x03, 0),
        color(0x1b, 0, 0),
        color(0x13, 0, 0),
    ];

    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE | SEEKING_SPECIAL) as u16;
        d.ship_info.ship_cost = 23;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.orz.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.orz.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.orz.meleeicons".as_ptr();
        d.fleet.strength = (333 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        d.fleet.known_loc = POINT { x: 3608, y: 2637 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: 35,
            thrust_increment: 5,
            energy_regeneration: 1,
            weapon_energy_cost: MAX_ENERGY / 3,
            special_energy_cost: 0,
            energy_wait: 6,
            turn_wait: 1,
            thrust_wait: 0,
            weapon_wait: 4,
            special_wait: 12,
            ship_mass: 4,
        };
        d.ship_data.ship_rsc = [
            c"ship.orz.graphics.nemesis.large".as_ptr(),
            c"ship.orz.graphics.nemesis.medium".as_ptr(),
            c"ship.orz.graphics.nemesis.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.orz.graphics.howitzer.large".as_ptr(),
            c"ship.orz.graphics.howitzer.medium".as_ptr(),
            c"ship.orz.graphics.howitzer.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.orz.graphics.turret.large".as_ptr(),
            c"ship.orz.graphics.turret.medium".as_ptr(),
            c"ship.orz.graphics.turret.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.orz.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.orz.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.orz.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (MISSILE_SPEED as i32 * MISSILE_LIFE as i32) as u16;
        d
    }

    static mut ORZ_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_orz() -> *mut RACE_DESC {
        unsafe {
            ORZ_DESC.preprocess_func = Some(orz_preprocess);
            ORZ_DESC.init_weapon_func = Some(initialize_turret_missile);
            ORZ_DESC.cyborg_control.intelligence_func = Some(orz_intelligence);
        }
        &raw mut ORZ_DESC
    }

    unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
        GlobData
            .Game_state
            .DisplayArray
            .add((*e).PrimIndex as usize)
    }

    unsafe fn sound(ss: *mut STARSHIP, index: u16, source: *mut ELEMENT) {
        ProcessSound(
            SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, index),
            source,
        );
    }

    // SIZE assignments happen both before and after the C WRAP_DELTA macros.
    fn wrap_delta(delta: SIZE, extent: i32) -> SIZE {
        let delta = i32::from(delta);
        if delta < 0 {
            if -delta <= extent >> 1 {
                delta as SIZE
            } else {
                (extent + delta) as SIZE
            }
        } else if delta <= extent >> 1 {
            delta as SIZE
        } else {
            (delta - extent) as SIZE
        }
    }

    unsafe fn wrapped_deltas(a: *mut ELEMENT, b: *mut ELEMENT) -> (SIZE, SIZE) {
        let dx = (i32::from((*a).current.location.x) - i32::from((*b).current.location.x)) as SIZE;
        let dy = (i32::from((*a).current.location.y) - i32::from((*b).current.location.y)) as SIZE;
        (
            wrap_delta(dx, ((ScreenWidth - 64) << ONE_SHIFT) << 3),
            wrap_delta(dy, (ScreenHeight << ONE_SHIFT) << 3),
        )
    }

    pub(crate) unsafe extern "C" fn howitzer_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        if (*a).playerNr != (*b).playerNr {
            weapon_collision(a, pa, b, pb);
        }
    }

    pub(crate) unsafe extern "C" fn initialize_turret_missile(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let turret = element((*ship).succ);
        if (*turret).__bindgen_anon_4.turn_wait == 0
            && (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).cur_status_flags & (LEFT | RIGHT) as u16 != 0
        {
            if (*ss).cur_status_flags & RIGHT as u16 != 0 {
                (*turret).__bindgen_anon_5.thrust_wait =
                    (*turret).__bindgen_anon_5.thrust_wait.wrapping_add(1);
            } else {
                (*turret).__bindgen_anon_5.thrust_wait =
                    (*turret).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
            }
            (*turret).__bindgen_anon_4.turn_wait = TURRET_WAIT + 1;
        }
        let facing = normalize_facing(
            i32::from((*ss).ShipFacing) + i32::from((*turret).__bindgen_anon_5.thrust_wait),
        );
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: facing,
            index: facing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: TURRET_OFFSET as SIZE,
            speed: MISSILE_SPEED,
            hit_points: 2,
            damage: 3,
            life: MISSILE_LIFE,
            preprocess_func: None,
            blast_offs: 1,
        };
        *out = initialize_missile(&mut block);
        if !(*out).is_null() {
            (*element(*out)).collision_func = Some(CollisionHook::Legacy(howitzer_collision));
        }
        1
    }

    unsafe fn count_marines(ss: *mut STARSHIP, find_spot: bool) -> BYTE {
        let mut id_use = [0u8; MAX_MARINES as usize];
        let mut count = 0u8;
        let mut h = disp_q.tail;
        while !h.is_null() {
            let e = element(h);
            let mut next = (*e).pred;
            if (*e).current.image.farray == (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast()
                && (*e).__bindgen_anon_1.life_span != 0
                && (*e).state_flags & (FINITE_LIFE | DISAPPEARING) as u16 == 0
            {
                if (*e).state_flags & NONSOLID as u16 != 0 {
                    id_use[(*e).__bindgen_anon_4.turn_wait as usize] = 1;
                }
                count = count.wrapping_add(1);
                if count == MAX_MARINES {
                    next = ptr::null_mut();
                }
            }
            h = next;
        }
        if find_spot {
            count = 0;
            while id_use[count as usize] != 0 {
                count = count.wrapping_add(1);
            }
        }
        count
    }

    pub(crate) unsafe extern "C" fn orz_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let turret = element((*ship).succ);
        (*turret).__bindgen_anon_4.turn_wait = (*turret).__bindgen_anon_4.turn_wait.wrapping_add(1);
        ship_intelligence(ship, concerns, count);
        (*turret).__bindgen_anon_4.turn_wait = (*turret).__bindgen_anon_4.turn_wait.wrapping_sub(1);
        let ss = starship(ship);
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if (*enemy).ObjectPtr.is_null() {
            (*ss).ship_input_state &= !(SPECIAL as u8);
        } else if (*ss).special_counter != 1 {
            let target = (*enemy).ObjectPtr;
            if (*ship).__bindgen_anon_4.turn_wait == 0
                && (*enemy).MoveState == MOVEMENT_STATE_ENTICE
                && (*enemy).which_turn < 24
                && (*ss).cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16 != 0
                && (*ss).ship_input_state & THRUST as u8 == 0
                && normalize_angle(
                    i32::from((*ship).velocity.travel_angle)
                        - i32::from(ARCTAN(
                            (i32::from((*target).next.location.x)
                                - i32::from((*ship).next.location.x))
                                as SIZE,
                            (i32::from((*target).next.location.y)
                                - i32::from((*ship).next.location.y))
                                as SIZE,
                        ))
                        + (QUADRANT as i32 - (OCTANT as i32 >> 1)),
                ) >= ((QUADRANT as i32 - (OCTANT as i32 >> 1)) << 1)
            {
                (*ss).ship_input_state &= !((LEFT | RIGHT) as u8);
            }
            (*ss).ship_input_state &= !(SPECIAL as u8);
            if (*ship).__bindgen_anon_4.turn_wait == 0
                && (*ss).ship_input_state & (LEFT | RIGHT | WEAPON) as u8 == 0
                && (*turret).__bindgen_anon_4.turn_wait == 0
            {
                let mut facing = normalize_facing(
                    i32::from((*ss).ShipFacing) + i32::from((*turret).__bindgen_anon_5.thrust_wait),
                );
                let mut delta = TrackShip(turret, &mut facing);
                if delta > 0 {
                    (*ss).ship_input_state |= SPECIAL as u8;
                    if i32::from(delta) == angle_to_facing(HALF_CIRCLE as i32) {
                        delta = (i32::from(delta) + (((TFB_Random() as u8 & 1) as i32) << 1) - 1)
                            as SIZE;
                    }
                    if i32::from(delta) < angle_to_facing(HALF_CIRCLE as i32) {
                        (*ss).ship_input_state |= RIGHT as u8;
                    } else {
                        (*ss).ship_input_state |= LEFT as u8;
                    }
                }
            }
            let enemy_ss = starship(target);
            if (*ss).special_counter == 0
                && (*ss).ship_input_state & WEAPON as u8 == 0
                && (*(*ss).RaceDescPtr).ship_info.crew_level
                    > ((*(*ss).RaceDescPtr).ship_info.max_crew >> 2) as BYTE as u16
                && (*(*enemy_ss).RaceDescPtr).ship_info.ship_flags & POINT_DEFENSE as u16 == 0
                && (u32::from(
                    (*(*enemy_ss).RaceDescPtr)
                        .cyborg_control
                        .ManeuverabilityIndex,
                ) < SLOW_SHIP
                    || (*enemy).which_turn <= 12
                    || count_marines(ss, false) < 2)
            {
                (*ss).ship_input_state |= (WEAPON | SPECIAL) as u8;
            }
        }
    }

    pub(crate) unsafe extern "C" fn ion_preprocess(e: *mut ELEMENT) {
        (*e).colorCycleIndex = (*e).colorCycleIndex.wrapping_add(1);
        if usize::from((*e).colorCycleIndex) != ION_COLORS.len() {
            (*e).__bindgen_anon_1.life_span = u16::from((*e).__bindgen_anon_5.thrust_wait);
            (*prim(e)).color = ION_COLORS[usize::from((*e).colorCycleIndex)];
            (*e).state_flags &= !(DISAPPEARING as u16);
            (*e).state_flags |= CHANGING as u16;
        }
    }

    unsafe fn silhouette(ship: *mut ELEMENT, marine: *mut ELEMENT, frame: FRAME, flags: BYTE) {
        let mut stamp = STAMP {
            origin: POINT {
                x: (16 + i32::from((*marine).__bindgen_anon_4.turn_wait & 3) * 9) as COORD,
                y: (14 + i32::from((*marine).__bindgen_anon_4.turn_wait >> 2) * 11) as COORD,
            },
            frame,
        };
        ModifySilhouette(ship, &mut stamp, flags);
    }

    #[no_mangle]
    pub(crate) unsafe extern "C" fn intruder_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let owner = element((*ss).hShip);
        if (*owner).__bindgen_anon_2.crew_level == 0
            && (*owner).__bindgen_anon_1.life_span == 1
            && (*owner).state_flags & (FINITE_LIFE | NONSOLID) as u16
                == (FINITE_LIFE | NONSOLID) as u16
        {
            (*e).__bindgen_anon_1.life_span = 0;
            (*e).state_flags |= DISAPPEARING as u16;
        }
        if (*e).__bindgen_anon_5.thrust_wait != 0 {
            (*e).__bindgen_anon_5.thrust_wait -= 1;
        }
        let mut h = disp_q.head;
        while !h.is_null() {
            let ship = element(h);
            if (*ship).state_flags & PLAYER_SHIP as u16 != 0 && (*ship).playerNr != (*e).playerNr {
                if (*e).__bindgen_anon_5.thrust_wait == MARINE_WAIT {
                    (*e).__bindgen_anon_5.thrust_wait -= 1;
                    let frame = *(*e).next.image.farray;
                    silhouette(
                        ship,
                        e,
                        SetAbsFrameIndex(frame, frame_count(frame).wrapping_sub(2)),
                        0,
                    );
                }
                (*e).next.location = (*ship).next.location;
                let mut left_ship = (*ship).__bindgen_anon_2.crew_level == 0
                    || (*e).__bindgen_anon_1.life_span == 0;
                if left_ship {
                    h = ptr::null_mut();
                } else if (*e).__bindgen_anon_5.thrust_wait == 0 {
                    (*e).__bindgen_anon_5.thrust_wait = MARINE_WAIT;
                    let randval = TFB_Random() as BYTE;
                    if u16::from(randval) < 0x100 / 16 {
                        (*e).__bindgen_anon_1.life_span = 0;
                        (*e).state_flags |= DISAPPEARING as u16;
                        sound(ss, 4, e);
                        left_ship = true;
                    } else if u16::from(randval) < 0x100 / 2 + 0x100 / 16 {
                        if DeltaCrew(ship, -1) == 0 {
                            (*ship).__bindgen_anon_1.life_span = 0;
                        }
                        (*e).__bindgen_anon_5.thrust_wait =
                            (*e).__bindgen_anon_5.thrust_wait.wrapping_add(1);
                        let frame = *(*e).next.image.farray;
                        silhouette(
                            ship,
                            e,
                            SetAbsFrameIndex(frame, frame_count(frame).wrapping_sub(1)),
                            0,
                        );
                        sound(ss, 3, e);
                    }
                }
                if left_ship {
                    silhouette(ship, e, (*e).next.image.frame, MODIFY_SWAP as BYTE);
                }
                break;
            }
            h = (*ship).succ;
        }
        if h.is_null() && (*e).__bindgen_anon_1.life_span != 0 {
            (*e).state_flags &= !(NONSOLID as u16);
            (*e).state_flags |= (CHANGING | CREW_OBJECT) as u16;
            (*prim(e)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            let frame = SetAbsFrameIndex((*(*ss).RaceDescPtr).ship_data.special[0], 21);
            image!(frame; (*e).current.image.frame, frame);
            image!(frame; (*e).next.image.frame, frame);
            (*e).__bindgen_anon_5.thrust_wait = 0;
            (*e).__bindgen_anon_4.turn_wait =
                (normalize_facing(i32::from(TFB_Random() as BYTE)) << 4) as BYTE;
            (*e).preprocess_func = Some(ElementHook::Legacy(marine_preprocess));
        }
    }

    unsafe fn spawn_marine_ion_trail(e: *mut ELEMENT, ss: *mut STARSHIP, facing: COUNT) {
        let h = AllocElement();
        if !h.is_null() {
            let angle = (facing_to_angle(i32::from(facing)) + HALF_CIRCLE as i32) as COUNT;
            InsertQueue((&raw mut disp_q).cast(), h, disp_q.head);
            let ion = element(h);
            (*ion).playerNr = NEUTRAL_PLAYER_NUM as SIZE;
            (*ion).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            (*ion).__bindgen_anon_5.thrust_wait = ION_LIFE;
            (*ion).__bindgen_anon_1.life_span = u16::from(ION_LIFE);
            (*prim(ion)).Type = gfx_object_POINT_PRIM as GRAPHICS_PRIM;
            (*prim(ion)).color = ION_COLORS[0];
            (*ion).colorCycleIndex = 0;
            (*ion).current.location = (*e).current.location;
            (*ion).current.location.x = (i32::from((*ion).current.location.x)
                + i32::from(cosine(i32::from(angle), 2 << ONE_SHIFT)))
                as COORD;
            (*ion).current.location.y = (i32::from((*ion).current.location.y)
                + i32::from(sine(i32::from(angle), 2 << ONE_SHIFT)))
                as COORD;
            (*ion).death_func = Some(ElementHook::Legacy(ion_preprocess));
            (*ion).pParent = ss.cast();
            image!(copy; (*ion).next, (*ion).current);
            (*ion).__bindgen_anon_1.life_span -= 1;
            (*ion).state_flags |= PRE_PROCESS as u16;
        }
    }

    pub(crate) unsafe extern "C" fn marine_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let ship = element((*ss).hShip);
        if (*ship).__bindgen_anon_2.crew_level == 0
            && (*ship).__bindgen_anon_1.life_span == 1
            && (*ship).state_flags & (FINITE_LIFE | NONSOLID) as u16
                == (FINITE_LIFE | NONSOLID) as u16
        {
            (*e).__bindgen_anon_1.life_span = 0;
            (*e).state_flags |= (DISAPPEARING | NONSOLID) as u16;
            (*e).__bindgen_anon_4.turn_wait = 1;
        }
        if (*e).__bindgen_anon_4.turn_wait & 0xf != 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
            return;
        }
        (*e).__bindgen_anon_5.thrust_wait &= !((SHIP_IN_GRAVITY_WELL >> 6) as BYTE);
        let mut pfacing: COUNT = 0;
        let mut target: HELEMENT = ptr::null_mut();
        let mut h = disp_q.head;
        while !h.is_null() {
            let object = element(h);
            let mut next = (*object).succ;
            if u32::from((*object).__bindgen_anon_3.mass_points) > MAX_SHIP_MASS * 10 {
                let (dx, dy) = wrapped_deltas(object, e);
                if i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
                    <= i64::from((GRAVITY_THRESHOLD as i32) << ONE_SHIFT).pow(2)
                {
                    pfacing = angle_to_facing(i32::from(ARCTAN(dx, dy))) as COUNT;
                    let delta = normalize_facing(
                        i32::from(pfacing) - angle_to_facing(i32::from((*e).velocity.travel_angle))
                            + angle_to_facing(OCTANT as i32),
                    ) as SIZE;
                    if i32::from(delta) <= angle_to_facing(QUADRANT as i32) {
                        target = h;
                        next = ptr::null_mut();
                    }
                    (*e).__bindgen_anon_5.thrust_wait |= (SHIP_IN_GRAVITY_WELL >> 6) as BYTE;
                }
            } else if (*object).state_flags & PLAYER_SHIP as u16 != 0
                && (*object).__bindgen_anon_2.crew_level != 0
                && !object_cloaked(object)
            {
                if (*object).playerNr != (*e).playerNr {
                    if (*e).state_flags & IGNORE_SIMILAR as u16 != 0 {
                        target = h;
                    }
                } else if target.is_null() {
                    target = h;
                }
            }
            h = next;
        }
        let mut facing = COUNT::from((*e).__bindgen_anon_4.turn_wait >> 4);
        let mut delta: SIZE;
        if target.is_null() {
            delta = -1;
        } else {
            let object = element(target);
            let (mut dx, mut dy) = wrapped_deltas(object, e);
            if u32::from((*object).__bindgen_anon_3.mass_points) > MAX_SHIP_MASS * 10 {
                delta = normalize_facing(
                    i32::from(pfacing) - i32::from(facing) + angle_to_facing(OCTANT as i32),
                ) as SIZE;
                if i32::from(delta) > angle_to_facing(QUADRANT as i32) {
                    delta = 0;
                } else if i32::from(delta) == angle_to_facing(OCTANT as i32) {
                    facing = (i32::from(facing) + (((TFB_Random() as SIZE & 1) as i32) << 1) - 1)
                        as COUNT;
                } else if i32::from(delta) < angle_to_facing(OCTANT as i32) {
                    facing = facing.wrapping_add(1);
                } else {
                    facing = facing.wrapping_sub(1);
                }
            } else {
                if (*object).playerNr == (*e).playerNr
                    && (*e).state_flags & IGNORE_SIMILAR as u16 != 0
                {
                    image!(frame; (*e).next.image.frame, SetAbsFrameIndex((*(*ss).RaceDescPtr).ship_data.special[0], 21));
                    (*e).state_flags &= !(IGNORE_SIMILAR as u16);
                    (*e).state_flags |= CHANGING as u16;
                }
                let mut frames = (square_root(
                    (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as DWORD,
                ) >> 6) as COUNT;
                if frames == 0 {
                    frames = 1;
                }
                let mut velocity = (*object).velocity;
                GetNextVelocityComponents(&mut velocity, &mut dx, &mut dy, frames);
                dx = (i32::from((*object).current.location.x) + i32::from(dx)
                    - i32::from((*e).current.location.x)) as SIZE;
                dy = (i32::from((*object).current.location.y) + i32::from(dy)
                    - i32::from((*e).current.location.y)) as SIZE;
                delta = normalize_facing(
                    angle_to_facing(i32::from(ARCTAN(dx, dy))) - i32::from(facing),
                ) as SIZE;
                if delta > 0 {
                    if i32::from(delta) == angle_to_facing(HALF_CIRCLE as i32) {
                        facing = (i32::from(facing) + (((TFB_Random() as BYTE & 1) as i32) << 1)
                            - 1) as COUNT;
                    } else if i32::from(delta) < angle_to_facing(HALF_CIRCLE as i32) {
                        facing = facing.wrapping_add(1);
                    } else {
                        facing = facing.wrapping_sub(1);
                    }
                }
            }
        }
        (*e).__bindgen_anon_4.turn_wait = (normalize_facing(i32::from(facing)) << 4) as BYTE;
        if delta == 0
            || ((*e).__bindgen_anon_5.thrust_wait & (SHIP_BEYOND_MAX_SPEED >> 6) as BYTE != 0
                && (*e).__bindgen_anon_5.thrust_wait & (SHIP_IN_GRAVITY_WELL >> 6) as BYTE == 0)
        {
            let old_facing = (*ss).ShipFacing;
            let old_status = (*ss).cur_status_flags;
            let old_increment = (*(*ss).RaceDescPtr).characteristics.thrust_increment;
            let old_thrust = (*(*ss).RaceDescPtr).characteristics.max_thrust;
            (*ss).ShipFacing = facing;
            (*ss).cur_status_flags = u16::from((*e).__bindgen_anon_5.thrust_wait) << 6;
            (*(*ss).RaceDescPtr).characteristics.thrust_increment = MARINE_THRUST_INCREMENT;
            (*(*ss).RaceDescPtr).characteristics.max_thrust = MARINE_MAX_THRUST;
            let thrust_status = inertial_thrust(e);
            (*(*ss).RaceDescPtr).characteristics.max_thrust = old_thrust;
            (*(*ss).RaceDescPtr).characteristics.thrust_increment = old_increment;
            (*ss).cur_status_flags = old_status;
            (*ss).ShipFacing = old_facing;
            if (*e).__bindgen_anon_5.thrust_wait & (SHIP_IN_GRAVITY_WELL >> 6) as BYTE != 0
                || delta != 0
                || thrust_status & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16 == 0
            {
                spawn_marine_ion_trail(e, ss, facing);
            }
            (*e).__bindgen_anon_5.thrust_wait = (thrust_status >> 6) as BYTE;
        }
    }

    #[no_mangle]
    pub(crate) unsafe extern "C" fn marine_collision(
        a: *mut ELEMENT,
        _pa: *mut POINT,
        b: *mut ELEMENT,
        _pb: *mut POINT,
    ) {
        if (*a).__bindgen_anon_1.life_span == 0
            || (*a).state_flags & (NONSOLID | COLLISION) as u16 != 0
            || (*b).state_flags & FINITE_LIFE as u16 != 0
        {
            return;
        }
        if (*a).playerNr != (*b).playerNr {
            (*a).__bindgen_anon_4.turn_wait = 5 | ((*a).__bindgen_anon_4.turn_wait & 0xf0);
            (*a).__bindgen_anon_5.thrust_wait &=
                !(((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) >> 6) as BYTE);
            (*a).state_flags |= COLLISION as u16;
        }
        if u32::from((*b).__bindgen_anon_3.mass_points) > MAX_SHIP_MASS * 10 {
            (*a).state_flags |= (NONSOLID | FINITE_LIFE) as u16;
            (*a).__bindgen_anon_2.hit_points = 0;
            (*a).__bindgen_anon_1.life_span = 0;
        } else if (*b).state_flags & PLAYER_SHIP as u16 != 0
            && ((*b).state_flags & FINITE_LIFE as u16 != 0
                || u32::from((*b).__bindgen_anon_1.life_span) == NORMAL_LIFE)
        {
            (*b).state_flags &= !(COLLISION as u16);
            if (*a).state_flags & COLLISION as u16 == 0 {
                DeltaCrew(b, 1);
                (*a).state_flags |= (DISAPPEARING | NONSOLID | FINITE_LIFE) as u16;
                (*a).__bindgen_anon_2.hit_points = 0;
                (*a).__bindgen_anon_1.life_span = 0;
            } else if (*a).state_flags & IGNORE_SIMILAR as u16 != 0
                && (*b).__bindgen_anon_2.crew_level != 0
            {
                let ss = starship(a);
                if DeltaCrew(b, -1) == 0 {
                    (*b).__bindgen_anon_1.life_span = 0;
                } else {
                    (*a).__bindgen_anon_4.turn_wait = count_marines(ss, true);
                    (*a).__bindgen_anon_5.thrust_wait = MARINE_WAIT;
                    image!(frame; (*a).next.image.frame, SetAbsFrameIndex(
                        *(*a).next.image.farray,
                        22 + u16::from((*a).__bindgen_anon_4.turn_wait),
                    ));
                    (*a).state_flags |= NONSOLID as u16;
                    (*a).state_flags &= !(CREW_OBJECT as u16);
                    (*prim(a)).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                    (*a).preprocess_func = Some(ElementHook::Legacy(intruder_preprocess));
                    silhouette(b, a, (*a).next.image.frame, 0);
                }
                sound(ss, 2, b);
            }
            (*a).state_flags &= !(COLLISION as u16);
        }
    }

    pub(crate) unsafe extern "C" fn turret_postprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_1.life_span != 0 {
            return;
        }
        (*prim(e)).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
        let ss = starship(e);
        if (*ss).hShip.is_null() {
            return;
        }
        let ship = element((*ss).hShip);
        let h = AllocElement();
        // C leaves facing uninitialized on allocation failure. Define that
        // path as zero while retaining TrackShip's call order and RNG effects.
        let mut facing: COUNT = 0;
        if !h.is_null() {
            let turret = element(h);
            (*turret).playerNr = (*e).playerNr;
            (*turret).state_flags =
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | CHANGING | PRE_PROCESS | POST_PROCESS)
                    as u16;
            (*turret).__bindgen_anon_1.life_span = 1;
            image!(copy; (*turret).current.image, (*e).current.image);
            (*turret).current.location = (*ship).next.location;
            (*turret).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_4.turn_wait;
            (*turret).__bindgen_anon_5.thrust_wait = (*e).__bindgen_anon_5.thrust_wait;
            if (*turret).__bindgen_anon_4.turn_wait != 0 {
                (*turret).__bindgen_anon_4.turn_wait -= 1;
            } else if (*ss).cur_status_flags & SPECIAL as u16 != 0
                && (*ss).cur_status_flags & (LEFT | RIGHT) as u16 != 0
            {
                if (*ss).cur_status_flags & RIGHT as u16 != 0 {
                    (*turret).__bindgen_anon_5.thrust_wait =
                        (*turret).__bindgen_anon_5.thrust_wait.wrapping_add(1);
                } else {
                    (*turret).__bindgen_anon_5.thrust_wait =
                        (*turret).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
                }
                (*turret).__bindgen_anon_4.turn_wait = TURRET_WAIT;
            }
            facing = normalize_facing(
                i32::from((*ss).ShipFacing) + i32::from((*turret).__bindgen_anon_5.thrust_wait),
            );
            (*(*ss).RaceDescPtr).ship_info.ship_flags &=
                !((FIRES_FORE | FIRES_RIGHT | FIRES_AFT | FIRES_LEFT) as u16);
            (*(*ss).RaceDescPtr).ship_info.ship_flags |= (FIRES_FORE
                << (i32::from(normalize_facing(
                    i32::from(facing) + angle_to_facing(OCTANT as i32),
                )) / angle_to_facing(QUADRANT as i32)))
                as u16;
            image!(frame; (*turret).current.image.frame, SetAbsFrameIndex((*turret).current.image.frame, facing));
            facing = facing_to_angle(i32::from(facing)) as COUNT;
            if (*ss).cur_status_flags & WEAPON as u16 != 0 {
                let effect = element(disp_q.tail);
                if effect != e
                    && (*effect).playerNr == (*e).playerNr
                    && (*effect).state_flags & APPEARING as u16 != 0
                    && (*prim(effect)).Type == gfx_object_STAMP_PRIM as GRAPHICS_PRIM
                {
                    let effect_h = AllocElement();
                    if !effect_h.is_null() {
                        (*turret).current.location.x = (i32::from((*turret).current.location.x)
                            - i32::from(cosine(i32::from(facing), 2 << ONE_SHIFT)))
                            as COORD;
                        (*turret).current.location.y = (i32::from((*turret).current.location.y)
                            - i32::from(sine(i32::from(facing), 2 << ONE_SHIFT)))
                            as COORD;
                        let effect = element(effect_h);
                        (*effect).playerNr = (*e).playerNr;
                        (*effect).state_flags =
                            (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16;
                        (*effect).__bindgen_anon_1.life_span = 4;
                        (*effect).current.location.x = (i32::from((*turret).current.location.x)
                            + i32::from(cosine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                            as COORD;
                        (*effect).current.location.y = (i32::from((*turret).current.location.y)
                            + i32::from(sine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                            as COORD;
                        image!(set; (*effect).current.image.farray, (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast());
                        image!(frame; (*effect).current.image.frame, SetAbsFrameIndex(
                            (*(*ss).RaceDescPtr).ship_data.special[0],
                            angle_to_facing(FULL_CIRCLE as i32) as COUNT,
                        ));
                        (*effect).preprocess_func = Some(ElementHook::Native(animation_preprocess));
                        (*effect).pParent = ss.cast();
                        (*prim(effect)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                        put_element(effect_h);
                    }
                }
            }
            image!(copy; (*turret).next, (*turret).current);
            (*prim(turret)).Type = (*prim(ship)).Type;
            (*prim(turret)).color = (*prim(ship)).color;
            (*turret).postprocess_func = (*e).postprocess_func;
            (*turret).pParent = ss.cast();
            InsertQueue((&raw mut disp_q).cast(), h, (*e).succ);
        }
        if (*ss).special_counter == 0
            && (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).cur_status_flags & WEAPON as u16 != 0
            && (*ship).__bindgen_anon_2.crew_level > 1
            && count_marines(ss, false) < MAX_MARINES
            && TrackShip(ship, &mut facing) >= 0
        {
            let h = AllocElement();
            if !h.is_null() {
                let marine = element(h);
                (*marine).playerNr = (*e).playerNr;
                (*marine).state_flags = (IGNORE_SIMILAR | APPEARING | CREW_OBJECT) as u16;
                (*marine).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
                (*marine).__bindgen_anon_2.hit_points = 3;
                (*marine).__bindgen_anon_3.mass_points = 1;
                facing = facing_to_angle(i32::from((*ss).ShipFacing)) as COUNT;
                (*marine).current.location.x = (i32::from((*ship).current.location.x)
                    - i32::from(cosine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                    as COORD;
                (*marine).current.location.y = (i32::from((*ship).current.location.y)
                    - i32::from(sine(i32::from(facing), TURRET_OFFSET << ONE_SHIFT)))
                    as COORD;
                image!(set; (*marine).current.image.farray, (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast());
                image!(frame; (*marine).current.image.frame, SetAbsFrameIndex((*(*ss).RaceDescPtr).ship_data.special[0], 20));
                (*marine).__bindgen_anon_4.turn_wait = (normalize_facing(angle_to_facing(
                    i32::from(facing) + HALF_CIRCLE as i32,
                )) << 4) as BYTE;
                (*marine).preprocess_func = Some(ElementHook::Legacy(marine_preprocess));
                (*marine).collision_func = Some(CollisionHook::Legacy(marine_collision));
                (*marine).pParent = ss.cast();
                (*prim(marine)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                put_element(h);
                DeltaCrew(ship, -1);
                sound(ss, 1, marine);
                (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
            }
        }
    }

    pub(crate) unsafe extern "C" fn orz_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*e).state_flags & APPEARING as u16 == 0 {
            if ((*ss).cur_status_flags | (*ss).old_status_flags) & SPECIAL as u16 != 0
                && (*ss).cur_status_flags & (LEFT | RIGHT) as u16 != 0
                && (*e).__bindgen_anon_4.turn_wait == 0
            {
                (*e).__bindgen_anon_4.turn_wait += 1;
            }
            if (*ss).cur_status_flags & SPECIAL as u16 != 0
                && (*ss).cur_status_flags & WEAPON as u16 != 0
                && (*ss).weapon_counter == 0
            {
                (*ss).weapon_counter += 1;
            }
        } else {
            let h = AllocElement();
            if !h.is_null() {
                let turret = element(h);
                (*turret).playerNr = (*e).playerNr;
                (*turret).state_flags = (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16;
                (*turret).__bindgen_anon_1.life_span = 1;
                image!(set; (*turret).current.image.farray, (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast());
                image!(frame; (*turret).current.image.frame, SetAbsFrameIndex((*(*ss).RaceDescPtr).ship_data.special[0], (*ss).ShipFacing));
                (*turret).postprocess_func = Some(ElementHook::Legacy(turret_postprocess));
                (*turret).pParent = ss.cast();
                InsertQueue((&raw mut disp_q).cast(), h, (*e).succ);
            }
        }
    }
}
