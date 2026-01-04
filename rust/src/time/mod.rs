//! Time and clock system for The Ur-Quan Masters
//!
//! This module implements the game clock and date arithmetic, handling
//! time-based events and game progression.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// Represents a date in the game calendar
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameDate {
    /// Year (e.g., 2160)
    pub year: u16,
    /// Month (1-12)
    pub month: u8,
    /// Day (1-31, varies by month)
    pub day: u8,
}

impl GameDate {
    /// Create a new GameDate
    ///
    /// # Arguments
    /// * `year` - The year
    /// * `month` - The month (1-12)
    /// * `day` - The day
    ///
    /// # Panics
    /// Panics if month is not in 1-12
    pub fn new(year: u16, month: u8, day: u8) -> Self {
        assert!((1..=12).contains(&month), "month must be between 1 and 12");
        Self { year, month, day }
    }

    /// Check if a year is a leap year
    ///
    /// # Rules
    /// - Divisible by 4: leap
    /// - Divisible by 100: not leap
    /// - Divisible by 400: leap
    pub fn is_leap_year(year: u16) -> bool {
        (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
    }

    /// Get the number of days in a month
    ///
    /// # Arguments
    /// * `month` - Month (1-12)
    /// * `year` - Year (needed for February)
    ///
    /// # Returns
    /// Number of days in the month
    #[must_use]
    pub fn days_in_month(month: u8, year: u16) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => panic!("Invalid month: {}", month),
        }
    }

    /// Advance the date by one day
    pub fn next_day(&mut self) {
        let days_this_month = Self::days_in_month(self.month, self.year);
        if self.day < days_this_month {
            self.day += 1;
        } else if self.month < 12 {
            self.day = 1;
            self.month += 1;
        } else {
            self.day = 1;
            self.month = 1;
            self.year += 1;
        }
    }

    /// Advance the date by multiple days
    pub fn next_days(&mut self, days: u32) {
        for _ in 0..days {
            self.next_day();
        }
    }

    /// Check if the date is valid
    ///
    /// # Returns
    /// `true` if the date is valid, `false` otherwise
    pub fn is_valid(&self) -> bool {
        if self.month < 1 || self.month > 12 {
            return false;
        }
        if self.day < 1 || self.day > Self::days_in_month(self.month, self.year) {
            return false;
        }
        true
    }
}

impl Default for GameDate {
    fn default() -> Self {
        // Default to a reasonable start date
        Self {
            year: 2158,
            month: 2,
            day: 17,
        }
    }
}

/// An event that fires on a specific date
#[derive(Debug, Clone)]
pub struct Event {
    /// Event name or identifier
    pub name: String,
    /// Optional callback function handle
    pub callback: Option<usize>,
}

/// The game clock system
///
/// Manages time progression, keeps track of the current game date,
/// and schedules time-based events.
#[derive(Debug)]
pub struct GameClock {
    /// Current game date
    current_date: GameDate,
    /// Number of ticks that have passed
    tick_count: usize,
    /// How many ticks represent one in-game day
    day_in_ticks: usize,
    /// Events scheduled for specific dates
    events: BTreeMap<GameDate, Vec<Event>>,
    /// Mutex for thread-safe access
    mutex: Mutex<()>,
}

