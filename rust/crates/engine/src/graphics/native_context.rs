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

    pub fn select(&self) {
        assert_eq!(*self.0.owner, std::thread::current().id());
        // SAFETY: the lease retains this context, and selection stays on its graphics thread.
        unsafe { context_abi::SetContext(self.0.context.as_ptr()) };
    }

    pub fn set_background(&self, color: Color) {
        self.select();
        // SAFETY: selection and the retained context belong to this graphics thread.
        unsafe { context_abi::SetContextBackGroundColor(color) };
    }

    /// Retain the registered target allocation before changing the context's frame.
    pub fn set_frame(&self, target: ContextFrame) {
        self.select();
        // SAFETY: non-null frame addresses were validated against the retained allocation.
        unsafe { drawable_abi::SetContextFGFrame(target.frame) };
        *self.0.target.borrow_mut() = Some(target);
    }

    fn retain_target(&self) -> ContextFrame {
        // SAFETY: this reads a field of the retained context, not the referenced frame.
        let frame = unsafe { self.0.context.as_ref().ForeGroundFrame };
        ContextFrame::retain(NonNull::new(frame))
    }

    pub fn set_clip(&self, mut rect: RECT) {
        self.select();
        let _target = self.retain_target();
        // SAFETY: the context and any non-null target remain retained through clipping.
        unsafe { context_abi::SetContextClipRect(&mut rect) };
    }

    pub fn clear(&self) {
        self.select();
        let _target = self.retain_target();
        // SAFETY: the context and any non-null target remain retained through submission.
        unsafe { frame_abi::ClearDrawable() };
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
