//! Stage B resource and record projections. Borrows end before legacy handoffs.
#[cfg(not(test))]
impl Cx<'_> {
    pub fn alloc_element(&mut self) -> Option<ElementId> {
        let allocated = self.b.kernel.elements.alloc();
        let address = if let Some(id) = allocated {
            self.assign_primitive(id);
            self.primitive_style_mut(id).kind = crate::graphics::native_abi::EMPTY_PRIMITIVE;
            self.b.kernel.elements.raw(id) as usize
        } else {
            0
        };
        crate::collections::observation::record(crate::collections::observation::Event::Allocate(
            address,
        ));
        allocated
    }

    pub fn dispose(&mut self, id: ElementId) {
        assert!(
            !self.b.kernel.elements.linked(id),
            "dispose requires an unlinked element"
        );
        let address = self.b.kernel.elements.raw(id) as usize;
        let index = self.b.kernel.elements[id].PrimIndex;
        self.release_primitive(index);
        crate::collections::observation::record(crate::collections::observation::Event::Free(
            address,
        ));
        self.b.kernel.elements.free(id);
    }

    pub(crate) fn finish_battle_storage(&mut self) {
        self.b.kernel.finish_battle();
    }

    pub(crate) fn complete_ship_reset(&mut self) {
        self.b.kernel.complete_ship_reset();
    }

    /// Move the boss result before descriptor disposal; no live graph is retained.
    pub(crate) fn retain_terminal_outcome(&mut self, owner: ShipId) {
        if matches!(self.desc(owner).race_state, super::RaceState::SaMatra(_)) {
            assert!(self.b.terminal_outcome.is_none(), "duplicate boss outcome");
            self.b.terminal_outcome = Some(std::mem::take(&mut self.desc_mut(owner).race_state));
        }
    }

    pub fn terminal_outcome(&self) -> Option<&super::RaceState> {
        self.b.terminal_outcome.as_ref()
    }
}

#[cfg(not(test))]
impl super::World {
    pub fn set_activity(&mut self, activity: u16) {
        // SAFETY: World construction retains this exclusive game-thread projection.
        unsafe {
            self.activity.as_ptr().write(activity);
        }
    }
}

#[cfg(not(test))]
use super::record::PRIMITIVE;
#[cfg(not(test))]
use super::Parent;
use super::{Cx, ElementId, ShipId};

#[cfg(not(test))]
impl Cx<'_> {
    pub fn owner(&self, id: ElementId) -> ShipId {
        match self.parent(id) {
            Parent::Ship(ship) => ship,
            other => panic!("element {id:?} has no ship owner: {other:?}"),
        }
    }

    pub fn parent(&self, id: ElementId) -> Parent {
        self.b.kernel.elements[id].parent
    }

    pub fn ship_element(&self, ship: ShipId) -> ElementId {
        self.optional_ship_element(ship)
            .unwrap_or_else(|| panic!("{}", "ship has no element"))
    }

    pub fn optional_ship_element(&self, ship: ShipId) -> Option<ElementId> {
        let id = self.ship(ship).hShip?;
        assert!(self.elements().contains(id), "stale ship element {id:?}");
        Some(id)
    }

    pub fn set_parent(&mut self, element: ElementId, parent: Option<ShipId>) {
        self.b.kernel.elements.record_mut(element).parent =
            parent.map_or(Parent::None, Parent::Ship);
    }

    pub fn target(&self, element: ElementId) -> Option<ElementId> {
        self.b.kernel.elements[element].target
    }

    pub fn set_target(&mut self, element: ElementId, target: Option<ElementId>) {
        if let Some(id) = target {
            assert!(self.b.kernel.elements.contains(id), "stale target {id:?}");
        }
        self.b.kernel.elements.record_mut(element).target = target;
    }

    pub(crate) fn set_ip_group_parent(&mut self, id: ElementId, slot: u8) {
        assert!(
            slot < self.groups().num_objects,
            "IP-group slot outside pool"
        );
        self.b.kernel.elements.record_mut(id).parent = Parent::IpGroup(slot);
    }

    pub fn put(&mut self, id: ElementId) {
        self.insert_before(id, None);
    }
}

