//! Pure landing-location selection and dispatch eligibility rules.

use super::model::SurfacePoint;

pub const LANDING_CURSOR_HZ: u32 = 40;
pub const LANDING_CURSOR_STEP: i32 = 4;
pub const MAP_WIDTH: i32 = 242;
pub const MAP_HEIGHT: i32 = 75;

/// One landing cursor input sample.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CursorInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub select: bool,
    pub cancel: bool,
    pub abort: bool,
}

/// Selection reducer outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorOutcome {
    Continue(SurfacePoint),
    Selected(SurfacePoint),
    Cancelled,
    Aborted,
}

/// Apply one legacy 40-Hz location-selection frame.
#[must_use]
pub fn reduce_cursor(position: SurfacePoint, input: CursorInput) -> CursorOutcome {
    if input.abort {
        return CursorOutcome::Aborted;
    }
    if input.cancel {
        return CursorOutcome::Cancelled;
    }
    if input.select {
        return CursorOutcome::Selected(position);
    }

    let mut next = position;
    let dx = i32::from(input.right) - i32::from(input.left);
    let dy = i32::from(input.down) - i32::from(input.up);
    if dx != 0 {
        next.x = (next.x + dx * LANDING_CURSOR_STEP).rem_euclid(MAP_WIDTH * LANDING_CURSOR_STEP);
    }
    if dy != 0 {
        let candidate = next.y + dy * LANDING_CURSOR_STEP;
        if (0..MAP_HEIGHT * LANDING_CURSOR_STEP).contains(&candidate) {
            next.y = candidate;
        }
    }
    CursorOutcome::Continue(next)
}

/// Landing rejection reason evaluated before entering selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchRejection {
    Shielded,
    GasGiant,
    InsufficientFuel,
    NoLander,
    NoCrew,
}

/// Values needed to decide whether a surface landing can begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchEligibility {
    pub shielded: bool,
    pub gas_giant: bool,
    pub fuel_on_board: u32,
    pub landers: u8,
    pub crew: u16,
    pub surface_gravity: u16,
    pub fuel_tank_scale: u16,
}

/// C-parity fuel cost: `min(surface_gravity * 2, 3 * FUEL_TANK_SCALE)`.
#[must_use]
pub fn landing_fuel_needed(surface_gravity: u16, fuel_tank_scale: u16) -> u16 {
    surface_gravity
        .saturating_mul(2)
        .min(fuel_tank_scale.saturating_mul(3))
}

/// Validate all landing prerequisites in source order.
pub fn validate_dispatch(input: DispatchEligibility) -> Result<u16, DispatchRejection> {
    if input.shielded {
        return Err(DispatchRejection::Shielded);
    }
    if input.gas_giant {
        return Err(DispatchRejection::GasGiant);
    }
    let fuel = landing_fuel_needed(input.surface_gravity, input.fuel_tank_scale);
    if input.fuel_on_board < u32::from(fuel) {
        return Err(DispatchRejection::InsufficientFuel);
    }
    if input.landers == 0 {
        return Err(DispatchRejection::NoLander);
    }
    if input.crew == 0 {
        return Err(DispatchRejection::NoCrew);
    }
    Ok(fuel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_moves_four_units_and_wraps_horizontally() {
        assert_eq!(
            reduce_cursor(
                SurfacePoint { x: 0, y: 20 },
                CursorInput {
                    left: true,
                    ..CursorInput::default()
                }
            ),
            CursorOutcome::Continue(SurfacePoint {
                x: MAP_WIDTH * 4 - 4,
                y: 20
            })
        );
        assert_eq!(
            reduce_cursor(
                SurfacePoint {
                    x: MAP_WIDTH * 4 - 4,
                    y: 20
                },
                CursorInput {
                    right: true,
                    ..CursorInput::default()
                }
            ),
            CursorOutcome::Continue(SurfacePoint { x: 0, y: 20 })
        );
    }

    #[test]
    fn cursor_rejects_vertical_out_of_range_movement() {
        assert_eq!(
            reduce_cursor(
                SurfacePoint { x: 8, y: 0 },
                CursorInput {
                    up: true,
                    ..CursorInput::default()
                }
            ),
            CursorOutcome::Continue(SurfacePoint { x: 8, y: 0 })
        );
    }

    #[test]
    fn abort_cancel_and_select_are_distinct_terminal_outcomes() {
        let point = SurfacePoint { x: 12, y: 16 };
        assert_eq!(
            reduce_cursor(
                point,
                CursorInput {
                    abort: true,
                    ..CursorInput::default()
                }
            ),
            CursorOutcome::Aborted
        );
        assert_eq!(
            reduce_cursor(
                point,
                CursorInput {
                    cancel: true,
                    ..CursorInput::default()
                }
            ),
            CursorOutcome::Cancelled
        );
        assert_eq!(
            reduce_cursor(
                point,
                CursorInput {
                    select: true,
                    ..CursorInput::default()
                }
            ),
            CursorOutcome::Selected(point)
        );
    }

    #[test]
    fn fuel_cost_is_twice_gravity_capped_at_three_tanks() {
        assert_eq!(landing_fuel_needed(10, 100), 20);
        assert_eq!(landing_fuel_needed(200, 100), 300);
        assert_eq!(landing_fuel_needed(u16::MAX, u16::MAX), u16::MAX);
    }

    #[test]
    fn eligibility_rejections_follow_source_order() {
        let valid = DispatchEligibility {
            shielded: false,
            gas_giant: false,
            fuel_on_board: 100,
            landers: 1,
            crew: 1,
            surface_gravity: 10,
            fuel_tank_scale: 100,
        };
        assert_eq!(validate_dispatch(valid), Ok(20));
        assert_eq!(
            validate_dispatch(DispatchEligibility {
                shielded: true,
                gas_giant: true,
                ..valid
            }),
            Err(DispatchRejection::Shielded)
        );
        assert_eq!(
            validate_dispatch(DispatchEligibility {
                fuel_on_board: 19,
                landers: 0,
                crew: 0,
                ..valid
            }),
            Err(DispatchRejection::InsufficientFuel)
        );
        assert_eq!(
            validate_dispatch(DispatchEligibility {
                landers: 0,
                ..valid
            }),
            Err(DispatchRejection::NoLander)
        );
        assert_eq!(
            validate_dispatch(DispatchEligibility { crew: 0, ..valid }),
            Err(DispatchRejection::NoCrew)
        );
    }
}
