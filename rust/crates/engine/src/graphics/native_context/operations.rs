//! Retained drawing primitives dispatched after context selection.
use super::*;

pub(super) enum Primitive {
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
