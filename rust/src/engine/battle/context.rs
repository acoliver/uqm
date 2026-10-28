//! The element-loop owner and borrowed views needed while legacy code remains.

use super::{record::QUEUE, table::ElementTable};
use std::ptr::NonNull;

pub struct Battle {
    pub elements: ElementTable,
    pub zoom_out: i16,
    pub nth_frame: u16,
    pub max_zoom_out: i16,
    pub counters: [u8; 2],
    pub net: Option<std::rc::Rc<dyn super::net::BattleNet>>,
}

impl Battle {
    pub fn ship_lost(&mut self, side: usize) {
        self.counters[side] = self.counters[side].wrapping_sub(1);
    }

    /// # Safety
    /// `queue` is the one game-thread element authority, kept live until cleanup.
    pub unsafe fn new(queue: *mut QUEUE) -> Self {
        Self {
            // SAFETY: the caller owns the queue for this battle context's lifetime.
            elements: unsafe { ElementTable::from_queue(queue) },
            zoom_out: 1 << 8,
            nth_frame: 0,
            max_zoom_out: 0,
            counters: [0; 2],
            net: None,
        }
    }
}

/// Projections into the single GlobData authority, not copies of world state.
/// Phase 4 replaces these projections with the owned World.
pub struct World {
    activity: NonNull<u16>,
    groups: NonNull<QUEUE>,
}

impl World {
    /// # Safety
    /// Pointers must project the game-thread-owned GlobData authority, remain live,
    /// and never have a Rust reference held across a legacy call.
    pub unsafe fn new(activity: *mut u16, groups: *mut QUEUE) -> Self {
        Self {
            activity: NonNull::new(activity).unwrap_or_else(|| panic!("null activity projection")),
            groups: NonNull::new(groups).unwrap_or_else(|| panic!("null group queue projection")),
        }
    }

    pub fn activity(&self) -> u16 {
        // SAFETY: construction guarantees the projection's validity and exclusivity.
        unsafe { self.activity.as_ptr().read() }
    }

    pub fn groups(&self) -> *const QUEUE {
        self.groups.as_ptr()
    }
}

pub struct Cx<'a> {
    pub b: &'a mut Battle,
    pub world: &'a mut World,
    /// The existing global stream. No seed is copied at context boundaries.
    pub rng: fn() -> u32,
}
