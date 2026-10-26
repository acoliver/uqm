//! Capture and restore of the "Evts" section against the live event queue.
//!
//! The queue is `GLOBAL(GameClock.event_q)`: a `QUEUE_TABLE` pool of
//! `NUM_EVENTS` slots of `EVENT`, linked head to tail in date order. Capture
//! walks the real links; restore empties the real queue and refills it with
//! `AllocLink` and `PutQueue`, the calls `load.c` makes through `AllocEvent`
//! and `PutEvent`.

use std::ffi::c_void;
use std::fmt;

use crate::collections::queue::{AllocLink, PutQueue, Queue, ReinitQueue};
use crate::savegame::types::EventEntry;
use crate::time::clock_bridge::{days_in_month, CEvent, NUM_EVENTS};

/// Why an event section could not be captured or restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventError {
    /// The queue has no slot table, so it was never initialised.
    PoolMissing,
    /// The queue's slots are not the size of an `EVENT`.
    WrongObjectSize { found: u16, expected: usize },
    /// The slot table is not aligned for `EVENT`.
    MisalignedPool,
    /// A link is outside the pool, off a slot boundary, repeated, or its
    /// back-link, or the queue's tail, disagrees with the forward links.
    CorruptQueue { position: usize },
    /// More events than the pool holds slots, or than there are event kinds.
    TooManyEvents { count: usize, capacity: usize },
    /// `func_index` names no event handler (`NUM_EVENTS` kinds exist).
    UnknownEventFunction { position: usize, func_index: u8 },
    /// The date is not a day of the calendar.
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
            Self::PoolMissing => f.write_str("the event queue has no slot table"),
            Self::WrongObjectSize { found, expected } => write!(
                f,
                "event queue slots are {found} bytes, an EVENT is {expected}"
            ),
            Self::MisalignedPool => f.write_str("the event queue slot table is misaligned"),
            Self::CorruptQueue { position } => {
                write!(f, "event queue is corrupt at position {position}")
            }
            Self::TooManyEvents { count, capacity } => {
                write!(
                    f,
                    "{count} events do not fit the {capacity}-slot event pool"
                )
            }
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

/// Slots of a queue that really is an `EVENT` pool.
fn pool_slots(queue: &Queue) -> Result<usize, EventError> {
    if queue.pq_tab.is_null() {
        return Err(EventError::PoolMissing);
    }
    let expected = std::mem::size_of::<CEvent>();
    if usize::from(queue.object_size) != expected {
        return Err(EventError::WrongObjectSize {
            found: queue.object_size,
            expected,
        });
    }
    if !(queue.pq_tab as usize).is_multiple_of(std::mem::align_of::<CEvent>()) {
        return Err(EventError::MisalignedPool);
    }
    Ok(usize::from(queue.num_objects))
}

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

/// The events in `queue`, head to tail, as a save writes them.
///
/// Every link is checked against the pool, so a queue the game corrupted is an
/// error and not a save of whatever the pointers reach.
pub fn capture_events(queue: &Queue) -> Result<Vec<EventEntry>, EventError> {
    let slots = pool_slots(queue)?;
    let pool = queue.pq_tab as usize;
    let slot_size = usize::from(queue.object_size);

    let mut captured = Vec::new();
    let mut previous: *mut c_void = std::ptr::null_mut();
    let mut link = queue.head;
    while !link.is_null() {
        let position = captured.len();
        let offset = (link as usize).wrapping_sub(pool);
        // A queue holds each slot at most once, so more links than slots is a loop.
        if position >= slots || offset >= slots * slot_size || !offset.is_multiple_of(slot_size) {
            return Err(EventError::CorruptQueue { position });
        }
        // SAFETY: `link` is an aligned slot inside the pool, checked above, and
        // the pool holds `EVENT`s (size and alignment checked by `pool_slots`).
        let event = unsafe { &*(link as *const CEvent) };
        if event.pred != previous {
            return Err(EventError::CorruptQueue { position });
        }
        let entry = EventEntry {
            day_index: event.day_index,
            month_index: event.month_index,
            year_index: event.year_index,
            func_index: event.func_index,
        };
        validate_event(position, &entry)?;
        captured.push(entry);
        previous = link;
        link = event.succ;
    }
    if previous != queue.tail {
        return Err(EventError::CorruptQueue {
            position: captured.len(),
        });
    }
    Ok(captured)
}

/// Everything `restore_events` could reject, checked without touching `queue`.
///
/// The room for events is the pool's slots, and never more than there are
/// event kinds.
pub fn validate_restore(queue: &Queue, events: &[EventEntry]) -> Result<(), EventError> {
    let capacity = pool_slots(queue)?.min(NUM_EVENTS);
    if events.len() > capacity {
        return Err(EventError::TooManyEvents {
            count: events.len(),
            capacity,
        });
    }
    events
        .iter()
        .enumerate()
        .try_for_each(|(position, event)| validate_event(position, event))
}

/// Empty `queue` and fill it with `events`, in order.
///
/// On error `queue` is untouched.
pub fn restore_events(queue: &mut Queue, events: &[EventEntry]) -> Result<(), EventError> {
    validate_restore(queue, events)?;
    install_events(queue, events);
    Ok(())
}

/// The mutation half of `restore_events`, for callers that validated already.
pub(super) fn install_events(queue: &mut Queue, events: &[EventEntry]) {
    unsafe {
        // SAFETY: The queue is initialized and exclusively borrowed; its previous links are discarded.
        ReinitQueue(queue)
    };
    for entry in events {
        let link = unsafe {
            // SAFETY: The initialized queue is exclusively borrowed and its free-list belongs to its pool.
            AllocLink(queue)
        };
        assert!(
            !link.is_null(),
            "event pool ran out of slots after validation"
        );
        // SAFETY: `link` is a free slot of the pool, which holds aligned
        // `EVENT`s (checked by `validate_restore`); `PutQueue` links it next.
        unsafe {
            (link as *mut CEvent).write(CEvent {
                pred: std::ptr::null_mut(),
                succ: std::ptr::null_mut(),
                day_index: entry.day_index,
                month_index: entry.month_index,
                year_index: entry.year_index,
                func_index: entry.func_index,
            });
        }
        unsafe {
            // SAFETY: The queue is exclusively borrowed and the element was allocated from this pool.
            PutQueue(queue, link)
        };
    }
}
