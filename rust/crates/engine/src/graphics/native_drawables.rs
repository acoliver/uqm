//! Shared allocation leases for native drawable handles.
use super::native_abi::{drawable_desc, FRAME};
use std::{collections::BTreeMap, ptr::NonNull, rc::Rc};

pub(super) type Allocations = BTreeMap<usize, Rc<Allocation>>;

pub(super) struct Allocation(NonNull<drawable_desc>);
impl Drop for Allocation {
    fn drop(&mut self) {
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
    let mut allocations = super::ffi::native_allocations();
    assert!(
        allocations
            .insert(drawable.as_ptr() as usize, Rc::new(Allocation(drawable)))
            .is_none(),
        "drawable allocation is already registered"
    );
}

pub(super) unsafe fn release(drawable: NonNull<drawable_desc>) {
    let allocation = super::ffi::native_allocations()
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
                    .get(&(drawable as usize))
                    .unwrap_or_else(|| panic!("frame belongs to an unowned drawable {drawable:p}"))
                    .clone(),
            }
        })
        .collect()
}
