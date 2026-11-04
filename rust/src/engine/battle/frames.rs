//! Frame-array identity and shared ownership of its loaded drawables.

use super::ShipId;
#[cfg(test)]
use crate::engine::Kernel as KernelOwner;
use crate::graphics::drawable_abi::{retain_frame_allocations, DrawableLease};
use crate::graphics::native_abi::FRAME;
use std::{cell::Cell, rc::Rc};

struct Array {
    views: [Cell<FRAME>; 3],
    leases: Vec<DrawableLease>,
    #[cfg(test)]
    _fixture: Option<Box<dyn std::any::Any>>,
    source: Option<usize>,
}

#[derive(Clone, Default)]
pub struct FrameArray(Option<Rc<Array>>);
impl FrameArray {
    pub const EMPTY: Self = Self(None);

    pub fn retain(views: [FrameRef<'_>; 3]) -> Self {
        Self::from_raw(views.map(FrameRef::raw), None)
    }

    /// Retain registered frames, rejecting unknown or retired addresses.
    pub fn from_native(views: [FRAME; 3]) -> Self {
        Self::from_raw(views, None)
    }

    fn from_raw(views: [FRAME; 3], source: Option<usize>) -> Self {
        let leases = retain_frame_allocations(&views);
        Self(Some(Rc::new(Array {
            views: views.map(|frame| {
                Cell::new(if frame.is_null() {
                    frame
                } else {
                    leases
                        .iter()
                        .find_map(|lease| lease.frame_at_address(frame))
                        .unwrap_or_else(|| panic!("frame has no owning drawable lease"))
                })
            }),
            leases,
            #[cfg(test)]
            _fixture: None,
            source,
        })))
    }

    pub fn view(&self, index: usize) -> FrameRef<'_> {
        FrameRef {
            raw: self
                .0
                .as_ref()
                .map_or(std::ptr::null_mut(), |a| a.views[index].get()),
            owner: self,
            resolution: index,
            identity: FrameOwner::Drawable,
        }
    }

    pub fn views(&self) -> [FrameRef<'_>; 3] {
        std::array::from_fn(|index| self.view(index))
    }

    fn set_view(&self, index: usize, frame: FrameRef<'_>) {
        let array = self
            .0
            .as_ref()
            .unwrap_or_else(|| panic!("absent frame array"));
        let view = &array.views[index];
        crate::collections::observation::record(
            crate::collections::observation::Event::ImageFrame {
                slot: view.as_ptr() as usize,
                address: frame.raw() as usize,
                source_view: true,
            },
        );
        view.set(frame.raw());
    }

    pub(crate) fn replace_view(&self, index: usize, frame: FrameRef<'_>) {
        let array = self
            .0
            .as_ref()
            .unwrap_or_else(|| panic!("absent frame array"));
        let previous = array.views[index].get();
        assert!(
            (previous.is_null() && frame.is_absent())
                || array
                    .leases
                    .iter()
                    .any(|lease| lease.owns_frame(previous) && lease.owns_frame(frame.raw())),
            "owning frame slot cannot change drawable"
        );
        self.set_view(index, frame);
    }

    fn frame_index(&self, resolution: usize, frame: FRAME) -> usize {
        if frame.is_null() {
            return 0;
        }
        let array = self
            .0
            .as_ref()
            .unwrap_or_else(|| panic!("absent frame array"));
        let base = array.views[resolution].get();
        #[cfg(test)]
        if array._fixture.is_some() {
            assert_eq!(base, frame);
            return 0;
        }
        array
            .leases
            .iter()
            .find(|lease| lease.owns_frame(base))
            .and_then(|lease| lease.frame_index(frame))
            .unwrap_or_else(|| panic!("frame has no owning drawable lease"))
    }

    fn indexed(&self, resolution: usize, index: usize) -> FRAME {
        let base = self.view(resolution).raw();
        if base.is_null() {
            return base;
        }
        let array = self
            .0
            .as_ref()
            .unwrap_or_else(|| panic!("absent frame array"));
        #[cfg(test)]
        if array._fixture.is_some() {
            assert_eq!(index, 0);
            return base;
        }
        array
            .leases
            .iter()
            .find(|lease| lease.owns_frame(base))
            .and_then(|lease| lease.frame(index))
            .unwrap_or_else(|| panic!("frame index outside owning drawable"))
    }

