//! Calendar and clock lifecycle operations over the campaign owner.

use std::ffi::c_int;

use crate::bridge_log::rust_bridge_log_msg;

pub use crate::engine::battle::schema::CLOCK_STATE as ClockState;

use crate::engine::World;

// Constants from clock.h
const CLOCK_BASE_FRAMERATE: usize = 24;
const START_YEAR: u16 = 2155;

// Event type constants

// Helper: Log a message to the bridge log (uses central logger)
pub(crate) fn log_clock_bridge(message: &str) {
    rust_bridge_log_msg(message);
}

pub(crate) use crate::engine::clock::days_in_month;
#[cfg(test)]
use crate::engine::clock::is_leap_year;

// Helper: Advance to next day (internal version that takes &mut ClockState)
pub(crate) fn next_clock_day_internal(clock: &mut ClockState) {
    clock.day_index += 1;

    if clock.day_index > days_in_month(clock.month_index, clock.year_index) {
        clock.day_index = 1;
        clock.month_index += 1;

        if clock.month_index > 12 {
            clock.month_index = 1;
            clock.year_index += 1;
        }
    }
}

// Initialize the game clock
pub fn rust_clock_init(world: &mut World) -> c_int {
    log_clock_bridge("RUST_CLOCK_INIT");

    world.clock.init();
    {
        let clock = &mut world.data.game.game_clock;
        // Initialize to Feb 17, START_YEAR
        clock.month_index = 2;
        clock.day_index = 17;
        clock.year_index = START_YEAR;
        clock.tick_count = 0;
        clock.day_in_ticks = 0;
    }

    1 // TRUE (success)
}

// Uninitialize the game clock
pub fn rust_clock_uninit(world: &mut World) -> c_int {
    log_clock_bridge("RUST_CLOCK_UNINIT");

    world.clock.uninit();
    {
        let clock = &mut world.data.game.game_clock;
        // Reset clock state
        clock.tick_count = 0;
        clock.day_in_ticks = 0;
    }

    1 // TRUE (success)
}

// Set game clock rate (seconds per day)
pub fn rust_clock_set_rate(world: &mut World, seconds_per_day: c_int) {
    log_clock_bridge("RUST_CLOCK_RATE");

    let new_day_in_ticks = seconds_per_day * (CLOCK_BASE_FRAMERATE as i32);

    {
        let clock = &mut world.data.game.game_clock;
        let new_tick_count = if clock.day_in_ticks == 0 {
            new_day_in_ticks as i16
        } else if clock.tick_count == 0 {
            0
        } else {
            // Preserve fraction of day
            let scaled = (clock.tick_count as i32) * new_day_in_ticks;
            let result = scaled / (clock.day_in_ticks as i32);
            if result == 0 {
                1
            } else {
                result as i16
            }
        };

        clock.day_in_ticks = new_day_in_ticks as i16;
        clock.tick_count = new_tick_count;
    }
}

fn tick(clock: &mut ClockState) -> bool {
    if clock.tick_count > 0 {
        clock.tick_count -= 1;
    }
    if clock.tick_count <= 0 && clock.day_in_ticks > 0 {
        clock.tick_count = clock.day_in_ticks;
        next_clock_day_internal(clock);
        true
    } else {
        false
    }
}

pub fn rust_clock_tick(world: &mut World) -> bool {
    tick(&mut world.data.game.game_clock)
}

// Check if game clock is running
pub fn rust_clock_is_running(world: &World) -> c_int {
    log_clock_bridge("RUST_CLOCK_RUNNING");
    i32::from(world.data.game.game_clock.day_in_ticks != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticking_preserves_pauses_and_rolls_the_calendar_once() {
        let mut world = crate::engine::World::new();
        let clock = &mut world.data.game.game_clock;
        clock.year_index = 2000;
        clock.month_index = 2;
        clock.day_index = 28;
        clock.day_in_ticks = 2;
        clock.tick_count = 2;
        assert!(!tick(clock));
        assert_eq!(clock.tick_count, 1);
        assert!(tick(clock));
        assert_eq!(
            (clock.year_index, clock.month_index, clock.day_index),
            (2000, 2, 29)
        );
        assert_eq!(clock.tick_count, 2);
        clock.day_in_ticks = 0;
        clock.tick_count = 0;
        assert!(!tick(clock));
        assert_eq!(clock.day_index, 29);
        clock.day_in_ticks = 1;
        assert!(tick(clock));
        assert_eq!((clock.month_index, clock.day_index), (3, 1));
    }

    #[test]
    fn supplied_clock_tick_uses_only_its_world_and_rolls_the_year() {
        let mut first = crate::engine::World::new();
        let mut second = crate::engine::World::new();
        for world in [&mut first, &mut second] {
            let clock = &mut world.data.game.game_clock;
            clock.year_index = 2155;
            clock.month_index = 12;
            clock.day_index = 31;
            clock.day_in_ticks = 2;
            clock.tick_count = 1;
        }
        assert!(rust_clock_tick(&mut first));
        assert_eq!(first.data.calendar_date(), (1, 1, 2156));
        assert_eq!(first.data.game.game_clock.tick_count, 2);
        assert_eq!(second.data.calendar_date(), (12, 31, 2155));
        assert_eq!(second.data.game.game_clock.tick_count, 1);
        assert!(!rust_clock_tick(&mut first));
        assert_eq!(first.data.game.game_clock.tick_count, 1);
        assert!(rust_clock_tick(&mut second));
        assert_eq!(second.data.calendar_date(), (1, 1, 2156));
    }

    #[test]
    fn supplied_clock_tick_does_not_advance_a_paused_clock() {
        let mut world = crate::engine::World::new();
        let clock = &mut world.data.game.game_clock;
        clock.year_index = 2156;
        clock.month_index = 2;
        clock.day_index = 28;
        clock.day_in_ticks = 0;
        clock.tick_count = 1;
        assert!(!rust_clock_tick(&mut world));
        assert_eq!(world.data.calendar_date(), (2, 28, 2156));
        assert_eq!(world.data.game.game_clock.tick_count, 0);
        assert!(!rust_clock_tick(&mut world));
        world.data.game.game_clock.day_in_ticks = 1;
        assert!(rust_clock_tick(&mut world));
        assert_eq!(world.data.calendar_date(), (2, 29, 2156));
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2004));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2001));
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(1, 2000), 31);
        assert_eq!(days_in_month(4, 2000), 30);
        assert_eq!(days_in_month(2, 2000), 29);
        assert_eq!(days_in_month(2, 2001), 28);
    }
}
