//! Software lines, fills and blits with the additive and alpha draw modes,
//! ported from primitives.c (GPL-2.0-or-later).
use super::{Canvas, PixelRect};

/// `DrawKind` values from gfxlib.h.
pub const DRAW_REPLACE: u8 = 0;
pub const DRAW_ADDITIVE: u8 = 1;
pub const DRAW_ALPHA: u8 = 2;

/// How one pixel is written for a draw kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plot {
    Replace,
    Additive,
    Alpha,
}

impl Plot {
    /// The renderer for `kind`; `None` where the C code had no renderer.
    pub fn for_canvas(canvas: &Canvas, kind: u8) -> Option<Self> {
        if canvas.format().bytes_per_pixel() != 4 || canvas.format().has_alpha() && kind != 0 {
            return None;
        }
        match kind {
            DRAW_REPLACE => Some(Self::Replace),
            DRAW_ADDITIVE => Some(Self::Additive),
            DRAW_ALPHA => Some(Self::Alpha),
            _ => None,
        }
    }

    fn put(self, canvas: &mut Canvas, x: i32, y: i32, pixel: u32, factor: i32) {
        if self == Self::Replace || self == Self::Alpha && factor == 255 {
            canvas.set_pixel(x, y, pixel);
            return;
        }
        let old = canvas.pixel(x, y);
        let mut result = 0;
        for shift in [24, 16, 8] {
            result |= u32::from(channel(
                (old >> shift) as u8,
                (pixel >> shift) as u8,
                factor,
                self,
            )) << shift;
        }
        canvas.set_pixel(x, y, result);
    }
}

fn channel(dst: u8, src: u8, factor: i32, plot: Plot) -> u8 {
    if plot == Plot::Additive {
        (i32::from(dst)
            + if factor == 255 {
                i32::from(src)
            } else {
                (i32::from(src) * factor) >> 8
            })
        .clamp(0, 255) as u8
    } else {
        ((((i32::from(src) - i32::from(dst)) * factor) >> 8) + i32::from(dst)) as u8
    }
}

fn outcode(x: f32, y: f32, xmin: f32, ymin: f32, xmax: f32, ymax: f32) -> u8 {
    let mut c = 0;
    if y > ymax {
        c |= 1;
    } else if y < ymin {
        c |= 2;
    }
    if x > xmax {
        c |= 4;
    } else if x < xmin {
        c |= 8;
    }
    c
}

/// Cohen-Sutherland clip in single precision, truncating like the C casts.
pub fn clip_line(line: &mut [i32; 4], rect: PixelRect) -> bool {
    let [mut x0, mut y0, mut x1, mut y1] = line.map(|v| v as f32);
    let (xmin, ymin) = (rect.x as f32, rect.y as f32);
    let (xmax, ymax) = (
        rect.x as f32 + rect.w as f32 - 1.0,
        rect.y as f32 + rect.h as f32 - 1.0,
    );
    let mut c0 = outcode(x0, y0, xmin, ymin, xmax, ymax);
    let mut c1 = outcode(x1, y1, xmin, ymin, xmax, ymax);
    loop {
        if c0 | c1 == 0 {
            *line = [x0 as i32, y0 as i32, x1 as i32, y1 as i32];
            return true;
        }
        if c0 & c1 != 0 {
            return false;
        }
        let c = if c0 != 0 { c0 } else { c1 };
        let (x, y) = if c & 1 != 0 {
            (x0 + (x1 - x0) * (ymax - y0) / (y1 - y0), ymax)
        } else if c & 2 != 0 {
            (x0 + (x1 - x0) * (ymin - y0) / (y1 - y0), ymin)
        } else if c & 4 != 0 {
            (xmax, y0 + (y1 - y0) * (xmax - x0) / (x1 - x0))
        } else {
            (xmin, y0 + (y1 - y0) * (xmin - x0) / (x1 - x0))
        };
        if c == c0 {
            (x0, y0) = (x, y);
            c0 = outcode(x0, y0, xmin, ymin, xmax, ymax);
        } else {
            (x1, y1) = (x, y);
            c1 = outcode(x1, y1, xmin, ymin, xmax, ymax);
        }
    }
}

