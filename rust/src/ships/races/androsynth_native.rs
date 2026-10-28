//! Androsynth behavior, retaining the original integer widths and RNG order.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceDesc, RaceState, ShipId, WeaponOrigin, WeaponSpawn,
    };
    use crate::ships::native::abi::*;
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

    pub fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut desc = RACE_DESC::EMPTY;
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
        desc.race = Some(&ANDROSYNTH);
        desc
    }

    pub static ANDROSYNTH: Race = Race {
        descriptor,
        preprocess: Some(androsynth_preprocess),
        postprocess: Some(androsynth_postprocess),
        init_weapon: initialize_bubble,
        intelligence: androsynth_intelligence,
        uninit: Some(uninit_androsynth),
    };

    pub(crate) fn blazer_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        let old_crew_level = cx.b.elements[a].hit_points();
        let old_life = cx.b.elements[a].life_span();
        let old_offs = cx.b.elements[a].thrust_wait();
        cx.b.elements[a].set_thrust_wait(BLAZER_OFFSET);
        cx.b.elements[a].set_mass_points(BLAZER_DAMAGE);
        cx.weapon_collision(a, pa, b, pb);
        cx.b.elements[a].set_mass_points(BLAZER_MASS);
        cx.b.elements[a].set_thrust_wait(old_offs);
        cx.b.elements[a].set_life_span(old_life);
        cx.b.elements[a].set_hit_points(old_crew_level);
        cx.b.elements[a].state_flags &= !((DISAPPEARING | NONSOLID) as u16);
        crate::battle::ship_native::collision(cx, a, pa, b, pb);
    }

    pub(crate) fn bubble_preprocess(cx: &mut Cx<'_>, missile: ElementId) {
        let packed = cx.b.elements[missile].turn_wait();
        let mut thrust_wait = packed >> 4;
        let mut turn_wait = packed & 0x0f;
        if thrust_wait > 0 {
            thrust_wait -= 1;
        } else {
            let frame = cx.inc_frame(cx.b.elements[missile].current.image.frame);
            image!(frame; cx.b.elements[missile].next.image.frame, frame);
            cx.b.elements[missile].state_flags |= CHANGING as u16;
            thrust_wait = ((cx.rng)() as u16 & 3) as u8;
        }
        if turn_wait > 0 {
            turn_wait -= 1;
        } else {
            let mut facing =
                (angle_to_facing(i32::from(cx.b.elements[missile].velocity.travel_angle))
                    & FACING_MASK) as u16;
            let delta_facing = cx.track_ship(missile, &mut facing);
            if delta_facing == -1 {
                facing = (cx.rng)() as u16;
            } else if i32::from(delta_facing) <= angle_to_facing(HALF_CIRCLE_ANGLE) {
                facing = facing.wrapping_add(
                    (cx.rng)() as u16 & (angle_to_facing(HALF_CIRCLE_ANGLE) - 1) as u16,
                );
            } else {
                facing = facing.wrapping_sub(
                    (cx.rng)() as u16 & (angle_to_facing(HALF_CIRCLE_ANGLE) - 1) as u16,
                );
            }
            cx.b.elements[missile]
                .velocity
                .set_vector(i32::from(MISSILE_SPEED), facing);
            turn_wait = TRACK_WAIT;
        }
        cx.b.elements[missile].set_turn_wait(make_byte(turn_wait, thrust_wait));
    }

    pub(crate) fn initialize_bubble(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = MISSILE_BLOCK {
            cx: origin.location.x,
            cy: origin.location.y,
            farray: cx.ship_frames(ss, 1),
            face: cx.ship(ss).ShipFacing,
            index: 0,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: ANDROSYNTH_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess_func: Some(ElementHook::Native(bubble_preprocess)),
            blast_offs: MISSILE_OFFSET,
        };
        let bubble = cx.missile(&mut block);
        if let Some(id) = bubble {
            cx.b.elements[id].set_turn_wait(0);
        }
        vec![bubble]
    }

    pub(crate) fn androsynth_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let weapon = ENEMY_WEAPON_INDEX as usize;
        let enemy = ENEMY_SHIP_INDEX as usize;
        if concerns.subject.next.image.farray == cx.ship_frames(ss, 2) {
            concerns.objects[CREW_OBJECT_INDEX as usize].object = None;
            if let Some(threat) = concerns.objects[weapon].object {
                if concerns.objects[weapon].movement == MOVEMENT_STATE_ENTICE {
                    let flags = cx.b.elements[threat].state_flags;
                    if flags & FINITE_LIFE as u16 != 0 && flags & CREW_OBJECT as u16 == 0 {
                        concerns.objects[weapon].movement = MOVEMENT_STATE_AVOID;
                    } else {
                        concerns.objects[weapon].object = None;
                    }
                }
            }
            cx.ship_intelligence(ss, concerns);
            return;
        }
        if let Some(target) = concerns.objects[enemy].object {
            let other = cx.owner(target);
            let intel = cx.desc(other).cyborg_control;
            let crew = i32::from(cx.b.elements[target].hit_points());
            if concerns.objects[enemy].which_turn <= 16
                && (cx.ship(ss).special_counter > 0
                    || i32::from(cx.desc(ss).ship_info.energy_level) < i32::from(MAX_ENERGY) / 3
                    || ((i32::from(intel.WeaponRange) <= CLOSE_RANGE_WEAPON
                        && crew > i32::from(BLAZER_DAMAGE))
                        || (crew > i32::from(BLAZER_DAMAGE) * 3
                            && u32::from(intel.ManeuverabilityIndex) > SLOW_SHIP)))
            {
                concerns.objects[enemy].movement = MOVEMENT_STATE_ENTICE;
            }
        }
        cx.ship_intelligence(ss, concerns);
        if cx.ship(ss).special_counter == 0 {
            cx.ship_mut(ss).ship_input_state &= !(SPECIAL as u8);
            let danger = concerns.objects[weapon].object.is_some()
                && concerns.objects[weapon].which_turn <= 4;
            let lure = concerns.objects[enemy].object.is_some_and(|target| {
                if i32::from(cx.desc(ss).ship_info.energy_level) < i32::from(MAX_ENERGY) / 3 {
                    return false;
                }
                let other = cx.owner(target);
                let intel = cx.desc(other).cyborg_control;
                let crew = i32::from(cx.b.elements[target].hit_points());
                i32::from(intel.WeaponRange)
                    >= i32::from(cx.desc(ss).cyborg_control.WeaponRange) << 1
                    || (concerns.objects[enemy].which_turn < 16
                        && (i32::from(intel.WeaponRange) > CLOSE_RANGE_WEAPON
                            || crew <= i32::from(BLAZER_DAMAGE))
                        && (crew <= i32::from(BLAZER_DAMAGE) * 3
                            || u32::from(intel.ManeuverabilityIndex) <= SLOW_SHIP))
            });
            if danger || lure {
                cx.ship_mut(ss).ship_input_state |= SPECIAL as u8;
            }
        }
        if cx.ship(ss).ship_input_state & SPECIAL as u8 == 0 && cx.ship(ss).weapon_counter == 0 {
            if let Some(target) = concerns.objects[enemy].object {
                if concerns.objects[enemy].which_turn <= 4 {
                    cx.ship_mut(ss).ship_input_state |= WEAPON as u8;
                } else if concerns.objects[enemy].movement != MOVEMENT_STATE_PURSUE
                    && concerns.objects[enemy].which_turn <= 12
                {
                    let (dx, dy) = concerns.subject.velocity.get_current_components();
                    let (other_dx, other_dy) =
                        cx.b.elements[target].velocity.get_current_components();
                    let travel =
                        crate::math::arctan((dx - other_dx) as SIZE, (dy - other_dy) as SIZE);
                    let direction = crate::math::arctan(
                        (i32::from(cx.b.elements[target].next.location.x)
                            - i32::from(concerns.subject.next.location.x))
                            as SIZE,
                        (i32::from(cx.b.elements[target].next.location.y)
                            - i32::from(concerns.subject.next.location.y))
                            as SIZE,
                    );
                    if normalize_angle(i32::from(travel) - i32::from(direction) + OCTANT_ANGLE)
                        <= QUADRANT_ANGLE
                    {
                        cx.ship_mut(ss).ship_input_state |= WEAPON as u8;
                    }
                }
            }
        }
    }

    pub(crate) fn androsynth_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.b.elements[ship].next.image.farray == cx.ship_frames(ss, 2) {
            if cx.ship(ss).cur_status_flags & SPECIAL as u16 != 0
                || cx.desc(ss).ship_info.energy_level == 0
            {
                cx.desc_mut(ss).characteristics.energy_regeneration = BLAZER_DEGENERATION as u8;
                cx.ship_mut(ss).energy_counter = ENERGY_WAIT;
                if cx.ship(ss).cur_status_flags & SPECIAL as u16 != 0 {
                    cx.process_ship_sound(ss, 1, ship);
                    cx.b.elements[ship].set_turn_wait(0);
                    cx.b.elements[ship].set_thrust_wait(0);
                    cx.desc_mut(ss).characteristics.special_wait =
                        cx.desc(ss).characteristics.turn_wait;
                    cx.b.elements[ship].set_mass_points(BLAZER_MASS);
                    cx.desc_mut(ss).characteristics.turn_wait = BLAZER_TURN_WAIT;
                    cx.desc_mut(ss).race_state = RaceState::Androsynth {
                        collision: cx.b.elements[ship].collision_func,
                    };
                    cx.b.elements[ship].collision_func =
                        Some(CollisionHook::Native(blazer_collision));
                }
            }
            if cx.desc(ss).ship_info.energy_level == 0 {
                cx.delta_energy(ship, -BLAZER_DEGENERATION);
                cx.ship_mut(ss).energy_counter = 1;
            }
        }
    }

    pub(crate) fn androsynth_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        let mut flags = cx.ship(ss).cur_status_flags;
        if cx.b.elements[ship].next.image.farray == cx.ship_frames(ss, 0) {
            if flags & SPECIAL as u16 != 0 {
                if cx.desc(ss).ship_info.energy_level < SPECIAL_ENERGY_COST {
                    cx.delta_energy(ship, -i16::from(SPECIAL_ENERGY_COST));
                } else {
                    flags &= !(WEAPON as u16);
                    let frames = cx.ship_frames(ss, 2);
                    image!(set; cx.b.elements[ship].next.image.farray, frames);
                    let frame = cx.equ_frame(
                        cx.desc(ss).ship_data.special[0],
                        cx.b.elements[ship].next.image.frame,
                    );
                    image!(frame; cx.b.elements[ship].next.image.frame, frame);
                    cx.b.elements[ship].state_flags |= CHANGING as u16;
                }
            }
        } else {
            flags &= !((THRUST | WEAPON | SPECIAL) as u16);
            if cx.desc(ss).characteristics.turn_wait > BLAZER_TURN_WAIT {
                cx.desc_mut(ss).characteristics.special_wait = cx
                    .desc(ss)
                    .characteristics
                    .special_wait
                    .wrapping_add(cx.desc(ss).characteristics.turn_wait - BLAZER_TURN_WAIT);
                cx.desc_mut(ss).characteristics.turn_wait = BLAZER_TURN_WAIT;
            }
            if cx.desc(ss).ship_info.energy_level == 0 {
                cx.b.elements[ship].velocity = crate::battle::velocity::VelocityDesc::new();
                flags &= !((LEFT | RIGHT | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                cx.desc_mut(ss).characteristics.turn_wait =
                    cx.desc(ss).characteristics.special_wait;
                cx.desc_mut(ss).characteristics.energy_regeneration = ENERGY_REGENERATION;
                cx.b.elements[ship].set_mass_points(SHIP_MASS);
                let RaceState::Androsynth { collision } = cx.desc(ss).race_state else {
                    panic!("blazer has no saved collision hook")
                };
                cx.b.elements[ship].collision_func = collision;
                let frames = cx.ship_frames(ss, 0);
                image!(set; cx.b.elements[ship].next.image.farray, frames);
                let frame = cx.equ_frame(
                    cx.desc(ss).ship_data.ship[0],
                    cx.b.elements[ship].next.image.frame,
                );
                image!(frame; cx.b.elements[ship].next.image.frame, frame);
                cx.b.elements[ship].state_flags |= CHANGING as u16;
            } else if cx.b.elements[ship].thrust_wait() != 0 {
                let wait = cx.b.elements[ship].thrust_wait() - 1;
                cx.b.elements[ship].set_thrust_wait(wait);
            } else {
                let mut facing = cx.ship(ss).ShipFacing;
                if cx.b.elements[ship].turn_wait() == 0 && flags & (LEFT | RIGHT) as u16 != 0 {
                    facing = if flags & LEFT as u16 != 0 {
                        facing.wrapping_sub(1)
                    } else {
                        facing.wrapping_add(1)
                    };
                }
                cx.b.elements[ship]
                    .velocity
                    .set_vector(i32::from(BLAZER_THRUST), facing & FACING_MASK as u16);
                flags |= (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            }
        }
        cx.ship_mut(ss).cur_status_flags = flags;
    }

    pub(crate) fn uninit_androsynth(desc: &mut RaceDesc) {
        desc.race_state = RaceState::None;
    }
}
