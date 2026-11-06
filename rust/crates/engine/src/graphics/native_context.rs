//! Retained ownership of native contexts on the serialized graphics thread.
use super::{context_abi, drawable_abi, ffi, frame_abi, native_abi::*, native_drawables};
use std::{cell::RefCell, ptr::NonNull, rc::Rc};

pub(super) struct Allocation {
    context: NonNull<context_desc>,
    owned: native_drawables::OwnedAllocation,
    target: RefCell<Option<ContextFrame>>,
    font: RefCell<Option<super::native_font::FontLease>>,
}

#[derive(Clone)]
pub struct ContextFrame {
    pub(super) frame: FRAME,
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

    pub fn rect(&self) -> RECT {
        assert!(!self.frame.is_null(), "absent frame has no bounds");
        for allocation in &self._allocations {
            allocation.assert_thread();
        }
        let mut rect = std::mem::MaybeUninit::uninit();
        // SAFETY: retained allocations keep this non-null frame live on the owning thread.
        unsafe {
            drawable_abi::GetFrameRect(self.frame, rect.as_mut_ptr());
            rect.assume_init()
        }
    }

    pub fn replace_hotspot(&self, hotspot: HOT_SPOT) -> HOT_SPOT {
        for allocation in &self._allocations {
            allocation.assert_thread();
        }
        // SAFETY: retained allocations keep this optional frame live on the owning thread.
        unsafe { drawable_abi::SetFrameHot(self.frame, hotspot) }
    }
}

enum Operation {
    Select,
    Deselect,
    BeginBatch,
    EndBatch,
    Background(Color),
    Frame(FRAME),
    Clip(Option<RECT>),
    Clear,
    Fill(Color, RECT),
    Outline(Color, RECT),
    FilledStamp(Color, STAMP, bool),
    Stamp(STAMP),
    Font(super::native_font::FontLease),
    Text(super::native_font::FontLease, TEXT, Color),
}

#[derive(Clone)]
pub struct ContextLease(Rc<Allocation>);

pub struct BatchPause(ContextLease);
impl Drop for BatchPause {
    fn drop(&mut self) {
        self.0.apply(Operation::BeginBatch);
    }
}

struct ContextBatch {
    selection: ContextSelection,
}
impl Drop for ContextBatch {
    fn drop(&mut self) {
        self.selection.context.apply(Operation::EndBatch);
    }
}

struct ContextSelection {
    context: ContextLease,
    previous: Option<ContextLease>,
}
impl Drop for ContextSelection {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            previous.select();
        } else {
            self.context.apply(Operation::Deselect);
        }
    }
}

struct ContextClip {
    context: ContextLease,
    previous: Option<RECT>,
}
impl Drop for ContextClip {
    fn drop(&mut self) {
        self.context.apply(Operation::Clip(self.previous));
    }
}
struct ContextOrigin {
    frame: ContextFrame,
    previous: POINT,
}
impl Drop for ContextOrigin {
    fn drop(&mut self) {
        self.frame.replace_hotspot(self.previous);
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

    pub(super) fn assert_thread(&self) {
        self.0.owned.assert_thread();
    }

    pub fn highlight(&self, rect: RECT) -> super::native_highlight::Highlight {
        self.assert_thread();
        super::native_highlight::Highlight::new(self.clone(), self.0.context, rect)
    }

    fn apply(&self, operation: Operation) -> (FRAME, CONTEXT) {
        self.assert_thread();
        let selected = if matches!(&operation, Operation::Deselect) {
            std::ptr::null_mut()
        } else {
            self.0.context.as_ptr()
        };
        // SAFETY: the lease owns the selected context on this graphics thread.
        // Set-frame retains its new allocation; clipping and drawing retain the
        // selected target before entering this native operation boundary.
        unsafe {
            let previous = match operation {
                Operation::BeginBatch | Operation::EndBatch => context_abi::_pCurContext,
                _ => context_abi::SetContext(selected),
            };
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
                    context_abi::SetContextClipRect(
                        rect.as_mut().map_or(std::ptr::null_mut(), |rect| rect),
                    );
                }
                Operation::Clear => frame_abi::ClearDrawable(),
                Operation::Fill(color, mut rect) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawFilledRectangle(&mut rect);
                }
                Operation::Outline(color, mut rect) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawRectangle(&mut rect);
                }
                Operation::FilledStamp(color, mut stamp, restore) => {
                    let previous = context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawFilledStamp(&mut stamp);
                    if restore {
                        context_abi::SetContextForeGroundColor(previous);
                    }
                }
                Operation::Stamp(mut stamp) => frame_abi::DrawStamp(&mut stamp),
                Operation::Font(font) => {
                    super::font_abi::SetContextFont(font.as_raw());
                }
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

