//! Androsynth behavior, retaining the original integer widths and RNG order.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceState, ShipId, WeaponOrigin, WeaponSpawn,
    };
    use crate::ships::native::abi::*;
    // Core characteristics
    pub(crate) const MAX_ENERGY: u8 = 24;
    pub(crate) const ENERGY_REGENERATION: u8 = 1;
    pub(crate) const ENERGY_WAIT: u8 = 8;
    pub(crate) const SHIP_MASS: u8 = 6;

    // Bubbles
    const ANDROSYNTH_OFFSET: i16 = 14;
    const MISSILE_OFFSET: i16 = 3;
    /// DISPLAY_TO_WORLD (8)
    const MISSILE_SPEED: i16 = 8 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 200;
    const MISSILE_HITS: i16 = 3;
    const MISSILE_DAMAGE: i16 = 2;
    const TRACK_WAIT: u8 = 2;

    // Blazer
    pub(crate) const SPECIAL_ENERGY_COST: u8 = 2;
    const BLAZER_DEGENERATION: i16 = -1;
    const BLAZER_OFFSET: u8 = 10;
    const BLAZER_THRUST: i16 = 60;
    const BLAZER_TURN_WAIT: u8 = 1;
    const BLAZER_DAMAGE: u8 = 3;
    const BLAZER_MASS: u8 = 1;

    // intel.h: DISPLAY_TO_WORLD (50) and DISPLAY_TO_WORLD (1000). They are
    // function-like macros, so bindgen does not emit them.
    const CLOSE_RANGE_WEAPON: i32 = 50 << ONE_SHIFT;
    /// races.h: ((COUNT) ~0)
    const FULL_CIRCLE_ANGLE: i32 = FULL_CIRCLE as i32;
    const HALF_CIRCLE_ANGLE: i32 = HALF_CIRCLE as i32;
    const QUADRANT_ANGLE: i32 = QUADRANT as i32;
    const OCTANT_ANGLE: i32 = OCTANT as i32;
    const FACING_MASK: i32 = (1 << FACING_SHIFT) - 1;

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

    fn build_descriptor(
        _world: &mut crate::engine::battle::World,
    ) -> crate::engine::battle::RaceDesc {
        crate::engine::battle::androsynth_template::descriptor(&ANDROSYNTH)
    }

    pub static ANDROSYNTH: Race = Race {
        descriptor: build_descriptor,
        preprocess: Some(androsynth_preprocess),
        postprocess: Some(androsynth_postprocess),
        init_weapon: Some(initialize_bubble),
        intelligence: Some(androsynth_intelligence),
        uninit: Some(uninit_androsynth),
    };

    pub(crate) fn blazer_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        let old_crew_level = cx.element(a).hit_points();
        let old_life = cx.element(a).life_span();
        let old_offs = cx.element(a).thrust_wait();
        cx.element_mut(a).set_thrust_wait(BLAZER_OFFSET);
        cx.element_mut(a).set_mass_points(BLAZER_DAMAGE);
        cx.weapon_collision(a, pa, b, pb);
        cx.element_mut(a).set_mass_points(BLAZER_MASS);
        cx.element_mut(a).set_thrust_wait(old_offs);
        cx.element_mut(a).set_life_span(old_life);
        cx.element_mut(a).set_hit_points(old_crew_level);
        cx.element_mut(a).flags &= !((DISAPPEARING | NONSOLID) as u16);
        crate::battle::ship_native::collision(cx, a, pa, b, pb);
    }

    pub(crate) fn bubble_preprocess(cx: &mut Cx<'_>, missile: ElementId) {
        let packed = cx.element(missile).turn_wait();
        let mut thrust_wait = packed >> 4;
        let mut turn_wait = packed & 0x0f;
        if thrust_wait > 0 {
            thrust_wait -= 1;
        } else {
            let frame = cx.advance_frame(cx.element(missile).current.image.frame);
            cx.set_frame(missile, true, frame);
            cx.element_mut(missile).flags |= CHANGING as u16;
            thrust_wait = (cx.random() as u16 & 3) as u8;
        }
        if turn_wait > 0 {
            turn_wait -= 1;
        } else {
            let mut facing = (angle_to_facing(i32::from(cx.element(missile).velocity.travel_angle))
                & FACING_MASK) as u16;
            let delta_facing = cx.track_ship(missile, &mut facing);
            if delta_facing == -1 {
                facing = cx.random() as u16;
            } else if i32::from(delta_facing) <= angle_to_facing(HALF_CIRCLE_ANGLE) {
                facing = facing.wrapping_add(
                    cx.random() as u16 & (angle_to_facing(HALF_CIRCLE_ANGLE) - 1) as u16,
                );
            } else {
                facing = facing.wrapping_sub(
                    cx.random() as u16 & (angle_to_facing(HALF_CIRCLE_ANGLE) - 1) as u16,
                );
            }
            cx.element_mut(missile)
                .velocity
                .set_vector(i32::from(MISSILE_SPEED), facing);
            turn_wait = TRACK_WAIT;
        }
        cx.element_mut(missile)
            .set_turn_wait(make_byte(turn_wait, thrust_wait));
    }

    pub(crate) fn initialize_bubble(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = crate::engine::battle::views::Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: 0,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: ANDROSYNTH_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess: Some(bubble_preprocess),
            blast_offs: MISSILE_OFFSET,
        };
        let bubble = cx.missile(&mut block);
        if let Some(id) = bubble {
            cx.element_mut(id).set_turn_wait(0);
        }
        vec![bubble]
    }

    fn blazer_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let weapon = ENEMY_WEAPON_INDEX as usize;

        concerns.objects[CREW_OBJECT_INDEX as usize].object = None;
        if let Some(threat) = concerns.objects[weapon].object {
            if concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE {
                let flags = cx.element(threat).flags;
                if flags & FINITE_LIFE as u16 != 0 && flags & CREW_OBJECT as u16 == 0 {
                    concerns.objects[weapon].movement = MOVEMENT_STATE_AVOID;
                } else {
                    concerns.objects[weapon].object = None;
                }
            }
        }
        cx.ship_intelligence(ss, concerns);
    }

    pub(crate) fn androsynth_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let weapon = ENEMY_WEAPON_INDEX as usize;
        let enemy = ENEMY_SHIP_INDEX as usize;
        if concerns.subject.next.image.frames == cx.ship_frame_set(ss, 2) {
            blazer_intelligence(cx, ss, concerns);
            return;
        }
        if let Some(target) = concerns.objects[enemy].object {
            let other = cx.owner(target);
            let intel = cx.race_stats(other).intelligence;
            let crew = i32::from(cx.element(target).hit_points());
            if concerns.objects[enemy].which_turn <= 16
                && (cx.ship_state(ss).special_counter > 0
                    || i32::from(cx.race_stats(ss).ship_info.energy_level)
                        < i32::from(MAX_ENERGY) / 3
                    || ((i32::from(intel.weapon_range) <= CLOSE_RANGE_WEAPON
                        && crew > i32::from(BLAZER_DAMAGE))
                        || (crew > i32::from(BLAZER_DAMAGE) * 3
                            && u32::from(intel.maneuverability) > SLOW_SHIP)))
            {
                concerns.objects[enemy].movement = MOVEMENT_STATE_ENTICE;
            }
        }
        cx.ship_intelligence(ss, concerns);
        if cx.ship_state(ss).special_counter == 0 {
            cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            let danger = concerns.objects[weapon].object.is_some()
                && concerns.objects[weapon].which_turn <= 4;
            let lure = concerns.objects[enemy].object.is_some_and(|target| {
                if i32::from(cx.race_stats(ss).ship_info.energy_level) < i32::from(MAX_ENERGY) / 3 {
                    return false;
                }
                let other = cx.owner(target);
                let intel = cx.race_stats(other).intelligence;
                let crew = i32::from(cx.element(target).hit_points());
                i32::from(intel.weapon_range)
                    >= i32::from(cx.race_stats(ss).intelligence.weapon_range) << 1
                    || (concerns.objects[enemy].which_turn < 16
                        && (i32::from(intel.weapon_range) > CLOSE_RANGE_WEAPON
                            || crew <= i32::from(BLAZER_DAMAGE))
                        && (crew <= i32::from(BLAZER_DAMAGE) * 3
                            || u32::from(intel.maneuverability) <= SLOW_SHIP))
            });
            if danger || lure {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
        }
        if cx.ship_state(ss).input & SPECIAL as u8 == 0 && cx.ship_state(ss).weapon_counter == 0 {
            if let Some(target) = concerns.objects[enemy].object {
                if concerns.objects[enemy].which_turn <= 4 {
                    cx.ship_state_mut(ss).input |= WEAPON as u8;
                } else if concerns.objects[enemy].movement != MOVEMENT_STATE_PURSUE
                    && concerns.objects[enemy].which_turn <= 12
                {
                    let (dx, dy) = concerns.subject.velocity.get_current_components();
                    let (other_dx, other_dy) = cx.element(target).velocity.get_current_components();
                    let travel =
                        crate::math::arctan((dx - other_dx) as SIZE, (dy - other_dy) as SIZE);
                    let direction = crate::math::arctan(
                        (i32::from(cx.element(target).next.location.x)
                            - i32::from(concerns.subject.next.location.x))
                            as SIZE,
                        (i32::from(cx.element(target).next.location.y)
                            - i32::from(concerns.subject.next.location.y))
                            as SIZE,
                    );
                    if normalize_angle(i32::from(travel) - i32::from(direction) + OCTANT_ANGLE)
                        <= QUADRANT_ANGLE
                    {
                        cx.ship_state_mut(ss).input |= WEAPON as u8;
                    }
                }
            }
        }
    }

    pub(crate) fn androsynth_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.element(ship).next.image.frames == cx.ship_frame_set(ss, 2) {
            if cx.ship_state(ss).status & SPECIAL as u16 != 0
                || cx.race_stats(ss).ship_info.energy_level == 0
            {
                cx.race_stats_mut(ss).characteristics.energy_regeneration =
                    BLAZER_DEGENERATION as u8;
                cx.ship_state_mut(ss).energy_counter = ENERGY_WAIT;
                if cx.ship_state(ss).status & SPECIAL as u16 != 0 {
                    cx.process_ship_sound(ss, 1, ship);
                    cx.element_mut(ship).set_turn_wait(0);
                    cx.element_mut(ship).set_thrust_wait(0);
                    cx.race_stats_mut(ss).characteristics.special_wait =
                        cx.race_stats(ss).characteristics.turn_wait;
                    cx.element_mut(ship).set_mass_points(BLAZER_MASS);
                    cx.race_stats_mut(ss).characteristics.turn_wait = BLAZER_TURN_WAIT;
                    cx.race_stats_mut(ss).state = RaceState::Androsynth {
                        collision: cx.element(ship).collide,
                    };
                    cx.element_mut(ship).collide = Some(CollisionHook::Native(blazer_collision));
                }
            }
            if cx.race_stats(ss).ship_info.energy_level == 0 {
                cx.delta_energy(ship, -BLAZER_DEGENERATION);
                cx.ship_state_mut(ss).energy_counter = 1;
            }
        }
    }

    pub(crate) fn androsynth_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        let mut flags = cx.ship_state(ss).status;
        if cx.element(ship).next.image.frames == cx.ship_frame_set(ss, 0) {
            if flags & SPECIAL as u16 != 0 {
                if cx.race_stats(ss).ship_info.energy_level < SPECIAL_ENERGY_COST {
                    cx.delta_energy(ship, -i16::from(SPECIAL_ENERGY_COST));
                } else {
                    flags &= !(WEAPON as u16);
                    let frames = cx.ship_frame_set(ss, 2);
                    cx.set_frames(ship, true, frames);
                    let frame = cx
                        .equivalent_frame(cx.ship_frame(ss, 2), cx.element(ship).next.image.frame);
                    cx.set_frame(ship, true, frame);
                    cx.element_mut(ship).flags |= CHANGING as u16;
                }
            }
        } else {
            flags &= !((THRUST | WEAPON | SPECIAL) as u16);
            if cx.race_stats(ss).characteristics.turn_wait > BLAZER_TURN_WAIT {
                cx.race_stats_mut(ss).characteristics.special_wait =
                    cx.race_stats(ss).characteristics.special_wait.wrapping_add(
                        cx.race_stats(ss).characteristics.turn_wait - BLAZER_TURN_WAIT,
                    );
                cx.race_stats_mut(ss).characteristics.turn_wait = BLAZER_TURN_WAIT;
            }
            if cx.race_stats(ss).ship_info.energy_level == 0 {
                cx.element_mut(ship).velocity = crate::battle::velocity::VelocityDesc::new();
                flags &= !((LEFT | RIGHT | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                cx.race_stats_mut(ss).characteristics.turn_wait =
                    cx.race_stats(ss).characteristics.special_wait;
                cx.race_stats_mut(ss).characteristics.energy_regeneration = ENERGY_REGENERATION;
                cx.element_mut(ship).set_mass_points(SHIP_MASS);
                let RaceState::Androsynth { collision } = cx.race_stats(ss).state else {
                    panic!("blazer has no saved collision hook")
                };
                cx.element_mut(ship).collide = collision;
                let frames = cx.ship_frame_set(ss, 0);
                cx.set_frames(ship, true, frames);
                let frame =
                    cx.equivalent_frame(cx.ship_frame(ss, 0), cx.element(ship).next.image.frame);
                cx.set_frame(ship, true, frame);
                cx.element_mut(ship).flags |= CHANGING as u16;
            } else if cx.element(ship).thrust_wait() != 0 {
                let wait = cx.element(ship).thrust_wait() - 1;
                cx.element_mut(ship).set_thrust_wait(wait);
            } else {
                let mut facing = cx.ship_state(ss).facing;
                if cx.element(ship).turn_wait() == 0 && flags & (LEFT | RIGHT) as u16 != 0 {
                    facing = if flags & LEFT as u16 != 0 {
                        facing.wrapping_sub(1)
                    } else {
                        facing.wrapping_add(1)
                    };
                }
                cx.element_mut(ship)
                    .velocity
                    .set_vector(i32::from(BLAZER_THRUST), facing & FACING_MASK as u16);
                flags |= (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            }
        }
        cx.ship_state_mut(ss).status = flags;
    }

    pub(crate) fn uninit_androsynth(
        _world: &mut crate::engine::battle::World,
        stats: &mut crate::engine::battle::views::RaceStats,
    ) {
        stats.state = RaceState::None;
    }
}
