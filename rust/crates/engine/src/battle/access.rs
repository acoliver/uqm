//! Stage B resource and record projections. Borrows end before legacy handoffs.
impl Cx<'_> {
    pub fn alloc_element(&mut self) -> Option<ElementId> {
        let allocated = self.kernel.elements.alloc();
        let address = if let Some(id) = allocated {
            self.assign_primitive(id);
            self.primitive_style_mut(id).kind = 7;
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
        let index = self.kernel.elements[id].prim_index;
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

use super::Parent;
use super::{Cx, ElementId, ShipId};

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
        let id = self.ship(ship).ship_element?;
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
        image!(frame;self.kernel.elements.record_mut(id).intersect_control.IntersectStamp.frame,frame.raw());
        self.kernel.elements.record_mut(id).intersect_frame = frame;
    }
}

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
    pub fn observe_special(&self, ship: ShipId, marker: fn(&mut Cx<'_>, ShipId)) {
        crate::collections::observation::observe(marker as usize, self.ship_pointer(ship) as usize);
    }
}

pub fn invoke<R>(callback: usize, parent: usize, call: impl FnOnce() -> R) -> R {
    crate::collections::observation::observe(callback, parent);
    call()
}

impl Cx<'_> {
    /// Stamp the ship's captain background into the status context and release it.
    pub fn draw_final_captain(&mut self, ship: ShipId) {
        let frame = self.desc_mut(ship).take_captain_background();
        let old = crate::graphics::menu::select(self.kernel.status_context);
        crate::graphics::menu::stamp(frame, crate::graphics::geometry::Point { x: 0, y: 0 });
        drop(crate::graphics::drawable::LoadedDrawable::from_frame(frame));
        crate::graphics::menu::select(old);
    }
}

use crate::graphics::drawable::FrameId;
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
    pub(super) fn abs_frame(&self, frame: FrameId, index: u16) -> FrameId {
        frame.at(index)
    }
    pub(super) fn equ_frame(&self, frame: FrameId, source: FrameId) -> FrameId {
        frame.equal_index(source)
    }
    pub(super) fn inc_frame(&self, frame: FrameId) -> FrameId {
        frame.next()
    }
    pub(super) fn frame_index(&self, frame: FrameId) -> u16 {
        frame.index()
    }
}

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
        e.intersect_control.IntersectStamp.origin.x = e.current.location.x >> 2;
        e.intersect_control.IntersectStamp.origin.y = e.current.location.y >> 2;
    }

    #[inline]
    pub fn init_intersect_end(&mut self, id: ElementId) {
        let e = self.kernel.elements.record_mut(id);
        e.intersect_control.EndPoint.x = e.next.location.x >> 2;
        e.intersect_control.EndPoint.y = e.next.location.y >> 2;
    }
}

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

/// The drawable and index of the element's selected frame, the drawable and
/// index the zoom level should select, and for the medium view the selected
/// mipmap image and the small view's normal image at the same index.
pub type ZoomSample = ((usize, u16), (usize, u16), Option<(usize, usize)>);

