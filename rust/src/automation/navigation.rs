//! Pure controller for driving the flagship through real interplanetary flight.

/// Real navigation state sampled from the running solar-system simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationObservation {
    pub active: bool,
    pub inner_planet: Option<u8>,
    pub in_orbit: bool,
    pub ship_x: i32,
    pub ship_y: i32,
    pub ship_facing: u8,
    pub target_x: i32,
    pub target_y: i32,
    pub velocity_x: i32,
    pub velocity_y: i32,
    /// Centre of the SIS view, i.e. `SIS_SCREEN_WIDTH >> 1` and
    /// `SIS_SCREEN_HEIGHT >> 1`. These depend on the runtime screen size, so
    /// they are sampled rather than assumed.
    pub view_center_x: i32,
    pub view_center_y: i32,
    /// The collision guard (`WaitIntersect`) currently names the moon being
    /// navigated to. `CheckIntersect` keeps it until the ship stops
    /// intersecting any body, so the guard being set while the ship has not
    /// entered orbit means the moon cannot be entered from where the ship is.
    pub target_guard_active: bool,
}

/// Player-one controls to apply for one admitted input callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavigationControl {
    pub thrust: bool,
    pub left: bool,
    pub right: bool,
    /// Leave the current orbit. The orbital screen is a menu, not ship
    /// flight, so this is driven with menu keys rather than a ship control.
    pub leave_orbit: bool,
}

/// Display-space radius, squared, within which the flagship is treated as
/// having arrived and stops steering. The flagship image is several pixels
/// across, so its collision rectangle already overlaps the target here.
const ARRIVAL_RADIUS_SQUARED: f64 = 64.0;

/// Compute a pure-pursuit steering command using UQM's sixteen facings.
#[must_use]
pub fn steer_toward_target(observation: NavigationObservation) -> NavigationControl {
    if !observation.active {
        return NavigationControl::default();
    }
    if observation.in_orbit {
        return NavigationControl {
            leave_orbit: true,
            ..NavigationControl::default()
        };
    }
    let (target_x, target_y) = if observation.inner_planet.is_some() {
        let outward_x = observation.ship_x - observation.view_center_x;
        let outward_y = observation.ship_y - observation.view_center_y;
        if outward_x == 0 && outward_y == 0 {
            (observation.view_center_x, -1)
        } else {
            (
                observation.ship_x + outward_x,
                observation.ship_y + outward_y,
            )
        }
    } else {
        (observation.target_x, observation.target_y)
    };

    let dx = f64::from(target_x - observation.ship_x);
    let dy = f64::from(target_y - observation.ship_y);
    let distance_squared = dx.mul_add(dx, dy * dy);
    if distance_squared <= ARRIVAL_RADIUS_SQUARED {
        return NavigationControl::default();
    }

    if observation.inner_planet.is_some() {
        return pursue_heading(dx, dy, observation.ship_facing);
    }
    let distance = distance_squared.sqrt();
    steer_to_velocity(
        dx * MOON_APPROACH_SPEED / distance,
        dy * MOON_APPROACH_SPEED / distance,
        observation,
    )
}

/// Recover the target's inner system before applying inertial orbital steering.
#[must_use]
pub fn steer_planet_orbit(mut observation: NavigationObservation, planet: u8) -> NavigationControl {
    if observation.inner_planet == Some(planet) {
        observation.inner_planet = None;
        steer_moon_navigation(observation)
    } else {
        steer_toward_target(observation)
    }
}

/// Turn toward a display-space direction and thrust once roughly aligned.
fn pursue_heading(dx: f64, dy: f64, ship_facing: u8) -> NavigationControl {
    let full_circle = std::f64::consts::TAU;
    let angle = dx.atan2(-dy).rem_euclid(full_circle);
    let desired = ((angle * 16.0 / full_circle).round() as i32).rem_euclid(16);
    let current = i32::from(ship_facing & 0x0f);
    let clockwise = (desired - current).rem_euclid(16);
    let turn_error = clockwise.min(16 - clockwise);

    NavigationControl {
        thrust: turn_error <= 2,
        left: clockwise > 8,
        right: clockwise != 0 && clockwise <= 8,
        leave_orbit: false,
    }
}

