//! Shared allocation leases for native drawable handles.
use super::native_abi::{drawable_desc, FRAME};
use std::{collections::BTreeMap, ptr::NonNull, rc::Rc, sync::Arc, thread::ThreadId};

pub(super) struct Allocations {
    pub owner: Option<Arc<ThreadId>>,
    entries: BTreeMap<usize, Rc<Allocation>>,
}
impl Allocations {
    pub const fn new() -> Self {
        Self {
            owner: None,
            entries: BTreeMap::new(),
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
    fn insert(&mut self, drawable: NonNull<drawable_desc>) {
        let owner = self
            .owner
            .clone()
            .unwrap_or_else(|| panic!("drawable registry has no owning thread"));
        let allocation = Rc::new(Allocation(drawable, owner));
        assert!(
            self.entries
                .insert(drawable.as_ptr() as usize, allocation)
                .is_none(),
            "drawable allocation is already registered"
        );
    }
}

pub(super) struct Allocation(NonNull<drawable_desc>, Arc<ThreadId>);
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

pub(super) unsafe fn register(drawable: NonNull<drawable_desc>) {
    super::ffi::native_allocations().insert(drawable);
}

pub(super) unsafe fn release(drawable: NonNull<drawable_desc>) {
    let allocation = super::ffi::native_allocations()
        .entries
        .remove(&(drawable.as_ptr() as usize))
        .unwrap_or_else(|| panic!("release of unowned drawable {drawable:p}"));
    drop(allocation);
}

/// # Safety
/// Non-null frames must name live native drawables. Call on the serialized graphics thread.
pub unsafe fn retain_frame_allocations(frames: &[FRAME]) -> Vec<DrawableLease> {
    let allocations = super::ffi::native_allocations();
    frames
        .iter()
        .filter(|frame| !frame.is_null())
        .map(|frame| {
            // SAFETY: the caller retains each non-null native frame during this lookup.
            let drawable = unsafe { (**frame).parent };
            DrawableLease {
                _allocation: allocations
                    .entries
                    .get(&(drawable as usize))
                    .unwrap_or_else(|| panic!("frame belongs to an unowned drawable {drawable:p}"))
                    .clone(),
            }
        })
        .collect()
}
