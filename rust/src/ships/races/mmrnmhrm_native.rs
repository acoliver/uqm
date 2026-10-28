//! Live engine callbacks for the Mmrnmhrm, mirroring `mmrnmhrm.c`.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::memory::{rust_hfree, rust_hmalloc};
    use crate::ships::native::{
        abi::*, cosine, facing_to_angle, normalize_angle, normalize_facing, sine, starship,
    };
    use std::{mem, ptr};

    const MAX_CREW: u16 = 20;
    const MAX_ENERGY: u8 = 10;
    const SHIP_MASS: u8 = 3;
    const ENERGY_REGENERATION: u8 = 2;
    const ENERGY_WAIT: u8 = 6;
    const MAX_THRUST: u16 = 20;
    const THRUST_INCREMENT: u16 = 5;
    const THRUST_WAIT: u8 = 1;
    const TURN_WAIT: u8 = 2;
    const YWING_ENERGY_REGENERATION: u8 = 1;
    const YWING_MAX_THRUST: u16 = 50;
    const YWING_THRUST_INCREMENT: u16 = 10;
    const YWING_THRUST_WAIT: u8 = 0;
    const YWING_TURN_WAIT: u8 = 14;
    const WEAPON_ENERGY_COST: u8 = 1;
    const WEAPON_WAIT: u8 = 0;
    const SPECIAL_ENERGY_COST: u8 = MAX_ENERGY;
    const SPECIAL_WAIT: u8 = 0;
    const YWING_WEAPON_ENERGY_COST: u8 = 1;
    const YWING_WEAPON_WAIT: u8 = 20;
    const MISSILE_LIFE: u16 = 40;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 1;
    const TRACK_WAIT: u8 = 5;
    const CLOSE_RANGE_WEAPON: i32 = 50 << ONE_SHIFT;
    const LONG_RANGE_WEAPON: i32 = 1000 << ONE_SHIFT;
    const SLOW_SHIP: u32 = 25;

    const YWING_CHARACTERISTICS: CHARACTERISTIC_STUFF = CHARACTERISTIC_STUFF {
        max_thrust: YWING_MAX_THRUST,
        thrust_increment: YWING_THRUST_INCREMENT,
        energy_regeneration: YWING_ENERGY_REGENERATION,
        weapon_energy_cost: YWING_WEAPON_ENERGY_COST,
        special_energy_cost: SPECIAL_ENERGY_COST,
        energy_wait: ENERGY_WAIT,
        turn_wait: YWING_TURN_WAIT,
        thrust_wait: YWING_THRUST_WAIT,
        weapon_wait: YWING_WEAPON_WAIT,
        special_wait: SPECIAL_WAIT,
        ship_mass: SHIP_MASS,
    };

    const fn descriptor() -> RACE_DESC {
        let mut desc = RACE_DESC::EMPTY;
        desc.ship_info.ship_flags = (FIRES_FORE | IMMEDIATE_WEAPON) as u16;
        desc.ship_info.ship_cost = 19;
        desc.ship_info.crew_level = MAX_CREW;
        desc.ship_info.max_crew = MAX_CREW;
        desc.ship_info.energy_level = MAX_ENERGY;
        desc.ship_info.max_energy = MAX_ENERGY;
        desc.ship_info.race_strings_rsc = c"ship.mmrnmhrm.text".as_ptr();
        desc.ship_info.icons_rsc = c"ship.mmrnmhrm.icons".as_ptr();
        desc.ship_info.melee_icon_rsc = c"ship.mmrnmhrm.meleeicons".as_ptr();
        desc.characteristics = CHARACTERISTIC_STUFF {
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
        desc.ship_data.ship_rsc = [
            c"ship.mmrnmhrm.graphics.xform.large".as_ptr(),
            c"ship.mmrnmhrm.graphics.xform.medium".as_ptr(),
            c"ship.mmrnmhrm.graphics.xform.small".as_ptr(),
        ];
        desc.ship_data.weapon_rsc = [
            c"ship.mmrnmhrm.graphics.torpedo.large".as_ptr(),
            c"ship.mmrnmhrm.graphics.torpedo.medium".as_ptr(),
            c"ship.mmrnmhrm.graphics.torpedo.small".as_ptr(),
        ];
        desc.ship_data.special_rsc = [
            c"ship.mmrnmhrm.graphics.ywing.large".as_ptr(),
            c"ship.mmrnmhrm.graphics.ywing.medium".as_ptr(),
            c"ship.mmrnmhrm.graphics.ywing.small".as_ptr(),
        ];
        desc.ship_data.captain_control.captain_rsc = c"ship.mmrnmhrm.graphics.captain".as_ptr();
        desc.ship_data.victory_ditty_rsc = c"ship.mmrnmhrm.ditty".as_ptr();
        desc.ship_data.ship_sounds_rsc = c"ship.mmrnmhrm.sounds".as_ptr();
        desc.cyborg_control.WeaponRange = CLOSE_RANGE_WEAPON as u16;
        desc.uninit_func = Some(uninit_mmrnmhrm);
        desc.preprocess_func = Some(mmrnmhrm_preprocess);
        desc.postprocess_func = Some(mmrnmhrm_postprocess);
        desc.init_weapon_func = Some(initialize_dual_weapons);
        desc.cyborg_control.intelligence_func = Some(mmrnmhrm_intelligence);
        desc
    }

    static mut MMRNMHRM_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_mmrnmhrm() -> *mut RACE_DESC {
        unsafe {
            MMRNMHRM_DESC = descriptor();
            set_custom_ship_data(&raw mut MMRNMHRM_DESC, Some(YWING_CHARACTERISTICS));
            &raw mut MMRNMHRM_DESC
        }
    }

    unsafe fn set_custom_ship_data(desc: *mut RACE_DESC, data: Option<CHARACTERISTIC_STUFF>) {
        if !(*desc).data.is_null() {
            rust_hfree((*desc).data);
            (*desc).data = ptr::null_mut();
        }
        if let Some(data) = data {
            let storage =
                rust_hmalloc(mem::size_of::<CHARACTERISTIC_STUFF>()).cast::<CHARACTERISTIC_STUFF>();
            storage.write(data);
            (*desc).data = storage.cast();
        }
    }

    pub(crate) unsafe extern "C" fn missile_preprocess(element: *mut ELEMENT) {
        if (*element).__bindgen_anon_4.turn_wait > 0 {
            (*element).__bindgen_anon_4.turn_wait -= 1;
        } else {
            let mut facing = crate::ships::native::frame_index((*element).next.image.frame);
            if TrackShip(element, &mut facing) > 0 {
                image!(frame; (*element).next.image.frame, SetAbsFrameIndex((*element).next.image.frame, facing));
                (*element).state_flags |= CHANGING as u16;
                SetVelocityVector(&mut (*element).velocity, 20 << ONE_SHIFT, facing);
            }
            (*element).__bindgen_anon_4.turn_wait = TRACK_WAIT;
        }
    }

    pub(crate) unsafe extern "C" fn mmrnmhrm_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let enemy_eval = concerns.add(ENEMY_SHIP_INDEX as usize);
        let enemy = (*enemy_eval).ObjectPtr;
        let can_transform = (*ss).special_counter == 0
            && (*(*ss).RaceDescPtr).ship_info.energy_level
                >= (*(*ss).RaceDescPtr).characteristics.special_energy_cost;
        let enemy_ss = if enemy.is_null() {
            ptr::null_mut()
        } else {
            starship(enemy)
        };

        ship_intelligence(ship, concerns, count);
        (*ss).ship_input_state &= !(SPECIAL as u8);
        if can_transform && !enemy.is_null() && (*ss).ship_input_state & WEAPON as u8 == 0 {
            let mut dx: SIZE = 0;
            let mut dy: SIZE = 0;
            GetCurrentVelocityComponents(&mut (*enemy).velocity, &mut dx, &mut dy);
            let (direction_angle, mut travel_angle);
            if dx == 0 && dy == 0 {
                direction_angle = 0;
                travel_angle = 0;
            } else {
                dx = ((*enemy).current.location.x as i32 - (*ship).current.location.x as i32)
                    as SIZE;
                dy = ((*enemy).current.location.y as i32 - (*ship).current.location.y as i32)
                    as SIZE;
                direction_angle = ARCTAN(-i32::from(dx) as _, -i32::from(dy) as _);
                travel_angle = (*enemy).velocity.travel_angle;
            }

            if (*ship).next.image.farray == (*(*ss).RaceDescPtr).ship_data.ship.as_mut_ptr() {
                if (*enemy_eval).which_turn > 8
                    && (u32::from(
                        (*(*enemy_ss).RaceDescPtr)
                            .cyborg_control
                            .ManeuverabilityIndex,
                    ) <= SLOW_SHIP
                        || normalize_angle(
                            direction_angle as i32 - travel_angle as i32 + QUADRANT as i32,
                        ) > HALF_CIRCLE as i32)
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            } else {
                let mut ship_dx: SIZE = 0;
                let mut ship_dy: SIZE = 0;
                GetCurrentVelocityComponents(&mut (*ship).velocity, &mut ship_dx, &mut ship_dy);
                dx = dx.wrapping_sub(ship_dx);
                dy = dy.wrapping_sub(ship_dy);
                travel_angle = ARCTAN(dx, dy);
                if (*enemy_eval).which_turn < 16 {
                    if (*enemy_eval).which_turn <= 8
                        || normalize_angle(
                            direction_angle as i32 - travel_angle as i32 + OCTANT as i32,
                        ) <= QUADRANT as i32
                    {
                        (*ss).ship_input_state |= SPECIAL as u8;
                    }
                } else if (*enemy_eval).which_turn > 32
                    && normalize_angle(
                        direction_angle as i32 - travel_angle as i32 + QUADRANT as i32,
                    ) > HALF_CIRCLE as i32
                {
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            }
        }
        if (*ship).current.image.farray == (*(*ss).RaceDescPtr).ship_data.special.as_mut_ptr() {
            if (*ss).ship_input_state & SPECIAL as u8 == 0 && !enemy.is_null() {
                (*ss).ship_input_state |= WEAPON as u8;
            } else {
                (*ss).ship_input_state &= !(WEAPON as u8);
            }
        }
    }

    pub(crate) unsafe extern "C" fn twin_laser_collision(
        e0: *mut ELEMENT,
        p0: *mut POINT,
        e1: *mut ELEMENT,
        p1: *mut POINT,
    ) {
        if (*e1).state_flags & PLAYER_SHIP as u16 == 0 || (*e0).playerNr != (*e1).playerNr {
            weapon_collision(e0, p0, e1, p1);
        }
    }

    pub(crate) unsafe extern "C" fn initialize_dual_weapons(
        ship: *mut ELEMENT,
        weapons: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let facing = (*ss).ShipFacing;
        let angle = facing_to_angle(i32::from(facing));
        let cx = (*ship)
            .next
            .location
            .x
            .wrapping_add(cosine(angle, 4 << ONE_SHIFT));
        let cy = (*ship)
            .next
            .location
            .y
            .wrapping_add(sine(angle, 4 << ONE_SHIFT));
        let race = (*ss).RaceDescPtr;
        if (*ship).next.image.farray == (*race).ship_data.ship.as_mut_ptr() {
            let ex = cx.wrapping_add(cosine(angle, 141 << ONE_SHIFT));
            let ey = cy.wrapping_add(sine(angle, 141 << ONE_SHIFT));
            let ox = -sine(angle, 10 << ONE_SHIFT);
            let oy = cosine(angle, 10 << ONE_SHIFT);
            let mut block: LASER_BLOCK = mem::zeroed();
            block.sender = (*ship).playerNr;
            block.flags = 0;
            block.pixoffs = 0;
            block.color = Color {
                r: 255,
                g: 82,
                b: 82,
                a: 255,
            };
            block.face = facing;
            for (index, sign) in [1i32, -1].iter().copied().enumerate() {
                block.cx = cx.wrapping_add((ox as i32 * sign) as _);
                block.cy = cy.wrapping_add((oy as i32 * sign) as _);
                block.ex = ex.wrapping_sub(block.cx);
                block.ey = ey.wrapping_sub(block.cy);
                *weapons.add(index) = initialize_laser(&mut block);
                let handle = *weapons.add(index);
                if !handle.is_null() {
                    let laser = crate::ships::native::element(handle);
                    (*laser).collision_func = Some(CollisionHook::Legacy(twin_laser_collision));
                }
            }
        } else {
            let mut block: MISSILE_BLOCK = mem::zeroed();
            block.farray = (*race).ship_data.weapon.as_mut_ptr();
            block.sender = (*ship).playerNr;
            block.flags = IGNORE_SIMILAR as u16;
            block.pixoffs = 0;
            block.speed = 20 << ONE_SHIFT;
            block.hit_points = MISSILE_HITS;
            block.damage = MISSILE_DAMAGE;
            block.life = MISSILE_LIFE;
            block.preprocess_func = Some(ElementHook::Legacy(missile_preprocess));
            block.blast_offs = 0;
            block.face = normalize_facing(i32::from(facing) - 1);
            block.index = block.face;
            let mut offs_x = -sine(facing_to_angle(i32::from(block.face)), 4 << ONE_SHIFT);
            let mut offs_y = cosine(facing_to_angle(i32::from(block.face)), 4 << ONE_SHIFT);
            for index in 0..2 {
                if index == 1 {
                    block.face = normalize_facing(i32::from(facing) + 1);
                    block.index = block.face;
                }
                block.cx = cx.wrapping_add(offs_x);
                block.cy = cy.wrapping_add(offs_y);
                *weapons.add(index) = initialize_missile(&mut block);
                let handle = *weapons.add(index);
                if !handle.is_null() {
                    (*crate::ships::native::element(handle))
                        .__bindgen_anon_4
                        .turn_wait = TRACK_WAIT;
                }
                offs_x = -offs_x;
                offs_y = -offs_y;
            }
        }
        2
    }

    pub(crate) unsafe extern "C" fn mmrnmhrm_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ship).next.image.farray == (*ship).current.image.farray {
            return;
        }
        ProcessSound(
            SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
            ship,
        );
        (*ss).weapon_counter = 0;
        if (*(*ss).RaceDescPtr).data.is_null() {
            return;
        }
        let race = (*ss).RaceDescPtr;
        let old = *((*race).data as *const CHARACTERISTIC_STUFF);
        let current = (*race).characteristics;
        set_custom_ship_data(race, Some(current));
        (*race).characteristics = old;
        (*race).cyborg_control.ManeuverabilityIndex = 0;
        if (*ship).next.image.farray == (*race).ship_data.special.as_mut_ptr() {
            (*race).cyborg_control.WeaponRange = LONG_RANGE_WEAPON as u16 - 1;
            (*race).ship_info.ship_flags &= !(IMMEDIATE_WEAPON as u16);
            (*race).ship_info.ship_flags |= SEEKING_WEAPON as u16;
            (*race).ship_data.ship_sounds =
                SetAbsStringTableIndex((*race).ship_data.ship_sounds, 2);
            (*ss).cur_status_flags &= !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
        } else {
            (*race).cyborg_control.WeaponRange = CLOSE_RANGE_WEAPON as u16;
            (*race).ship_info.ship_flags &= !(SEEKING_WEAPON as u16);
            (*race).ship_info.ship_flags |= IMMEDIATE_WEAPON as u16;
            (*race).ship_data.ship_sounds =
                SetAbsStringTableIndex((*race).ship_data.ship_sounds, 0);
            if (*ss).cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16 != 0 {
                (*ss).cur_status_flags |= (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            }
        }
    }

    pub(crate) unsafe extern "C" fn mmrnmhrm_preprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ship).state_flags & APPEARING as u16 == 0
            && (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && DeltaEnergy(
                ship,
                -i16::from((*(*ss).RaceDescPtr).characteristics.special_energy_cost),
            ) != 0
        {
            let race = (*ss).RaceDescPtr;
            if (*ship).next.image.farray == (*race).ship_data.ship.as_mut_ptr() {
                image!(set; (*ship).next.image.farray, (*race).ship_data.special.as_mut_ptr());
            } else {
                image!(set; (*ship).next.image.farray, (*race).ship_data.ship.as_mut_ptr());
            }
            image!(frame; (*ship).next.image.frame, SetEquFrameIndex(*(*ship).next.image.farray, (*ship).next.image.frame));
            (*ship).state_flags |= CHANGING as u16;
            (*ss).special_counter = (*race).characteristics.special_wait;
        }
    }

    pub(crate) unsafe extern "C" fn uninit_mmrnmhrm(race: *mut RACE_DESC) {
        set_custom_ship_data(race, None);
    }
}
