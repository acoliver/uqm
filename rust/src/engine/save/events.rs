//! Capture and restore of scheduled events in save-file order.

use crate::engine::clock::days_in_month;
use crate::engine::clock::{GameClock, NUM_EVENTS};
use std::fmt;
use uqm_save::EventEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventError {
    TooManyEvents {
        count: usize,
        capacity: usize,
    },
    UnknownEventFunction {
        position: usize,
        func_index: u8,
    },
    InvalidEventDate {
        position: usize,
        day_index: u8,
        month_index: u8,
        year_index: u16,
    },
}
impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TooManyEvents { count, capacity } => write!(
                f,
                "{count} events do not fit the {capacity}-slot event pool"
            ),
            Self::UnknownEventFunction {
                position,
                func_index,
            } => write!(
                f,
                "event {position} has handler {func_index}, there are {NUM_EVENTS}"
            ),
            Self::InvalidEventDate {
                position,
                day_index,
                month_index,
                year_index,
            } => write!(
                f,
                "event {position} is dated {month_index}/{day_index}/{year_index}"
            ),
        }
    }
}
impl std::error::Error for EventError {}

/// The checks `AddEvent` makes by construction: a real handler and a real day.
fn validate_event(position: usize, event: &EventEntry) -> Result<(), EventError> {
    if usize::from(event.func_index) >= NUM_EVENTS {
        return Err(EventError::UnknownEventFunction {
            position,
            func_index: event.func_index,
        });
    }
    let month_ok = (1..=12).contains(&event.month_index);
    if !month_ok
        || event.day_index == 0
        || event.day_index > days_in_month(event.month_index, event.year_index)
    {
        return Err(EventError::InvalidEventDate {
            position,
            day_index: event.day_index,
            month_index: event.month_index,
            year_index: event.year_index,
        });
    }
    Ok(())
}

pub fn capture_events(clock: &GameClock) -> Result<Vec<EventEntry>, EventError> {
    let queue = clock.events_ref();
    let mut captured = Vec::with_capacity(queue.count());
    let mut next = queue.head();
    while let Some(id) = next {
        let event = queue.get(id);
        let entry = EventEntry {
            day_index: event.day_index,
            month_index: event.month_index,
            year_index: event.year_index,
            func_index: event.func_index,
        };
        validate_event(captured.len(), &entry)?;
        captured.push(entry);
        next = queue.succ(id);
    }
    Ok(captured)
}

pub fn validate_restore(events: &[EventEntry]) -> Result<(), EventError> {
    if events.len() > NUM_EVENTS {
        return Err(EventError::TooManyEvents {
            count: events.len(),
            capacity: NUM_EVENTS,
        });
    }
    events
        .iter()
        .enumerate()
        .try_for_each(|(position, event)| validate_event(position, event))
}

/// Validate the complete section before replacing any live event.
pub fn restore_events(clock: &mut GameClock, events: &[EventEntry]) -> Result<(), EventError> {
    validate_restore(events)?;
    install_events(clock, events);
    Ok(())
}

pub(crate) fn install_events(clock: &mut GameClock, events: &[EventEntry]) {
    let mut queue = clock.events();
    queue.reinit();
    for entry in events {
        let id = queue
            .alloc()
            .unwrap_or_else(|| panic!("event capacity validated before installation"));
        let event = queue.get_mut(id);
        event.day_index = entry.day_index;
        event.month_index = entry.month_index;
        event.year_index = entry.year_index;
        event.func_index = entry.func_index;
        queue.put(id);
    }
}

#[cfg(test)]
mod owned_tests {
    use super::*;
    #[test]
    fn owned_event_pool_preserves_file_order_and_rejects_before_reset() {
        let mut clock = crate::engine::clock::GameClock::new();
        let entries = [
            EventEntry {
                day_index: 20,
                month_index: 2,
                year_index: 2155,
                func_index: 3,
            },
            EventEntry {
                day_index: 18,
                month_index: 2,
                year_index: 2155,
                func_index: 1,
            },
        ];
        restore_events(&mut clock, &entries).unwrap();
        let first = clock.events_ref().head().unwrap();
        let mut invalid = entries.clone();
        invalid[1].month_index = 13;
        assert!(matches!(
            restore_events(&mut clock, &invalid),
            Err(EventError::InvalidEventDate { position: 1, .. })
        ));
        assert_eq!(clock.events_ref().head(), Some(first));
        assert_eq!(capture_events(&clock).unwrap(), entries);
        restore_events(&mut clock, &entries[..1]).unwrap();
        assert!(!clock.events_ref().contains(first));
        assert_eq!(capture_events(&clock).unwrap(), entries[..1]);
    }
}
