//! Fixed-width collision response on borrowed live ABI records.

use super::*;
use crate::engine::battle::ai_constants::MAX_SHIP_MASS;
use crate::graphics::geometry::Point;
use crate::math::battle_types::{FULL_CIRCLE, HALF_CIRCLE, OCTANT, QUADRANT, SIN_SHIFT};
use crate::math::velocity::{VelocityDesc, VELOCITY_SHIFT};
use crate::math::{arctan as ARCTAN, square_root};

fn sine(angle: i32, magnitude: i32) -> i16 {
    let value = crate::math::SINE_TABLE[(angle & (FULL_CIRCLE as i32 - 1)) as usize];
    ((i64::from(value) * i64::from(magnitude)) >> SIN_SHIFT) as i16
}

fn cosine(angle: i32, magnitude: i32) -> i16 {
    sine(angle + QUADRANT as i32, magnitude)
}

pub(super) struct Motion<'a> {
    pub flags: &'a mut u16,
    pub velocity: &'a mut VelocityDesc,
}

pub(super) fn impulse(
    a: &mut Motion<'_>,
    b: &mut Motion<'_>,
    locations: [(Point, Point); 2],
    masses: [u8; 2],
) -> (i64, [i16; 2]) {
    let [(current_a, next_a), (current_b, next_b)] = locations;
    let dx = next_a.x.wrapping_sub(next_b.x);
    let dy = next_a.y.wrapping_sub(next_b.y);
    let mut impact_a = ARCTAN(dx, dy) as i16;
    let mut impact_b =
        ((i32::from(impact_a) + HALF_CIRCLE as i32) & (FULL_CIRCLE as i32 - 1)) as i16;
    let (dx0, dy0) = a.velocity.get_current_components();
    let (dx1, dy1) = b.velocity.get_current_components();
    let travel_a = a.velocity.travel_angle as i16;
    let travel_b = b.velocity.travel_angle as i16;
    let rel_x = (dx0 as i16).wrapping_sub(dx1 as i16);
    let rel_y = (dy0 as i16).wrapping_sub(dy1 as i16);
    let relative_angle = ARCTAN(rel_x, rel_y) as i16;
    let speed = square_root(
        (i64::from(rel_x) * i64::from(rel_x) + i64::from(rel_y) * i64::from(rel_y)) as u32,
    ) as i16;
    let mut directness =
        ((i32::from(relative_angle) - i32::from(impact_a)) & (FULL_CIRCLE as i32 - 1)) as i16;
    if i32::from(directness) <= QUADRANT as i32
        || i32::from(directness) >= (HALF_CIRCLE + QUADRANT) as i32
    {
        directness = HALF_CIRCLE as i16;
        impact_a = (i32::from(travel_a) + HALF_CIRCLE as i32) as i16;
        impact_b = (i32::from(travel_b) + HALF_CIRCLE as i32) as i16;
    }
    if next_a.x == current_a.x
        && next_a.y == current_a.y
        && next_b.x == current_b.x
        && next_b.y == current_b.y
    {
        if *a.flags & *b.flags & DEFY_PHYSICS as u16 != 0 {
            impact_a = (i32::from(travel_a) + (HALF_CIRCLE - OCTANT) as i32) as i16;
            impact_b = (i32::from(travel_b) + (HALF_CIRCLE - OCTANT) as i32) as i16;
            a.velocity.zero();
            b.velocity.zero();
        }
        *a.flags |= (DEFY_PHYSICS | COLLISION) as u16;
        *b.flags |= (DEFY_PHYSICS | COLLISION) as u16;
    }
    let scalar = i64::from(sine(i32::from(directness), i32::from(speed) << 1))
        * i64::from(i32::from(masses[0]) * i32::from(masses[1]));
    (scalar, [impact_a, impact_b])
}

pub(super) fn movable(mass: u8) -> bool {
    i32::from(mass) < (MAX_SHIP_MASS * 10) as i32
}

pub(super) fn respond(
    e: &mut Motion<'_>,
    mass: u8,
    total_mass: i16,
    angle: i16,
    scalar: i64,
    waits: &mut (u8, u8),
) {
    if *e.flags & PLAYER_SHIP as u16 != 0 && *e.flags & DEFY_PHYSICS as u16 == 0 {
        if waits.0 < 1 {
            waits.0 = waits.0.wrapping_add(1);
        }
        if waits.1 < 3 {
            waits.1 = waits.1.wrapping_add(3);
        }
    }
    let speed = (scalar / (i64::from(mass) * i64::from(total_mass))) as i16;
    // The ABI getters and setters narrow at SIZE assignments, not in the velocity model.
    let (dx, dy) = e.velocity.get_current_components();
    let dx = (dx as i16).wrapping_add(cosine(i32::from(angle), i32::from(speed)));
    let dy = (dy as i16).wrapping_add(sine(i32::from(angle), i32::from(speed)));
    e.velocity.set_components(i32::from(dx), i32::from(dy));
    let (dx, dy) = e.velocity.get_current_components();
    let dx = (dx as i16).wrapping_abs();
    let dy = (dy as i16).wrapping_abs();
    if (i32::from(dx) + i32::from(dy)) >> VELOCITY_SHIFT < 1 << ONE_SHIFT {
        let minimum = ((1 << ONE_SHIFT) << VELOCITY_SHIFT) - 1;
        e.velocity.set_components(
            i32::from(cosine(i32::from(angle), minimum)),
            i32::from(sine(i32::from(angle), minimum)),
        );
    }
}

#[cfg(test)]
mod tests;
