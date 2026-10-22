//! Live Sa-Matra callbacks translated from ships/lastbat/lastbat.c.
#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, frame_index, normalize_facing, put_element, sine,
        starship, wrap_delta,
    };
    use crate::state::game_state_keys::get_game_state;
    use std::{ffi::c_void, mem};
    extern "C" {
        fn InsertQueue(q: *mut c_void, h: *mut c_void, before: *mut c_void);
    }
    const COMET_SPEED: SIZE = 12 << ONE_SHIFT;
    const SENTINEL_SPEED: SIZE = 8 << ONE_SHIFT;
    const RECOIL: i32 = (10 << ONE_SHIFT) << VELOCITY_SHIFT;
    const MAX_RECOIL: i32 = RECOIL * 4;
    const SHIP_MASS: u8 = (MAX_SHIP_MASS * 10) as u8;
    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.ship_flags = (IMMEDIATE_WEAPON | CREW_IMMUNE) as u16;
        d.ship_info.ship_cost = 16;
        d.ship_info.crew_level = 1;
        d.ship_info.max_crew = 1;
        d.ship_info.energy_level = MAX_ENERGY_SIZE as u8;
        d.ship_info.max_energy = MAX_ENERGY_SIZE as u8;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 2;
        d.characteristics.special_energy_cost = 3;
        d.characteristics.energy_wait = 6;
        d.characteristics.weapon_wait = 240;
        d.characteristics.special_wait = 72;
        d.characteristics.ship_mass = SHIP_MASS;
        d.ship_data.ship_rsc = [
            c"ship.samatra.graphics.samatra.large".as_ptr(),
            c"ship.samatra.graphics.samatra.medium".as_ptr(),
            c"ship.samatra.graphics.samatra.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.samatra.graphics.sentinel.large".as_ptr(),
            c"ship.samatra.graphics.sentinel.medium".as_ptr(),
            c"ship.samatra.graphics.sentinel.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.samatra.graphics.generator.large".as_ptr(),
            c"ship.samatra.graphics.generator.medium".as_ptr(),
            c"ship.samatra.graphics.generator.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.samatra.graphics.captain".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.samatra.sounds".as_ptr();
        d
    }
    static mut DESC: RACE_DESC = descriptor();
    #[no_mangle]
    pub unsafe extern "C" fn init_samatra() -> *mut RACE_DESC {
        DESC.preprocess_func = Some(samatra_preprocess);
        DESC.postprocess_func = Some(samatra_postprocess);
        DESC.cyborg_control.intelligence_func = Some(samatra_intelligence);
        &raw mut DESC
    }
    unsafe fn primitive(e: *mut ELEMENT) -> *mut PRIMITIVE {
        GlobData
            .Game_state
            .DisplayArray
            .add((*e).PrimIndex as usize)
    }
    unsafe fn insert(h: HELEMENT, before: HELEMENT) {
        InsertQueue((&raw mut disp_q).cast(), h, before);
    }
    unsafe extern "C" fn comet_preprocess(e: *mut ELEMENT) {
        let index = frame_index((*e).current.image.frame) + 1;
        if index < 29 {
            if index == 25 {
                let ss = starship(e);
                (*(*ss).RaceDescPtr).characteristics.weapon_wait = (*(*ss).RaceDescPtr)
                    .characteristics
                    .weapon_wait
                    .wrapping_add(1);
                spawn_comet(e);
                (*e).state_flags |= NONSOLID as u16;
                let (mut dx, mut dy) = (0, 0);
                GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut dx, &mut dy);
                SetVelocityComponents(&raw mut (*e).velocity, dx / 2, dy / 2);
            }
            (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
        }
        (*e).next.image.frame = SetAbsFrameIndex((*e).current.image.frame, index);
        (*e).state_flags |= CHANGING as u16;
    }
    unsafe extern "C" fn comet_collision(
        e: *mut ELEMENT,
        p: *mut POINT,
        o: *mut ELEMENT,
        q: *mut POINT,
    ) {
        if (*o).playerNr == RPG_PLAYER_NUM as SIZE {
            (*e).__bindgen_anon_3.mass_points = if (*o).state_flags & PLAYER_SHIP as u16 != 0 {
                2
            } else {
                50
            };
            let hits = (*e).__bindgen_anon_2.hit_points as u8;
            let life = (*e).__bindgen_anon_1.life_span;
            let blast = weapon_collision(e, p, o, q);
            if (*o).state_flags & PLAYER_SHIP as u16 != 0 {
                (*e).__bindgen_anon_2.hit_points = u16::from(hits);
                (*e).__bindgen_anon_1.life_span = life;
                (*e).state_flags &= !((DISAPPEARING | NONSOLID | COLLISION) as u16);
                if !blast.is_null() {
                    RemoveElement(blast);
                    FreeElement(blast);
                }
            }
            if (*e).state_flags & DISAPPEARING as u16 != 0 {
                let ss = starship(e);
                (*(*ss).RaceDescPtr).characteristics.weapon_wait = (*(*ss).RaceDescPtr)
                    .characteristics
                    .weapon_wait
                    .wrapping_sub(1);
            }
        }
    }
    unsafe fn spawn_comet(e: *mut ELEMENT) -> HELEMENT {
        let ss = starship(e);
        let mut b = MISSILE_BLOCK {
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: 0,
            index: 24,
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 12,
            damage: 2,
            life: 2,
            preprocess_func: Some(comet_preprocess),
            blast_offs: 0,
        };
        let h = initialize_missile(&mut b);
        if !h.is_null() {
            put_element(h);
            let c = element(h);
            (*c).collision_func = Some(comet_collision);
            (*c).pParent = ss.cast();
            (*c).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_4.turn_wait;
            (*c).hTarget = (*e).hTarget;
            let mut facing;
            if (*e).state_flags & PLAYER_SHIP as u16 != 0 {
                (*c).__bindgen_anon_4.turn_wait = 0;
                facing = TFB_Random() as COUNT;
                SetVelocityVector(&raw mut (*c).velocity, COMET_SPEED, facing);
            } else {
                (*c).velocity = (*e).velocity;
                (*c).__bindgen_anon_2.hit_points = (*e).__bindgen_anon_2.hit_points;
                facing = angle_to_facing((*c).velocity.TravelAngle as i32) as COUNT;
            }
            if (*c).__bindgen_anon_4.turn_wait != 0 {
                (*c).__bindgen_anon_4.turn_wait -= 1;
            } else {
                facing = normalize_facing(i32::from(facing));
                if TrackShip(c, &mut facing) > 0 {
                    SetVelocityVector(&raw mut (*c).velocity, COMET_SPEED, facing);
                }
                (*c).__bindgen_anon_4.turn_wait = 3;
            }
        }
        h
    }
    unsafe extern "C" fn turret_preprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            (*e).next.image.frame = SetAbsFrameIndex(
                (*e).current.image.frame,
                frame_index((*e).current.image.frame) % 10 + 1,
            );
            (*e).state_flags |= CHANGING as u16;
            (*e).__bindgen_anon_4.turn_wait = 0;
        }
    }
    unsafe extern "C" fn gate_collision(
        e: *mut ELEMENT,
        p: *mut POINT,
        o: *mut ELEMENT,
        q: *mut POINT,
    ) {
        if (*o).playerNr == RPG_PLAYER_NUM as SIZE {
            let ss = starship(e);
            if (*(*ss).RaceDescPtr).characteristics.max_thrust == 0 {
                if (*o).state_flags & FINITE_LIFE as u16 == 0 {
                    (*e).state_flags |= COLLISION as u16;
                }
                if (*o).state_flags & PLAYER_SHIP as u16 != 0
                    && (*primitive(e)).Type == gfx_object_STAMPFILL_PRIM as u8
                    && get_game_state("BOMB_CARRIER") != 0
                {
                    GlobData.Game_state.CurrentActivity &= !(IN_BATTLE as u16);
                }
            } else {
                (*e).__bindgen_anon_3.mass_points = if (*o).state_flags & PLAYER_SHIP as u16 != 0 {
                    1
                } else {
                    50
                };
                (*e).__bindgen_anon_2.hit_points = 100;
                let blast = weapon_collision(e, p, o, q);
                (*e).state_flags &= !((DISAPPEARING | NONSOLID | COLLISION) as u16);
                (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
                if !blast.is_null() {
                    RemoveElement(blast);
                    FreeElement(blast);
                }
            }
        }
    }
    unsafe extern "C" fn gate_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*(*ss).RaceDescPtr).characteristics.max_thrust == 0 {
            (*e).__bindgen_anon_3.mass_points = SHIP_MASS;
            (*e).state_flags &= !(FINITE_LIFE as u16);
            (*e).__bindgen_anon_1.life_span = (NORMAL_LIFE + 1) as COUNT;
            (*e).preprocess_func = None;
            (*primitive(e)).color = Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            };
            (*primitive(e)).Type = gfx_object_STAMPFILL_PRIM as u8;
        } else {
            (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
            (*e).next.image.frame = IncFrameIndex((*e).current.image.frame);
            if frame_index((*e).next.image.frame) == 0 {
                (*e).next.image.frame = SetAbsFrameIndex((*e).next.image.frame, 11);
            }
            (*e).state_flags |= CHANGING as u16;
        }
    }
    unsafe extern "C" fn generator_death(e: *mut ELEMENT) {
        if (*e).state_flags & FINITE_LIFE as u16 == 0 {
            let ss = starship(e);
            (*(*ss).RaceDescPtr).characteristics.max_thrust = (*(*ss).RaceDescPtr)
                .characteristics
                .max_thrust
                .wrapping_sub(1);
            (*e).state_flags |= (FINITE_LIFE | NONSOLID) as u16;
            (*e).preprocess_func = None;
            (*e).__bindgen_anon_4.turn_wait = 12;
            (*e).__bindgen_anon_5.thrust_wait = 0;
            (*e).current.image.frame = SetAbsFrameIndex((*e).current.image.frame, 9);
        }
        if (*e).__bindgen_anon_5.thrust_wait != 0 {
            (*e).__bindgen_anon_5.thrust_wait -= 1;
            (*e).state_flags &= !(DISAPPEARING as u16);
            (*e).state_flags |= CHANGING as u16;
            (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
        } else {
            let wait = (*e).__bindgen_anon_4.turn_wait;
            (*e).__bindgen_anon_4.turn_wait = wait.wrapping_sub(1);
            if wait != 0 {
                (*e).state_flags &= !(DISAPPEARING as u16);
                (*e).state_flags |= CHANGING as u16;
                (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
                (*e).next.image.frame = IncFrameIndex((*e).current.image.frame);
                (*e).__bindgen_anon_5.thrust_wait = 1;
            }
        }
    }
    unsafe extern "C" fn generator_preprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            (*e).__bindgen_anon_4.turn_wait =
                ((15 - i32::from((*e).__bindgen_anon_2.hit_points)) / 5) as u8;
            if (*e).__bindgen_anon_4.turn_wait < 3 {
                (*e).next.image.frame = SetAbsFrameIndex(
                    (*e).current.image.frame,
                    (frame_index((*e).current.image.frame) + 1) % 10,
                );
                (*e).state_flags |= CHANGING as u16;
            }
        }
    }
    unsafe extern "C" fn generator_collision(
        e: *mut ELEMENT,
        _p: *mut POINT,
        o: *mut ELEMENT,
        _q: *mut POINT,
    ) {
        if (*o).state_flags & FINITE_LIFE as u16 == 0 {
            (*e).state_flags |= COLLISION as u16;
        }
    }
    unsafe extern "C" fn sentinel_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        (*(*ss).RaceDescPtr).characteristics.special_wait = (*(*ss).RaceDescPtr)
            .characteristics
            .special_wait
            .wrapping_add(1);
        (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
        if (*e).__bindgen_anon_5.thrust_wait != 0 {
            (*e).__bindgen_anon_5.thrust_wait -= 1;
        } else {
            (*e).next.image.frame = SetAbsFrameIndex(
                (*e).current.image.frame,
                (frame_index((*e).current.image.frame) + 1) % 6,
            );
            (*e).state_flags |= CHANGING as u16;
            (*e).__bindgen_anon_5.thrust_wait = 1;
        }
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            let mut facing;
            if (*e).state_flags & NONSOLID as u16 == 0 {
                facing = angle_to_facing((*e).velocity.TravelAngle as i32) as COUNT;
            } else {
                (*e).state_flags &= !(NONSOLID as u16);
                facing = TFB_Random() as COUNT;
                SetVelocityVector(&raw mut (*e).velocity, SENTINEL_SPEED, facing);
            }
            facing = normalize_facing(i32::from(facing));
            if (*e).hTarget.is_null() {
                let mut f = facing;
                TrackShip(e, &mut f);
            }
            let target = if (*e).hTarget.is_null() {
                (*ss).hShip
            } else if (*ss).hShip.is_null() {
                (*e).hTarget
            } else {
                let ship = element((*ss).hShip);
                let enemy = element((*e).hTarget);
                let dx0 = (*ship)
                    .current
                    .location
                    .x
                    .wrapping_sub((*e).current.location.x);
                let dy0 = (*ship)
                    .current
                    .location
                    .y
                    .wrapping_sub((*e).current.location.y);
                let dx1 = (*ship)
                    .current
                    .location
                    .x
                    .wrapping_sub((*enemy).current.location.x);
                let dy1 = (*ship)
                    .current
                    .location
                    .y
                    .wrapping_sub((*enemy).current.location.y);
                if i64::from(dx0) * i64::from(dx0) + i64::from(dy0) * i64::from(dy0)
                    > i64::from(dx1) * i64::from(dx1) + i64::from(dy1) * i64::from(dy1)
                {
                    (*ss).hShip
                } else {
                    (*e).hTarget
                }
            };
            if !target.is_null() {
                let t = element(target);
                let width = (ScreenWidth - 64) << (ONE_SHIFT + 3);
                let height = ScreenHeight << (ONE_SHIFT + 3);
                let mut dx = wrap_delta(
                    (*t).current
                        .location
                        .x
                        .wrapping_sub((*e).current.location.x),
                    width,
                );
                let mut dy = wrap_delta(
                    (*t).current
                        .location
                        .y
                        .wrapping_sub((*e).current.location.y),
                    height,
                );
                let frames = (square_root(
                    (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32,
                ) >> 6)
                    .max(1);
                let mut v = (*t).velocity;
                GetNextVelocityComponents(&mut v, &mut dx, &mut dy, frames);
                dx = wrap_delta(
                    (*t).current
                        .location
                        .x
                        .wrapping_add(dx)
                        .wrapping_sub((*e).current.location.x),
                    width,
                );
                dy = wrap_delta(
                    (*t).current
                        .location
                        .y
                        .wrapping_add(dy)
                        .wrapping_sub((*e).current.location.y),
                    height,
                );
                dx = normalize_facing(angle_to_facing(ARCTAN(dx, dy) as i32) - i32::from(facing))
                    as SIZE;
                if dx > 0 {
                    if dx <= angle_to_facing(HALF_CIRCLE as i32) as SIZE {
                        facing = facing.wrapping_add(1);
                    } else {
                        facing = facing.wrapping_sub(1);
                    }
                }
                SetVelocityVector(&raw mut (*e).velocity, SENTINEL_SPEED, facing);
            }
            (*e).__bindgen_anon_4.turn_wait = 1;
        }
    }
    unsafe extern "C" fn sentinel_collision(
        e: *mut ELEMENT,
        p: *mut POINT,
        o: *mut ELEMENT,
        q: *mut POINT,
    ) {
        if (*o).playerNr == NPC_PLAYER_NUM as SIZE {
            if (*e).preprocess_func.map(|f| f as usize) == (*o).preprocess_func.map(|f| f as usize)
                && (*e).state_flags & DEFY_PHYSICS as u16 == 0
                && ((*p).x != (*e).IntersectControl.IntersectStamp.origin.x
                    || (*p).y != (*e).IntersectControl.IntersectStamp.origin.y)
            {
                let angle = ARCTAN((*p).x.wrapping_sub((*q).x), (*p).y.wrapping_sub((*q).y));
                SetVelocityComponents(
                    &raw mut (*e).velocity,
                    cosine(
                        i32::from(angle),
                        i32::from(SENTINEL_SPEED) << VELOCITY_SHIFT,
                    ),
                    sine(
                        i32::from(angle),
                        i32::from(SENTINEL_SPEED) << VELOCITY_SHIFT,
                    ),
                );
                (*e).__bindgen_anon_4.turn_wait = 1;
                (*e).state_flags |= (COLLISION | DEFY_PHYSICS) as u16;
            }
        } else {
            let hits = (*e).__bindgen_anon_2.hit_points as u8;
            let life = (*e).__bindgen_anon_1.life_span;
            (*e).__bindgen_anon_5.blast_offset = 0;
            let blast = weapon_collision(e, p, o, q);
            (*e).__bindgen_anon_5.thrust_wait = 0;
            if (*o).state_flags & PLAYER_SHIP as u16 != 0
                && (*o).__bindgen_anon_2.crew_level != 0
                && u32::from((*o).__bindgen_anon_3.mass_points) < MAX_SHIP_MASS * 10
            {
                (*e).__bindgen_anon_1.life_span = life;
                (*e).__bindgen_anon_2.hit_points = u16::from(hits);
                (*e).state_flags &= !(DISAPPEARING as u16);
                (*e).state_flags |= DEFY_PHYSICS as u16;
                (*e).__bindgen_anon_4.turn_wait = 12;
                let ss = starship(o);
                (*ss).cur_status_flags &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                if (*o).__bindgen_anon_4.turn_wait < COLLISION_TURN_WAIT as u8 {
                    (*o).__bindgen_anon_4.turn_wait += COLLISION_TURN_WAIT as u8;
                }
                if (*o).__bindgen_anon_5.thrust_wait < COLLISION_THRUST_WAIT as u8 {
                    (*o).__bindgen_anon_5.thrust_wait += COLLISION_THRUST_WAIT as u8;
                }
                let angle = (*e).velocity.TravelAngle;
                DeltaVelocityComponents(
                    &raw mut (*o).velocity,
                    cosine(i32::from(angle), RECOIL),
                    sine(i32::from(angle), RECOIL),
                );
                let (mut dx, mut dy) = (0, 0);
                GetCurrentVelocityComponents(&raw mut (*o).velocity, &mut dx, &mut dy);
                if i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
                    > i64::from(MAX_RECOIL) * i64::from(MAX_RECOIL)
                {
                    let angle = ARCTAN(dx, dy);
                    SetVelocityComponents(
                        &raw mut (*o).velocity,
                        cosine(i32::from(angle), MAX_RECOIL),
                        sine(i32::from(angle), MAX_RECOIL),
                    );
                }
                (*e).velocity = mem::zeroed();
            }
            if (*e).state_flags & DISAPPEARING as u16 != 0 {
                let ss = starship(e);
                (*(*ss).RaceDescPtr).characteristics.special_wait = (*(*ss).RaceDescPtr)
                    .characteristics
                    .special_wait
                    .wrapping_sub(1);
                if !blast.is_null() {
                    let b = element(blast);
                    (*b).__bindgen_anon_1.life_span = 6;
                    (*b).current.image.frame = SetAbsFrameIndex(*(*b).current.image.farray, 6);
                }
            }
        }
    }
    unsafe extern "C" fn samatra_intelligence(e: *mut ELEMENT, c: *mut EVALUATE_DESC, n: COUNT) {
        ship_intelligence(e, c, n);
    }
    unsafe extern "C" fn samatra_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let d = (*ss).RaceDescPtr;
        if (*d).characteristics.max_thrust != 0 {
            if (*ss).weapon_counter == 0
                && (*d).characteristics.weapon_wait < 3
                && !spawn_comet(e).is_null()
            {
                (*ss).weapon_counter = 240;
            }
            if (*ss).special_counter == 0 && (*d).characteristics.special_wait < 4 {
                let mut b = MISSILE_BLOCK {
                    cx: (*e).next.location.x,
                    cy: (*e).next.location.y,
                    farray: (&raw mut (*d).ship_data.weapon).cast(),
                    face: 0,
                    index: 0,
                    sender: (*e).playerNr,
                    flags: 0,
                    pixoffs: 0,
                    speed: SENTINEL_SPEED,
                    hit_points: 10,
                    damage: 1,
                    life: 2,
                    preprocess_func: Some(sentinel_preprocess),
                    blast_offs: 0,
                };
                let h = initialize_missile(&mut b);
                if !h.is_null() {
                    let s = element(h);
                    (*s).collision_func = Some(sentinel_collision);
                    (*s).__bindgen_anon_4.turn_wait = 3;
                    (*s).pParent = ss.cast();
                    (*ss).special_counter = 72;
                    put_element(h);
                }
            }
        }
    }
    unsafe extern "C" fn samatra_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let d = (*ss).RaceDescPtr;
        (*d).characteristics.weapon_wait = 0;
        (*d).characteristics.special_wait = 0;
        if (*e).state_flags & APPEARING as u16 == 0 {
            (*e).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_4.turn_wait.wrapping_add(1);
            (*e).__bindgen_anon_5.thrust_wait = (*e).__bindgen_anon_5.thrust_wait.wrapping_add(1);
        } else {
            let offsets = [
                (-136, -35),
                (-47, -70),
                (35, -67),
                (118, -42),
                (115, 46),
                (64, 79),
                (-96, 76),
                (-145, 47),
            ];
            let center_x = ((ScreenWidth - 64) << (ONE_SHIFT + 3)) >> 1;
            let center_y = (ScreenHeight << (ONE_SHIFT + 3)) >> 1;
            (*d).characteristics.max_thrust = 0;
            while (*d).characteristics.max_thrust < 8 {
                let h = AllocElement();
                if !h.is_null() {
                    let g = element(h);
                    (*g).__bindgen_anon_2.hit_points = 15;
                    (*g).__bindgen_anon_3.mass_points = SHIP_MASS;
                    (*g).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
                    (*g).playerNr = (*e).playerNr;
                    (*g).state_flags = (APPEARING | IGNORE_SIMILAR) as u16;
                    (*primitive(g)).Type = gfx_object_STAMP_PRIM as u8;
                    let (x, y) = offsets[(*d).characteristics.max_thrust as usize];
                    (*g).current.location = POINT {
                        x: ((center_x + (x << ONE_SHIFT)) & !15) as i16,
                        y: ((center_y + (y << ONE_SHIFT)) & !15) as i16,
                    };
                    (*g).current.image.farray = (&raw mut (*d).ship_data.special).cast();
                    (*g).current.image.frame = SetAbsFrameIndex(
                        (*d).ship_data.special[0],
                        u16::from(TFB_Random() as u8 % 10),
                    );
                    (*g).preprocess_func = Some(generator_preprocess);
                    (*g).collision_func = Some(generator_collision);
                    (*g).death_func = Some(generator_death);
                    (*g).pParent = ss.cast();
                    insert(h, disp_q.head);
                }
                (*d).characteristics.max_thrust += 1;
            }
            let h = AllocElement();
            if !h.is_null() {
                let t = element(h);
                (*t).__bindgen_anon_2.hit_points = 1;
                (*t).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
                (*t).playerNr = (*e).playerNr;
                (*t).state_flags = (APPEARING | IGNORE_SIMILAR | NONSOLID) as u16;
                (*primitive(t)).Type = gfx_object_STAMP_PRIM as u8;
                (*t).current.location = POINT {
                    x: center_x as i16,
                    y: center_y as i16,
                };
                (*t).current.image.farray = (&raw mut (*d).ship_data.ship).cast();
                (*t).current.image.frame = SetAbsFrameIndex((*d).ship_data.ship[0], 1);
                (*t).preprocess_func = Some(turret_preprocess);
                (*t).pParent = ss.cast();
                insert(h, (*e).succ);
            }
            let h = AllocElement();
            if !h.is_null() {
                let g = element(h);
                (*g).__bindgen_anon_2.hit_points = 100;
                (*g).__bindgen_anon_3.mass_points = 1;
                (*g).__bindgen_anon_1.life_span = 2;
                (*g).playerNr = (*e).playerNr;
                (*g).state_flags = (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16;
                (*primitive(g)).Type = gfx_object_STAMP_PRIM as u8;
                (*g).current.location = POINT {
                    x: center_x as i16,
                    y: center_y as i16,
                };
                (*g).current.image.farray = (&raw mut (*d).ship_data.ship).cast();
                (*g).current.image.frame = SetAbsFrameIndex((*d).ship_data.ship[0], 11);
                (*g).preprocess_func = Some(gate_preprocess);
                (*g).collision_func = Some(gate_collision);
                (*g).pParent = ss.cast();
                insert(h, (*e).succ);
            }
            (*ss).weapon_counter = 120;
            (*ss).special_counter = 36;
        }
    }
}
