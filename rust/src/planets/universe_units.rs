//! The universe-to-logical coordinate macros of `units.h`
//! (`UNIVERSE_TO_LOGX`, `UNIVERSE_TO_LOGY`), with the original hyperspace
//! speed factors (`NORMALIZED_HYPERSPACE_SPEED` is not defined).
//!
//! The logical space is the SIS screen: `ScreenWidth` is 320 and
//! `ScreenHeight` is 240, so `SPACE_WIDTH` is `320 - STATUS_WIDTH` and
//! `SPACE_HEIGHT` is 240.

const STATUS_WIDTH: i32 = 64;
const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 240;
const SPACE_WIDTH: i32 = SCREEN_WIDTH - STATUS_WIDTH;
const SPACE_HEIGHT: i32 = SCREEN_HEIGHT;

/// `DISPLAY_TO_WORLD (x) << MAX_REDUCTION`, with `ONE_SHIFT` 2 and
/// `MAX_REDUCTION` 3.
const fn log_space(display: i32) -> i32 {
    (display << 2) << 3
}

const LOG_SPACE_WIDTH: i32 = log_space(SPACE_WIDTH);
const LOG_SPACE_HEIGHT: i32 = log_space(SPACE_HEIGHT);

const MAX_X_UNIVERSE: i32 = 9999;
const MAX_Y_UNIVERSE: i32 = 9999;
const SECTOR_WIDTH: i32 = 195;
const SECTOR_HEIGHT: i32 = 25;

const LOG_UNITS_X: i32 = (LOG_SPACE_WIDTH >> 4) * SECTOR_WIDTH;
const LOG_UNITS_Y: i32 = (LOG_SPACE_HEIGHT >> 4) * SECTOR_HEIGHT;
const UNIVERSE_UNITS_X: i32 = ((MAX_X_UNIVERSE + 1) >> 4) * 10;
const UNIVERSE_UNITS_Y: i32 = (MAX_Y_UNIVERSE + 1) >> 4;

/// `ROUNDING_ERROR (div)`.
const fn rounding_error(div: i32) -> i32 {
    div >> 1
}

/// `UNIVERSE_TO_LOGX (ux)`.
pub const fn universe_to_logx(ux: i16) -> i32 {
    (ux as i32 * LOG_UNITS_X + rounding_error(UNIVERSE_UNITS_X)) / UNIVERSE_UNITS_X
}

/// `UNIVERSE_TO_LOGY (uy)`: the Y axis is inverted.
pub const fn universe_to_logy(uy: i16) -> i32 {
    ((MAX_Y_UNIVERSE - uy as i32) * LOG_UNITS_Y + rounding_error(UNIVERSE_UNITS_Y))
        / UNIVERSE_UNITS_Y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unit_factors_are_the_original_hyperspace_speed_factors() {
        assert_eq!(LOG_SPACE_WIDTH, 8192);
        assert_eq!(LOG_SPACE_HEIGHT, 7680);
        assert_eq!(LOG_UNITS_X, 99_840);
        assert_eq!(LOG_UNITS_Y, 12_000);
        assert_eq!(UNIVERSE_UNITS_X, 6_250);
        assert_eq!(UNIVERSE_UNITS_Y, 625);
    }

    #[test]
    fn the_ur_quan_ship_point_converts_as_the_macros_do() {
        // (5288 * 99840 + 3125) / 6250 and ((9999 - 4892) * 12000 + 312) / 625,
        // evaluated with C integer division.
        assert_eq!(universe_to_logx(5288), 84_473);
        assert_eq!(universe_to_logy(4892), 98_054);
    }

    #[test]
    fn the_universe_corners_convert_as_the_macros_do() {
        assert_eq!(universe_to_logx(0), 0);
        assert_eq!(universe_to_logx(9999), (9999 * 99_840 + 3_125) / 6_250);
        assert_eq!(universe_to_logy(9999), 0);
        assert_eq!(universe_to_logy(0), (9999 * 12_000 + 312) / 625);
    }
}
