//! Arilou Skiff callbacks and descriptor for the live engine.
//!
//! Ported from `sc2/src/uqm/ships/arilou/arilou.c`. The teleport placement
//! macros (`DISPLAY_ALIGN_X/Y`, `WRAP_X/Y`, `LOG_SPACE_*`) depend on the
//! live screen size, so they are translated here with the same `int`/COUNT
//! /COORD conversions the C macros perform.

use crate::ships::native::abi::{CIRCLE_SHIFT, FACING_SHIFT, ONE_SHIFT, SIN_SHIFT};

/// `MAX_REDUCTION` from `units.h`: how far the battle view can zoom out.
const MAX_REDUCTION: i32 = 3;
/// `STATUS_WIDTH` from `units.h`.
const STATUS_WIDTH: i32 = 64;

/// `DISPLAY_TO_WORLD`.
const fn display_to_world(x: i32) -> i32 {
    x << ONE_SHIFT
}

/// `LOG_SPACE_WIDTH` for a given `SCREEN_WIDTH` (`SAFE_X` is 0).
const fn log_space_width(screen_width: i32) -> i32 {
    display_to_world(screen_width - STATUS_WIDTH) << MAX_REDUCTION
}

/// `LOG_SPACE_HEIGHT` for a given `SCREEN_HEIGHT` (`SAFE_Y` is 0).
const fn log_space_height(screen_height: i32) -> i32 {
    display_to_world(screen_height) << MAX_REDUCTION
}

/// `FACING_TO_ANGLE`.
const fn facing_to_angle(facing: i32) -> i32 {
    facing << (CIRCLE_SHIFT - FACING_SHIFT)
}

/// `NORMALIZE_FACING`: the macro masks an `int` and casts the result to COUNT.
const fn normalize_facing(facing: i32) -> u16 {
    (facing & ((1 << FACING_SHIFT) - 1)) as u16
}

/// `NORMALIZE_ANGLE`, as the index into `sinetab`.
const fn normalize_angle(angle: i32) -> usize {
    (angle & ((1 << CIRCLE_SHIFT) - 1)) as usize
}

/// The arithmetic of `SINE (a, m)` once `SINVAL (a)` is known: the product
/// is taken in `long` and the shifted result truncated to SIZE.
const fn scale_sine(sine_value: i16, magnitude: i32) -> i16 {
    ((sine_value as i64 * magnitude as i64) >> SIN_SHIFT) as i16
}

/// `DISPLAY_ALIGN_X` / `DISPLAY_ALIGN_Y`: the random value is cast to COUNT,
/// reduced modulo the logical extent, cast to COORD, then snapped to a
/// display pixel.
const fn display_align(random: u32, log_extent: i32) -> i32 {
    let reduced = (random as u16 as i32 % log_extent) as i16 as i32;
    reduced & !((1 << ONE_SHIFT) - 1)
}

