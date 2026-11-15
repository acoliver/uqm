//! Native Rust port of the cyborg battle intelligence in `cyborg.c`.
//!
//! The arithmetic helpers are kept independent of engine state so their C
//! integer-width behavior can be tested without linking the game archive.

#[cfg(not(test))]
use crate::battle::context_access::ContextOps;
#[cfg(not(test))]
use crate::engine::battle::intercept::{next_velocity, plot_intercept};
use crate::math::{angle_to_facing, normalize_facing};
#[cfg(not(test))]
use crate::math::{facing_to_angle, normalize_angle};
use crate::ships::native::abi;

#[inline]
fn break_opposite_facing_tie(delta: u16, random: impl FnOnce() -> u32) -> u16 {
    let jitter = angle_to_facing((abi::OCTANT >> 1) as i32);
    normalize_facing(i32::from(delta) + if random() & 1 != 0 { jitter } else { -jitter })
}

#[cfg(not(test))]
use crate::engine::battle::{Concern, Concerns, Cx, Parent, ShipId, Subject};
#[cfg(not(test))]
use abi::*;
#[cfg(not(test))]
fn subject_owner(subject: &Subject) -> ShipId {
    match subject.parent {
        Parent::Ship(id) => id,
        other => panic!("AI subject has no ship owner: {other:?}"),
    }
}
#[cfg(not(test))]
fn is_crew_preprocess(subject: &Subject) -> bool {
    subject.pre.is_some_and(|f| {
        f.address() == crate::engine::battle::world_elements::crew_preprocess as *const () as usize
    })
}
#[cfg(not(test))]
fn ai_arctan(x: i16, y: i16) -> u16 {
    crate::math::arctan(x, y)
}
#[cfg(not(test))]
fn collision_possible(element: &Subject, ship: &Subject) -> bool {
    element.flags & (NONSOLID | DISAPPEARING) as u16 == 0
        && (ship.flags & element.flags) & COLLISION as u16 == 0
        && ((ship.flags & element.flags) & IGNORE_SIMILAR as u16 == 0
            || ship.parent != element.parent)
        && (ship.mass != 0 || element.mass != 0)
}
#[cfg(not(test))]
const CLOSE_RANGE_WEAPON: u16 = (50 << ONE_SHIFT) as u16;
#[cfg(not(test))]
const LONG_RANGE_WEAPON: u16 = (1000 << ONE_SHIFT) as u16;
#[cfg(not(test))]
const FAST_SHIP: u16 = 150;

#[cfg(not(test))]
pub(crate) fn turn_ship(cx: &mut Cx<'_>, ship: &mut Subject, angle: COUNT) -> BOOLEAN {
    let starship = subject_owner(ship);
    let facing = cx.ship_state(starship).facing;
    let mut delta = normalize_facing(angle_to_facing(i32::from(angle)) - i32::from(facing));
    if delta == 0 {
        return 0;
    }
    if delta == angle_to_facing(HALF_CIRCLE as i32) as u16 {
        delta = break_opposite_facing_tie(delta, || cx.random());
    }
    let mut f = i32::from(facing);
    if delta < angle_to_facing(HALF_CIRCLE as i32) as u16 {
        cx.ship_state_mut(starship).input |= RIGHT as u8;
        f += 1;
        ship.next.image.frame = cx.advance_frame(ship.current.image.frame.view());
    } else {
        cx.ship_state_mut(starship).input |= LEFT as u8;
        f -= 1;
        ship.next.image.frame = cx.retreat_frame(ship.current.image.frame.view());
    }
    cx.ship_state_mut(starship).facing = normalize_facing(f);
    1
}

