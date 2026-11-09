//! Retained ownership of native contexts on the serialized graphics thread.
use super::{
    context_abi, drawable_abi, ffi, frame_abi, gfx_common_abi as gfx, native_abi::*,
    native_drawables,
};
use std::{cell::RefCell, ptr::NonNull, rc::Rc};

pub(super) struct Allocation {
    context: NonNull<context_desc>,
    owned: native_drawables::OwnedAllocation,
    target: RefCell<Option<ContextFrame>>,
    font: RefCell<Option<super::native_font::FontLease>>,
    effect: RefCell<Option<ContextFrame>>,
}

#[derive(Clone)]
pub struct ContextFrame {
    pub(super) frame: FRAME,
    _allocations: Vec<native_drawables::DrawableLease>,
}
impl ContextFrame {
    pub fn as_raw(&self) -> FRAME {
        for allocation in &self._allocations {
            allocation.assert_thread();
        }
        self.frame
    }

    pub fn retain(frame: Option<NonNull<frame_desc>>) -> Self {
        Self {
            frame: frame.map_or(std::ptr::null_mut(), NonNull::as_ptr),
            _allocations: frame.map_or_else(Vec::new, |frame| {
                native_drawables::retain_frame_allocations(&[frame.as_ptr()])
            }),
        }
    }

    pub fn next_frame(&self) -> Self {
        for allocation in &self._allocations {
            allocation.assert_thread();
        }
        // SAFETY: these leases retain this optional frame and its complete drawable allocation.
        let frame = unsafe { super::pixmap_abi::IncFrameIndex(self.frame) };
        Self {
            frame,
            _allocations: self._allocations.clone(),
        }
    }

    pub fn wrapped_frame(&self, index: u16) -> Self {
        let allocation = self
            ._allocations
            .first()
            .unwrap_or_else(|| panic!("absent frame has no allocation"));
        Self {
            frame: allocation.wrapped_frame(usize::from(index)),
            _allocations: self._allocations.clone(),
        }
    }

