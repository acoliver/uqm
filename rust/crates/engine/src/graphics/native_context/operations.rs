//! Retained drawing primitives dispatched after context selection.
use super::*;

pub(super) enum Primitive {
    ScaledStamp(STAMP, i32, i32),
    TracedText(super::super::native_font::FontLease, TEXT, Color, Color),
    Point(Color, POINT),
    Line(Color, LINE),
    DrawMode(DrawMode),
    ScreenRegion(RECT, unsafe extern "C" fn(*mut RECT)),
}

impl Primitive {
    pub(super) fn draw(self, context: &ContextLease) {
        context.assert_thread();
        // SAFETY: apply selected this retained context before dispatching the primitive.
        unsafe {
            match self {
                Self::ScaledStamp(mut stamp, scale, mode) => {
                    let previous_mode = gfx::set_graphic_scale_mode(mode);
                    let previous_scale = gfx::set_graphic_scale(scale);
                    frame_abi::DrawStamp(&mut stamp);
                    gfx::set_graphic_scale(previous_scale);
                    gfx::set_graphic_scale_mode(previous_mode);
                }
                Self::TracedText(font, mut text, color, trace) => {
                    super::super::font_abi::SetContextFont(font.as_raw());
                    super::super::font_abi::font_DrawTracedText(&mut text, color, trace);
                }

                Self::DrawMode(mode) => {
                    context_abi::SetContextDrawMode(mode);
                }
                Self::Line(color, mut line) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawLine(&mut line);
                }
                Self::ScreenRegion(mut rect, draw) => draw(&mut rect),
                Self::Point(color, mut point) => {
                    context_abi::SetContextForeGroundColor(color);
                    frame_abi::DrawPoint(&mut point);
                }
            }
        }
    }
}

impl ContextLease {
    pub fn draw_scaled_stamp(&self, source: &ContextFrame, origin: POINT, scale: i32, mode: i32) {
        let _target = self.target();
        self.apply(Operation::Primitive(Primitive::ScaledStamp(
            STAMP {
                origin,
                frame: source.as_raw(),
            },
            scale,
            mode,
        )));
    }

    pub fn draw_traced_text(
        &self,
        font: &super::super::native_font::FontLease,
        string: &std::ffi::CStr,
        baseline: POINT,
        align: TEXT_ALIGN,
        color: Color,
        trace: Color,
    ) {
        let _target = self.target();
        *self.0.font.borrow_mut() = Some(font.clone());
        self.apply(Operation::Primitive(Primitive::TracedText(
            font.clone(),
            TEXT {
                baseline,
                pStr: string.as_ptr(),
                align,
                CharCount: u16::MAX,
            },
            color,
            trace,
        )));
    }
}

struct ContextTarget {
    context: ContextLease,
    previous: ContextFrame,
}
impl Drop for ContextTarget {
    fn drop(&mut self) {
        self.context.set_frame(self.previous.clone());
    }
}

impl ContextLease {
    /// Execute committed drawing without presenting a frame before releasing its resources.
    pub fn drain(&self) {
        super::super::dcqueue_abi::require_queue(self);
        // The screen wait drains without swapping while a graphics batch is active.
        self.with_batch(|| self.flush());
    }

    pub fn with_frame<R>(&self, target: ContextFrame, draw: impl FnOnce() -> R) -> R {
        self.with_selected(|| {
            let restore = ContextTarget {
                context: self.clone(),
                previous: self.target(),
            };
            self.set_frame(target);
            let result = draw();
            drop(restore);
            result
        })
    }
}
