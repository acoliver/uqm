//! Immutable observations of typed element and ship links for automation.
use crate::comm::locdata::CQueue;
use crate::engine::battle::{
    link_queue::{LinkQueue, Starship},
    record::*,
    views::*,
};
use crate::savegame::runtime::queues::{walk, QueueKind};

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
    let links = walk::<ELEMENT>(queue, QueueKind::DisplayList)
        .unwrap_or_else(|e| panic!("live display queue: {e}"));
    let mut result = Vec::new();
    for pointer in links {
        #[cfg(not(test))]
        let record = {
            let id = elements
                .id(pointer.cast())
                .unwrap_or_else(|| panic!("validated live display element"));
            &elements[id]
        };
        #[cfg(test)]
        // SAFETY: the validator checked every link in the retained local immutable test pool.
        let record = unsafe { &*pointer };
        // SAFETY: the checked display pool remains immutable during this observation.
        result.push(unsafe { ElementObservation::decode(record) });
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
    Androsynth(ElementCollisionFunc),
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
    unsafe fn decode(species: u32, descriptor: &'a RACE_DESC) -> Self {
        use crate::engine::battle::RaceState;
        match &descriptor.race_state {
            RaceState::Androsynth { collision } => return Self::Androsynth(*collision),
            RaceState::Mmrnmhrm { alternate } => return Self::Mmrnmhrm(alternate),
            RaceState::Pkunk {
                phoenix,
                saved_pre,
                saved_post,
                saved_death,
            } => {
                let address = phoenix.map_or(0, |id| {
                    // SAFETY: the snapshot retains the pool; retired tokens preserve slot addresses.
                    let queue = unsafe { &*crate::c_bindings::live_queues::display_list() };
                    assert!(
                        !queue.pq_tab.is_null(),
                        "Pkunk snapshot pool is not initialized"
                    );
                    assert!(
                        id.slot < queue.num_objects,
                        "Pkunk snapshot slot outside pool"
                    );
                    assert!(queue.object_size > 0, "Pkunk snapshot has no record stride");
                    queue.pq_tab as usize + usize::from(id.slot) * usize::from(queue.object_size)
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
        assert!(
            descriptor.data.is_null(),
            "unexpected custom data for species {species}"
        );
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
    walk::<STARSHIP>(queue, QueueKind::PlayerShips)
        .unwrap_or_else(|e| panic!("live ship queue: {e}"));
    // SAFETY: validation above checked this immutable pool's layout and links.
    let ships = unsafe { LinkQueue::<Starship>::borrow(&*std::ptr::from_ref(queue).cast()) };
    let mut result = Vec::new();
    let mut next = ships.head();
    while let Some(id) = next {
        // SAFETY: the checked id names storage retained by the caller throughout this immutable borrow.
        let s = unsafe { &*ships.raw(id).cast::<STARSHIP>() };
        next = ships.succ(id);
        // SAFETY: the caller retains every reachable resource and descriptor for this borrow.
        #[cfg(not(test))]
        let (race, icons, strings) = unsafe {
            let race = s.RaceDescPtr.as_ref().map(|d| DescriptorObservation {
                address: d as *const _ as usize,
                info: &d.ship_info,
                data: &d.ship_data,
                fleet: &d.fleet,
                characteristics: &d.characteristics,
                intel: &d.cyborg_control,
                callbacks: d.callbacks(),
                code_loaded: !d.code_ref.is_null(),
                #[cfg(not(test))]
                custom: CustomData::decode(s.SpeciesID, d),
                #[cfg(test)]
                custom: CustomData::None,
            });
            (
                race,
                s.icons
                    .cast::<crate::graphics::native_abi::frame_desc>()
                    .as_ref(),
                s.race_strings.as_ref(),
            )
        };
        result.push(ShipObservation {
            // SAFETY: the retained ship pool remains immutable during observation.
            state: unsafe { crate::engine::battle::access::observe_ship_record(s) },
            pick: ships.get(id),
            address: ships.raw(id) as usize,
            pred: s.pred as usize,
            succ: s.succ as usize,
            element: s.hShip as usize,
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
