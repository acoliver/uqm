//! Androsynth callbacks and descriptor used by the live battle engine.
//!
//! Every value and expression below mirrors `sc2/src/uqm/ships/androsyn/androsyn.c`,
//! including C integer promotion and truncation, so battles replay identically.

#[cfg(not(test))]
mod engine {
    use crate::memory::{rust_hfree, rust_hmalloc};
    use crate::ships::native::{abi::*, element, starship};
    use std::{mem, ptr};

    // Core characteristics
    const MAX_CREW: u16 = 20;
    const MAX_ENERGY: u8 = 24;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 8;
    const MAX_THRUST: u16 = 24;
    const THRUST_INCREMENT: u16 = 3;
    const TURN_WAIT: u8 = 4;
    const THRUST_WAIT: u8 = 0;
    const SHIP_MASS: u8 = 6;

    // Bubbles
    const WEAPON_ENERGY_COST: u8 = 3;
    const WEAPON_WAIT: u8 = 0;
    const ANDROSYNTH_OFFSET: i16 = 14;
    const MISSILE_OFFSET: i16 = 3;
    /// DISPLAY_TO_WORLD (8)
    const MISSILE_SPEED: i16 = 8 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 200;
    const MISSILE_HITS: i16 = 3;
    const MISSILE_DAMAGE: i16 = 2;
    const TRACK_WAIT: u8 = 2;

    // Blazer
    const SPECIAL_ENERGY_COST: u8 = 2;
    const BLAZER_DEGENERATION: i16 = -1;
    const SPECIAL_WAIT: u8 = 0;
    const BLAZER_OFFSET: u8 = 10;
    const BLAZER_THRUST: i16 = 60;
    const BLAZER_TURN_WAIT: u8 = 1;
    const BLAZER_DAMAGE: u8 = 3;
    const BLAZER_MASS: u8 = 1;

    // intel.h: DISPLAY_TO_WORLD (50) and DISPLAY_TO_WORLD (1000). They are
    // function-like macros, so bindgen does not emit them.
    const CLOSE_RANGE_WEAPON: i32 = 50 << ONE_SHIFT;
    const LONG_RANGE_WEAPON: i32 = 1000 << ONE_SHIFT;
    /// races.h: ((COUNT) ~0)
    const INFINITE_RADIUS: u16 = u16::MAX;
    /// units.h: MAX_X_UNIVERSE and MAX_Y_UNIVERSE
    const MAX_UNIVERSE: i32 = 9999;

    const FULL_CIRCLE_ANGLE: i32 = FULL_CIRCLE as i32;
    const HALF_CIRCLE_ANGLE: i32 = HALF_CIRCLE as i32;
    const QUADRANT_ANGLE: i32 = QUADRANT as i32;
    const OCTANT_ANGLE: i32 = OCTANT as i32;
    const FACING_MASK: i32 = (1 << FACING_SHIFT) - 1;

    /// Per-instance data kept in `RACE_DESC.data`: the collision function the
    /// ship had before it became a blazer.
    #[repr(C)]
    struct AndrosynthData {
        collision_func: ElementCollisionFunc,
    }

    /// units.h ANGLE_TO_FACING; evaluated in `int` like the C macro.
    const fn angle_to_facing(angle: i32) -> i32 {
        (angle + (1 << (CIRCLE_SHIFT - FACING_SHIFT - 1))) >> (CIRCLE_SHIFT - FACING_SHIFT)
    }

    /// units.h NORMALIZE_ANGLE
    const fn normalize_angle(angle: i32) -> i32 {
        angle & (FULL_CIRCLE_ANGLE - 1)
    }

