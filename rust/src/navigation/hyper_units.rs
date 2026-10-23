//! Original, resolution-dependent hyperspace coordinate factors.

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
