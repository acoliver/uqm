//! Ilwrath Avenger callbacks and descriptor for the live engine.
//!
//! Ported from `sc2/src/uqm/ships/ilwrath/ilwrath.c`. The cloak is a color
//! walk on the ship's display primitive, so colors are built the way
//! `gfxlib.h` builds them: `BUILD_COLOR` returns its RGB argument and drops
//! the legacy palette index, and `MAKE_RGB15` always yields alpha 255.

use crate::ships::native::abi::{Color, CIRCLE_SHIFT, FACING_SHIFT};

/// `ANGLE_TO_FACING`: rounds an angle to the nearest of sixteen facings.
const fn angle_to_facing(angle: i32) -> i32 {
    (angle + (1 << (CIRCLE_SHIFT - FACING_SHIFT - 1))) >> (CIRCLE_SHIFT - FACING_SHIFT)
}

/// `NORMALIZE_FACING`: the macro masks an `int` and casts the result to COUNT.
const fn normalize_facing(facing: i32) -> u16 {
    (facing & ((1 << FACING_SHIFT) - 1)) as u16
}

/// `CC5TO8` form 1: `(c << 3) | (c >> 2)`.
const fn cc5_to_8(c: u8) -> u8 {
    (c << 3) | (c >> 2)
}

/// `BUILD_COLOR (MAKE_RGB15 (r, g, b), index)`. The palette index is
/// meaningless in the engine and `BUILD_COLOR` discards it.
const fn build_color(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: cc5_to_8(r),
        g: cc5_to_8(g),
        b: cc5_to_8(b),
        a: 0xFF,
    }
}

/// `sameColor`.
const fn same_color(c1: Color, c2: Color) -> bool {
    c1.r == c2.r && c1.g == c2.g && c1.b == c2.b && c1.a == c2.a
}

/// `BLACK_COLOR`: a fully cloaked ship.
const BLACK_COLOR: Color = build_color(0x00, 0x00, 0x00);
/// Fully visible ship, and the first step of every cloak transition.
const WHITE_COLOR: Color = build_color(0x1F, 0x1F, 0x1F);
const CYAN_LIGHT: Color = build_color(0x0A, 0x1F, 0x1F);
const CYAN_DARK: Color = build_color(0x00, 0x14, 0x14);
const BLUE_LIGHT: Color = build_color(0x0A, 0x0A, 0x1F);
const BLUE_DARK: Color = build_color(0x00, 0x00, 0x14);

