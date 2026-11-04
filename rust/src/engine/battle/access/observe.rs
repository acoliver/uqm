//! Checked record and resource field projections for immutable observers.
use super::super::ElementId;
use super::super::ElementTable;
use super::super::{
    link_queue::{LinkId, LinkQueue, Starship},
    record::*,
    views::*,
};
use std::ffi::c_void;
/// Encode an owner identity for the existing journal, without decoding record fields.
///
/// # Safety
/// The game-thread ship and group pools must remain live throughout the observation.
unsafe fn parent_address(
    parent: super::super::Parent,
    groups: Option<
        &crate::engine::battle::link_queue::LinkQueue<'_, crate::engine::world::queues::Group, 32>,
    >,
) -> *mut c_void {
    use super::super::Parent;
    let (queue, group) = match parent {
        Parent::None => return std::ptr::null_mut(),
        Parent::Ship(id) => {
            // SAFETY: the observation retains the live ship authority.
            let queue =
                unsafe { super::super::record::ship_queue(usize::from(id.side())).as_ref() };
            return queue.pool.retired_address(usize::from(id.slot())) as *mut c_void;
        }
        Parent::IpGroup(id) => (
            groups.unwrap_or_else(|| panic!("missing IP-group authority")),
            id,
        ),
    };
    queue.journal_address(group) as *mut c_void
}

impl super::super::Cx<'_> {
    pub(crate) fn element_parent_address(&self, id: ElementId) -> *mut c_void {
        use super::super::Parent;
        match self.kernel.elements[id].parent {
            Parent::None => std::ptr::null_mut(),
            Parent::Ship(ship) => self.kernel.races[usize::from(ship.side())]
                .pool
                .retired_address(usize::from(ship.slot()))
                as *mut c_void,
            Parent::IpGroup(group) => self.groups().journal_address(group) as *mut c_void,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ElementObservation {
    pub state: ElementState,
    pub address: usize,
    pub pred: usize,
    pub succ: usize,
    pub parent: usize,
    pub target: usize,
    pub primitive: u16,
    pub intersect: INTERSECT_CONTROL,
}
impl std::ops::Deref for ElementObservation {
    type Target = ElementState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl ElementObservation {
    /// # Safety
    /// The caller retains the live source record and its reachable resource graph.
    pub(crate) unsafe fn decode(
        record: &ElementRecord,
        groups: Option<
            &crate::engine::battle::link_queue::LinkQueue<
                '_,
                crate::engine::world::queues::Group,
                32,
            >,
        >,
        target: usize,
    ) -> Self {
        Self {
            state: crate::engine::battle::access::observe_element_record(record),
            address: std::ptr::from_ref(record) as usize,
            pred: 0,
            succ: 0,
            // SAFETY: the observation retains all referenced owner pools.
            parent: unsafe { parent_address(record.parent, groups) } as usize,
            target,
            primitive: record.PrimIndex,
            intersect: record.IntersectControl,
        }
    }
}

pub(crate) fn element(
    table: &ElementTable,
    id: ElementId,
    groups: Option<
        &crate::engine::battle::link_queue::LinkQueue<'_, crate::engine::world::queues::Group, 32>,
    >,
) -> ElementObservation {
    let record = &table[id];
    let target = record.target.map_or(0, |id| table.raw(id) as usize);
    // SAFETY: the checked table retains the decoded record through this synchronous copy.
    let mut value = unsafe { ElementObservation::decode(record, groups, target) };
    value.pred = table.pred(id).map_or(0, |id| table.raw(id) as usize);
    value.succ = table.succ(id).map_or(0, |id| table.raw(id) as usize);
    value
}
#[cfg(not(test))]
pub(crate) unsafe fn retired_element_address(id: ElementId) -> usize {
    // SAFETY: the caller retains the published Kernel while taking the snapshot.
    unsafe { &**super::super::record::Kernel::current() }
        .elements
        .journal_address(id)
}
pub(crate) struct ShipRecordObservation<'a> {
    pub state: ShipState,
    pub pick: super::super::link_queue::PickShip,
    pub address: usize,
    pub pred: usize,
    pub succ: usize,
    pub element: usize,
    #[cfg(not(test))]
    pub descriptor: Option<DescriptorRecordObservation<'a>>,
    #[cfg(not(test))]
    pub icons: Option<&'a crate::graphics::native_abi::frame_desc>,
    #[cfg(not(test))]
    pub strings: Option<&'a crate::strings::string_table::StringTableEntry>,
    #[cfg(test)]
    pub lifetime: std::marker::PhantomData<&'a ()>,
}
#[cfg(not(test))]
pub(crate) struct DescriptorRecordObservation<'a> {
    pub address: usize,
    pub info: &'a SHIP_INFO,
    pub data: &'a DATA_STUFF,
    pub fleet: &'a FLEET_STUFF,
    pub characteristics: &'a CHARACTERISTIC_STUFF,
    pub intel: &'a INTEL_STUFF,
    pub callbacks: [usize; 5],
    pub code_loaded: bool,
    pub state: &'a super::super::RaceState,
}
/// # Safety
/// The caller retains every descriptor and loaded resource referenced by this immutable queue view.
pub(crate) unsafe fn ship<'a>(
    pool: &'a super::super::link_queue::RacePool,
    id: LinkId<Starship>,
    elements: &ElementTable,
) -> ShipRecordObservation<'a> {
    let ships = LinkQueue::<Starship>::borrow(pool);
    let s = pool.pool.get(ships.key(id));
    ShipRecordObservation {
        state: ShipState::decode(s),
        pick: ships.get(id),
        address: ships.raw(id) as usize,
        pred: ships.pred(id).map_or(0, |id| ships.raw(id) as usize),
        succ: ships.succ(id).map_or(0, |id| ships.raw(id) as usize),
        element: s.hShip.map_or(0, |id| elements.journal_address(id)),
        #[cfg(not(test))]
        descriptor: s
            .descriptor
            .as_deref()
            .map(|d| DescriptorRecordObservation {
                address: d as *const _ as usize,
                info: &d.ship_info,
                data: &d.ship_data,
                fleet: &d.fleet,
                characteristics: &d.characteristics,
                intel: &d.cyborg_control,
                callbacks: d.callbacks(),
                code_loaded: d.race.is_some(),
                state: &d.race_state,
            }),
        #[cfg(not(test))]
        // SAFETY: the caller retains the loaded icon array for the observation lifetime.
        icons: unsafe { s.icons.cast::<crate::graphics::native_abi::frame_desc>().as_ref() },
        #[cfg(not(test))]
        // SAFETY: the caller retains this ship's loaded string table for the observation lifetime.
        strings: unsafe { s.race_strings.as_ref() },
        #[cfg(test)]
        lifetime: std::marker::PhantomData,
    }
}

#[cfg(test)]
mod owner_tests {
    use super::super::super::{Battle, Cx, Parent};

    #[test]
    fn callback_parent_identity_uses_the_retained_kernel_without_publication() {
        let mut game = crate::engine::Game::new();
        game.kernel.reset_ship_capacity(1, 1);
        let mut ships = game.kernel.ships(1);
        let link = ships.alloc().unwrap();
        let ship = ships.ship_id(link);
        let address = ships.raw(link);
        let element = game.kernel.elements.alloc().unwrap();
        game.kernel.elements.record_mut(element).parent = Parent::Ship(ship);
        let mut battle = Battle::new();
        let cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        assert_eq!(cx.element_parent_address(element), address);
    }
}