    fn selection(&self) -> ContextSelection {
        let previous = NonNull::new(self.apply(Operation::Select).1).map(Self::retain);
        ContextSelection {
            context: self.clone(),
            previous,
        }
    }

    pub fn with_selected<R>(&self, draw: impl FnOnce() -> R) -> R {
        let _selection = self.selection();
        draw()
    }

    pub fn draw_batch<R>(&self, draw: impl FnOnce() -> R) -> R {
        let batch = ContextBatch {
            selection: self.selection(),
        };
        self.apply(Operation::BeginBatch);
        let result = draw();
        drop(batch);
        result
    }

    pub fn pause_batch(&self) -> BatchPause {
        self.apply(Operation::EndBatch);
        BatchPause(self.clone())
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
        self.apply(Operation::Clip(Some(rect)));
    }

    fn clip_snapshot(&self) -> (RECT, bool) {
        let _target = self.retain_target();
        let mut rect = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: 0,
                height: 0,
            },
        };
        // SAFETY: retain_target selects this retained context and keeps its frame live.
        let explicit = unsafe { context_abi::GetContextClipRect(&mut rect) } != 0;
        (rect, explicit)
    }

    pub fn clip_rect(&self) -> RECT {
        self.clip_snapshot().0
    }

    pub fn clear_clip(&self) {
        let _target = self.retain_target();
        self.apply(Operation::Clip(None));
    }

    pub fn with_clip<R>(&self, clip: RECT, draw: impl FnOnce() -> R) -> R {
        let (previous, explicit) = self.clip_snapshot();
        let restore = ContextClip {
            context: self.clone(),
            previous: explicit.then_some(previous),
        };
        self.set_clip(clip);
        let result = draw();
        drop(restore);
        result
    }

    pub fn with_origin<R>(&self, origin: POINT, draw: impl FnOnce() -> R) -> R {
        let frame = self.retain_target();
        let previous = frame.replace_hotspot(origin);
        let restore = ContextOrigin { frame, previous };
        let result = draw();
        drop(restore);
        result
    }

    pub fn draw_rectangle(&self, color: Color, rect: RECT) {
        let _target = self.retain_target();
        self.apply(Operation::Outline(color, rect));
    }

    pub fn draw_filled_stamp(&self, source: &ContextFrame, origin: POINT, color: Color) {
        self.filled_stamp(source, origin, color, false);
    }

    pub fn draw_filled_stamp_preserving_color(
        &self,
        source: &ContextFrame,
        origin: POINT,
        color: Color,
    ) {
        self.filled_stamp(source, origin, color, true);
    }

    fn filled_stamp(&self, source: &ContextFrame, origin: POINT, color: Color, restore: bool) {
        let _target = self.retain_target();
        self.apply(Operation::FilledStamp(
            color,
            STAMP {
                origin,
                frame: source.frame,
            },
            restore,
        ));
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

    pub fn set_font(&self, font: &super::native_font::FontLease) {
        let previous = self.0.font.replace(Some(font.clone()));
        self.apply(Operation::Font(font.clone()));
        drop(previous);
    }

    pub fn draw_text(
        &self,
        font: &super::native_font::FontLease,
        string: &std::ffi::CStr,
        baseline: POINT,
        align: TEXT_ALIGN,
        color: Color,
    ) {
        self.draw_text_bounded(font, string, baseline, align, color, u16::MAX);
    }

    pub fn draw_text_bounded(
        &self,
        font: &super::native_font::FontLease,
        string: &std::ffi::CStr,
        baseline: POINT,
        align: TEXT_ALIGN,
        color: Color,
        characters: u16,
    ) {
        let _target = self.retain_target();
        *self.0.font.borrow_mut() = Some(font.clone());
        let text = TEXT {
            baseline,
            pStr: string.as_ptr().cast(),
            align,
            CharCount: characters,
        };
        self.apply(Operation::Text(font.clone(), text, color));
    }

    pub fn draw_primitives(&self, batch: &mut super::native_batch::NativeBatch, scale: i32) {
        let _target = self.retain_target();
        let head = if batch.is_empty() { u16::MAX } else { 0 };
        // SAFETY: this lease retains the graphics-thread context/target. The
        // batch owns bounded links and retains all registered stamp sources;
        // it cannot contain text pointers into caller storage.
        unsafe {
            super::gfx_common_abi::SetGraphicScale(scale);
            super::frame_abi::DrawBatch(batch.primitives.as_mut_ptr(), u32::from(head), 0);
            super::gfx_common_abi::SetGraphicScale(0);
        }
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
