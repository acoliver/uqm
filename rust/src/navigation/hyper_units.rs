//! Original, resolution-dependent hyperspace coordinate factors.

pub const ARILOU_SPACE_X: u32 = 438;
pub const ARILOU_SPACE_Y: u32 = 6372;
pub const QUASI_SPACE_X: u32 = 5000;
pub const QUASI_SPACE_Y: u32 = 5000;
pub const ARILOU_HOME_X: u32 = 6134;
pub const ARILOU_HOME_Y: u32 = 5900;

pub(super) fn log_x(universe: i16, space_width: i32) -> i32 {
    (universe as i32 * ((space_width << 5) >> 4) * 195 + 3125) / 6250
}
pub(super) fn log_y(universe: i16, space_height: i32) -> i32 {
    ((9999 - universe as i32) * ((space_height << 5) >> 4) * 25 + 312) / 625
}
pub(super) fn universe_x(log: i32, space_width: i32) -> i16 {
    let factor = ((space_width << 5) >> 4) * 195;
    ((log * 6250 + factor / 2) / factor) as i16
}
pub(super) fn universe_y(log: i32, space_height: i32) -> i16 {
    let factor = ((space_height << 5) >> 4) * 25;
    (9999 - (log * 625 + factor / 2) / factor) as i16
}

#[derive(Clone, Copy)]
pub struct Geometry {
    pub space_width: i32,
    pub space_height: i32,
}

impl Geometry {
    pub fn new((width, height): (i32, i32)) -> Self {
        Self {
            space_width: width - 64,
            space_height: height,
        }
    }
    pub fn width(self) -> i32 {
        self.space_width << 5
    }
    pub fn height(self) -> i32 {
        self.space_height << 5
    }
    pub fn log_x(self, value: i16) -> i32 {
        log_x(value, self.space_width)
    }
    pub fn log_y(self, value: i16) -> i32 {
        log_y(value, self.space_height)
    }
    pub fn universe_x(self, value: i32) -> i16 {
        universe_x(value, self.space_width)
    }
    pub fn universe_y(self, value: i32) -> i16 {
        universe_y(value, self.space_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_resolution_factors_round_trip_the_universe() {
        for value in 0..=9999i16 {
            assert_eq!(universe_x(log_x(value, 256), 256), value);
            assert_eq!(universe_y(log_y(value, 240), 240), value);
        }
        assert_eq!(log_x(1752, 256), 27987);
        assert_eq!(log_y(1450, 240), 164141);
    }
    #[test]
    fn physical_scale_keeps_the_original_axis_asymmetry() {
        assert_eq!(universe_x(16 * 100, 256), 100);
        assert_eq!(9999 - universe_y(16 * 100, 240), 83);
    }
}