use super::views::*;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn ship_frame_set(&self, id: ShipId, role: usize) -> Frames {
        super::frames::Frames::ship(id, role.into(), self.ship_array(id, role).clone())
    }
    pub fn ship_frame(&self, id: ShipId, role: usize) -> FrameRef<'_> {
        self.ship_array(id, role)
            .view(0)
            .owned_by(super::frames::FrameOwner::Ship(id, role.into()))
    }
    pub fn advance_frame(&self, frame: FrameRef<'_>) -> FrameSelection {
        frame.to_owned().with_raw(self.inc_frame(frame.raw()))
    }
    pub fn equivalent_frame(&self, frame: FrameRef<'_>, source: FrameRef<'_>) -> FrameSelection {
        frame
            .to_owned()
            .with_raw(self.equ_frame(frame.raw(), source.raw()))
    }
    pub fn absolute_frame(&self, frame: FrameRef<'_>, index: u16) -> FrameSelection {
        frame
            .to_owned()
            .with_raw(self.abs_frame(frame.raw(), index))
    }
    pub fn frame_number(&self, frame: FrameRef<'_>) -> u16 {
        self.frame_index(frame.raw())
    }
    pub fn silhouette_frame(&mut self, id: ElementId, frame: FrameSelection, flags: u8) {
        self.silhouette(id, frame, flags);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn weapon_origin(&self, id: ElementId) -> super::Subject {
        Subject::decode(&self.b.kernel.elements[id])
    }

    pub fn set_frame(&mut self, id: ElementId, next: bool, frame: FrameSelection) {
        use crate::collections::observation::image;
        if next {
            image!(frame;self.b.kernel.elements.record_mut(id).next.image.frame,frame);
        } else {
            image!(frame;self.b.kernel.elements.record_mut(id).current.image.frame,frame);
        }
    }
    pub fn set_single_frames(&mut self, id: ElementId, frames: Frames, frame: FrameSelection) {
        crate::collections::observation::image!(single; self.b.kernel.elements.record_mut(id).current.image.frames, frames, frame.raw());
    }

    pub fn set_frames(&mut self, id: ElementId, next: bool, frames: Frames) {
        use crate::collections::observation::image;
        if next {
            image!(set;self.b.kernel.elements.record_mut(id).next.image.frames,frames);
        } else {
            image!(set;self.b.kernel.elements.record_mut(id).current.image.frames,frames);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn copy_visual_to_next(&mut self, id: ElementId, visual: Visual) {
        self.b.kernel.elements.record_mut(id).next.location = visual.location;
        self.set_frame(id, true, visual.image.frame);
        self.set_frames(id, true, visual.image.frames);
    }
    pub fn element_frame_set_first(&self, id: ElementId, next: bool) -> FrameRef<'_> {
        let record = &self.b.kernel.elements[id];
        let image = if next {
            &record.next.image
        } else {
            &record.current.image
        };
        image.frames.view(0)
    }
    pub fn set_intersect_frame(&mut self, id: ElementId, frame: FrameSelection) {
        use crate::collections::observation::image;
        image!(frame;self.b.kernel.elements.record_mut(id).IntersectControl.IntersectStamp.frame,frame.raw());
        self.b.kernel.elements.record_mut(id).intersect_frame = frame;
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn callbacks(&self, ship: ShipId) -> super::Callbacks {
        self.desc(ship).race_callbacks
    }
    pub fn set_preprocess(
        &mut self,
        ship: ShipId,
        selection: super::Selection<super::race::ShipFn>,
    ) {
        self.desc_mut(ship).race_callbacks.pre = selection;
    }
    pub fn set_postprocess(
        &mut self,
        ship: ShipId,
        selection: super::Selection<super::race::ShipFn>,
    ) {
        self.desc_mut(ship).race_callbacks.post = selection;
    }
    pub fn set_weapon(&mut self, ship: ShipId, selection: super::Selection<super::race::WeaponFn>) {
        self.desc_mut(ship).race_callbacks.weapon = selection;
    }
}

impl Cx<'_> {
    fn project<T>(&self, pointer: std::ptr::NonNull<T>) -> &T {
        // SAFETY: record decoders supply checked pointers retained by this context.
        unsafe { &*pointer.as_ptr() }
    }
}

impl Cx<'_> {
    pub fn element(&self, id: ElementId) -> ElementState {
        ElementState::decode(&self.b.kernel.elements[id])
    }
    pub fn ship_state(&self, id: ShipId) -> ShipState {
        ShipState::decode(self.ship(id))
    }
    pub fn race_stats(&self, id: ShipId) -> RaceStats {
        RaceStats::decode(self.desc(id))
    }
}

