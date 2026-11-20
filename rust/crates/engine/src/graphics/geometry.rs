//! Drawing values shared by the graphics API: colours, points, extents,
//! rectangles and draw modes. Coordinates keep the C `COORD` width (`i16`).

/// An RGBA colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// C: `BUILD_COLOR_RGBA` with full alpha.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0xff }
    }

    /// `BUILD_COLOR(MAKE_RGB15(0x1F, 0x1F, 0x1F), 0x0F)`: the grey background
    /// after the Utwig bomb self-destruct fade.
    pub const fn white_gray() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 0x0F,
        }
    }
}

/// A pixel position (C: `POINT`, also used for hot spots).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Point {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

/// C: `HOT_SPOT`.
pub type HotSpot = Point;

/// A size (C: `EXTENT`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Extent {
    pub width: i16,
    pub height: i16,
}

impl Extent {
    pub const fn new(width: i16, height: i16) -> Self {
        Self { width, height }
    }

    pub const fn zero() -> Self {
        Self::new(0, 0)
    }
}

/// A rectangle (C: `RECT`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub corner: Point,
    pub extent: Extent,
}

impl Rect {
    pub const fn new(x: i16, y: i16, width: i16, height: i16) -> Self {
        Self {
            corner: Point::new(x, y),
            extent: Extent::new(width, height),
        }
    }
}

/// A line segment (C: `LINE`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Line {
    pub first: Point,
    pub second: Point,
}

/// How pixels are written (C: `DrawMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DrawMode {
    pub kind: u8,
    pub factor: i16,
}

impl DrawMode {
    /// C: `DRAW_REPLACE_MODE`.
    pub const REPLACE: Self = Self { kind: 0, factor: 0 };

    pub const fn new(kind: u8, factor: i16) -> Self {
        Self { kind, factor }
    }
}

/// Horizontal text alignment (C: `TEXT_ALIGN`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    /// The C `TEXT_ALIGN` value: 0 left, 1 centre, 2 right. Any other value
    /// is a broken caller.
    pub fn from_c(value: i32) -> Self {
        match value {
            0 => Self::Left,
            1 => Self::Center,
            2 => Self::Right,
            _ => panic!("invalid text alignment {value}"),
        }
    }
}