/// Trace label recorded when `navigate_to_hyperspace` observes the real
/// transition into hyperspace.
pub const HYPERSPACE_REACHED_LABEL: &str = "navigation_reached:hyperspace";

/// Low byte of `CurrentActivity` while the hyperspace loop runs
/// (`IN_HYPERSPACE` in `globdata.h`).
const IN_HYPERSPACE: u16 = 3;

/// Fly the flagship straight away from the centre of the current view.
///
/// Both the outer system (centred on the sun) and an inner system (centred on
/// its planet) are drawn around the view centre, so flying outward first
/// leaves any inner system, then crosses each outer zoom level until
/// production `CheckShipLocation` sees the ship leave the screen at maximum
/// zoom and ends interplanetary flight. No position or route is encoded.
#[must_use]
pub fn steer_out_of_system(observation: NavigationObservation) -> NavigationControl {
    if !observation.active {
        return NavigationControl::default();
    }
    if observation.in_orbit {
        return NavigationControl {
            leave_orbit: true,
            ..NavigationControl::default()
        };
    }
    let outward_x = observation.ship_x - observation.view_center_x;
    let outward_y = observation.ship_y - observation.view_center_y;
    let (dx, dy) = if outward_x == 0 && outward_y == 0 {
        (0.0, -1.0)
    } else {
        (f64::from(outward_x), f64::from(outward_y))
    };
    pursue_heading(dx, dy, observation.ship_facing)
}

/// Whether a `navigate_to_hyperspace` step has seen production enter
/// hyperspace.
///
/// `flight_observed` must record that this step itself sampled live
/// interplanetary flight. Arrival then requires that the solar-system state
/// is gone and the main loop dispatched the hyperspace activity, so a step
/// that starts anywhere other than interplanetary flight can never succeed.
#[must_use]
pub const fn hyperspace_reached(
    flight_observed: bool,
    solar_system_active: bool,
    activity_word: u16,
) -> bool {
    flight_observed && !solar_system_active && activity_word & 0x00ff == IN_HYPERSPACE
}

/// `CheckIntersect` guard for a planet in its inner-system view.
#[must_use]
pub const fn planet_wait_intersect(planet: u8) -> u16 {
    (planet as u16 + 1) | (1 << 8)
}

/// `MAKE_WORD (planetOffset, moonOffset)`, with one-based planets and
/// two-based moons (moon offset 1 denotes the planet itself).
#[must_use]
pub const fn moon_wait_intersect(planet: u8, moon: u8) -> u16 {
    (planet as u16 + 1) | ((moon as u16 + 2) << 8)
}

/// Approach speed held while closing on a moon.
///
/// The flagship only turns one sixteenth of a circle per `turn_wait` frames,
/// so a bounded approach speed keeps the turn radius small enough to correct
/// heading before overshooting a moon that is only a few pixels wide.
const MOON_APPROACH_SPEED: f64 = 16.0;

/// Squared correction magnitude below which thrust would overshoot the
/// desired velocity rather than converge on it.
const VELOCITY_DEADBAND_SQUARED: f64 = 4.0;

/// Steer toward a moon by driving the flagship's velocity vector, not just its
/// facing.
///
/// The flagship is inertial: `flagship_inertial_thrust` adds to an existing
/// velocity, so pure-pursuit steering (which only aims the nose) accumulates
/// lateral drift and orbits the target instead of intersecting it. This
/// controller instead computes the velocity error between the current velocity
/// and a bounded approach velocity, and thrusts only while pointing along that
/// error. That converges on the target from any starting geometry without
/// encoding any position-specific route.
#[must_use]
pub fn steer_moon_navigation(observation: NavigationObservation) -> NavigationControl {
    if !observation.active {
        return NavigationControl::default();
    }
    if observation.in_orbit {
        return NavigationControl {
            leave_orbit: true,
            ..NavigationControl::default()
        };
    }

    let dx = f64::from(observation.target_x - observation.ship_x);
    let dy = f64::from(observation.target_y - observation.ship_y);
    let distance = dx.hypot(dy);

    if observation.target_guard_active {
        return steer_out_of_target(observation);
    }

    // Sitting exactly on the target with no guard held is a transient: the
    // next frame either commits the orbit or sets the guard.
    if distance == 0.0 {
        return NavigationControl::default();
    }

    steer_to_velocity(
        dx * MOON_APPROACH_SPEED / distance,
        dy * MOON_APPROACH_SPEED / distance,
        observation,
    )
}

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
    observation: NavigationObservation,
    target_planet: u8,
) -> NavigationControl {
    if observation.inner_planet == Some(target_planet) {
        steer_moon_navigation(observation)
    } else {
        steer_toward_target(observation)
    }
}

