//! Retained ownership of native contexts on the serialized graphics thread.
use super::{context_abi, drawable_abi, ffi, frame_abi, native_abi::*, native_drawables};
use std::{cell::RefCell, ptr::NonNull, rc::Rc};

pub(super) struct Allocation {
    context: NonNull<context_desc>,
    owned: native_drawables::OwnedAllocation,
    target: RefCell<Option<ContextFrame>>,
    font: RefCell<Option<super::native_font::FontLease>>,
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
    Deselect,
    BeginBatch,
    EndBatch,
    Background(Color),
    Frame(FRAME),
    Clip(RECT),
    Clear,
    Fill(Color, RECT),
    Stamp(STAMP),
    Text(super::native_font::FontLease, TEXT, Color),
}

#[derive(Clone)]
pub struct ContextLease(Rc<Allocation>);

struct ContextBatch {
    context: ContextLease,
    previous: Option<ContextLease>,
}
impl Drop for ContextBatch {
    fn drop(&mut self) {
        self.context.apply(Operation::EndBatch);
        if let Some(previous) = &self.previous {
            previous.select();
        } else {
            self.context.apply(Operation::Deselect);
        }
    }
}

impl ContextLease {
    /// Retain a registered context without dereferencing the supplied address.
    pub fn retain(context: NonNull<context_desc>) -> Self {
        let allocation = ffi::native_allocations()
            .contexts
            .get(&(context.as_ptr() as usize))
            .cloned();
        Self(allocation.unwrap_or_else(|| panic!("context is unowned or retired: {context:p}")))
    }

    fn apply(&self, operation: Operation) -> (FRAME, CONTEXT) {
        self.0.owned.assert_thread();
        let selected = if matches!(&operation, Operation::Deselect) {
            std::ptr::null_mut()
        } else {
            self.0.context.as_ptr()
        };
        // SAFETY: the lease owns the selected context on this graphics thread.
        // Set-frame retains its new allocation; clipping and drawing retain the
        // selected target before entering this native operation boundary.
        unsafe {
            let previous = context_abi::SetContext(selected);
            match operation {
                Operation::Select | Operation::Deselect => (),
                Operation::BeginBatch => super::gfx_common_abi::batch_graphics(),
                Operation::EndBatch => super::gfx_common_abi::unbatch_graphics(),
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
                Operation::Fill(color, mut rect) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawFilledRectangle(&mut rect);
                }
                Operation::Stamp(mut stamp) => frame_abi::DrawStamp(&mut stamp),
                Operation::Text(font, mut text, color) => {
                    super::font_abi::SetContextFont(font.as_raw());
                    context_abi::SetContextForeGroundColor(color);
                    super::font_abi::font_DrawText(&mut text);
                }
            }
            (self.0.context.as_ref().ForeGroundFrame, previous)
        }
    }

    fn retain_target(&self) -> ContextFrame {
        ContextFrame::retain(NonNull::new(self.apply(Operation::Select).0))
    }

    pub fn draw_batch<R>(&self, draw: impl FnOnce() -> R) -> R {
        let previous = NonNull::new(self.apply(Operation::Select).1).map(Self::retain);
        let batch = ContextBatch {
            context: self.clone(),
            previous,
        };
        self.apply(Operation::BeginBatch);
        let result = draw();
        drop(batch);
        result
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

    pub fn fill_rectangle(&self, color: Color, rect: RECT) {
        let _target = self.retain_target();
        self.apply(Operation::Fill(color, rect));
    }

    pub fn draw_stamp(&self, source: &ContextFrame, origin: POINT) {
        let _target = self.retain_target();
        self.apply(Operation::Stamp(STAMP {
            origin,
            frame: source.frame,
        }));
    }

    pub fn draw_text(
        &self,
        font: &super::native_font::FontLease,
        string: &std::ffi::CStr,
        baseline: POINT,
        align: TEXT_ALIGN,
        color: Color,
    ) {
        assert!(string.to_str().is_ok(), "font text must be UTF-8");
        let _target = self.retain_target();
        *self.0.font.borrow_mut() = Some(font.clone());
        let text = TEXT {
            baseline,
            pStr: string.as_ptr().cast(),
            align,
            CharCount: u16::MAX,
        };
        self.apply(Operation::Text(font.clone(), text, color));
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
        font: RefCell::new(None),
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