pub(crate) fn invoke<R>(callback: usize, parent: usize, call: impl FnOnce() -> R) -> R {
    crate::collections::observation::observe(callback, parent);
    call()
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn clear_captain_background(&mut self, ship: ShipId) {
        self.desc_mut(ship).ship_data.captain_control.background = std::ptr::null_mut();
    }
}

#[cfg(not(test))]
use crate::graphics::native_abi::FRAME;
#[cfg(not(test))]
use crate::graphics::pixmap_abi::{IncFrameIndex, SetAbsFrameIndex, SetEquFrameIndex};
#[cfg(not(test))]
impl Cx<'_> {
    pub(super) fn ship_array(&self, ship: ShipId, role: usize) -> &super::frames::FrameArray {
        let data = &self.desc(ship).ship_data;
        match role {
            0 => &data.ship,
            1 => &data.weapon,
            2 => &data.special,
            _ => panic!("invalid frame role {role}"),
        }
    }
    pub(super) fn abs_frame(&self, frame: FRAME, index: u16) -> FRAME {
        // SAFETY: resource handles come from this battle's loaded descriptors.
        unsafe { SetAbsFrameIndex(frame, index) }
    }
    pub(super) fn equ_frame(&self, frame: FRAME, source: FRAME) -> FRAME {
        // SAFETY: both resource handles remain loaded for this battle.
        unsafe { SetEquFrameIndex(frame, source) }
    }
    pub(super) fn inc_frame(&self, frame: FRAME) -> FRAME {
        // SAFETY: the current element's resource remains loaded.
        unsafe { IncFrameIndex(frame) }
    }
    pub(super) fn frame_index(&self, frame: FRAME) -> u16 {
        // SAFETY: this battle retains the element's loaded frame resources.
        unsafe { crate::graphics::pixmap_abi::GetFrameIndex(frame.cast()) }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn shared_frame_selection(&self, role: SharedFrames) -> FrameSelection {
        self.shared_frame_set(role).view(0).to_owned()
    }
    pub fn copy_image_to_current(&mut self, id: ElementId, image: Image) {
        self.set_frame(id, false, image.frame);
        self.set_frames(id, false, image.frames);
    }
}