/// Speed held while leaving a moon whose collision guard is still set.
const MOON_BACKOFF_SPEED: f64 = 8.0;

/// Fly out of a moon that `CheckIntersect` is suppressing.
///
/// The guard that keeps a just-left moon from being entered again is released
/// only once the ship no longer intersects any body, so a ship that stops on
/// the moon (the Starbase puts the ship back there when it is left) stays
/// locked out. Backing away is what a player does. The heading is directly
/// away from the moon; a ship exactly on it leaves along the line from the
/// view centre, since moons orbit the centre.
fn steer_out_of_target(observation: NavigationObservation) -> NavigationControl {
    let mut away_x = f64::from(observation.ship_x - observation.target_x);
    let mut away_y = f64::from(observation.ship_y - observation.target_y);
    if away_x == 0.0 && away_y == 0.0 {
        away_x = f64::from(observation.ship_x - observation.view_center_x);
        away_y = f64::from(observation.ship_y - observation.view_center_y);
    }
    if away_x == 0.0 && away_y == 0.0 {
        away_y = -1.0;
    }
    let length = away_x.hypot(away_y);
    steer_to_velocity(
        away_x * MOON_BACKOFF_SPEED / length,
        away_y * MOON_BACKOFF_SPEED / length,
        observation,
    )
}

/// Steer so that thrust moves the current velocity toward `desired`.
fn steer_to_velocity(
    desired_velocity_x: f64,
    desired_velocity_y: f64,
    observation: NavigationObservation,
) -> NavigationControl {
    let correction_x = desired_velocity_x - f64::from(observation.velocity_x);
    let correction_y = desired_velocity_y - f64::from(observation.velocity_y);
    let correction_magnitude_squared =
        correction_x.mul_add(correction_x, correction_y * correction_y);

    steering_for_vector(
        correction_x,
        correction_y,
        observation.ship_facing,
        correction_magnitude_squared > VELOCITY_DEADBAND_SQUARED,
    )
}

