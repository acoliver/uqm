//! The element-loop owner and borrowed views needed while legacy code remains.

use super::table::ElementTable;

pub struct Battle {
    pub(super) terminal_outcome: Option<super::RaceState>,
    pub zoom_out: i16,
    pub nth_frame: u16,
    pub max_zoom_out: i16,
    pub counters: [u8; 2],
    pub net: Option<std::rc::Rc<dyn super::net::BattleNet>>,
    pub selection_frames: Option<[crate::graphics::context_abi::ContextFrame; 2]>,
}

impl Battle {
    pub fn initialize(&mut self, kernel: &mut super::record::Kernel) {
        kernel.begin_battle();
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

    pub fn new() -> Self {
        Self {
            terminal_outcome: None,
            zoom_out: 1 << 8,
            nth_frame: 0,
            max_zoom_out: 0,
            counters: [0; 2],
            net: None,
            selection_frames: None,
        }
    }
}

impl super::record::Kernel {
    pub fn ships(
        &mut self,
        side: usize,
    ) -> super::link_queue::LinkQueue<'_, super::link_queue::Starship> {
        super::link_queue::LinkQueue::new(&mut self.races[side])
    }
    pub fn reset_ship_capacity(&mut self, side: usize, capacity: usize) {
        self.races[side].reset_capacity(capacity);
    }
    pub fn reset_element_capacity(&mut self, capacity: usize) {
        self.elements.reset_capacity(capacity);
    }
}

pub use crate::engine::world::World;

pub struct Cx<'a> {
    pub(crate) b: &'a mut Battle,
    pub world: &'a mut World,
    pub(crate) kernel: &'a mut super::record::Kernel,
    /// The existing global stream. No seed is copied at context boundaries.
    pub(crate) rng: fn() -> u32,
}

impl Cx<'_> {
    pub fn random(&mut self) -> u32 {
        (self.rng)()
    }
    pub fn kernel_mut(&mut self) -> &mut super::record::Kernel {
        self.kernel
    }
    pub fn elements(&self) -> &ElementTable {
        &self.kernel.elements
    }
    pub(crate) fn groups(
        &self,
    ) -> super::link_queue::LinkQueue<'_, crate::engine::world::queues::Group, 32> {
        self.world.ip_groups_ref()
    }
}

impl<'a> Cx<'a> {
    pub(crate) fn new(
        b: &'a mut Battle,
        world: &'a mut World,
        kernel: &'a mut super::record::Kernel,
        rng: fn() -> u32,
    ) -> Self {
        Self {
            b,
            world,
            kernel,
            rng,
        }
    }
}

impl Cx<'_> {
    pub(crate) fn kernel(&mut self) -> *mut super::record::Kernel {
        &raw mut *self.kernel
    }
}

impl Default for Battle {
    fn default() -> Self {
        Self::new()
    }
}
