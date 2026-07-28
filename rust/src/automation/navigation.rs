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
}

/// Player-one controls to apply for one admitted input callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavigationControl {
    pub thrust: bool,
    pub left: bool,
    pub right: bool,
    pub escape: bool,
}

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
        const SCREEN_CENTER_X: i32 = 128;
        const SCREEN_CENTER_Y: i32 = 91;
        let outward_x = observation.ship_x - SCREEN_CENTER_X;
        let outward_y = observation.ship_y - SCREEN_CENTER_Y;
        if outward_x == 0 && outward_y == 0 {
            (SCREEN_CENTER_X, -1)
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
    if distance_squared <= 64.0 {
        return NavigationControl::default();
    }

    let full_circle = std::f64::consts::TAU;
    let angle = dx.atan2(-dy).rem_euclid(full_circle);
    let desired = ((angle * 16.0 / full_circle).round() as i32).rem_euclid(16);
    let current = i32::from(observation.ship_facing & 0x0f);
    let clockwise = (desired - current).rem_euclid(16);
    let turn_error = clockwise.min(16 - clockwise);

    NavigationControl {
        thrust: turn_error <= 2,
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
        assert_eq!(
            steer_toward_target(observation(124, 91, 4, 128, 91)),
            NavigationControl::default()
        );
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
}