    #[cfg(test)]
    pub(crate) fn fixture(views: [FRAME; 3], resource: impl std::any::Any) -> Self {
        Self::fixture_source(views, resource, None)
    }
    #[cfg(test)]
    pub(crate) fn fixture_source(
        views: [FRAME; 3],
        resource: impl std::any::Any,
        source: Option<usize>,
    ) -> Self {
        Self(Some(Rc::new(Array {
            views: views.map(Cell::new),
            leases: Vec::new(),
            _fixture: Some(Box::new(resource)),
            source,
        })))
    }

    pub(crate) fn observe(&self, role: &str) {
        if self.is_absent() {
            return;
        }
        crate::collections::observation::record(crate::collections::observation::Event::FrameSet {
            address: self.as_ptr() as usize,
            frames: self
                .views()
                .iter()
                .map(|frame| frame.raw() as usize)
                .collect(),
            role: role.into(),
        });
    }

    pub(crate) fn retire_views(&self) {
        if self.is_absent() {
            return;
        }
        crate::collections::observation::record(crate::collections::observation::Event::FrameSet {
            address: self.as_ptr() as usize,
            frames: vec![0; 3],
            role: "animation".into(),
        });
    }

    fn source_address(&self) -> usize {
        self.0
            .as_ref()
            .and_then(|array| array.source)
            .unwrap_or(self.as_ptr() as usize)
    }

    pub fn is_absent(&self) -> bool {
        self.0.is_none()
    }

