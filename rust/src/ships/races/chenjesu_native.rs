//! Chenjesu callbacks and descriptor translated from `sc2/src/uqm/ships/chenjesu/chenjesu.c`.

#[cfg(not(test))]
mod engine {
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_index, normalize_facing,
        put_element, sine, starship,
    };
    use std::mem;

    const MAX_CREW: u16 = 36;
    const MAX_ENERGY: u8 = 30;
    const MAX_THRUST: u16 = 27;
    const THRUST_INCREMENT: u16 = 3;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const TURN_WAIT: u8 = 6;
    const THRUST_WAIT: u8 = 4;
    const SHIP_MASS: u8 = 10;
    const WEAPON_ENERGY_COST: u8 = 5;
    const SPECIAL_ENERGY_COST: u8 = MAX_ENERGY;
    const WEAPON_WAIT: u8 = 0;
    const SPECIAL_WAIT: u8 = 0;
    const CHENJESU_OFFSET: i16 = 16;
    const MISSILE_OFFSET: i16 = 0;
    const MISSILE_SPEED: i16 = 16 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 90;
    const MISSILE_HITS: i16 = 10;
    const MISSILE_DAMAGE: i16 = 6;
    const NUM_SPARKLES: u16 = 8;
    const FRAGMENT_OFFSET: i16 = 2;
    const NUM_FRAGMENTS: i32 = 8;
    const FRAGMENT_LIFE: u16 = 10;
    const FRAGMENT_HITS: i16 = 1;
    const FRAGMENT_DAMAGE: i16 = 2;
    const DOGGY_OFFSET: i32 = 18;
    const DOGGY_SPEED: i16 = 8 << ONE_SHIFT;
    const ENERGY_DRAIN: u8 = 10;
    const MAX_DOGGIES: u8 = 4;
    const DOGGY_HITS: u16 = 3;
    const DOGGY_MASS: u8 = 4;
    const LONG_RANGE_WEAPON: u32 = 1000 << ONE_SHIFT;
    const FRAGMENT_RANGE: i32 = FRAGMENT_LIFE as i32 * MISSILE_SPEED as i32;

    const fn descriptor() -> RACE_DESC {
        let mut d: RACE_DESC = unsafe { mem::zeroed() };
        d.ship_info.ship_flags = (FIRES_FORE | SEEKING_SPECIAL | SEEKING_WEAPON) as u16;
        d.ship_info.ship_cost = 28;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.chenjesu.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.chenjesu.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.chenjesu.meleeicons".as_ptr();
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: ENERGY_REGENERATION,
            weapon_energy_cost: WEAPON_ENERGY_COST,
            special_energy_cost: SPECIAL_ENERGY_COST,
            energy_wait: ENERGY_WAIT,
            turn_wait: TURN_WAIT,
            thrust_wait: THRUST_WAIT,
            weapon_wait: WEAPON_WAIT,
            special_wait: SPECIAL_WAIT,
            ship_mass: SHIP_MASS,
        };
        d.ship_data.ship_rsc = [
            c"ship.chenjesu.graphics.broodhome.large".as_ptr(),
            c"ship.chenjesu.graphics.broodhome.medium".as_ptr(),
            c"ship.chenjesu.graphics.broodhome.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.chenjesu.graphics.spark.large".as_ptr(),
            c"ship.chenjesu.graphics.spark.medium".as_ptr(),
            c"ship.chenjesu.graphics.spark.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.chenjesu.graphics.doggy.large".as_ptr(),
            c"ship.chenjesu.graphics.doggy.medium".as_ptr(),
            c"ship.chenjesu.graphics.doggy.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.chenjesu.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.chenjesu.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.chenjesu.sounds".as_ptr();
        d.cyborg_control.WeaponRange = LONG_RANGE_WEAPON as u16;
        d
    }

    static mut CHENJESU_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_chenjesu() -> *mut RACE_DESC {
        unsafe {
            CHENJESU_DESC.preprocess_func = Some(chenjesu_preprocess);
            CHENJESU_DESC.postprocess_func = Some(chenjesu_postprocess);
            CHENJESU_DESC.init_weapon_func = Some(initialize_crystal);
            CHENJESU_DESC.cyborg_control.intelligence_func = Some(chenjesu_intelligence);
        }
        &raw mut CHENJESU_DESC
    }

    unsafe fn sound(ss: *mut STARSHIP, index: u16, source: *mut ELEMENT) {
        let sounds = (*(*ss).RaceDescPtr).ship_data.ship_sounds;
        ProcessSound(SetAbsStringTableIndex(sounds, index), source);
    }

    unsafe extern "C" fn crystal_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let mut block = MISSILE_BLOCK {
            cx: (*e).next.location.x,
            cy: (*e).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: 0,
            index: 1,
            sender: (*e).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: MISSILE_SPEED,
            hit_points: FRAGMENT_HITS,
            damage: FRAGMENT_DAMAGE,
            life: FRAGMENT_LIFE,
            preprocess_func: None,
            blast_offs: FRAGMENT_OFFSET,
        };
        let facing_circle = angle_to_facing(FULL_CIRCLE as i32);
        while i32::from(block.face) < facing_circle {
            let h = initialize_missile(&mut block);
            if !h.is_null() {
                (*element(h)).pParent = ss.cast();
                put_element(h);
            }
            block.face = (i32::from(block.face) + facing_circle / NUM_FRAGMENTS) as COUNT;
        }
        sound(ss, 1, e);
    }

    unsafe extern "C" fn crystal_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & WEAPON as u16 != 0 {
            (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_add(1);
        } else {
            (*e).__bindgen_anon_1.life_span = 1;
            (*e).postprocess_func = Some(crystal_postprocess);
        }
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

    unsafe extern "C" fn crystal_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        let h = weapon_collision(a, pa, b, pb);
        if !h.is_null() {
            let blast = element(h);
            (*blast).current.location = (*b).current.location;
            (*blast).__bindgen_anon_1.life_span = NUM_SPARKLES;
            (*blast).__bindgen_anon_4.turn_wait = 0;
            (*blast).__bindgen_anon_5.next_turn = 0;
            (*blast).preprocess_func = Some(animate);
            (*blast).current.image.farray = (*a).next.image.farray;
            (*blast).current.image.frame =
                SetAbsFrameIndex((*blast).current.image.farray.read(), 2);
        }
    }

    unsafe extern "C" fn doggy_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        (*ss).special_counter = (*ss).special_counter.wrapping_add(1);
        if (*e).__bindgen_anon_5.thrust_wait > 0 {
            (*e).__bindgen_anon_5.thrust_wait -= 1;
        } else {
            let original = normalize_facing(angle_to_facing(i32::from((*e).velocity.TravelAngle)));
            let mut facing = original;
            if TrackShip(e, &mut facing) < 0 {
                facing = normalize_facing(TFB_Random() as i32);
            } else {
                let target = element((*e).hTarget);
                // ARCTAN takes SIZE: C promotes the subtraction to int, then
                // truncates each argument to signed 16 bits at the call.
                let dx = (i32::from((*target).current.location.x)
                    - i32::from((*e).current.location.x)) as SIZE;
                let dy = (i32::from((*target).current.location.y)
                    - i32::from((*e).current.location.y)) as SIZE;
                facing = normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy))));
                let delta_facing = normalize_facing(
                    i32::from(facing) - i32::from(frame_index((*target).current.image.frame)),
                );
                if i32::from(delta_facing) > angle_to_facing(HALF_CIRCLE as i32 - OCTANT as i32)
                    && i32::from(delta_facing) < angle_to_facing(HALF_CIRCLE as i32 + OCTANT as i32)
                {
                    if i32::from(delta_facing) >= angle_to_facing(HALF_CIRCLE as i32) {
                        facing = (i32::from(facing) - angle_to_facing(QUADRANT as i32)) as COUNT;
                    } else {
                        facing = (i32::from(facing) + angle_to_facing(QUADRANT as i32)) as COUNT;
                    }
                }
                facing = normalize_facing(i32::from(facing));
            }
            if facing != original {
                SetVelocityVector(&raw mut (*e).velocity, DOGGY_SPEED, facing);
            }
        }
    }

    unsafe extern "C" fn doggy_death(e: *mut ELEMENT) {
        sound(starship(e), 3, e);
        (*e).state_flags &= !(DISAPPEARING as u16);
        (*e).state_flags |= (NONSOLID | FINITE_LIFE) as u16;
        (*e).__bindgen_anon_1.life_span = 6;
        (*e).preprocess_func = Some(animate);
        (*e).death_func = None;
        (*e).collision_func = None;
        (*e).velocity = mem::zeroed();
        (*e).__bindgen_anon_4.turn_wait = 0;
        (*e).__bindgen_anon_5.next_turn = 0;
    }

    unsafe extern "C" fn doggy_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        collision(a, pa, b, pb);
        if (*b).state_flags & PLAYER_SHIP as u16 != 0 && (*a).playerNr != (*b).playerNr {
            sound(starship(a), 2, a);
            let enemy = starship(b);
            let energy = (*(*enemy).RaceDescPtr).ship_info.energy_level;
            DeltaEnergy(b, -i16::from(energy.min(ENERGY_DRAIN)));
        }
        if u32::from((*a).__bindgen_anon_5.thrust_wait) <= COLLISION_THRUST_WAIT {
            (*a).__bindgen_anon_5.thrust_wait =
                (u32::from((*a).__bindgen_anon_5.thrust_wait) + (COLLISION_THRUST_WAIT << 1)) as u8;
        }
    }

    unsafe extern "C" fn spawn_doggy(e: *mut ELEMENT) {
        let h = AllocElement();
        if !h.is_null() {
            (*e).state_flags |= DEFY_PHYSICS as u16;
            put_element(h);
            let dog = element(h);
            (*dog).__bindgen_anon_2.hit_points = DOGGY_HITS;
            (*dog).__bindgen_anon_3.mass_points = DOGGY_MASS;
            (*dog).__bindgen_anon_5.thrust_wait = 0;
            (*dog).playerNr = (*e).playerNr;
            (*dog).state_flags = APPEARING as u16;
            (*dog).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
            let prim = GlobData
                .Game_state
                .DisplayArray
                .add((*dog).PrimIndex as usize);
            (*prim).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            (*dog).preprocess_func = Some(doggy_preprocess);
            (*dog).postprocess_func = None;
            (*dog).collision_func = Some(doggy_collision);
            (*dog).death_func = Some(doggy_death);
            let ss = starship(e);
            let angle =
                (facing_to_angle(i32::from((*ss).ShipFacing)) + HALF_CIRCLE as i32) as COUNT;
            let offset = (i32::from(CHENJESU_OFFSET) + DOGGY_OFFSET) << ONE_SHIFT;
            (*dog).current.location.x = (i32::from((*e).next.location.x)
                + i32::from(cosine(i32::from(angle), offset)))
                as COORD;
            (*dog).current.location.y = (i32::from((*e).next.location.y)
                + i32::from(sine(i32::from(angle), offset)))
                as COORD;
            (*dog).current.image.farray = (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast();
            (*dog).current.image.frame = (*(*ss).RaceDescPtr).ship_data.special[0];
            SetVelocityVector(
                &raw mut (*dog).velocity,
                DOGGY_SPEED,
                normalize_facing(angle_to_facing(i32::from(angle))),
            );
            (*dog).pParent = ss.cast();
            sound(ss, 4, dog);
        }
    }

    unsafe extern "C" fn chenjesu_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        (*ss).ship_input_state &= !(SPECIAL as u8);
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        let mut eval = enemy;
        if !(*enemy).ObjectPtr.is_null() {
            let enemy_ss = starship((*enemy).ObjectPtr);
            let control = &(*(*enemy_ss).RaceDescPtr).cyborg_control;
            if ((*enemy).which_turn <= 16 && u32::from(control.ManeuverabilityIndex) >= MEDIUM_SHIP)
                || (u32::from(control.ManeuverabilityIndex) <= SLOW_SHIP
                    && u32::from(control.WeaponRange) >= LONG_RANGE_WEAPON * 3 / 4
                    && (*(*enemy_ss).RaceDescPtr).ship_info.ship_flags & SEEKING_WEAPON as u16 != 0)
            {
                (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
            }
        }
        if (*ss).special_counter == 1
            && !(*weapon).ObjectPtr.is_null()
            && (*weapon).MoveState == MOVEMENT_STATE_ENTICE
            && (*weapon).which_turn <= 8
        {
            eval = weapon;
        }
        ship_intelligence(ship, concerns, count);
        if !(*eval).ObjectPtr.is_null() {
            let mut h = if (*ss).old_status_flags & WEAPON as u16 != 0 {
                disp_q.tail
            } else {
                std::ptr::null_mut()
            };
            while !h.is_null() {
                let crystal = element(h);
                let mut next = (*crystal).pred;
                if (*crystal).state_flags & NONSOLID as u16 == 0
                    && (*crystal).next.image.farray
                        == (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast()
                    && (*crystal).preprocess_func.is_some()
                    && (*crystal).__bindgen_anon_1.life_span > 0
                    && (*crystal).playerNr == (*ship).playerNr
                {
                    if !(*enemy).ObjectPtr.is_null() {
                        let turn = PlotIntercept(
                            crystal,
                            (*enemy).ObjectPtr,
                            (*crystal).__bindgen_anon_1.life_span,
                            (FRAGMENT_RANGE / 2) as COUNT,
                        );
                        if turn == 0
                            || (turn == 1
                                && PlotIntercept(
                                    crystal,
                                    (*enemy).ObjectPtr,
                                    (*crystal).__bindgen_anon_1.life_span,
                                    0,
                                ) == 0)
                        {
                            (*ss).ship_input_state &= !(WEAPON as u8);
                        } else if (*ss).weapon_counter == 0 {
                            (*ss).ship_input_state |= WEAPON as u8;
                            eval = enemy;
                        }
                        break;
                    }
                    next = std::ptr::null_mut();
                }
                h = next;
            }
            if h.is_null() {
                if (*ss).old_status_flags & WEAPON as u16 != 0 {
                    (*ss).ship_input_state &= !(WEAPON as u8);
                    if eval == weapon {
                        (*ss).weapon_counter = 3;
                    }
                } else if (*ss).weapon_counter == 0
                    && ship_weapons(ship, (*eval).ObjectPtr, (FRAGMENT_RANGE / 2) as COUNT) != 0
                {
                    (*ss).ship_input_state |= WEAPON as u8;
                }
            }
        }
        if (*ss).special_counter < MAX_DOGGIES
            && !(*eval).ObjectPtr.is_null()
            && (*(*ss).RaceDescPtr).ship_info.energy_level <= SPECIAL_ENERGY_COST
            && (*ss).ship_input_state & WEAPON as u8 == 0
        {
            (*ss).ship_input_state |= SPECIAL as u8;
        }
    }

    unsafe extern "C" fn initialize_crystal(ship: *mut ELEMENT, out: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: 0,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: CHENJESU_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess_func: Some(crystal_preprocess),
            blast_offs: MISSILE_OFFSET,
        };
        *out = initialize_missile(&mut block);
        if !(*out).is_null() {
            (*element(*out)).collision_func = Some(crystal_collision);
        }
        1
    }

    unsafe extern "C" fn chenjesu_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter < MAX_DOGGIES
            && DeltaEnergy(e, -i16::from(SPECIAL_ENERGY_COST)) != 0
        {
            spawn_doggy(e);
        }
        (*ss).special_counter = 1;
    }

    unsafe extern "C" fn chenjesu_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).special_counter > 1 {
            (*ss).special_counter = (*ss).special_counter.wrapping_add(MAX_DOGGIES);
        }
        if (*ss).cur_status_flags & (*ss).old_status_flags & WEAPON as u16 != 0 {
            (*ss).weapon_counter = (*ss).weapon_counter.wrapping_add(1);
        }
    }
}