/// The color a decloaking ship takes after `color`, or `None` when `color`
/// is not one of the four intermediate steps (the C `else` arm).
const fn decloak_step(color: Color) -> Option<Color> {
    if same_color(color, CYAN_LIGHT) {
        Some(WHITE_COLOR)
    } else if same_color(color, CYAN_DARK) {
        Some(CYAN_LIGHT)
    } else if same_color(color, BLUE_LIGHT) {
        Some(CYAN_DARK)
    } else if same_color(color, BLUE_DARK) {
        Some(BLUE_LIGHT)
    } else {
        None
    }
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::ships::native::{abi::*, element, starship};

    // Core characteristics
    const MAX_CREW: u16 = 22;
    const MAX_ENERGY: u8 = 16;
    const ENERGY_REGENERATION: u8 = 4;
    const ENERGY_WAIT: u8 = 4;
    const MAX_THRUST: u16 = 25;
    const THRUST_INCREMENT: u16 = 5;
    const THRUST_WAIT: u8 = 0;
    const TURN_WAIT: u8 = 2;
    const SHIP_MASS: u8 = 7;
    /// Frames of look-ahead used when the cloak drops to aim the flame.
    const LOOK_AHEAD: u16 = 4;

    // Hellfire Spout
    const WEAPON_ENERGY_COST: u8 = 1;
    const WEAPON_WAIT: u8 = 0;
    const MISSILE_LIFE: u16 = 8;
    const ILWRATH_OFFSET: i16 = 29;
    const MISSILE_SPEED: i16 = MAX_THRUST as i16;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 1;
    const MISSILE_OFFSET: i16 = 0;

    // Cloaking Device
    const SPECIAL_ENERGY_COST: u8 = 3;
    const SPECIAL_WAIT: u8 = 13;

    /// `CLOSE_RANGE_WEAPON` from `intel.h`: `DISPLAY_TO_WORLD (50)`.
    const CLOSE_RANGE_WEAPON: u16 = display_to_world(50) as u16;

    /// `DISPLAY_TO_WORLD`.
    const fn display_to_world(x: i32) -> i32 {
        x << ONE_SHIFT
    }

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut d: RACE_DESC = unsafe { std::mem::zeroed() };
        d.ship_info.ship_flags = FIRES_FORE as u16;
        d.ship_info.ship_cost = 10;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.ilwrath.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.ilwrath.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.ilwrath.meleeicons".as_ptr();
        d.fleet.strength = 1410 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet.known_loc = POINT { x: 48, y: 1700 };
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
            c"ship.ilwrath.graphics.avenger.large".as_ptr(),
            c"ship.ilwrath.graphics.avenger.medium".as_ptr(),
            c"ship.ilwrath.graphics.avenger.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.ilwrath.graphics.fire.large".as_ptr(),
            c"ship.ilwrath.graphics.fire.medium".as_ptr(),
            c"ship.ilwrath.graphics.fire.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.ilwrath.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.ilwrath.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.ilwrath.sounds".as_ptr();
        d.cyborg_control.WeaponRange = CLOSE_RANGE_WEAPON;
        d.cyborg_control.intelligence_func = Some(ilwrath_intelligence);
        d.preprocess_func = Some(ilwrath_preprocess);
        d.init_weapon_func = Some(initialize_flame);
        d
    }

    static mut ILWRATH_DESC: RACE_DESC = descriptor();

    /// Returns the static template copied by CaptureCodeRes for each ship.
    #[no_mangle]
    pub extern "C" fn init_ilwrath() -> *mut RACE_DESC {
        &raw mut ILWRATH_DESC
    }

    /// `&(GLOBAL (DisplayArray))[ElementPtr->PrimIndex]`.
    unsafe fn display_prim(element: *mut ELEMENT) -> *mut PRIMITIVE {
        GlobData
            .Game_state
            .DisplayArray
            .add(usize::from((*element).PrimIndex))
    }

    /// `OBJECT_CLOAKED` from `element.h`.
    unsafe fn object_cloaked(element: *mut ELEMENT) -> bool {
        let prim = display_prim(element);
        (*prim).Type >= gfx_object_NUM_PRIMS as u8
            || ((*prim).Type == gfx_object_STAMPFILL_PRIM as u8
                && same_color((*prim).color, BLACK_COLOR))
    }

    unsafe extern "C" fn flame_preprocess(flame: *mut ELEMENT) {
        if (*flame).__bindgen_anon_4.turn_wait > 0 {
            (*flame).__bindgen_anon_4.turn_wait -= 1;
        } else {
            (*flame).next.image.frame = IncFrameIndex((*flame).current.image.frame);
            (*flame).state_flags |= CHANGING as u16;

            (*flame).__bindgen_anon_4.turn_wait = (*flame).__bindgen_anon_5.next_turn;
        }
    }

    unsafe extern "C" fn flame_collision(
        flame: *mut ELEMENT,
        flame_point: *mut POINT,
        other: *mut ELEMENT,
        other_point: *mut POINT,
    ) {
        weapon_collision(flame, flame_point, other, other_point);
        (*flame).state_flags &= !(DISAPPEARING as u16);
        (*flame).state_flags |= NONSOLID as u16;
    }

    unsafe extern "C" fn ilwrath_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
        (*enemy).MoveState = MOVEMENT_STATE_PURSUE;
        if !(*enemy).ObjectPtr.is_null() && (*enemy).which_turn <= 10 {
            // don't want to dodge when you could be flaming
            (*concerns.add(ENEMY_WEAPON_INDEX as usize)).ObjectPtr = std::ptr::null_mut();
        }

        ship_intelligence(ship, concerns, count);

        let ss = starship(ship);
        let enemy_weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
        if !(*enemy).ObjectPtr.is_null()
            && ((*enemy).which_turn <= 6
                || ((*enemy).which_turn <= 10 && (*enemy_weapon).which_turn <= 10))
        {
            (*ss).ship_input_state &= !(SPECIAL as u8);
            if object_cloaked(ship) {
                // The original reads `&= ~LEFT | RIGHT`, which is
                // `&= (~LEFT) | RIGHT` and therefore clears only LEFT.
                (*ss).ship_input_state &= (!(LEFT as i32) | RIGHT as i32) as u8;
                (*ss).ship_input_state |= THRUST as u8;
            }
            (*ss).ship_input_state |= WEAPON as u8;
        } else if (*ss).special_counter == 0
            && (GlobData.Game_state.CurrentActivity as u8 != IN_ENCOUNTER as u8
                || crate::state::ffi::rust_get_game_state(c"PROBE_ILWRATH_ENCOUNTER".as_ptr()) == 0)
        {
            (*ss).ship_input_state &= !(SPECIAL as u8);
            if !object_cloaked(ship) && (*ss).ship_input_state & WEAPON as u8 == 0 {
                (*ss).ship_input_state |= SPECIAL as u8;
            }
        }
    }

    unsafe extern "C" fn initialize_flame(ship: *mut ELEMENT, flames: *mut HELEMENT) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            flags: IGNORE_SIMILAR as u16,
            sender: (*ship).playerNr,
            pixoffs: ILWRATH_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            face: (*ss).ShipFacing,
            index: 0,
            life: MISSILE_LIFE,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast::<FRAME>(),
            preprocess_func: Some(flame_preprocess),
            blast_offs: MISSILE_OFFSET,
        };
        *flames = initialize_missile(&mut block);

        if !(*flames).is_null() {
            let flame = element(*flames);
            let mut dx: SIZE = 0;
            let mut dy: SIZE = 0;
            GetCurrentVelocityComponents(&raw mut (*ship).velocity, &mut dx, &mut dy);
            DeltaVelocityComponents(&raw mut (*flame).velocity, dx, dy);
            // VELOCITY_TO_WORLD (v) is `(v) >> VELOCITY_SHIFT` on a promoted int.
            (*flame).current.location.x = (i32::from((*flame).current.location.x)
                - (i32::from(dx) >> VELOCITY_SHIFT))
                as COORD;
            (*flame).current.location.y = (i32::from((*flame).current.location.y)
                - (i32::from(dy) >> VELOCITY_SHIFT))
                as COORD;

            (*flame).collision_func = Some(flame_collision);
            (*flame).__bindgen_anon_4.turn_wait = 0;
        }

        1
    }

    /// The weapon-discharge half of the decloak step: turn the ship toward
    /// where the target will be `LOOK_AHEAD` frames from now.
    unsafe fn aim_flame_after_decloak(ship: *mut ELEMENT, ss: *mut STARSHIP) {
        let mut facing = (*ss).ShipFacing;
        if TrackShip(ship, &mut facing) >= 0 {
            let target = element((*ship).hTarget);
            let mut v = (*target).velocity;
            let (mut dx0, mut dy0, mut dx1, mut dy1): (SIZE, SIZE, SIZE, SIZE) = (0, 0, 0, 0);
            GetNextVelocityComponents(&mut v, &mut dx0, &mut dy0, LOOK_AHEAD);
            v = (*ship).velocity;
            GetNextVelocityComponents(&mut v, &mut dx1, &mut dy1, LOOK_AHEAD);
            dx0 = ((i32::from((*target).current.location.x) + i32::from(dx0))
                - (i32::from((*ship).current.location.x) + i32::from(dx1)))
                as SIZE;
            dy0 = ((i32::from((*target).current.location.y) + i32::from(dy0))
                - (i32::from((*ship).current.location.y) + i32::from(dy1)))
                as SIZE;

            (*ss).ShipFacing = normalize_facing(angle_to_facing(i32::from(ARCTAN(dx0, dy0))));
            if (*ship).__bindgen_anon_4.turn_wait == 0 {
                (*ship).__bindgen_anon_4.turn_wait += 1;
            }
            (*ship).next.image.frame = SetAbsFrameIndex((*ship).next.image.frame, (*ss).ShipFacing);
        }
        (*ship).hTarget = std::ptr::null_mut();
    }

    unsafe extern "C" fn ilwrath_preprocess(ship: *mut ELEMENT) {
        let ss = starship(ship);
        let race = (*ss).RaceDescPtr;
        // Local copy: clearing SPECIAL below must not touch cur_status_flags.
        let mut status_flags: STATUS_FLAGS = (*ss).cur_status_flags;
        let prim = display_prim(ship);
        if (*prim).Type == gfx_object_STAMPFILL_PRIM as u8 {
            let color = (*prim).color;
            let weapon_discharge = (status_flags & WEAPON as u16 != 0)
                && (*race).ship_info.energy_level >= WEAPON_ENERGY_COST;
            if weapon_discharge
                || ((*ss).special_counter == 0
                    && ((status_flags & SPECIAL as u16 != 0) || !same_color(color, BLACK_COLOR)))
            {
                if same_color(color, WHITE_COLOR) {
                    (*prim).Type = gfx_object_STAMP_PRIM as u8;
                } else if let Some(next) = decloak_step(color) {
                    (*prim).color = next;
                } else {
                    // CLOAKING_OFF
                    ProcessSound(
                        SetAbsStringTableIndex((*race).ship_data.ship_sounds, 2),
                        ship,
                    );
                    (*prim).color = BLUE_DARK;
                    if weapon_discharge {
                        aim_flame_after_decloak(ship, ss);
                    }
                }

                (*ship).state_flags |= CHANGING as u16;
                status_flags &= !(SPECIAL as u16);
                (*ss).special_counter = 0;
            } else if !same_color(color, BLACK_COLOR) {
                if same_color(color, BLUE_DARK) {
                    (*prim).color = BLACK_COLOR;
                    Untarget(ship);
                } else if same_color(color, BLUE_LIGHT) {
                    (*prim).color = BLUE_DARK;
                } else if same_color(color, CYAN_DARK) {
                    (*prim).color = BLUE_LIGHT;
                } else if same_color(color, CYAN_LIGHT) {
                    (*prim).color = CYAN_DARK;
                } else {
                    (*prim).color = CYAN_LIGHT;
                }

                (*ship).state_flags |= CHANGING as u16;
            }
        }

        if (status_flags & SPECIAL as u16 != 0)
            && (*ss).special_counter == 0
            && DeltaEnergy(ship, -(SPECIAL_ENERGY_COST as SIZE)) != 0
        {
            (*prim).color = WHITE_COLOR;
            (*prim).Type = gfx_object_STAMPFILL_PRIM as u8;

            // CLOAKING_ON
            ProcessSound(
                SetAbsStringTableIndex((*race).ship_data.ship_sounds, 1),
                ship,
            );
            (*ss).special_counter = (*race).characteristics.special_wait;

            (*ship).state_flags |= CHANGING as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb15_expands_five_bit_channels_with_full_alpha() {
        let white = build_color(0x1F, 0x1F, 0x1F);
        assert_eq!((white.r, white.g, white.b, white.a), (255, 255, 255, 255));
        let cyan_light = build_color(0x0A, 0x1F, 0x1F);
        assert_eq!(
            (cyan_light.r, cyan_light.g, cyan_light.b, cyan_light.a),
            (82, 255, 255, 255)
        );
        let blue_dark = build_color(0x00, 0x00, 0x14);
        assert_eq!(
            (blue_dark.r, blue_dark.g, blue_dark.b, blue_dark.a),
            (0, 0, 165, 255)
        );
    }

    #[test]
    fn cloak_colors_are_distinct_and_black_is_opaque() {
        let steps = [WHITE_COLOR, CYAN_LIGHT, CYAN_DARK, BLUE_LIGHT, BLUE_DARK];
        for (i, a) in steps.iter().enumerate() {
            assert!(!same_color(*a, BLACK_COLOR));
            for b in &steps[i + 1..] {
                assert!(!same_color(*a, *b));
            }
        }
        assert_eq!(BLACK_COLOR.a, 255);
    }

    #[test]
    fn decloak_walks_from_dark_blue_up_to_white() {
        assert!(decloak_step(BLUE_DARK).is_some_and(|c| same_color(c, BLUE_LIGHT)));
        assert!(decloak_step(BLUE_LIGHT).is_some_and(|c| same_color(c, CYAN_DARK)));
        assert!(decloak_step(CYAN_DARK).is_some_and(|c| same_color(c, CYAN_LIGHT)));
        assert!(decloak_step(CYAN_LIGHT).is_some_and(|c| same_color(c, WHITE_COLOR)));
        assert!(decloak_step(BLACK_COLOR).is_none());
        assert!(decloak_step(WHITE_COLOR).is_none());
    }

    #[test]
    fn precedence_of_the_original_input_mask_clears_only_left() {
        let mask = (!(crate::ships::native::abi::LEFT as i32)
            | crate::ships::native::abi::RIGHT as i32) as u8;
        assert_eq!(mask, 0xFE);
    }

    #[test]
    fn angle_to_facing_rounds_to_nearest_facing() {
        assert_eq!(normalize_facing(angle_to_facing(2)), 1);
        assert_eq!(normalize_facing(angle_to_facing(63)), 0);
    }
}
