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
}

/// Player-one controls to apply for one admitted input callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavigationControl {
    pub thrust: bool,
    pub left: bool,
    pub right: bool,
    pub escape: bool,
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
            escape: true,
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

    let full_circle = std::f64::consts::TAU;
    let angle = dx.atan2(-dy).rem_euclid(full_circle);
    let desired = ((angle * 16.0 / full_circle).round() as i32).rem_euclid(16);
    let current = i32::from(observation.ship_facing & 0x0f);
    let clockwise = (desired - current).rem_euclid(16);
    let turn_error = clockwise.min(16 - clockwise);

    // Thrust whenever roughly aimed at the target. `navigate_to_planet` ends
    // on a real collision, and the display-space distance covered per frame
    // depends on the zoom radius, which this observation does not carry.
    // Cutting thrust on proximity alone therefore lets the flagship's inertia
    // carry it past an orbiting planet, after which it circles indefinitely
    // without ever intersecting.
    NavigationControl {
        thrust: turn_error <= 2,
        left: clockwise > 8,
        right: clockwise != 0 && clockwise <= 8,
        escape: false,
    }
}

/// Encode the collision guard used by C for a moon in an inner system.
///
/// Mirrors `MAKE_WORD (planetOffset, moonOffset)` in `CheckIntersect`, where
/// the planet offset is one-based and the moon offset is two-based (offset 1
/// denotes the planet itself).
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
            escape: true,
            ..NavigationControl::default()
        };
    }

    let dx = f64::from(observation.target_x - observation.ship_x);
    let dy = f64::from(observation.target_y - observation.ship_y);
    let distance = dx.hypot(dy);

    // Sitting exactly on the target without the game having committed an orbit
    // means the collision guard still suppresses this moon. Hold station: the
    // guard clears on its own once CheckIntersect stops matching, and thrusting
    // blindly here would only add drift to correct later.
    if distance == 0.0 {
        return NavigationControl::default();
    }

    let desired_velocity_x = dx * MOON_APPROACH_SPEED / distance;
    let desired_velocity_y = dy * MOON_APPROACH_SPEED / distance;
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
        escape: false,
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
        }
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
    fn escape_leaves_current_orbit_before_steering() {
        let mut state = observation(100, 100, 0, 200, 100);
        state.inner_planet = Some(2);
        state.in_orbit = true;
        assert_eq!(
            steer_toward_target(state),
            NavigationControl {
                escape: true,
                ..NavigationControl::default()
            }
        );
    }

    #[test]
    /// `navigate_to_planet` ends on a real collision, so approach momentum
    /// must never suppress thrust. Coasting on proximity lets the flagship's
    /// inertia carry it past an orbiting planet, after which it circles
    /// forever without intersecting.
    fn generic_navigation_keeps_thrusting_while_closing_on_the_target() {
        let mut state = observation(100, 100, 4, 120, 100);
        state.velocity_x = 200;
        let control = steer_toward_target(state);
        assert!(control.thrust);
        assert!(!control.left);
        assert!(!control.right);
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
    fn moon_controller_holds_station_when_already_at_the_target() {
        // Coincident with the target but production has not committed an
        // orbit: the collision guard is still suppressing this moon, so
        // adding velocity would only have to be undone later.
        assert_eq!(
            steer_moon_navigation(observation(86, 113, 4, 86, 113)),
            NavigationControl::default()
        );
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
    fn moon_controller_escapes_wrong_orbit_before_resuming_navigation() {
        let mut state = observation(111, 160, 0, 86, 113);
        state.in_orbit = true;
        assert_eq!(
            steer_moon_navigation(state),
            NavigationControl {
                escape: true,
                ..NavigationControl::default()
            }
        );
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