    pub fn as_ptr(&self) -> *mut FRAME {
        self.0
            .as_ref()
            .map_or(std::ptr::null_mut(), |array| array.views[0].as_ptr().cast())
    }
}
impl std::fmt::Debug for FrameArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FrameArray").field(&self.as_ptr()).finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Ship,
    Weapon,
    Special,
}
impl From<usize> for Role {
    fn from(index: usize) -> Self {
        match index {
            0 => Self::Ship,
            1 => Self::Weapon,
            2 => Self::Special,
            _ => panic!("invalid frame role {index}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedFrames {
    Asteroid,
    Explosion,
    Blast,
    Stars,
    Hyperspace(u8),
    Portal,
    Flagship,
    Planet,
    GroupIcon(usize),
    Fixture(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOwner {
    Ship(ShipId, Role),
    Shared(SharedFrames),
    Drawable,
}

#[derive(Clone, Debug)]
pub struct FrameSetRef {
    pub owner: FrameOwner,
    frames: FrameArray,
}
impl FrameSetRef {
    pub fn ship(ship: ShipId, role: Role, frames: FrameArray) -> Self {
        Self {
            owner: FrameOwner::Ship(ship, role),
            frames,
        }
    }

    pub fn shared(role: SharedFrames, frames: FrameArray) -> Self {
        Self {
            owner: FrameOwner::Shared(role),
            frames,
        }
    }

    pub fn view(&self, index: usize) -> FrameRef<'_> {
        self.frames.view(index).owned_by(self.owner)
    }

    pub(crate) fn raw(&self) -> *mut FRAME {
        self.frames.source_address() as *mut FRAME
    }
}
impl PartialEq for FrameSetRef {
    fn eq(&self, other: &Self) -> bool {
        // A catalog ship can load a replacement descriptor while old projectiles retain its frames.
        self.owner == other.owner && self.frames.as_ptr() == other.frames.as_ptr()
    }
}
impl Eq for FrameSetRef {}
impl crate::collections::observation::ImagePointer for Frames {
    fn address(&self) -> usize {
        self.raw() as usize
    }
}

/// Optional image binding; a present set always owns its frame allocation lease.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frames(Option<FrameSetRef>);
impl Frames {
    pub const ABSENT: Self = Self(None);

    pub fn ship(ship: ShipId, role: Role, frames: FrameArray) -> Self {
        Self(Some(FrameSetRef::ship(ship, role, frames)))
    }

    pub fn shared(role: SharedFrames, frames: FrameArray) -> Self {
        Self(Some(FrameSetRef::shared(role, frames)))
    }

    pub fn view(&self, index: usize) -> FrameRef<'_> {
        self.0
            .as_ref()
            .map_or_else(|| FrameArray::EMPTY.view(index), |set| set.view(index))
    }

    pub fn is_absent(&self) -> bool {
        self.0.is_none()
    }

    pub(crate) fn raw(&self) -> *mut FRAME {
        self.0
            .as_ref()
            .map_or(std::ptr::null_mut(), FrameSetRef::raw)
    }

    /// # Safety
    /// The source contains one or three live native frame slots on the graphics thread.
    pub(crate) unsafe fn native_shared(
        role: SharedFrames,
        source: std::ptr::NonNull<FRAME>,
        count: usize,
    ) -> Self {
        assert!(
            count == 1 || count == 3,
            "invalid native frame array length {count}"
        );
        // SAFETY: construction retains the registered allocations behind these live source slots.
        let raw =
            unsafe { std::array::from_fn(|index| *source.as_ptr().add(index.min(count - 1))) };
        Self::shared(
            role,
            FrameArray::from_raw(raw, Some(source.as_ptr() as usize)),
        )
    }
}

/// A view cannot outlive the array retaining its drawable allocation.
#[derive(Clone, Copy, Debug)]
pub struct FrameRef<'a> {
    raw: FRAME,
    owner: &'a FrameArray,
    resolution: usize,
    identity: FrameOwner,
}
impl FrameRef<'_> {
    pub fn raw(self) -> FRAME {
        self.raw
    }
    pub fn is_absent(self) -> bool {
        self.raw.is_null()
    }
    pub(super) fn owned_by(mut self, owner: FrameOwner) -> Self {
        self.identity = owner;
        self
    }
    pub fn to_owned(self) -> FrameSelection {
        if self.is_absent() {
            return FrameSelection::ABSENT;
        }
        FrameSelection {
            set: Some(FrameSetRef {
                owner: self.identity,
                frames: self.owner.clone(),
            }),
            resolution: self.resolution,
            index: self.owner.frame_index(self.resolution, self.raw),
        }
    }
}
impl PartialEq for FrameRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl Eq for FrameRef<'_> {}

/// Stored animation state retains the frame set and an allocation-relative index.
#[derive(Clone, Debug)]
pub struct FrameSelection {
    set: Option<FrameSetRef>,
    resolution: usize,
    index: usize,
}
impl FrameSelection {
    pub const ABSENT: Self = Self {
        set: None,
        resolution: 0,
        index: 0,
    };
    pub fn view(&self) -> FrameRef<'_> {
        match &self.set {
            Some(set) => FrameRef {
                raw: set.frames.indexed(self.resolution, self.index),
                owner: &set.frames,
                resolution: self.resolution,
                identity: set.owner,
            },
            None => FrameArray::EMPTY.view(0),
        }
    }
    pub fn raw(&self) -> FRAME {
        self.view().raw()
    }
    pub fn is_absent(&self) -> bool {
        self.set.is_none()
    }
    pub(super) fn with_raw(&self, raw: FRAME) -> Self {
        if raw.is_null() {
            return Self::ABSENT;
        }
        let set = self
            .set
            .as_ref()
            .unwrap_or_else(|| panic!("absent frame selection"));
        Self {
            set: self.set.clone(),
            resolution: self.resolution,
            index: set.frames.frame_index(self.resolution, raw),
        }
    }
    /// Retain the registered allocation without dereferencing the supplied address.
    /// Unknown or expired addresses are rejected.
    pub fn retain(raw: FRAME) -> Self {
        FrameArray::from_raw([raw; 3], None).view(0).to_owned()
    }
}
impl PartialEq for FrameSelection {
    fn eq(&self, other: &Self) -> bool {
        self.raw() == other.raw()
    }
}
impl Eq for FrameSelection {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::observation::{self, Event};