#[cfg(not(test))]
pub(crate) fn thrust_ship(cx: &mut Cx<'_>, ship: &mut Subject, angle: COUNT) -> BOOLEAN {
    let starship = subject_owner(ship);
    let mut should = cx.ship_state(starship).input & THRUST as u8 != 0;
    if !should {
        let travel = ship.velocity.travel_angle;
        let at_cap = cx.ship_state(starship).status
            & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
            != 0;
        if normalize_facing(angle_to_facing(i32::from(angle)) - angle_to_facing(i32::from(travel)))
            == 0
            && at_cap
            && cx.ship_state(starship).status & SHIP_IN_GRAVITY_WELL as u16 == 0
        {
            should = false;
        } else {
            let d = normalize_facing(
                angle_to_facing(i32::from(angle)) - i32::from(cx.ship_state(starship).facing)
                    + angle_to_facing(QUADRANT as i32),
            );
            should = d == angle_to_facing(QUADRANT as i32) as u16
                || (cx.ship_state(starship).status & SHIP_BEYOND_MAX_SPEED as u16 != 0
                    && d <= angle_to_facing(HALF_CIRCLE as i32) as u16);
        }
    }
    if should {
        let stats = cx.race_stats(starship);
        crate::engine::battle::motion::thrust(
            &mut ship.velocity,
            cx.ship_state(starship).facing,
            cx.ship_state(starship).status,
            stats.characteristics.thrust_increment,
            stats.characteristics.max_thrust,
        );
        cx.ship_state_mut(starship).input |= THRUST as u8;
    }
    should as BOOLEAN
}

#[cfg(not(test))]
pub(crate) fn weapons(
    cx: &mut Cx<'_>,
    ship: &Subject,
    other: &Subject,
    mut margin: COUNT,
) -> BOOLEAN {
    if cx.subject_cloaked(other) {
        margin = margin.wrapping_add((40 << ONE_SHIFT) as u16);
    }
    let mut moved = ship.clone();
    let (dx, dy) = next_velocity(&mut moved.velocity, 1);
    moved.next.location.x = moved.current.location.x.wrapping_add(dx);
    moved.next.location.y = moved.current.location.y.wrapping_add(dy);
    moved.current.location = moved.next.location;
    let weapons = cx.spawn_weapons(subject_owner(&moved), moved);
    for (index, id) in weapons.iter().copied().enumerate() {
        let Some(id) = id else {
            continue;
        };
        if cx.element(id).flags & APPEARING as u16 != 0 {
            let current = cx.element(id).current;
            cx.copy_visual_to_next(id, current);
            cx.init_intersect(id);
            let frame = cx.equivalent_frame(
                cx.element_frame_set_first(id, true),
                cx.element(id).next.image.frame.view(),
            );
            cx.set_intersect_frame(id, frame);
        }
        let weapon = cx.weapon_origin(id);
        let hit = plot_intercept(cx, &weapon, other, weapon.life, margin) != 0;
        cx.dispose(id);
        if hit {
            for unused in weapons[index + 1..].iter().flatten() {
                cx.dispose(*unused);
            }
            return 1;
        }
    }
    0
}

#[cfg(not(test))]
pub(crate) fn intelligence(
    cx: &mut Cx<'_>,
    ship: &mut Subject,
    concerns: &mut [Concern],
    count: COUNT,
) {
    let owner = subject_owner(ship);
    let mut ship_moved = ship.turn != 0 && ship.thrust != 0;
    let mut ship_fired = true;
    if cx.ship_state(owner).weapon_counter == 0 {
        cx.ship_state_mut(owner).input &= !(WEAPON as u8);
        if cx.race_stats(owner).ship_info.ship_flags & SEEKING_WEAPON as u16 == 0 {
            ship_fired = false;
        }
    }
    let margin = if cx.pick_ship(owner).control & AWESOME_RATING != 0 {
        0
    } else if cx.pick_ship(owner).control & GOOD_RATING != 0 {
        (20 << ONE_SHIFT) as u16
    } else {
        (40 << ONE_SHIFT) as u16
    };
    let mut i = count;

    while i != 0 {
        i -= 1;
        let item = &mut concerns[i as usize];
        if item.object.is_none() {
            continue;
        }
        if !ship_moved
            && (i != ENEMY_WEAPON_INDEX as u16
                || item.movement == MOVEMENT_STATE_PURSUE
                || cx.element(item.object.unwrap()).flags & CREW_OBJECT as u16 != 0
                || cx.race_stats(owner).intelligence.maneuverability >= MEDIUM_SHIP as u16)
        {
            if item.which_turn == 0 {
                item.which_turn = 1;
            }
            match item.movement {
                MOVEMENT_STATE_PURSUE => pursue(cx, ship, item),
                MOVEMENT_STATE_AVOID | MOVEMENT_STATE_ENTICE => entice(cx, ship, item),
                _ => {}
            }
            ship_moved = true;
        }
        if !ship_fired
            && (i == ENEMY_SHIP_INDEX as u16
                || (i == ENEMY_WEAPON_INDEX as u16 && item.movement != MOVEMENT_STATE_AVOID))
        {
            ship_fired = weapons(cx, ship, &cx.weapon_origin(item.object.unwrap()), margin) != 0;
            if ship_fired {
                cx.ship_state_mut(owner).input |= WEAPON as u8;
            }
        }
    }
}

