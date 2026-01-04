// Event Management for Game Clock
// Handles scheduling and processing of game events

use super::game_date::GameDate;
use std::sync::atomic::{AtomicU32, Ordering};

/// Event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Absolute,  // Event occurs at a specific date
    Relative,  // Event occurs N days from now
}

/// Game event
#[derive(Debug, Clone)]
pub struct Event {
    /// Unique event ID
    pub id: u32,
    /// Event type (absolute or relative)
    pub event_type: EventType,
    /// Target date (for absolute events)
    pub target_date: Option<GameDate>,
    /// Days offset (for relative events, converted to absolute date when added)
    pub days_offset: u32,
    /// Function index (callback identifier)
    pub func_index: u8,
    /// Priority (lower = higher priority)
    pub priority: u8,
}

impl Event {
    /// Create a new absolute event
    pub fn new_absolute(target_date: GameDate, func_index: u8, priority: u8) -> Self {
        static EVENT_ID_COUNTER: AtomicU32 = AtomicU32::new(0);
        
        Event {
            id: EVENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
            event_type: EventType::Absolute,
            target_date: Some(target_date),
            days_offset: 0,
            func_index,
            priority,
        }
    }

    /// Create a new relative event
    pub fn new_relative(days_offset: u32, func_index: u8, priority: u8) -> Self {
        static EVENT_ID_COUNTER: AtomicU32 = AtomicU32::new(0);
        
        Event {
            id: EVENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
            event_type: EventType::Relative,
            target_date: None,
            days_offset,
            func_index,
            priority,
        }
    }

    /// Convert a relative event to absolute based on a base date
    pub fn to_absolute(&mut self, base_date: GameDate) {
        if self.event_type == EventType::Relative {
            self.target_date = Some(base_date.add_days(self.days_offset));
            self.event_type = EventType::Absolute;
            self.days_offset = 0;
        }
    }

    /// Check if the event is due on a specific date
    pub fn is_due_on(&self, date: GameDate) -> bool {
        self.target_date.map(|d| d == date).unwrap_or(false)
    }

    /// Check if the event is due before or on a specific date
    pub fn is_due_by(&self, date: GameDate) -> bool {
        self.target_date
            .map(|d| {
                let event_ordinal = d.to_ordinal();
                let target_ordinal = date.to_ordinal();
                event_ordinal <= target_ordinal
            })
            .unwrap_or(false)
    }

