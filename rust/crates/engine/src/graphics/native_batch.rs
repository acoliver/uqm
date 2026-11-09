//! Native drawing batches built from initialized geometry and retained stamp sources.

use super::{native_abi::*, native_context::ContextFrame};

pub enum NativeShape {
    Point(POINT),
    Stamp {
        source: ContextFrame,
        origin: POINT,
        filled: bool,
    },
    Line(LINE),
    Rectangle {
        rect: RECT,
        filled: bool,
    },
    Empty,
}

pub struct DrawCommand {
    pub color: Color,
    pub shape: NativeShape,
}

pub struct NativeBatch {
    pub(super) primitives: Vec<PRIMITIVE>,
    pub(super) head: u16,
    _commands: Vec<DrawCommand>,
}

impl NativeBatch {
    pub fn new(commands: Vec<DrawCommand>) -> Self {
        assert!(
            commands.len() < usize::from(u16::MAX),
            "native batch too large"
        );
        let count = commands.len();
        let primitives = commands
            .iter()
            .enumerate()
            .map(|(index, command)| {
                let (kind, object) = match &command.shape {
                    NativeShape::Point(point) => (0, PRIM_DESC { Point: *point }),
                    NativeShape::Stamp {
                        source,
                        origin,
                        filled,
                    } => (
                        if *filled { 2 } else { 1 },
                        PRIM_DESC {
                            Stamp: STAMP {
                                origin: *origin,
                                frame: source.frame,
                            },
                        },
                    ),
                    NativeShape::Line(line) => (3, PRIM_DESC { Line: *line }),
                    NativeShape::Rectangle { rect, filled } => {
                        (if *filled { 6 } else { 5 }, PRIM_DESC { Rect: *rect })
                    }
                    NativeShape::Empty => (
                        7,
                        PRIM_DESC {
                            Point: POINT { x: 0, y: 0 },
                        },
                    ),
                };
                let previous = index.checked_sub(1).map_or(u16::MAX, |ix| ix as u16);
                let next = if index + 1 < count {
                    (index + 1) as u16
                } else {
                    u16::MAX
                };
                PRIMITIVE {
                    Links: u32::from(previous) | (u32::from(next) << 16),
                    Type: kind,
                    color: command.color,
                    Object: object,
                }
            })
            .collect();
        Self {
            primitives,
            head: 0,
            _commands: commands,
        }
    }

    pub fn forward_chain(commands: Vec<DrawCommand>) -> Self {
        let mut batch = Self::new(commands);
        for primitive in &mut batch.primitives {
            primitive.Links |= u32::from(u16::MAX);
        }
        batch
    }

    pub fn point_chain(
        points: impl ExactSizeIterator<Item = (POINT, Color)>,
        first: usize,
    ) -> Self {
        let mut batch = Self::new(Vec::new());
        batch.replace_point_chain(points, first);
        batch
    }

    pub fn replace_point_chain(
        &mut self,
        points: impl ExactSizeIterator<Item = (POINT, Color)>,
        first: usize,
    ) {
        let count = points.len();
        assert!(count < usize::from(u16::MAX), "native batch too large");
        assert!(first < count, "point chain head outside batch");
        self.primitives.clear();
        self.primitives
            .extend(points.enumerate().map(|(index, (point, color))| {
                let next = if index + 1 < count {
                    index as u16 + 1
                } else {
                    u16::MAX
                };
                PRIMITIVE {
                    Links: u32::from(u16::MAX) | (u32::from(next) << 16),
                    Type: 0,
                    color,
                    Object: PRIM_DESC { Point: point },
                }
            }));
        self._commands.clear();
        self.head = first as u16;
    }

    pub fn primitives(&self) -> &[PRIMITIVE] {
        &self.primitives
    }
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_chain_preserves_sentinel_predecessors_for_rectangle_rows() {
        let batch = NativeBatch::forward_chain(
            (0..2)
                .map(|row| DrawCommand {
                    color: Color {
                        r: 11,
                        g: 22,
                        b: 33,
                        a: 101,
                    },
                    shape: NativeShape::Rectangle {
                        rect: RECT {
                            corner: POINT { x: 1, y: row },
                            extent: EXTENT {
                                width: 5,
                                height: 1,
                            },
                        },
                        filled: true,
                    },
                })
                .collect(),
        );
        assert_eq!(batch.primitives()[0].Links, u32::from(u16::MAX) | (1 << 16));
        assert_eq!(batch.primitives()[1].Links, u32::MAX);
        assert_eq!(batch.primitives()[0].Type, 6);
        assert_eq!(batch.primitives()[1].color.a, 101);
    }