/// Bresenham line inside the canvas clip rectangle.
pub fn line(canvas: &mut Canvas, mut points: [i32; 4], color: u32, plot: Plot, factor: i32) {
    if !clip_line(&mut points, canvas.clip_rect()) {
        return;
    }
    let [x1, y1, x2, y2] = points;
    let (dx, dy) = (x2 - x1, y2 - y1);
    let (ax, ay) = (dx.abs() << 1, dy.abs() << 1);
    let (sx, sy) = (if dx < 0 { -1 } else { 1 }, if dy < 0 { -1 } else { 1 });
    let (mut x, mut y) = (x1, y1);
    if ax > ay {
        let mut d = ay - (ax >> 1);
        loop {
            plot.put(canvas, x, y, color, factor);
            if x == x2 {
                return;
            }
            if d >= 0 {
                y += sy;
                d -= ax;
            }
            x += sx;
            d += ay;
        }
    } else {
        let mut d = ax - (ay >> 1);
        loop {
            plot.put(canvas, x, y, color, factor);
            if y == y2 {
                return;
            }
            if d >= 0 {
                x += sx;
                d -= ay;
            }
            y += sy;
            d += ax;
        }
    }
}

/// Clip `rect` to `clip`; an empty result is zero-sized.
pub fn clip_rect(rect: &mut PixelRect, clip: PixelRect) -> bool {
    let (mut w, mut h) = (rect.w, rect.h);
    let dx = clip.x - rect.x;
    if dx > 0 {
        w -= dx;
        rect.x += dx;
    }
    let dx = rect.x + w - clip.x - clip.w;
    if dx > 0 {
        w -= dx;
    }
    let dy = clip.y - rect.y;
    if dy > 0 {
        h -= dy;
        rect.y += dy;
    }
    let dy = rect.y + h - clip.y - clip.h;
    if dy > 0 {
        h -= dy;
    }
    if w <= 0 || h <= 0 {
        rect.w = 0;
        rect.h = 0;
        return false;
    }
    rect.w = w;
    rect.h = h;
    true
}

pub fn fill_rect(canvas: &mut Canvas, mut rect: PixelRect, color: u32, plot: Plot, factor: i32) {
    if !clip_rect(&mut rect, canvas.clip_rect()) {
        return;
    }
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            plot.put(canvas, x, y, color, factor);
        }
    }
}

/// Clip a blit's source and destination to the destination clip rectangle.
pub fn clip_blit_rects(src: &mut PixelRect, dst: &mut PixelRect, clip: PixelRect) -> bool {
    let (mut w, mut h) = (src.w, src.h);
    if src.x < 0 {
        w += src.x;
        dst.x -= src.x;
        src.x = 0;
    }
    if src.y < 0 {
        h += src.y;
        dst.y -= src.y;
        src.y = 0;
    }
    let dx = clip.x - dst.x;
    if dx > 0 {
        w -= dx;
        dst.x += dx;
        src.x += dx;
    }
    let dx = dst.x + w - clip.x - clip.w;
    if dx > 0 {
        w -= dx;
    }
    let dy = clip.y - dst.y;
    if dy > 0 {
        h -= dy;
        dst.y += dy;
        src.y += dy;
    }
    let dy = dst.y + h - clip.y - clip.h;
    if dy > 0 {
        h -= dy;
    }
    if w <= 0 || h <= 0 {
        src.w = 0;
        src.h = 0;
        return false;
    }
    src.w = w;
    src.h = h;
    true
}

