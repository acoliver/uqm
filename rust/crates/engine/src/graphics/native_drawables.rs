//! Shared allocation leases for native drawable handles.
use super::native_abi::{drawable_desc, FRAME};
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
}
impl Allocations {
    pub const fn new() -> Self {
        Self {
            owner: None,
            entries: BTreeMap::new(),
            frames: BTreeMap::new(),
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
        let allocation = Rc::new(Allocation(drawable, owner, frames));
        self.frames
            .retain(|_, allocation| allocation.strong_count() != 0);
        for address in &allocation.2 {
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

pub(super) struct Allocation(NonNull<drawable_desc>, Arc<ThreadId>, Vec<FRAME>);
impl Drop for Allocation {
    fn drop(&mut self) {
        assert_eq!(*self.1, std::thread::current().id());
        // SAFETY: this is the last graphics-thread owner of the registered allocation.
        unsafe {
            super::gfxload::release_cel_data_now(self.0);
        }
    }
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

pub(super) unsafe fn release(drawable: NonNull<drawable_desc>) {
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
    pub fn frame_at_address(&self, frame: FRAME) -> Option<FRAME> {
        self._allocation
            .2
            .iter()
            .copied()
            .find(|owned| *owned == frame)
    }
}
