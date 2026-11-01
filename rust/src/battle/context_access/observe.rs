//! Immutable observations of typed element and ship links for automation.
use crate::comm::locdata::CQueue;
use crate::engine::battle::access::observe::{ElementObservation, ELEMENT_LINKS, SHIP_LINKS};
use crate::engine::battle::{
    link_queue::{LinkQueue, Starship},
    record::*,
    views::*,
};
use crate::savegame::runtime::queues::{walk_layout, QueueKind};

/// # Safety
/// The queue and pool stay live and immutable until this observation completes.
pub(crate) unsafe fn elements(
    queue: &CQueue,
    #[cfg(not(test))] elements: &crate::engine::battle::ElementTable,
) -> Vec<ElementObservation> {
    if queue.pq_tab.is_null() && queue.head.is_null() {
        assert!(
            queue.tail.is_null(),
            "uninitialized display queue has a tail"
        );
        return Vec::new();
    }
    walk_layout(queue, QueueKind::DisplayList, ELEMENT_LINKS)
        .unwrap_or_else(|e| panic!("live display queue: {e}"));
    let mut result = Vec::new();
    #[cfg(not(test))]
    {
        let mut next = elements.head();
        while let Some(id) = next {
            next = elements.succ(id);
            result.push(crate::engine::battle::access::observe::element(
                elements, id,
            ));
        }
    }
    #[cfg(test)]
    for pointer in walk_layout(queue, QueueKind::DisplayList, ELEMENT_LINKS).unwrap() {
        result.push(ElementObservation::decode(&*pointer.cast()));
    }
    result
}

pub(crate) struct ShipObservation<'a> {
    pub state: ShipState,
    pub pick: crate::engine::battle::link_queue::PickShip,
    pub address: usize,
    pub pred: usize,
    pub succ: usize,
    pub element: usize,
    #[cfg(test)]
    lifetime: std::marker::PhantomData<&'a ()>,
    #[cfg(not(test))]
    pub icons: Option<&'a crate::graphics::native_abi::frame_desc>,
    #[cfg(not(test))]
    pub strings: Option<&'a crate::strings::string_table::StringTableEntry>,
    #[cfg(not(test))]
    pub race: Option<DescriptorObservation<'a>>,
}
#[cfg(not(test))]
pub(crate) struct DescriptorObservation<'a> {
    pub address: usize,
    pub info: &'a SHIP_INFO,
    pub data: &'a DATA_STUFF,
    pub fleet: &'a FLEET_STUFF,
    pub characteristics: &'a CHARACTERISTIC_STUFF,
    pub intel: &'a INTEL_STUFF,
    pub callbacks: [usize; 5],
    pub code_loaded: bool,
    pub custom: CustomData<'a>,
}
pub(crate) enum CustomData<'a> {
    None,
    #[cfg(not(test))]
    Androsynth(Option<crate::engine::battle::CollisionHook>),
    #[cfg(not(test))]
    NativePkunk {
        phoenix: usize,
        saved: [Option<crate::engine::battle::ElementHook>; 3],
    },
    #[cfg(not(test))]
    NativeUmgah(u16),
    Mmrnmhrm(&'a CHARACTERISTIC_STUFF),
    #[cfg(not(test))]
    Sis(&'a crate::engine::battle::race::SisState),
}
#[cfg(not(test))]
impl<'a> CustomData<'a> {
    unsafe fn decode(_species: u32, state: &'a crate::engine::battle::RaceState) -> Self {
        use crate::engine::battle::RaceState;
        match state {
            RaceState::Androsynth { collision } => return Self::Androsynth(*collision),
            RaceState::Mmrnmhrm { alternate } => return Self::Mmrnmhrm(alternate),
            RaceState::Pkunk {
                phoenix,
                saved_pre,
                saved_post,
                saved_death,
            } => {
                let address = phoenix.map_or(0, |id| {
                    crate::engine::battle::access::observe::retired_element_address(id)
                });
                return Self::NativePkunk {
                    phoenix: address,
                    saved: [*saved_pre, *saved_post, *saved_death],
                };
            }
            RaceState::Sis(data) => return Self::Sis(data),
            RaceState::Umgah { previous_facing } => {
                return previous_facing.map_or(Self::None, Self::NativeUmgah)
            }
            _ => {}
        }
        Self::None
    }
}
/// # Safety
/// The queue, ship resources and descriptor allocations remain immutable for the borrow.
pub(crate) unsafe fn ships<'a>(queue: &'a CQueue) -> Vec<ShipObservation<'a>> {
    if queue.pq_tab.is_null() && queue.head.is_null() {
        assert!(queue.tail.is_null(), "uninitialized ship queue has a tail");
        return Vec::new();
    }
    walk_layout(queue, QueueKind::PlayerShips, SHIP_LINKS)
        .unwrap_or_else(|e| panic!("live ship queue: {e}"));
    // SAFETY: validation above checked this immutable pool's layout and links.
    let ships = unsafe { LinkQueue::<Starship>::borrow(&*std::ptr::from_ref(queue).cast()) };
    let mut result = Vec::new();
    let mut next = ships.head();
    while let Some(id) = next {
        let s = crate::engine::battle::access::observe::ship(&ships, id);
        next = ships.succ(id);
        #[cfg(not(test))]
        let (race, icons, strings) = {
            let race = s.descriptor.map(|d| DescriptorObservation {
                address: d.address,
                info: d.info,
                data: d.data,
                fleet: d.fleet,
                characteristics: d.characteristics,
                intel: d.intel,
                callbacks: d.callbacks,
                code_loaded: d.code_loaded,
                custom: CustomData::decode(s.pick.species, d.state),
            });
            (race, s.icons, s.strings)
        };
        result.push(ShipObservation {
            // SAFETY: the retained ship pool remains immutable during observation.
            state: s.state,
            pick: s.pick,
            address: s.address,
            pred: s.pred,
            succ: s.succ,
            element: s.element,
            #[cfg(not(test))]
            icons,
            #[cfg(not(test))]
            strings,
            #[cfg(not(test))]
            race,
            #[cfg(test)]
            lifetime: std::marker::PhantomData,
        });
    }
    result
}
