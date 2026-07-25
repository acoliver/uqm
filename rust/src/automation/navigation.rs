//! Pure controller for driving the flagship through real interplanetary flight.

/// Real navigation state sampled from the running solar-system simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationObservation {
    pub active: bool,
    pub inner_planet: Option<u8>,
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
}

/// Compute a pure-pursuit steering command using UQM's sixteen facings.
#[must_use]
pub fn steer_toward_target(observation: NavigationObservation) -> NavigationControl {
    if !observation.active || observation.inner_planet.is_some() {
        return NavigationControl::default();
    }

    let dx = f64::from(observation.target_x - observation.ship_x);
    let dy = f64::from(observation.target_y - observation.ship_y);
    if dx == 0.0 && dy == 0.0 {
        return NavigationControl {
            thrust: true,
            ..NavigationControl::default()
        };
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
                left: false,
                right: false,
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
    fn turns_right_toward_target_on_the_right() {
        let control = steer_toward_target(observation(100, 100, 0, 200, 100));
        assert!(control.right);
        assert!(!control.left);
    }

    #[test]
    fn stops_controls_after_entering_inner_system() {
        let mut state = observation(100, 100, 0, 100, 100);
        state.inner_planet = Some(2);
        assert_eq!(steer_toward_target(state), NavigationControl::default());
    }
}
