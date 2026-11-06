use super::super::frames::FrameSelection;
use crate::graphics::{
    context_abi::ContextFrame,
    native_abi::*,
    native_batch::{DrawCommand, NativeShape},
};

#[derive(Clone)]
pub struct DisplayPrimitive {
    pub kind: u8,
    pub color: Color,
    origin: POINT,
    second: POINT,
    extent: EXTENT,
    frame: FrameSelection,
}

impl Default for DisplayPrimitive {
    fn default() -> Self {
        Self {
            kind: 0,
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            origin: POINT { x: 0, y: 0 },
            second: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: 0,
                height: 0,
            },
            frame: FrameSelection::ABSENT,
        }
    }
}

impl DisplayPrimitive {
    pub fn point(&self) -> POINT {
        self.origin
    }
    pub fn set_point(&mut self, point: POINT) {
        self.origin = point;
    }
    pub fn set_line(&mut self, line: LINE) {
        self.origin = line.first;
        self.second = line.second;
    }
    pub fn stamp(&self) -> STAMP {
        STAMP {
            origin: self.origin,
            frame: self.frame.raw(),
        }
    }
    pub fn set_stamp(&mut self, stamp: STAMP) {
        self.origin = stamp.origin;
        self.frame = FrameSelection::retain(stamp.frame);
    }
    pub(crate) fn set_image(&mut self, origin: POINT, frame: FrameSelection) {
        self.origin = origin;
        self.frame = frame;
    }
    pub fn set_frame(&mut self, frame: FrameSelection) {
        self.frame = frame;
    }
    pub(super) fn command(&self) -> DrawCommand {
        let shape = match self.kind {
            0 => NativeShape::Point(self.origin),
            1 | 2 => NativeShape::Stamp {
                source: ContextFrame::retain(std::ptr::NonNull::new(self.frame.raw())),
                origin: self.origin,
                filled: self.kind == 2,
            },
            3 => NativeShape::Line(LINE {
                first: self.origin,
                second: self.second,
            }),
            5 | 6 => NativeShape::Rectangle {
                rect: RECT {
                    corner: self.origin,
                    extent: self.extent,
                },
                filled: self.kind == 6,
            },
            7 => NativeShape::Empty,
            _ => panic!("unsupported battle primitive {}", self.kind),
        };
        DrawCommand {
            color: self.color,
            shape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_and_line_updates_share_the_stamp_origin_without_union_reads() {
        let mut primitive = DisplayPrimitive::default();
        primitive.set_image(POINT { x: 3, y: 5 }, FrameSelection::ABSENT);
        assert_eq!(
            (primitive.stamp().origin.x, primitive.stamp().origin.y),
            (3, 5)
        );
        primitive.set_line(LINE {
            first: POINT { x: 7, y: 11 },
            second: POINT { x: 13, y: 17 },
        });
        primitive.kind = 3;
        match primitive.command().shape {
            NativeShape::Line(line) => assert_eq!((line.first.x, line.second.y), (7, 17)),
            _ => panic!("expected a line"),
        }
        primitive.kind = 1;
        assert!(primitive.stamp().frame.is_null());
        assert_eq!(
            (primitive.stamp().origin.x, primitive.stamp().origin.y),
            (7, 11)
        );
    }
}
