//! Live engine ABI for race ports. Unlike `types::RaceDesc`, these records can
//! be returned directly to the engine's `init_<race>` callers.

pub(crate) fn copy_display_name(target: &mut [std::ffi::c_char], source: &[u8]) {
    target.fill(0);
    let length = source
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(source.len());
    let count = length.min(target.len().saturating_sub(1));
    for (target, source) in target.iter_mut().zip(source).take(count) {
        *target = *source as std::ffi::c_char;
    }
}

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::all
)]
pub mod abi {
    pub use crate::ships::DescriptorLoader;

    include!("../abi/ship_abi.rs");

    pub use crate::engine::battle::ai_constants::{
        AWESOME_RATING, BATTLE_LEFT, BATTLE_RIGHT, BATTLE_SPECIAL, BATTLE_THRUST, BATTLE_WEAPON,
        GOOD_RATING, STANDARD_RATING,
    };
    pub use crate::math::seed_random as TFB_SeedRandom;

    pub use crate::engine::battle::tracking::track_ship;
}

pub use crate::math::{angle_to_facing, facing_to_angle, normalize_angle, normalize_facing};

pub use crate::math::{cosine, sine};

pub use crate::math::wrap_delta;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_trigonometry_uses_the_immutable_table_with_size_truncation() {
        for angle in -128..128 {
            for magnitude in [i32::MIN, -65537, -1, 0, 1, 32767, 65537, i32::MAX] {
                let value = crate::math::sinetab[(angle & 63) as usize];
                let expected = ((i64::from(value) * i64::from(magnitude)) >> 14) as i16;
                assert_eq!(sine(angle, magnitude), expected);
                assert_eq!(cosine(angle - 16, magnitude), expected);
            }
        }
        assert_eq!(sine(0, 4096), -4096);
        assert_eq!(sine(32, 4096), 4096);
        assert_eq!(cosine(16, 4096), 4096);
    }

    #[test]
    fn c_angle_rounding_and_wrap_boundaries() {
        assert_eq!(angle_to_facing(1), 0);
        assert_eq!(angle_to_facing(2), 1);
        assert_eq!(angle_to_facing(-3), -1);
        assert_eq!(facing_to_angle(15), 60);
        assert_eq!(normalize_facing(-1), 15);
        assert_eq!(normalize_facing(16), 0);
        assert_eq!(normalize_angle(-1), 63);
        assert_eq!(normalize_angle(64), 0);
    }

    #[test]
    fn wrap_delta_preserves_half_extent_and_adjusts_once() {
        for (delta, expected) in [
            (-4097, 4095),
            (-4096, -4096),
            (4096, 4096),
            (4097, -4095),
            (20000, 11808),
            (i16::MIN, -24576),
        ] {
            assert_eq!(wrap_delta(delta, 8192), expected);
        }
    }
}

#[cfg(test)]
mod name_copy_tests {
    use super::copy_display_name;

    #[test]
    fn display_names_are_terminated_padded_and_truncated_by_bytes() {
        let mut target = [7; 8];
        copy_display_name(&mut target, b"Orz");
        assert_eq!(target, [b'O' as _, b'r' as _, b'z' as _, 0, 0, 0, 0, 0]);
        copy_display_name(&mut target, b"abc\0ignored");
        assert_eq!(target, [b'a' as _, b'b' as _, b'c' as _, 0, 0, 0, 0, 0]);
        let mut target = [0; 3];
        copy_display_name(&mut target, &[b'A', 0xc3, 0xa9]);
        assert_eq!(target, [b'A' as _, 0xc3_u8 as _, 0]);
    }

    #[test]
    fn empty_and_single_byte_destinations_do_not_copy_a_name() {
        copy_display_name(&mut [], b"Orz");
        let mut target = [7];
        copy_display_name(&mut target, b"Orz");
        assert_eq!(target, [0]);
    }
}
