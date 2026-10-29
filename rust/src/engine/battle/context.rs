//! The element-loop owner and borrowed views needed while legacy code remains.

use super::{record::QUEUE, table::ElementTable};
use std::ptr::NonNull;

pub struct Battle {
    pub(super) elements: ElementTable,
    kernel: super::record::Kernel,
    pub zoom_out: i16,
    pub nth_frame: u16,
    pub max_zoom_out: i16,
    pub counters: [u8; 2],
    pub net: Option<std::rc::Rc<dyn super::net::BattleNet>>,
}

impl Battle {
    pub fn network(&self) -> std::rc::Rc<dyn super::net::BattleNet> {
        self.net
            .clone()
            .unwrap_or_else(|| panic!("battle has no network session"))
    }

    pub fn ship_lost(&mut self, side: usize) {
        self.counters[side] = self.counters[side].wrapping_sub(1);
    }

    /// # Safety
    /// `queue` is the one game-thread element authority, kept live until cleanup.
    pub unsafe fn new(queue: *mut QUEUE) -> Self {
        Self {
            // SAFETY: the caller owns the queue for this battle context's lifetime.
            elements: unsafe { ElementTable::from_queue(queue) },
            kernel: super::record::Kernel::new(),
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
use super::record::Tracker;

pub struct World {
    pub(super) tracking: Option<Tracker>,
    pub(super) activity: NonNull<u16>,
    groups: NonNull<QUEUE>,
    pub shared_frames: [super::views::Frames; 2],
}

impl World {
    /// # Safety
    /// Pointers must project the game-thread-owned GlobData authority, remain live,
    /// and never have a Rust reference held across a legacy call. The caller must
    /// also retain exclusive game-thread access to GlobData, input, and story
    /// authorities until this World is dropped or its use is suspended.
    pub unsafe fn new(
        activity: *mut u16,
        groups: *mut QUEUE,
        shared_frames: [*mut crate::graphics::native_abi::FRAME; 2],
        _tracking: Option<Tracker>,
    ) -> Self {
        Self {
            shared_frames: shared_frames.map(super::views::Frames),
            tracking: _tracking,
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
    pub(crate) b: &'a mut Battle,
    pub world: &'a mut World,
    /// The existing global stream. No seed is copied at context boundaries.
    pub(crate) rng: fn() -> u32,
}

impl Cx<'_> {
    /// # Safety
    /// Legacy game-thread access must not retain record borrows across callbacks.
    #[cfg(not(test))]
    pub(crate) unsafe fn records(&mut self) -> &mut ElementTable {
        &mut self.b.elements
    }
    pub fn random(&mut self) -> u32 {
        (self.rng)()
    }
    pub fn elements(&self) -> &ElementTable {
        &self.b.elements
    }
}

impl<'a> Cx<'a> {
    pub(crate) fn new(b: &'a mut Battle, world: &'a mut World, rng: fn() -> u32) -> Self {
        Self { b, world, rng }
    }
}

impl Cx<'_> {
    pub(crate) fn kernel(&mut self) -> *mut super::record::Kernel {
        &raw mut self.b.kernel
    }
}
