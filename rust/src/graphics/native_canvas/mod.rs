//! SDL surface canvas operations, ported from sdl/canvas.c (GPL-2.0-or-later).
//!
//! A `TFB_Canvas` is a live `SDL_Surface`. Every export keeps the C symbol,
//! argument types and arithmetic of the original, so C callers and the
//! `TFB_Image` layout in `native_abi` are unchanged.
//!
//! * `surface`      thin SDL helpers shared by the other modules
//! * `lifecycle`    canvas creation, deletion, format and raw access
//! * `transparency` palettes, colorkeys and transparent colors
//! * `drawing`      lines, rects, blits, images, filled images, font chars
//! * `scaling`      scaled extents, nearest and bilinear rescale
//! * `trilinear`    mipmap-blended rescale
//! * `rotation`     rotated extents and rotation
//! * `pixels`       pixel access, collision tests and bulk transfers

pub mod drawing;
pub mod lifecycle;
pub mod pixels;
pub mod rotation;
pub mod scaling;
pub mod surface;
pub mod transparency;
pub mod trilinear;

#[cfg(test)]
mod tests;

/// `GSCALE_IDENTITY` from gfx_common.h: the fixed-point scale of 1.0.
pub const GSCALE_IDENTITY: i32 = 256;

/// `TFB_SCALE_*` scaler kinds from gfx_common.h (`TFB_SCALE_STEP` is 0).
pub const TFB_SCALE_NEAREST: i32 = 1;
pub const TFB_SCALE_BILINEAR: i32 = 2;
pub const TFB_SCALE_TRILINEAR: i32 = 3;

/// `DrawKind` values from gfxlib.h.
pub const DRAW_REPLACE: u8 = 0;
pub const DRAW_ADDITIVE: u8 = 1;
pub const DRAW_ALPHA: u8 = 2;