    /// libs/compiler.h MAKE_BYTE; the high nibble is shifted in a BYTE, so
    /// bits above the nibble are discarded exactly as in C.
    const fn make_byte(lo: u8, hi: u8) -> u8 {
        (hi << 4) | lo
    }

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut desc: RACE_DESC = unsafe { mem::zeroed() };
        desc.ship_info.ship_flags = (FIRES_FORE | SEEKING_WEAPON) as u16;
        desc.ship_info.ship_cost = 15;
        desc.ship_info.crew_level = MAX_CREW;
        desc.ship_info.max_crew = MAX_CREW;
        desc.ship_info.energy_level = MAX_ENERGY;
        desc.ship_info.max_energy = MAX_ENERGY;
        desc.ship_info.race_strings_rsc = c"ship.androsynth.text".as_ptr();
        desc.ship_info.icons_rsc = c"ship.androsynth.icons".as_ptr();
        desc.ship_info.melee_icon_rsc = c"ship.androsynth.meleeicons".as_ptr();
        desc.fleet.strength = INFINITE_RADIUS;
        desc.fleet.known_loc = POINT {
            x: (MAX_UNIVERSE >> 1) as i16,
            y: (MAX_UNIVERSE >> 1) as i16,
        };
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
            c"ship.androsynth.graphics.guardian.large".as_ptr(),
            c"ship.androsynth.graphics.guardian.medium".as_ptr(),
            c"ship.androsynth.graphics.guardian.small".as_ptr(),
        ];
        desc.ship_data.weapon_rsc = [
            c"ship.androsynth.graphics.bubble.large".as_ptr(),
            c"ship.androsynth.graphics.bubble.medium".as_ptr(),
            c"ship.androsynth.graphics.bubble.small".as_ptr(),
        ];
        desc.ship_data.special_rsc = [
            c"ship.androsynth.graphics.blazer.large".as_ptr(),
            c"ship.androsynth.graphics.blazer.medium".as_ptr(),
            c"ship.androsynth.graphics.blazer.small".as_ptr(),
        ];
        desc.ship_data.captain_control.captain_rsc = c"ship.androsynth.graphics.captain".as_ptr();
        desc.ship_data.victory_ditty_rsc = c"ship.androsynth.ditty".as_ptr();
        desc.ship_data.ship_sounds_rsc = c"ship.androsynth.sounds".as_ptr();
        desc.cyborg_control.ManeuverabilityIndex = 0;
        desc.cyborg_control.WeaponRange = (LONG_RANGE_WEAPON >> 2) as u16;
        desc.cyborg_control.intelligence_func = Some(androsynth_intelligence);
        desc.uninit_func = Some(uninit_androsynth);
        desc.preprocess_func = Some(androsynth_preprocess);
        desc.postprocess_func = Some(androsynth_postprocess);
        desc.init_weapon_func = Some(initialize_bubble);
        desc
    }

    static mut ANDROSYNTH_DESC: RACE_DESC = descriptor();

    /// Returns the static template the engine copies for each Androsynth ship.
    #[no_mangle]
    pub extern "C" fn init_androsynth() -> *mut RACE_DESC {
        &raw mut ANDROSYNTH_DESC
    }

    /// Replaces the engine-owned `RACE_DESC.data` block, as C `SetCustomShipData`.
    /// The block is a private copy, so it is freed by `uninit_androsynth` and
    /// never shared with the template descriptor.
    unsafe fn set_custom_ship_data(desc: *mut RACE_DESC, data: Option<AndrosynthData>) {
        if !(*desc).data.is_null() {
            rust_hfree((*desc).data);
            (*desc).data = ptr::null_mut();
        }
        if let Some(data) = data {
            let new_data = rust_hmalloc(mem::size_of::<AndrosynthData>()).cast::<AndrosynthData>();
            new_data.write(data);
            (*desc).data = new_data.cast();
        }
    }

    unsafe fn ship_frames(desc: *mut RACE_DESC) -> *mut FRAME {
        (&raw mut (*desc).ship_data.ship).cast()
    }

    unsafe fn special_frames(desc: *mut RACE_DESC) -> *mut FRAME {
        (&raw mut (*desc).ship_data.special).cast()
    }

    unsafe fn weapon_frames(desc: *mut RACE_DESC) -> *mut FRAME {
        (&raw mut (*desc).ship_data.weapon).cast()
    }

    unsafe extern "C" fn blazer_collision(
        a: *mut ELEMENT,
        pa: *mut POINT,
        b: *mut ELEMENT,
        pb: *mut POINT,
    ) {
        let old_crew_level = (*a).__bindgen_anon_2.crew_level;
        let old_life = (*a).__bindgen_anon_1.life_span;
        let old_offs = (*a).__bindgen_anon_5.blast_offset;
        (*a).__bindgen_anon_5.blast_offset = BLAZER_OFFSET;
        (*a).__bindgen_anon_3.mass_points = BLAZER_DAMAGE;
        weapon_collision(a, pa, b, pb);
        (*a).__bindgen_anon_3.mass_points = BLAZER_MASS;
        (*a).__bindgen_anon_5.blast_offset = old_offs;
        (*a).__bindgen_anon_1.life_span = old_life;
        (*a).__bindgen_anon_2.crew_level = old_crew_level;

        (*a).state_flags &= !((DISAPPEARING | NONSOLID) as u16);
        collision(a, pa, b, pb);
    }

    unsafe extern "C" fn bubble_preprocess(missile: *mut ELEMENT) {
        let packed = (*missile).__bindgen_anon_4.turn_wait;
        let mut thrust_wait = packed >> 4;
        let mut turn_wait = packed & 0x0f;
        if thrust_wait > 0 {
            thrust_wait -= 1;
        } else {
            (*missile).next.image.frame = IncFrameIndex((*missile).current.image.frame);
            (*missile).state_flags |= CHANGING as u16;

            thrust_wait = (TFB_Random() as u16 & 3) as u8;
        }

        if turn_wait > 0 {
            turn_wait -= 1;
        } else {
            let mut facing =
                (angle_to_facing(i32::from((*missile).velocity.TravelAngle)) & FACING_MASK) as u16;
            let delta_facing = TrackShip(missile, &mut facing);
            if delta_facing == -1 {
                facing = TFB_Random() as u16;
            } else if i32::from(delta_facing) <= angle_to_facing(HALF_CIRCLE_ANGLE) {
                facing = facing.wrapping_add(
                    TFB_Random() as u16 & (angle_to_facing(HALF_CIRCLE_ANGLE) - 1) as u16,
                );
            } else {
                facing = facing.wrapping_sub(
                    TFB_Random() as u16 & (angle_to_facing(HALF_CIRCLE_ANGLE) - 1) as u16,
                );
            }
            SetVelocityVector(&raw mut (*missile).velocity, MISSILE_SPEED, facing);
            turn_wait = TRACK_WAIT;
        }

        (*missile).__bindgen_anon_4.turn_wait = make_byte(turn_wait, thrust_wait);
    }

    unsafe extern "C" fn initialize_bubble(ship: *mut ELEMENT, bubbles: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: weapon_frames((*ss).RaceDescPtr),
            face: (*ss).ShipFacing,
            index: 0,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: ANDROSYNTH_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess_func: Some(bubble_preprocess),
            blast_offs: MISSILE_OFFSET,
        };
        *bubbles = initialize_missile(&mut block);

        if !(*bubbles).is_null() {
            (*element(*bubbles)).__bindgen_anon_4.turn_wait = 0;
        }

        1
    }

    unsafe extern "C" fn androsynth_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let desc = (*ss).RaceDescPtr;

        let enemy_weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        // in blazer form
        if (*ship).next.image.farray == special_frames(desc) {
            (*concerns.add(CREW_OBJECT_INDEX as usize)).ObjectPtr = ptr::null_mut();
            if !(*enemy_weapon).ObjectPtr.is_null()
                && (*enemy_weapon).MoveState == MOVEMENT_STATE_ENTICE
            {
                let threat_flags = (*(*enemy_weapon).ObjectPtr).state_flags;
                if threat_flags & FINITE_LIFE as u16 != 0 && threat_flags & CREW_OBJECT as u16 == 0
                {
                    (*enemy_weapon).MoveState = MOVEMENT_STATE_AVOID;
                } else {
                    (*enemy_weapon).ObjectPtr = ptr::null_mut();
                }
            }

            ship_intelligence(ship, concerns, count);
            return;
        }

        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        let mut enemy_ss: *mut STARSHIP = ptr::null_mut();
        if !(*enemy).ObjectPtr.is_null() {
            enemy_ss = starship((*enemy).ObjectPtr);
            let enemy_intel = (*(*enemy_ss).RaceDescPtr).cyborg_control;
            let enemy_crew = i32::from((*(*enemy).ObjectPtr).__bindgen_anon_2.crew_level);
            if (*enemy).which_turn <= 16
                && ((*ss).special_counter > 0
                    || i32::from((*desc).ship_info.energy_level) < i32::from(MAX_ENERGY) / 3
                    || ((i32::from(enemy_intel.WeaponRange) <= CLOSE_RANGE_WEAPON
                        && enemy_crew > i32::from(BLAZER_DAMAGE))
                        || (enemy_crew > i32::from(BLAZER_DAMAGE) * 3
                            && u32::from(enemy_intel.ManeuverabilityIndex) > SLOW_SHIP)))
            {
                (*enemy).MoveState = MOVEMENT_STATE_ENTICE;
            }
        }

        ship_intelligence(ship, concerns, count);

        if (*ss).special_counter == 0 {
            (*ss).ship_input_state &= !(SPECIAL as u8);
            let enemy_obj = (*enemy).ObjectPtr;
            let danger = !(*enemy_weapon).ObjectPtr.is_null() && (*enemy_weapon).which_turn <= 4;
            let lure = !enemy_obj.is_null()
                && i32::from((*desc).ship_info.energy_level) >= i32::from(MAX_ENERGY) / 3
                && {
                    let enemy_intel = (*(*enemy_ss).RaceDescPtr).cyborg_control;
                    let enemy_crew = i32::from((*enemy_obj).__bindgen_anon_2.crew_level);
                    i32::from(enemy_intel.WeaponRange)
                        >= i32::from((*desc).cyborg_control.WeaponRange) << 1
                        || ((*enemy).which_turn < 16
                            && (i32::from(enemy_intel.WeaponRange) > CLOSE_RANGE_WEAPON
                                || enemy_crew <= i32::from(BLAZER_DAMAGE))
                            && (enemy_crew <= i32::from(BLAZER_DAMAGE) * 3
                                || u32::from(enemy_intel.ManeuverabilityIndex) <= SLOW_SHIP))
                };
            if danger || lure {
                (*ss).ship_input_state |= SPECIAL as u8;
            }
        }

        if (*ss).ship_input_state & SPECIAL as u8 == 0
            && (*ss).weapon_counter == 0
            && !(*enemy).ObjectPtr.is_null()
        {
            let target = (*enemy).ObjectPtr;
            if (*enemy).which_turn <= 4 {
                (*ss).ship_input_state |= WEAPON as u8;
            } else if (*enemy).MoveState != MOVEMENT_STATE_PURSUE && (*enemy).which_turn <= 12 {
                let (mut ship_dx, mut ship_dy): (SIZE, SIZE) = (0, 0);
                let (mut other_dx, mut other_dy): (SIZE, SIZE) = (0, 0);
                GetCurrentVelocityComponents(&raw mut (*ship).velocity, &mut ship_dx, &mut ship_dy);
                GetCurrentVelocityComponents(
                    &raw mut (*target).velocity,
                    &mut other_dx,
                    &mut other_dy,
                );
                let delta_x = (i32::from(ship_dx) - i32::from(other_dx)) as SIZE;
                let delta_y = (i32::from(ship_dy) - i32::from(other_dy)) as SIZE;
                let travel_facing = ARCTAN(delta_x, delta_y);

                let delta_x = (i32::from((*target).next.location.x)
                    - i32::from((*ship).next.location.x)) as SIZE;
                let delta_y = (i32::from((*target).next.location.y)
                    - i32::from((*ship).next.location.y)) as SIZE;
                let direction_facing = ARCTAN(delta_x, delta_y);

                if normalize_angle(
                    i32::from(travel_facing) - i32::from(direction_facing) + OCTANT_ANGLE,
                ) <= QUADRANT_ANGLE
                {
                    (*ss).ship_input_state |= WEAPON as u8;
                }
            }
        }
    }

    unsafe extern "C" fn androsynth_postprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        let desc = (*ss).RaceDescPtr;

        // take care of blazer effect
        if (*ship).next.image.farray == special_frames(desc) {
            if (*ss).cur_status_flags & SPECIAL as u16 != 0 || (*desc).ship_info.energy_level == 0 {
                (*desc).characteristics.energy_regeneration = BLAZER_DEGENERATION as u8;
                (*ss).energy_counter = ENERGY_WAIT;

                if (*ss).cur_status_flags & SPECIAL as u16 != 0 {
                    // COMET_ON
                    ProcessSound(
                        SetAbsStringTableIndex((*desc).ship_data.ship_sounds, 1),
                        ship,
                    );
                    (*ship).__bindgen_anon_4.turn_wait = 0;
                    (*ship).__bindgen_anon_5.thrust_wait = 0;
                    (*desc).characteristics.special_wait = (*desc).characteristics.turn_wait;
                    (*ship).__bindgen_anon_3.mass_points = BLAZER_MASS;
                    (*desc).characteristics.turn_wait = BLAZER_TURN_WAIT;

                    // Save the current collision func because we were not the
                    // ones who set it.
                    set_custom_ship_data(
                        desc,
                        Some(AndrosynthData {
                            collision_func: (*ship).collision_func,
                        }),
                    );
                    (*ship).collision_func = Some(blazer_collision);
                }
            }

            // if blazer wasn't able to change back into ship give it a little
            // more juice to try to get out of its predicament.
            if (*desc).ship_info.energy_level == 0 {
                DeltaEnergy(ship, -BLAZER_DEGENERATION);
                (*ss).energy_counter = 1;
            }
        }
    }

    unsafe extern "C" fn androsynth_preprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        let desc = (*ss).RaceDescPtr;

        let mut cur_status_flags = (*ss).cur_status_flags;
        if (*ship).next.image.farray == ship_frames(desc) {
            if cur_status_flags & SPECIAL as u16 != 0 {
                if (*desc).ship_info.energy_level < SPECIAL_ENERGY_COST {
                    // so text will flash
                    DeltaEnergy(ship, -i16::from(SPECIAL_ENERGY_COST));
                } else {
                    cur_status_flags &= !(WEAPON as u16);

                    (*ship).next.image.farray = special_frames(desc);
                    (*ship).next.image.frame =
                        SetEquFrameIndex((*desc).ship_data.special[0], (*ship).next.image.frame);
                    (*ship).state_flags |= CHANGING as u16;
                }
            }
        } else {
            cur_status_flags &= !((THRUST | WEAPON | SPECIAL) as u16);

            // protection against vux
            if (*desc).characteristics.turn_wait > BLAZER_TURN_WAIT {
                (*desc).characteristics.special_wait = (*desc)
                    .characteristics
                    .special_wait
                    .wrapping_add((*desc).characteristics.turn_wait - BLAZER_TURN_WAIT);
                (*desc).characteristics.turn_wait = BLAZER_TURN_WAIT;
            }

            if (*desc).ship_info.energy_level == 0 {
                // ZeroVelocityComponents
                ptr::write_bytes(&raw mut (*ship).velocity, 0, 1);
                cur_status_flags &=
                    !((LEFT | RIGHT | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);

                (*desc).characteristics.turn_wait = (*desc).characteristics.special_wait;
                (*desc).characteristics.energy_regeneration = ENERGY_REGENERATION;
                (*ship).__bindgen_anon_3.mass_points = SHIP_MASS;
                (*ship).collision_func = (*(*desc).data.cast::<AndrosynthData>()).collision_func;
                (*ship).next.image.farray = ship_frames(desc);
                (*ship).next.image.frame =
                    SetEquFrameIndex((*desc).ship_data.ship[0], (*ship).next.image.frame);
                (*ship).state_flags |= CHANGING as u16;
            } else if (*ship).__bindgen_anon_5.thrust_wait != 0 {
                (*ship).__bindgen_anon_5.thrust_wait -= 1;
            } else {
                let mut facing = (*ss).ShipFacing;
                if (*ship).__bindgen_anon_4.turn_wait == 0
                    && cur_status_flags & (LEFT | RIGHT) as u16 != 0
                {
                    if cur_status_flags & LEFT as u16 != 0 {
                        facing = facing.wrapping_sub(1);
                    } else {
                        facing = facing.wrapping_add(1);
                    }
                }

                SetVelocityVector(
                    &raw mut (*ship).velocity,
                    BLAZER_THRUST,
                    facing & FACING_MASK as u16,
                );
                cur_status_flags |= (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            }
        }
        (*ss).cur_status_flags = cur_status_flags;
    }

    unsafe extern "C" fn uninit_androsynth(desc: *mut RACE_DESC) {
        set_custom_ship_data(desc, None);
    }
}
