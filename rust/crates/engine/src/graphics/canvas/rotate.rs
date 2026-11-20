//! Canvas rotation, from the SDL_gfx rotozoomer as modified by UQM
//! (LGPL, (c) A. Schiffler). Fixed-point steps match the C implementation.
use super::{blit, Canvas};
use crate::graphics::geometry::Extent;

const VALUE_LIMIT: f64 = 0.001;

/// Destination size and trigonometry of a rotation by `angle` degrees.
fn size_trig(width: i32, height: i32, angle: f64) -> (i32, i32, f64, f64) {
    let radangle = angle * (std::f64::consts::PI / 180.0);
    let sin = radangle.sin();
    let cos = radangle.cos();
    let x = f64::from(width / 2);
    let y = f64::from(height / 2);
    let (cx, cy, sx, sy) = (cos * x, cos * y, sin * x, sin * y);
    (
        2 * ((cx.abs() + sy.abs()).ceil().max(1.0) as i32),
        2 * ((sx.abs() + cy.abs()).ceil().max(1.0) as i32),
        cos,
        sin,
    )
}

/// The canvas size a rotation by `angle` degrees needs.
pub fn rotated_extent(src: &Canvas, angle: i32) -> Extent {
    let (w, h, _, _) = size_trig(src.width(), src.height(), f64::from(angle));
    Extent::new(w as i16, h as i16)
}

/// A same-format canvas sized for the rotation, carrying the source's
/// transparency.
pub fn rotation_target(src: &Canvas, angle: i32) -> Canvas {
    let size = rotated_extent(src, angle);
    let mut target = Canvas::new(src.format(), i32::from(size.width), i32::from(size.height));
    target.copy_transparency(src);
    target
}

/// Rotate `src` into `dst` without smoothing.
pub fn rotate(src: &Canvas, dst: &mut Canvas, angle: i32, size: Extent) {
    if i32::from(size.width) > dst.width() || i32::from(size.height) > dst.height() {
        log::warn!(
            "TFB_DrawCanvas_Rotate: Tried to rotate image to size {} {} when dst_canvas has only dimensions of {} {}! Failing.",
            size.width,
            size.height,
            dst.width(),
            dst.height()
        );
        return;
    }
    if let Some(color) = src.transparent_color() {
        dst.set_transparent_color(color);
        // fill the destination with the transparent colour before rotating
        let fill = dst.map_rgba(color.r, color.g, color.b, 0);
        dst.fill_rect(None, fill);
    }
    let angle = f64::from(angle);
    if src.format() != dst.format() {
        log::warn!("TFB_DrawCanvas_Rotate: WARNING: actual rotation func returned failure");
        return;
    }
    if angle.abs() <= VALUE_LIMIT {
        blit(src, None, dst, None);
        return;
    }
    let (w, h, cos, sin) = size_trig(src.width(), src.height(), angle);
    if dst.width() < w || dst.height() < h {
        log::warn!("TFB_DrawCanvas_Rotate: WARNING: actual rotation func returned failure");
        return;
    }
    let (isin, icos) = ((sin * 65536.0) as i32, (cos * 65536.0) as i32);
    let (cx, cy) = (w / 2, h / 2);
    if src.is_indexed() {
        if let Some(palette) = src.palette() {
            dst.set_palette_colors(0, palette.as_slice());
        }
        let key = src.color_key().unwrap_or(0) as u8;
        dst.indexes_mut().fill(key);
    }
    let xd = (src.width() - dst.width()) << 15;
    let yd = (src.height() - dst.height()) << 15;
    let ax = (cx << 16) - icos * cx;
    let ay = (cy << 16) - isin * cx;
    for y in 0..dst.height() {
        let dy = cy - y;
        let mut sdx = ax + isin * dy + xd;
        let mut sdy = ay - icos * dy + yd;
        for x in 0..dst.width() {
            let dx = (sdx >> 16) as i16 as i32;
            let dy = (sdy >> 16) as i16 as i32;
            if dx >= 0 && dy >= 0 && dx < src.width() && dy < src.height() {
                dst.set_pixel(x, y, src.pixel(dx, dy));
            }
            sdx = sdx.wrapping_add(icos);
            sdy = sdy.wrapping_add(isin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::PixelFormat;
    use super::*;

    #[test]
    fn quarter_turn_moves_pixels_and_sizes_the_target() {
        let mut src = Canvas::new(PixelFormat::Rgbx, 4, 2);
        src.words_mut().fill(0x1100_0000);
        let size = rotated_extent(&src, 90);
        assert_eq!(size, Extent::new(4, 4));
        let mut dst = rotation_target(&src, 90);
        rotate(&src, &mut dst, 90, size);
        let copied = dst.words().iter().filter(|&&p| p == 0x1100_0000).count();
        assert!(copied > 0 && copied < dst.words().len());
    }

    #[test]
    fn indexed_rotation_fills_with_the_key_and_copies_the_palette() {
        let mut src = Canvas::new(PixelFormat::Indexed, 2, 2);
        src.set_color_key(Some(7));
        src.indexes_mut().fill(3);
        let mut dst = rotation_target(&src, 45);
        let size = rotated_extent(&src, 45);
        rotate(&src, &mut dst, 45, size);
        assert!(dst.indexes().contains(&7));
        assert!(dst.indexes().contains(&3));
        // the legacy transparent-index query drops the key of keyed targets
        assert_eq!(dst.color_key(), None);
    }
}
