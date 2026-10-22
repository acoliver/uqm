//! Syreen callbacks registered with the live engine race descriptor.
//!
//! Every value and expression below mirrors `sc2/src/uqm/ships/syreen/syreen.c`,
//! including C integer promotion and truncation, so battles replay identically.

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use crate::ships::native::{abi::*, element, put_element, starship};
    use std::{mem, ptr};

    // Core characteristics
    const MAX_CREW: u16 = 12;
    /// SYREEN_MAX_CREW_SIZE is MAX_CREW_SIZE from element.h
    const SYREEN_MAX_CREW_SIZE: u16 = MAX_CREW_SIZE as u16;
    const MAX_ENERGY: u8 = 16;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 6;
    const MAX_THRUST: u16 = 36;
    const THRUST_INCREMENT: u16 = 9;
    const THRUST_WAIT: u8 = 1;
    const TURN_WAIT: u8 = 1;
    const SHIP_MASS: u8 = 2;

    // Particle Beam Stiletto
    const WEAPON_ENERGY_COST: u8 = 1;
    const WEAPON_WAIT: u8 = 8;
    const SYREEN_OFFSET: i16 = 30;
    /// DISPLAY_TO_WORLD (30)
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 10;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 2;
    const MISSILE_OFFSET: i16 = 3;

    // Syreen song
    const SPECIAL_ENERGY_COST: u8 = 5;
    const SPECIAL_WAIT: u8 = 20;
    /// Originally SPACE_HEIGHT.
    const ABANDONER_RANGE: i32 = 208;
    /// (DWORD)((UWORD)ABANDONER_RANGE * (UWORD)ABANDONER_RANGE)
    const ABANDONER_RANGE_SQUARED: u32 = (ABANDONER_RANGE * ABANDONER_RANGE) as u32;
    const MAX_ABANDONERS: i32 = 8;

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut desc: RACE_DESC = unsafe { mem::zeroed() };
        desc.ship_info.ship_flags = FIRES_FORE as u16;
        desc.ship_info.ship_cost = 13;
        desc.ship_info.crew_level = MAX_CREW;
        desc.ship_info.max_crew = SYREEN_MAX_CREW_SIZE;
        desc.ship_info.energy_level = MAX_ENERGY;
        desc.ship_info.max_energy = MAX_ENERGY;
        desc.ship_info.race_strings_rsc = c"ship.syreen.text".as_ptr();
        desc.ship_info.icons_rsc = c"ship.syreen.icons".as_ptr();
        desc.ship_info.melee_icon_rsc = c"ship.syreen.meleeicons".as_ptr();
        desc.fleet.strength = 0;
        desc.fleet.known_loc = POINT { x: 0, y: 0 };
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
            c"ship.syreen.graphics.penetrator.large".as_ptr(),
            c"ship.syreen.graphics.penetrator.medium".as_ptr(),
            c"ship.syreen.graphics.penetrator.small".as_ptr(),
        ];
        desc.ship_data.weapon_rsc = [
            c"ship.syreen.graphics.dagger.large".as_ptr(),
            c"ship.syreen.graphics.dagger.medium".as_ptr(),
            c"ship.syreen.graphics.dagger.small".as_ptr(),
        ];
        // special_rsc stays NULL_RESOURCE for all three views.
        desc.ship_data.captain_control.captain_rsc = c"ship.syreen.graphics.captain".as_ptr();
        desc.ship_data.victory_ditty_rsc = c"ship.syreen.ditty".as_ptr();
        desc.ship_data.ship_sounds_rsc = c"ship.syreen.sounds".as_ptr();
        desc.cyborg_control.ManeuverabilityIndex = 0;
        desc.cyborg_control.WeaponRange =
            (MISSILE_SPEED as i32 * MISSILE_LIFE as i32 * 2 / 3) as u16;
        desc.cyborg_control.intelligence_func = Some(syreen_intelligence);
        desc.postprocess_func = Some(syreen_postprocess);
        desc.init_weapon_func = Some(initialize_dagger);
        desc
    }

    static mut SYREEN_DESC: RACE_DESC = descriptor();

    /// Returns the static template the engine copies for each Syreen ship.
    #[no_mangle]
    pub extern "C" fn init_syreen() -> *mut RACE_DESC {
        &raw mut SYREEN_DESC
    }

    unsafe extern "C" fn initialize_dagger(ship: *mut ELEMENT, daggers: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: SYREEN_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess_func: None,
            blast_offs: MISSILE_OFFSET,
        };
        *daggers = initialize_missile(&mut block);

        1
    }

    /// Queued by the song's first pass and run again as the element's death
    /// function, which is when the nearby enemy ships actually lose crew.
    unsafe extern "C" fn spawn_crew(source: *mut ELEMENT) {
        if (*source).state_flags & PLAYER_SHIP as u16 != 0 {
            let crew_handle = AllocElement();
            if !crew_handle.is_null() {
                let crew = element(crew_handle);
                (*crew).next.location = (*source).next.location;
                (*crew).playerNr = (*source).playerNr;
                (*crew).state_flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
                (*crew).__bindgen_anon_1.life_span = 0;
                (*crew).death_func = Some(spawn_crew);
                (*crew).pParent = (*source).pParent;
                (*crew).hTarget = ptr::null_mut();

                put_element(crew_handle);
            }
            return;
        }

        let mut handle = disp_q.head;
        while !handle.is_null() {
            let object = element(handle);
            // AbandonShip may append to the queue, so read the link first.
            let next = (*object).succ;

            if (*object).state_flags & PLAYER_SHIP as u16 != 0
                && (*object).playerNr != (*source).playerNr
                && (*object).__bindgen_anon_2.crew_level > 1
            {
                let mut dx = (i32::from((*object).next.location.x)
                    - i32::from((*source).next.location.x)) as SIZE;
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                let mut dy = (i32::from((*object).next.location.y)
                    - i32::from((*source).next.location.y)) as SIZE;
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }

                // WORLD_TO_DISPLAY
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                if i32::from(dx) <= ABANDONER_RANGE && i32::from(dy) <= ABANDONER_RANGE {
                    // (DWORD)((UWORD)dx * (UWORD)dx): the product is formed in
                    // `int` before it is widened.
                    let square =
                        |d: SIZE| i32::from(d as u16).wrapping_mul(i32::from(d as u16)) as u32;
                    let d_squared = square(dx).wrapping_add(square(dy));
                    if d_squared <= ABANDONER_RANGE_SQUARED {
                        let mut crew_loss = ((MAX_ABANDONERS
                            * (ABANDONER_RANGE - i32::from(square_root(d_squared))))
                            / ABANDONER_RANGE
                            + 1) as COUNT;
                        if crew_loss >= (*object).__bindgen_anon_2.crew_level {
                            crew_loss = (*object).__bindgen_anon_2.crew_level - 1;
                        }

                        AbandonShip(object, source, crew_loss);
                    }
                }
            }

            handle = next;
        }
    }

    unsafe extern "C" fn syreen_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        ship_intelligence(ship, concerns, count);

        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        if !(*enemy).ObjectPtr.is_null() {
            let ss = starship(ship);
            let enemy_ss = starship((*enemy).ObjectPtr);
            if (*(*enemy_ss).RaceDescPtr).ship_info.ship_flags & CREW_IMMUNE as u16 == 0
                && (*ss).special_counter == 0
                && (*(*enemy).ObjectPtr).__bindgen_anon_2.crew_level > 1
                && (*enemy).which_turn <= 14
            {
                (*ss).ship_input_state |= SPECIAL as u8;
            } else {
                (*ss).ship_input_state &= !(SPECIAL as u8);
            }
        }
    }

    unsafe extern "C" fn syreen_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        if (*ss).cur_status_flags & SPECIAL as u16 != 0
            && (*ss).special_counter == 0
            && DeltaEnergy(ship, -i16::from(SPECIAL_ENERGY_COST)) != 0
        {
            // SYREEN_SONG
            ProcessSound(
                SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1),
                ship,
            );
            spawn_crew(ship);

            (*ss).special_counter = (*(*ss).RaceDescPtr).characteristics.special_wait;
        }
    }
}
