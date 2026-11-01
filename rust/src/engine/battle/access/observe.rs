//! Checked record and resource field projections for immutable observers.
use super::super::{
    link_queue::{LinkId, LinkQueue, Starship},
    record::*,
    views::*,
};
#[cfg(not(test))]
use super::super::{ElementId, ElementTable};
#[cfg(not(test))]
use std::ffi::c_void;
pub(crate) const SHIP_LINKS: crate::collections::queue::LinkLayout =
    crate::collections::queue::LinkLayout {
        size: std::mem::size_of::<STARSHIP>(),
        align: std::mem::align_of::<STARSHIP>(),
        read: |p| {
            // SAFETY: the queue walker checked the retained ship slot's layout and bounds.
            let r = unsafe { &*p.cast::<STARSHIP>().as_ptr() };
            (r.pred, r.succ)
        },
    };
pub(crate) const ELEMENT_LINKS: crate::collections::queue::LinkLayout =
    crate::collections::queue::LinkLayout {
        size: std::mem::size_of::<ELEMENT>(),
        align: std::mem::align_of::<ELEMENT>(),
        read: |p| {
            // SAFETY: the queue walker checked the retained element slot's layout and bounds.
            let r = unsafe { &*p.cast::<ELEMENT>().as_ptr() };
            (r.pred, r.succ)
        },
    };
impl super::super::Cx<'_> {
    #[cfg(not(test))]
    pub(crate) fn element_parent_address(&self, id: ElementId) -> *mut c_void {
        self.b.elements[id].pParent
    }
}
#[derive(Clone, Copy)]
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
    pub(crate) unsafe fn decode(record: &ELEMENT) -> Self {
        Self {
            // SAFETY: the snapshot retains the source element and its live graph.
            state: unsafe { crate::engine::battle::access::observe_element_record(record) },
            address: std::ptr::from_ref(record) as usize,
            pred: record.pred as usize,
            succ: record.succ as usize,
            parent: record.pParent as usize,
            target: record.hTarget as usize,
            primitive: record.PrimIndex,
            intersect: record.IntersectControl,
        }
    }
}

#[cfg(not(test))]
pub(crate) fn element(table: &ElementTable, id: ElementId) -> ElementObservation {
    // SAFETY: the checked table retains the decoded record through this synchronous copy.
    unsafe { ElementObservation::decode(&table[id]) }
}
#[cfg(not(test))]
pub(crate) unsafe fn retired_element_address(id: ElementId) -> usize {
    let pointer = &raw const super::super::legacy::disp_q;
    let queue = &*pointer;
    assert!(
        !queue.pq_tab.is_null(),
        "Pkunk snapshot pool is not initialized"
    );
    assert!(
        id.slot < queue.num_objects,
        "Pkunk snapshot slot outside pool"
    );
    assert!(queue.object_size > 0, "Pkunk snapshot has no record stride");
    queue
        .pq_tab
        .wrapping_add(usize::from(id.slot) * usize::from(queue.object_size)) as usize
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
    ships: &LinkQueue<'a, Starship>,
    id: LinkId<Starship>,
) -> ShipRecordObservation<'a> {
    let s = &*ships.raw(id).cast::<STARSHIP>();
    ShipRecordObservation {
        state: ShipState::decode(s),
        pick: ships.get(id),
        address: ships.raw(id) as usize,
        pred: s.pred as usize,
        succ: s.succ as usize,
        element: s.hShip as usize,
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
        icons: s
            .icons
            .cast::<crate::graphics::native_abi::frame_desc>()
            .as_ref(),
        #[cfg(not(test))]
        strings: s.race_strings.as_ref(),
        #[cfg(test)]
        lifetime: std::marker::PhantomData,
    }
}
