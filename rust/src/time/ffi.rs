// FFI bindings for Time module
// Provides C-compatible interface for game clock and date operations

use std::sync::Mutex;

use super::game_date::GameDate;
use super::game_clock::GameClock;
use super::events::Event;

/// Global game clock instance
static GLOBAL_GAME_CLOCK: Mutex<Option<GameClock>> = Mutex::new(None);

/// Initialize the global game clock
#[no_mangle]
pub extern "C" fn rust_init_game_clock() {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if clock.is_none() {
        *clock = Some(GameClock::new());
    }
}

/// Set the game clock rate (ticks per day)
#[no_mangle]
pub extern "C" fn rust_set_clock_rate(ticks_per_day: usize) {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.set_rate(ticks_per_day);
    }
}

/// Tick the clock forward one tick
/// Returns 1 if day changed, 0 otherwise
#[no_mangle]
pub extern "C" fn rust_clock_tick() -> i32 {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        if gc.tick() { 1 } else { 0 }
    } else {
        0
    }
}

/// Advance the clock by a specific number of ticks
#[no_mangle]
pub extern "C" fn rust_clock_advance_ticks(ticks: usize) {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.advance_ticks(ticks);
    }
}

/// Advance the clock by a specific number of days
#[no_mangle]
pub extern "C" fn rust_clock_advance_days(days: u32) {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.advance_days(days);
    }
}

/// Get the current day
#[no_mangle]
pub extern "C" fn rust_get_clock_day() -> u8 {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| gc.date().day).unwrap_or(0)
}

/// Get the current month
#[no_mangle]
pub extern "C" fn rust_get_clock_month() -> u8 {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| gc.date().month).unwrap_or(0)
}

/// Get the current year
#[no_mangle]
pub extern "C" fn rust_get_clock_year() -> u32 {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| gc.date().year).unwrap_or(0)
}

/// Get day fraction (0.0 to 1.0)
#[no_mangle]
pub extern "C" fn rust_get_day_fraction() -> f64 {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| gc.day_fraction()).unwrap_or(0.0)
}

/// Add an absolute event
#[no_mangle]
pub extern "C" fn rust_add_event_absolute(
    year: u32,
    month: u8,
    day: u8,
    func_index: u8,
) -> u32 {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        let date = GameDate::new(day, month, year);
        let event = Event::new_absolute(date, func_index, 100);
        let event_id = event.id;
        
        if gc.add_event(event).is_ok() {
            event_id
        } else {
            0
        }
    } else {
        0
    }
}

/// Add a relative event (days from now)
#[no_mangle]
pub extern "C" fn rust_add_event_relative(
    days_offset: u32,
    func_index: u8,
) -> u32 {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        let event = Event::new_relative(days_offset, func_index, 100);
        let event_id = event.id;
        
        if gc.add_event(event).is_ok() {
            event_id
        } else {
            0
        }
    } else {
        0
    }
}

/// Remove an event by ID
#[no_mangle]
pub extern "C" fn rust_remove_event(event_id: u32) -> i32 {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        match gc.remove_event(event_id) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    } else {
        0
    }
}

/// Clear all events
#[no_mangle]
pub extern "C" fn rust_clear_events() {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.clear_events();
    }
}

/// Check if the clock is running
#[no_mangle]
pub extern "C" fn rust_clock_is_running() -> i32 {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| if gc.is_running() { 1 } else { 0 }).unwrap_or(0)
}

/// Lock the clock (for debugging)
#[no_mangle]
pub extern "C" fn rust_clock_lock() {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.lock();
    }
}

/// Unlock the clock
#[no_mangle]
pub extern "C" fn rust_clock_unlock() {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.unlock();
    }
}

/// Check if the clock is locked
#[no_mangle]
pub extern "C" fn rust_clock_is_locked() -> i32 {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| if gc.is_locked() { 1 } else { 0 }).unwrap_or(0)
}