impl Cx<'_> {
    pub fn shared_frame_set(&self, role: SharedFrames) -> Frames {
        let source = (self.world.frame_source)(role);
        // SAFETY: World retains these source slots; the result leases their loaded allocations.
        unsafe { Frames::native_shared(role, source, 3) }
    }
    pub fn init_intersect(&mut self, id: ElementId) {
        self.init_intersect_start(id);
        self.init_intersect_end(id);
    }

    pub fn init_intersect_start(&mut self, id: ElementId) {
        let e = self.b.kernel.elements.record_mut(id);
        e.IntersectControl.IntersectStamp.origin.x = e.current.location.x >> 2;
        e.IntersectControl.IntersectStamp.origin.y = e.current.location.y >> 2;
    }

    pub fn init_intersect_end(&mut self, id: ElementId) {
        let e = self.b.kernel.elements.record_mut(id);
        e.IntersectControl.EndPoint.x = e.next.location.x >> 2;
        e.IntersectControl.EndPoint.y = e.next.location.y >> 2;
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn primitive_style_mut(
        &mut self,
        id: ElementId,
    ) -> impl std::ops::DerefMut<Target = PrimitiveStyle> + '_ {
        fn apply(p: &mut PRIMITIVE, v: PrimitiveStyle) {
            p.Type = v.kind;
            p.color = v.color;
        }
        let record = self.primitive(id);
        let value = PrimitiveStyle {
            kind: record.Type,
            color: record.color,
        };
        Edit {
            value,
            record,
            commit: apply,
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn frame_count(&self, frame: FrameRef<'_>) -> u16 {
        // SAFETY: opaque frames originate in retained battle resources, or are absent.
        unsafe { crate::graphics::pixmap_abi::GetFrameCount(frame.raw().cast()) }
    }
    pub fn retreat_frame(&self, frame: FrameRef<'_>) -> FrameSelection {
        let raw = frame.raw().cast();
        // SAFETY: the selection retains the frame's drawable.
        let previous = unsafe { crate::graphics::pixmap_abi::DecFrameIndex(raw).cast() };
        frame.to_owned().with_raw(previous)
    }
    pub fn relative_frame(&self, frame: FrameRef<'_>, offset: i16) -> FrameSelection {
        let raw = frame.raw().cast();
        // SAFETY: the selection retains the frame's drawable.
        let relative = unsafe { crate::graphics::pixmap_abi::SetRelFrameIndex(raw, offset).cast() };
        frame.to_owned().with_raw(relative)
    }
    pub fn frame_rect(&self, frame: FrameRef<'_>) -> crate::graphics::native_abi::RECT {
        let mut rect = crate::graphics::native_abi::RECT {
            corner: crate::graphics::native_abi::POINT { x: 0, y: 0 },
            extent: crate::graphics::native_abi::EXTENT {
                width: 0,
                height: 0,
            },
        };
        // SAFETY: the resource owner retains the frame and rect is local output.
        unsafe {
            crate::graphics::drawable_abi::GetFrameRect(frame.raw().cast(), &mut rect);
        }
        rect
    }
    pub fn set_ship_frame(
        &mut self,
        ship: ShipId,
        role: usize,
        resolution: usize,
        frame: FrameSelection,
    ) {
        assert!(resolution < 3, "invalid frame resolution {resolution}");
        let array = self.ship_array(ship, role);
        array.replace_view(resolution, frame.view());
    }
    pub fn captain_special(&self, ship: ShipId) -> FrameRef<'_> {
        self.desc(ship).ship_data.captain_control.special.view()
    }
    pub fn set_captain_special(&mut self, ship: ShipId, frame: FrameSelection) {
        self.desc_mut(ship).ship_data.captain_control.special = frame;
    }
    pub fn primitive_style(&self, id: ElementId) -> PrimitiveStyle {
        let index = usize::from(self.b.kernel.elements[id].PrimIndex);
        let record = self.b.kernel.display.get(index as u16);
        PrimitiveStyle {
            kind: record.Type,
            color: record.color,
        }
    }
    /// Copy into an allocated, unlinked slot, retaining its display allocation.
    pub fn copy_allocated_element(&mut self, destination: ElementId, source: ElementId) {
        assert_ne!(destination, source, "copy requires distinct slots");
        assert!(
            !self.elements().linked(destination),
            "copy destination is linked"
        );
        let index = self.b.kernel.elements[destination].PrimIndex;
        let source_index = usize::from(self.b.kernel.elements[source].PrimIndex);
        let primitive = *self.b.kernel.display.get(source_index as u16);
        let value = self.b.kernel.elements[source].clone();
        let address = std::ptr::from_ref(&self.b.kernel.elements[source]) as usize;
        crate::collections::observation::copy_image(
            self.b.kernel.elements.record_mut(destination),
            value,
            address,
        );
        self.b.kernel.elements.record_mut(destination).PrimIndex = index;
        *self.primitive(destination) = primitive;
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn set_ship_sound_bank(&mut self, ship: ShipId, index: u16) {
        let sounds = self.desc(ship).ship_data.ship_sounds;
        assert!(
            index < self.ship_sound_count(ship),
            "invalid sound bank index {index}"
        );
        // SAFETY: the checked index names an entry in this ship's loaded sound bank.
        let sound =
            unsafe { crate::strings::string_table::SetAbsStringTableIndex(sounds.cast(), index) };
        self.desc_mut(ship).ship_data.ship_sounds = sound.cast();
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn set_ship_identity(&mut self, id: ShipId, species: u32, index: u16) {
        let record = self.ship_mut(id);
        record.SpeciesID = species;
        record.index = index;
    }

    pub(crate) fn queued_ships(&self, side: u8) -> Vec<ShipId> {
        let queue = super::link_queue::LinkQueue::borrow(&self.b.kernel.races[usize::from(side)]);
        let mut result = Vec::new();
        let mut next = queue.head();
        while let Some(id) = next {
            result.push(queue.ship_id(id));
            next = queue.succ(id);
        }
        result
    }

    pub(crate) fn first_ship(&self, side: usize) -> Option<ShipId> {
        let ships = super::link_queue::LinkQueue::borrow(&self.b.kernel.races[side]);
        ships.head().map(|id| ships.ship_id(id))
    }

    pub(crate) fn pick_ship(
        &self,
        id: crate::engine::battle::ShipId,
    ) -> crate::engine::battle::link_queue::PickShip {
        let ships =
            super::link_queue::LinkQueue::borrow(&self.b.kernel.races[usize::from(id.side())]);
        ships.get(ships.link(id))
    }
    pub(crate) fn subject_style(
        &self,
        subject: &Subject,
    ) -> crate::engine::battle::views::PrimitiveStyle {
        let primitive = self.b.kernel.display.get(subject.primitive);
        crate::engine::battle::views::PrimitiveStyle {
            kind: primitive.Type,
            color: primitive.color,
        }
    }
    pub(crate) fn subject_intersect_frame<'a>(&self, subject: &'a Subject) -> FrameRef<'a> {
        subject.intersect_frame.view()
    }
}