#[cfg(not(test))]
pub(crate) fn pursue(cx: &mut Cx<'_>, ship: &mut Subject, eval: &mut Concern) {
    let object = eval.object.unwrap_or_else(|| panic!("AI object missing"));
    let other = cx.weapon_origin(object);
    let mut ship_velocity = ship.velocity;
    let mut other_velocity = other.velocity;
    let (sx, sy) = next_velocity(&mut ship_velocity, eval.which_turn);
    ship.next.location.x = ship.current.location.x.wrapping_add(sx);
    ship.next.location.y = ship.current.location.y.wrapping_add(sy);
    let (ox, oy) = next_velocity(&mut other_velocity, eval.which_turn);
    let mut dx = (other.current.location.x as i32 + ox as i32 - ship.next.location.x as i32) as i16;
    let mut dy = (other.current.location.y as i32 + oy as i32 - ship.next.location.y as i32) as i16;
    dx = crate::math::wrap_delta(
        dx,
        (cx.screen_dimensions().0 - 64) << (ONE_SHIFT + MAX_REDUCTION),
    );
    dy = crate::math::wrap_delta(dy, cx.screen_dimensions().1 << (ONE_SHIFT + MAX_REDUCTION));
    let mut thrust_angle = ai_arctan(dx, dy);
    let mut maneuver: u8 = 0;
    if ship.turn == 0 {
        maneuver |= (LEFT | RIGHT) as u8;
    }
    if ship.thrust == 0
        && (other.flags & PLAYER_SHIP as u16 != 0
            || (other.player == ship.player)
            || is_crew_preprocess(&other))
    {
        maneuver |= THRUST as u8;
    }
    let turn_angle = normalize_angle(thrust_angle as i32 + HALF_CIRCLE as i32) as u16;
    if other.flags & PLAYER_SHIP as u16 != 0 && other.mass <= MAX_SHIP_MASS as u8 {
        let us = subject_owner(ship);
        let enemy = subject_owner(&other);
        let ur = us;
        let er = enemy;
        if (cx.race_stats(ur).intelligence.maneuverability >= FAST_SHIP
            && cx.race_stats(ur).intelligence.weapon_range > CLOSE_RANGE_WEAPON)
            || (eval.which_turn >= 24
                && ((cx.race_stats(ur).characteristics.max_thrust as u32 * 2 / 3)
                    < cx.race_stats(er).characteristics.max_thrust as u32
                    || cx.ship_state(enemy).status & SHIP_BEYOND_MAX_SPEED as u16 != 0))
        {
            let flags = cx.race_stats(er).ship_info.ship_flags;
            if cx.race_stats(ur).intelligence.maneuverability >= MEDIUM_SHIP as u16 {
                let mut fire = FIRES_FORE as u16;
                let mut facing = eval.facing;
                while fire <= FIRES_LEFT as u16 {
                    if flags & fire != 0
                        && normalize_angle(turn_angle as i32 - facing as i32 + OCTANT as i32)
                            <= QUADRANT as i32
                        && (ox != 0 || oy != 0)
                        && normalize_angle(
                            other_velocity.travel_angle as i32 + HALF_CIRCLE as i32 - facing as i32
                                + OCTANT as i32
                                + (OCTANT >> 1) as i32,
                        ) <= ((OCTANT + (OCTANT >> 1)) << 1) as i32
                    {
                        thrust_angle = turn_angle;
                        break;
                    }
                    fire <<= 1;
                    facing = facing.wrapping_add(QUADRANT as u16);
                }
            }
            if thrust_angle != turn_angle
                && (ox != 0 || oy != 0)
                && eval.which_turn >= 24
                && normalize_angle(
                    thrust_angle as i32 - other_velocity.travel_angle as i32 + OCTANT as i32,
                ) <= QUADRANT as i32
                && ((normalize_angle(
                    other_velocity.travel_angle as i32 - ship_velocity.travel_angle as i32
                        + OCTANT as i32,
                ) <= QUADRANT as i32
                    && ((cx.ship_state(us).status & SHIP_AT_MAX_SPEED as u16 != 0
                        && cx.ship_state(us).status & SHIP_BEYOND_MAX_SPEED as u16 == 0)
                        || flags & DONT_CHASE as u16 != 0))
                    || normalize_angle(
                        turn_angle as i32 - facing_to_angle(cx.ship_state(us).facing as i32)
                            + OCTANT as i32,
                    ) <= QUADRANT as i32)
            {
                thrust_angle = turn_angle;
            }
        }
    }
    if maneuver & (LEFT | RIGHT) as u8 != 0 {
        turn_ship(cx, ship, thrust_angle);
    }
    if maneuver & THRUST as u8 != 0 {
        thrust_ship(cx, ship, thrust_angle);
    }
}

