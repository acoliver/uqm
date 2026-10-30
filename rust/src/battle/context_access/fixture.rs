//! Typed, value-only command planning views for synchronous automation fixtures.
#[cfg(test)]
#[test]
fn ship_detachment_is_an_explicit_fixture_command() {
    // SAFETY: zero is valid for the native record's scalars and optional pointers.
    let record: STARSHIP = unsafe { std::mem::zeroed() };
    let descriptor = RACE_DESC::EMPTY;
    let id = ElementId { slot: 0, gen: 1 };
    let mut ship = Ship::for_test(&record, &descriptor, id);
    assert!(!ship.clear_element);
    assert_eq!(ship.element, Some(id));
    ship.clear_element();
    assert!(ship.clear_element);
    assert_eq!(ship.element, None);
}

#[cfg(not(test))]
use crate::engine::battle::{link_queue, Cx, ShipId};
use crate::engine::battle::{record::*, views::*, ElementId, Parent};
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
pub(crate) struct ShipFrames {
    pub ship: Frames,
    pub weapon: Frames,
    pub special: Frames,
    pub captain_special: Frame,
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
        let handle = self.elements()[id].hTarget;
        let target_ship = if handle.is_null() {
            None
        } else {
            self.queued_ships(0)
                .into_iter()
                .chain(self.queued_ships(1))
                .find(|&ship| self.ship_address(ship) == handle as usize)
        };
        Element {
            state: self.element(id),
            parent: self.parent(id),
            target: if target_ship.is_none() {
                self.target(id)
            } else {
                None
            },
            pred: self.elements().pred(id),
        }
    }
    pub(crate) fn fixture_ship(&self, id: ShipId) -> Ship {
        let s = self.ship(id);
        let mut view = Ship {
            state: self.ship_state(id),
            species: s.SpeciesID,
            index: s.index,
            element: self.elements().allocated_id(s.hShip),
            clear_element: false,
            info: None,
            data: None,
            characteristics: None,
            phoenix: None,
        };
        if s.RaceDescPtr.is_null() {
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
            // SAFETY: the descriptor retains this loaded captain frame.
            captain_special: unsafe { Frame::from_native(d.ship_data.captain_control.special) },
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
        // SAFETY: the checked queue view is exclusive and no callback runs during command planning.
        unsafe {
            let mut queue = link_queue::ships(usize::from(id.side));
            let link = queue.link(id);
            queue.edit(link, |pick| {
                pick.species = ship.species;
                pick.index = ship.index;
            });
        }
        if ship.clear_element {
            // SAFETY: the checked ship is exclusively owned during the command commit.
            unsafe {
                (*self.ship_pointer(id)).hShip = std::ptr::null_mut();
            }
        }
        if let Some(info) = ship.info {
            self.race_stats_mut(id).ship_info = info;
        }
        if let Some(data) = ship.data {
            self.set_captain_special(id, data.captain_special);
        }
    }
    pub(crate) fn ship_address(&self, id: ShipId) -> usize {
        self.ship_pointer(id) as usize
    }
    pub(crate) fn detach_descriptor(&mut self, id: ShipId) -> Descriptor {
        // SAFETY: this lifecycle command exclusively detaches the checked ship descriptor.
        unsafe {
            Descriptor(std::mem::replace(
                &mut (*self.ship_pointer(id)).RaceDescPtr,
                std::ptr::null_mut(),
            ))
        }
    }
    pub(crate) fn descriptor_handle(&self, id: ShipId) -> Descriptor {
        Descriptor(self.ship(id).RaceDescPtr)
    }
    pub(crate) fn fixture_spawn(&mut self, id: ShipId) -> bool {
        let pointer = self.ship_pointer(id);
        // SAFETY: all command planning values have been committed before this lifecycle handoff.
        unsafe {
            crate::app::bridge::publish(self, || crate::battle::ship_native::spawn_ship(pointer))
                != 0
        }
    }
    pub(crate) fn fixture_damage(&mut self, id: ElementId, damage: i16) {
        let pointer = self.elements().raw(id);
        // SAFETY: this checked element remains live across the legacy damage callback.
        unsafe {
            crate::app::bridge::publish(self, || {
                crate::battle::world_elements::do_damage(pointer.cast(), damage)
            });
        }
    }
}
#[derive(Clone, Copy)]
#[cfg(not(test))]
pub(crate) struct Descriptor(*mut RACE_DESC);
#[cfg(not(test))]
impl Descriptor {
    pub(crate) fn free(self) {
        if !self.0.is_null() {
            // SAFETY: the caller retires the detached descriptor only after its last frame user.
            unsafe {
                crate::ships::loader_native::free_ship(self.0, 1, 1);
            }
        }
    }
}
#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn queued_ships(&self, side: u8) -> Vec<ShipId> {
        // SAFETY: only typed ids leave this queue borrow; no callback or record borrow crosses it.
        unsafe {
            let queue = link_queue::ships(usize::from(side));
            let mut result = Vec::new();
            let mut next = queue.head();
            while let Some(id) = next {
                result.push(queue.ship_id(id, side));
                next = queue.succ(id);
            }
            result
        }
    }
    pub(crate) fn fixture_think(&mut self, ship: ShipId, crew: u16) {
        let pointer = self.ship_pointer(ship);
        // SAFETY: command planning ended before invoking the existing tactical routine.
        let input = unsafe {
            crate::app::bridge::publish(self, || {
                crate::battle::cyborg_native::tactical_intelligence(std::ptr::null_mut(), pointer)
            })
        };
        self.ship_state_mut(ship).input = crate::battle::battle_native::ship_input(input, crew);
    }
    pub(crate) fn fixture_intelligence(
        &mut self,
        element: ElementId,
        objects: [crate::engine::battle::Concern; 4],
    ) {
        let ship = self.owner(element);
        let mut concerns = crate::engine::battle::Concerns {
            count: objects.len() as u16,
            objects,
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
        // SAFETY: this test source has the same zero-valid representation as a newly allocated element.
        let record: crate::engine::battle::record::ELEMENT = unsafe { std::mem::zeroed() };
        Self {
            // SAFETY: this test retains its initialized local record.
            state: unsafe { crate::engine::battle::access::observe_element_record(&record) },
            parent: Parent::None,
            target: None,
            pred: None,
        }
    }
}
#[cfg(test)]
impl Ship {
    pub(crate) fn for_test(s: &STARSHIP, d: &RACE_DESC, element: ElementId) -> Self {
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
                // SAFETY: the test descriptor retains all three ship frames.
                ship: unsafe {
                    Frames::from_native(std::ptr::NonNull::from(&d.ship_data.ship).cast())
                },
                // SAFETY: the test descriptor retains all three weapon frames.
                weapon: unsafe {
                    Frames::from_native(std::ptr::NonNull::from(&d.ship_data.weapon).cast())
                },
                // SAFETY: the test descriptor retains all three special frames.
                special: unsafe {
                    Frames::from_native(std::ptr::NonNull::from(&d.ship_data.special).cast())
                },
                // SAFETY: the descriptor retains this loaded captain frame.
                captain_special: unsafe { Frame::from_native(d.ship_data.captain_control.special) },
            }),
            characteristics: Some(d.characteristics),
            phoenix: None,
        }
    }
}
