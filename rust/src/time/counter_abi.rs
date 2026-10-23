//! SDL-backed game time counter.
//!
//! The counter runs at `ONE_SECOND` (840) units per second, derived from
//! `SDL_GetTicks`.

#![allow(non_snake_case)]

/// `ONE_SECOND` from `libs/timelib.h`: the LCM of every fraction of a second
/// the game uses.
pub const ONE_SECOND: u32 = 840;

/// Convert milliseconds since SDL init into game time units.
///
/// The seconds and sub-second parts are scaled separately so the result
/// stays exact for the whole `u32` millisecond range. Wrapping matches the
/// C `Uint32` arithmetic.
pub fn ticks_to_time_count(ticks: u32) -> u32 {
    (ticks / 1000)
        .wrapping_mul(ONE_SECOND)
        .wrapping_add((ticks % 1000) * ONE_SECOND / 1000)
}

pub fn InitTimeSystem() {}

pub fn UnInitTimeSystem() {}

pub fn GetTimeCounter() -> u32 {
    // SAFETY: SDL_GetTicks has no preconditions.
    ticks_to_time_count(unsafe { sdl2_sys::SDL_GetTicks() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_points_are_safe_rust_functions() {
        let _: fn() = InitTimeSystem;
        let _: fn() = UnInitTimeSystem;
        let _: fn() -> u32 = GetTimeCounter;
    }

    #[test]
    fn full_tick_range_matches_integer_scaling() {
        for ticks in [0, 1, 998, 999, 1000, 1001, u32::MAX - 1, u32::MAX] {
            let expected = (u64::from(ticks) * u64::from(ONE_SECOND) / 1000) as u32;
            assert_eq!(ticks_to_time_count(ticks), expected, "ticks={ticks}");
        }
    }

    #[test]
    fn whole_seconds_scale_exactly() {
        assert_eq!(ticks_to_time_count(0), 0);
        assert_eq!(ticks_to_time_count(1000), 840);
        assert_eq!(ticks_to_time_count(3000), 2520);
    }

    #[test]
    fn sub_second_part_truncates() {
        // 25 ms -> 25 * 840 / 1000 = 21 (see automation/script.rs).
        assert_eq!(ticks_to_time_count(25), 21);
        assert_eq!(ticks_to_time_count(999), 839);
        assert_eq!(ticks_to_time_count(1500), 840 + 420);
    }
}