#[cfg(not(test))]
pub(crate) fn entice(cx: &mut Cx<'_>, ship: &mut Subject, eval: &mut Concern) {
    let object = eval.object.unwrap_or_else(|| panic!("AI object missing"));
    let other = cx.weapon_origin(object);
    let mut ship_velocity = ship.velocity;
    let mut other_velocity = other.velocity;
    let (sx, sy) = next_velocity(&mut ship_velocity, eval.which_turn);
    ship.next.location.x = ship.current.location.x.wrapping_add(sx);
    ship.next.location.y = ship.current.location.y.wrapping_add(sy);
    let (ox, oy) = next_velocity(&mut other_velocity, eval.which_turn);
    let mut dx = (other.current.location.x as i32 + ox as i32 - ship.next.location.x as i32) as i16;
    let mut dy = (other.current.location.y as i32 + oy as i32 - ship.next.location.y as i32) as i16;
    dx = crate::math::wrap_delta(
        dx,
        (cx.screen_dimensions().0 - 64) << (ONE_SHIFT + MAX_REDUCTION),
    );
    dy = crate::math::wrap_delta(dy, cx.screen_dimensions().1 << (ONE_SHIFT + MAX_REDUCTION));
    let mut thrust_angle = ai_arctan(dx, dy);
    let mut maneuver = 0u8;
    if ship.turn == 0 {
        maneuver |= (LEFT | RIGHT) as u8;
    }
    if ship.thrust == 0 {
        maneuver |= THRUST as u8;
    }
    let travel = ai_arctan(sx.wrapping_sub(ox), sy.wrapping_sub(oy));
    let mut turn_angle = normalize_angle(thrust_angle as i32 + HALF_CIRCLE as i32) as u16;
    let rd = subject_owner(ship);

    if eval.movement == MOVEMENT_STATE_AVOID {
        turn_angle = normalize_angle(turn_angle as i32 - eval.facing as i32) as u16;
        let direction = if normalize_facing(angle_to_facing(turn_angle as i32)) != 0 {
            if turn_angle <= HALF_CIRCLE as u16 {
                RIGHT as u16
            } else {
                LEFT as u16
            }
        } else {
            turn_angle = normalize_angle(
                facing_to_angle(cx.ship_state(rd).facing as i32) - eval.facing as i32,
            ) as u16;
            if turn_angle & (HALF_CIRCLE as u16 - 1) == 0 {
                if cx.random() & 1 != 0 {
                    RIGHT as u16
                } else {
                    LEFT as u16
                }
            } else if turn_angle < HALF_CIRCLE as u16 {
                RIGHT as u16
            } else {
                LEFT as u16
            }
        };
        thrust_angle = if direction == LEFT as u16 {
            eval.facing
                .wrapping_sub(QUADRANT as u16)
                .wrapping_add(OCTANT as u16 / 2)
        } else {
            eval.facing
                .wrapping_add(QUADRANT as u16)
                .wrapping_sub(OCTANT as u16 / 2)
        };
        thrust_angle = normalize_angle(thrust_angle as i32) as u16;
    } else if other.mass as u16 > (MAX_SHIP_MASS * 10) as u16 {
        let planet_facing = normalize_facing(angle_to_facing(thrust_angle as i32));
        let cone = normalize_facing(
            i32::from(planet_facing) - i32::from(cx.ship_state(rd).facing)
                + angle_to_facing(QUADRANT as i32),
        );
        if cx.race_stats(rd).characteristics.thrust_increment
            != cx.race_stats(rd).characteristics.max_thrust
        {
            maneuver &= !(THRUST as u8);
        }
        if cone > angle_to_facing((QUADRANT << 1) as i32) as u16 {
            turn_angle = thrust_angle;
        } else if cone == angle_to_facing(QUADRANT as i32) as u16
            && normalize_facing(angle_to_facing(travel as i32)) != planet_facing
        {
            turn_angle = travel;
        } else if cone == 0
            || cone == angle_to_facing((QUADRANT << 1) as i32) as u16
            || (maneuver & THRUST as u8 == 0
                && (cone < angle_to_facing(OCTANT as i32) as u16
                    || cone > angle_to_facing(((QUADRANT << 1) - OCTANT) as i32) as u16))
        {
            turn_angle = facing_to_angle(cx.ship_state(rd).facing as i32) as u16;
            if normalize_angle(turn_angle as i32 - travel as i32 + QUADRANT as i32)
                > HALF_CIRCLE as i32
            {
                turn_angle = travel;
            }
            if ship.thrust == 0 {
                maneuver |= THRUST as u8;
            }
        }
        thrust_angle = turn_angle;
    } else {
        let wrange = cx.race_stats(rd).intelligence.weapon_range;
        let mut cone =
            normalize_angle(turn_angle as i32 - eval.facing as i32 + OCTANT as i32) as u16;
        if other.flags & PLAYER_SHIP as u16 != 0 {
            let enemy = subject_owner(&other);
            let flags = cx.race_stats(enemy).ship_info.ship_flags;
            let mut fire = FIRES_FORE as u16;
            let mut facing = eval.facing;
            let mut go_maneuver = false;
            while fire <= FIRES_LEFT as u16 {
                if flags & fire != 0 {
                    cone =
                        normalize_angle(turn_angle as i32 - facing as i32 + OCTANT as i32) as u16;
                    if cone <= QUADRANT as u16
                        && (ox != 0 || oy != 0)
                        && normalize_angle(
                            other_velocity.travel_angle as i32 + HALF_CIRCLE as i32 - facing as i32
                                + OCTANT as i32,
                        ) <= QUADRANT as i32
                    {
                        if wrange < (1000 << ONE_SHIFT) as u16 && eval.which_turn <= 32 {
                            thrust_angle = turn_angle;
                            go_maneuver = true;
                            break;
                        }
                        break;
                    }
                }
                fire <<= 1;
                facing = facing.wrapping_add(QUADRANT as u16);
            }
            if go_maneuver {
                if maneuver & (LEFT | RIGHT) as u8 != 0 {
                    turn_ship(cx, ship, thrust_angle);
                }
                if maneuver & THRUST as u8 != 0 {
                    thrust_ship(cx, ship, thrust_angle);
                }
                return;
            }
            if eval.which_turn <= 8
                && cx.race_stats(rd).characteristics.max_thrust
                    <= cx.race_stats(enemy).characteristics.max_thrust
            {
                if maneuver & (LEFT | RIGHT) as u8 != 0 {
                    turn_ship(cx, ship, thrust_angle);
                }
                if maneuver & THRUST as u8 != 0 {
                    thrust_ship(cx, ship, thrust_angle);
                }
                return;
            }
        }
        if cx.ship_state(rd).status & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16 == 0
            && plot_intercept(cx, ship, &other, 40, CLOSE_RANGE_WEAPON << 1) != 0
        {
            if normalize_angle(
                thrust_angle as i32 - facing_to_angle(cx.ship_state(rd).facing as i32)
                    + OCTANT as i32,
            ) <= QUADRANT as i32
                || cone > QUADRANT as u16
            {
                thrust_angle = turn_angle;
            } else {
                thrust_angle = facing_to_angle(cx.ship_state(rd).facing as i32) as u16;
            }
        } else {
            cone = plot_intercept(cx, ship, &other, 10, wrange.wrapping_sub(wrange >> 2));
            if cone != 0 {
                if cx.race_stats(rd).characteristics.thrust_increment
                    != cx.race_stats(rd).characteristics.max_thrust
                    && cx.ship_state(rd).status & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
                        != 0
                    && (normalize_angle(
                        turn_angle as i32 - ai_arctan(sx, sy) as i32 + OCTANT as i32 + 2,
                    ) <= ((OCTANT + 2) << 1) as i32
                        || plot_intercept(cx, ship, &other, 30, CLOSE_RANGE_WEAPON << 1) == 0)
                {
                    maneuver &= !(THRUST as u8);
                } else if cone == 1
                    || cx.race_stats(rd).characteristics.thrust_increment
                        != cx.race_stats(rd).characteristics.max_thrust
                {
                    if maneuver & (LEFT | RIGHT) as u8 != 0 {
                        turn_ship(cx, ship, turn_angle);
                        maneuver &= !((LEFT | RIGHT) as u8);
                    }
                    if normalize_angle(
                        thrust_angle as i32 - ai_arctan(sx, sy) as i32 + OCTANT as i32 + 2,
                    ) <= ((OCTANT + 2) << 1) as i32
                    {
                        thrust_angle = facing_to_angle(cx.ship_state(rd).facing as i32) as u16;
                    } else {
                        thrust_angle = turn_angle;
                    }
                }
            }
        }
    }
    if maneuver & (LEFT | RIGHT) as u8 != 0 {
        turn_ship(cx, ship, thrust_angle);
    }
    if maneuver & THRUST as u8 != 0 {
        thrust_ship(cx, ship, thrust_angle);
    }
}

