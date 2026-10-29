//! Native Rust port of the cyborg battle intelligence in `cyborg.c`.
//!
//! The arithmetic helpers are kept independent of engine state so their C
//! integer-width behavior can be tested without linking the game archive.

use crate::ships::native::{
    abi, angle_to_facing, facing_to_angle, normalize_angle, normalize_facing,
};

#[inline]
fn c_size(v: i32) -> i16 {
    v as i16
}

#[inline]
fn break_opposite_facing_tie(delta: u16, random: impl FnOnce() -> u32) -> u16 {
    let jitter = angle_to_facing((abi::OCTANT >> 1) as i32);
    normalize_facing(i32::from(delta) + if random() & 1 != 0 { jitter } else { -jitter })
}

#[inline]
fn velocity_to_world(v: i64) -> i16 {
    (v >> abi::VELOCITY_SHIFT) as i16
}

/// Return the first turn at which the two swept rectangles can overlap.
///
/// This is the integer-only broad phase from `PlotIntercept`; the engine
/// narrow-phase collision callback is applied by the live implementation.
pub(crate) fn intercept_window(
    p0: (i16, i16),
    v0: (i16, i16),
    p1: (i16, i16),
    v1: (i16, i16),
    max_turns: u16,
    extents: [(i16, i16); 2],
) -> Option<(i16, i16)> {
    let (mut dx0, mut dy0) = v0;
    let (mut dx1, mut dy1) = v1;
    dx0 = velocity_to_world(i64::from(dx0) * i64::from(max_turns));
    dy0 = velocity_to_world(i64::from(dy0) * i64::from(max_turns));
    dx1 = velocity_to_world(i64::from(dx1) * i64::from(max_turns));
    dy1 = velocity_to_world(i64::from(dy1) * i64::from(max_turns));

    let x0 = i32::from(p0.0);
    let y0 = i32::from(p0.1);
    let x1 = i32::from(p1.0);
    let y1 = i32::from(p1.1);
    let mut t0y = i32::from(extents[0].1);
    let mut t1y = i32::from(extents[1].1);
    let diff_y = i32::from(c_size(y1 - y0));
    t0y = i32::from(c_size(diff_y - t0y + 1));
    t1y = i32::from(c_size(diff_y + t1y - 1));
    let mut rel_y = i32::from(c_size(i32::from(dy0) - i32::from(dy1)));

    if !axis_possible(t0y, t1y, rel_y) {
        return None;
    }
    let mut t0x = i32::from(extents[0].0);
    let mut t1x = i32::from(extents[1].0);
    let diff_x = i32::from(c_size(x1 - x0));
    t0x = i32::from(c_size(diff_x - t0x + 1));
    t1x = i32::from(c_size(diff_x + t1x - 1));
    let mut rel_x = i32::from(c_size(i32::from(dx0) - i32::from(dx1)));
    if !axis_possible(t0x, t1x, rel_x) {
        return None;
    }
    if rel_x == 0 && rel_y == 0 {
        return Some((0, 0));
    }

    if t1y < 0 {
        (t0y, t1y) = (i32::from(c_size(-t1y)), i32::from(c_size(-t0y)));
    } else if t0y <= 0 {
        if rel_y < 0 {
            t1y = i32::from(c_size(-t0y));
        }
        t0y = 0;
    }
    if rel_y < 0 {
        rel_y = i32::from(c_size(-rel_y));
    }
    if rel_y < t1y {
        t1y = rel_y;
    }
    if t1x < 0 {
        (t0x, t1x) = (i32::from(c_size(-t1x)), i32::from(c_size(-t0x)));
    } else if t0x <= 0 {
        if rel_x < 0 {
            t1x = i32::from(c_size(-t0x));
        }
        t0x = 0;
    }
    if rel_x < 0 {
        rel_x = i32::from(c_size(-rel_x));
    }
    if rel_x < t1x {
        t1x = rel_x;
    }

    let (begin, end, fract): (i64, i64, i64) = if rel_x == 0 {
        (i64::from(t0y), i64::from(t1y), i64::from(rel_y))
    } else if rel_y == 0 {
        (i64::from(t0x), i64::from(t1x), i64::from(rel_x))
    } else {
        let tx0 = i64::from(t0x) * i64::from(rel_y);
        let ty0 = i64::from(t0y) * i64::from(rel_x);
        let tx1 = i64::from(t1x) * i64::from(rel_y);
        let ty1 = i64::from(t1y) * i64::from(rel_x);
        (
            tx0.max(ty0),
            tx1.min(ty1),
            i64::from(rel_x) * i64::from(rel_y),
        )
    };
    let first = if begin * i64::from(max_turns) < fract {
        0
    } else {
        ((begin * i64::from(max_turns)) / fract) as i32
    } as i16;
    let last = if end >= fract {
        i32::from(max_turns) - 1
    } else {
        ((end * i64::from(max_turns)) / fract) as i32
    } as i16;
    if first <= last {
        Some((first, last))
    } else {
        None
    }
}