    #[test]
    fn replacement_descriptor_has_distinct_frames_even_for_the_same_ship_and_role() {
        let mut kernel = KernelOwner::new();
        kernel.reset_ship_capacity(0, 1);
        let mut ships = kernel.ships(0);
        let link = ships.alloc().unwrap();
        let ship = ships.ship_id(link);
        let old = Frames::ship(
            ship,
            Role::Weapon,
            FrameArray::fixture([std::ptr::null_mut(); 3], ()),
        );
        let replacement = Frames::ship(
            ship,
            Role::Weapon,
            FrameArray::fixture([std::ptr::null_mut(); 3], ()),
        );
        assert_eq!(old, old.clone());
        assert_ne!(old, replacement);
    }

    #[test]
    #[serial_test::serial]
    fn native_sources_snapshot_views_without_borrowing_source_storage() {
        let frames = {
            let mut source = [std::ptr::null_mut(); 3];
            let address = source.as_mut_ptr();
            // SAFETY: the source contains three absent frame slots throughout construction.
            let frames = unsafe {
                Frames::native_shared(
                    SharedFrames::Blast,
                    std::ptr::NonNull::new(address).unwrap(),
                    3,
                )
            };
            assert_eq!(frames.raw(), address);
            frames
        };
        for index in 0..3 {
            assert!(frames.view(index).is_absent());
        }
    }

    #[test]
    #[serial_test::serial]
    fn absent_arrays_do_not_publish_a_frame_source() {
        observation::enable();
        observation::drain();
        FrameArray::EMPTY.observe("absent");
        FrameArray::EMPTY.retire_views();
        assert!(observation::drain().is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn array_edits_and_retirement_observe_storage_without_releasing_clones() {
        observation::enable();
        observation::drain();
        let array = FrameArray::fixture([std::ptr::null_mut(); 3], ());
        let copied = array.clone();
        array.observe("test-array");
        array.set_view(1, FrameArray::EMPTY.view(0));
        array.retire_views();
        let events = observation::drain();
        assert!(
            matches!(&events[0], Event::FrameSet { address, frames, role } if *address == array.as_ptr() as usize && frames == &[0; 3] && role == "test-array")
        );
        assert!(matches!(
            &events[1],
            Event::ImageFrame {
                address: 0,
                source_view: true,
                ..
            }
        ));
        assert!(
            matches!(&events[2], Event::FrameSet { frames, role, .. } if frames == &[0; 3] && role == "animation")
        );
        assert!(copied.views().iter().all(|view| view.is_absent()));
    }
}

#[cfg(test)]
mod replacement_tests {
    use super::*;
    #[test]
    #[serial_test::serial]
    fn replacement_rejects_a_foreign_drawable_but_accepts_a_sibling_frame() {
        use crate::graphics::drawable_abi::{AllocDrawable, DestroyDrawable};
        // SAFETY: the test owns both serialized native allocations and their frame slots.
        unsafe {
            let a = AllocDrawable(2);
            let b = AllocDrawable(1);
            (*a).MaxIndex = 1;
            let owner = FrameArray::from_native([(*a).Frame.cast(); 3]);
            let first = owner.view(0);
            let siblings = FrameArray::from_native([(*a).Frame.add(1).cast(); 3]);
            let sibling = siblings.view(0);
            let other = FrameArray::from_native([(*b).Frame.cast(); 3]);
            let foreign = other.view(0);
            let array = FrameArray::retain([first; 3]);
            assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || array.replace_view(0, foreign)
            ))
            .is_err());
            assert_eq!(array.view(0), first);
            let set = FrameSetRef::shared(SharedFrames::Blast, array.clone());
            let selected = set.view(0).to_owned();
            assert_eq!(
                selected
                    .set
                    .as_ref()
                    .unwrap_or_else(|| panic!("absent selection"))
                    .owner,
                set.owner
            );
            array.replace_view(0, sibling);
            assert_eq!(array.view(0), sibling);
            assert_eq!(selected.raw(), first.raw());
            assert_eq!(selected.with_raw(sibling.raw()).raw(), sibling.raw());
            DestroyDrawable(a);
            DestroyDrawable(b);
        }
    }
}