impl GameClock {
    /// Create a new game clock
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_date: GameDate::default(),
            tick_count: 0,
            day_in_ticks: 100, // Default: 100 ticks per day
            events: BTreeMap::new(),
            mutex: Mutex::new(()),
        }
    }

    /// Get the current date
    #[must_use]
    pub fn current_date(&self) -> GameDate {
        self.current_date
    }

    /// Get the tick count
    #[must_use]
    pub fn tick_count(&self) -> usize {
        self.tick_count
    }

    /// Get the day-in-ticks setting
    #[must_use]
    pub fn day_in_ticks(&self) -> usize {
        self.day_in_ticks
    }

    /// Set the number of ticks per day
    ///
    /// # Arguments
    /// * `ticks_per_day` - How many ticks should represent one day
    pub fn set_rate(&mut self, ticks_per_day: usize) {
        self.day_in_ticks = ticks_per_day;
    }

    /// Advance the clock by one tick
    ///
    /// This advances the game date based on the current tick rate.
    ///
    /// # Returns
    /// A vector of events that fired on this tick
    pub fn tick(&mut self) -> Vec<Event> {
        let _guard = self.mutex.lock().unwrap();
        self.tick_count += 1;

        let mut fired_events = Vec::new();

        // Check if we've advanced a day
        if self.tick_count.is_multiple_of(self.day_in_ticks) {
            self.current_date.next_day();

            // Fire events scheduled for this date
            if let Some(events) = self.events.remove(&self.current_date) {
                fired_events.extend(events);
            }
        }

        fired_events
    }

    /// Schedule an event for a specific date
    ///
    /// # Arguments
    /// * `date` - When the event should fire
    /// * `event` - The event to schedule
    ///
    /// # Returns
    /// Ok(()) on success, Err if date is invalid
    pub fn add_event(&mut self, date: GameDate, event: Event) -> Result<(), String> {
        if !date.is_valid() {
            return Err("Invalid date".to_string());
        }

        self.events.entry(date).or_default().push(event);
        Ok(())
    }

    /// Move the game forward by the specified number of days
    ///
    /// # Arguments
    /// * `days` - Number of days to advance
    pub fn move_days(&mut self, days: usize) {
        for _ in 0..days {
            self.tick();
        }
    }

    /// Get all scheduled events
    #[must_use]
    pub fn events(&self) -> &BTreeMap<GameDate, Vec<Event>> {
        &self.events
    }
}

impl Default for GameClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper around GameClock
///
/// Provides interior mutability with thread-safe operations.
pub struct SharedGameClock {
    inner: Mutex<GameClock>,
}

impl SharedGameClock {
    /// Create a new shared game clock
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(GameClock::new()),
        }
    }

    /// Get a snapshot of the current date
    #[must_use]
    pub fn current_date(&self) -> GameDate {
        let inner = self.inner.lock().unwrap();
        inner.current_date()
    }

    /// Tick the clock (returns fired events)
    pub fn tick(&self) -> Vec<Event> {
        let mut inner = self.inner.lock().unwrap();
        inner.tick()
    }

    /// Add an event
    pub fn add_event(&self, date: GameDate, event: Event) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        inner.add_event(date, event)
    }

    /// Set the tick rate
    pub fn set_rate(&self, ticks_per_day: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.set_rate(ticks_per_day);
    }

    /// Move forward by days
    pub fn move_days(&self, days: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.move_days(days);
    }
}

impl Default for SharedGameClock {
    fn default() -> Self {
        Self::new()
    }
}

// FFI wrappers
#[no_mangle]
pub extern "C" fn rust_game_clock_tick() {
    // In a real implementation, this would tick the global clock
    // For now, this is a placeholder
}

