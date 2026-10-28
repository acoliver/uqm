//! Layout-checked declarations for the remaining engine graphics callers.
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::all,
    reason = "Rust-owned engine ABI declarations"
)]
include!("../abi/graphics_abi.rs");

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

use self::extent as Extent;
