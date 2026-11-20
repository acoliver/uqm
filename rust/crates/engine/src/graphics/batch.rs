//! Primitive batches drawn in one `DrawBatch` call; stamps hold their frames.
use super::drawable::ContextFrame;
use super::geometry::{Color, Line, Point, Rect};

/// One primitive of a batch (C: `PRIMITIVE` and its type).
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    Point(Color, Point),
    Stamp(ContextFrame, Point),
    StampFill(Color, ContextFrame, Point),
    Line(Color, Line),
    Rect(Color, Rect),
    FillRect(Color, Rect),
    Empty,
}

/// A shape to batch with its colour.
#[derive(Clone, Debug)]
pub enum NativeShape {
    Point(Point),
    Stamp {
        source: ContextFrame,
        origin: Point,
        filled: bool,
    },
    Line(Line),
    Rectangle {
        rect: Rect,
        filled: bool,
    },
    Empty,
}

#[derive(Clone, Debug)]
pub struct DrawCommand {
    pub color: Color,
    pub shape: NativeShape,
}

impl DrawCommand {
    fn primitive(self) -> Primitive {
        let color = self.color;
        match self.shape {
            NativeShape::Point(point) => Primitive::Point(color, point),
            NativeShape::Stamp {
                source,
                origin,
                filled: false,
            } => Primitive::Stamp(source, origin),
            NativeShape::Stamp {
                source,
                origin,
                filled: true,
            } => Primitive::StampFill(color, source, origin),
            NativeShape::Line(line) => Primitive::Line(color, line),
            NativeShape::Rectangle {
                rect,
                filled: false,
            } => Primitive::Rect(color, rect),
            NativeShape::Rectangle { rect, filled: true } => Primitive::FillRect(color, rect),
            NativeShape::Empty => Primitive::Empty,
        }
    }
}

/// Primitives drawn in order from `head` to the end.
#[derive(Clone, Debug, Default)]
pub struct NativeBatch {
    primitives: Vec<Primitive>,
    head: usize,
}

impl NativeBatch {
    pub fn new(commands: Vec<DrawCommand>) -> Self {
        Self {
            primitives: commands.into_iter().map(DrawCommand::primitive).collect(),
            head: 0,
        }
    }

    /// A batch whose primitives each draw in order (C: rows linked forward
    /// only); drawing is the same as [`Self::new`].
    pub fn forward_chain(commands: Vec<DrawCommand>) -> Self {
        Self::new(commands)
    }

    /// Points drawn from `first` to the end.
    pub fn point_chain(
        points: impl ExactSizeIterator<Item = (Point, Color)>,
        first: usize,
    ) -> Self {
        let mut batch = Self::default();
        batch.replace_point_chain(points, first);
        batch
    }

    pub fn replace_point_chain(
        &mut self,
        points: impl ExactSizeIterator<Item = (Point, Color)>,
        first: usize,
    ) {
        let count = points.len();
        assert!(first < count, "point chain head outside batch");
        self.primitives.clear();
        self.primitives
            .extend(points.map(|(point, color)| Primitive::Point(color, point)));
        self.head = first;
    }

    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }

    /// The primitives a draw visits.
    pub(crate) fn drawn(&self) -> &[Primitive] {
        &self.primitives[self.head.min(self.primitives.len())..]
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_chain_draws_from_its_head_and_reuses_storage() {
        let points =
            (0..64).map(|index| (Point::new(index, -index), Color::rgb(index as u8, 2, 3)));
        let mut batch = NativeBatch::point_chain(points, 61);
        assert_eq!(batch.drawn().len(), 3);
        assert_eq!(
            batch.drawn()[0],
            Primitive::Point(Color::rgb(61, 2, 3), Point::new(61, -61))
        );
        let storage = batch.primitives.as_ptr();
        batch.replace_point_chain(
            (0..64).map(|index| (Point::new(index * 2, 0), Color::rgb(9, 8, 7))),
            59,
        );
        assert_eq!(batch.primitives.as_ptr(), storage);
        assert_eq!(batch.drawn().len(), 5);
    }

    #[test]
    fn typed_commands_keep_their_order_and_kind() {
        let color = Color::rgb(1, 2, 3);
        let batch = NativeBatch::new(vec![
            DrawCommand {
                color,
                shape: NativeShape::Point(Point::new(4, 5)),
            },
            DrawCommand {
                color,
                shape: NativeShape::Rectangle {
                    rect: Rect::new(1, 2, 3, 4),
                    filled: true,
                },
            },
            DrawCommand {
                color,
                shape: NativeShape::Empty,
            },
        ]);
        assert_eq!(
            batch.drawn(),
            [
                Primitive::Point(color, Point::new(4, 5)),
                Primitive::FillRect(color, Rect::new(1, 2, 3, 4)),
                Primitive::Empty
            ]
        );
        assert!(NativeBatch::new(Vec::new()).is_empty());
    }
}
