//! Retained ownership of native contexts on the serialized graphics thread.
use super::{context_abi, drawable_abi, ffi, frame_abi, native_abi::*, native_drawables};
use std::{cell::RefCell, ptr::NonNull, rc::Rc};

pub(super) struct Allocation {
    context: NonNull<context_desc>,
    owned: native_drawables::OwnedAllocation,
    target: RefCell<Option<ContextFrame>>,
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
    Frame(FRAME),
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

    fn apply(&self, operation: Operation) -> FRAME {
        self.0.owned.assert_thread();
        // SAFETY: the lease owns the selected context on this graphics thread.
        // Set-frame retains its new allocation; clipping and drawing retain the
        // selected target before entering this native operation boundary.
        unsafe {
            context_abi::SetContext(self.0.context.as_ptr());
            match operation {
                Operation::Select => (),
                Operation::Background(color) => {
                    context_abi::SetContextBackGroundColor(color);
                }
                Operation::Frame(frame) => {
                    drawable_abi::SetContextFGFrame(frame);
                }
                Operation::Clip(mut rect) => {
                    context_abi::SetContextClipRect(&mut rect);
                }
                Operation::Clear => frame_abi::ClearDrawable(),
            }
            self.0.context.as_ref().ForeGroundFrame
        }
    }

    fn retain_target(&self) -> ContextFrame {
        ContextFrame::retain(NonNull::new(self.apply(Operation::Select)))
    }

    pub fn select(&self) {
        self.apply(Operation::Select);
    }

    pub fn set_background(&self, color: Color) {
        self.apply(Operation::Background(color));
    }

    /// Retain the registered target allocation before changing the context's frame.
    pub fn set_frame(&self, target: ContextFrame) {
        self.apply(Operation::Frame(target.frame));
        *self.0.target.borrow_mut() = Some(target);
    }

    pub fn set_clip(&self, rect: RECT) {
        let _target = self.retain_target();
        self.apply(Operation::Clip(rect));
    }

    pub fn clear(&self) {
        let _target = self.retain_target();
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
        owned: native_drawables::OwnedAllocation::new(
            native_drawables::AllocationKind::Context(context),
            owner,
        ),
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
