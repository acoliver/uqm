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
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceDesc, RaceTemplate, ShipId, StoryKey, WeaponOrigin,
        WeaponSpawn, World,
    };

    use crate::ships::native::abi::*;

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

    pub static ILWRATH: Race = Race {
        descriptor,
        preprocess: Some(ilwrath_preprocess),
        postprocess: None,
        init_weapon: Some(initialize_flame),
        intelligence: Some(ilwrath_intelligence),
        uninit: None,
    };

    fn descriptor(_world: &mut World) -> RaceDesc {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut d = RaceTemplate::default();
        d.info.ship_flags = FIRES_FORE as u16;
        d.info.ship_cost = 10;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.ilwrath.text");
        d.resources.icons = Some(c"ship.ilwrath.icons");
        d.resources.melee_icon = Some(c"ship.ilwrath.meleeicons");
        d.fleet_strength = 1410 / SPHERE_RADIUS_INCREMENT as u16 * 2;
        d.fleet_location = POINT { x: 48, y: 1700 };
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
        d.resources.ship = [
            Some(c"ship.ilwrath.graphics.avenger.large"),
            Some(c"ship.ilwrath.graphics.avenger.medium"),
            Some(c"ship.ilwrath.graphics.avenger.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.ilwrath.graphics.fire.large"),
            Some(c"ship.ilwrath.graphics.fire.medium"),
            Some(c"ship.ilwrath.graphics.fire.small"),
        ];
        d.resources.captain = Some(c"ship.ilwrath.graphics.captain");
        d.resources.victory = Some(c"ship.ilwrath.ditty");
        d.resources.sounds = Some(c"ship.ilwrath.sounds");
        d.intelligence.weapon_range = CLOSE_RANGE_WEAPON;
        d.descriptor(&ILWRATH)
    }

    fn object_cloaked(cx: &Cx<'_>, e: ElementId) -> bool {
        let prim = cx.primitive_style(e);
        prim.kind >= gfx_object_NUM_PRIMS as u8
            || (prim.kind == gfx_object_STAMPFILL_PRIM as u8 && same_color(prim.color, BLACK_COLOR))
    }

    pub(crate) fn flame_preprocess(cx: &mut Cx<'_>, flame: ElementId) {
        if cx.element(flame).turn > 0 {
            cx.element_mut(flame).turn -= 1;
        } else {
            cx.set_frame(
                flame,
                true,
                cx.advance_frame(cx.element(flame).current.image.frame),
            );
            cx.element_mut(flame).flags |= CHANGING as u16;

            cx.element_mut(flame).turn = cx.element(flame).thrust;
        }
    }

    pub(crate) fn flame_collision(
        cx: &mut Cx<'_>,
        flame: ElementId,
        flame_point: &mut POINT,
        other: ElementId,
        other_point: &mut POINT,
    ) {
        cx.weapon_collision(flame, flame_point, other, other_point);
        cx.element_mut(flame).flags &= !(DISAPPEARING as u16);
        cx.element_mut(flame).flags |= NONSOLID as u16;
    }

    pub(crate) fn ilwrath_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let enemy = ENEMY_SHIP_INDEX as usize;
        concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
        if concerns.objects[enemy].object.is_some() && concerns.objects[enemy].which_turn <= 10 {
            // don't want to dodge when you could be flaming
            concerns.objects[ENEMY_WEAPON_INDEX as usize].object = None;
        }

        cx.ship_intelligence(ss, concerns);

        let ship = cx.ship_element(ss);
        let enemy_weapon = ENEMY_WEAPON_INDEX as usize;
        if concerns.objects[enemy].object.is_some()
            && (concerns.objects[enemy].which_turn <= 6
                || (concerns.objects[enemy].which_turn <= 10
                    && concerns.objects[enemy_weapon].which_turn <= 10))
        {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            if object_cloaked(cx, ship) {
                // The original reads `&= ~LEFT | RIGHT`, which is
                // `&= (~LEFT) | RIGHT` and therefore clears only LEFT.
                cx.ship_state_mut(ss).input &= (!(LEFT as i32) | RIGHT as i32) as u8;
                cx.ship_state_mut(ss).input |= THRUST as u8;
            }
            cx.ship_state_mut(ss).input |= WEAPON as u8;
        } else if cx.ship_state(ss).special_counter == 0
            && (cx.world.activity() as u8 != IN_ENCOUNTER as u8
                || cx.story_state(StoryKey::ProbeIlwrathEncounter) == 0)
        {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            if !object_cloaked(cx, ship) && cx.ship_state(ss).input & WEAPON as u8 == 0 {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
        }
    }

    pub(crate) fn initialize_flame(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            flags: IGNORE_SIMILAR as u16,
            sender: origin.player,
            pixoffs: ILWRATH_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            face: cx.ship_state(ss).facing,
            index: 0,
            life: MISSILE_LIFE,
            farray: cx.ship_frame_set(ss, 1),
            preprocess: Some(flame_preprocess),
            blast_offs: MISSILE_OFFSET,
        };
        let missile = cx.missile(&mut block);
        if let Some(flame) = missile {
            let (vx, vy) = origin.velocity.get_current_components();
            let dx = vx as SIZE;
            let dy = vy as SIZE;
            cx.element_mut(flame)
                .velocity
                .delta_components(i32::from(dx), i32::from(dy));
            // VELOCITY_TO_WORLD (v) is `(v) >> VELOCITY_SHIFT` on a promoted int.
            cx.element_mut(flame).current.location.x =
                (i32::from(cx.element(flame).current.location.x)
                    - (i32::from(dx) >> VELOCITY_SHIFT)) as COORD;
            cx.element_mut(flame).current.location.y =
                (i32::from(cx.element(flame).current.location.y)
                    - (i32::from(dy) >> VELOCITY_SHIFT)) as COORD;

            cx.element_mut(flame).collide = Some(CollisionHook::Native(flame_collision));
            cx.element_mut(flame).turn = 0;
        }

        vec![missile]
    }

    /// The weapon-discharge half of the decloak step: turn the ship toward
    /// where the target will be `LOOK_AHEAD` frames from now.
    fn aim_flame_after_decloak(cx: &mut Cx<'_>, ship: ElementId, ss: ShipId) {
        let mut facing = cx.ship_state(ss).facing;
        if cx.track_ship(ship, &mut facing) >= 0 {
            let target = cx
                .target(ship)
                .unwrap_or_else(|| panic!("tracked Ilwrath target"));
            let mut v = cx.element(target).velocity;
            let (x, y) = v.get_next_components(LOOK_AHEAD);
            let mut dx0 = x as SIZE;
            let mut dy0 = y as SIZE;
            v = cx.element(ship).velocity;
            let (x, y) = v.get_next_components(LOOK_AHEAD);
            let dx1 = x as SIZE;
            let dy1 = y as SIZE;
            dx0 = ((i32::from(cx.element(target).current.location.x) + i32::from(dx0))
                - (i32::from(cx.element(ship).current.location.x) + i32::from(dx1)))
                as SIZE;
            dy0 = ((i32::from(cx.element(target).current.location.y) + i32::from(dy0))
                - (i32::from(cx.element(ship).current.location.y) + i32::from(dy1)))
                as SIZE;

            cx.ship_state_mut(ss).facing =
                normalize_facing(angle_to_facing(i32::from(crate::math::arctan(dx0, dy0))));
            if cx.element(ship).turn == 0 {
                cx.element_mut(ship).turn += 1;
            }
            cx.set_frame(
                ship,
                true,
                cx.absolute_frame(cx.element(ship).next.image.frame, cx.ship_state(ss).facing),
            );
        }
        cx.set_target(ship, None);
    }

    pub(crate) fn ilwrath_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        // Local copy: clearing SPECIAL below must not touch cur_status_flags.
        let mut status_flags: STATUS_FLAGS = cx.ship_state(ss).status;
        let prim = cx.primitive_style(ship);
        if prim.kind == gfx_object_STAMPFILL_PRIM as u8 {
            let color = prim.color;
            let weapon_discharge = (status_flags & WEAPON as u16 != 0)
                && cx.race_stats(ss).ship_info.energy_level >= WEAPON_ENERGY_COST;
            if weapon_discharge
                || (cx.ship_state(ss).special_counter == 0
                    && ((status_flags & SPECIAL as u16 != 0) || !same_color(color, BLACK_COLOR)))
            {
                if same_color(color, WHITE_COLOR) {
                    cx.primitive_style_mut(ship).kind = gfx_object_STAMP_PRIM as u8;
                } else if let Some(next) = decloak_step(color) {
                    cx.primitive_style_mut(ship).color = next;
                } else {
                    // CLOAKING_OFF
                    cx.process_ship_sound(ss, 2, ship);
                    cx.primitive_style_mut(ship).color = BLUE_DARK;
                    if weapon_discharge {
                        aim_flame_after_decloak(cx, ship, ss);
                    }
                }

                cx.element_mut(ship).flags |= CHANGING as u16;
                status_flags &= !(SPECIAL as u16);
                cx.ship_state_mut(ss).special_counter = 0;
            } else if !same_color(color, BLACK_COLOR) {
                if same_color(color, BLUE_DARK) {
                    cx.primitive_style_mut(ship).color = BLACK_COLOR;
                    cx.untarget(ship);
                } else if same_color(color, BLUE_LIGHT) {
                    cx.primitive_style_mut(ship).color = BLUE_DARK;
                } else if same_color(color, CYAN_DARK) {
                    cx.primitive_style_mut(ship).color = BLUE_LIGHT;
                } else if same_color(color, CYAN_LIGHT) {
                    cx.primitive_style_mut(ship).color = CYAN_DARK;
                } else {
                    cx.primitive_style_mut(ship).color = CYAN_LIGHT;
                }

                cx.element_mut(ship).flags |= CHANGING as u16;
            }
        }

        if (status_flags & SPECIAL as u16 != 0)
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(ship, -(SPECIAL_ENERGY_COST as SIZE))
        {
            cx.primitive_style_mut(ship).color = WHITE_COLOR;
            cx.primitive_style_mut(ship).kind = gfx_object_STAMPFILL_PRIM as u8;

            // CLOAKING_ON
            cx.process_ship_sound(ss, 1, ship);
            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;

            cx.element_mut(ship).flags |= CHANGING as u16;
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