#[path = "link_queue.rs"]
pub mod link_queue;
#[path = "record.rs"]
pub mod record;
#[path = "table.rs"]
pub mod table;

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn sound_point(&self, id: ElementId) -> crate::graphics::native_abi::POINT {
        let index = usize::from(self.elements()[id].PrimIndex);
        let primitive = self.b.kernel.display.get(index as u16);
        // SAFETY: the checked element owns the primitive; its type selects the union variant.
        unsafe {
            if primitive.Type == crate::graphics::native_abi::LINE_PRIMITIVE {
                primitive.Object.Line.first
            } else {
                primitive.Object.Point
            }
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn primitive_stamp(&mut self, id: ElementId) -> crate::graphics::native_abi::STAMP {
        // SAFETY: the caller selected a stamp primitive from the retained graphics union.
        unsafe { self.primitive(id).Object.Stamp }
    }

    pub fn set_primitive_stamp(
        &mut self,
        id: ElementId,
        stamp: crate::graphics::native_abi::STAMP,
    ) {
        self.primitive(id).Object.Stamp = stamp;
    }

    pub fn copy_current_to_next(&mut self, id: ElementId) {
        let record = self.b.kernel.elements.record_mut(id);
        crate::collections::observation::image!(copy; record.next, record.current);
    }

    pub fn copy_next_to_current(&mut self, id: ElementId) {
        let record = self.b.kernel.elements.record_mut(id);
        crate::collections::observation::image!(copy; record.current, record.next);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn has_descriptor(&self, ship: ShipId) -> bool {
        !self.ship(ship).descriptor.is_none()
    }
}

fn observe_element_record(record: &super::record::ElementRecord) -> ElementState {
    ElementState::decode(record)
}

/// # Safety
/// The caller retains the ship pool throughout this staging observation.
#[cfg(test)]
pub(crate) unsafe fn observe_ship_record(record: &super::record::ShipRecord) -> ShipState {
    ShipState::decode(record)
}

#[cfg(test)]
/// # Safety
/// The caller retains the test descriptor and all resources referenced by its state.
pub(crate) unsafe fn observe_descriptor_record(
    record: &super::record::RaceDescriptor,
) -> RaceStats {
    RaceStats::decode(record)
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn frames_first<'a>(&self, frames: &'a Frames) -> FrameRef<'a> {
        // SAFETY: the supplied frame set belongs to this context's loaded resources.
        frames.view(0)
    }

    pub(crate) fn copy_next_frame_set(&mut self, destination: ElementId, source: ElementId) {
        crate::collections::observation::image!(copy; self.b.kernel.elements.record_mut(destination).current.image.frames, self.b.kernel.elements[source].next.image.frames);
    }
}

