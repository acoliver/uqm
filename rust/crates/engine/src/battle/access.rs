//! Stage B resource and record projections. Borrows end before legacy handoffs.
#[cfg(not(test))]
impl Cx<'_> {
    pub fn alloc_element(&mut self) -> Option<ElementId> {
        let allocated = self.kernel.elements.alloc();
        let address = if let Some(id) = allocated {
            self.assign_primitive(id);
            self.primitive_style_mut(id).kind = crate::graphics::native_abi::EMPTY_PRIMITIVE;
            self.kernel.elements.raw(id) as usize
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
            !self.kernel.elements.linked(id),
            "dispose requires an unlinked element"
        );
        let address = self.kernel.elements.raw(id) as usize;
        let index = self.kernel.elements[id].PrimIndex;
        self.release_primitive(index);
        crate::collections::observation::record(crate::collections::observation::Event::Free(
            address,
        ));
        self.kernel.elements.free(id);
    }

    pub fn finish_battle_storage(&mut self) {
        self.kernel.finish_battle();
    }

    pub fn complete_ship_reset(&mut self) {
        self.kernel.complete_ship_reset();
    }

    /// Move the boss result before descriptor disposal; no live graph is retained.
    pub fn retain_terminal_outcome(&mut self, owner: ShipId) {
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

    #[inline]
    pub fn parent(&self, id: ElementId) -> Parent {
        self.kernel.elements[id].parent
    }

    #[inline]
    pub fn ship_element(&self, ship: ShipId) -> ElementId {
        self.optional_ship_element(ship)
            .unwrap_or_else(|| panic!("{}", "ship has no element"))
    }

    #[inline]
    pub fn optional_ship_element(&self, ship: ShipId) -> Option<ElementId> {
        let id = self.ship(ship).hShip?;
        assert!(self.elements().contains(id), "stale ship element {id:?}");
        Some(id)
    }

    #[inline]
    pub fn set_parent(&mut self, element: ElementId, parent: Option<ShipId>) {
        self.kernel.elements.record_mut(element).parent = parent.map_or(Parent::None, Parent::Ship);
    }

    #[inline]
    pub fn target(&self, element: ElementId) -> Option<ElementId> {
        self.kernel.elements[element].target
    }

    pub fn set_target(&mut self, element: ElementId, target: Option<ElementId>) {
        if let Some(id) = target {
            assert!(self.kernel.elements.contains(id), "stale target {id:?}");
        }
        self.kernel.elements.record_mut(element).target = target;
    }

    pub fn set_ip_group_parent(
        &mut self,
        id: ElementId,
        group: super::link_queue::LinkId<crate::world::queues::Group>,
    ) {
        assert!(
            self.groups().contains(group),
            "stale or foreign IP-group parent"
        );
        self.kernel.elements.record_mut(id).parent = Parent::IpGroup(group);
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
    #[inline]
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
    #[inline]
    pub fn frame_number(&self, frame: FrameRef<'_>) -> u16 {
        self.frame_index(frame.raw())
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    #[inline]
    pub fn weapon_origin(&self, id: ElementId) -> super::Subject {
        Subject::decode(&self.kernel.elements[id])
    }

    pub fn set_frame(&mut self, id: ElementId, next: bool, frame: FrameSelection) {
        use crate::collections::observation::image;
        if next {
            image!(frame;self.kernel.elements.record_mut(id).next.image.frame,frame);
        } else {
            image!(frame;self.kernel.elements.record_mut(id).current.image.frame,frame);
        }
    }
    pub fn set_single_frames(&mut self, id: ElementId, frames: Frames, frame: FrameSelection) {
        crate::collections::observation::image!(single; self.kernel.elements.record_mut(id).current.image.frames, frames, frame.raw());
    }

    pub fn set_frames(&mut self, id: ElementId, next: bool, frames: Frames) {
        use crate::collections::observation::image;
        if next {
            image!(set;self.kernel.elements.record_mut(id).next.image.frames,frames);
        } else {
            image!(set;self.kernel.elements.record_mut(id).current.image.frames,frames);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    #[inline]
    pub fn copy_visual_to_next(&mut self, id: ElementId, visual: Visual) {
        self.kernel.elements.record_mut(id).next.location = visual.location;
        self.set_frame(id, true, visual.image.frame);
        self.set_frames(id, true, visual.image.frames);
    }
    pub fn element_frame_set_first(&self, id: ElementId, next: bool) -> FrameRef<'_> {
        let record = &self.kernel.elements[id];
        let image = if next {
            &record.next.image
        } else {
            &record.current.image
        };
        image.frames.view(0)
    }
    pub fn set_intersect_frame(&mut self, id: ElementId, frame: FrameSelection) {
        use crate::collections::observation::image;
        image!(frame;self.kernel.elements.record_mut(id).IntersectControl.IntersectStamp.frame,frame.raw());
        self.kernel.elements.record_mut(id).intersect_frame = frame;
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    #[inline]
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
    #[inline]
    pub fn element(&self, id: ElementId) -> ElementState {
        ElementState::decode(&self.kernel.elements[id])
    }
    #[inline]
    pub fn ship_state(&self, id: ShipId) -> ShipState {
        ShipState::decode(self.ship(id))
    }
    #[inline]
    pub fn race_stats(&self, id: ShipId) -> RaceStats {
        RaceStats::decode(self.desc(id))
    }
    /// Records a SPECIAL activation that runs inline in a race callback under
    /// that race's `special` hook. `marker` only names the hook; it is not called.
    #[cfg(not(test))]
    pub fn observe_special(&self, ship: ShipId, marker: fn(&mut Cx<'_>, ShipId)) {
        crate::collections::observation::observe(marker as usize, self.ship_pointer(ship) as usize);
    }
}

pub fn invoke<R>(callback: usize, parent: usize, call: impl FnOnce() -> R) -> R {
    crate::collections::observation::observe(callback, parent);
    call()
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn take_captain_background(&mut self, ship: ShipId) -> crate::graphics::native_abi::FRAME {
        self.desc_mut(ship).take_captain_background()
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
        Frames::retain_shared(role, self.kernel.shared_frame_source(role))
    }
    #[cfg(not(test))]
    pub fn setup_element(&mut self, id: ElementId) {
        self.copy_current_to_next(id);
        if self.element(id).flags & ((1 << 4) | (1 << 6)) == 0 {
            self.init_intersect(id);
            let frame = self.equivalent_frame(
                self.element_frame_set_first(id, true),
                self.element(id).next.image.frame.view(),
            );
            self.set_intersect_frame(id, frame);
        }
    }

    pub fn init_intersect(&mut self, id: ElementId) {
        self.init_intersect_start(id);
        self.init_intersect_end(id);
    }

    #[inline]
    pub fn init_intersect_start(&mut self, id: ElementId) {
        let e = self.kernel.elements.record_mut(id);
        e.IntersectControl.IntersectStamp.origin.x = e.current.location.x >> 2;
        e.IntersectControl.IntersectStamp.origin.y = e.current.location.y >> 2;
    }

    #[inline]
    pub fn init_intersect_end(&mut self, id: ElementId) {
        let e = self.kernel.elements.record_mut(id);
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
        fn apply(p: &mut super::display::DisplayPrimitive, v: PrimitiveStyle) {
            p.kind = v.kind;
            p.color = v.color;
        }
        let record = self.primitive(id);
        let value = PrimitiveStyle {
            kind: record.kind,
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
        let index = usize::from(self.kernel.elements[id].PrimIndex);
        let record = self.kernel.display.get(index as u16);
        PrimitiveStyle {
            kind: record.kind,
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
        let index = self.kernel.elements[destination].PrimIndex;
        let source_index = usize::from(self.kernel.elements[source].PrimIndex);
        let primitive = self.kernel.display.get(source_index as u16).clone();
        let value = self.kernel.elements[source].clone();
        let address = std::ptr::from_ref(&self.kernel.elements[source]) as usize;
        crate::collections::observation::copy_image(
            self.kernel.elements.record_mut(destination),
            value,
            address,
        );
        self.kernel.elements.record_mut(destination).PrimIndex = index;
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
    pub fn set_ship_identity(&mut self, id: ShipId, species: u32, index: u16) {
        let record = self.ship_mut(id);
        record.SpeciesID = species;
        record.index = index;
    }

    pub fn queued_ships(&self, side: u8) -> Vec<ShipId> {
        let queue = super::link_queue::LinkQueue::borrow(&self.kernel.races[usize::from(side)]);
        let mut result = Vec::new();
        let mut next = queue.head();
        while let Some(id) = next {
            result.push(queue.ship_id(id));
            next = queue.succ(id);
        }
        result
    }

    pub fn first_ship(&self, side: usize) -> Option<ShipId> {
        let ships = super::link_queue::LinkQueue::borrow(&self.kernel.races[side]);
        ships.head().map(|id| ships.ship_id(id))
    }

    pub fn pick_ship(&self, id: crate::battle::ShipId) -> crate::battle::link_queue::PickShip {
        let ships =
            super::link_queue::LinkQueue::borrow(&self.kernel.races[usize::from(id.side())]);
        ships.get(ships.link(id))
    }
    pub fn subject_style(&self, subject: &Subject) -> crate::battle::views::PrimitiveStyle {
        let primitive = self.kernel.display.get(subject.primitive);
        crate::battle::views::PrimitiveStyle {
            kind: primitive.kind,
            color: primitive.color,
        }
    }
    pub fn subject_intersect_frame<'a>(&self, subject: &'a Subject) -> FrameRef<'a> {
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
    pub fn screen_dimensions(&self) -> (i32, i32) {
        use crate::graphics::gfx_common_abi::{ScreenHeight, ScreenWidth};
        // SAFETY: dimensions are initialized by the owning game thread.
        unsafe { (ScreenWidth, ScreenHeight) }
    }

    pub fn sound_point(&self, id: ElementId) -> crate::graphics::native_abi::POINT {
        let index = usize::from(self.elements()[id].PrimIndex);
        let primitive = self.kernel.display.get(index as u16);
        primitive.point()
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn primitive_stamp(&mut self, id: ElementId) -> crate::graphics::native_abi::STAMP {
        self.primitive(id).stamp()
    }

    pub fn project_display_stamp(
        &mut self,
        id: ElementId,
        origin: crate::graphics::native_abi::POINT,
    ) {
        let element = &self.kernel.elements[id];
        self.kernel
            .display
            .get_mut(element.PrimIndex)
            .set_image(origin, element.next.image.frame.clone());
    }

    pub fn set_primitive_stamp(
        &mut self,
        id: ElementId,
        stamp: crate::graphics::native_abi::STAMP,
    ) {
        self.primitive(id).set_stamp(stamp);
    }

    pub fn copy_current_to_next(&mut self, id: ElementId) {
        let record = self.kernel.elements.record_mut(id);
        crate::collections::observation::image!(copy; record.next, record.current);
    }

    pub fn copy_next_to_current(&mut self, id: ElementId) {
        let record = self.kernel.elements.record_mut(id);
        crate::collections::observation::image!(copy; record.current, record.next);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn has_descriptor(&self, ship: ShipId) -> bool {
        !self.ship(ship).descriptor.is_none()
    }
}

fn observe_element_record(record: &super::record::ElementRecord) -> ElementState {
    ElementState::decode(record)
}

/// # Safety
/// The caller retains the ship pool throughout this staging observation.
#[cfg(any(test, feature = "test-support"))]
pub unsafe fn observe_ship_record(record: &super::record::ShipRecord) -> ShipState {
    ShipState::decode(record)
}

#[cfg(any(test, feature = "test-support"))]
/// # Safety
/// The caller retains the test descriptor and all resources referenced by its state.
pub unsafe fn observe_descriptor_record(record: &super::record::RaceDescriptor) -> RaceStats {
    RaceStats::decode(record)
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn frames_first<'a>(&self, frames: &'a Frames) -> FrameRef<'a> {
        // SAFETY: the supplied frame set belongs to this context's loaded resources.
        frames.view(0)
    }

    pub fn copy_next_frame_set(&mut self, destination: ElementId, source: ElementId) {
        crate::collections::observation::image!(copy; self.kernel.elements.record_mut(destination).current.image.frames, self.kernel.elements[source].next.image.frames);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn intersect_elements(&mut self, a: ElementId, b: ElementId, time: u16) -> u16 {
        assert_ne!(a, b, "collision operands alias");
        let (left, right) = self.kernel.elements.pair_mut(a, b);
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
    pub fn attach_descriptor(&mut self, ship: ShipId, descriptor: super::RaceDesc) {
        self.ship_mut(ship).descriptor = Some(descriptor.into_storage());
    }
    pub fn take_descriptor(&mut self, ship: ShipId) -> Option<super::RaceDesc> {
        self.ship_mut(ship)
            .descriptor
            .take()
            .map(super::RaceDesc::from_storage)
    }

    pub fn next_ship(&self, ship: ShipId) -> Option<ShipId> {
        let queue =
            super::link_queue::LinkQueue::borrow(&self.kernel.races[usize::from(ship.side())]);
        queue.succ(queue.link(ship)).map(|id| queue.ship_id(id))
    }
    pub fn reset_ships(&mut self, side: usize) {
        self.kernel.races[side].reinit();
    }
    pub fn set_ship_element(&mut self, ship: ShipId, element: Option<ElementId>) {
        self.ship_mut(ship).hShip = element;
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn captain_controls_mut(&mut self, ship: ShipId) -> &mut super::record::CAPTAIN_STUFF {
        &mut self.desc_mut(ship).ship_data.captain_control
    }
}
impl Cx<'_> {
    pub fn refresh_intersect_frame(&mut self, id: ElementId) {
        let e = self.kernel.elements.record_mut(id);
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
    pub fn status_offset(&mut self, ship: ShipId) -> i16 {
        self.kernel.status_offsets[self.ship_state(ship).player as usize]
    }
    pub fn winner(&mut self) -> Option<ShipId> {
        self.kernel.winner
    }
    pub fn set_winner_slot(&mut self, ship: Option<ShipId>) {
        self.kernel.winner = ship;
    }
    pub fn set_ship_species(&mut self, ship: ShipId, species: u32) {
        self.ship_mut(ship).SpeciesID = species;
    }
}

#[cfg(test)]
mod intersection_projection_tests {
    use crate::Kernel as KernelOwner;
    #[test]
    #[serial_test::serial]
    fn live_helpers_preserve_record_coordinates_and_pool_addresses() {
        use crate::battle::{Battle, Cx, World};
        use crate::graphics::native_abi::POINT;
        // SAFETY: a zeroed group queue has no allocated storage.

        // SAFETY: the local queue authorities and their pool remain exclusive for this test.
        {
            let mut battle = Battle::new();
            let mut kernel = KernelOwner::new();
            kernel.elements.reset_capacity(1);
            let id = kernel.elements.alloc().unwrap();
            kernel.elements.record_mut(id).current.location = POINT { x: 44, y: 52 };
            kernel.elements.record_mut(id).next.location = POINT { x: 60, y: 68 };
            kernel.elements.record_mut(id).next.image.frames = crate::battle::views::Frames::shared(
                crate::battle::frames::SharedFrames::Fixture(0),
                crate::battle::frames::FrameArray::fixture([std::ptr::null_mut(); 3], ()),
            );
            let mut world = World::new();
            let mut cx = Cx::new(&mut battle, &mut world, &mut kernel, || 0);
            cx.init_intersect_start(id);
            cx.init_intersect_end(id);
            assert_eq!(
                cx.elements()[id].IntersectControl.IntersectStamp.origin.x,
                11
            );
            assert_eq!(cx.elements()[id].IntersectControl.EndPoint.y, 17);
            cx.element_mut(id).flags = 64;
            cx.refresh_intersect_frame(id);
            assert!(cx.elements()[id]
                .IntersectControl
                .IntersectStamp
                .frame
                .is_null());
            assert_eq!(
                cx.elements()
                    .id(std::ptr::NonNull::new(cx.elements().raw(id))),
                Some(id)
            );
            assert_eq!(
                cx.elements()
                    .id(std::ptr::NonNull::new(std::ptr::null_mut())),
                None
            );
            let primitive = cx.display_pool().alloc().unwrap();
            cx.kernel.display.get_mut(primitive).kind = 3;
            assert_eq!(cx.display_primitive(primitive).kind, 3);
            cx.append_display(primitive);
            assert_eq!(cx.display_batch().primitives()[0].Type, 3);
            cx.clear_display_order();
            assert!(cx.display_batch().is_empty());
            cx.reset_display();
            assert_eq!(cx.kernel.display.alloc(), Some(0));
            assert_eq!(cx.display_primitive(0).kind, 3);
            drop(battle);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn assign_primitive(&mut self, id: ElementId) {
        self.kernel.elements.record_mut(id).PrimIndex = self
            .kernel
            .display
            .alloc()
            .unwrap_or_else(|| panic!("AllocElement: Out of display prims!"));
    }
    pub fn release_primitive(&mut self, index: u16) {
        self.kernel.display.free(index);
    }
    pub fn link_storage(&mut self, id: ElementId, before: Option<ElementId>) {
        self.kernel.elements.insert_before(id, before);
    }
    pub fn unlink_storage(&mut self, id: ElementId) {
        self.kernel.elements.remove(id);
    }
}
pub mod observe;

pub mod schema;

pub mod lifecycle;

impl Cx<'_> {
    pub fn display_pool(&mut self) -> &mut super::display::DisplayPool {
        &mut self.kernel.display
    }
    pub fn display_primitive(&mut self, index: u16) -> &mut super::display::DisplayPrimitive {
        self.kernel.display.get_mut(index)
    }
    pub fn reset_display(&mut self) {
        self.kernel.display.reinit();
    }
    pub fn clear_display_order(&mut self) {
        self.kernel.display.clear_order();
    }
    pub fn append_display(&mut self, index: u16) {
        self.kernel.display.insert_before(index, None);
    }
    pub fn display_batch(&self) -> crate::graphics::native_batch::NativeBatch {
        self.kernel.display.batch()
    }
}

#[path = "androsynth_template.rs"]
pub mod androsynth_template;
#[path = "template.rs"]
pub mod template;
#[path = "views.rs"]
pub mod views;
#[path = "vux_template.rs"]
pub mod vux_template;

#[cfg(any(test, feature = "test-support"))]
pub fn fixture_element_state(record: &super::record::ElementRecord) -> ElementState {
    observe_element_record(record)
}

#[path = "access/processing.rs"]
mod processing;

#[path = "ship_assets.rs"]
mod ship_assets;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn draw_stamp(
        &self,
        frame: super::frames::FrameRef<'_>,
        origin: crate::graphics::native_abi::POINT,
        in_status_context: bool,
    ) {
        let mut stamp = crate::graphics::native_abi::STAMP {
            origin,
            frame: frame.raw(),
        };
        // SAFETY: the frame borrow retains its registered drawable through the synchronous draw.
        unsafe {
            let old = in_status_context
                .then(|| crate::graphics::context_abi::SetContext(self.kernel.status_context));
            crate::graphics::frame_abi::DrawStamp(&mut stamp);
            if let Some(old) = old {
                crate::graphics::context_abi::SetContext(old);
            }
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn set_mipmap(
        &self,
        frame: super::frames::FrameRef<'_>,
        mipmap: super::frames::FrameRef<'_>,
    ) {
        assert!(
            !frame.is_absent() && !mipmap.is_absent(),
            "mipmap requires two loaded frames"
        );
        let frame = frame.raw();
        let mipmap = mipmap.raw();
        // SAFETY: both non-absent frame borrows retain their allocations during the image update.
        unsafe {
            let hot = crate::graphics::drawable_abi::GetFrameHot(mipmap);
            crate::graphics::tfb_draw_abi::TFB_DrawScreen_SetMipmap(
                (*frame).image,
                (*mipmap).image,
                i32::from(hot.x),
                i32::from(hot.y),
            );
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn insert_before(&mut self, id: ElementId, before: Option<ElementId>) {
        assert!(!self.elements().linked(id), "element already linked");
        if let Some(before) = before {
            assert!(
                self.elements().linked(before),
                "insertion reference is not linked"
            );
        }
        self.link_storage(id, before);
    }
    pub fn ship_sound_count(&self, ship: ShipId) -> u16 {
        let sounds = self.desc(ship).ship_data.ship_sounds;
        // SAFETY: loaded sound entries are retained by the descriptor.
        unsafe { crate::strings::string_table::GetStringTableCount(sounds.cast()) }
    }
}