fn steering_for_vector(dx: f64, dy: f64, ship_facing: u8, thrust: bool) -> NavigationControl {
    let full_circle = std::f64::consts::TAU;
    let angle = dx.atan2(-dy).rem_euclid(full_circle);
    let desired = ((angle * 16.0 / full_circle).round() as i32).rem_euclid(16);
    let current = i32::from(ship_facing & 0x0f);
    let clockwise = (desired - current).rem_euclid(16);

    NavigationControl {
        thrust: thrust && clockwise == 0,
        left: clockwise > 8,
        right: clockwise != 0 && clockwise <= 8,
        leave_orbit: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        ship_x: i32,
        ship_y: i32,
        facing: u8,
        target_x: i32,
        target_y: i32,
    ) -> NavigationObservation {
        NavigationObservation {
            active: true,
            inner_planet: None,
            in_orbit: false,
            ship_x,
            ship_y,
            ship_facing: facing,
            target_x,
            target_y,
            velocity_x: 0,
            velocity_y: 0,
            // Matches the 320x240 SIS view: SIS_SCREEN_WIDTH/HEIGHT are
            // 242x227, so the centre is (121, 113).
            view_center_x: 121,
            view_center_y: 113,
            target_guard_active: false,
        }
    }

    #[test]
    fn outer_approach_cancels_lateral_drift_before_accelerating_toward_planet() {
        let mut obs = observation(100, 100, 0, 100, 20);
        obs.velocity_x = 120;
        let control = steer_toward_target(obs);
        assert!(
            control.left,
            "a north-facing ship drifting east must brake west, not thrust north"
        );
        assert!(!control.thrust);
    }

    #[test]
    fn orbit_reapproaches_from_outer_flight_after_an_encounter() {
        let mut outer = observation(120, 160, 0, 100, 100);
        outer.velocity_x = 30;
        assert_eq!(steer_planet_orbit(outer, 0), steer_toward_target(outer));
        let mut wrong_inner = outer;
        wrong_inner.inner_planet = Some(1);
        assert_eq!(
            steer_planet_orbit(wrong_inner, 0),
            steer_toward_target(wrong_inner)
        );
        let mut inner = outer;
        inner.inner_planet = Some(0);
        let mut orbital = inner;
        orbital.inner_planet = None;
        assert_eq!(steer_planet_orbit(inner, 0), steer_moon_navigation(orbital));
    }

    #[test]
    fn thrusts_when_target_is_ahead() {
        assert_eq!(
            steer_toward_target(observation(100, 200, 0, 100, 100)),
            NavigationControl {
                thrust: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    fn turns_left_toward_target_on_the_left() {
        let control = steer_toward_target(observation(100, 100, 0, 0, 100));
        assert!(control.left);
        assert!(!control.right);
    }

    #[test]
    fn coasts_when_close_enough_to_intersect_the_target() {
        // 8 display pixels away: the flagship image already overlaps.
        assert_eq!(
            steer_toward_target(observation(120, 91, 4, 128, 91)),
            NavigationControl::default()
        );
        // 9 display pixels away: still steering.
        assert!(steer_toward_target(observation(119, 91, 4, 128, 91)).thrust);
    }

    #[test]
    fn turns_right_toward_target_on_the_right() {
        let control = steer_toward_target(observation(100, 100, 0, 200, 100));
        assert!(control.right);
        assert!(!control.left);
    }

    #[test]
    fn flies_outward_after_entering_the_wrong_inner_system() {
        let mut state = observation(128, 150, 8, 100, 100);
        state.inner_planet = Some(2);
        assert_eq!(
            steer_toward_target(state),
            NavigationControl {
                thrust: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    fn leaves_current_orbit_before_steering() {
        let mut state = observation(100, 100, 0, 200, 100);
        state.inner_planet = Some(2);
        state.in_orbit = true;
        assert_eq!(
            steer_toward_target(state),
            NavigationControl {
                leave_orbit: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    /// A fast closing ship must turn toward its braking vector before
    /// thrusting, rather than add forward speed and miss the planet.
    fn generic_navigation_brakes_instead_of_accelerating_past_the_target() {
        let mut state = observation(100, 100, 4, 120, 100);
        state.velocity_x = 200;
        let control = steer_toward_target(state);
        assert!(!control.thrust);
        assert!(!control.left);
        assert!(control.right);
    }

    #[test]
    fn departure_leaves_orbit_before_flying() {
        let mut state = observation(100, 100, 0, 0, 0);
        state.in_orbit = true;
        assert_eq!(
            steer_out_of_system(state),
            NavigationControl {
                leave_orbit: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    fn departure_is_inert_without_a_live_solar_system() {
        let mut state = observation(10, 10, 0, 0, 0);
        state.active = false;
        assert_eq!(steer_out_of_system(state), NavigationControl::default());
    }

    #[test]
    fn departure_thrusts_when_already_facing_away_from_the_centre() {
        // Directly above the centre (121, 113) and facing up (0).
        assert_eq!(
            steer_out_of_system(observation(121, 40, 0, 0, 0)),
            NavigationControl {
                thrust: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    fn departure_turns_away_from_the_centre_before_thrusting() {
        // Right of the centre but facing left (12): outward is facing 4.
        let control = steer_out_of_system(observation(200, 113, 12, 0, 0));
        assert!(!control.thrust);
        assert!(control.left ^ control.right);
    }

    #[test]
    fn departure_picks_a_direction_at_the_exact_centre() {
        assert!(steer_out_of_system(observation(121, 113, 0, 0, 0)).thrust);
    }

    #[test]
    fn hyperspace_requires_observed_flight_then_torn_down_system_and_activity() {
        // IN_HYPERSPACE low byte with high-byte flags still counts.
        assert!(hyperspace_reached(true, false, 0x0003));
        assert!(hyperspace_reached(true, false, 0x0403));
        // Never flew out during this step.
        assert!(!hyperspace_reached(false, false, 0x0003));
        // Solar system still live, or still interplanetary.
        assert!(!hyperspace_reached(true, true, 0x0003));
        assert!(!hyperspace_reached(true, false, 0x0004));
        // Load dialog / encounter words are not hyperspace.
        assert!(!hyperspace_reached(true, false, 0x1004));
        assert!(!hyperspace_reached(true, false, 0x0002));
    }

    /// Closed-loop property: from any start inside the view and any initial
    /// drift, outward steering leaves the screen rectangle, which is what
    /// makes production `CheckShipLocation` end interplanetary flight.
    #[test]
    fn departure_leaves_the_view_from_any_starting_geometry() {
        const TURN_WAIT: u32 = 2;
        const THRUST_INCREMENT: f64 = 6.0;
        const MAX_SPEED: f64 = 60.0;
        const WIDTH: f64 = 242.0;
        const HEIGHT: f64 = 227.0;

        let starts = [
            (121, 113, 0, 0.0, 0.0),
            (130, 120, 8, 0.0, 0.0),
            (30, 30, 4, 20.0, -20.0),
            (200, 200, 12, -40.0, 0.0),
            (121, 20, 8, 0.0, 50.0),
            (5, 113, 4, 60.0, 0.0),
            (160, 150, 2, -30.0, -30.0),
        ];
        for (start_x, start_y, start_facing, start_vx, start_vy) in starts {
            let mut x = f64::from(start_x);
            let mut y = f64::from(start_y);
            let mut velocity_x: f64 = start_vx;
            let mut velocity_y: f64 = start_vy;
            let mut facing: i32 = start_facing;
            let mut turn_counter: u32 = 0;
            let mut left_view = false;
            for _ in 0..2000 {
                if !(0.0..WIDTH).contains(&x) || !(0.0..HEIGHT).contains(&y) {
                    left_view = true;
                    break;
                }
                let control = steer_out_of_system(NavigationObservation {
                    ship_x: x.round() as i32,
                    ship_y: y.round() as i32,
                    ship_facing: facing as u8,
                    ..observation(0, 0, 0, 0, 0)
                });
                assert!(!control.leave_orbit);
                if turn_counter > 0 {
                    turn_counter -= 1;
                } else if control.left || control.right {
                    facing = (facing + if control.right { 1 } else { -1 }).rem_euclid(16);
                    turn_counter = TURN_WAIT;
                }
                if control.thrust {
                    let angle = f64::from(facing) * std::f64::consts::TAU / 16.0
                        - std::f64::consts::FRAC_PI_2;
                    velocity_x += angle.cos() * THRUST_INCREMENT;
                    velocity_y += angle.sin() * THRUST_INCREMENT;
                    let speed = velocity_x.hypot(velocity_y);
                    if speed > MAX_SPEED {
                        velocity_x = velocity_x * MAX_SPEED / speed;
                        velocity_y = velocity_y * MAX_SPEED / speed;
                    }
                }
                x += velocity_x / 16.0;
                y += velocity_y / 16.0;
            }
            assert!(
                left_view,
                "departure failed to leave the view from ({start_x}, {start_y})"
            );
        }
    }

    #[test]
    fn a_guarded_planet_is_left_before_reapproaching_it() {
        assert_eq!(planet_wait_intersect(0), 0x0101);
        assert_eq!(planet_wait_intersect(8), 0x0109);
        let mut state = observation(120, 100, 0, 120, 100);
        state.view_center_x = 120;
        state.view_center_y = 100;
        assert_eq!(steer_moon_navigation(state), NavigationControl::default());
        state.target_guard_active = true;
        let backing_off = steer_moon_navigation(state);
        assert!(backing_off.thrust || backing_off.left || backing_off.right);
    }

    #[test]
    fn moon_wait_intersect_matches_c_make_word_encoding() {
        // MAKE_WORD (planetOffset=3, moonOffset=2) for Earth (index 2), moon 0.
        assert_eq!(moon_wait_intersect(2, 0), 0x0203);
        // Earth's moon 1 (Luna), the moon that carries the moon base.
        assert_eq!(moon_wait_intersect(2, 1), 0x0303);
    }

    #[test]
    fn moon_controller_thrusts_toward_a_stationary_distant_target() {
        // Target directly above (facing 0 is up), no velocity: the whole
        // correction is "go up", which the ship is already aligned with.
        let control = steer_moon_navigation(observation(100, 200, 0, 100, 100));
        assert_eq!(
            control,
            NavigationControl {
                thrust: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    fn moon_controller_turns_before_thrusting_when_misaligned() {
        // Target is up but the ship points down (facing 8): it must rotate
        // and must not thrust while the correction is behind it.
        let control = steer_moon_navigation(observation(100, 200, 8, 100, 100));
        assert!(!control.thrust);
        assert!(control.left ^ control.right);
    }

    #[test]
    fn moon_controller_backs_away_when_the_guard_holds_the_moon_under_the_ship() {
        // Leaving the Starbase puts the ship back on the moon it just left and
        // `CheckIntersect` suppresses that moon until the ship stops
        // intersecting it. Standing still never clears the guard, so the ship
        // must fly out of the moon, which lies left of the view centre.
        let mut state = observation(86, 113, 12, 86, 113);
        state.target_guard_active = true;
        assert_eq!(
            steer_moon_navigation(state),
            NavigationControl {
                thrust: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    fn moon_controller_turns_away_from_the_view_centre_before_backing_off() {
        let mut state = observation(86, 113, 4, 86, 113);
        state.target_guard_active = true;
        let control = steer_moon_navigation(state);
        assert!(!control.thrust, "must not thrust into the view centre");
        assert!(control.left ^ control.right);
    }

    #[test]
    fn moon_controller_backs_away_along_the_line_from_the_target() {
        // Off-centre inside the guard: leave along target -> ship.
        let mut state = observation(86, 119, 8, 86, 113);
        state.target_guard_active = true;
        assert!(steer_moon_navigation(state).thrust);
    }

    /// The guard holds from the moment a collision is ignored until the ship
    /// clears the moon image; only then can the moon be entered again.
    #[test]
    fn moon_controller_re_enters_a_moon_whose_guard_holds_it_under_the_ship() {
        const TURN_WAIT: u32 = 2;
        const THRUST_INCREMENT: f64 = 6.0;
        const MAX_SPEED: f64 = 60.0;
        const HIT_RADIUS: f64 = 4.0;
        const GUARD_CLEARS_BEYOND: f64 = 8.0;

        let target = (86.0_f64, 113.0_f64);
        for start_facing in 0..16 {
            let mut x = target.0;
            let mut y = target.1;
            let mut velocity_x = 0.0_f64;
            let mut velocity_y = 0.0_f64;
            let mut facing: i32 = start_facing;
            let mut turn_counter: u32 = 0;
            let mut guard_active = true;
            let mut cleared_guard = false;
            let mut hit = false;

            for _ in 0..4000 {
                let distance = (x - target.0).hypot(y - target.1);
                if guard_active && distance > GUARD_CLEARS_BEYOND {
                    guard_active = false;
                    cleared_guard = true;
                }
                if !guard_active && distance <= HIT_RADIUS {
                    hit = true;
                    break;
                }

                let control = steer_moon_navigation(NavigationObservation {
                    ship_x: x.round() as i32,
                    ship_y: y.round() as i32,
                    ship_facing: facing as u8,
                    velocity_x: velocity_x.round() as i32,
                    velocity_y: velocity_y.round() as i32,
                    target_guard_active: guard_active,
                    ..observation(0, 0, 0, target.0 as i32, target.1 as i32)
                });

                if turn_counter > 0 {
                    turn_counter -= 1;
                } else if control.left || control.right {
                    facing = (facing + if control.right { 1 } else { -1 }).rem_euclid(16);
                    turn_counter = TURN_WAIT;
                }

                if control.thrust {
                    let angle = f64::from(facing) * std::f64::consts::TAU / 16.0
                        - std::f64::consts::FRAC_PI_2;
                    velocity_x += angle.cos() * THRUST_INCREMENT;
                    velocity_y += angle.sin() * THRUST_INCREMENT;
                    let speed = velocity_x.hypot(velocity_y);
                    if speed > MAX_SPEED {
                        velocity_x = velocity_x * MAX_SPEED / speed;
                        velocity_y = velocity_y * MAX_SPEED / speed;
                    }
                }

                x += velocity_x / 16.0;
                y += velocity_y / 16.0;
            }

            assert!(
                cleared_guard,
                "ship never left the moon (start facing {start_facing})"
            );
            assert!(
                hit,
                "ship left the moon but never re-entered it (start facing {start_facing})"
            );
        }
    }

    #[test]
    fn moon_controller_coasts_once_travelling_at_approach_speed() {
        // Already moving toward the target at exactly the approach speed:
        // the velocity error is zero, so no thrust is required.
        let mut state = observation(100, 200, 0, 100, 100);
        state.velocity_y = -(MOON_APPROACH_SPEED as i32);
        assert!(!steer_moon_navigation(state).thrust);
    }

    #[test]
    fn moon_controller_brakes_overspeed_approach() {
        // Travelling toward the target far faster than the approach speed.
        // The correction vector points backward, so the controller must turn
        // around to shed speed rather than keep accelerating.
        let mut state = observation(100, 200, 0, 100, 100);
        state.velocity_y = -4 * MOON_APPROACH_SPEED as i32;
        let control = steer_moon_navigation(state);
        assert!(!control.thrust);
        assert!(control.left ^ control.right);
    }

    #[test]
    fn moon_controller_corrects_lateral_drift_rather_than_only_aiming() {
        // Target straight ahead but the ship is drifting sideways. A
        // pure-pursuit controller would thrust straight at the target and
        // keep the drift; this controller must steer against the drift.
        let mut state = observation(100, 200, 0, 100, 100);
        state.velocity_x = 3 * MOON_APPROACH_SPEED as i32;
        let control = steer_moon_navigation(state);
        assert!(control.left, "must steer against rightward drift");
        assert!(!control.thrust);
    }

    #[test]
    fn moon_controller_leaves_wrong_orbit_before_resuming_navigation() {
        let mut state = observation(111, 160, 0, 86, 113);
        state.in_orbit = true;
        assert_eq!(
            steer_moon_navigation(state),
            NavigationControl {
                leave_orbit: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    fn moon_route_leaves_another_planets_inner_system_instead_of_chasing_the_moon_point() {
        // Mercury (0) inner system while Earth (2) is the target planet. The
        // ship sits down-left of the view centre; the reported target is a
        // meaningless point to its right. Facing 4 points at that point, so a
        // moon controller would thrust; the route must instead turn outward.
        let mut state = observation(77, 145, 4, 82, 145);
        state.inner_planet = Some(0);
        let control = steer_moon_route(state, 2);
        assert_eq!(control, steer_toward_target(state));
        assert!(!control.thrust);
        assert!(control.left ^ control.right);
    }

    #[test]
    fn moon_route_thrusts_outward_once_aligned_in_another_planets_inner_system() {
        // Outward from (121, 113) through (77, 145) is facing 10 or 11.
        let mut state = observation(77, 145, 11, 82, 145);
        state.inner_planet = Some(0);
        assert!(steer_moon_route(state, 2).thrust);
    }

    #[test]
    fn moon_route_flies_to_the_planet_from_the_outer_system() {
        // Outer system: the target is the planet's display position.
        let state = observation(100, 200, 0, 100, 100);
        assert_eq!(
            steer_moon_route(state, 2),
            NavigationControl {
                thrust: true,
                ..NavigationControl::default()
            }
        );
        let turning = observation(100, 100, 0, 200, 100);
        assert!(steer_moon_route(turning, 2).right);
    }

    #[test]
    fn moon_route_uses_the_moon_controller_inside_the_target_planets_inner_system() {
        let mut state = observation(100, 200, 0, 100, 100);
        state.inner_planet = Some(2);
        assert_eq!(steer_moon_route(state, 2), steer_moon_navigation(state));
        // Pure pursuit would fly outward from the view centre here; the moon
        // controller closes on the moon instead.
        assert_ne!(steer_moon_route(state, 2), steer_toward_target(state));
        let mut guarded = observation(100, 100, 0, 100, 100);
        guarded.inner_planet = Some(2);
        guarded.target_guard_active = true;
        assert_eq!(steer_moon_route(guarded, 2), steer_moon_navigation(guarded));
    }

    #[test]
    fn moon_route_leaves_a_wrong_orbit_in_any_system() {
        let mut state = observation(111, 160, 0, 86, 113);
        state.in_orbit = true;
        let leave = NavigationControl {
            leave_orbit: true,
            ..NavigationControl::default()
        };
        state.inner_planet = Some(0);
        assert_eq!(steer_moon_route(state, 2), leave);
        state.inner_planet = Some(2);
        assert_eq!(steer_moon_route(state, 2), leave);
    }

    #[test]
    fn moon_controller_is_inert_while_navigation_is_inactive() {
        let mut state = observation(100, 200, 0, 100, 100);
        state.active = false;
        assert_eq!(steer_moon_navigation(state), NavigationControl::default());
    }

    /// Closed-loop convergence: the controller must actually intersect the
    /// target from many starting geometries. This is the property that the
    /// previous coordinate-specific route heuristics only approximated for a
    /// single Earth/Luna layout.
    #[test]
    fn moon_controller_converges_on_the_target_from_any_starting_geometry() {
        // Mirrors ProcessShipControls + flagship_inertial_thrust closely
        // enough to expose steering divergence: one facing step per turn_wait
        // frames, thrust adds to velocity along the facing, speed is capped.
        const TURN_WAIT: u32 = 2;
        const THRUST_INCREMENT: f64 = 6.0;
        const MAX_SPEED: f64 = 60.0;
        const HIT_RADIUS: f64 = 4.0;

        let target = (86.0_f64, 113.0_f64);
        let starts = [
            (147, 193),
            (30, 30),
            (200, 40),
            (86, 20),
            (86, 180),
            (10, 113),
            (220, 113),
            (160, 150),
        ];

        for (start_x, start_y) in starts {
            let mut x = f64::from(start_x);
            let mut y = f64::from(start_y);
            let mut velocity_x = 0.0_f64;
            let mut velocity_y = 0.0_f64;
            let mut facing: i32 = 0;
            let mut turn_counter: u32 = 0;
            let mut hit = false;

            for _ in 0..4000 {
                if (x - target.0).hypot(y - target.1) <= HIT_RADIUS {
                    hit = true;
                    break;
                }

                let control = steer_moon_navigation(NavigationObservation {
                    ship_x: x.round() as i32,
                    ship_y: y.round() as i32,
                    ship_facing: facing as u8,
                    velocity_x: velocity_x.round() as i32,
                    velocity_y: velocity_y.round() as i32,
                    ..observation(0, 0, 0, target.0 as i32, target.1 as i32)
                });

                if turn_counter > 0 {
                    turn_counter -= 1;
                } else if control.left || control.right {
                    facing = (facing + if control.right { 1 } else { -1 }).rem_euclid(16);
                    turn_counter = TURN_WAIT;
                }

                if control.thrust {
                    let angle = f64::from(facing) * std::f64::consts::TAU / 16.0
                        - std::f64::consts::FRAC_PI_2;
                    velocity_x += angle.cos() * THRUST_INCREMENT;
                    velocity_y += angle.sin() * THRUST_INCREMENT;
                    let speed = velocity_x.hypot(velocity_y);
                    if speed > MAX_SPEED {
                        velocity_x = velocity_x * MAX_SPEED / speed;
                        velocity_y = velocity_y * MAX_SPEED / speed;
                    }
                }

                // Display units advance at velocity/SCALED_ONE per frame.
                x += velocity_x / 16.0;
                y += velocity_y / 16.0;
            }

            assert!(
                hit,
                "moon controller failed to reach the target from ({start_x}, {start_y})"
            );
        }
    }
}

/// A real encounter preview or direct greeting must name the requested ship.
#[must_use]
pub fn contact_reached(wanted: u8, actual: Option<u8>, preview: bool, conversation: bool) -> bool {
    (preview || conversation) && actual == Some(wanted)
}

#[cfg(test)]
mod contact_tests {
    #[test]
    fn only_a_live_matching_preview_or_direct_greeting_completes_contact() {
        use super::contact_reached;
        assert!(contact_reached(12, Some(12), true, false));
        assert!(contact_reached(12, Some(12), false, true));
        assert!(!contact_reached(12, Some(12), false, false));
        assert!(!contact_reached(12, Some(8), false, true));
        assert!(!contact_reached(12, None, true, false));
    }
}