/// Get number of ticks per day
#[no_mangle]
pub extern "C" fn rust_get_ticks_per_day() -> usize {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| gc.day_in_ticks()).unwrap_or(0)
}

/// Get current tick count
#[no_mangle]
pub extern "C" fn rust_get_tick_count() -> usize {
    let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    clock.as_ref().map(|gc| gc.tick_count()).unwrap_or(0)
}

/// Check if a year is a leap year
#[no_mangle]
pub extern "C" fn rust_is_leap_year(year: u32) -> i32 {
    if GameDate::is_leap_year(year) { 1 } else { 0 }
}

/// Get days in month for a given year
#[no_mangle]
pub extern "C" fn rust_days_in_month(month: u8, year: u32) -> u8 {
    GameDate::days_in_month(month, year)
}

/// Reset the clock to initial state
#[no_mangle]
pub extern "C" fn rust_reset_clock() {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.reset();
    }
}

/// Set a specific date
#[no_mangle]
pub extern "C" fn rust_set_clock_date(day: u8, month: u8, year: u32) {
    let mut clock = GLOBAL_GAME_CLOCK.lock().unwrap();
    if let Some(gc) = clock.as_mut() {
        gc.date = GameDate::new(day, month, year);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_init_game_clock() {
        rust_init_game_clock();
        
        let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
        assert!(clock.is_some());
    }

    #[test]
    fn test_rust_set_clock_rate() {
        rust_init_game_clock();
        
        rust_set_clock_rate(120);
        
        let clock = GLOBAL_GAME_CLOCK.lock().unwrap();
        assert_eq!(clock.as_ref().unwrap().day_in_ticks(), 120);
    }

    #[test]
    fn test_rust_get_clock_day() {
        rust_init_game_clock();
        
        let day = rust_get_clock_day();
        assert_eq!(day, GameDate::default().day);
    }

    #[test]
    fn test_rust_get_clock_month() {
        rust_init_game_clock();
        
        let month = rust_get_clock_month();
        assert_eq!(month, GameDate::default().month);
    }

    #[test]
    fn test_rust_get_clock_year() {
        rust_init_game_clock();
        
        let year = rust_get_clock_year();
        assert_eq!(year, GameDate::default().year);
    }

    #[test]
    fn test_rust_clock_tick() {
        rust_init_game_clock();
        rust_set_clock_rate(10);
        
        let day_changed = rust_clock_tick();
        assert!(day_changed == 0); // 1 tick shouldn't change day
        
        // Ticks 2-10 should advance day
        for _ in 0..9 {
            rust_clock_tick();
        }
        
        let next_day = rust_clock_tick();
        assert!(next_day == 1); // Day should have changed
    }

    #[test]
    fn test_rust_clock_advance_days() {
        rust_init_game_clock();
        rust_set_clock_rate(120);
        
        let initial_day = rust_get_clock_day();
        let initial_month = rust_get_clock_month();
        
        rust_clock_advance_days(5);
        
        let current_day = rust_get_clock_day();
        let current_month = rust_get_clock_month();
        
        // Should be 5 days later
        assert_ne!((current_day, current_month), (initial_day, initial_month));
    }

    #[test]
    fn test_rust_clock_tick_after_advance() {
        rust_init_game_clock();
        rust_set_clock_rate(10);
        
        rust_clock_advance_days(0); // Reset tick count
        
        // Clock should be at rate ticks
        assert_eq!(rust_get_tick_count(), 10);
    }

    #[test]
    fn test_rust_add_event_absolute() {
        rust_init_game_clock();
        rust_set_clock_rate(120);
        
        let event_id = rust_add_event_absolute(2155, 3, 1, 100);
        assert_ne!(event_id, 0);
    }

    #[test]
    fn test_rust_add_event_relative() {
        rust_init_game_clock();
        rust_set_clock_rate(120);
        
        let event_id = rust_add_event_relative(10, 100);
        assert_ne!(event_id, 0);
    }

    #[test]
    fn test_rust_remove_event() {
        rust_init_game_clock();
        rust_set_clock_rate(120);
        
        let event_id = rust_add_event_relative(10, 100);
        let result = rust_remove_event(event_id);
        
        assert_eq!(result, 1);
    }

    #[test]
    fn test_rust_clear_events() {
        rust_init_game_clock();
        rust_set_clock_rate(120);
        
        rust_add_event_relative(10, 100);
        rust_add_event_relative(20, 200);
        
        rust_clear_events();
        
        // Events should be cleared
        // In a real test, we'd have a way to query event count
    }

    #[test]
    fn test_rust_clock_is_running() {
        rust_init_game_clock();
        
        assert_eq!(rust_clock_is_running(), 0);
        
        rust_set_clock_rate(120);
        assert_eq!(rust_clock_is_running(), 1);
    }

    #[test]
    fn test_rust_clock_lock_unlock() {
        rust_init_game_clock();
        rust_set_clock_rate(10);
        
        rust_clock_lock();
        assert_eq!(rust_clock_is_locked(), 1);
        
        // Ticks should be ignored when locked
        let day_changed = rust_clock_tick();
        assert!(day_changed == 0);
        assert_eq!(rust_get_tick_count(), 10); // Tick count unchanged
        
        rust_clock_unlock();
        assert_eq!(rust_clock_is_locked(), 0);
    }

    #[test]
    fn test_rust_is_leap_year() {
        assert_eq!(rust_is_leap_year(2000), 1);
        assert_eq!(rust_is_leap_year(2004), 1);
        assert_eq!(rust_is_leap_year(1900), 0);
        assert_eq!(rust_is_leap_year(2001), 0);
    }

    #[test]
    fn test_rust_days_in_month() {
        assert_eq!(rust_days_in_month(1, 2000), 31);
        assert_eq!(rust_days_in_month(2, 2000), 29); // Leap year
        assert_eq!(rust_days_in_month(2, 2001), 28); // Non-leap year
        assert_eq!(rust_days_in_month(4, 2000), 30);
    }

    #[test]
    fn test_rust_get_day_fraction() {
        rust_init_game_clock();
        rust_set_clock_rate(100);
        
        assert_eq!(rust_get_day_fraction(), 0.0);
        
        rust_clock_advance_ticks(50);
        assert_eq!(rust_get_day_fraction(), 0.5);
    }

    #[test]
    fn test_rust_reset_clock() {
        rust_init_game_clock();
        rust_set_clock_rate(120);
        rust_clock_advance_days(5);
        
        rust_reset_clock();
        
        assert_eq!(rust_get_ticks_per_day(), 0);
        assert_eq!(rust_get_tick_count(), 0);
        assert_eq!(rust_get_clock_year(), GameDate::default().year);
    }

    #[test]
    fn test_rust_set_clock_date() {
        rust_init_game_clock();
        
        rust_set_clock_date(1, 3, 2000);
        
        assert_eq!(rust_get_clock_day(), 1);
        assert_eq!(rust_get_clock_month(), 3);
        assert_eq!(rust_get_clock_year(), 2000);
    }

    #[test]
    fn test_multiple_ticks() {
        rust_init_game_clock();
        rust_set_clock_rate(10);
        
        let mut days_changed = 0;
        for _ in 0..30 {
            if rust_clock_tick() == 1 {
                days_changed += 1;
            }
        }
        
        // With 10 ticks per day, 30 ticks should advance 3 days
        assert_eq!(days_changed, 3);
    }

    #[test]
    fn test_date_persistence() {
        rust_init_game_clock();
        rust_set_clock_rate(120);
        
        let initial_date = (rust_get_clock_day(), rust_get_clock_month(), rust_get_clock_year());
        
        rust_clock_advance_days(7);
        
        let new_date = (rust_get_clock_day(), rust_get_clock_month(), rust_get_clock_year());
        
        assert_ne!(initial_date, new_date);
    }
}
