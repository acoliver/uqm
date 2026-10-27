//! Melnorme callbacks and descriptor for the live battle engine.
//! Expressions preserve the C promotions, truncations, traversal and RNG order.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{
        abi::*, cosine, element, facing_to_angle, frame_index, put_element, sine, starship,
    };
    use std::mem;

    use crate::collections::queue::InsertQueue;

    const MAX_CREW: u16 = 20;
    const MAX_ENERGY: u8 = 42;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const MAX_THRUST: u16 = 36;
    const THRUST_INCREMENT: u16 = 6;
    const THRUST_WAIT: u8 = 4;
    const TURN_WAIT: u8 = 4;
    const SHIP_MASS: u8 = 7;
    const WEAPON_ENERGY_COST: u8 = 5;
    const WEAPON_WAIT: u8 = 1;
    const MELNORME_OFFSET: i16 = 24;
    const MELNORME_SPEED: i16 = 24 << ONE_SHIFT;
    const LEVEL_COUNTER: u8 = 72;
    const MAX_PUMP: u8 = 4;
    const PUMPUP_SPEED: i16 = 45 << ONE_SHIFT;
    const PUMPUP_LIFE: u16 = 10;
    const PUMPUP_DAMAGE: u8 = 2;
    const MIN_PUMPITUDE_ANIMS: u16 = 3;
    const NUM_PUMP_ANIMS: u16 = 5;
    const REVERSE_DIR: u8 = 1 << 7;
    const SPECIAL_ENERGY_COST: u8 = 20;
    const SPECIAL_WAIT: u8 = 20;
    const CMISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const CMISSILE_LIFE: u16 = 20;
    const CMISSILE_HITS: i16 = 200;
    const CMISSILE_DAMAGE: i16 = 0;
    const CMISSILE_OFFSET: i16 = 4;
    const MAX_UNIVERSE: i16 = 9999;

    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = FIRES_FORE as u16;
        d.ship_info.ship_cost = 18;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.melnorme.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.melnorme.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.melnorme.meleeicons".as_ptr();
        d.fleet.strength = u16::MAX;
        d.fleet.known_loc = POINT {
            x: MAX_UNIVERSE >> 1,
            y: MAX_UNIVERSE >> 1,
        };
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
            c"ship.melnorme.graphics.trader.large".as_ptr(),
            c"ship.melnorme.graphics.trader.medium".as_ptr(),
            c"ship.melnorme.graphics.trader.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.melnorme.graphics.pumpup.large".as_ptr(),
            c"ship.melnorme.graphics.pumpup.medium".as_ptr(),
            c"ship.melnorme.graphics.pumpup.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.melnorme.graphics.confuse.large".as_ptr(),
            c"ship.melnorme.graphics.confuse.medium".as_ptr(),
            c"ship.melnorme.graphics.confuse.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.melnorme.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.melnorme.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.melnorme.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (PUMPUP_SPEED as i32 * PUMPUP_LIFE as i32) as u16;
        d
    }
    static mut MELNORME_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_melnorme() -> *mut RACE_DESC {
        unsafe {
            MELNORME_DESC.postprocess_func = Some(melnorme_postprocess);
            MELNORME_DESC.init_weapon_func = Some(initialize_pump_up);
            MELNORME_DESC.cyborg_control.intelligence_func = Some(melnorme_intelligence);
        }
        &raw mut MELNORME_DESC
    }

    pub(crate) unsafe extern "C" fn pump_up_preprocess(e: *mut ELEMENT) {
        (*e).__bindgen_anon_5.thrust_wait = (*e).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
        if (*e).__bindgen_anon_5.thrust_wait & 1 != 0 {
            let mut idx = frame_index((*e).current.image.frame);
            let turning_back = (*e).__bindgen_anon_4.turn_wait & REVERSE_DIR != 0;
            if (turning_back && !idx.is_multiple_of(NUM_PUMP_ANIMS))
                || (!turning_back && (i32::from(idx) + 1) % i32::from(NUM_PUMP_ANIMS) == 0)
            {
                idx = idx.wrapping_sub(1);
                (*e).__bindgen_anon_4.turn_wait |= REVERSE_DIR;
            } else {
                idx = idx.wrapping_add(1);
                (*e).__bindgen_anon_4.turn_wait &= !REVERSE_DIR;
            }
            image!(frame; (*e).next.image.frame, SetAbsFrameIndex((*e).current.image.frame, idx));
            (*e).state_flags |= CHANGING as u16;
        }
    }

    pub(crate) unsafe extern "C" fn pump_up_postprocess(e: *mut ELEMENT) {
        if (*e).state_flags & APPEARING as u16 != 0 {
            (*e).velocity = mem::zeroed();
            return;
        }
        let ss = starship(e);
        let ship_h = (*ss).hShip;
        let ship = element(ship_h);
        let mut handles = [std::ptr::null_mut(); 1];
        initialize_pump_up(ship, handles.as_mut_ptr());
        DeltaEnergy(ship, 0);
        let h = handles[0];
        let p = element(h);
        image!(copy; (*p).current.image.frame, (*e).current.image.frame);
        (*p).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_4.turn_wait;
        (*p).__bindgen_anon_5.thrust_wait = (*e).__bindgen_anon_5.thrust_wait;
        (*p).__bindgen_anon_5.thrust_wait = (*p).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
        if (*p).__bindgen_anon_5.thrust_wait == 0 {
            if ((*p).__bindgen_anon_4.turn_wait & !REVERSE_DIR) < MAX_PUMP - 1 {
                (*p).__bindgen_anon_4.turn_wait += 1;
                image!(frame; (*p).current.image.frame, crate::ships::native::rel_frame_index(
                    (*p).current.image.frame,
                    NUM_PUMP_ANIMS as i16,
                ));
                ProcessSound(
                    SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 2),
                    p,
                );
            }
            (*p).__bindgen_anon_5.thrust_wait = LEVEL_COUNTER;
        }
        // C computes damage from the old pulse, even on the growth tick.
        let damage = i32::from(PUMPUP_DAMAGE) << ((*e).__bindgen_anon_4.turn_wait & !REVERSE_DIR);
        (*p).__bindgen_anon_2.hit_points = damage as COUNT;
        (*p).__bindgen_anon_3.mass_points = (*p).__bindgen_anon_2.hit_points as BYTE;
        (*p).pParent = ss.cast();
        if (*p).__bindgen_anon_5.thrust_wait & 1 != 0 {
            let mut idx = frame_index((*p).current.image.frame);
            let back = (*p).__bindgen_anon_4.turn_wait & REVERSE_DIR != 0;
            if (back && !idx.is_multiple_of(NUM_PUMP_ANIMS))
                || (!back && (i32::from(idx) + 1) % i32::from(NUM_PUMP_ANIMS) == 0)
            {
                idx = idx.wrapping_sub(1);
                (*p).__bindgen_anon_4.turn_wait |= REVERSE_DIR;
            } else {
                idx = idx.wrapping_add(1);
                (*p).__bindgen_anon_4.turn_wait &= !REVERSE_DIR;
            }
            image!(frame; (*p).current.image.frame, SetAbsFrameIndex((*p).current.image.frame, idx));
        }
        if (*ss).cur_status_flags & (*ss).old_status_flags & WEAPON as u16 != 0 {
            (*ss).weapon_counter = WEAPON_WAIT;
        } else {
            (*p).__bindgen_anon_1.life_span = PUMPUP_LIFE;
            (*p).preprocess_func = Some(pump_up_preprocess);
            (*p).postprocess_func = None;
            let a = facing_to_angle(i32::from((*ss).ShipFacing)) as COUNT;
            SetVelocityComponents(
                &raw mut (*p).velocity,
                cosine(i32::from(a), i32::from(PUMPUP_SPEED) << VELOCITY_SHIFT),
                sine(i32::from(a), i32::from(PUMPUP_SPEED) << VELOCITY_SHIFT),
            );
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 3),
                p,
            );
        }
        put_element(h);
        (*GlobData
            .Game_state
            .DisplayArray
            .add(usize::from((*e).PrimIndex)))
        .Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
        (*e).state_flags |= NONSOLID as u16;
    }

    pub(crate) unsafe extern "C" fn pump_up_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        let mut rect: RECT = mem::zeroed();
        GetFrameRect((*a).next.image.frame, &mut rect);
        let wait = (*a).__bindgen_anon_5.thrust_wait;
        (*a).__bindgen_anon_5.blast_offset = (i32::from(rect.extent.width) >> 1) as BYTE;
        let blast = weapon_collision(a, pa, b, pb);
        (*a).__bindgen_anon_5.thrust_wait = wait;
        if !blast.is_null() {
            let p = element(blast);
            (*p).__bindgen_anon_1.life_span =
                MIN_PUMPITUDE_ANIMS + u16::from((*a).__bindgen_anon_4.turn_wait & !REVERSE_DIR);
            (*p).__bindgen_anon_4.turn_wait = 0;
            (*p).__bindgen_anon_5.next_turn = 0;
            (*p).preprocess_func = Some(animation_preprocess);
            image!(copy; (*p).current.image.farray, (*a).next.image.farray);
            image!(frame; (*p).current.image.frame, SetAbsFrameIndex(
                *(*p).current.image.farray,
                u16::from(MAX_PUMP) * NUM_PUMP_ANIMS,
            ));
        }
    }

    pub(crate) unsafe extern "C" fn initialize_pump_up(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: 0,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: MELNORME_OFFSET,
            speed: MELNORME_SPEED,
            hit_points: PUMPUP_DAMAGE as i16,
            damage: PUMPUP_DAMAGE as i16,
            life: 2,
            preprocess_func: None,
            blast_offs: 0,
        };
        *out = initialize_missile(&mut block);
        if !(*out).is_null() {
            let p = element(*out);
            (*p).postprocess_func = Some(pump_up_postprocess);
            (*p).collision_func = Some(pump_up_collision);
            (*p).__bindgen_anon_5.thrust_wait = LEVEL_COUNTER;
        }
        1
    }

    pub(crate) unsafe extern "C" fn confuse_preprocess(e: *mut ELEMENT) {
        if (*e).state_flags & NONSOLID as u16 == 0 {
            image!(frame; (*e).next.image.frame, SetAbsFrameIndex(
                (*e).current.image.frame,
                (frame_index((*e).current.image.frame).wrapping_add(1)) & 7,
            ));
            (*e).state_flags |= CHANGING as u16;
        } else if (*e).hTarget.is_null() {
            (*e).__bindgen_anon_1.life_span = 0;
            (*e).state_flags |= DISAPPEARING as u16;
        } else {
            let target = element((*e).hTarget);
            (*e).next.location = (*target).next.location;
            if (*e).__bindgen_anon_4.turn_wait != 0 {
                let mut next = IncFrameIndex((*e).current.image.frame);
                if frame_index(next) == 0 {
                    next = crate::ships::native::rel_frame_index(next, -8);
                }
                image!(frame; (*e).next.image.frame, next);
                let target_ss = starship(target);
                (*target_ss).ship_input_state = ((*target_ss).ship_input_state
                    & !((LEFT | RIGHT | SPECIAL) as u8))
                    | (*e).__bindgen_anon_4.turn_wait;
                let effect = AllocElement();
                if !effect.is_null() {
                    let p = element(effect);
                    (*p).playerNr = (*e).playerNr;
                    (*p).state_flags = (FINITE_LIFE | NONSOLID | CHANGING) as u16;
                    (*p).__bindgen_anon_1.life_span = 1;
                    image!(copy; (*p).current, (*e).next);
                    image!(copy; (*p).next, (*e).next);
                    (*p).preprocess_func = Some(confuse_preprocess);
                    (*GlobData
                        .Game_state
                        .DisplayArray
                        .add(usize::from((*p).PrimIndex)))
                    .Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                    (*p).pParent = starship(e).cast();
                    (*p).hTarget = (*e).hTarget;
                    put_element(effect);
                }
            }
        }
    }

    pub(crate) unsafe extern "C" fn confusion_collision(
        a: *mut ELEMENT,
        _pa: *mut POINT,
        b: *mut ELEMENT,
        _pb: *mut POINT,
    ) {
        if (*b).state_flags & PLAYER_SHIP as u16 == 0 {
            return;
        }
        let ss = starship(a);
        let desc = (*ss).RaceDescPtr;
        let mut h = disp_q.head;
        let mut found = std::ptr::null_mut();
        while !h.is_null() {
            let p = element(h);
            if (*p).playerNr == (*a).playerNr
                && (*p).current.image.farray == (&raw mut (*desc).ship_data.special).cast()
                && (*p).state_flags & NONSOLID as u16 != 0
            {
                found = h;
                break;
            }
            h = (*p).succ;
        }
        if found.is_null() {
            found = AllocElement();
        }
        if !found.is_null() {
            let p = element(found);
            if (*p).state_flags == 0 {
                InsertQueue((&raw mut disp_q).cast(), found, disp_q.head);
                image!(copy; (*p).current, (*a).next);
                image!(frame; (*p).current.image.frame, SetAbsFrameIndex((*p).current.image.frame, 8));
                image!(copy; (*p).next, (*p).current);
                (*p).playerNr = (*a).playerNr;
                (*p).state_flags = (FINITE_LIFE | NONSOLID | CHANGING) as u16;
                (*p).preprocess_func = Some(confuse_preprocess);
                (*GlobData
                    .Game_state
                    .DisplayArray
                    .add(usize::from((*p).PrimIndex)))
                .Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                (*p).pParent = ss.cast();
                let target_ss = starship(b);
                (*p).hTarget = (*target_ss).hShip;
            }
            (*p).__bindgen_anon_1.life_span = 400;
            (*p).__bindgen_anon_4.turn_wait = 1u8 << (TFB_Random() as u8 & 1);
        }
        (*a).__bindgen_anon_2.hit_points = 0;
        (*a).__bindgen_anon_1.life_span = 0;
        (*a).state_flags |= (DISAPPEARING | COLLISION | NONSOLID) as u16;
    }

    pub(crate) unsafe extern "C" fn initialize_confusion(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast(),
            index: 0,
            face: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: MELNORME_OFFSET,
            speed: CMISSILE_SPEED,
            hit_points: CMISSILE_HITS,
            damage: CMISSILE_DAMAGE,
            life: CMISSILE_LIFE,
            preprocess_func: Some(confuse_preprocess),
            blast_offs: CMISSILE_OFFSET,
        };
        *out = initialize_missile(&mut block);
        if !(*out).is_null() {
            let p = element(*out);
            (*p).collision_func = Some(confusion_collision);
            (*p).pParent = ss.cast();
        }
        1
    }

    pub(crate) unsafe extern "C" fn initialize_test_pump_up(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: 0,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: MELNORME_OFFSET,
            speed: PUMPUP_SPEED,
            hit_points: PUMPUP_DAMAGE as i16,
            damage: PUMPUP_DAMAGE as i16,
            life: PUMPUP_LIFE,
            preprocess_func: None,
            blast_offs: 0,
        };
        *out = initialize_missile(&mut block);
        1
    }

    pub(crate) unsafe extern "C" fn melnorme_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let desc = (*ss).RaceDescPtr;
        (*desc).init_weapon_func = Some(initialize_test_pump_up);
        let old = (*ss).weapon_counter;
        if old == WEAPON_WAIT {
            (*ss).weapon_counter = 0;
        }
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if !(*enemy).ObjectPtr.is_null() {
            if (*desc).ship_info.energy_level < SPECIAL_ENERGY_COST + WEAPON_ENERGY_COST
                && (*ss).old_status_flags & WEAPON as u16 == 0
            {
                (*enemy).MoveState = MOVEMENT_STATE_ENTICE;
            } else {
                let es = starship((*enemy).ObjectPtr);
                if (*(*es).RaceDescPtr).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 == 0 {
                    (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
                }
            }
        }
        ship_intelligence(ship, concerns, count);
        if (*ss).weapon_counter == 0
            && (old != 0
                || (((*ss).special_counter != 0
                    || (*desc).ship_info.energy_level >= SPECIAL_ENERGY_COST + WEAPON_ENERGY_COST)
                    && (*ss).ship_input_state & WEAPON as u8 == 0))
        {
            (*ss).ship_input_state ^= WEAPON as u8;
        }
        (*ss).ship_input_state &= !(SPECIAL as u8);
        if (*ss).special_counter == 0 && (*desc).ship_info.energy_level >= SPECIAL_ENERGY_COST {
            let old_input = (*ss).ship_input_state;
            (*desc).init_weapon_func = Some(initialize_confusion);
            (*ship).__bindgen_anon_4.turn_wait = (*ship).__bindgen_anon_4.turn_wait.wrapping_add(1);
            (*ship).__bindgen_anon_5.thrust_wait =
                (*ship).__bindgen_anon_5.thrust_wait.wrapping_add(1);
            ship_intelligence(ship, concerns, (ENEMY_SHIP_INDEX + 1) as COUNT);
            (*ship).__bindgen_anon_5.thrust_wait =
                (*ship).__bindgen_anon_5.thrust_wait.wrapping_sub(1);
            (*ship).__bindgen_anon_4.turn_wait = (*ship).__bindgen_anon_4.turn_wait.wrapping_sub(1);
            if (*ss).ship_input_state & WEAPON as u8 != 0 {
                (*ss).ship_input_state &= !(WEAPON as u8);
                (*ss).ship_input_state |= SPECIAL as u8;
            }
            (*ss).ship_input_state = old_input | ((*ss).ship_input_state & SPECIAL as u8);
        }
        (*ss).weapon_counter = old;
        (*desc).init_weapon_func = Some(initialize_pump_up);
    }

    pub(crate) unsafe extern "C" fn melnorme_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && DeltaEnergy(ship, -i16::from(SPECIAL_ENERGY_COST)) != 0
        {
            let mut h = std::ptr::null_mut();
            initialize_confusion(ship, &mut h);
            if !h.is_null() {
                let p = element(h);
                ProcessSound(
                    SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                    p,
                );
                put_element(h);
                (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
            }
        }
    }
}
