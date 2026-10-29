//! Immutable observations of typed element and ship links for automation.
use super::super::{
    link_queue::{LinkQueue, Starship},
    record::*,
    views::*,
};
use crate::comm::locdata::CQueue;
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
    pub(crate) fn decode(record: &ELEMENT) -> Self {
        Self {
            state: ElementState::decode(record),
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
    #[cfg(not(test))] elements: &super::super::ElementTable,
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
        result.push(ElementObservation::decode(record));
    }
    result
}

pub(crate) struct ShipObservation<'a> {
    pub state: ShipState,
    pub pick: super::super::link_queue::PickShip,
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
    Pkunk(&'a crate::ships::races::pkunk_native::engine::PkunkData),
    #[cfg(not(test))]
    NativePkunk {
        phoenix: usize,
        saved: [Option<super::super::ElementHook>; 3],
    },
    #[cfg(not(test))]
    Umgah(&'a crate::ships::races::umgah_native::engine::UmgahData),
    #[cfg(not(test))]
    NativeUmgah(u16),
    Mmrnmhrm(&'a CHARACTERISTIC_STUFF),
    #[cfg(not(test))]
    Sis(&'a crate::ships::races::sis_native::engine::SisData),
}
#[cfg(not(test))]
impl<'a> CustomData<'a> {
    unsafe fn decode(
        species: u32,
        descriptor: &'a RACE_DESC,
        elements: &super::super::ElementTable,
    ) -> Self {
        use super::super::RaceState;
        match &descriptor.race_state {
            RaceState::Androsynth { collision } => return Self::Androsynth(*collision),
            RaceState::Mmrnmhrm { alternate } => return Self::Mmrnmhrm(alternate),
            RaceState::Pkunk {
                phoenix,
                saved_pre,
                saved_post,
                saved_death,
            } => {
                let address = phoenix.map_or(0, |id| elements.raw(id) as usize);
                return Self::NativePkunk {
                    phoenix: address,
                    saved: [*saved_pre, *saved_post, *saved_death],
                };
            }
            RaceState::Umgah { previous_facing } => {
                return previous_facing.map_or(Self::None, Self::NativeUmgah)
            }
            _ => {}
        }
        let pointer = descriptor.data;
        if pointer.is_null() {
            return Self::None;
        }
        // SAFETY: unconverted initializers retain the corresponding species allocation.
        unsafe {
            match species {
                crate::ships::native::abi::SPECIES_ID_PKUNK_ID => Self::Pkunk(&*pointer.cast()),
                crate::ships::native::abi::SPECIES_ID_UMGAH_ID => Self::Umgah(&*pointer.cast()),
                crate::ships::native::abi::SPECIES_ID_SIS_SHIP_ID => Self::Sis(&*pointer.cast()),
                _ => panic!("unexpected custom data for species {species}"),
            }
        }
    }
}
/// # Safety
/// The queue, ship resources and descriptor allocations remain immutable for the borrow.
pub(crate) unsafe fn ships<'a>(
    queue: &'a CQueue,
    #[cfg(not(test))] elements: &super::super::ElementTable,
) -> Vec<ShipObservation<'a>> {
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
                custom: CustomData::decode(s.SpeciesID, d, elements),
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
            state: ShipState::decode(s),
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
