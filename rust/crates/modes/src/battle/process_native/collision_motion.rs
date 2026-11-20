//! Fixed-width collision response on borrowed live ABI records.

use super::*;

fn sine(angle: i32, magnitude: i32) -> SIZE {
    let value = crate::math::sinetab[(angle & (FULL_CIRCLE as i32 - 1)) as usize];
    ((i64::from(value) * i64::from(magnitude)) >> SIN_SHIFT) as SIZE
}

fn cosine(angle: i32, magnitude: i32) -> SIZE {
    sine(angle + QUADRANT as i32, magnitude)
}

pub(super) struct Motion<'a> {
    pub flags: &'a mut ELEMENT_FLAGS,
    pub velocity: &'a mut VELOCITY_DESC,
}

pub(super) fn impulse(
    a: &mut Motion<'_>,
    b: &mut Motion<'_>,
    locations: [(POINT, POINT); 2],
    masses: [BYTE; 2],
) -> (i64, [SIZE; 2]) {
    let [(current_a, next_a), (current_b, next_b)] = locations;
    let dx = next_a.x.wrapping_sub(next_b.x);
    let dy = next_a.y.wrapping_sub(next_b.y);
    let mut impact_a = ARCTAN(dx, dy) as SIZE;
    let mut impact_b =
        ((i32::from(impact_a) + HALF_CIRCLE as i32) & (FULL_CIRCLE as i32 - 1)) as SIZE;
    let (dx0, dy0) = a.velocity.get_current_components();
    let (dx1, dy1) = b.velocity.get_current_components();
    let travel_a = a.velocity.travel_angle as SIZE;
    let travel_b = b.velocity.travel_angle as SIZE;
    let rel_x = (dx0 as SIZE).wrapping_sub(dx1 as SIZE);
    let rel_y = (dy0 as SIZE).wrapping_sub(dy1 as SIZE);
    let relative_angle = ARCTAN(rel_x, rel_y) as SIZE;
    let speed = square_root(
        (i64::from(rel_x) * i64::from(rel_x) + i64::from(rel_y) * i64::from(rel_y)) as DWORD,
    ) as SIZE;
    let mut directness =
        ((i32::from(relative_angle) - i32::from(impact_a)) & (FULL_CIRCLE as i32 - 1)) as SIZE;
    if i32::from(directness) <= QUADRANT as i32
        || i32::from(directness) >= (HALF_CIRCLE + QUADRANT) as i32
    {
        directness = HALF_CIRCLE as SIZE;
        impact_a = (i32::from(travel_a) + HALF_CIRCLE as i32) as SIZE;
        impact_b = (i32::from(travel_b) + HALF_CIRCLE as i32) as SIZE;
    }
    if next_a.x == current_a.x
        && next_a.y == current_a.y
        && next_b.x == current_b.x
        && next_b.y == current_b.y
    {
        if *a.flags & *b.flags & DEFY_PHYSICS as ELEMENT_FLAGS != 0 {
            impact_a = (i32::from(travel_a) + (HALF_CIRCLE - OCTANT) as i32) as SIZE;
            impact_b = (i32::from(travel_b) + (HALF_CIRCLE - OCTANT) as i32) as SIZE;
            a.velocity.zero();
            b.velocity.zero();
        }
        *a.flags |= (DEFY_PHYSICS | COLLISION) as ELEMENT_FLAGS;
        *b.flags |= (DEFY_PHYSICS | COLLISION) as ELEMENT_FLAGS;
    }
    let scalar = i64::from(sine(i32::from(directness), i32::from(speed) << 1))
        * i64::from(i32::from(masses[0]) * i32::from(masses[1]));
    (scalar, [impact_a, impact_b])
}

pub(super) fn movable(mass: BYTE) -> bool {
    i32::from(mass) < (MAX_SHIP_MASS * 10) as i32
}

pub(super) fn respond(
    e: &mut Motion<'_>,
    mass: BYTE,
    total_mass: SIZE,
    angle: SIZE,
    scalar: i64,
    waits: &mut (BYTE, BYTE),
) {
    if *e.flags & PLAYER_SHIP as ELEMENT_FLAGS != 0 && *e.flags & DEFY_PHYSICS as ELEMENT_FLAGS == 0
    {
        if waits.0 < 1 {
            waits.0 = waits.0.wrapping_add(1);
        }
        if waits.1 < 3 {
            waits.1 = waits.1.wrapping_add(3);
        }
    }
    let speed = (scalar / (i64::from(mass) * i64::from(total_mass))) as SIZE;
    // The ABI getters and setters narrow at SIZE assignments, not in the velocity model.
    let (dx, dy) = e.velocity.get_current_components();
    let dx = (dx as SIZE).wrapping_add(cosine(i32::from(angle), i32::from(speed)));
    let dy = (dy as SIZE).wrapping_add(sine(i32::from(angle), i32::from(speed)));
    e.velocity.set_components(i32::from(dx), i32::from(dy));
    let (dx, dy) = e.velocity.get_current_components();
    let dx = (dx as SIZE).wrapping_abs();
    let dy = (dy as SIZE).wrapping_abs();
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