    #[test]
    fn point_chain_keeps_tail_indices_and_sentinel_predecessors() {
        let points = (0..64).map(|index| {
            (
                POINT {
                    x: index,
                    y: -index,
                },
                Color {
                    r: index as u8,
                    g: 2,
                    b: 3,
                    a: 255,
                },
            )
        });
        let mut batch = NativeBatch::point_chain(points, 61);
        assert_eq!(batch.head, 61);
        assert_eq!(batch.primitives().len(), 64);
        for (index, primitive) in batch.primitives().iter().enumerate() {
            let next = if index == 63 {
                u16::MAX
            } else {
                index as u16 + 1
            };
            assert_eq!(
                primitive.Links,
                u32::from(u16::MAX) | (u32::from(next) << 16)
            );
            assert_eq!(primitive.Type, 0);
            assert_eq!(primitive.color.r, index as u8);
            assert_eq!(primitive.color.a, 255);
            // SAFETY: each command constructed a point primitive.
            unsafe {
                assert_eq!(primitive.Object.Point.x, index as i16);
                assert_eq!(primitive.Object.Point.y, -(index as i16));
            }
        }
        let storage = batch.primitives.as_ptr();
        batch.replace_point_chain(
            (0..64).map(|index| {
                (
                    POINT { x: index * 2, y: 0 },
                    Color {
                        r: 9,
                        g: 8,
                        b: 7,
                        a: 255,
                    },
                )
            }),
            59,
        );
        assert_eq!(batch.primitives.as_ptr(), storage);
        assert_eq!(batch.head, 59);
        assert_eq!(batch.primitives[59].color.r, 9);
        assert_eq!(batch.primitives[63].Links, u32::MAX);
    }

    #[test]
    fn typed_commands_link_in_order_and_empty_batches_have_no_records() {
        let color = Color {
            r: 1,
            g: 2,
            b: 3,
            a: 255,
        };
        let batch = NativeBatch::new(vec![
            DrawCommand {
                color,
                shape: NativeShape::Point(POINT { x: 4, y: 5 }),
            },
            DrawCommand {
                color,
                shape: NativeShape::Empty,
            },
        ]);
        assert_eq!(batch.primitives()[0].Links, u32::from(u16::MAX) | (1 << 16));
        assert_eq!(batch.primitives()[1].Links, u32::from(u16::MAX) << 16);
        assert_eq!(batch.primitives()[0].Type, 0);
        assert_eq!(batch.primitives()[1].Type, 7);
        assert!(NativeBatch::new(Vec::new()).is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn a_batch_retains_its_stamp_after_the_native_owner_is_released() {
        // SAFETY: the test exclusively owns the initialized native allocation.
        let drawable = unsafe { super::super::drawable_abi::AllocDrawable(1) };
        // SAFETY: the allocated drawable contains one initialized frame.
        let frame = unsafe { super::super::pixmap_abi::CaptureDrawable(drawable) };
        let source = ContextFrame::retain(std::ptr::NonNull::new(frame));
        let batch = NativeBatch::new(vec![DrawCommand {
            color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            shape: NativeShape::Stamp {
                source,
                origin: POINT { x: 1, y: 2 },
                filled: false,
            },
        }]);
        // SAFETY: consume the native allocation owner; the batch retains its frame.
        let released = unsafe { super::super::drawable_abi::DestroyDrawable(drawable) };
        assert_eq!(released, 1);
        assert_eq!(
            super::super::drawable_abi::retain_frame_allocations(&[frame])[0].frame(0),
            Some(frame)
        );
        drop(batch);
        assert!(std::panic::catch_unwind(|| {
            super::super::drawable_abi::retain_frame_allocations(&[frame])
        })
        .is_err());
    }
}