impl crate::collections::observation::ImagePointer for FrameSelection {
    fn address(&self) -> usize {
        self.raw() as usize
    }
}

use crate::graphics::native_abi::frame_desc;
#[cfg(not(test))]
use crate::{
    collections::observation::observe_views,
    graphics::{
        drawable_abi::DestroyDrawable,
        pixmap_abi::{CaptureDrawable, ReleaseDrawable},
        resgfx::LoadGraphicInstance,
    },
};
use std::ptr;

fn release_images<const N: usize>(
    frames: &mut [*mut frame_desc; N],
    mut destroy: impl FnMut(*mut frame_desc) -> bool,
) -> bool {
    let mut freed = [ptr::null_mut(); N];
    let mut result = true;
    for i in 0..N {
        let frame = frames[i];
        if !frame.is_null() {
            if !freed[..i].contains(&frame) && !destroy(frame) {
                result = false;
            }
            freed[i] = frame;
            frames[i] = ptr::null_mut();
        }
    }
    result
}

#[cfg(not(test))]
pub(crate) fn load_graphic(key: &std::ffi::CStr) -> FRAME {
    // SAFETY: the borrowed key remains live through decoding; a successful load owns its frames.
    unsafe { CaptureDrawable(LoadGraphicInstance(key.as_ptr()).cast()) }
}

#[cfg(not(test))]
pub fn load_animation(
    frames: &mut [*mut frame_desc; 3],
    keys: [Option<&std::ffi::CStr>; 3],
    role: &str,
) -> u32 {
    let Some(big) = keys[0] else { return 0 };
    let mut frame = load_graphic(big);
    if frame.is_null() {
        return 0;
    }
    frames[0] = frame;
    for (index, key) in keys[1..].iter().enumerate() {
        if let Some(key) = key {
            frame = load_graphic(key);
            if frame.is_null() {
                return 0;
            }
        }
        frames[index + 1] = frame;
    }
    observe_views(frames, role, |frame| *frame as usize);
    1
}

#[cfg(not(test))]
pub fn free_image<const N: usize>(frames: &mut [*mut frame_desc; N], role: &str) -> u32 {
    let _allocations = crate::graphics::drawable_abi::retain_frame_allocations(frames);
    let freed = release_images(frames, |frame| {
        // SAFETY: every non-null address was validated and its allocation remains retained through release.
        unsafe { DestroyDrawable(ReleaseDrawable(frame)) != 0 }
    });
    observe_views(frames, role, |frame| *frame as usize);
    freed as u32
}

#[cfg(test)]
mod animation_release_tests {
    use super::*;
    #[test]
    fn one_frame_and_empty_sets_use_the_same_release_contract() {
        let frame = ptr::dangling_mut::<frame_desc>();
        let mut frames = [frame];
        let mut destroyed = vec![];
        assert!(release_images(&mut frames, |frame| {
            destroyed.push(frame);
            true
        }));
        assert_eq!(destroyed, [frame]);
        assert_eq!(frames, [ptr::null_mut()]);
        let mut empty: [*mut frame_desc; 0] = [];
        assert!(release_images(&mut empty, |_| panic!("empty frame set")));
    }
    #[test]
    fn aliases_are_destroyed_once_and_all_slots_are_cleared() {
        let a = ptr::dangling_mut::<frame_desc>();
        let b = a.wrapping_add(1);
        let mut frames = [a, b, a];
        let mut destroyed = vec![];
        assert!(!release_images(&mut frames, |frame| {
            destroyed.push(frame);
            frame != a
        }));
        assert_eq!(destroyed, [a, b]);
        assert_eq!(frames, [ptr::null_mut(); 3]);
    }
    #[test]
    fn null_slots_do_not_own_resources() {
        let mut frames = [ptr::null_mut(); 3];
        assert!(release_images(&mut frames, |_| panic!(
            "null frame destroyed"
        )));
    }
}
