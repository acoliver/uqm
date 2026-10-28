use super::native_abi::{extent as Extent, Color};

impl Extent {
    pub const fn new(width: i16, height: i16) -> Self {
        Extent { width, height }
    }

    pub const fn zero() -> Self {
        Extent {
            width: 0,
            height: 0,
        }
    }
}

impl Color {
    /// `BUILD_COLOR(MAKE_RGB15(0x1F, 0x1F, 0x1F), 0x0F)` — the gray
    /// background used after the Utwig bomb self-destruct fade.
    /// RGB15(0x1F, 0x1F, 0x1F) = (255, 255, 255) with ramp 0x0F.
    pub const fn white_gray() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 0x0F,
        }
    }
}
