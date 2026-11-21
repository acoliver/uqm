//! Immutable observations of typed element and ship links for automation.
use uqm_engine::battle::access::observe::ElementObservation;
use uqm_engine::battle::{
    link_queue::{LinkQueue, Starship},
    record::*,
    views::*,
};

pub fn elements(
    elements: &uqm_engine::battle::ElementTable,
    parent_address: impl Fn(uqm_engine::battle::ElementId) -> usize,
) -> Vec<ElementObservation> {
    let mut result = Vec::new();
    let mut next = elements.head();
    while let Some(id) = next {
        next = elements.succ(id);
        result.push(uqm_engine::battle::access::observe::element(
            elements,
            id,
            parent_address(id),
        ));
    }
    result
}

pub struct ShipObservation<'a> {
    pub state: ShipState,
    pub pick: uqm_engine::battle::link_queue::PickShip,
    pub address: usize,
    pub pred: usize,
    pub succ: usize,
    pub element: usize,
    #[cfg(test)]
    lifetime: std::marker::PhantomData<&'a ()>,
    pub icons: uqm_engine::graphics::drawable::FrameId,
    pub strings: Option<uqm_engine::strings::owned_table::EntryRef>,
    pub race: Option<DescriptorObservation<'a>>,
}
pub struct DescriptorObservation<'a> {
    pub address: usize,
    pub info: &'a ShipInfoRecord,
    pub data: &'a DataRecord,
    pub fleet: &'a FleetRecord,
    pub characteristics: &'a CharacteristicRecord,
    pub intel: &'a IntelRecord,
    pub callbacks: [usize; 5],
    pub code_loaded: bool,
    pub custom: CustomData<'a>,
}
pub enum CustomData<'a> {
    None,
    Androsynth(Option<uqm_engine::battle::CollisionHook>),
    NativePkunk {
        phoenix: usize,
        saved: [Option<uqm_engine::battle::ElementHook>; 3],
    },
    NativeUmgah(u16),
    Mmrnmhrm(&'a CharacteristicRecord),
    Sis(&'a uqm_engine::battle::race::SisState),
}
impl<'a> CustomData<'a> {
    fn decode(
        state: &'a uqm_engine::battle::RaceState,
        elements: &uqm_engine::battle::ElementTable,
    ) -> Self {
        use uqm_engine::battle::RaceState;
        match state {
            RaceState::Androsynth { collision } => return Self::Androsynth(*collision),
            RaceState::Mmrnmhrm { alternate } => return Self::Mmrnmhrm(alternate),
            RaceState::Pkunk {
                phoenix,
                saved_pre,
                saved_post,
                saved_death,
            } => {
                let address = phoenix.map_or(0, |id| elements.journal_address(id));
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
pub fn ships<'a>(
    queue: &'a uqm_engine::battle::link_queue::RacePool,
    elements: &uqm_engine::battle::ElementTable,
) -> Vec<ShipObservation<'a>> {
    let ships = LinkQueue::<Starship>::borrow(queue);
    let mut result = Vec::new();
    let mut next = ships.head();
    while let Some(id) = next {
        let s = uqm_engine::battle::access::observe::ship_record(queue, id, elements);
        next = ships.succ(id);
        let (race, icons, strings) = {
            let resources = uqm_engine::battle::access::observe::ship_resources(queue, id);
            let race = resources.descriptor.map(|d| DescriptorObservation {
                address: d.address,
                info: d.info,
                data: d.data,
                fleet: d.fleet,
                characteristics: d.characteristics,
                intel: d.intel,
                callbacks: d.callbacks,
                code_loaded: d.code_loaded,
                custom: CustomData::decode(d.state, elements),
            });
            (race, resources.icons, resources.strings)
        };
        result.push(ShipObservation {
            state: s.state,
            pick: s.pick,
            address: s.address,
            pred: s.pred,
            succ: s.succ,
            element: s.element,
            icons,
            strings,
            race,
            #[cfg(test)]
            lifetime: std::marker::PhantomData,
        });
    }
    result
}