#[cfg(not(test))]
fn init_cyborg(cx: &mut Cx<'_>, ship: ShipId) {
    let race = ship;
    let characteristics = cx.race_stats(race).characteristics;
    let product = (u32::from(characteristics.max_thrust)
        * u32::from(characteristics.thrust_increment)) as u16;
    let divisor =
        (u32::from(characteristics.turn_wait) + u32::from(characteristics.thrust_wait)) as u16;
    cx.race_stats_mut(race).intelligence.maneuverability =
        product.checked_div(divisor).unwrap_or(product >> 1);
}

#[cfg(not(test))]
pub fn tactical(cx: &mut Cx<'_>, owner: ShipId) -> BATTLE_INPUT_STATE {
    let rd = owner;
    if cx.race_stats(rd).intelligence.maneuverability == 0 {
        init_cyborg(cx, owner);
    }
    let handle = cx.ship_element(owner);
    let ship = cx.weapon_origin(handle);
    if cx.race_stats(rd).ship_info.crew_level == 0
        || u32::from(cx.subject_style(&ship).kind) == gfx_object_NUM_PRIMS
    {
        return 0;
    }
    let mut ship_copy = ship;
    if cx.pick_ship(owner).control & STANDARD_RATING != 0 {
        cx.ship_state_mut(owner).special_counter =
            cx.ship_state(owner).special_counter.wrapping_add(1);
    }

    let original_facing = cx.ship_state(owner).facing;
    let mut concerns = [Concern {
        object: None,
        facing: 0,
        which_turn: 0,
        movement: 0,
    }; 10];
    for concern in &mut concerns[..=FIRST_EMPTY_INDEX as usize] {
        concern.object = None;
        concern.movement = MOVEMENT_STATE_NO_MOVEMENT;
        concern.which_turn = u16::MAX;
    }
    let ultra = cx.race_stats(rd).characteristics.thrust_increment
        == cx.race_stats(rd).characteristics.max_thrust
        && cx.race_stats(rd).intelligence.maneuverability >= MEDIUM_SHIP as u16;
    let mut moved = true;
    if ship_copy.turn == 0 {
        moved = false;
        cx.ship_state_mut(owner).input &= !((LEFT | RIGHT) as u8);
    }
    if ship_copy.thrust == 0 {
        moved = false;
        cx.ship_state_mut(owner).input &= !(THRUST as u8);
    }

    let mut h = cx.elements().head();
    while let Some(e_id) = h {
        let e = cx.weapon_origin(e_id);
        let next = cx.elements().succ(e_id);
        if collision_possible(&e, &ship_copy) {
            let mut dx = e.next.location.x.wrapping_sub(ship_copy.next.location.x);
            let mut dy = e.next.location.y.wrapping_sub(ship_copy.next.location.y);
            dx = crate::math::wrap_delta(
                dx,
                (cx.screen_dimensions().0 - 64) << (ONE_SHIFT + MAX_REDUCTION),
            );
            dy = crate::math::wrap_delta(
                dy,
                cx.screen_dimensions().1 << (ONE_SHIFT + MAX_REDUCTION),
            );
            let mass = e.mass;
            if mass as u16 > (MAX_SHIP_MASS * 10) as u16 {
                let turn = if ultra {
                    16
                } else if cx.race_stats(rd).intelligence.maneuverability <= MEDIUM_SHIP as u16 {
                    48
                } else {
                    32
                };

                let frame = cx.absolute_frame(cx.subject_intersect_frame(&ship_copy), 0);
                let rect = cx.frame_rect(frame.view());
                let bounds = (i32::from(rect.extent.width) + i32::from(rect.extent.height)) as u16;
                let mut intercept_turn = 0;
                if !moved {
                    intercept_turn = plot_intercept(
                        cx,
                        &e,
                        &ship_copy,
                        turn,
                        ((30 + bounds as i32 * 3) << ONE_SHIFT) as u16,
                    );
                }
                if intercept_turn != 0 {
                    if intercept_turn > 1
                        || plot_intercept(
                            cx,
                            &e,
                            &ship_copy,
                            1,
                            ((35 + bounds as i32) << ONE_SHIFT) as u16,
                        ) != 0
                        || plot_intercept(
                            cx,
                            &e,
                            &ship_copy,
                            turn << 1,
                            ((40 + bounds as i32) << ONE_SHIFT) as u16,
                        ) > 1
                    {
                        concerns[GRAVITY_MASS_INDEX as usize].object = Some(e_id);
                        concerns[GRAVITY_MASS_INDEX as usize].which_turn = intercept_turn;
                        concerns[GRAVITY_MASS_INDEX as usize].facing =
                            ai_arctan(dx.wrapping_neg(), dy.wrapping_neg());
                        concerns[GRAVITY_MASS_INDEX as usize].movement = if ultra {
                            MOVEMENT_STATE_AVOID
                        } else {
                            MOVEMENT_STATE_ENTICE
                        };
                    } else if !ultra && !ship_copy.velocity.is_zero() {
                        ship_copy.thrust = ship_copy.thrust.wrapping_add(1);
                        if ship_copy.turn != 0 {
                            moved = true;
                        }
                    }
                }
            } else if e.flags & PLAYER_SHIP as u16 != 0 {
                let enemy = cx.owner(e_id);
                let erd = enemy;
                if cx.race_stats(erd).intelligence.maneuverability == 0 {
                    init_cyborg(cx, enemy);
                }
                let dist2 = (dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32;
                let mut turns = square_root(dist2) >> 6;
                let idx = ENEMY_SHIP_INDEX as usize;
                if turns <= concerns[idx].which_turn {
                    if turns == 0 {
                        turns = 1;
                    }
                    concerns[idx].object = Some(e_id);
                    concerns[idx].which_turn = turns;
                    concerns[idx].facing =
                        facing_to_angle(cx.ship_state(enemy).facing as i32) as u16;
                    let pursue = moved
                        || mass > MAX_SHIP_MASS as u8
                        || (cx.race_stats(rd).intelligence.weapon_range < LONG_RANGE_WEAPON
                            && (cx.race_stats(rd).intelligence.weapon_range <= CLOSE_RANGE_WEAPON
                                || (cx.race_stats(erd).intelligence.weapon_range
                                    >= LONG_RANGE_WEAPON
                                    && cx.race_stats(erd).ship_info.ship_flags
                                        & SEEKING_WEAPON as u16
                                        != 0)
                                || (cx.race_stats(rd).characteristics.max_thrust
                                    < cx.race_stats(erd).characteristics.max_thrust
                                    && cx.race_stats(rd).intelligence.weapon_range
                                        < cx.race_stats(erd).intelligence.weapon_range)));
                    concerns[idx].movement = if pursue {
                        MOVEMENT_STATE_PURSUE
                    } else {
                        MOVEMENT_STATE_ENTICE
                    };
                    if cx.race_stats(erd).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 != 0
                        && weapons(cx, &e, &ship_copy, 0) != 0
                    {
                        concerns[ENEMY_WEAPON_INDEX as usize] = concerns[idx];
                        concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 1;
                        concerns[ENEMY_WEAPON_INDEX as usize].movement = MOVEMENT_STATE_AVOID;
                    }
                }
            } else if e.parent == Parent::None {
                if e.flags & FINITE_LIFE as u16 == 0 {
                    let turns =
                        square_root((dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32) >> 6;
                    let idx = FIRST_EMPTY_INDEX as usize;
                    if turns < concerns[idx].which_turn {
                        concerns[idx].object = Some(e_id);
                        concerns[idx].which_turn = turns;
                        concerns[idx].movement = MOVEMENT_STATE_PURSUE;
                        concerns[idx].facing = e.velocity.travel_angle;
                    }
                }
            } else if !(e.player == ship_copy.player)
                && !is_crew_preprocess(&e)
                && concerns[ENEMY_WEAPON_INDEX as usize].which_turn > 1
                && e.life > 0
            {
                let enemy = cx.owner(e_id);
                let erd = enemy;
                let special_frames = cx.ship_frame_set(erd, 2);
                let seeking = (cx.race_stats(erd).ship_info.ship_flags & SEEKING_WEAPON as u16
                    != 0
                    && e.next.image.frames != special_frames)
                    || (cx.race_stats(erd).ship_info.ship_flags & SEEKING_SPECIAL as u16 != 0
                        && e.next.image.frames == special_frames);
                let mut turns = 0u16;
                let mut movement = MOVEMENT_STATE_NO_MOVEMENT;
                if seeking {
                    let travel = e.velocity.travel_angle;
                    let too_fast = e.flags & (FINITE_LIFE | CREW_OBJECT) as u16 == 0
                        && cx.race_stats(rd).characteristics.max_thrust > (8 << ONE_SHIFT) as u16;
                    let behind = normalize_angle(
                        travel as i32 - ai_arctan(dx.wrapping_neg(), dy.wrapping_neg()) as i32
                            + QUADRANT as i32,
                    ) > HALF_CIRCLE as i32;
                    if !too_fast && !behind {
                        turns = square_root((dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32)
                            >> 6;
                        movement = MOVEMENT_STATE_ENTICE;
                        if ultra {
                            if turns == 0 {
                                turns = 1;
                            } else if turns > 16 {
                                turns = 0;
                            }
                        } else if turns == 0 {
                            turns = 1;
                        } else if turns > 16
                            || (cx.race_stats(rd).intelligence.maneuverability > MEDIUM_SHIP as u16
                                && turns > 8)
                        {
                            turns = 0;
                        }
                    }
                } else if cx.pick_ship(owner).control & AWESOME_RATING != 0 {
                    turns = plot_intercept(cx, &e, &ship_copy, e.life, (40 << ONE_SHIFT) as u16);
                    movement = MOVEMENT_STATE_AVOID;
                }
                let idx = ENEMY_WEAPON_INDEX as usize;
                if turns > 0
                    && (turns < concerns[idx].which_turn
                        || (turns == concerns[idx].which_turn && movement == MOVEMENT_STATE_AVOID))
                {
                    concerns[idx].object = Some(e_id);
                    concerns[idx].which_turn = turns;
                    concerns[idx].movement = movement;
                    concerns[idx].facing = e.velocity.travel_angle;
                }
            } else if e.flags & CREW_OBJECT as u16 != 0
                && ((e.flags & IGNORE_SIMILAR as u16 == 0 && (e.player == ship_copy.player))
                    || is_crew_preprocess(&e))
                && concerns[CREW_OBJECT_INDEX as usize].which_turn > 1
            {
                let mut turns =
                    square_root((dx as i64 * dx as i64 + dy as i64 * dy as i64) as u32) >> 6;
                if turns == 0 {
                    turns = 1;
                }
                let idx = CREW_OBJECT_INDEX as usize;
                let enemy_turns = concerns[ENEMY_SHIP_INDEX as usize].which_turn;
                if turns < concerns[idx].which_turn
                    && (enemy_turns > 32
                        || (enemy_turns > 8 && Some(cx.ship_element(owner)) == e.target))
                {
                    concerns[idx].object = Some(e_id);
                    concerns[idx].which_turn = turns;
                    concerns[idx].movement = MOVEMENT_STATE_PURSUE;
                    concerns[idx].facing = 0;
                }
            }
        }

        h = next;
    }

    let mut evaluation = Concerns {
        objects: concerns,
        subject: ship_copy,
        count: FIRST_EMPTY_INDEX as u16,
    };
    cx.race_intelligence_with_junk(owner, &mut evaluation);
    cx.ship_state_mut(owner).facing = original_facing;
    crate::battle::process_native::dispatch::battle_input(cx.ship_state(owner).input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opposite_heading_tie_uses_c_jitter_and_consumes_one_random_value() {
        let opposite = angle_to_facing(abi::HALF_CIRCLE as i32) as u16;
        let mut calls = 0;
        assert_eq!(
            break_opposite_facing_tie(opposite, || {
                calls += 1;
                0
            }),
            7
        );
        assert_eq!(calls, 1);
        assert_eq!(break_opposite_facing_tie(opposite, || 1), 9);
    }
}
