//! Shared allocation leases for native drawable handles.
use super::native_abi::{context_desc, drawable_desc, font_desc, FRAME};
use std::{
    collections::BTreeMap,
    ptr::NonNull,
    rc::{Rc, Weak},
    sync::Arc,
    thread::ThreadId,
};

pub(super) struct Allocations {
    pub owner: Option<Arc<ThreadId>>,
    entries: BTreeMap<usize, Rc<Allocation>>,
    frames: BTreeMap<usize, Weak<Allocation>>,
    pub(super) contexts: BTreeMap<usize, Rc<super::native_context::Allocation>>,
    pub(super) fonts: BTreeMap<usize, Rc<super::native_font::Allocation>>,
}
impl Allocations {
    pub const fn new() -> Self {
        Self {
            owner: None,
            entries: BTreeMap::new(),
            frames: BTreeMap::new(),
            contexts: BTreeMap::new(),
            fonts: BTreeMap::new(),
        }
    }
    pub fn assert_thread(&mut self) {
        let current = std::thread::current().id();
        // Allocations keep the owner pinned even after logical release removes their map entry.
        match &self.owner {
            Some(owner) if Arc::strong_count(owner) > 1 => assert_eq!(
                **owner, current,
                "drawable allocation registry accessed from a non-owner thread"
            ),
            _ => self.owner = Some(Arc::new(current)),
        }
    }
    fn insert(&mut self, drawable: NonNull<drawable_desc>, frames: Vec<FRAME>) {
        let owner = self
            .owner
            .clone()
            .unwrap_or_else(|| panic!("drawable registry has no owning thread"));
        let allocation = Rc::new(Allocation {
            _owned: OwnedAllocation::new(AllocationKind::Drawable(drawable), owner),
            frames,
        });
        self.frames
            .retain(|_, allocation| allocation.strong_count() != 0);
        for address in &allocation.frames {
            self.frames
                .insert(*address as usize, Rc::downgrade(&allocation));
        }
        assert!(
            self.entries
                .insert(drawable.as_ptr() as usize, allocation)
                .is_none(),
            "drawable allocation is already registered"
        );
    }
}

pub(super) enum AllocationKind {
    Drawable(NonNull<drawable_desc>),
    Context(NonNull<context_desc>),
    Font(NonNull<font_desc>),
}

pub(super) struct OwnedAllocation {
    kind: AllocationKind,
    owner: Arc<ThreadId>,
}
impl OwnedAllocation {
    pub(super) fn new(kind: AllocationKind, owner: Arc<ThreadId>) -> Self {
        Self { kind, owner }
    }
    pub(super) fn assert_thread(&self) {
        assert_eq!(*self.owner, std::thread::current().id());
    }
}
impl Drop for OwnedAllocation {
    fn drop(&mut self) {
        self.assert_thread();
        // SAFETY: this is the final graphics-thread owner of the registered allocation.
        unsafe {
            match self.kind {
                AllocationKind::Drawable(drawable) => {
                    super::gfxload::release_cel_data_now(drawable)
                }
                AllocationKind::Font(font) => super::gfxload::release_font_data_now(font),
                AllocationKind::Context(context) => {
                    if super::context_abi::_pCurContext == context.as_ptr() {
                        super::context_abi::SetContext(std::ptr::null_mut());
                    }
                    super::context_abi::free_context(context);
                }
            }
        }
    }
}

pub(super) struct Allocation {
    _owned: OwnedAllocation,
    frames: Vec<FRAME>,
}

#[derive(Clone)]
pub struct DrawableLease {
    pub(super) _allocation: Rc<Allocation>,
}

pub(super) unsafe fn register(drawable: NonNull<drawable_desc>, count: u16) {
    // SAFETY: AllocDrawable has initialized this descriptor and every one of its frame slots.
    let frames = unsafe { drawable.as_ref().Frame };
    let addresses = (0..count)
        .map(|i| frames.wrapping_add(usize::from(i)).cast())
        .collect();
    super::ffi::native_allocations().insert(drawable, addresses);
}

pub(super) fn release(drawable: NonNull<drawable_desc>) {
    let allocation = super::ffi::native_allocations()
        .entries
        .remove(&(drawable.as_ptr() as usize))
        .unwrap_or_else(|| panic!("release of unowned drawable {drawable:p}"));
    drop(allocation);
}

/// Retain registered frame allocations without dereferencing caller-provided addresses.
/// Unknown or expired frames are rejected on the serialized graphics thread.
pub fn retain_frame_allocations(frames: &[FRAME]) -> Vec<DrawableLease> {
    let allocations = super::ffi::native_allocations();
    let retained = frames
        .iter()
        .filter(|frame| !frame.is_null())
        .map(|frame| {
            allocations
                .frames
                .get(&(*frame as usize))
                .and_then(Weak::upgrade)
                .map(|allocation| DrawableLease {
                    _allocation: allocation,
                })
                .ok_or(*frame as usize)
        })
        .collect::<Result<Vec<_>, _>>();
    drop(allocations);
    retained.unwrap_or_else(|frame| panic!("frame belongs to an unowned drawable: {frame:#x}"))
}
impl DrawableLease {
    pub fn owns_frame(&self, frame: FRAME) -> bool {
        self.frame_at_address(frame).is_some()
    }
    pub fn frame_index(&self, frame: FRAME) -> Option<usize> {
        self._allocation
            .frames
            .iter()
            .position(|owned| *owned == frame)
    }
    pub fn frame(&self, index: usize) -> Option<FRAME> {
        self._allocation.frames.get(index).copied()
    }
    pub fn frame_at_address(&self, frame: FRAME) -> Option<FRAME> {
        self._allocation
            .frames
            .iter()
            .copied()
            .find(|owned| *owned == frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn registry_release_validates_addresses_and_keeps_retained_frames_live() {
        // SAFETY: this serialized fixture exclusively owns the newly allocated drawable.
        let drawable = unsafe { super::super::drawable_abi::AllocDrawable(1) };
        // SAFETY: drawable was just allocated and remains owned by this fixture.
        let frame = unsafe { super::super::pixmap_abi::CaptureDrawable(drawable) };
        let retained = retain_frame_allocations(&[frame]);
        let drawable = NonNull::new(drawable).unwrap();
        release(drawable);
        assert_eq!(retained[0].frame(0), Some(frame));
        assert!(std::panic::catch_unwind(|| release(drawable)).is_err());
        assert!(std::panic::catch_unwind(|| release(NonNull::dangling())).is_err());
        drop(retained);
        assert!(std::panic::catch_unwind(|| retain_frame_allocations(&[frame])).is_err());
    }
}
