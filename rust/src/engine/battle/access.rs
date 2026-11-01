//! Stage B resource and record projections. Borrows end before legacy handoffs.
#[cfg(not(test))]
impl Cx<'_> {
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
use super::{
    record::{state__bindgen_ty_1, STATE},
    Parent,
};
use super::{CollisionHook, Cx, ElementHook, ElementId, ShipId};

#[cfg(not(test))]
impl Cx<'_> {
    pub fn owner(&self, id: ElementId) -> ShipId {
        match self.parent(id) {
            Parent::Ship(ship) => ship,
            other => panic!("element {id:?} has no ship owner: {other:?}"),
        }
    }

    pub fn parent(&self, id: ElementId) -> Parent {
        // SAFETY: Cx owns the game-thread authorities for this synchronous call.
        unsafe {
            self.b.elements.parent(
                id,
                [
                    super::record::ship_queue(0).as_ptr(),
                    super::record::ship_queue(1).as_ptr(),
                ],
                self.world.groups(),
            )
        }
    }

    pub fn ship_element(&self, ship: ShipId) -> ElementId {
        self.optional_ship_element(ship)
            .unwrap_or_else(|| panic!("{}", "ship has no element"))
    }

    pub fn optional_ship_element(&self, ship: ShipId) -> Option<ElementId> {
        self.b.elements.id(self.ship(ship).hShip)
    }

    pub fn set_parent(&mut self, element: ElementId, parent: Option<ShipId>) {
        self.b.elements[element].pParent =
            parent.map_or(std::ptr::null_mut(), |ship| self.ship_pointer(ship).cast());
    }

    pub fn target(&self, element: ElementId) -> Option<ElementId> {
        self.b.elements.id(self.b.elements[element].hTarget)
    }

    pub fn set_target(&mut self, element: ElementId, target: Option<ElementId>) {
        self.b.elements[element].hTarget =
            target.map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id));
    }

    pub fn put(&mut self, id: ElementId) {
        self.insert_before(id, None);
    }
}