impl super::Battle {
    /// # Safety
    /// The game thread retains the element authority throughout this battle lifecycle.
    pub(crate) unsafe fn for_lifecycle() -> Self {
        // SAFETY: lifecycle ownership excludes any other active battle projection.
        Self::new(150)
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn intersect_elements(&mut self, a: ElementId, b: ElementId, time: u16) -> u16 {
        assert_ne!(a, b, "collision operands alias");
        let (left, right) = self.b.kernel.elements.pair_mut(a, b);
        // SAFETY: checked distinct slots and loaded frames remain live during the synchronous geometry call.
        unsafe {
            crate::graphics::geometry_ffi::DrawablesIntersect(
                &mut left.IntersectControl,
                &mut right.IntersectControl,
                time,
            )
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn attach_descriptor(&mut self, ship: ShipId, descriptor: super::RaceDesc) {
        self.ship_mut(ship).descriptor = Some(descriptor.into_storage());
    }
    pub(crate) fn take_descriptor(&mut self, ship: ShipId) -> Option<super::RaceDesc> {
        self.ship_mut(ship)
            .descriptor
            .take()
            .map(super::RaceDesc::from_storage)
    }

    pub(crate) fn next_ship(&self, ship: ShipId) -> Option<ShipId> {
        let queue =
            super::link_queue::LinkQueue::borrow(&self.b.kernel.races[usize::from(ship.side())]);
        queue.succ(queue.link(ship)).map(|id| queue.ship_id(id))
    }
    pub(crate) fn reset_ships(&mut self, side: usize) {
        self.b.kernel.races[side].reinit();
    }
    pub(crate) fn set_ship_element(&mut self, ship: ShipId, element: Option<ElementId>) {
        self.ship_mut(ship).hShip = element;
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn captain_pointer(&mut self, ship: ShipId) -> *mut super::record::CAPTAIN_STUFF {
        &raw mut self.desc_mut(ship).ship_data.captain_control
    }
    pub(crate) fn ship_info_pointer(&mut self, ship: ShipId) -> *mut super::record::SHIP_INFO {
        &raw mut self.desc_mut(ship).ship_info
    }
}
impl Cx<'_> {
    pub(crate) fn refresh_intersect_frame(&mut self, id: ElementId) {
        let e = self.b.kernel.elements.record_mut(id);
        let first = e.next.image.frames.view(0).to_owned();
        // SAFETY: both selections retain their native allocations throughout this projection.
        let raw = unsafe {
            crate::graphics::pixmap_abi::SetEquFrameIndex(first.raw(), e.next.image.frame.raw())
        };
        e.intersect_frame = first.with_raw(raw);
        crate::collections::observation::image!(frame; e.IntersectControl.IntersectStamp.frame, raw);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn status_offset(&mut self, ship: ShipId) -> i16 {
        self.b.kernel.status_offsets[self.ship_state(ship).player as usize]
    }
    pub fn winner(&mut self) -> Option<ShipId> {
        self.b.kernel.winner
    }
    pub(crate) fn set_winner_slot(&mut self, ship: Option<ShipId>) {
        self.b.kernel.winner = ship;
    }
    pub(crate) fn set_ship_species(&mut self, ship: ShipId, species: u32) {
        self.ship_mut(ship).SpeciesID = species;
    }
}

#[cfg(test)]
mod intersection_projection_tests {
    #[test]
    #[serial_test::serial]
    fn live_helpers_preserve_record_coordinates_and_pool_addresses() {
        use crate::engine::battle::{Battle, Cx, World};
        use crate::ships::native::abi::*;
        // SAFETY: a zeroed group queue has no allocated storage.
        let mut groups: super::super::record::QUEUE = unsafe { std::mem::zeroed() };
        let mut activity = 0;
        // SAFETY: the local queue authorities and their pool remain exclusive for this test.
        unsafe {
            let mut battle = Battle::new(1);
            let id = battle.kernel.elements.alloc().unwrap();
            battle.kernel.elements.record_mut(id).current.location = POINT { x: 44, y: 52 };
            battle.kernel.elements.record_mut(id).next.location = POINT { x: 60, y: 68 };
            battle.kernel.elements.record_mut(id).next.image.frames =
                crate::engine::battle::views::Frames::shared(
                    crate::engine::battle::frames::SharedFrames::Fixture(0),
                    crate::engine::battle::frames::FrameArray::fixture(
                        [std::ptr::null_mut(); 3],
                        (),
                    ),
                );
            let mut world = World::new(
                std::ptr::NonNull::from(&mut activity),
                std::ptr::NonNull::from(&mut groups),
                |_| panic!("unused shared frame source"),
            );
            let mut cx = Cx {
                b: &mut battle,
                world: &mut world,
                rng: || 0,
            };
            cx.init_intersect_start(id);
            cx.init_intersect_end(id);
            assert_eq!(
                cx.elements()[id].IntersectControl.IntersectStamp.origin.x,
                11
            );
            assert_eq!(cx.elements()[id].IntersectControl.EndPoint.y, 17);
            cx.element_mut(id).flags = NONSOLID as u16;
            cx.refresh_intersect_frame(id);
            assert!(cx.elements()[id]
                .IntersectControl
                .IntersectStamp
                .frame
                .is_null());
            assert_eq!(cx.elements().id(cx.elements().raw(id)), Some(id));
            assert_eq!(cx.elements().id(std::ptr::null_mut()), None);
            let primitive = cx.display_pool().alloc().unwrap();
            cx.b.kernel.display.get_mut(primitive).Type = 3;
            assert_eq!(cx.display_primitive(primitive).Type, 3);
            cx.append_display(primitive);
            assert_eq!(cx.display_batch()[0].Type, 3);
            cx.clear_display_order();
            assert!(cx.display_batch().is_empty());
            cx.reset_display();
            assert_eq!(cx.b.kernel.display.alloc(), Some(0));
            assert_eq!(cx.display_primitive(0).Type, 3);
            drop(battle);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn assign_primitive(&mut self, id: ElementId) {
        self.b.kernel.elements.record_mut(id).PrimIndex = self
            .b
            .kernel
            .display
            .alloc()
            .unwrap_or_else(|| panic!("AllocElement: Out of display prims!"));
    }
    pub(crate) fn release_primitive(&mut self, index: u16) {
        self.b.kernel.display.free(index);
    }
    pub(crate) fn link_storage(&mut self, id: ElementId, before: Option<ElementId>) {
        self.b.kernel.elements.insert_before(id, before);
    }
    pub(crate) fn unlink_storage(&mut self, id: ElementId) {
        self.b.kernel.elements.remove(id);
    }
}
#[cfg(not(test))]
pub(crate) fn race_desc_layout() -> RaceDescLayout {
    use super::record::*;
    use std::mem;
    RaceDescLayout {
        race_desc_size: mem::size_of::<RaceDescriptor>(),
        ship_data_offset: mem::offset_of!(RaceDescriptor, ship_data),
        ship_info_offset: mem::offset_of!(RaceDescriptor, ship_info),
        characteristics_offset: mem::offset_of!(RaceDescriptor, characteristics),
        ship_data_ship_offset: mem::offset_of!(DATA_STUFF, ship),
        ship_info_crew_offset: mem::offset_of!(SHIP_INFO, crew_level),
        ship_info_max_crew_offset: mem::offset_of!(SHIP_INFO, max_crew),
        characteristics_mass_offset: mem::offset_of!(CHARACTERISTIC_STUFF, ship_mass),
    }
}

pub(crate) mod observe;

/// Layout descriptor for RACE_DESC C-side field offsets.
/// Matches C `RACE_DESC_LAYOUT` in `rust_bridge_ships.h`.
#[repr(C)]
pub struct RaceDescLayout {
    pub race_desc_size: usize,
    pub ship_data_offset: usize,
    pub ship_info_offset: usize,
    pub characteristics_offset: usize,
    /// Offset of `DATA_STUFF.ship` within DATA_STUFF (not within RACE_DESC).
    pub ship_data_ship_offset: usize,
    /// Offset of `SHIP_INFO.crew_level` within SHIP_INFO (not within RACE_DESC).
    pub ship_info_crew_offset: usize,
    /// Offset of `SHIP_INFO.max_crew` within SHIP_INFO (not within RACE_DESC).
    pub ship_info_max_crew_offset: usize,
    /// Offset of `CHARACTERISTIC_STUFF.ship_mass` within CHARACTERISTIC_STUFF.
    pub characteristics_mass_offset: usize,
}

pub mod schema;

pub(crate) mod lifecycle;

/// The running battle or flight's shared queues, borrowed on the game thread.
#[cfg(not(test))]
pub(crate) mod live_queues {

    /// `&race_q[player]`, the `STARSHIP` queue of one side (`RPG_PLAYER_NUM` is
    /// the human player's).
    ///
    /// # Safety
    /// Game thread only. The caller may read the queue only while no other code is
    /// changing it.
    pub unsafe fn player_ships(player: usize) -> *const super::super::link_queue::RacePool {
        super::super::record::ship_queue(player).as_ptr()
    }

    /// The Kernel-owned element pool, retained for this game-thread call.
    ///
    /// The caller may read through this pointer only on the game thread while no
    /// other code is changing the queue.
    pub unsafe fn display_list() -> *const super::super::ElementTable {
        // SAFETY: the caller retains the published, game-thread-owned Kernel.
        &raw const unsafe { super::super::record::Kernel::current() }.elements
    }
}

impl Cx<'_> {
    pub(crate) fn display_pool(&mut self) -> &mut super::display::DisplayPool {
        &mut self.b.kernel.display
    }
    pub fn display_primitive(&mut self, index: u16) -> &mut super::record::PRIMITIVE {
        self.b.kernel.display.get_mut(index)
    }
    pub(crate) fn reset_display(&mut self) {
        self.b.kernel.display.reinit();
    }
    pub(crate) fn clear_display_order(&mut self) {
        self.b.kernel.display.clear_order();
    }
    pub(crate) fn append_display(&mut self, index: u16) {
        self.b.kernel.display.insert_before(index, None);
    }
    pub(crate) fn display_batch(&self) -> Vec<super::record::PRIMITIVE> {
        self.b.kernel.display.batch()
    }
}

impl super::Battle {
    pub fn ships(
        &mut self,
        side: usize,
    ) -> super::link_queue::LinkQueue<'_, super::link_queue::Starship> {
        super::link_queue::LinkQueue::new(&mut self.kernel.races[side])
    }
    pub fn reset_ship_capacity(&mut self, side: usize, capacity: usize) {
        self.kernel.races[side].reset_capacity(capacity);
    }
}
impl Cx<'_> {
    /// # Safety
    /// Legacy game-thread access must not retain record borrows across callbacks.
    #[cfg(not(test))]
    pub(crate) unsafe fn records(&mut self) -> &mut super::table::Records {
        self.b.kernel.elements.records_mut()
    }
}

#[path = "androsynth_template.rs"]
pub(crate) mod androsynth_template;
#[path = "template.rs"]
pub mod template;
#[path = "views.rs"]
pub mod views;
#[path = "vux_template.rs"]
pub(crate) mod vux_template;

#[cfg(test)]
pub(crate) fn fixture_element_state(record: &super::record::ElementRecord) -> ElementState {
    observe_element_record(record)
}

pub(super) mod links;

#[cfg(not(test))]
impl super::RaceDesc {
    pub(crate) fn info_pointer(&mut self) -> *mut super::record::SHIP_INFO {
        &raw mut self.raw.ship_info
    }
}

impl Cx<'_> {
    pub(crate) fn groups(&self) -> &super::record::QUEUE {
        self.project(self.world.groups)
    }
}

#[cfg(not(test))]
impl super::RaceDesc {
    pub(crate) fn data_pointer(&mut self) -> *mut super::record::DATA_STUFF {
        &raw mut self.raw.ship_data
    }
}
