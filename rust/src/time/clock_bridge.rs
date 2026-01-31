// Clock Bridge - Rust implementations of C game clock functions
// These functions bridge to the existing C GameClock state

use std::ffi::c_int;

use crate::bridge_log::rust_bridge_log_msg;

// CLOCK_STATE layout must match C struct exactly
// C layout (from clock.h):
//   BYTE day_index, month_index;  // 2 bytes
//   COUNT year_index;             // 2 bytes (unsigned short)
//   SIZE tick_count, day_in_ticks; // 4 bytes (2x signed short)
//   QUEUE event_q;                // 40 bytes (inline struct)
// Total: 48 bytes
#[repr(C)]
pub struct ClockState {
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,       // COUNT = unsigned short
    pub tick_count: i16,       // SIZE = signed short
    pub day_in_ticks: i16,     // SIZE = signed short
    // event_q is a 40-byte QUEUE struct - we treat it as opaque
    // but must account for its size to not corrupt memory
    pub event_q: [u8; 40],
}

// Access the C GameClock global variable via GetGameClock() function
extern "C" {
    #[link_name = "GetGameClock"]
    fn get_game_clock() -> *mut ClockState;
}

// Functions from clock_rust.c that we need to call
extern "C" {
    fn ValidateEvent(type_: c_int, pmonth_index: *mut c_int, pday_index: *mut c_int, pyear_index: *mut c_int) -> c_int;
    fn AddEvent(type_: c_int, month_index: c_int, day_index: c_int, year_index: c_int, func_index: u8) -> usize;
}

// Constants from clock.h
const CLOCK_BASE_FRAMERATE: usize = 24;
const START_YEAR: u16 = 2155;

// Event type constants
const ABSOLUTE_EVENT: c_int = 0;
const RELATIVE_EVENT: c_int = 1;

// Helper: Log a message to the bridge log (uses central logger)
fn log_clock_bridge(message: &str) {
    rust_bridge_log_msg(message);
}

// Helper: Check if year is leap year
fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// Helper: Get days in month
fn days_in_month(month: u8, year: u16) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 30, // Should never happen
    }
}

// Helper: Advance to next day (internal version that takes &mut ClockState)
fn next_clock_day_internal(clock: &mut ClockState) {
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

// Helper: Access the GameClock safely
fn with_game_clock<F, R>(f: F) -> R
where
    F: FnOnce(&mut ClockState) -> R,
{
    unsafe {
        let ptr = get_game_clock();
        f(&mut *ptr)
    }
}

// Initialize the game clock
#[no_mangle]
pub extern "C" fn rust_clock_init() -> c_int {
    log_clock_bridge("RUST_CLOCK_INIT");
    
    with_game_clock(|clock| {
        // Initialize to Feb 17, START_YEAR
        clock.month_index = 2;
        clock.day_index = 17;
        clock.year_index = START_YEAR;
        clock.tick_count = 0;
        clock.day_in_ticks = 0;
        // event_q initialization is handled by C code
    });
    
    1 // TRUE (success)
}

// Uninitialize the game clock
#[no_mangle]
pub extern "C" fn rust_clock_uninit() -> c_int {
    log_clock_bridge("RUST_CLOCK_UNINIT");
    
    with_game_clock(|clock| {
        // Reset clock state
        clock.tick_count = 0;
        clock.day_in_ticks = 0;
    });
    
    1 // TRUE (success)
}

// Set game clock rate (seconds per day)
#[no_mangle]
pub extern "C" fn rust_clock_set_rate(seconds_per_day: c_int) {
    log_clock_bridge("RUST_CLOCK_RATE");
    
    let new_day_in_ticks = (seconds_per_day as i32) * (CLOCK_BASE_FRAMERATE as i32);
    
    with_game_clock(|clock| {
        let new_tick_count = if clock.day_in_ticks == 0 {
            new_day_in_ticks as i16
        } else if clock.tick_count == 0 {
            0
        } else {
            // Preserve fraction of day
            let scaled = (clock.tick_count as i32) * new_day_in_ticks;
            let result = scaled / (clock.day_in_ticks as i32);
            if result == 0 { 1 } else { result as i16 }
        };
        
        clock.day_in_ticks = new_day_in_ticks as i16;
        clock.tick_count = new_tick_count;
    });
}

// Tick the game clock forward one tick
#[no_mangle]
pub extern "C" fn rust_clock_tick() {
    // Don't log every tick - too noisy
    
    with_game_clock(|clock| {
        // Decrement tick count
        if clock.tick_count > 0 {
            clock.tick_count -= 1;
        }
        
        // Check if we've reached a new day
        if clock.tick_count == 0 && clock.day_in_ticks > 0 {
            clock.tick_count = clock.day_in_ticks;
            next_clock_day_internal(clock);
            // Note: processClockDayEvents is handled by C code
        }
    });
}

// Move game clock forward by specific number of days
#[no_mangle]
pub extern "C" fn rust_clock_advance_days(days: c_int) {
    log_clock_bridge(&format!("RUST_CLOCK_MOVE: {} days", days));
    
    with_game_clock(|clock| {
        for _ in 0..days {
            next_clock_day_internal(clock);
            // Note: processClockDayEvents is handled by C code
        }
        clock.tick_count = clock.day_in_ticks;
    });
}

// Lock the game clock (for debugging)
#[no_mangle]
pub extern "C" fn rust_clock_lock() {
    log_clock_bridge("RUST_CLOCK_LOCK");
    // Note: The mutex lock is handled by C code
}

// Unlock the game clock (for debugging)
#[no_mangle]
pub extern "C" fn rust_clock_unlock() {
    log_clock_bridge("RUST_CLOCK_UNLOCK");
    // Note: The mutex unlock is handled by C code
}

// Check if game clock is running
#[no_mangle]
pub extern "C" fn rust_clock_is_running() -> c_int {
    log_clock_bridge("RUST_CLOCK_RUNNING");
    
    with_game_clock(|clock| {
        if clock.day_in_ticks != 0 {
            1 // TRUE (running)
        } else {
            0 // FALSE (not running)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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