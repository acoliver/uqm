//! Integer fractions and cache rounding used by flashing UI regions.

pub(super) fn blend_fraction(
    numer: i32,
    denom: i32,
    start: i32,
    end: i32,
    factor: i32,
) -> (i32, i32) {
    (numer * end + (denom - numer) * start, denom * factor)
}

pub(super) fn cache_position(size: u16, numer: i32, denom: i32) -> i32 {
    ((i32::from(size) - 1) * numer + denom / 2) / denom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_preserves_endpoints_and_negative_brightness() {
        assert_eq!(blend_fraction(0, 8, -2, 3, 2), (-16, 16));
        assert_eq!(blend_fraction(8, 8, -2, 3, 2), (24, 16));
        assert_eq!(blend_fraction(3, 8, -2, 3, 2), (-1, 16));
    }

    #[test]
    fn cache_rounds_half_up_without_changing_endpoints() {
        assert_eq!(cache_position(9, 0, 100), 0);
        assert_eq!(cache_position(9, 100, 100), 8);
        assert_eq!(cache_position(9, 49, 100), 4);
        assert_eq!(cache_position(2, 49, 100), 0);
        assert_eq!(cache_position(2, 50, 100), 1);
    }
}
