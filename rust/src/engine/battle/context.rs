//! The element-loop owner and borrowed views needed while legacy code remains.

use super::{record::QUEUE, table::ElementTable};
use std::ptr::NonNull;

pub struct Battle {
    pub(super) terminal_outcome: Option<super::RaceState>,
    pub(super) kernel: super::record::Kernel,
    pub zoom_out: i16,
    pub nth_frame: u16,
    pub max_zoom_out: i16,
    pub counters: [u8; 2],
    pub net: Option<std::rc::Rc<dyn super::net::BattleNet>>,
}

impl Battle {
    pub fn reset_element_capacity(&mut self, capacity: usize) {
        self.kernel.elements.reset_capacity(capacity);
    }

    /// Start a battle, including encounters with no Sa-Matra descriptor.
    pub fn initialize(&mut self) {
        self.terminal_outcome = None;
    }

    pub fn network(&self) -> std::rc::Rc<dyn super::net::BattleNet> {
        self.net
            .clone()
            .unwrap_or_else(|| panic!("battle has no network session"))
    }

    pub fn ship_lost(&mut self, side: usize) {
        self.counters[side] = self.counters[side].wrapping_sub(1);
    }

    #[cfg(not(test))]
    pub(crate) fn master_queue(&mut self) -> &mut super::kernel::MasterCatalog {
        &mut self.kernel.master_queue
    }

    #[cfg(not(test))]
    pub(crate) fn kernel_channels(&mut self) -> &mut super::record::Channels {
        &mut self.kernel.channels
    }

    #[cfg(not(test))]
    pub(crate) fn kernel_sounds(&mut self) -> &mut super::record::SoundQueue {
        &mut self.kernel.sounds
    }

    pub fn new(capacity: usize) -> Self {
        let mut kernel = super::record::Kernel::new();
        kernel.elements.reset_capacity(capacity);
        Self {
            terminal_outcome: None,
            kernel,
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
    pub(super) activity: NonNull<u16>,
    pub(super) groups: NonNull<QUEUE>,
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
    ) -> Self {
        Self {
            shared_frames: shared_frames.map(super::views::Frames),
            activity: NonNull::new(activity).unwrap_or_else(|| panic!("null activity projection")),
            groups: NonNull::new(groups).unwrap_or_else(|| panic!("null group queue projection")),
        }
    }

    pub fn activity(&self) -> u16 {
        // SAFETY: construction guarantees the projection's validity and exclusivity.
        unsafe { self.activity.as_ptr().read() }
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
        &mut self.b.kernel.elements
    }
    pub fn random(&mut self) -> u32 {
        (self.rng)()
    }
    pub fn elements(&self) -> &ElementTable {
        &self.b.kernel.elements
    }
    pub(crate) fn groups(&self) -> &QUEUE {
        self.project(self.world.groups)
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

impl Cx<'_> {
    #[cfg(not(test))]
    pub(crate) fn master_queue(&mut self) -> &mut super::kernel::MasterCatalog {
        self.b.master_queue()
    }
}