#[inline]
fn axis_possible(t0: i32, t1: i32, velocity: i32) -> bool {
    (t0 <= 0 && t1 >= 0) || (t0 > 0 && velocity >= t0) || (t1 < 0 && velocity <= t1)
}

#[cfg(not(test))]
pub(crate) use crate::battle::context_access::ai_legacy as engine;
#[cfg(not(test))]
use crate::battle::context_access::ai_legacy::frame_rect;
#[cfg(not(test))]
use crate::engine::battle::{Concern, Concerns, Cx, Parent, ShipId, Subject};
#[cfg(not(test))]
use crate::ships::native::abi::*;
#[cfg(not(test))]
pub use engine::{ship_intelligence, ship_weapons, tactical_intelligence, PlotIntercept};

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
        f.address() == crate::battle::world_elements::engine::crew_preprocess as *const () as usize
    })
}
#[cfg(not(test))]
fn ai_arctan(x: i16, y: i16) -> u16 {
    crate::battle::battle_types::arctan(i32::from(x), i32::from(y))
}
#[cfg(not(test))]
fn next_velocity(v: &mut crate::battle::velocity::VelocityDesc, turns: u16) -> (i16, i16) {
    let (x, y) = v.get_next_components(turns);
    (x as i16, y as i16)
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
mod intercept {
    use crate::battle::context_access::ai_legacy::{frame_rect, intersect};
    use crate::battle::cyborg_native::intercept_window;
    use crate::battle::velocity::VelocityDesc;
    use crate::engine::battle::{Cx, Subject};
    use crate::ships::native::abi::*;
    fn current_velocity(v: &VelocityDesc) -> (i16, i16) {
        let (x, y) = v.get_current_components();
        (x as i16, y as i16)
    }
    fn next_velocity(v: &mut VelocityDesc, n: u16) -> (i16, i16) {
        let (x, y) = v.get_next_components(n);
        (x as i16, y as i16)
    }
    pub(crate) fn plot_intercept(
        _cx: &mut Cx<'_>,
        a: &Subject,
        b: &Subject,
        mut turns: COUNT,
        margin: COUNT,
    ) -> COUNT {
        if (a.flags | b.flags) & FINITE_LIFE as u16 != 0 {
            if a.flags & FINITE_LIFE as u16 == 0 {
                turns = turns.min(b.life);
            } else if b.flags & FINITE_LIFE as u16 == 0 {
                turns = turns.min(a.life);
            } else {
                turns = turns.min(a.life).min(b.life);
            }
        }
        let (avx, avy) = current_velocity(&a.velocity);
        let (bvx, bvy) = current_velocity(&b.velocity);
        let (ha, hb, pos_a, pos_b) = if margin != 0 {
            let m = i32::from(margin);
            (
                (1, 1),
                ((m << 1) as i16, (m << 1) as i16),
                (a.current.location.x, a.current.location.y),
                (
                    b.current.location.x.wrapping_sub(m as i16),
                    b.current.location.y.wrapping_sub(m as i16),
                ),
            )
        } else {
            let ar = frame_rect(a.intersection().IntersectStamp.frame);
            let br = frame_rect(b.intersection().IntersectStamp.frame);
            let ax = (ar.corner.x as i32).wrapping_shl(ONE_SHIFT) as i16;
            let ay = (ar.corner.y as i32).wrapping_shl(ONE_SHIFT) as i16;
            let bx = (br.corner.x as i32).wrapping_shl(ONE_SHIFT) as i16;
            let by = (br.corner.y as i32).wrapping_shl(ONE_SHIFT) as i16;
            (
                (
                    (i32::from(ar.extent.width) << ONE_SHIFT) as i16,
                    (i32::from(ar.extent.height) << ONE_SHIFT) as i16,
                ),
                (
                    (i32::from(br.extent.width) << ONE_SHIFT) as i16,
                    (i32::from(br.extent.height) << ONE_SHIFT) as i16,
                ),
                (
                    a.current.location.x.wrapping_add(ax),
                    a.current.location.y.wrapping_add(ay),
                ),
                (
                    b.current.location.x.wrapping_add(bx),
                    b.current.location.y.wrapping_add(by),
                ),
            )
        };
        let Some((first, last)) =
            intercept_window(pos_a, (avx, avy), pos_b, (bvx, bvy), turns, [ha, hb])
        else {
            return 0;
        };
        if margin != 0 {
            return (first as u16).wrapping_add(1);
        }

        let mut ca = a.intersection();
        let mut cb = b.intersection();
        let mut pa = a.current.location;
        let mut pb = b.current.location;
        let mut va = a.velocity;
        let mut vb = b.velocity;
        if first != 0 {
            let (x, y) = next_velocity(&mut va, first as u16);
            pa.x = pa.x.wrapping_add(x);
            pa.y = pa.y.wrapping_add(y);
            ca.EndPoint.x = pa.x >> ONE_SHIFT;
            ca.EndPoint.y = pa.y >> ONE_SHIFT;
            let (x, y) = next_velocity(&mut vb, first as u16);
            pb.x = pb.x.wrapping_add(x);
            pb.y = pb.y.wrapping_add(y);
            cb.EndPoint.x = pb.x >> ONE_SHIFT;
            cb.EndPoint.y = pb.y >> ONE_SHIFT;
        }
        let mut turn = first;
        loop {
            turn = turn.wrapping_add(1);
            let (x, y) = next_velocity(&mut va, 1);
            pa.x = pa.x.wrapping_add(x);
            pa.y = pa.y.wrapping_add(y);
            let (x, y) = next_velocity(&mut vb, 1);
            pb.x = pb.x.wrapping_add(x);
            pb.y = pb.y.wrapping_add(y);
            ca.IntersectStamp.origin = ca.EndPoint;
            ca.EndPoint.x = pa.x >> ONE_SHIFT;
            ca.EndPoint.y = pa.y >> ONE_SHIFT;
            cb.IntersectStamp.origin = cb.EndPoint;
            cb.EndPoint.x = pb.x >> ONE_SHIFT;
            cb.EndPoint.y = pb.y >> ONE_SHIFT;
            let hit = intersect(&mut ca, &mut cb, MAX_TIME_VALUE as u16);
            if hit != 0 {
                if hit == 1 && turn == 1 && (a.flags | b.flags) & APPEARING as u16 != 0 {
                    ca.EndPoint.x = pa.x >> ONE_SHIFT;
                    ca.EndPoint.y = pa.y >> ONE_SHIFT;
                    cb.EndPoint.x = pb.x >> ONE_SHIFT;
                    cb.EndPoint.y = pb.y >> ONE_SHIFT;
                } else {
                    return turn as u16;
                }
            }
            if turn >= last {
                break;
            }
        }
        0
    }
}
#[cfg(not(test))]
pub(crate) use intercept::plot_intercept;

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
        ship.next.image.frame = cx.advance_frame(ship.current.image.frame);
    } else {
        cx.ship_state_mut(starship).input |= LEFT as u8;
        f -= 1;
        ship.next.image.frame = cx.previous_frame(ship.current.image.frame);
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
        crate::battle::ship_native::thrust(
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
    let mut moved = *ship;
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
                cx.element(id).next.image.frame,
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
    dx = crate::ships::native::wrap_delta(
        dx,
        (cx.screen_dimensions().0 - 64) << (ONE_SHIFT + MAX_REDUCTION),
    );
    dy = crate::ships::native::wrap_delta(
        dy,
        cx.screen_dimensions().1 << (ONE_SHIFT + MAX_REDUCTION),
    );
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
    dx = crate::ships::native::wrap_delta(
        dx,
        (cx.screen_dimensions().0 - 64) << (ONE_SHIFT + MAX_REDUCTION),
    );
    dy = crate::ships::native::wrap_delta(
        dy,
        cx.screen_dimensions().1 << (ONE_SHIFT + MAX_REDUCTION),
    );
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
pub(crate) fn tactical(cx: &mut Cx<'_>, owner: ShipId) -> BATTLE_INPUT_STATE {
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
    }; 5];
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
            dx = crate::ships::native::wrap_delta(
                dx,
                (cx.screen_dimensions().0 - 64) << (ONE_SHIFT + MAX_REDUCTION),
            );
            dy = crate::ships::native::wrap_delta(
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
                let rect = frame_rect(frame.raw());
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
        objects: concerns[..4].try_into().unwrap(),
        subject: ship_copy,
        count: FIRST_EMPTY_INDEX as u16,
    };
    cx.race_intelligence_with_junk(owner, &mut evaluation, concerns[4]);
    cx.ship_state_mut(owner).facing = original_facing;
    crate::battle::process_native::dispatch::battle_input(cx.ship_state(owner).input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_window_handles_stationary_and_relative_motion() {
        assert_eq!(
            intercept_window((0, 0), (0, 0), (0, 0), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((0, 0))
        );
        assert_eq!(
            intercept_window((0, 0), (640, 0), (50, 0), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((2, 2))
        );
        assert_eq!(
            intercept_window((0, 0), (0, 0), (100, 100), (0, 0), 10, [(1, 1), (1, 1)]),
            None
        );
    }

    #[test]
    fn intercept_requires_overlapping_time_windows_on_both_axes() {
        assert_eq!(
            intercept_window((0, 0), (320, 640), (100, 100), (0, 0), 20, [(1, 1), (1, 1)]),
            None
        );
        assert_eq!(
            intercept_window((0, 0), (320, 320), (100, 100), (0, 0), 20, [(1, 1), (1, 1)]),
            Some((10, 10))
        );
    }

    #[test]
    fn intercept_handles_negative_motion_and_one_stationary_axis() {
        for (position, velocity) in [(100, 320), (-100, -320)] {
            assert_eq!(
                intercept_window(
                    (0, 0),
                    (velocity, 0),
                    (position, 0),
                    (-velocity, 0),
                    10,
                    [(1, 1), (1, 1)]
                ),
                Some((5, 5))
            );
        }
        assert_eq!(
            intercept_window((0, 0), (0, 320), (0, 50), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((5, 5))
        );
    }

    #[test]
    fn intercept_rejects_receding_objects_and_preserves_initial_overlap() {
        assert_eq!(
            intercept_window((0, 0), (-320, 0), (100, 0), (320, 0), 10, [(1, 1), (1, 1)]),
            None
        );
        assert_eq!(
            intercept_window((0, 0), (0, 0), (3, 3), (0, 0), 0, [(4, 4), (4, 4)]),
            Some((0, 0))
        );
    }

    #[test]
    fn intercept_narrows_turn_endpoints_to_signed_size_before_comparing() {
        assert_eq!(
            intercept_window((0, 0), (16, 0), (19000, 0), (0, 0), 40000, [(1, 1), (1, 1)]),
            Some((-27536, -27536))
        );
    }

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

    #[test]
    fn conversion_widths_follow_signed_size_and_unsigned_count() {
        assert_eq!(c_size(32768), i16::MIN);

        assert_eq!(
            velocity_to_world(i64::from(i16::MAX) * 64),
            (i32::from(i16::MAX) * 2) as i16
        );
        assert_eq!(normalize_angle(-1), 63);
        assert_eq!(facing_to_angle(15), 60);
    }
}