    pub fn indexed_frame(&self, index: usize) -> Self {
        let allocation = self
            ._allocations
            .first()
            .unwrap_or_else(|| panic!("absent frame has no allocation"));
        allocation.assert_thread();
        let frame = allocation
            .frame(index)
            .unwrap_or_else(|| panic!("drawable frame index {index}"));
        Self::retain(NonNull::new(frame))
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

    /// Read a frame-local pixel without applying its hotspot. Flush pending drawing first.
    pub fn pixel(&self, point: POINT) -> Color {
        let extent = self.rect().extent;
        assert!(
            point.x >= 0 && point.y >= 0 && point.x < extent.width && point.y < extent.height,
            "frame pixel is outside its extent"
        );
        // SAFETY: the retained frame is live on its graphics thread and the pixel is in bounds.
        unsafe { drawable_abi::GetFramePixel(self.frame, point) }
    }

    /// Read the normal canvas alpha mask, ignoring its hotspot and draw mode.
    pub fn alpha_occupancy(&self) -> Option<Vec<u8>> {
        for allocation in &self._allocations {
            allocation.assert_thread();
        }
        // SAFETY: registry leases retain this frame, image and canvas on their owning thread.
        unsafe {
            let descriptor = self.frame.as_ref()?;
            if descriptor.image.is_null()
                || descriptor.Bounds.width <= 0
                || descriptor.Bounds.height <= 0
            {
                return None;
            }
            let canvas = (*descriptor.image).NormalImg;
            if canvas.is_null() {
                return None;
            }
            super::native_canvas::lifecycle::TFB_DrawCanvas_Lock(canvas);
            let mut occupancy = Vec::with_capacity(
                descriptor.Bounds.width as usize * descriptor.Bounds.height as usize,
            );
            for y in 0..i32::from(descriptor.Bounds.height) {
                for x in 0..i32::from(descriptor.Bounds.width) {
                    occupancy.push(u8::from(
                        super::native_canvas::pixels::TFB_DrawCanvas_GetPixel(canvas, x, y).a != 0,
                    ));
                }
            }
            super::native_canvas::lifecycle::TFB_DrawCanvas_Unlock(canvas);
            Some(occupancy)
        }
    }

    pub fn frame_count(&self) -> u16 {
        for allocation in &self._allocations {
            allocation.assert_thread();
        }
        // SAFETY: the retained frame belongs to a complete registered drawable.
        unsafe { super::pixmap_abi::GetFrameCount(self.frame) }
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
    Point(Color, POINT),
    Select,
    Deselect,
    BeginBatch,
    EndBatch,
    Flush,
    Fade(u32, i16),
    FinishTransforms,
    TransitionSource(Option<RECT>),
    Transition(Option<RECT>),
    ScaleMode(i32),
    Background(Color),
    Foreground(Color),
    Frame(FRAME, unsafe extern "C" fn(FRAME) -> FRAME),
    Clip(Option<RECT>),
    Clear,
    Fill(Color, RECT),
    Outline(Color, RECT),
    ShadowedBox(RECT, Color, Color, Color),
    FilledStamp(Color, STAMP, bool),
    Stamp(STAMP),
    Font(Option<super::native_font::FontLease>),
    Text(super::native_font::FontLease, TEXT, Color),
}

impl Operation {
    fn selected_context(&self, context: CONTEXT) -> CONTEXT {
        if matches!(self, Self::Deselect) {
            std::ptr::null_mut()
        } else {
            context
        }
    }

    fn changes_selection(&self) -> bool {
        !matches!(
            self,
            Self::BeginBatch
                | Self::EndBatch
                | Self::Fade(..)
                | Self::FinishTransforms
                | Self::TransitionSource(..)
                | Self::Transition(..)
                | Self::ScaleMode(..)
        )
    }

    fn rect(&self) -> Option<RECT> {
        match self {
            Self::Clip(rect) | Self::TransitionSource(rect) | Self::Transition(rect) => *rect,
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ContextLease(Rc<Allocation>);

/// Unique release authority for a context retained by drawing leases.
pub struct LoadedContext {
    context: ContextLease,
}

impl LoadedContext {
    pub fn create(name: &'static std::ffi::CStr) -> Self {
        #[cfg(feature = "debug-process")]
        let context = {
            // SAFETY: the registry admits this graphics thread; the name remains live.
            unsafe { context_abi::CreateContextAux(name.as_ptr()) }
        };
        #[cfg(not(feature = "debug-process"))]
        let context = {
            let _ = name;
            // SAFETY: the registry admits this serialized graphics thread.
            unsafe { context_abi::CreateContextAux() }
        };
        Self {
            context: ContextLease::require(context),
        }
    }

    pub fn lease(&self) -> ContextLease {
        self.context.clone()
    }
}

impl Drop for LoadedContext {
    fn drop(&mut self) {
        self.context.assert_thread();
        if self
            .context
            .selected_context()
            .is_some_and(|selected| Rc::ptr_eq(&selected.0, &self.context.0))
        {
            self.context.apply(Operation::Deselect);
        }
        release(self.context.0.context);
    }
}

pub struct BatchPause(ContextLease);
impl Drop for BatchPause {
    fn drop(&mut self) {
        self.0.apply(Operation::BeginBatch);
    }
}

pub struct BatchGuard(ContextLease);
impl Drop for BatchGuard {
    fn drop(&mut self) {
        self.0.apply(Operation::EndBatch);
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

struct ContextEffect {
    context: ContextLease,
    previous: ContextFrame,
}
impl Drop for ContextEffect {
    fn drop(&mut self) {
        self.context.set_font_effect(self.previous.clone());
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

struct ContextTextState {
    context: ContextLease,
    font: Option<super::native_font::FontLease>,
    color: Color,
}
impl Drop for ContextTextState {
    fn drop(&mut self) {
        self.context.replace_font(self.font.take());
        self.context.apply(Operation::Foreground(self.color));
    }
}

impl ContextLease {
    pub fn as_raw(&self) -> CONTEXT {
        self.assert_thread();
        self.0.context.as_ptr()
    }

    pub fn require(context: CONTEXT) -> Self {
        Self::retain(NonNull::new(context).unwrap_or_else(|| panic!("required context is absent")))
    }

    pub fn fade_screen(&self, kind: u32, interval: i16) -> u32 {
        self.apply(Operation::Fade(kind, interval)).2
    }

    pub fn finish_color_transforms(&self) {
        self.apply(Operation::FinishTransforms);
    }

    pub fn transition_source(&self, rect: Option<RECT>) {
        self.apply(Operation::TransitionSource(rect));
    }

    pub fn transition(&self, rect: Option<RECT>) {
        self.apply(Operation::Transition(rect));
    }

    pub fn scale_mode(&self, mode: i32) {
        self.apply(Operation::ScaleMode(mode));
    }

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

    pub fn selected_context(&self) -> Option<Self> {
        self.assert_thread();
        // SAFETY: this lease admits the serialized graphics thread that owns context selection.
        NonNull::new(unsafe { context_abi::_pCurContext }).map(Self::retain)
    }

    pub fn highlight(&self, rect: RECT) -> super::native_highlight::Highlight {
        self.assert_thread();
        super::native_highlight::Highlight::new(self.clone(), self.0.context, rect)
    }

    fn apply(&self, operation: Operation) -> (FRAME, CONTEXT, u32) {
        self.assert_thread();
        let selected = operation.selected_context(self.0.context.as_ptr());
        let mut rectangle = operation.rect();
        let rectangle = rectangle
            .as_mut()
            .map_or(std::ptr::null_mut(), std::ptr::from_mut);
        // SAFETY: the lease owns the selected context on this graphics thread.
        // Set-frame retains its new allocation; clipping and drawing retain the
        // selected target before entering this native operation boundary.
        unsafe {
            let previous = if operation.changes_selection() {
                context_abi::SetContext(selected)
            } else {
                context_abi::_pCurContext
            };
            let mut deadline = 0;
            let mut effect = None;
            match operation {
                Operation::Select | Operation::Deselect => (),
                Operation::BeginBatch => gfx::batch_graphics(),
                Operation::EndBatch => gfx::unbatch_graphics(),
                Operation::Flush => gfx::flush_graphics(),
                Operation::Fade(kind, interval) => {
                    deadline = super::cmap_abi::FadeScreen(kind, interval)
                }
                Operation::FinishTransforms => super::cmap_abi::FlushColorXForms(),
                Operation::TransitionSource(_) => gfx::SetTransitionSource(rectangle),
                Operation::Transition(_) => gfx::ScreenTransition(3, rectangle),
                Operation::ScaleMode(mode) => {
                    gfx::set_graphic_scale_mode(mode);
                }
                Operation::Background(color) => {
                    context_abi::SetContextBackGroundColor(color);
                }
                Operation::Foreground(color) => {
                    context_abi::SetContextForeGroundColor(color);
                }
                Operation::Frame(frame, setter) => effect = Some(setter(frame)),
                Operation::Clip(_) => {
                    context_abi::SetContextClipRect(rectangle);
                }
                Operation::Clear => frame_abi::ClearDrawable(),
                Operation::Point(color, mut point) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawPoint(&mut point);
                }
                Operation::Fill(color, mut rect) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawFilledRectangle(&mut rect);
                }
                Operation::Outline(color, mut rect) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawRectangle(&mut rect);
                }
                Operation::ShadowedBox(mut rect, background, dark, medium) => {
                    super::widgets_abi::DrawShadowedBox(&mut rect, background, dark, medium);
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
                    super::font_abi::SetContextFont(
                        font.as_ref()
                            .map_or(std::ptr::null_mut(), |font| font.as_raw()),
                    );
                }
                Operation::Text(font, mut text, color) => {
                    super::font_abi::SetContextFont(font.as_raw());
                    context_abi::SetContextForeGroundColor(color);
                    super::font_abi::font_DrawText(&mut text);
                }
            }
            let frame = effect.unwrap_or(self.0.context.as_ref().ForeGroundFrame);
            (frame, previous, deadline)
        }
    }

    pub fn target(&self) -> ContextFrame {
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

    pub fn batch(&self) -> BatchGuard {
        self.apply(Operation::BeginBatch);
        BatchGuard(self.clone())
    }

    pub fn with_batch<R>(&self, draw: impl FnOnce() -> R) -> R {
        let depth = self.batch();
        let result = draw();
        drop(depth);
        result
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

    pub fn with_text_state<R>(&self, draw: impl FnOnce() -> R) -> R {
        self.with_selected(|| {
            // SAFETY: the selected lease owns this descriptor on its graphics thread.
            let descriptor = unsafe { self.0.context.as_ref() };
            let font = NonNull::new(descriptor.Font).map(|raw| {
                self.0
                    .font
                    .borrow()
                    .as_ref()
                    .filter(|font| font.as_raw() == raw.as_ptr())
                    .cloned()
                    .unwrap_or_else(|| super::native_font::FontLease::retain(raw))
            });
            let restore = ContextTextState {
                context: self.clone(),
                font,
                color: descriptor.ForeGroundColor,
            };
            let result = draw();
            drop(restore);
            result
        })
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
        self.apply(Operation::Frame(
            target.frame,
            drawable_abi::SetContextFGFrame,
        ));
        *self.0.target.borrow_mut() = Some(target);
    }

    pub fn set_clip(&self, rect: RECT) {
        let _target = self.target();
        self.apply(Operation::Clip(Some(rect)));
    }

    fn clip_snapshot(&self) -> (RECT, bool) {
        let _target = self.target();
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
        let _target = self.target();
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
        let frame = self.target();
        let previous = frame.replace_hotspot(origin);
        let restore = ContextOrigin { frame, previous };
        let result = draw();
        drop(restore);
        result
    }

    pub fn with_unclipped<R>(&self, draw: impl FnOnce() -> R) -> R {
        let (previous, explicit) = self.clip_snapshot();
        let restore = ContextClip {
            context: self.clone(),
            previous: explicit.then_some(previous),
        };
        self.clear_clip();
        let result = draw();
        drop(restore);
        result
    }

    pub fn system_rect(&self, rect: Option<RECT>) {
        self.assert_thread();
        // SAFETY: this lease admits the serialized graphics thread; the rectangle is borrowed locally.
        unsafe {
            match rect {
                Some(rect) => super::sdl::lifecycle_abi::SetSystemRect(&rect),
                None => super::sdl::lifecycle_abi::ClearSystemRect(),
            }
        }
    }

    pub fn screen_dimensions(&self) -> (i32, i32) {
        self.assert_thread();
        // SAFETY: the graphics thread initializes and owns these display dimensions.
        unsafe { (gfx::ScreenWidth, gfx::ScreenHeight) }
    }

    fn copy_screen_region(&self, mut rect: RECT, save: bool) {
        self.select();
        // SAFETY: the selected lease retains its context; the rectangle is local.
        unsafe {
            if save {
                gfx::LoadIntoExtraScreen(&mut rect);
            } else {
                gfx::DrawFromExtraScreen(&mut rect);
            }
        }
    }

    pub fn save_screen_region(&self, rect: RECT) {
        self.copy_screen_region(rect, true);
    }

    pub fn restore_screen_region(&self, rect: RECT) {
        self.copy_screen_region(rect, false);
    }

    pub fn draw_line(&self, color: Color, first: POINT, second: POINT) {
        self.select();
        let mut line = LINE { first, second };
        // SAFETY: this lease selects and retains its context on the drawing thread.
        unsafe {
            context_abi::SetContextForeGroundColor(color);
            frame_abi::DrawLine(&mut line);
        }
    }

    pub fn line_visible(&self, clip: RECT, first: POINT, second: POINT) -> bool {
        self.assert_thread();
        let mut line = super::geometry_ffi::BresenhamLine {
            first,
            second,
            abs_delta_x: 0,
            abs_delta_y: 0,
            error_term: 0,
            end_points_exchanged: 0,
            intersect_code: 0,
        };
        // SAFETY: the clipping operation borrows only these initialized local values.
        unsafe { super::geometry_ffi::_clip_line(&clip, &mut line) != 0 }
    }

    pub fn draw_point(&self, color: Color, point: POINT) {
        self.apply(Operation::Point(color, point));
    }

    pub fn draw_rectangle(&self, color: Color, rect: RECT) {
        let _target = self.target();
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
        let _target = self.target();
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
        let _target = self.target();
        self.apply(Operation::Fill(color, rect));
    }

    pub fn flush(&self) {
        self.apply(Operation::Flush);
    }

    pub fn foreground(&self) -> Color {
        self.assert_thread();
        // SAFETY: this lease retains the descriptor on its owning graphics thread.
        unsafe { self.0.context.as_ref().ForeGroundColor }
    }

    pub fn set_foreground(&self, color: Color) {
        self.apply(Operation::Foreground(color));
    }

    pub fn draw_shadowed_box(&self, rect: RECT, background: Color, dark: Color, medium: Color) {
        let _target = self.target();
        self.apply(Operation::ShadowedBox(rect, background, dark, medium));
    }

    pub fn draw_stamp(&self, source: &ContextFrame, origin: POINT) {
        let _target = self.target();
        self.apply(Operation::Stamp(STAMP {
            origin,
            frame: source.frame,
        }));
    }

    pub fn replace_font(&self, font: Option<super::native_font::FontLease>) {
        let previous = self.0.font.replace(font.clone());
        self.apply(Operation::Font(font));
        drop(previous);
    }

    pub fn font_lease(&self) -> Option<super::native_font::FontLease> {
        self.assert_thread();
        // SAFETY: the retained context owns this graphics-thread descriptor.
        let font = unsafe { self.0.context.as_ref().Font };
        NonNull::new(font).map(|font| {
            self.0
                .font
                .borrow()
                .as_ref()
                .filter(|lease| lease.as_raw() == font.as_ptr())
                .cloned()
                .unwrap_or_else(|| super::native_font::FontLease::retain(font))
        })
    }

    pub fn font_effect(&self) -> ContextFrame {
        self.assert_thread();
        // SAFETY: the retained context owns this graphics-thread descriptor.
        let frame = unsafe { self.0.context.as_ref().FontEffect };
        self.0
            .effect
            .borrow()
            .as_ref()
            .filter(|effect| effect.frame == frame)
            .cloned()
            .unwrap_or_else(|| ContextFrame::retain(NonNull::new(frame)))
    }

    pub fn set_font_effect(&self, effect: ContextFrame) {
        let previous = self.0.effect.replace(Some(effect.clone()));
        self.apply(Operation::Frame(
            effect.frame,
            context_abi::SetContextFontEffect,
        ));
        drop(previous);
    }

    pub fn with_font_effect<R>(
        &self,
        effect: Option<&ContextFrame>,
        draw: impl FnOnce() -> R,
    ) -> R {
        self.with_selected(|| {
            let previous = self.font_effect();
            self.set_font_effect(
                effect
                    .cloned()
                    .unwrap_or_else(|| ContextFrame::retain(None)),
            );
            let restore = ContextEffect {
                context: self.clone(),
                previous,
            };
            let result = draw();
            drop(restore);
            result
        })
    }

    pub fn set_font(&self, font: &super::native_font::FontLease) {
        self.replace_font(Some(font.clone()));
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
        let _target = self.target();
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
        self.submit_batch(batch, Some(scale));
    }

    pub fn draw_preserving_scale(&self, batch: &mut super::native_batch::NativeBatch) {
        self.submit_batch(batch, None);
    }

    fn submit_batch(&self, batch: &mut super::native_batch::NativeBatch, scale: Option<i32>) {
        let _target = self.target();
        let head = if batch.is_empty() {
            u16::MAX
        } else {
            batch.head
        };
        if let Some(scale) = scale {
            // SAFETY: selecting the retained target checked its graphics thread.
            unsafe { gfx::SetGraphicScale(scale) };
        }
        // SAFETY: this lease retains the graphics-thread context/target. The
        // batch owns bounded links and retains all registered stamp sources;
        // it cannot contain text pointers into caller storage.
        unsafe { super::frame_abi::DrawBatch(batch.primitives.as_mut_ptr(), u32::from(head), 0) };
        if scale.is_some() {
            // SAFETY: this context remains selected on its graphics thread.
            unsafe { gfx::SetGraphicScale(0) };
        }
    }

    pub fn clear(&self) {
        let _target = self.target();
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
        effect: RefCell::new(None),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn retained_frame_metadata_rejects_pixel_less_frames_and_survives_logical_release() {
        // SAFETY: this serialized fixture owns this newly registered drawable.
        let drawable = unsafe {
            let drawable = drawable_abi::AllocDrawable(16);
            (*drawable).MaxIndex = 15;
            drawable
        };
        // SAFETY: the complete drawable remains owned until its explicit release below.
        let raw = unsafe { super::super::pixmap_abi::CaptureDrawable(drawable) };
        let retained = ContextFrame::retain(NonNull::new(raw));
        assert_eq!(retained.frame_count(), 16);
        assert!(retained.alpha_occupancy().is_none());
        // SAFETY: this unique native release leaves physical storage with retained frames.
        unsafe { drawable_abi::DestroyDrawable(drawable) };
        assert_eq!(retained.wrapped_frame(31).frame_count(), 16);
        assert!(ContextFrame::retain(None).alpha_occupancy().is_none());
        assert_eq!(ContextFrame::retain(None).frame_count(), 0);
    }

    #[test]
    #[serial_test::serial]
    fn context_owners_release_once_and_leases_keep_each_allocation_alive() {
        let baseline = ffi::native_allocations().contexts.len();
        let first = LoadedContext::create(c"first-owned-context");
        let second = LoadedContext::create(c"second-owned-context");
        let first_lease = first.lease();
        let second_lease = second.lease();
        let first_allocation = Rc::downgrade(&first_lease.0);
        let second_allocation = Rc::downgrade(&second_lease.0);
        assert!(!Rc::ptr_eq(&first_lease.0, &second_lease.0));
        assert_eq!(ffi::native_allocations().contexts.len(), baseline + 2);
        first_lease.select();
        second_lease.select();
        drop(first);
        assert_eq!(ffi::native_allocations().contexts.len(), baseline + 1);
        assert!(first_allocation.upgrade().is_some());
        drop(first_lease);
        assert!(first_allocation.upgrade().is_none());
        assert_eq!(
            // SAFETY: this serialized fixture retains the selected second context.
            unsafe { context_abi::_pCurContext },
            second_lease.0.context.as_ptr()
        );
        drop(second);
        assert_eq!(ffi::native_allocations().contexts.len(), baseline);
        assert!(second_allocation.upgrade().is_some());
        drop(second_lease);
        assert!(second_allocation.upgrade().is_none());
        // SAFETY: releasing the selected owner clears native context selection.
        assert!(unsafe { context_abi::_pCurContext }.is_null());
    }
}