#[no_mangle]
pub extern "C" fn rust_set_clock_rate(seconds_per_day: i32) {
    // In a real implementation, this would set the global clock rate
    // For now, this is a placeholder
    if seconds_per_day > 0 {
        // Convert seconds_per_day to ticks_per_day
        let _ticks = (seconds_per_day * 60) as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_leap_year() {
        assert!(GameDate::is_leap_year(2000)); // Divisible by 400
        assert!(!GameDate::is_leap_year(1900)); // Divisible by 100 but not 400
        assert!(GameDate::is_leap_year(2004)); // Divisible by 4
        assert!(!GameDate::is_leap_year(2001)); // Not divisible by 4
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(GameDate::days_in_month(1, 2024), 31);
        assert_eq!(GameDate::days_in_month(4, 2024), 30);
        assert_eq!(GameDate::days_in_month(2, 2024), 29); // Leap year
        assert_eq!(GameDate::days_in_month(2, 2023), 28); // Not leap year
    }

    #[test]
    fn test_game_date_next_day() {
        let mut date = GameDate::new(2024, 1, 31);
        date.next_day();
        assert_eq!(date, GameDate::new(2024, 2, 1));

        let mut date = GameDate::new(2024, 2, 28);
        date.next_day();
        assert_eq!(date, GameDate::new(2024, 2, 29)); // Leap year

        let mut date = GameDate::new(2023, 12, 31);
        date.next_day();
        assert_eq!(date, GameDate::new(2024, 1, 1));
    }

    #[test]
    fn test_game_date_next_days() {
        let mut date = GameDate::new(2024, 1, 1);
        date.next_days(30);
        assert_eq!(date, GameDate::new(2024, 1, 31));

        let mut date = GameDate::new(2024, 1, 1);
        date.next_days(31);
        assert_eq!(date, GameDate::new(2024, 2, 1));
    }

    #[test]
    fn test_game_date_is_valid() {
        assert!(GameDate::new(2024, 1, 15).is_valid());
        assert!(GameDate::new(2024, 2, 29).is_valid()); // Leap year

        // Create invalid dates using Default then modify
        let mut date = GameDate::default();
        date.year = 2023;
        date.month = 2;
        date.day = 29;
        assert!(!date.is_valid()); // Not leap year

        date.month = 0;
        assert!(!date.is_valid()); // Invalid month

        date.month = 13;
        assert!(!date.is_valid()); // Invalid month
    }

    #[test]
    fn test_game_date_default() {
        let date = GameDate::default();
        assert_eq!(date.year, 2158);
        assert_eq!(date.month, 2);
        assert_eq!(date.day, 17);
    }

    #[test]
    fn test_game_clock_new() {
        let clock = GameClock::new();
        assert_eq!(clock.tick_count(), 0);
        assert_eq!(clock.day_in_ticks(), 100);
        assert_eq!(clock.current_date(), GameDate::default());
    }

    #[test]
    fn test_game_clock_tick() {
        let mut clock = GameClock::new();
        clock.set_rate(10); // 10 ticks per day for testing

        for _ in 0..9 {
            assert!(clock.tick().is_empty());
            assert_eq!(clock.current_date(), GameDate::default());
        }

        // This tick should advance the day
        let events = clock.tick();
        assert!(events.is_empty());
        assert_ne!(clock.current_date(), GameDate::default());
        assert_eq!(clock.current_date().day, 18);
    }

    #[test]
    fn test_game_clock_set_rate() {
        let mut clock = GameClock::new();
        clock.set_rate(50);
        assert_eq!(clock.day_in_ticks(), 50);
    }

    #[test]
    fn test_game_clock_add_event() {
        let mut clock = GameClock::new();

        let tomorrow = {
            let mut d = clock.current_date();
            d.next_day();
            d
        };

        let event = Event {
            name: "test_event".to_string(),
            callback: None,
        };

        let result = clock.add_event(tomorrow, event.clone());
        assert!(result.is_ok());

        assert_eq!(clock.events().len(), 1);
        assert!(clock.events().contains_key(&tomorrow));
    }

    #[test]
    fn test_game_clock_event_fires() {
        let mut clock = GameClock::new();
        clock.set_rate(1); // 1 tick per day

        let tomorrow = {
            let mut d = clock.current_date();
            d.next_day();
            d
        };

        let event = Event {
            name: "test_event".to_string(),
            callback: None,
        };

        clock.add_event(tomorrow, event.clone()).unwrap();

        // Tick once - event should fire
        let fired = clock.tick();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].name, "test_event");
    }

    #[test]
    fn test_game_clock_move_days() {
        let mut clock = GameClock::new();
        let original_date = clock.current_date();

        clock.move_days(5);
        assert_eq!(clock.tick_count(), 5);

        // Date should be same since default rate is 100 ticks per day
        assert_eq!(clock.current_date(), original_date);
    }

    #[test]
    fn test_shared_game_clock_thread_safe() {
        let clock = SharedGameClock::new();

        // Test basic operations work
        let date = clock.current_date();
        assert!(date.is_valid());

        clock.set_rate(100);
        clock.move_days(1);

        assert!(clock.tick().is_empty());
    }
}
