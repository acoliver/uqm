//! C ABI for the SDL-backed game time counter (`libs/timelib.h`).
//!
//! The counter runs at `ONE_SECOND` (840) units per second, derived from
//! `SDL_GetTicks`.

use std::ffi::c_uint;

/// `ONE_SECOND` from `libs/timelib.h`: the LCM of every fraction of a second
/// the game uses.
pub const ONE_SECOND: u32 = 840;

extern "C" {
    fn SDL_GetTicks() -> u32;
}

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

#[no_mangle]
pub extern "C" fn InitTimeSystem() {}

#[no_mangle]
pub extern "C" fn UnInitTimeSystem() {}

#[no_mangle]
pub extern "C" fn GetTimeCounter() -> c_uint {
    // SAFETY: SDL_GetTicks has no preconditions.
    ticks_to_time_count(unsafe { SDL_GetTicks() })
}

#[cfg(test)]
mod tests {
    use super::*;

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