/// Blit with a custom draw kind, skipping transparent source pixels.
pub fn blit(
    src: &Canvas,
    mut src_r: PixelRect,
    plot: Plot,
    factor: i32,
    dst: &mut Canvas,
    mut dst_r: PixelRect,
) {
    if !clip_blit_rects(&mut src_r, &mut dst_r, dst.clip_rect()) {
        return;
    }
    if src_r.x >= src.width() || src_r.y >= src.height() {
        return;
    }
    src_r.w = src_r.w.min(src.width() - src_r.x);
    src_r.h = src_r.h.min(src.height() - src_r.y);
    let (mask, key) = if src.format().has_alpha() {
        (src.format().alpha_mask(), 0)
    } else if let Some(key) = src.color_key() {
        (u32::MAX, key)
    } else {
        (0, u32::MAX)
    };
    for y in 0..src_r.h {
        for x in 0..src_r.w {
            let pixel = src.pixel(src_r.x + x, src_r.y + y);
            let transparent = if src.is_indexed() {
                pixel == key
            } else {
                pixel & mask == key
            };
            if transparent {
                continue;
            }
            let c = src.get_rgba(pixel);
            let pixel = dst.map_rgba(c.r, c.g, c.b, c.a);
            plot.put(dst, dst_r.x + x, dst_r.y + y, pixel, factor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::PixelFormat;
    use super::*;

    #[test]
    fn small_canvas_lines_fills_and_blits_obey_clipping() {
        let mut dst = Canvas::new(PixelFormat::Rgbx, 5, 4);
        let mut src = Canvas::new(PixelFormat::Rgbx, 3, 2);
        dst.set_clip_rect(Some(PixelRect::new(1, 1, 3, 2)));
        let plot = Plot::for_canvas(&dst, 0).unwrap();
        line(&mut dst, [-2, 1, 6, 1], 5, plot, 0);
        fill_rect(&mut dst, PixelRect::new(2, 2, 5, 3), 7, plot, 0);
        for y in 0..4 {
            for x in 0..5 {
                let expected = if y == 1 && (1..=3).contains(&x) {
                    5
                } else if y == 2 && (2..=3).contains(&x) {
                    7
                } else {
                    0
                };
                assert_eq!(dst.pixel(x, y), expected);
            }
        }
        src.set_pixel(1, 0, 0x1234_5600);
        blit(
            &src,
            PixelRect::new(0, 0, 3, 2),
            plot,
            0,
            &mut dst,
            PixelRect::new(0, 1, 3, 2),
        );
        assert_eq!(dst.pixel(1, 1), 0x1234_5600);
        assert_eq!(dst.pixel(0, 1), 0);
    }

    #[test]
    fn additive_and_alpha_use_signed_shift_and_opaque_special_cases() {
        assert_eq!(channel(20, 200, 255, Plot::Additive), 220);
        assert_eq!(channel(20, 200, 128, Plot::Additive), 120);
        assert_eq!(channel(20, 200, -128, Plot::Additive), 0);
        assert_eq!(channel(200, 20, 128, Plot::Alpha), 110);
        assert_eq!(channel(20, 200, 128, Plot::Alpha), 110);
    }

    #[test]
    fn line_clipping_truncates_float_intersections() {
        let clip = PixelRect::new(1, 1, 4, 4);
        let mut points = [-3, 0, 8, 5];
        assert!(clip_line(&mut points, clip));
        assert_eq!(points, [1, 1, 4, 3]);
        let mut src = PixelRect::new(-2, -1, 5, 4);
        let mut dst = PixelRect::new(0, 0, 5, 4);
        assert!(clip_blit_rects(&mut src, &mut dst, clip));
        assert_eq!(
            (src.x, src.y, src.w, src.h, dst.x, dst.y),
            (0, 0, 3, 3, 2, 1)
        );
    }

    #[test]
    fn alpha_surfaces_only_take_replace() {
        let rgba = Canvas::new(PixelFormat::Rgba, 1, 1);
        assert_eq!(Plot::for_canvas(&rgba, 0), Some(Plot::Replace));
        assert_eq!(Plot::for_canvas(&rgba, 2), None);
        assert_eq!(
            Plot::for_canvas(&Canvas::new(PixelFormat::Indexed, 1, 1), 0),
            None
        );
    }
}
