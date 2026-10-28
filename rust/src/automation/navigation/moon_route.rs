use super::{steer_moon_navigation, steer_toward_target, NavigationControl, NavigationObservation};

/// Route toward a moon of `target_planet`, wherever the ship currently is.
///
/// Moons exist only inside their planet's inner system, so the moon controller
/// is only meaningful once the ship is in that inner system. From the outer
/// system, or from another planet's inner system, the ship first flies to the
/// planet exactly as `navigate_to_planet` does: `steer_toward_target` leaves a
/// wrong inner system outward, then closes on the planet in the outer view
/// (whose position the observation reports as the target) to enter its inner
/// system.
#[must_use]
pub fn steer_moon_route(
    mut observation: NavigationObservation,
    target_planet: u8,
    obstacles: &[(i32, i32)],
) -> NavigationControl {
    if observation.inner_planet == Some(target_planet) {
        if !observation.in_orbit && !observation.target_guard_active {
            (observation.target_x, observation.target_y) = detour_target(
                (observation.ship_x, observation.ship_y),
                (observation.target_x, observation.target_y),
                obstacles,
            );
        }
        steer_moon_navigation(observation)
    } else {
        steer_toward_target(observation)
    }
}

fn detour_target(ship: (i32, i32), target: (i32, i32), obstacles: &[(i32, i32)]) -> (i32, i32) {
    let dx = f64::from(target.0 - ship.0);
    let dy = f64::from(target.1 - ship.1);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return target;
    }
    let (ux, uy) = (dx / length, dy / length);
    const CLEARANCE: f64 = 24.0;
    let obstruction = obstacles
        .iter()
        .filter_map(|&(x, y)| {
            let (ox, oy) = (f64::from(x - ship.0), f64::from(y - ship.1));
            let along = ox * ux + oy * uy;
            let across = ox * -uy + oy * ux;
            (along > 0.0 && along < length && across.abs() < CLEARANCE)
                .then_some((along, across, x, y))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let Some((_, across, x, y)) = obstruction else {
        return target;
    };
    let side = if across >= 0.0 { -1.0 } else { 1.0 };
    (
        (f64::from(x) - uy * side * CLEARANCE * 2.0).round() as i32,
        (f64::from(y) + ux * side * CLEARANCE * 2.0).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luna_approach_does_not_pass_through_the_starbase() {
        let ship = (51, 43);
        let target = (111, 160);
        let starbase = (86, 113);
        let waypoint = detour_target(ship, target, &[starbase]);
        assert_ne!(waypoint, target);
        let dx = f64::from(waypoint.0 - ship.0);
        let dy = f64::from(waypoint.1 - ship.1);
        let cross = f64::from(starbase.0 - ship.0) * dy - f64::from(starbase.1 - ship.1) * dx;
        assert!(cross.abs() / dx.hypot(dy) > 16.0);
    }

    #[test]
    fn successive_waypoints_clear_the_other_moon_and_reach_the_target() {
        let mut ship = (51.0_f64, 43.0_f64);
        let target = (111, 160);
        let obstacle = (86, 113);
        for _ in 0..300 {
            assert!((ship.0 - 86.0).hypot(ship.1 - 113.0) > 16.0);
            if (ship.0 - 111.0).hypot(ship.1 - 160.0) < 2.0 {
                return;
            }
            let waypoint = detour_target(
                (ship.0.round() as i32, ship.1.round() as i32),
                target,
                &[obstacle],
            );
            let dx = f64::from(waypoint.0) - ship.0;
            let dy = f64::from(waypoint.1) - ship.1;
            let length = dx.hypot(dy);
            ship = (ship.0 + dx / length, ship.1 + dy / length);
        }
        panic!("route did not reach Luna: {ship:?}");
    }

    #[test]
    fn bodies_behind_the_ship_or_beyond_the_target_do_not_change_the_route() {
        assert_eq!(
            detour_target((0, 0), (100, 0), &[(-20, 0), (120, 0), (50, 40)]),
            (100, 0)
        );
        assert_eq!(detour_target((10, 10), (10, 10), &[(10, 10)]), (10, 10));
    }
}
