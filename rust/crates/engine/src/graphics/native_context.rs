//! Retained ownership of native contexts on the serialized graphics thread.
use super::{context_abi, drawable_abi, ffi, frame_abi, native_abi::*, native_drawables};
use std::{cell::RefCell, ptr::NonNull, rc::Rc, sync::Arc, thread::ThreadId};

pub(super) struct Allocation {
    context: NonNull<context_desc>,
    owner: Arc<ThreadId>,
    target: RefCell<Option<ContextFrame>>,
}

impl Drop for Allocation {
    fn drop(&mut self) {
        assert_eq!(*self.owner, std::thread::current().id());
        // SAFETY: this is the final graphics-thread owner of the registered allocation.
        unsafe { context_abi::free_context(self.context) };
    }
}

pub struct ContextFrame {
    frame: FRAME,
    _allocations: Vec<native_drawables::DrawableLease>,
}
impl ContextFrame {
    pub fn retain(frame: Option<NonNull<frame_desc>>) -> Self {
        let frame = frame.map_or(std::ptr::null_mut(), NonNull::as_ptr);
        Self {
            frame,
            _allocations: native_drawables::retain_frame_allocations(&[frame]),
        }
    }
}

enum Operation {
    Select,
    Background(Color),
    Frame(ContextFrame),
    Clip(RECT),
    Clear,
}

#[derive(Clone)]
pub struct ContextLease(Rc<Allocation>);

impl ContextLease {
    /// Retain a registered context without dereferencing the supplied address.
    pub fn retain(context: NonNull<context_desc>) -> Self {
        let allocation = ffi::native_allocations()
            .contexts
            .get(&(context.as_ptr() as usize))
            .cloned();
        Self(allocation.unwrap_or_else(|| panic!("context is unowned or retired: {context:p}")))
    }

    fn apply(&self, operation: Operation) {
        assert_eq!(*self.0.owner, std::thread::current().id());
        // SAFETY: the lease owns the selected context on this graphics thread.
        // Frame changes carry allocation leases; drawing and clipping retain the
        // current target before the native operation can dereference it.
        unsafe {
            context_abi::SetContext(self.0.context.as_ptr());
            let _target = if matches!(operation, Operation::Clip(_) | Operation::Clear) {
                Some(ContextFrame::retain(NonNull::new(
                    self.0.context.as_ref().ForeGroundFrame,
                )))
            } else {
                None
            };
            match operation {
                Operation::Select => (),
                Operation::Background(color) => {
                    context_abi::SetContextBackGroundColor(color);
                }
                Operation::Frame(target) => {
                    drawable_abi::SetContextFGFrame(target.frame);
                    *self.0.target.borrow_mut() = Some(target);
                }
                Operation::Clip(mut rect) => {
                    context_abi::SetContextClipRect(&mut rect);
                }
                Operation::Clear => frame_abi::ClearDrawable(),
            }
        }
    }

    pub fn select(&self) {
        self.apply(Operation::Select);
    }

    pub fn set_background(&self, color: Color) {
        self.apply(Operation::Background(color));
    }

    /// Retain the registered target allocation before changing the context's frame.
    pub fn set_frame(&self, target: ContextFrame) {
        self.apply(Operation::Frame(target));
    }

    pub fn set_clip(&self, rect: RECT) {
        self.apply(Operation::Clip(rect));
    }

    pub fn clear(&self) {
        self.apply(Operation::Clear);
    }
}

pub(super) fn register(context: NonNull<context_desc>) {
    let mut allocations = ffi::native_allocations();
    let owner = allocations
        .owner
        .clone()
        .unwrap_or_else(|| panic!("context registry has no owner"));
    let allocation = Rc::new(Allocation {
        context,
        owner,
        target: RefCell::new(None),
    });
    assert!(
        allocations
            .contexts
            .insert(context.as_ptr() as usize, allocation)
            .is_none(),
        "context already registered"
    );
}

pub(super) fn release(context: NonNull<context_desc>) {
    let allocation = ffi::native_allocations()
        .contexts
        .remove(&(context.as_ptr() as usize))
        .unwrap_or_else(|| panic!("release of an unowned context: {context:p}"));
    drop(allocation);
}