use super::views::*;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn ship_frame_set(&self, id: ShipId, role: usize) -> Frames {
        Frames(self.ship_frames(id, role))
    }
    pub fn ship_frame(&self, id: ShipId, role: usize) -> Frame {
        let array = self.ship_frames(id, role);
        // SAFETY: the checked owner retains the loaded three-frame array.
        Frame(unsafe { *array })
    }
    pub fn advance_frame(&self, frame: Frame) -> Frame {
        Frame(self.inc_frame(frame.0))
    }
    pub fn equivalent_frame(&self, frame: Frame, source: Frame) -> Frame {
        Frame(self.equ_frame(frame.0, source.0))
    }
    pub fn absolute_frame(&self, frame: Frame, index: u16) -> Frame {
        Frame(self.abs_frame(frame.0, index))
    }
    pub fn frame_number(&self, frame: Frame) -> u16 {
        self.frame_index(frame.0)
    }
    pub fn silhouette_frame(&mut self, id: ElementId, frame: Frame, flags: u8) {
        self.silhouette(id, frame, flags);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn weapon_origin(&self, id: ElementId) -> super::Subject {
        self.decode_subject(&self.b.elements[id])
    }

    pub fn set_frame(&mut self, id: ElementId, next: bool, frame: Frame) {
        use crate::collections::observation::image;
        if next {
            image!(frame;self.b.elements[id].next.image.frame,frame.0);
        } else {
            image!(frame;self.b.elements[id].current.image.frame,frame.0);
        }
    }
    pub fn set_frames(&mut self, id: ElementId, next: bool, frames: Frames) {
        use crate::collections::observation::image;
        if next {
            image!(set;self.b.elements[id].next.image.farray,frames.0);
        } else {
            image!(set;self.b.elements[id].current.image.farray,frames.0);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn copy_visual_to_next(&mut self, id: ElementId, visual: Visual) {
        use crate::collections::observation::image;
        let value = STATE {
            location: visual.location,
            image: state__bindgen_ty_1 {
                frame: visual.image.frame.0,
                farray: visual.image.frames.0,
            },
        };
        image!(copy;self.b.elements[id].next,value);
    }
    pub fn element_frame_set_first(&self, id: ElementId, next: bool) -> Frame {
        Frame(self.frame_at(id, next))
    }
    pub fn set_intersect_frame(&mut self, id: ElementId, frame: Frame) {
        use crate::collections::observation::image;
        image!(frame;self.b.elements[id].IntersectControl.IntersectStamp.frame,frame.0);
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

impl super::ElementTable {
    pub fn tail(&self) -> Option<ElementId> {
        // SAFETY: the table's construction contract retains the queue authority.
        self.id(unsafe { (*self.queue()).tail })
    }
    pub fn pred(&self, id: ElementId) -> Option<ElementId> {
        self.id(self[id].pred)
    }
    pub fn linked(&self, id: ElementId) -> bool {
        self.raw(id);
        let mut current = self.head();
        while let Some(entry) = current {
            if entry == id {
                return true;
            }
            current = self.succ(entry);
        }
        false
    }
}

impl Cx<'_> {
    pub(super) fn project<T>(&self, pointer: std::ptr::NonNull<T>) -> &T {
        // SAFETY: record decoders supply checked pointers retained by this context.
        unsafe { &*pointer.as_ptr() }
    }
    pub(super) fn project_mut<T>(&mut self, pointer: std::ptr::NonNull<T>) -> &mut T {
        // SAFETY: the exclusive context borrow scopes one checked record projection.
        unsafe { &mut *pointer.as_ptr() }
    }
}

impl Cx<'_> {
    pub fn element(&self, id: ElementId) -> ElementState {
        ElementState::decode(&self.b.elements[id])
    }
    pub fn ship_state(&self, id: ShipId) -> ShipState {
        ShipState::decode(self.ship(id))
    }
    pub fn race_stats(&self, id: ShipId) -> RaceStats {
        RaceStats::decode(self.desc(id))
    }
}

/// # Safety
/// `call` may use only live legacy storage and must retain no record reference.
pub(crate) unsafe fn invoke_legacy<R>(
    cx: &mut crate::engine::battle::Cx<'_>,
    callback: usize,
    parent: usize,
    call: impl FnOnce() -> R,
) -> R {
    // SAFETY: publishing suspends context access until the legacy invocation returns.
    unsafe { invoke(callback, parent, || crate::app::bridge::publish(cx, call)) }
}

#[cfg(not(test))]
/// # Safety
/// The resource owner thread must retain the initialized descriptor's resources.
pub(crate) unsafe fn descriptor_template(
    source: super::record::DescriptorSource,
) -> Option<super::RaceDesc> {
    // SAFETY: the initializer and its returned template remain live for this synchronous value copy.
    unsafe {
        match source {
            super::record::DescriptorSource::Native(race) => {
                Some((race.descriptor)(&mut super::World::for_legacy_lifecycle()))
            }
            super::record::DescriptorSource::Legacy(init) => {
                let pointer = init();
                if pointer.is_null() {
                    None
                } else {
                    Some(super::RaceDesc::from_legacy(*pointer))
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum CollisionCall {
    Hook,
    Direct,
}

/// # Safety
/// Pointers name live legacy elements and collision points on the game thread.
pub(super) unsafe fn dispatch_legacy_collision<E, P>(
    hook: super::CollisionHook<unsafe extern "C" fn(*mut E, *mut P, *mut E, *mut P)>,
    operands: super::record::CollisionOperands<E, P>,
    call: CollisionCall,
) {
    let super::record::CollisionOperands(a, pa, b, pb) = operands;
    // SAFETY: the caller is a published boundary and retains no record borrow.
    unsafe {
        crate::app::bridge::enter(|cx| match hook {
            super::CollisionHook::Legacy(callback) => {
                let parent = cx.legacy_parent(
                    std::ptr::NonNull::new(a).unwrap_or_else(|| panic!("null collision element")),
                );
                invoke_legacy(cx, callback as usize, parent, || callback(a, pa, b, pb));
            }
            super::CollisionHook::Native(callback) => {
                let a =
                    cx.b.elements
                        .id(a.cast())
                        .unwrap_or_else(|| panic!("{}", "null first collision element"));
                let b =
                    cx.b.elements
                        .id(b.cast())
                        .unwrap_or_else(|| panic!("{}", "null second collision element"));
                match call {
                    CollisionCall::Hook => {
                        let parent = cx.b.elements[a].pParent as usize;
                        invoke(callback as usize, parent, || {
                            callback(cx, a, &mut *pa.cast(), b, &mut *pb.cast())
                        });
                    }
                    CollisionCall::Direct => callback(cx, a, &mut *pa.cast(), b, &mut *pb.cast()),
                }
            }
        });
    }
}

/// # Safety
/// The pointer is a live element; no record borrow crosses this call.
pub(super) unsafe fn dispatch_legacy_element<E>(
    hook: super::ElementHook<unsafe extern "C" fn(*mut E)>,
    operand: super::record::ElementOperand<E>,
) {
    let super::record::ElementOperand(element) = operand;
    // SAFETY: this legacy boundary owns the currently published context.
    unsafe {
        crate::app::bridge::enter(|cx| match hook {
            super::ElementHook::Legacy(callback) => {
                let parent = cx.legacy_parent(
                    std::ptr::NonNull::new(element)
                        .unwrap_or_else(|| panic!("null legacy element")),
                );
                invoke_legacy(cx, callback as usize, parent, || callback(element));
            }
            super::ElementHook::Native(callback) => {
                let id =
                    cx.b.elements
                        .id(element.cast())
                        .unwrap_or_else(|| panic!("{}", "null native element"));
                let parent = cx.b.elements[id].pParent as usize;
                invoke(callback as usize, parent, || callback(cx, id));
            }
        });
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
    pub(super) fn ship_frames(&self, ship: ShipId, role: usize) -> *mut FRAME {
        let pointer = self.ship(ship).RaceDescPtr;
        // SAFETY: the ship owns this descriptor and its stable embedded arrays.
        unsafe {
            match role {
                0 => (&raw mut (*pointer).ship_data.ship).cast(),
                1 => (&raw mut (*pointer).ship_data.weapon).cast(),
                2 => (&raw mut (*pointer).ship_data.special).cast(),
                _ => panic!("invalid frame role {role}"),
            }
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
    pub(super) fn frame_at(&self, element: ElementId, next: bool) -> FRAME {
        let array = if next {
            self.b.elements[element].next.image.farray
        } else {
            self.b.elements[element].current.image.farray
        };
        assert!(!array.is_null(), "element {element:?} has no frame set");
        // SAFETY: the array is retained by the descriptor or shared battle resources.
        unsafe { *array }
    }
    pub(super) fn frame_index(&self, frame: FRAME) -> u16 {
        // SAFETY: this battle retains the element's loaded frame resources.
        unsafe { crate::graphics::pixmap_abi::GetFrameIndex(frame.cast()) }
    }
    pub(super) fn shared_frames(&self, role: SharedFrames) -> *mut FRAME {
        match role {
            SharedFrames::Asteroid => self.world.shared_frames[0].0,
            SharedFrames::Explosion => self.world.shared_frames[1].0,
        }
    }
    pub(super) fn shared_frame(&self, role: SharedFrames) -> FRAME {
        // SAFETY: shared frame sets stay loaded until battle cleanup.
        unsafe { *self.shared_frames(role) }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn shared_frame_set(&self, role: SharedFrames) -> Frames {
        Frames(self.shared_frames(role))
    }
    pub fn first_shared_frame(&self, role: SharedFrames) -> Frame {
        Frame(self.shared_frame(role))
    }
    pub fn copy_image_to_current(&mut self, id: ElementId, image: Image) {
        use crate::collections::observation::image;
        let raw = state__bindgen_ty_1 {
            frame: image.frame.0,
            farray: image.frames.0,
        };
        image!(copy; self.b.elements[id].current.image, raw);
    }
}

/// # Safety
/// The pointer names a live legacy element with no record borrow across the hook.
pub unsafe fn invoke_legacy_element<E>(
    hook: ElementHook<unsafe extern "C" fn(*mut E)>,
    element: *mut E,
) {
    let operand = super::record::ElementOperand(element);
    // SAFETY: the caller retains the element through the synchronous hook.
    unsafe {
        dispatch_legacy_element(hook, operand);
    }
}

/// # Safety
/// The pointers name live legacy collision operands; Native hooks require live points.
pub unsafe fn invoke_legacy_collision<E, P>(
    hook: CollisionHook<unsafe extern "C" fn(*mut E, *mut P, *mut E, *mut P)>,
    a: *mut E,
    pa: *mut P,
    b: *mut E,
    pb: *mut P,
    call: CollisionCall,
) {
    let operands = super::record::CollisionOperands(a, pa, b, pb);
    // SAFETY: the caller retains all operands through the synchronous hook.
    unsafe {
        dispatch_legacy_collision(hook, operands, call);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn init_intersect_start(&mut self, id: ElementId) {
        let e = &mut self.b.elements[id];
        e.IntersectControl.IntersectStamp.origin.x = e.current.location.x >> 2;
        e.IntersectControl.IntersectStamp.origin.y = e.current.location.y >> 2;
    }

    pub fn init_intersect(&mut self, id: ElementId) {
        self.init_intersect_start(id);
        let e = &mut self.b.elements[id];
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
    pub fn track_subject(&mut self, subject: &mut super::Subject, facing: &mut u16) -> i16 {
        let mut raw = self.encode_subject(subject);
        let tracking = self
            .world
            .tracking
            .unwrap_or_else(|| panic!("world has no legacy tracking dependency"));
        // SAFETY: construction supplies the tracker and the checked projection retains its resources.
        let result = unsafe { tracking(&mut raw, facing) };
        *subject = self.decode_subject(&raw);
        result
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn frame_count(&self, frame: Frame) -> u16 {
        // SAFETY: opaque frames originate in retained battle resources, or are absent.
        unsafe { crate::graphics::pixmap_abi::GetFrameCount(frame.0.cast()) }
    }
    pub fn retreat_frame(&self, frame: Frame) -> Frame {
        // SAFETY: the context retains the frame's drawable.
        Frame(unsafe { crate::graphics::pixmap_abi::DecFrameIndex(frame.0.cast()).cast() })
    }
    pub fn relative_frame(&self, frame: Frame, offset: i16) -> Frame {
        // SAFETY: the context retains the frame's drawable.
        Frame(unsafe {
            crate::graphics::pixmap_abi::SetRelFrameIndex(frame.0.cast(), offset).cast()
        })
    }
    pub fn frame_rect(&self, frame: Frame) -> crate::graphics::native_abi::RECT {
        let mut rect = crate::graphics::native_abi::RECT {
            corner: crate::graphics::native_abi::POINT { x: 0, y: 0 },
            extent: crate::graphics::native_abi::EXTENT {
                width: 0,
                height: 0,
            },
        };
        // SAFETY: the resource owner retains the frame and rect is local output.
        unsafe {
            crate::graphics::drawable_abi::GetFrameRect(frame.0.cast(), &mut rect);
        }
        rect
    }
    pub fn set_ship_frame(&mut self, ship: ShipId, role: usize, resolution: usize, frame: Frame) {
        assert!(resolution < 3, "invalid frame resolution {resolution}");
        let array = self.ship_frames(ship, role);
        // SAFETY: the checked descriptor retains the slot and both opaque drawable handles.
        unsafe {
            let slot = array.add(resolution);
            assert_eq!(
                crate::graphics::pixmap_abi::GetFrameParentDrawable((*slot).cast()),
                crate::graphics::pixmap_abi::GetFrameParentDrawable(frame.0.cast()),
                "owning frame slot cannot change drawable"
            );
            crate::collections::observation::image!(view; *slot, frame.0);
        }
    }
    pub fn captain_special(&self, ship: ShipId) -> Frame {
        Frame(self.desc(ship).ship_data.captain_control.special)
    }
    pub fn set_captain_special(&mut self, ship: ShipId, frame: Frame) {
        self.desc_mut(ship).ship_data.captain_control.special = frame.0;
    }
    pub fn primitive_style(&self, id: ElementId) -> PrimitiveStyle {
        let index = usize::from(self.b.elements[id].PrimIndex);
        let record = self.project(super::record::primitive_slot(index));
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
        let index = self.b.elements[destination].PrimIndex;
        let source_index = usize::from(self.b.elements[source].PrimIndex);
        let primitive = *self.project(super::record::primitive_slot(source_index));
        let value = self.b.elements[source];
        let address = std::ptr::from_ref(&self.b.elements[source]) as usize;
        crate::collections::observation::copy_image(
            &mut self.b.elements[destination],
            value,
            address,
        );
        self.b.elements[destination].PrimIndex = index;
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
    pub(crate) fn pick_ship(
        &self,
        id: crate::engine::battle::ShipId,
    ) -> crate::engine::battle::link_queue::PickShip {
        // SAFETY: Cx owns this pool and no record borrow survives this value read.
        unsafe {
            let ships = crate::engine::battle::link_queue::ships(usize::from(id.side));
            ships.get(ships.link(id))
        }
    }
    pub(crate) fn subject_style(
        &self,
        subject: &Subject,
    ) -> crate::engine::battle::views::PrimitiveStyle {
        let primitive = self.project(crate::engine::battle::record::primitive_slot(usize::from(
            subject.primitive,
        )));
        crate::engine::battle::views::PrimitiveStyle {
            kind: primitive.Type,
            color: primitive.color,
        }
    }
    pub(crate) fn subject_intersect_frame(
        &self,
        subject: &Subject,
    ) -> crate::engine::battle::views::Frame {
        crate::engine::battle::views::Frame(subject.intersect.IntersectStamp.frame)
    }
}

pub(super) mod links;

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn sound_point(&self, id: ElementId) -> crate::graphics::native_abi::POINT {
        let index = usize::from(self.elements()[id].PrimIndex);
        let primitive = self.project(crate::engine::battle::record::primitive_slot(index));
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