impl Cx<'_> {
    /// Samples which zoom-level frame an element shows, for the melee fixture's
    /// trilinear checks.
    pub fn zoom_sample(
        &self,
        frames: Frames,
        element: ElementId,
        view: usize,
        index: u16,
    ) -> ZoomSample {
        assert!(view < 3, "fixture zoom view");
        let frame = self.element(element).current.image.frame.raw();
        let expected = frames.view(view).raw();
        let mipmap = (view == 1).then(|| {
            let adjacent = frames.view(view + 1).raw().at(index);
            let selected = frame
                .image_identity()
                .unwrap_or_else(|| panic!("zoom frame has no image"));
            let adjacent = adjacent
                .image_identity()
                .unwrap_or_else(|| panic!("zoom frame has no image"));
            (selected.1.unwrap_or(0) as usize, adjacent.0 as usize)
        });
        let owner = |frame: FrameId| frame.drawable().map_or(0, |drawable| drawable.bits());
        (
            (owner(frame), frame.index()),
            (owner(expected), index),
            mipmap,
        )
    }

    pub fn frame_count(&self, frame: FrameRef<'_>) -> u16 {
        frame.raw().count()
    }
    pub fn retreat_frame(&self, frame: FrameRef<'_>) -> FrameSelection {
        frame.to_owned().with_raw(frame.raw().previous())
    }
    pub fn relative_frame(&self, frame: FrameRef<'_>, offset: i16) -> FrameSelection {
        frame.to_owned().with_raw(frame.raw().relative(offset))
    }
    pub fn frame_rect(&self, frame: FrameRef<'_>) -> crate::graphics::geometry::Rect {
        frame.raw().rect().unwrap_or_default()
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
        let index = usize::from(self.kernel.elements[id].prim_index);
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
        let index = self.kernel.elements[destination].prim_index;
        let source_index = usize::from(self.kernel.elements[source].prim_index);
        let primitive = self.kernel.display.get(source_index as u16).clone();
        let value = self.kernel.elements[source].clone();
        let address = std::ptr::from_ref(&self.kernel.elements[source]) as usize;
        crate::collections::observation::copy_image(
            self.kernel.elements.record_mut(destination),
            value,
            address,
        );
        self.kernel.elements.record_mut(destination).prim_index = index;
        *self.primitive(destination) = primitive;
    }
}

impl Cx<'_> {
    pub fn set_ship_sound_bank(&mut self, ship: ShipId, index: u16) {
        assert!(
            index < self.ship_sound_count(ship),
            "invalid sound bank index {index}"
        );
        let sounds = self.desc(ship).ship_data.ship_sounds;
        self.desc_mut(ship).ship_data.ship_sounds = sounds.map(|sounds| sounds.with_index(index));
    }
}

impl Cx<'_> {
    pub fn set_ship_identity(&mut self, id: ShipId, species: u32, index: u16) {
        let record = self.ship_mut(id);
        record.species_id = species;
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

impl Cx<'_> {
    pub fn screen_dimensions(&self) -> (i32, i32) {
        crate::graphics::state::screen_dimensions()
    }

    pub fn sound_point(&self, id: ElementId) -> crate::graphics::geometry::Point {
        let index = usize::from(self.elements()[id].prim_index);
        let primitive = self.kernel.display.get(index as u16);
        primitive.point()
    }
}

impl Cx<'_> {
    pub fn primitive_stamp(&mut self, id: ElementId) -> crate::graphics::intersection::Stamp {
        self.primitive(id).stamp()
    }

    pub fn project_display_stamp(
        &mut self,
        id: ElementId,
        origin: crate::graphics::geometry::Point,
    ) {
        let element = &self.kernel.elements[id];
        self.kernel
            .display
            .get_mut(element.prim_index)
            .set_image(origin, element.next.image.frame.clone());
    }

    pub fn set_primitive_stamp(
        &mut self,
        id: ElementId,
        stamp: crate::graphics::intersection::Stamp,
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

impl Cx<'_> {
    pub fn has_descriptor(&self, ship: ShipId) -> bool {
        !self.ship(ship).descriptor.is_none()
    }
}

fn observe_element_record(record: &super::record::ElementRecord) -> ElementState {
    ElementState::decode(record)
}

#[cfg(any(test, feature = "test-support"))]
pub fn observe_ship_record(record: &super::record::ShipRecord) -> ShipState {
    ShipState::decode(record)
}

#[cfg(any(test, feature = "test-support"))]
pub fn observe_descriptor_record(record: &super::record::RaceDescriptor) -> RaceStats {
    RaceStats::decode(record)
}

impl Cx<'_> {
    pub fn frames_first<'a>(&self, frames: &'a Frames) -> FrameRef<'a> {
        // SAFETY: the supplied frame set belongs to this context's loaded resources.
        frames.view(0)
    }

    pub fn copy_next_frame_set(&mut self, destination: ElementId, source: ElementId) {
        crate::collections::observation::image!(copy; self.kernel.elements.record_mut(destination).current.image.frames, self.kernel.elements[source].next.image.frames);
    }
}

