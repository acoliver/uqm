//! Scheduled campaign events owned by the game clock.

use crate::collections::pool::OrderedPool;
use crate::engine::battle::link_queue::{queue_record, LinkId, LinkQueue, QueueRecord};
use crate::time::clock_bridge::{CEvent, NUM_EVENTS};

#[derive(Debug, PartialEq, Eq)]
pub enum ScheduledEvent {}
impl queue_record::Sealed for ScheduledEvent {}
impl QueueRecord for ScheduledEvent {
    type Record = CEvent;
}
impl LinkQueue<'_, ScheduledEvent, NUM_EVENTS> {
    pub fn get(&self, id: LinkId<ScheduledEvent>) -> &CEvent {
        self.record(id)
    }
}

pub struct GameClock {
    events: OrderedPool<CEvent, NUM_EVENTS>,
}
impl GameClock {
    pub fn new() -> Self {
        Self {
            events: OrderedPool::new(),
        }
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
