// Game Clock Management
// Handles game time progression with tick-based day advancement

use std::sync::{Arc, Mutex};

use super::game_date::GameDate;
use super::events::{Event, EventType};

/// Game clock state
#[derive(Debug)]
pub struct GameClock {
    pub date: GameDate,
    tick_count: usize,
    day_in_ticks: usize,
    events: Arc<Mutex<Vec<Event>>>,
    locked: bool,
}

impl GameClock {
    /// Create a new game clock with default date
    pub fn new() -> Self {
        GameClock {
            date: GameDate::default(),
            tick_count: 0,
            day_in_ticks: 0,
            events: Arc::new(Mutex::new(Vec::new())),
            locked: false,
        }
    }

    /// Create a game clock with a specific starting date
    pub fn with_date(date: GameDate) -> Self {
        let mut clock = Self::new();
        clock.date = date;
        clock
    }

    /// Get current date
    pub fn date(&self) -> GameDate {
        self.date
    }

    /// Set the clock rate (ticks per day)
    pub fn set_rate(&mut self, ticks_per_day: usize) {
        self.day_in_ticks = ticks_per_day;
        
        // Adjust tick count proportionally if the clock was already running
        if self.tick_count > 0 && self.day_in_ticks > 0 {
            // Preserve the fraction of the day that has passed
            let old_fraction = self.tick_count as f64 / (self.day_in_ticks as f64);
            if self.day_in_ticks > 0 {
                self.tick_count = (old_fraction * self.day_in_ticks as f64) as usize;
                if self.tick_count == 0 {
                    self.tick_count = 1;
                }
            }
        } else if self.tick_count == 0 {
            self.tick_count = self.day_in_ticks;
        }
    }

    /// Tick the clock forward one tick
    /// 
    /// Returns true if the day changed, false otherwise
    pub fn tick(&mut self) -> bool {
        if self.locked {
            return false;
        }

        if self.day_in_ticks == 0 {
            return false;
        }

        self.tick_count -= 1;
        
        if self.tick_count <= 0 {
            self.tick_count = self.day_in_ticks;
            self.advance_day();
            return true;
        }
        
        false
    }

