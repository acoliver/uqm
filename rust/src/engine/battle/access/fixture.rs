//! Typed, value-only command planning views for synchronous automation fixtures.
#[cfg(not(test))]
use super::super::{link_queue, Cx, ShipId};
use super::super::{link_queue::PickShip, record::*, views::*, ElementId, Parent};
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
    pub pick: PickShip,
    pub element: Option<ElementId>,
    pub info: Option<Info>,
    pub data: Option<ShipFrames>,
    pub characteristics: Option<CHARACTERISTIC_STUFF>,
    pub intelligence: Option<usize>,
    pub phoenix: Option<Phoenix>,
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
            pick: self.pick_ship(id),
            element: self.elements().id(s.hShip),
            info: None,
            data: None,
            characteristics: None,
            intelligence: None,
            phoenix: None,
        };
        if s.RaceDescPtr.is_null() {
            return view;
        }
        let d = self.desc(id);
        let stats = self.race_stats(id);
        view.info = Some(stats.ship_info);
        view.characteristics = Some(stats.characteristics);
        let callbacks = d.callbacks();
        view.intelligence = (callbacks[4] != 0).then_some(callbacks[4]);
        view.data = Some(ShipFrames {
            ship: self.ship_frame_set(id, 0),
            weapon: self.ship_frame_set(id, 1),
            special: self.ship_frame_set(id, 2),
            captain_special: Frame(d.ship_data.captain_control.special),
        });
        view.phoenix = match d.race_state {
            super::super::RaceState::Pkunk {
                phoenix,
                saved_pre,
                saved_post,
                saved_death,
            } => Some(Phoenix {
                handle: phoenix,
                hooks: [saved_pre, saved_post, saved_death].map(|f| f.map_or(0, |f| f.address())),
            }),
            _ if s.SpeciesID == crate::ships::native::abi::SPECIES_ID_PKUNK_ID => {
                std::ptr::NonNull::new(
                    d.data
                        .cast::<crate::ships::races::pkunk_native::engine::PkunkData>(),
                )
                .map(|pointer| {
                    let data = self.project(pointer);
                    Phoenix {
                        handle: self.elements().id(data.hPhoenix),
                        hooks: [
                            data.saved_preprocess_func,
                            data.saved_postprocess_func,
                            data.saved_death_func,
                        ]
                        .map(|f| f.map_or(0, |f| f.address())),
                    }
                })
            }
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
            queue.set(link, ship.pick);
        }
        let element = ship
            .element
            .map_or(std::ptr::null_mut(), |id| self.elements().raw(id).cast());
        self.ship_mut(id).hShip = element;
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
        Descriptor(std::mem::replace(
            &mut self.ship_mut(id).RaceDescPtr,
            std::ptr::null_mut(),
        ))
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
        objects: [super::super::Concern; 4],
    ) {
        let ship = self.owner(element);
        let mut concerns = super::super::Concerns {
            objects,
            subject: self.weapon_origin(element),
            count: 10,
        };
        if self.desc(ship).race.is_some() {
            self.race_intelligence(ship, &mut concerns);
            return;
        }
        let callback = self
            .desc(ship)
            .cyborg_control
            .intelligence_func
            .unwrap_or_else(|| panic!("fixture intelligence callback"));
        let pointer = self.elements().raw(element);
        let mut raw = [EVALUATE_DESC {
            ObjectPtr: std::ptr::null_mut(),
            facing: 0,
            which_turn: 0,
            MoveState: 0,
        }; 10];
        for (dest, source) in raw.iter_mut().zip(objects) {
            dest.ObjectPtr = source
                .object
                .map_or(std::ptr::null_mut(), |id| self.elements().raw(id).cast());
            dest.facing = source.facing;
            dest.which_turn = source.which_turn;
            dest.MoveState = source.movement;
        }
        let parent = self.ship_address(ship);
        // SAFETY: ten real operands reproduce the legacy fixture callback's count and scratch space.
        unsafe {
            crate::battle::process_native::dispatch::invoke_legacy(
                self,
                callback as usize,
                parent,
                || callback(pointer.cast(), raw.as_mut_ptr(), 10),
            );
        }
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
            let frame = &*self.element(element).current.image.frame.0;
            let expected = &**frames.0.add(view);
            let mipmap = (view == 1).then(|| {
                let adjacent = &*crate::graphics::pixmap_abi::SetAbsFrameIndex(
                    (*frames.0.add(view + 1)).cast(),
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
        let record: super::super::record::ELEMENT = unsafe { std::mem::zeroed() };
        Self {
            state: ElementState::decode(&record),
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
            state: ShipState::decode(s),
            pick: PickShip {
                species: s.SpeciesID,
                captain: s.captains_name_index,
                crew: s.crew_level,
                max_crew: s.max_crew,
                cost: s.ship_cost,
                index: s.index,
                strings: s.race_strings as usize,
                icons: s.icons as usize,
                energy_counter: s.energy_counter,
                player: s.playerNr,
                control: s.control,
            },
            element: Some(element),
            info: Some(RaceStats::decode(d).ship_info),
            data: Some(ShipFrames {
                ship: Frames(std::ptr::from_ref(&d.ship_data.ship).cast_mut().cast()),
                weapon: Frames(std::ptr::from_ref(&d.ship_data.weapon).cast_mut().cast()),
                special: Frames(std::ptr::from_ref(&d.ship_data.special).cast_mut().cast()),
                captain_special: Frame(d.ship_data.captain_control.special),
            }),
            characteristics: Some(d.characteristics),
            intelligence: None,
            phoenix: None,
        }
    }
}