    /// Get the ordinal day of the target date (for sorting)
    pub fn ordinal(&self) -> Option<u64> {
        self.target_date.map(|d| d.to_ordinal())
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Event {}

/// Event manager for handling collections of events
#[derive(Debug)]
pub struct EventManager {
    events: Vec<Event>,
}

impl EventManager {
    /// Create a new event manager
    pub fn new() -> Self {
        EventManager { events: Vec::new() }
    }

    /// Add an event
    pub fn add(&mut self, event: Event) {
        self.events.push(event);
        self.sort();
    }

    /// Add an event and convert to absolute based on current date
    pub fn add_relative(&mut self, mut event: Event, current_date: GameDate) {
        if event.event_type == EventType::Relative {
            event.to_absolute(current_date);
        }
        self.add(event);
    }

    /// Remove an event by ID
    pub fn remove(&mut self, event_id: u32) -> Option<Event> {
        if let Some(pos) = self.events.iter().position(|e| e.id == event_id) {
            Some(self.events.remove(pos))
        } else {
            None
        }
    }

    /// Get an event by ID
    pub fn get(&self, event_id: u32) -> Option<&Event> {
        self.events.iter().find(|e| e.id == event_id)
    }

    /// Get all events due on a specific date
    pub fn get_due_events(&self, date: GameDate) -> Vec<Event> {
        self.events
            .iter()
            .filter(|e| e.is_due_on(date))
            .cloned()
            .collect()
    }

    /// Get the next upcoming event
    pub fn get_next_event(&self, current_date: GameDate) -> Option<&Event> {
        self.events
            .iter()
            .filter(|e| {
                e.target_date
                    .map(|d| {
                        let event_ordinal = d.to_ordinal();
                        let current_ordinal = current_date.to_ordinal();
                        event_ordinal >= current_ordinal
                    })
                    .unwrap_or(false)
            })
            .min_by_key(|e| e.ordinal())
    }

    /// Remove all events up to (and including) a specific date
    pub fn remove_up_to(&mut self, date: GameDate) -> Vec<Event> {
        let (to_remove, to_keep): (Vec<Event>, Vec<Event>) = self
            .events
            .drain(..)
            .partition(|e| e.is_due_by(date));
        
        self.events = to_keep;
        to_remove
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Get the number of events
    pub fn count(&self) -> usize {
        self.events.len()
    }

    /// Check if there are any events
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Sort events by date and priority
    fn sort(&mut self) {
        self.events.sort_by(|a, b| {
            // First compare by date
            match (a.ordinal(), b.ordinal()) {
                (Some(a_ord), Some(b_ord)) => {
                    match a_ord.cmp(&b_ord) {
                        std::cmp::Ordering::Equal => {
                            // Same date, compare by priority (lower = higher priority)
                            a.priority.cmp(&b.priority)
                        }
                        other => other,
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }

    /// Get iterator over all events
    pub fn iter(&self) -> impl Iterator<Item = &Event> {
        self.events.iter()
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_new_absolute() {
        let date = GameDate::new(15, 6, 2155);
        let event = Event::new_absolute(date, 1, 10);

        assert_eq!(event.event_type, EventType::Absolute);
        assert_eq!(event.target_date, Some(date));
        assert_eq!(event.days_offset, 0);
        assert_eq!(event.func_index, 1);
        assert_eq!(event.priority, 10);
    }

    #[test]
    fn test_event_new_relative() {
        let event = Event::new_relative(5, 1, 10);

        assert_eq!(event.event_type, EventType::Relative);
        assert_eq!(event.target_date, None);
        assert_eq!(event.days_offset, 5);
        assert_eq!(event.func_index, 1);
        assert_eq!(event.priority, 10);
    }

    #[test]
    fn test_event_to_absolute() {
        let base_date = GameDate::new(1, 1, 2000);
        let mut event = Event::new_relative(10, 1, 10);
        
        event.to_absolute(base_date);
        
        assert_eq!(event.event_type, EventType::Absolute);
        assert_eq!(event.target_date, Some(base_date.add_days(10)));
        assert_eq!(event.days_offset, 0);
    }

    #[test]
    fn test_event_is_due_on() {
        let date = GameDate::new(15, 6, 2155);
        let event = Event::new_absolute(date, 1, 10);
        
        assert!(event.is_due_on(date));
        assert!(!event.is_due_on(date.add_days(1)));
        assert!(!event.is_due_on(date.sub_days(1)));
    }

    #[test]
    fn test_event_is_due_by() {
        let date = GameDate::new(15, 6, 2155);
        let event = Event::new_absolute(date, 1, 10);
        
        assert!(event.is_due_by(date));
        assert!(event.is_due_by(date.add_days(1)));
        assert!(!event.is_due_by(date.sub_days(1)));
    }

    #[test]
    fn test_event_ordinal() {
        let date = GameDate::new(15, 6, 2155);
        let event = Event::new_absolute(date, 1, 10);
        
        assert!(event.ordinal().is_some());
        assert_eq!(event.ordinal(), Some(date.to_ordinal()));
    }

    #[test]
    fn test_event_manager_new() {
        let manager = EventManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_event_manager_add() {
        let mut manager = EventManager::new();
        let date = GameDate::new(1, 1, 2000);
        let event = Event::new_absolute(date, 1, 10);
        
        manager.add(event.clone());
        
        assert_eq!(manager.count(), 1);
        assert!(!manager.is_empty());
        assert_eq!(manager.get(event.id), Some(&event));
    }

    #[test]
    fn test_event_manager_add_relative() {
        let mut manager = EventManager::new();
        let base_date = GameDate::new(1, 1, 2000);
        let event = Event::new_relative(10, 1, 10);
        
        manager.add_relative(event.clone(), base_date);
        
        assert_eq!(manager.count(), 1);
        
        let added_event = manager.get(event.id).unwrap();
        assert_eq!(added_event.target_date, Some(base_date.add_days(10)));
    }

    #[test]
    fn test_event_manager_remove() {
        let mut manager = EventManager::new();
        let date = GameDate::new(1, 1, 2000);
        let event = Event::new_absolute(date, 1, 10);
        
        manager.add(event.clone());
        
        let removed = manager.remove(event.id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, event.id);
        
        assert!(manager.is_empty());
    }

    #[test]
    fn test_event_manager_get_due_events() {
        let mut manager = EventManager::new();
        let date1 = GameDate::new(10, 6, 2155);
        let date2 = GameDate::new(15, 6, 2155);
        let date3 = GameDate::new(20, 6, 2155);
        
        manager.add(Event::new_absolute(date1, 1, 10));
        manager.add(Event::new_absolute(date2, 2, 10));
        manager.add(Event::new_absolute(date3, 3, 10));
        
        let due = manager.get_due_events(date2);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].func_index, 2);
    }

    #[test]
    fn test_event_manager_get_next_event() {
        let mut manager = EventManager::new();
        let current = GameDate::new(1, 1, 2000);
        
        manager.add(Event::new_absolute(current.add_days(10), 1, 10));
        manager.add(Event::new_absolute(current.add_days(5), 2, 10));
        manager.add(Event::new_absolute(current.add_days(15), 3, 10));
        
        let next = manager.get_next_event(current);
        assert!(next.is_some());
        assert_eq!(next.unwrap().func_index, 2);
    }

    #[test]
    fn test_event_manager_remove_up_to() {
        let mut manager = EventManager::new();
        let current = GameDate::new(1, 1, 2000);
        
        manager.add(Event::new_absolute(current.add_days(5), 1, 10));
        manager.add(Event::new_absolute(current.add_days(10), 2, 10));
        manager.add(Event::new_absolute(current.add_days(15), 3, 10));
        
        let removed = manager.remove_up_to(current.add_days(10));
        assert_eq!(removed.len(), 2);
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_event_manager_clear() {
        let mut manager = EventManager::new();
        let date = GameDate::new(1, 1, 2000);
        
        manager.add(Event::new_absolute(date, 1, 10));
        manager.add(Event::new_absolute(date.add_days(5), 2, 10));
        manager.add(Event::new_absolute(date.add_days(10), 3, 10));
        
        assert_eq!(manager.count(), 3);
        
        manager.clear();
        
        assert!(manager.is_empty());
    }

    #[test]
    fn test_event_manager_iter() {
        let mut manager = EventManager::new();
        let date = GameDate::new(1, 1, 2000);
        
        manager.add(Event::new_absolute(date, 1, 10));
        manager.add(Event::new_absolute(date.add_days(5), 2, 10));
        
        let events: Vec<_> = manager.iter().collect();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_event_manager_sorting_by_date() {
        let mut manager = EventManager::new();
        let date = GameDate::new(1, 1, 2000);
        
        // Add events out of order
        manager.add(Event::new_absolute(date.add_days(15), 3, 10));
        manager.add(Event::new_absolute(date.add_days(5), 2, 10));
        manager.add(Event::new_absolute(date.add_days(10), 1, 10));
        
        // Should be sorted by date
        let event_ids: Vec<_> = manager.iter().map(|e| e.func_index).collect();
        assert_eq!(event_ids, vec![2, 1, 3]);
    }

    #[test]
    fn test_event_manager_sorting_by_priority() {
        let mut manager = EventManager::new();
        let date = GameDate::new(1, 1, 2000);
        
        // Add events with same date but different priorities
        manager.add(Event::new_absolute(date, 3, 30)); // Low priority
        manager.add(Event::new_absolute(date, 1, 10)); // High priority
        manager.add(Event::new_absolute(date.add_days(5), 2, 20)); // Different date
        
        // First should be the high priority event
        let events: Vec<_> = manager.iter().collect();
        assert_eq!(events[0].func_index, 1);
        assert_eq!(events[0].priority, 10);
    }

    #[test]
    fn test_event_partial_eq() {
        let event1 = Event::new_absolute(GameDate::new(1, 1, 2000), 1, 10);
        let event2 = event1.clone();
        assert_eq!(event1, event2);

        let event3 = Event::new_absolute(GameDate::new(2, 1, 2000), 1, 10);
        assert_ne!(event1, event3);
    }

    #[test]
    fn test_event_debug() {
        let date = GameDate::new(1, 1, 2000);
        let event = Event::new_absolute(date, 1, 10);
        
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("id:"));
        assert!(debug_str.contains("func_index: 1"));
    }

    #[test]
    fn test_unique_event_ids() {
        let date = GameDate::new(1, 1, 2000);
        
        let event1 = Event::new_absolute(date, 1, 10);
        let event2 = Event::new_absolute(date, 2, 10);
        let event3 = Event::new_absolute(date, 3, 10);
        
        assert_ne!(event1.id, event2.id);
        assert_ne!(event2.id, event3.id);
        assert_ne!(event1.id, event3.id);
    }
}