    /// Advance the clock by a specific number of ticks
    pub fn advance_ticks(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.tick();
        }
    }

    /// Advance the clock by a specific number of days
    pub fn advance_days(&mut self, days: u32) {
        for _ in 0..days {
            self.advance_day();
        }
        self.tick_count = self.day_in_ticks;
    }

    /// Advance to the next day without processing ticks
    fn advance_day(&mut self) {
        self.date = self.date.add_days(1);
        self.process_day_events();
    }

    /// Get the current tick count
    pub fn tick_count(&self) -> usize {
        self.tick_count
    }

    /// Get the ticks per day
    pub fn day_in_ticks(&self) -> usize {
        self.day_in_ticks
    }

    /// Check if the clock is running
    pub fn is_running(&self) -> bool {
        self.day_in_ticks != 0
    }

    /// Lock the clock (prevents ticking - for debugging)
    pub fn lock(&mut self) {
        self.locked = true;
    }

    /// Unlock the clock
    pub fn unlock(&mut self) {
        self.locked = false;
    }

    /// Check if the clock is locked
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Add an event to the clock
    pub fn add_event(&mut self, event: Event) -> Result<(), ClockError> {
        let mut events = self.events.lock().map_err(|_| ClockError::LockError)?;
        
        // Calculate absolute date if this is a relative event
        let mut abs_event = if event.event_type == EventType::Relative {
            let mut abs_event = event;
            let target_date = self.date.add_days(event.days_offset);
            abs_event.target_date = Some(target_date);
            abs_event.event_type = EventType::Absolute;
            abs_event
        } else {
            event
        };

        // Validate event
        if let Some(target_date) = abs_event.target_date {
            if target_date < self.date {
                return Err(ClockError::EventInPast);
            }
        }

        events.push(abs_event);
        
        // Sort events by date
        events.sort_by(|a, b| {
            match (a.target_date, b.target_date) {
                (Some(ad), Some(bd)) => {
                    let a_ordinal = ad.to_ordinal();
                    let b_ordinal = bd.to_ordinal();
                    a_ordinal.cmp(&b_ordinal)
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        Ok(())
    }

    /// Get all events for a specific date
    pub fn get_events_for_date(&self, date: GameDate) -> Vec<Event> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .filter(|e| e.target_date.map(|d| d == date).unwrap_or(false))
            .cloned()
            .collect()
    }

    /// Get the next upcoming event
    pub fn get_next_event(&self) -> Option<Event> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .filter(|e| {
                e.target_date
                    .map(|d| {
                        let event_ordinal = d.to_ordinal();
                        let current_ordinal = self.date.to_ordinal();
                        event_ordinal >= current_ordinal
                    })
                    .unwrap_or(false)
            })
            .min_by_key(|e| e.target_date.map(|d| d.to_ordinal()))
            .cloned()
    }

    /// Clear all pending events
    pub fn clear_events(&mut self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }

    /// Remove a specific event by ID
    pub fn remove_event(&mut self, event_id: u32) -> Result<(), ClockError> {
        let mut events = self.events.lock().map_err(|_| ClockError::LockError)?;
        if let Some(pos) = events.iter().position(|e| e.id == event_id) {
            events.remove(pos);
            Ok(())
        } else {
            Err(ClockError::EventNotFound)
        }
    }

    /// Process events for the current day
    fn process_day_events(&mut self) {
        let mut events = self.events.lock().unwrap();
        let events_to_process: Vec<Event> = events
            .iter()
            .filter(|e| e.target_date.map(|d| d == self.date).unwrap_or(false))
            .cloned()
            .collect();

        // Remove processed events
        events.retain(|e| !e.target_date.map(|d| d == self.date).unwrap_or(false));

        // In a real implementation, we would trigger callbacks here
        // For now, we just collect them
        for event in events_to_process {
            // TODO: Trigger event callback
            let _ = event;
        }
    }

    /// Get the fraction of the current day that has passed (0.0 to 1.0)
    pub fn day_fraction(&self) -> f64 {
        if self.day_in_ticks == 0 {
            return 0.0;
        }
        
        let elapsed = self.day_in_ticks - self.tick_count;
        elapsed as f64 / self.day_in_ticks as f64
    }

    /// Reset the clock to initial state
    pub fn reset(&mut self) {
        self.date = GameDate::default();
        self.tick_count = 0;
        self.day_in_ticks = 0;
        self.locked = false;
        self.clear_events();
    }
}

impl Default for GameClock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockError {
    EventInPast,
    EventNotFound,
    LockError,
}

impl std::fmt::Display for ClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClockError::EventInPast => write!(f, "Event is in the past"),
            ClockError::EventNotFound => write!(f, "Event not found"),
            ClockError::LockError => write!(f, "Failed to acquire lock"),
        }
    }
}

