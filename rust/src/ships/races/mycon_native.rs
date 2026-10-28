//! Mycon Podship callbacks and descriptor ported from `sc2/src/uqm/ships/mycon/mycon.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{abi::*, angle_to_facing, element, normalize_facing, starship};
    use std::ptr;

    const MAX_CREW: u16 = 20;
    const MAX_ENERGY: u8 = 40;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const MAX_THRUST: u16 = 27;
    const THRUST_INCREMENT: u16 = 9;
    const THRUST_WAIT: u8 = 6;
    const TURN_WAIT: u8 = 6;
    const SHIP_MASS: u8 = 7;
    const WEAPON_ENERGY_COST: u8 = 20;
    const WEAPON_WAIT: u8 = 5;
    const MYCON_OFFSET: i16 = 24;
    const MISSILE_OFFSET: i16 = 0;
    const NUM_PLASMAS: i32 = 11;
    const NUM_GLOBALLS: i32 = 8;
    const PLASMA_DURATION: i32 = 13;
    const MISSILE_LIFE: i32 = NUM_PLASMAS * PLASMA_DURATION;
    const MISSILE_SPEED: i16 = 8 << ONE_SHIFT;
    const MISSILE_DAMAGE: i32 = 10;
    const TRACK_WAIT: u8 = 1;
    const SPECIAL_ENERGY_COST: u8 = MAX_ENERGY;
    const REGENERATION_AMOUNT: i32 = 4;

    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE | SEEKING_WEAPON) as u16;
        d.ship_info.ship_cost = 21;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.mycon.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.mycon.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.mycon.meleeicons".as_ptr();
        d.fleet.strength = (1070 / SPHERE_RADIUS_INCREMENT * 2) as u16;
        d.fleet.known_loc = POINT { x: 6392, y: 2200 };
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
            special_wait: 0,
            ship_mass: SHIP_MASS,
        };
        d.ship_data.ship_rsc = [
            c"ship.mycon.graphics.podship.large".as_ptr(),
            c"ship.mycon.graphics.podship.medium".as_ptr(),
            c"ship.mycon.graphics.podship.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.mycon.graphics.plasma.large".as_ptr(),
            c"ship.mycon.graphics.plasma.medium".as_ptr(),
            c"ship.mycon.graphics.plasma.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.mycon.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.mycon.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.mycon.sounds".as_ptr();
        d.cyborg_control.WeaponRange = 800 << ONE_SHIFT;
        d.cyborg_control.intelligence_func = Some(mycon_intelligence);
        d.postprocess_func = Some(mycon_postprocess);
        d.init_weapon_func = Some(initialize_plasma);
        d
    }

    static mut MYCON_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_mycon() -> *mut RACE_DESC {
        unsafe {
            MYCON_DESC.postprocess_func = Some(mycon_postprocess);
            MYCON_DESC.init_weapon_func = Some(initialize_plasma);
            MYCON_DESC.cyborg_control.intelligence_func = Some(mycon_intelligence);
        }
        &raw mut MYCON_DESC
    }

    pub(crate) unsafe extern "C" fn plasma_preprocess(e: *mut ELEMENT) {
        if u16::from((*e).__bindgen_anon_3.mass_points) > (*e).__bindgen_anon_2.hit_points {
            (*e).__bindgen_anon_1.life_span =
                (*e).__bindgen_anon_2.hit_points * PLASMA_DURATION as u16;
        } else {
            (*e).__bindgen_anon_2.hit_points =
                (((i32::from((*e).__bindgen_anon_1.life_span) * MISSILE_DAMAGE + MISSILE_LIFE - 1)
                    / MISSILE_LIFE) as u8) as _;
        }
        (*e).__bindgen_anon_3.mass_points = (*e).__bindgen_anon_2.hit_points as u8;
        let index = (NUM_PLASMAS
            - ((i32::from((*e).__bindgen_anon_1.life_span) + PLASMA_DURATION - 1)
                / PLASMA_DURATION)) as u16;
        if index != crate::ships::native::frame_index((*e).next.image.frame) {
            image!(frame; (*e).next.image.frame, SetAbsFrameIndex((*e).next.image.frame, index));
            (*e).state_flags |= CHANGING as u16;
        }
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            let mut face = normalize_facing(angle_to_facing(i32::from((*e).velocity.travel_angle)));
            if track_ship(e, &mut face) > 0 {
                SetVelocityVector(&raw mut (*e).velocity, MISSILE_SPEED as SIZE, face);
            }
            (*e).__bindgen_anon_4.turn_wait = TRACK_WAIT;
        }
    }

    pub(crate) unsafe extern "C" fn plasma_blast_preprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_1.life_span >= u16::from((*e).__bindgen_anon_5.thrust_wait) {
            image!(frame; (*e).next.image.frame, IncFrameIndex((*e).next.image.frame));
        } else {
            image!(frame; (*e).next.image.frame, DecFrameIndex((*e).next.image.frame));
        }
        if !(*e).hTarget.is_null() {
            let target = element((*e).hTarget);
            (*e).next.location = (*target).next.location;
        }
        (*e).state_flags |= CHANGING as u16;
    }

    pub(crate) unsafe extern "C" fn plasma_collision(
        e0: *mut ELEMENT,
        p0: *mut POINT,
        e1: *mut ELEMENT,
        p1: *mut POINT,
    ) {
        let old_mass = (*e0).__bindgen_anon_3.mass_points as i16;
        let h = if (*e0).pParent != (*e1).pParent || (*e1).state_flags & PLAYER_SHIP as u16 != 0 {
            weapon_collision(e0, p0, e1, p1)
        } else {
            ptr::null_mut()
        };
        if h.is_null() {
            return;
        }
        let blast = element(h);
        (*blast).pParent = (*e0).pParent;
        if (*e1).state_flags & PLAYER_SHIP as u16 == 0 {
            (*blast).hTarget = ptr::null_mut();
        } else {
            (*blast).hTarget = (*starship(e1)).hShip;
        }
        (*blast).current.location = (*e1).current.location;
        let mut animations =
            ((i32::from(old_mass) * NUM_GLOBALLS + MISSILE_DAMAGE - 1) / MISSILE_DAMAGE) as u16;
        if animations == 0 {
            animations = 1;
        }
        (*blast).__bindgen_anon_5.thrust_wait = animations as u8;
        (*blast).__bindgen_anon_1.life_span = (animations << 1) - 1;
        (*blast).preprocess_func = Some(ElementHook::Legacy(plasma_blast_preprocess));
        image!(copy; (*blast).current.image.farray, (*e0).next.image.farray);
        image!(frame; (*blast).current.image.frame, SetAbsFrameIndex(*(*blast).current.image.farray, NUM_PLASMAS as u16));
    }

    pub(crate) unsafe extern "C" fn mycon_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let enemy_weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*enemy_weapon).ObjectPtr.is_null()
            && (*enemy_weapon).MoveState == MOVEMENT_STATE_ENTICE
        {
            if (*(*enemy_weapon).ObjectPtr).state_flags & FINITE_LIFE as u16 != 0
                && (*(*enemy_weapon).ObjectPtr).state_flags & CREW_OBJECT as u16 == 0
            {
                (*enemy_weapon).MoveState = MOVEMENT_STATE_AVOID;
            } else {
                (*enemy_weapon).MoveState = MOVEMENT_STATE_PURSUE;
            }
        }
        ship_intelligence(ship, concerns, count);
        let ss = starship(ship);
        if (*concerns.add(ENEMY_WEAPON_INDEX as usize)).MoveState == MOVEMENT_STATE_PURSUE {
            (*ss).ship_input_state &= !(THRUST as u8);
        }
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if (*ss).weapon_counter == 0
            && !(*enemy).ObjectPtr.is_null()
            && ((*enemy).which_turn <= 16
                || (*ship).__bindgen_anon_2.crew_level == (*(*ss).RaceDescPtr).ship_info.max_crew)
        {
            let travel = normalize_facing(angle_to_facing(
                i32::from((*ship).velocity.travel_angle) + HALF_CIRCLE as i32,
            ));
            let target = (*enemy).ObjectPtr;
            let dx = (i32::from((*target).current.location.x)
                - i32::from((*ship).current.location.x)) as SIZE;
            let dy = (i32::from((*target).current.location.y)
                - i32::from((*ship).current.location.y)) as SIZE;
            let direction = normalize_facing(angle_to_facing(i32::from(ARCTAN(dx, dy))));
            if normalize_facing(
                i32::from(direction) - i32::from((*ss).ShipFacing)
                    + angle_to_facing(QUADRANT as i32),
            ) <= angle_to_facing(HALF_CIRCLE as i32) as u16
                && ((*ss).cur_status_flags & (SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16
                    == 0
                    || normalize_facing(
                        i32::from(direction) - i32::from(travel) + angle_to_facing(OCTANT as i32),
                    ) <= angle_to_facing(QUADRANT as i32) as u16)
            {
                (*ss).ship_input_state |= WEAPON as u8;
            }
        }
        if (*ss).special_counter == 0 {
            (*ss).ship_input_state &= !(SPECIAL as u8);
            (*(*ss).RaceDescPtr).cyborg_control.WeaponRange = 800 << ONE_SHIFT;
            if (*ship).__bindgen_anon_2.crew_level < (*(*ss).RaceDescPtr).ship_info.max_crew {
                (*(*ss).RaceDescPtr).cyborg_control.WeaponRange =
                    (i32::from(MISSILE_SPEED) * MISSILE_LIFE) as u16;
                if (*(*ss).RaceDescPtr).ship_info.energy_level >= SPECIAL_ENERGY_COST
                    && (*ss).ship_input_state & WEAPON as u8 == 0
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            }
        }
    }

    pub(crate) unsafe extern "C" fn initialize_plasma(
        ship: *mut ELEMENT,
        array: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: 0,
            sender: (*ship).playerNr,
            flags: 0,
            pixoffs: MYCON_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_DAMAGE as i16,
            damage: MISSILE_DAMAGE as i16,
            life: MISSILE_LIFE as u16,
            preprocess_func: Some(ElementHook::Legacy(plasma_preprocess)),
            blast_offs: MISSILE_OFFSET,
        };
        *array = initialize_missile(&mut block);
        if !(*array).is_null() {
            let p = element(*array);
            (*p).collision_func = Some(CollisionHook::Legacy(plasma_collision));
            (*p).__bindgen_anon_4.turn_wait = TRACK_WAIT + 2;
        }
        1
    }

    pub(crate) unsafe extern "C" fn mycon_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && (*e).__bindgen_anon_2.crew_level != (*(*ss).RaceDescPtr).ship_info.max_crew
            && DeltaEnergy(e, -(SPECIAL_ENERGY_COST as SIZE)) != 0
        {
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                e,
            );
            let mut add = REGENERATION_AMOUNT;
            let headroom = i32::from((*(*ss).RaceDescPtr).ship_info.max_crew)
                - i32::from((*e).__bindgen_anon_2.crew_level);
            if add > headroom {
                add = headroom;
            }
            DeltaCrew(e, add as SIZE);
            (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
        }
    }
}
