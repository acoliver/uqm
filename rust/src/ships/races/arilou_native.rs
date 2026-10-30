//! Arilou Skiff callbacks and descriptor for the live engine.
//!
//! Ported from `sc2/src/uqm/ships/arilou/arilou.c`. The teleport placement
//! macros (`DISPLAY_ALIGN_X/Y`, `WRAP_X/Y`, `LOG_SPACE_*`) depend on the
//! live screen size, so they are translated here with the same `int`/COUNT
//! /COORD conversions the C macros perform.

#[cfg(test)]
use crate::ships::native::abi::SIN_SHIFT;
use crate::ships::native::abi::{CIRCLE_SHIFT, FACING_SHIFT, ONE_SHIFT};

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
#[cfg(test)]
const fn normalize_angle(angle: i32) -> usize {
    (angle & ((1 << CIRCLE_SHIFT) - 1)) as usize
}

/// The arithmetic of `SINE (a, m)` once `SINVAL (a)` is known: the product
/// is taken in `long` and the shifted result truncated to SIZE.
#[cfg(test)]
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
    use crate::engine::battle::{Concerns, Cx, ShipId, WeaponOrigin, WeaponSpawn};
    use crate::ships::native::abi::*;

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

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let d = crate::engine::battle::template::RaceTemplate {
            info: crate::engine::battle::views::Info {
                ship_flags: IMMEDIATE_WEAPON as u16,
                ship_cost: 16,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
            },
            resources: crate::engine::battle::template::RaceResources {
                strings: Some(c"ship.arilou.text"),
                icons: Some(c"ship.arilou.icons"),
                melee_icon: Some(c"ship.arilou.meleeicons"),
                ship: [
                    Some(c"ship.arilou.graphics.skiff.large"),
                    Some(c"ship.arilou.graphics.skiff.medium"),
                    Some(c"ship.arilou.graphics.skiff.small"),
                ],
                special: [
                    Some(c"ship.arilou.graphics.warp.large"),
                    Some(c"ship.arilou.graphics.warp.medium"),
                    Some(c"ship.arilou.graphics.warp.small"),
                ],
                captain: Some(c"ship.arilou.graphics.captain"),
                victory: Some(c"ship.arilou.ditty"),
                sounds: Some(c"ship.arilou.sounds"),
                ..Default::default()
            },
            fleet_strength: 250 / SPHERE_RADIUS_INCREMENT as u16 * 2,
            fleet_location: POINT { x: 438, y: 6372 },
            characteristics: CHARACTERISTIC_STUFF {
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
            },
            intelligence: crate::engine::battle::views::Intelligence {
                weapon_range: (LASER_RANGE >> 1) as COUNT,
                ..Default::default()
            },
            ..Default::default()
        };
        d.descriptor(&ARILOU)
    }

    pub static ARILOU: crate::engine::battle::Race = crate::engine::battle::Race {
        descriptor,
        preprocess: Some(arilou_preprocess),
        postprocess: None,
        init_weapon: Some(initialize_autoaim_laser),
        intelligence: Some(arilou_intelligence),
        uninit: None,
    };

    pub(crate) fn initialize_autoaim_laser(
        cx: &mut Cx<'_>,
        ss: ShipId,
        mut origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let orig_facing = cx.ship_state(ss).facing;
        let mut face = orig_facing;
        let delta = cx.track_subject(&mut origin, &mut face);
        if delta > 0 {
            face = normalize_facing(i32::from(orig_facing) + i32::from(delta));
        }
        origin.target = None;
        let angle = facing_to_angle(i32::from(face));
        vec![cx.spawn_laser(crate::engine::battle::views::Laser {
            origin: origin.next.location,
            end: POINT {
                x: crate::ships::native::cosine(angle, LASER_RANGE),
                y: crate::ships::native::sine(angle, LASER_RANGE),
            },
            flags: IGNORE_SIMILAR as u16,
            sender: origin.player,
            pixoffs: ARILOU_OFFSET,
            face,
            color: Color {
                r: 0xff,
                g: 0xff,
                b: 0x52,
                a: 0xff,
            },
        })]
    }

    pub(crate) fn arilou_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.ship_state_mut(ss).input |= THRUST as u8;
        concerns.objects[ENEMY_SHIP_INDEX as usize].movement = MOVEMENT_STATE_ENTICE;
        cx.ship_intelligence(ss, concerns);
        if cx.ship_state(ss).special_counter == 0 {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            let eval = concerns.objects[ENEMY_WEAPON_INDEX as usize];
            if let Some(threat) = eval.object.filter(|_| eval.which_turn <= 6) {
                let owner = cx.owner(threat);
                let flags = cx.race_stats(owner).ship_info.ship_flags;
                let object = cx.element(threat);
                let tracking = (flags & SEEKING_WEAPON as u16 != 0
                    && object.next.image.frames == cx.ship_frame_set(owner, 1))
                    || (flags & SEEKING_SPECIAL as u16 != 0
                        && object.next.image.frames == cx.ship_frame_set(owner, 2));
                if (object.flags & PLAYER_SHIP as u16 != 0
                    || (tracking
                        && (eval.which_turn == 1 || object.flags & CREW_OBJECT as u16 != 0))
                    || cx.plot_intercept(&cx.weapon_origin(threat), &concerns.subject, 3, 0) != 0)
                    && cx.random() & 3 == 0
                {
                    cx.ship_state_mut(ss).input &= !((LEFT | RIGHT | THRUST | WEAPON) as u8);
                    cx.ship_state_mut(ss).input |= SPECIAL as u8;
                }
            }
        }
        if u16::from(cx.race_stats(ss).ship_info.energy_level)
            <= u16::from(SPECIAL_ENERGY_COST) << 1
        {
            cx.ship_state_mut(ss).input &= !(WEAPON as u8);
        }
    }

    pub(crate) fn arilou_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        let special = cx.ship_frame_set(ss, 2);
        if cx.element(ship).flags & NONSOLID as u16 == 0 {
            if cx.element(ship).thrust == 0 {
                cx.element_mut(ship).velocity = VELOCITY_DESC::default();
                cx.ship_state_mut(ss).status &= !(SHIP_AT_MAX_SPEED as u16);
            }
            if cx.ship_state(ss).status & SPECIAL as u16 != 0
                && cx.ship_state(ss).special_counter == 0
                && cx.delta_energy(ship, -(SPECIAL_ENERGY_COST as SIZE))
            {
                cx.element_mut(ship).velocity = VELOCITY_DESC::default();
                cx.ship_state_mut(ss).status &=
                    !((SHIP_AT_MAX_SPEED | LEFT | RIGHT | THRUST | WEAPON) as u16);
                cx.element_mut(ship).flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
                cx.element_mut(ship).life = HYPER_LIFE;
                cx.set_frames(ship, true, special);
                cx.set_frame(ship, true, cx.ship_frame(ss, 2));
                cx.process_ship_sound(ss, 1, ship);
                cx.ship_state_mut(ss).special_counter =
                    cx.race_stats(ss).characteristics.special_wait;
            }
        } else if cx.element(ship).next.image.frames == special {
            const INPUT_KEYS: u16 = (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;
            {
                let mut state = cx.ship_state_mut(ss);
                state.status = (state.status & !INPUT_KEYS) | (state.old_status & INPUT_KEYS);
                state.weapon_counter = state.weapon_counter.wrapping_add(1);
                state.special_counter = state.special_counter.wrapping_add(1);
                state.energy_counter = state.energy_counter.wrapping_add(1);
            }
            {
                let mut state = cx.element_mut(ship);
                state.turn = state.turn.wrapping_add(1);
                state.thrust = state.thrust.wrapping_add(1);
            }
            let mut life = cx.element(ship).life;
            if life == NORMAL_LIFE as COUNT {
                cx.element_mut(ship).flags &= !((NONSOLID | FINITE_LIFE) as u16);
                cx.element_mut(ship).flags |= APPEARING as u16;
                let frames = cx.ship_frame_set(ss, 0);
                cx.set_frames(ship, false, frames);
                cx.set_frames(ship, true, frames);
                let frame = cx.absolute_frame(cx.ship_frame(ss, 0), cx.ship_state(ss).facing);
                cx.set_frame(ship, false, frame);
                cx.set_frame(ship, true, frame);
                cx.init_intersect_start(ship);
            } else {
                life = life.wrapping_sub(1);
                if life != 2 {
                    let frame = cx.element(ship).next.image.frame;
                    let frame = if life < 2 {
                        cx.retreat_frame(frame)
                    } else {
                        cx.advance_frame(frame)
                    };
                    cx.set_frame(ship, true, frame);
                } else {
                    let (width, height) = cx.screen_dimensions();
                    let width = log_space_width(width);
                    let height = log_space_height(height);
                    cx.element_mut(ship).next.location.x =
                        wrap_val(display_align(cx.random(), width), width) as COORD;
                    cx.element_mut(ship).next.location.y =
                        wrap_val(display_align(cx.random(), height), height) as COORD;
                }
            }
            cx.element_mut(ship).flags |= CHANGING as u16;
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