impl std::error::Error for ClockError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let clock = GameClock::new();
        assert_eq!(clock.date(), GameDate::default());
        assert_eq!(clock.tick_count(), 0);
        assert_eq!(clock.day_in_ticks(), 0);
        assert!(!clock.is_running());
    }

    #[test]
    fn test_with_date() {
        let date = GameDate::new(1, 1, 2000);
        let clock = GameClock::with_date(date);
        assert_eq!(clock.date(), date);
    }

    #[test]
    fn test_set_rate() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        assert_eq!(clock.day_in_ticks(), 120);
        assert_eq!(clock.tick_count(), 120);
        assert!(clock.is_running());
    }

    #[test]
    fn test_tick() {
        let mut clock = GameClock::new();
        clock.set_rate(10);
        
        let start_date = clock.date();
        
        // 9 ticks should not advance the day
        for _ in 0..9 {
            assert!(!clock.tick());
        }
        
        assert_eq!(clock.date(), start_date);
        
        // 10th tick should advance the day
        assert!(clock.tick());
        assert_eq!(clock.date(), start_date.add_days(1));
    }

    #[test]
    fn test_advance_ticks() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        let start_date = clock.date();
        
        clock.advance_ticks(130);
        
        // Should advance about 1 day
        assert_eq!(clock.date(), start_date.add_days(1));
    }

    #[test]
    fn test_advance_days() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        let start_date = clock.date();
        
        clock.advance_days(5);
        
        assert_eq!(clock.date(), start_date.add_days(5));
        assert_eq!(clock.tick_count(), 120);
    }

    #[test]
    fn test_lock() {
        let mut clock = GameClock::new();
        clock.set_rate(10);
        
        clock.lock();
        assert!(clock.is_locked());
        
        // Ticks should be ignored when locked
        for _ in 0..20 {
            assert!(!clock.tick());
        }
        
        clock.unlock();
        assert!(!clock.is_locked());
    }

    #[test]
    fn test_day_fraction() {
        let mut clock = GameClock::new();
        clock.set_rate(100);
        
        // At start
        assert_eq!(clock.day_fraction(), 0.0);
        
        // Halfway through
        clock.advance_ticks(50);
        assert_eq!(clock.day_fraction(), 0.5);
        
        // End of day
        clock.advance_ticks(50);
        assert_eq!(clock.day_fraction(), 0.0); // Reset at day end
    }

    #[test]
    fn test_add_event_absolute() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        let target_date = clock.date().add_days(5);
        let event = Event::new_absolute(target_date, 1, 100);
        
        let result = clock.add_event(event);
        assert!(result.is_ok());
        
        let next_event = clock.get_next_event();
        assert!(next_event.is_some());
    }

    #[test]
    fn test_add_event_relative() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        let event = Event::new_relative(5, 1, 100);
        
        let result = clock.add_event(event);
        assert!(result.is_ok());
        
        let next_event = clock.get_next_event();
        assert!(next_event.is_some());
        
        // The event should be set to 5 days from now
        if let Some(e) = next_event {
            let expected_date = GameDate::default().add_days(5);
            assert_eq!(e.target_date, Some(expected_date));
        }
    }

    #[test]
    fn test_add_event_in_past() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        let past_date = clock.date().sub_days(1);
        let event = Event::new_absolute(past_date, 1, 100);
        
        let result = clock.add_event(event);
        assert_eq!(result, Err(ClockError::EventInPast));
    }

    #[test]
    fn test_get_events_for_date() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        let target_date = clock.date().add_days(10);
        let event = Event::new_absolute(target_date, 1, 100);
        clock.add_event(event).unwrap();
        
        let events = clock.get_events_for_date(target_date);
        assert_eq!(events.len(), 1);
        
        let other_date = clock.date().add_days(5);
        let events = clock.get_events_for_date(other_date);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_remove_event() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        let target_date = clock.date().add_days(10);
        let event = Event::new_absolute(target_date, 1, 100);
        clock.add_event(event.clone()).unwrap();
        
        let remove_result = clock.remove_event(event.id);
        assert!(remove_result.is_ok());
        
        let events = clock.get_events_for_date(target_date);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_clear_events() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        let target_date = clock.date().add_days(10);
        let event1 = Event::new_absolute(target_date, 1, 100);
        let event2 = Event::new_absolute(clock.date().add_days(20), 2, 200);
        
        clock.add_event(event1).unwrap();
        clock.add_event(event2).unwrap();
        
        clock.clear_events();
        
        assert!(clock.get_next_event().is_none());
    }

    #[test]
    fn test_reset() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        clock.advance_days(5);
        
        let event = Event::new_relative(10, 1, 100);
        clock.add_event(event).unwrap();
        
        clock.reset();
        
        assert_eq!(clock.date(), GameDate::default());
        assert_eq!(clock.tick_count(), 0);
        assert_eq!(clock.day_in_ticks(), 0);
        assert!(!clock.is_running());
        assert!(clock.get_next_event().is_none());
    }

    #[test]
    fn test_event_processing_on_day_advance() {
        let mut clock = GameClock::new();
        clock.set_rate(1);
        
        // Add event for tomorrow
        let event = Event::new_relative(1, 1, 100);
        clock.add_event(event.clone()).unwrap();
        
        // Advance one day
        clock.tick();
        
        // Event should be processed and removed
        let events = clock.get_events_for_date(event.target_date.unwrap());
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_multiple_events_same_day() {
        let mut clock = GameClock::new();
        clock.set_rate(120);
        
        let target_date = clock.date().add_days(10);
        let event1 = Event::new_absolute(target_date, 1, 100);
        let event2 = Event::new_absolute(target_date, 2, 200);
        let event3 = Event::new_absolute(target_date, 3, 300);
        
        clock.add_event(event1).unwrap();
        clock.add_event(event2).unwrap();
        clock.add_event(event3).unwrap();
        
        let events = clock.get_events_for_date(target_date);
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_clock_error_display() {
        let err = ClockError::EventInPast;
        let display = format!("{}", err);
        assert!(display.contains("past"));
    }
}
