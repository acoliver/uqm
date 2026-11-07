//! Checked record and resource field projections for immutable observers.
use super::super::ElementId;
use super::super::ElementTable;
use super::super::{
    link_queue::{LinkId, LinkQueue, Starship},
    record::*,
    views::*,
};
use std::ffi::c_void;
impl super::super::Cx<'_> {
    pub fn element_parent_address(&self, id: ElementId) -> *mut c_void {
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
pub struct ElementObservation {
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
    pub fn decode(record: &ElementRecord, parent: usize, target: usize) -> Self {
        Self {
            state: crate::battle::access::observe_element_record(record),
            address: std::ptr::from_ref(record) as usize,
            pred: 0,
            succ: 0,
            parent,
            target,
            primitive: record.PrimIndex,
            intersect: record.IntersectControl,
        }
    }
}

pub fn element(table: &ElementTable, id: ElementId, parent: usize) -> ElementObservation {
    let record = &table[id];
    let target = record.target.map_or(0, |id| table.raw(id) as usize);
    let mut value = ElementObservation::decode(record, parent, target);
    value.pred = table.pred(id).map_or(0, |id| table.raw(id) as usize);
    value.succ = table.succ(id).map_or(0, |id| table.raw(id) as usize);
    value
}
pub struct ShipRecordObservation {
    pub state: ShipState,
    pub pick: super::super::link_queue::PickShip,
    pub address: usize,
    pub pred: usize,
    pub succ: usize,
    pub element: usize,
}

pub fn ship_record(
    pool: &super::super::link_queue::RacePool,
    id: LinkId<Starship>,
    elements: &ElementTable,
) -> ShipRecordObservation {
    let ships = LinkQueue::<Starship>::borrow(pool);
    let s = pool.pool.get(ships.key(id));
    ShipRecordObservation {
        state: ShipState::decode(s),
        pick: ships.get(id),
        address: ships.raw(id) as usize,
        pred: ships.pred(id).map_or(0, |id| ships.raw(id) as usize),
        succ: ships.succ(id).map_or(0, |id| ships.raw(id) as usize),
        element: s.hShip.map_or(0, |id| elements.journal_address(id)),
    }
}

#[cfg(not(test))]
pub struct ShipResourceObservation<'a> {
    pub descriptor: Option<DescriptorRecordObservation<'a>>,
    pub icons: Option<&'a crate::graphics::native_abi::frame_desc>,
    pub strings: Option<&'a crate::strings::string_table::StringTableEntry>,
}
#[cfg(not(test))]
pub struct DescriptorRecordObservation<'a> {
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
#[cfg(not(test))]
pub unsafe fn ship_resources(
    pool: &super::super::link_queue::RacePool,
    id: LinkId<Starship>,
) -> ShipResourceObservation<'_> {
    let ships = LinkQueue::<Starship>::borrow(pool);
    let s = pool.pool.get(ships.key(id));
    let icons = s
        .picker_resources
        .as_ref()
        .map_or(std::ptr::null_mut(), |resources| resources.icon().raw());
    ShipResourceObservation {
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
        // SAFETY: the caller retains the loaded icon array for the observation lifetime.
        icons: unsafe { icons.as_ref() },
        strings: s
            .picker_resources
            .as_ref()
            .and_then(|resources| resources.strings.as_ref())
            .map(crate::strings::owned_table::TextEntry::record),
    }
}

#[cfg(test)]
mod owner_tests {
    use super::super::super::{Battle, Cx, Parent};

    #[test]
    fn scalar_ship_observation_does_not_follow_resource_handles() {
        let mut game = crate::Game::new();
        game.kernel.reset_ship_capacity(0, 1);
        let mut ships = game.kernel.ships(0);
        let link = ships.alloc().unwrap();
        let record = ships.record_mut(link);
        record.ship_facing = 9;
        record.crew_level = 42;
        let mut descriptor = crate::battle::record::RaceDescriptor::EMPTY;
        descriptor.ship_info.icons = std::ptr::without_provenance_mut(0x1234);
        descriptor.ship_info.race_strings = std::ptr::without_provenance_mut(0x5678);
        record.descriptor = Some(Box::new(descriptor));
        ships.put(link);
        let observed = super::ship_record(&game.kernel.races[0], link, &game.kernel.elements);
        assert_eq!(observed.state.facing, 9);
        assert_eq!(observed.state.crew, 42);
        assert_eq!(observed.pick.crew, 42);
        assert_ne!(observed.address, 0);
        assert_eq!(observed.pred, 0);
        assert_eq!(observed.succ, 0);
        assert_eq!(observed.element, 0);
    }

    #[test]
    fn callback_parent_identity_uses_the_retained_kernel_without_publication() {
        let mut game = crate::Game::new();
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
        let observed = super::element(
            cx.elements(),
            element,
            cx.element_parent_address(element) as usize,
        );
        assert_eq!(observed.parent, address as usize);
    }
}
