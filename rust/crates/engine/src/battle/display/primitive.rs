use super::super::frames::FrameSelection;
use crate::graphics::batch::{DrawCommand, NativeShape};
use crate::graphics::drawable::ContextFrame;
use crate::graphics::geometry::{Color, Extent, Line, Point, Rect};
use crate::graphics::intersection::Stamp;

#[derive(Clone)]
pub struct DisplayPrimitive {
    pub kind: u8,
    pub color: Color,
    origin: Point,
    second: Point,
    extent: Extent,
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
            origin: Point { x: 0, y: 0 },
            second: Point { x: 0, y: 0 },
            extent: Extent {
                width: 0,
                height: 0,
            },
            frame: FrameSelection::ABSENT,
        }
    }
}

impl DisplayPrimitive {
    pub fn point(&self) -> Point {
        self.origin
    }
    pub fn set_point(&mut self, point: Point) {
        self.origin = point;
    }
    pub fn set_line(&mut self, line: Line) {
        self.origin = line.first;
        self.second = line.second;
    }
    pub fn stamp(&self) -> Stamp {
        Stamp {
            origin: self.origin,
            frame: self.frame.raw(),
        }
    }
    pub fn set_stamp(&mut self, stamp: Stamp) {
        self.origin = stamp.origin;
        self.frame = FrameSelection::retain(stamp.frame);
    }
    pub fn set_image(&mut self, origin: Point, frame: FrameSelection) {
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
                source: ContextFrame::retain(self.frame.raw()),
                origin: self.origin,
                filled: self.kind == 2,
            },
            3 => NativeShape::Line(Line {
                first: self.origin,
                second: self.second,
            }),
            5 | 6 => NativeShape::Rectangle {
                rect: Rect {
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
        primitive.set_image(Point { x: 3, y: 5 }, FrameSelection::ABSENT);
        assert_eq!(
            (primitive.stamp().origin.x, primitive.stamp().origin.y),
            (3, 5)
        );
        primitive.set_line(Line {
            first: Point { x: 7, y: 11 },
            second: Point { x: 13, y: 17 },
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
