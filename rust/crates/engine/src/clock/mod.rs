//! Scheduled campaign events owned by the game clock.

use super::battle::link_queue::{queue_record, LinkId, LinkQueue, QueueRecord};
use crate::collections::pool::OrderedPool;
pub const NUM_EVENTS: usize = 18;

// Selector values are stored in campaign saves; keep their IDs and native widths.
pub const ARILOU_ENTRANCE_EVENT: u32 = 0;
pub const ARILOU_EXIT_EVENT: u32 = 1;
pub const HYPERSPACE_ENCOUNTER_EVENT: u32 = 2;
pub const KOHR_AH_VICTORIOUS_EVENT: u32 = 3;
pub const ADVANCE_PKUNK_MISSION: u32 = 4;
pub const ADVANCE_THRADD_MISSION: u32 = 5;
pub const ZOQFOT_DISTRESS_EVENT: u32 = 6;
pub const ZOQFOT_DEATH_EVENT: u32 = 7;
pub const SHOFIXTI_RETURN_EVENT: u32 = 8;
pub const ADVANCE_UTWIG_SUPOX_MISSION: u32 = 9;
pub const KOHR_AH_GENOCIDE_EVENT: u32 = 10;
pub const SPATHI_SHIELD_EVENT: u32 = 11;
pub const ADVANCE_ILWRATH_MISSION: u32 = 12;
pub const ADVANCE_MYCON_MISSION: u32 = 13;
pub const ARILOU_UMGAH_CHECK: u32 = 14;
pub const YEHAT_REBEL_EVENT: u32 = 15;
pub const SLYLANDRO_RAMP_UP: u32 = 16;
pub const SLYLANDRO_RAMP_DOWN: u32 = 17;

#[derive(Default)]
pub struct CEvent {
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
    pub func_index: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScheduledEvent {}
impl queue_record::Sealed for ScheduledEvent {}
impl super::battle::link_queue::MutableQueueRecord for ScheduledEvent {}
impl QueueRecord for ScheduledEvent {
    type Record = CEvent;
}
impl LinkQueue<'_, ScheduledEvent, NUM_EVENTS> {
    pub fn get(&self, id: LinkId<ScheduledEvent>) -> &CEvent {
        self.record(id)
    }
}

pub struct GameClock {
    initialized: bool,
    events: OrderedPool<CEvent, NUM_EVENTS>,
}
impl GameClock {
    pub fn new() -> Self {
        Self {
            initialized: false,
            events: OrderedPool::new(),
        }
    }
    pub fn initialized(&self) -> bool {
        self.initialized
    }
    pub fn init(&mut self) {
        self.clear();
        self.initialized = true;
    }
    pub fn uninit(&mut self) {
        self.clear();
        self.initialized = false;
    }
    pub fn events(&mut self) -> LinkQueue<'_, ScheduledEvent, NUM_EVENTS> {
        LinkQueue::from_pool(&mut self.events)
    }
    pub fn events_ref(&self) -> LinkQueue<'_, ScheduledEvent, NUM_EVENTS> {
        LinkQueue::borrow_pool(&self.events)
    }
    pub fn clear(&mut self) {
        self.events.reinit();
    }
}
impl Default for GameClock {
    fn default() -> Self {
        Self::new()
    }
}

// Helper: Check if year is leap year
pub fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

// Helper: Get days in month
pub fn days_in_month(month: u8, year: u16) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30, // Should never happen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SELECTORS: [u32; NUM_EVENTS] = [
        ARILOU_ENTRANCE_EVENT,
        ARILOU_EXIT_EVENT,
        HYPERSPACE_ENCOUNTER_EVENT,
        KOHR_AH_VICTORIOUS_EVENT,
        ADVANCE_PKUNK_MISSION,
        ADVANCE_THRADD_MISSION,
        ZOQFOT_DISTRESS_EVENT,
        ZOQFOT_DEATH_EVENT,
        SHOFIXTI_RETURN_EVENT,
        ADVANCE_UTWIG_SUPOX_MISSION,
        KOHR_AH_GENOCIDE_EVENT,
        SPATHI_SHIELD_EVENT,
        ADVANCE_ILWRATH_MISSION,
        ADVANCE_MYCON_MISSION,
        ARILOU_UMGAH_CHECK,
        YEHAT_REBEL_EVENT,
        SLYLANDRO_RAMP_UP,
        SLYLANDRO_RAMP_DOWN,
    ];

    #[test]
    fn campaign_selectors_keep_native_width_and_save_ids() {
        assert_eq!(
            SELECTORS,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
        );
    }

    #[test]
    fn campaign_selectors_survive_owned_queue_save_round_trip() {
        let entries: Vec<_> = SELECTORS
            .into_iter()
            .rev()
            .map(|selector| uqm_save::EventEntry {
                day_index: 17,
                month_index: 3,
                year_index: 2155,
                func_index: selector as u8,
            })
            .collect();
        let mut clock = GameClock::new();
        crate::save::events::restore_events(&mut clock, &entries).unwrap();
        assert_eq!(
            crate::save::events::capture_events(&clock).unwrap(),
            entries
        );
    }

    #[test]
    fn pool_reset_preserves_clock_lifecycle_but_uninit_ends_it() {
        let mut clock = GameClock::new();
        assert!(!clock.initialized());
        clock.init();
        assert!(clock.initialized());
        let id = clock.events().alloc().unwrap();
        clock.events().put(id);
        clock.clear();
        assert!(clock.initialized());
        assert!(!clock.events_ref().contains(id));
        let replacement = clock.events().alloc().unwrap();
        clock.events().put(replacement);
        clock.uninit();
        assert!(!clock.initialized());
        assert_eq!(clock.events_ref().count(), 0);
        assert!(!clock.events_ref().contains(replacement));
        clock.init();
        assert!(clock.initialized());
        assert_eq!(clock.events_ref().count(), 0);
    }
}