impl Cx<'_> {
    pub fn intersect_elements(&mut self, a: ElementId, b: ElementId, time: u16) -> u16 {
        assert_ne!(a, b, "collision operands alias");
        let (left, right) = self.kernel.elements.pair_mut(a, b);
        crate::graphics::intersection::drawables_intersect(
            &mut left.intersect_control,
            &mut right.intersect_control,
            time,
        )
    }
}

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
        self.ship_mut(ship).ship_element = element;
    }
}

impl Cx<'_> {
    pub fn captain_controls_mut(&mut self, ship: ShipId) -> &mut super::record::CaptainRecord {
        &mut self.desc_mut(ship).ship_data.captain_control
    }
}
impl Cx<'_> {
    pub fn refresh_intersect_frame(&mut self, id: ElementId) {
        let e = self.kernel.elements.record_mut(id);
        let first = e.next.image.frames.view(0).to_owned();
        let raw = first.raw().equal_index(e.next.image.frame.raw());
        e.intersect_frame = first.with_raw(raw);
        crate::collections::observation::image!(frame; e.intersect_control.IntersectStamp.frame, raw);
    }
}

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
        self.ship_mut(ship).species_id = species;
    }
}

#[cfg(test)]
mod intersection_projection_tests {
    use crate::Kernel as KernelOwner;
    #[test]
    #[serial_test::serial]
    fn live_helpers_preserve_record_coordinates_and_pool_addresses() {
        use crate::battle::{Battle, Cx, World};
        use crate::graphics::geometry::Point;
        // SAFETY: a zeroed group queue has no allocated storage.

        // SAFETY: the local queue authorities and their pool remain exclusive for this test.
        {
            let mut battle = Battle::new();
            let mut kernel = KernelOwner::new();
            kernel.elements.reset_capacity(1);
            let id = kernel.elements.alloc().unwrap();
            kernel.elements.record_mut(id).current.location = Point { x: 44, y: 52 };
            kernel.elements.record_mut(id).next.location = Point { x: 60, y: 68 };
            kernel.elements.record_mut(id).next.image.frames = crate::battle::views::Frames::shared(
                crate::battle::frames::SharedFrames::Fixture(0),
                crate::battle::frames::FrameArray::fixture(
                    [crate::graphics::drawable::FrameId::NULL; 3],
                    (),
                ),
            );
            let mut world = World::new();
            let mut cx = Cx::new(&mut battle, &mut world, &mut kernel, || 0);
            cx.init_intersect_start(id);
            cx.init_intersect_end(id);
            assert_eq!(
                cx.elements()[id].intersect_control.IntersectStamp.origin.x,
                11
            );
            assert_eq!(cx.elements()[id].intersect_control.EndPoint.y, 17);
            cx.element_mut(id).flags = 64;
            cx.refresh_intersect_frame(id);
            assert!(cx.elements()[id]
                .intersect_control
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
            assert!(matches!(
                cx.display_batch().primitives()[0],
                crate::graphics::batch::Primitive::Line(..)
            ));
            cx.clear_display_order();
            assert!(cx.display_batch().is_empty());
            cx.reset_display();
            assert_eq!(cx.kernel.display.alloc(), Some(0));
            assert_eq!(cx.display_primitive(0).kind, 3);
            drop(battle);
        }
    }
}

impl Cx<'_> {
    pub fn assign_primitive(&mut self, id: ElementId) {
        self.kernel.elements.record_mut(id).prim_index = self
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
    pub fn display_batch(&self) -> crate::graphics::batch::NativeBatch {
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
pub use ship_assets::CampaignProbeAssets;

impl Cx<'_> {
    pub fn draw_stamp(
        &self,
        frame: super::frames::FrameRef<'_>,
        origin: crate::graphics::geometry::Point,
        in_status_context: bool,
    ) {
        let old =
            in_status_context.then(|| crate::graphics::menu::select(self.kernel.status_context));
        crate::graphics::menu::stamp(frame.raw(), origin);
        if let Some(old) = old {
            crate::graphics::menu::select(old);
        }
    }
}

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
        frame.raw().set_mipmap(mipmap.raw());
    }
}

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
        self.desc(ship)
            .sound_bank()
            .map_or(0, |bank| bank.len() as u16)
    }
}

mod record_ops;
