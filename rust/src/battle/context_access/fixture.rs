//! Typed, value-only command planning views for synchronous automation fixtures.
#[cfg(test)]
#[test]
fn ship_detachment_is_an_explicit_fixture_command() {
    // SAFETY: zero is valid for the native record's scalars and optional pointers.
    let record: ShipRecord = unsafe { std::mem::zeroed() };
    let descriptor = RaceDescriptor::EMPTY;
    let id = ElementId::fixture(0, 1);
    let mut ship = Ship::for_test(&record, &descriptor, id);
    assert!(!ship.clear_element);
    assert_eq!(ship.element, Some(id));
    ship.clear_element();
    assert!(ship.clear_element);
    assert_eq!(ship.element, None);
}

use crate::engine::battle::{record::*, views::*, ElementId, Parent};
#[cfg(not(test))]
use crate::engine::battle::{Cx, ShipId};
#[derive(Clone)]
pub(crate) struct Element {
    pub state: ElementState,
    pub parent: Parent,
    pub target: Option<ElementId>,
    pub pred: Option<ElementId>,
}
impl std::ops::Deref for Element {
    type Target = ElementState;
    fn deref(&self) -> &ElementState {
        &self.state
    }
}
impl std::ops::DerefMut for Element {
    fn deref_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }
}
impl Element {
    pub fn is_player_ship(&self) -> bool {
        self.flags & crate::ships::native::abi::PLAYER_SHIP as u16 != 0
    }
    pub fn has_finite_life(&self) -> bool {
        self.flags & crate::ships::native::abi::FINITE_LIFE as u16 != 0
    }
    pub fn set_appearing(&mut self) {
        self.flags |= crate::ships::native::abi::APPEARING as u16;
    }
    pub fn clear_appearing(&mut self) {
        self.flags &= !(crate::ships::native::abi::APPEARING as u16);
    }
}
#[cfg(not(test))]
pub(crate) type ZoomSample = ((usize, u16), (usize, u16), Option<(usize, usize)>);
#[derive(Clone)]
pub(crate) struct ShipFrames {
    pub ship: Frames,
    pub weapon: Frames,
    pub special: Frames,
    pub captain_special: FrameSelection,
}
#[derive(Clone, Copy)]
pub(crate) struct Phoenix {
    pub handle: Option<ElementId>,
    pub hooks: [usize; 3],
}
pub(crate) struct Ship {
    pub state: ShipState,
    pub species: u32,
    pub index: u16,
    pub element: Option<ElementId>,
    clear_element: bool,
    pub info: Option<Info>,
    pub data: Option<ShipFrames>,
    pub characteristics: Option<CHARACTERISTIC_STUFF>,
    pub phoenix: Option<Phoenix>,
}
impl Ship {
    pub(crate) fn clear_element(&mut self) {
        self.element = None;
        self.clear_element = true;
    }
}
#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn fixture_element(&self, id: ElementId) -> Element {
        Element {
            state: self.element(id),
            parent: self.parent(id),
            target: self.target(id),
            pred: self.elements().pred(id),
        }
    }
    pub(crate) fn fixture_ship(&self, id: ShipId) -> Ship {
        let s = self.ship(id);
        let mut view = Ship {
            state: self.ship_state(id),
            species: s.SpeciesID,
            index: s.index,
            element: s.hShip,
            clear_element: false,
            info: None,
            data: None,
            characteristics: None,
            phoenix: None,
        };
        if s.descriptor.is_none() {
            return view;
        }
        let d = self.desc(id);
        let stats = self.race_stats(id);
        view.info = Some(stats.ship_info);
        view.characteristics = Some(stats.characteristics);
        view.data = Some(ShipFrames {
            ship: self.ship_frame_set(id, 0),
            weapon: self.ship_frame_set(id, 1),
            special: self.ship_frame_set(id, 2),
            captain_special: d.ship_data.captain_control.special.clone(),
        });
        view.phoenix = match d.race_state {
            crate::engine::battle::RaceState::Pkunk {
                phoenix,
                saved_pre,
                saved_post,
                saved_death,
            } => Some(Phoenix {
                handle: phoenix,
                hooks: [saved_pre, saved_post, saved_death].map(|f| f.map_or(0, |f| f.address())),
            }),
            _ => None,
        };
        view
    }
    pub(crate) fn apply_fixture_ship(&mut self, id: ShipId, ship: &Ship) {
        *self.ship_state_mut(id) = ship.state;
        self.set_ship_identity(id, ship.species, ship.index);
        if ship.clear_element {
            self.set_ship_element(id, None);
        }
        if let Some(info) = ship.info {
            self.race_stats_mut(id).ship_info = info;
        }
        if let Some(data) = &ship.data {
            self.set_captain_special(id, data.captain_special.clone());
        }
    }
    pub(crate) fn ship_address(&self, id: ShipId) -> usize {
        self.ship_pointer(id) as usize
    }
    pub(crate) fn detach_descriptor(&mut self, id: ShipId) -> Descriptor {
        Descriptor(self.take_descriptor(id))
    }
    pub(crate) fn fixture_spawn(&mut self, id: ShipId) -> bool {
        // SAFETY: all command planning values are committed before this lifecycle handoff.
        unsafe { crate::battle::ship_native::spawn_ship(self, id) != 0 }
    }
    pub(crate) fn fixture_damage(&mut self, id: ElementId, damage: i16) {
        self.damage(id, damage);
    }
}
#[cfg(not(test))]
pub(crate) struct Descriptor(Option<crate::engine::battle::RaceDesc>);
#[cfg(not(test))]
impl Descriptor {
    pub(crate) fn free(self) {
        if let Some(descriptor) = self.0 {
            // SAFETY: the caller retires the detached descriptor only after its last frame user.
            unsafe {
                crate::ships::loader_native::unload_ship(descriptor, 1, 1);
            }
        }
    }
}
#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn fixture_think(&mut self, ship: ShipId, crew: u16) {
        let input = crate::battle::cyborg_native::tactical(self, ship);
        self.ship_state_mut(ship).input = crate::battle::battle_native::ship_input(input, crew);
    }
    pub(crate) fn fixture_intelligence(
        &mut self,
        element: ElementId,
        objects: [crate::engine::battle::Concern; 4],
    ) {
        let ship = self.owner(element);
        let count = objects.len() as u16;
        let mut scratch = [crate::engine::battle::Concern {
            object: None,
            facing: 0,
            which_turn: 0,
            movement: 0,
        }; 10];
        scratch[..objects.len()].copy_from_slice(&objects);
        let mut concerns = crate::engine::battle::Concerns {
            count,
            objects: scratch,
            subject: self.weapon_origin(element),
        };
        assert!(
            self.race_intelligence(ship, &mut concerns),
            "fixture intelligence callback"
        );
    }
    pub(crate) fn zoom_sample(
        &self,
        frames: Frames,
        element: ElementId,
        view: usize,
        index: u16,
    ) -> ZoomSample {
        assert!(view < 3, "fixture zoom view");
        // SAFETY: the fixture retains the descriptor's three views and all selected drawable images.
        unsafe {
            let frame = &*self.element(element).current.image.frame.raw();
            let expected = &**frames.raw().add(view);
            let mipmap = (view == 1).then(|| {
                let adjacent = &*crate::graphics::pixmap_abi::SetAbsFrameIndex(
                    (*frames.raw().add(view + 1)).cast(),
                    index,
                );
                let selected = &*frame.image;
                let adjacent = &*adjacent.image;
                (selected.MipmapImg as usize, adjacent.NormalImg as usize)
            });
            (
                (frame.parent as usize, frame.Index),
                (expected.parent as usize, index),
                mipmap,
            )
        }
    }
}
#[cfg(test)]
impl Element {
    pub(crate) fn new() -> Self {
        let record = crate::engine::battle::record::ElementRecord::default();
        Self {
            state: crate::engine::battle::access::fixture_element_state(&record),
            parent: Parent::None,
            target: None,
            pred: None,
        }
    }
}
#[cfg(test)]
impl Ship {
    pub(crate) fn for_test(s: &ShipRecord, d: &RaceDescriptor, element: ElementId) -> Self {
        Self {
            // SAFETY: the retained ship pool remains immutable during observation.
            state: unsafe { crate::engine::battle::access::observe_ship_record(s) },
            species: s.SpeciesID,
            index: s.index,
            element: Some(element),
            clear_element: false,
            // SAFETY: the test retains its descriptor throughout this value snapshot.
            info: Some(
                unsafe { crate::engine::battle::access::observe_descriptor_record(d) }.ship_info,
            ),
            data: Some(ShipFrames {
                ship: Frames::shared(
                    crate::engine::battle::frames::SharedFrames::Fixture(0),
                    d.ship_data.ship.clone(),
                ),
                weapon: Frames::shared(
                    crate::engine::battle::frames::SharedFrames::Fixture(1),
                    d.ship_data.weapon.clone(),
                ),
                special: Frames::shared(
                    crate::engine::battle::frames::SharedFrames::Fixture(2),
                    d.ship_data.special.clone(),
                ),
                captain_special: d.ship_data.captain_control.special.clone(),
            }),
            characteristics: Some(d.characteristics),
            phoenix: None,
        }
    }
}