/// `WRAP_VAL`, whose result is a COUNT.
const fn wrap_val(value: i32, extent: i32) -> u16 {
    (if value < 0 {
        value + extent
    } else if value >= extent {
        value - extent
    } else {
        value
    }) as u16
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::collections::observation::image;
    use crate::ships::native::{abi::*, starship};
    use std::ptr;

    // Core characteristics
    const MAX_CREW: u16 = 6;
    const MAX_ENERGY: u8 = 20;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 6;
    const MAX_THRUST: u16 = 40;
    const THRUST_INCREMENT: u16 = MAX_THRUST;
    const THRUST_WAIT: u8 = 0;
    const TURN_WAIT: u8 = 0;
    const SHIP_MASS: u8 = 1;

    // Tracking Laser
    const WEAPON_ENERGY_COST: u8 = 2;
    const WEAPON_WAIT: u8 = 1;
    const ARILOU_OFFSET: i16 = 9;
    const LASER_RANGE: i32 = display_to_world(100 + ARILOU_OFFSET as i32);

    // Teleporter
    const SPECIAL_ENERGY_COST: u8 = 3;
    const SPECIAL_WAIT: u8 = 2;
    const HYPER_LIFE: u16 = 5;

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = IMMEDIATE_WEAPON as u16;
        d.ship_info.ship_cost = 16;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.arilou.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.arilou.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.arilou.meleeicons".as_ptr();
        d.fleet.strength = 250 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet.known_loc = POINT { x: 438, y: 6372 };
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
            c"ship.arilou.graphics.skiff.large".as_ptr(),
            c"ship.arilou.graphics.skiff.medium".as_ptr(),
            c"ship.arilou.graphics.skiff.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.arilou.graphics.warp.large".as_ptr(),
            c"ship.arilou.graphics.warp.medium".as_ptr(),
            c"ship.arilou.graphics.warp.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.arilou.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.arilou.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.arilou.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (LASER_RANGE >> 1) as COUNT;
        d.cyborg_control.intelligence_func = Some(arilou_intelligence);
        d.preprocess_func = Some(arilou_preprocess);
        d.init_weapon_func = Some(initialize_autoaim_laser);
        d
    }

    static mut ARILOU_DESC: RACE_DESC = descriptor();

    /// Returns the static template copied by CaptureCodeRes for each ship.
    #[no_mangle]
    pub extern "C" fn init_arilou() -> *mut RACE_DESC {
        &raw mut ARILOU_DESC
    }

    /// `SINE (a, m)`.
    unsafe fn sine(angle: i32, magnitude: i32) -> SIZE {
        let table = (&raw const sinetab).cast::<SIZE>();
        scale_sine(*table.add(normalize_angle(angle)), magnitude)
    }

    /// `COSINE (a, m)`.
    unsafe fn cosine(angle: i32, magnitude: i32) -> SIZE {
        sine(angle + QUADRANT as i32, magnitude)
    }

    pub(crate) unsafe extern "C" fn initialize_autoaim_laser(
        ship: *mut ELEMENT,
        lasers: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let orig_facing = (*ss).ShipFacing;
        let mut face = orig_facing;
        let delta_facing = TrackShip(ship, &mut face);
        if delta_facing > 0 {
            face = normalize_facing(i32::from(orig_facing) + i32::from(delta_facing));
        }
        (*ship).hTarget = ptr::null_mut();

        let angle = facing_to_angle(i32::from(face));
        let mut block = LASER_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            ex: cosine(angle, LASER_RANGE),
            ey: sine(angle, LASER_RANGE),
            flags: IGNORE_SIMILAR as u16,
            sender: (*ship).playerNr,
            pixoffs: ARILOU_OFFSET,
            face,
            // BUILD_COLOR (MAKE_RGB15 (0x1F, 0x1F, 0x0A), 0x0E)
            color: Color {
                r: 0xFF,
                g: 0xFF,
                b: 0x52,
                a: 0xFF,
            },
        };
        *lasers = initialize_laser(&mut block);

        1
    }

    pub(crate) unsafe extern "C" fn arilou_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        (*ss).ship_input_state |= THRUST as u8;

        (*concerns.add(ENEMY_SHIP_INDEX as usize)).MoveState = MOVEMENT_STATE_ENTICE;
        ship_intelligence(ship, concerns, count);

        if (*ss).special_counter == 0 {
            (*ss).ship_input_state &= !(SPECIAL as u8);

            let eval = concerns.add(ENEMY_WEAPON_INDEX as usize);
            if !(*eval).ObjectPtr.is_null() && (*eval).which_turn <= 6 {
                let threat = (*eval).ObjectPtr;
                let enemy_race = (*starship(threat)).RaceDescPtr;
                let weapon_frames = (&raw mut (*enemy_race).ship_data.weapon).cast::<FRAME>();
                let special_frames = (&raw mut (*enemy_race).ship_data.special).cast::<FRAME>();
                let is_tracking_weapon =
                    ((*enemy_race).ship_info.ship_flags & SEEKING_WEAPON as u16 != 0
                        && (*threat).next.image.farray == weapon_frames)
                        || ((*enemy_race).ship_info.ship_flags & SEEKING_SPECIAL as u16 != 0
                            && (*threat).next.image.farray == special_frames);

                // PLAYER_SHIP here means an IMMEDIATE_WEAPON; the tracking
                // case covers fighters.
                if (((*threat).state_flags & PLAYER_SHIP as u16 != 0)
                    || (is_tracking_weapon
                        && ((*eval).which_turn == 1
                            || ((*threat).state_flags & CREW_OBJECT as u16 != 0)))
                    || PlotIntercept(threat, ship, 3, 0) != 0)
                    && (TFB_Random() & 3) == 0
                {
                    (*ss).ship_input_state &= !((LEFT | RIGHT | THRUST | WEAPON) as u8);
                    (*ss).ship_input_state |= SPECIAL as u8;
                }
            }
        }
        if u16::from((*(*ss).RaceDescPtr).ship_info.energy_level)
            <= u16::from(SPECIAL_ENERGY_COST) << 1
        {
            (*ss).ship_input_state &= !(WEAPON as u8);
        }
    }

    /// `ZeroVelocityComponents`: the macro memsets the whole VELOCITY_DESC,
    /// TravelAngle included.
    unsafe fn zero_velocity_components(velocity: *mut VELOCITY_DESC) {
        ptr::write_bytes(velocity, 0, 1);
    }

    /// `InitIntersectStartPoint` from `collide.h`.
    unsafe fn init_intersect_start_point(element: *mut ELEMENT) {
        let origin = &raw mut (*element).IntersectControl.IntersectStamp.origin;
        (*origin).x = (i32::from((*element).current.location.x) >> ONE_SHIFT) as COORD;
        (*origin).y = (i32::from((*element).current.location.y) >> ONE_SHIFT) as COORD;
    }

    pub(crate) unsafe extern "C" fn arilou_preprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        let race = (*ss).RaceDescPtr;
        let special_frames = (&raw mut (*race).ship_data.special).cast::<FRAME>();
        if (*ship).state_flags & NONSOLID as u16 == 0 {
            if (*ship).__bindgen_anon_5.thrust_wait == 0 {
                zero_velocity_components(&raw mut (*ship).velocity);
                (*ss).cur_status_flags &= !(SHIP_AT_MAX_SPEED as u16);
            }

            if ((*ss).cur_status_flags & SPECIAL as u16 != 0)
                && (*ss).special_counter == 0
                && DeltaEnergy(ship, -(SPECIAL_ENERGY_COST as SIZE)) != 0
            {
                // Special key is pressed; start teleport
                zero_velocity_components(&raw mut (*ship).velocity);
                (*ss).cur_status_flags &=
                    !((SHIP_AT_MAX_SPEED | LEFT | RIGHT | THRUST | WEAPON) as u16);

                (*ship).state_flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
                (*ship).__bindgen_anon_1.life_span = HYPER_LIFE;

                image!(set; (*ship).next.image.farray, special_frames);
                image!(frame; (*ship).next.image.frame, (*race).ship_data.special[0]);

                // HYPERJUMP
                ProcessSound(
                    SetAbsStringTableIndex((*race).ship_data.ship_sounds, 1),
                    ship,
                );
                (*ss).special_counter = (*race).characteristics.special_wait;
            }
        } else if (*ship).next.image.farray == special_frames {
            const INPUT_KEYS: u16 = (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;
            (*ss).cur_status_flags =
                ((*ss).cur_status_flags & !INPUT_KEYS) | ((*ss).old_status_flags & INPUT_KEYS);
            (*ss).weapon_counter = (*ss).weapon_counter.wrapping_add(1);
            (*ss).special_counter = (*ss).special_counter.wrapping_add(1);
            (*ss).energy_counter = (*ss).energy_counter.wrapping_add(1);
            (*ship).__bindgen_anon_4.turn_wait = (*ship).__bindgen_anon_4.turn_wait.wrapping_add(1);
            (*ship).__bindgen_anon_5.thrust_wait =
                (*ship).__bindgen_anon_5.thrust_wait.wrapping_add(1);

            // The original decrements this local copy only; the engine's
            // FINITE_LIFE processing owns the element's life_span.
            let mut life_span: COUNT = (*ship).__bindgen_anon_1.life_span;
            if life_span == NORMAL_LIFE as COUNT {
                // Ending teleport
                (*ship).state_flags &= !((NONSOLID | FINITE_LIFE) as u16);
                (*ship).state_flags |= APPEARING as u16;
                let ship_frames = (&raw mut (*race).ship_data.ship).cast::<FRAME>();
                image!(set; (*ship).current.image.farray, ship_frames);
                image!(set; (*ship).next.image.farray, ship_frames);
                let frame = SetAbsFrameIndex((*race).ship_data.ship[0], (*ss).ShipFacing);
                image!(frame; (*ship).current.image.frame, frame);
                image!(frame; (*ship).next.image.frame, frame);
                init_intersect_start_point(ship);
            } else {
                // Teleporting in progress
                life_span = life_span.wrapping_sub(1);
                if life_span != 2 {
                    if life_span < 2 {
                        image!(frame; (*ship).next.image.frame, DecFrameIndex((*ship).next.image.frame));
                    } else {
                        image!(frame; (*ship).next.image.frame, IncFrameIndex((*ship).next.image.frame));
                    }
                } else {
                    let log_width = log_space_width(ScreenWidth);
                    let log_height = log_space_height(ScreenHeight);
                    (*ship).next.location.x =
                        wrap_val(display_align(TFB_Random(), log_width), log_width) as COORD;
                    (*ship).next.location.y =
                        wrap_val(display_align(TFB_Random(), log_height), log_height) as COORD;
                }
            }

            (*ship).state_flags |= CHANGING as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_space_tracks_the_screen_size() {
        // 320x240 screen: 256x240 battle area, eight times zoomed out in
        // four-unit-per-pixel world coordinates.
        assert_eq!(log_space_width(320), 256 * 4 * 8);
        assert_eq!(log_space_height(240), 240 * 4 * 8);
        assert_eq!(log_space_width(640), 576 * 4 * 8);
        assert_eq!(log_space_height(480), 480 * 4 * 8);
    }

    #[test]
    fn display_align_casts_to_count_before_the_modulus() {
        let width = log_space_width(320);
        // Only the low sixteen bits of the random value participate.
        assert_eq!(display_align(0xABCD_0007, width), 4);
        assert_eq!(display_align(0x0001_0000, width), 0);
        assert_eq!(display_align(width as u32 + 5, width), 4);
        assert_eq!(display_align(width as u32 - 1, width), width - 4);
    }

    #[test]
    fn display_align_truncates_to_coord_before_snapping() {
        // 1280-wide screens give an extent above i16::MAX, so the COORD cast
        // can go negative and the wrap must add the extent back.
        let width = log_space_width(1280);
        assert_eq!(width, 1216 * 32);
        let aligned = display_align(0x9000, width);
        assert!(aligned < 0);
        assert_eq!(wrap_val(aligned, width) as i32, aligned + width);
    }

    #[test]
    fn wrap_val_folds_one_extent_in_either_direction() {
        assert_eq!(wrap_val(-4, 100), 96);
        assert_eq!(wrap_val(100, 100), 0);
        assert_eq!(wrap_val(250, 100), 150);
        assert_eq!(wrap_val(42, 100), 42);
    }

    #[test]
    fn facing_and_angle_conversions_follow_units_h() {
        assert_eq!(facing_to_angle(0), 0);
        assert_eq!(facing_to_angle(4), 16);
        assert_eq!(facing_to_angle(15), 60);
        assert_eq!(normalize_facing(19), 3);
        assert_eq!(normalize_facing(-1), 15);
        assert_eq!(normalize_angle(64 + 5), 5);
        assert_eq!(normalize_angle(-1), 63);
    }

    #[test]
    fn sine_scaling_shifts_the_long_product_and_truncates_to_size() {
        // sinetab peaks at 1 << SIN_SHIFT.
        assert_eq!(scale_sine(1 << 14, 436), 436);
        assert_eq!(scale_sine(-(1 << 14), 436), -436);
        assert_eq!(scale_sine(0, 436), 0);
        assert_eq!(scale_sine(8192, 436), 218);
        // The result is truncated to 16 bits, not saturated.
        assert_eq!(scale_sine(1 << 14, 0x1_0005), 5);
    }
}
