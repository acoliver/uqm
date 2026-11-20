//! Rectangle intersection, line clipping and the drawn-region bounding box,
//! ported from boxint.c, clipline.c and bbox.c (GPL-2.0-or-later).
use super::geometry::{Extent, Point, Rect};

pub const LEFT: u8 = 1;
pub const TOP: u8 = 2;
pub const RIGHT: u8 = 4;
pub const BOTTOM: u8 = 8;
pub const NOCLIP: u8 = 128;
const ALL_SIDES: u8 = LEFT | TOP | RIGHT | BOTTOM;

/// C: `BoxIntersect`: the overlap of `a` with `b` in `out` and the clip code,
/// zero when they do not overlap (`out` is then unchanged).
pub fn box_intersect(a: Rect, b: Rect, out: &mut Rect) -> u8 {
    let mut code = NOCLIP;
    let mut x = i32::from(a.corner.x) - i32::from(b.corner.x);
    let mut w = i32::from(a.extent.width);
    let delta = i32::from(b.extent.width) - x;
    if delta <= w {
        if delta != w {
            w = delta;
            code &= !NOCLIP;
        }
        code |= RIGHT;
    }
    if x <= 0 {
        if x < 0 {
            w += x;
            x = 0;
            code &= !NOCLIP;
        }
        code |= LEFT;
    }
    if w <= 0 {
        return 0;
    }
    let mut y = i32::from(a.corner.y) - i32::from(b.corner.y);
    let mut h = i32::from(a.extent.height);
    let delta = i32::from(b.extent.height) - y;
    if delta <= h {
        if delta != h {
            h = delta;
            code &= !NOCLIP;
        }
        code |= BOTTOM;
    }
    if y <= 0 {
        if y < 0 {
            h += y;
            y = 0;
            code &= !NOCLIP;
        }
        code |= TOP;
    }
    if h <= 0 {
        return 0;
    }
    *out = Rect {
        corner: Point::new(
            (x + i32::from(b.corner.x)) as i16,
            (y + i32::from(b.corner.y)) as i16,
        ),
        extent: Extent::new(w as i16, h as i16),
    };
    code
}

/// C: `BoxUnion`.
pub fn box_union(a: Rect, b: Rect) -> Rect {
    let x = i32::from(a.corner.x).min(i32::from(b.corner.x));
    let y = i32::from(a.corner.y).min(i32::from(b.corner.y));
    let right = (i32::from(a.corner.x) + i32::from(a.extent.width))
        .max(i32::from(b.corner.x) + i32::from(b.extent.width));
    let bottom = (i32::from(a.corner.y) + i32::from(a.extent.height))
        .max(i32::from(b.corner.y) + i32::from(b.extent.height));
    Rect::new(x as i16, y as i16, (right - x) as i16, (bottom - y) as i16)
}

/// A line prepared for Bresenham walking (C: `BRESENHAM_LINE`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BresenhamLine {
    pub first: Point,
    pub second: Point,
    pub abs_delta_x: i16,
    pub abs_delta_y: i16,
    pub error_term: i16,
    pub end_points_exchanged: bool,
    pub intersect_code: u8,
}

/// C: `_clip_line`: clip the line to `clip`, with the same half-pixel
/// rounding as the original line walker. Returns the clip code, zero when
/// the line misses.
pub fn clip_line(clip: Rect, l: &mut BresenhamLine) -> u8 {
    if clip.extent.width <= 0 || clip.extent.height <= 0 {
        return 0;
    }
    let (xmin, ymin) = (i32::from(clip.corner.x), i32::from(clip.corner.y));
    let (xmax, ymax) = (
        xmin + i32::from(clip.extent.width) - 1,
        ymin + i32::from(clip.extent.height) - 1,
    );
    if l.first.x > l.second.x {
        std::mem::swap(&mut l.first, &mut l.second);
        l.end_points_exchanged = true;
    } else {
        l.end_points_exchanged = false;
    }
    let (mut x0, mut y0) = (0i32, 0i32);
    let (mut x1, mut y1) = (
        i32::from(l.second.x) - i32::from(l.first.x),
        i32::from(l.second.y) - i32::from(l.first.y),
    );
    let dx = x1.abs() * 2;
    let dy = y1.abs() * 2;
    let down = y1 < 0;
    if i32::from(l.first.x) > xmax
        || i32::from(l.second.x) < xmin
        || (i32::from(l.first.y) > ymax && i32::from(l.second.y) > ymax)
        || (i32::from(l.first.y) < ymin && i32::from(l.second.y) < ymin)
    {
        return 0;
    }
    l.abs_delta_x = dx as i16;
    l.abs_delta_y = (if down { -dy } else { dy }) as i16;
    let mut code = NOCLIP;
    if dy == 0 {
        if i32::from(l.first.x) < xmin {
            l.first.x = xmin as i16;
            code |= LEFT;
        }
        if i32::from(l.second.x) > xmax {
            l.second.x = xmax as i16;
            code |= RIGHT;
        }
    } else if dx == 0 {
        if l.first.y > l.second.y {
            std::mem::swap(&mut l.first.y, &mut l.second.y);
            l.abs_delta_y = dy as i16;
        }
        if i32::from(l.first.y) < ymin {
            l.first.y = ymin as i16;
            code |= TOP;
        }
        if i32::from(l.second.y) > ymax {
            l.second.y = ymax as i16;
            code |= BOTTOM;
        }
    } else {
        let ox = i32::from(l.first.x);
        let oy = i32::from(l.first.y);
        x1 = i32::from(l.second.x) - ox;
        let (mut yminr, mut ymaxr) = if !down {
            (ymin - oy, ymax - oy)
        } else {
            (oy - ymin, oy - ymax)
        };
        let (xminr, xmaxr) = (xmin - ox, xmax - ox);
        if down {
            std::mem::swap(&mut yminr, &mut ymaxr);
        }
        if dx > dy {
            let half = dx >> 1;
            if x0 < xminr {
                x0 = xminr;
                y0 = (dy * x0 + half) / dx;
                if y0 > ymaxr {
                    return 0;
                }
                code |= LEFT;
            }
            if x1 > xmaxr {
                x1 = xmaxr;
                y1 = (dy * x1 + half) / dx;
                if y1 < yminr {
                    return 0;
                }
                code |= RIGHT;
            }
            if y0 < yminr {
                y0 = yminr;
                x0 = (dx * y0 - half + (dy - 1)) / dy;
                if x0 > xmaxr {
                    return 0;
                }
                code |= TOP;
                code &= !LEFT;
            }
            if y1 > ymaxr {
                y1 = ymaxr;
                x1 = (dx * (y1 + 1) - half + (dy - 1)) / dy - 1;
                if x1 < xminr {
                    return 0;
                }
                code |= BOTTOM;
                code &= !RIGHT;
            }
        } else {
            let half = dy >> 1;
            if y0 < yminr {
                y0 = yminr;
                x0 = (dx * y0 + half) / dy;
                if x0 > xmaxr {
                    return 0;
                }
                code |= TOP;
            }
            if y1 > ymaxr {
                y1 = ymaxr;
                x1 = (dx * y1 + half) / dy;
                if x1 < xminr {
                    return 0;
                }
                code |= BOTTOM;
            }
            if x0 < xminr {
                x0 = xminr;
                y0 = (dy * x0 - half + (dx - 1)) / dx;
                if y0 > ymaxr {
                    return 0;
                }
                code |= LEFT;
                code &= !TOP;
            }
            if x1 > xmaxr {
                x1 = xmaxr;
                y1 = (dy * (x1 + 1) - half + (dx - 1)) / dx - 1;
                if y1 < yminr {
                    return 0;
                }
                code |= RIGHT;
                code &= !BOTTOM;
            }
        }
        l.first.x = (ox + x0) as i16;
        l.second.x = (ox + x1) as i16;
        l.first.y = (if down { oy - y0 } else { oy + y0 }) as i16;
        l.second.y = (if down { oy - y1 } else { oy + y1 }) as i16;
        if down {
            let sides = code & (TOP | BOTTOM);
            if sides != 0 && sides != (TOP | BOTTOM) {
                code ^= TOP | BOTTOM;
            }
        }
    }
    if code & ALL_SIDES == 0 {
        l.error_term = (-(dx.max(dy) >> 1)) as i16;
    } else {
        code &= !NOCLIP;
        l.error_term = if dx > dy {
            (x0 * dy - y0 * dx - (dx >> 1)) as i16
        } else {
            (y0 * dx - x0 * dy - (dy >> 1)) as i16
        };
    }
    l.intersect_code = code;
    code
}

/// The region of the main screen drawn since the last flush.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BoundingBox {
    pub valid: bool,
    pub region: Rect,
    pub clip: Rect,
    max: Extent,
}

impl BoundingBox {
    pub const fn new() -> Self {
        Self {
            valid: false,
            region: Rect::new(0, 0, 0, 0),
            clip: Rect::new(0, 0, 0, 0),
            max: Extent::new(0, 0),
        }
    }

    /// C: `TFB_BBox_Init`.
    pub fn init(&mut self, width: i32, height: i32) {
        self.max = Extent::new(width as i16, height as i16);
        self.clip.extent = self.max;
    }

    /// C: `TFB_BBox_Reset`.
    pub fn reset(&mut self) {
        self.valid = false;
    }

    /// C: `TFB_BBox_SetClipRect`.
    pub fn set_clip(&mut self, rect: Option<Rect>) {
        let Some(rect) = rect else {
            self.clip = Rect {
                corner: Point::default(),
                extent: self.max,
            };
            return;
        };
        let (max_w, max_h) = (i32::from(self.max.width), i32::from(self.max.height));
        self.clip = rect;
        self.clip.corner.x = self.clip.corner.x.max(0);
        self.clip.corner.y = self.clip.corner.y.max(0);
        if i32::from(self.clip.corner.x) + i32::from(self.clip.extent.width) > max_w {
            self.clip.extent.width = (max_w - i32::from(self.clip.corner.x)) as i16;
        }
        if i32::from(self.clip.corner.y) + i32::from(self.clip.extent.height) > max_h {
            self.clip.extent.height = (max_h - i32::from(self.clip.corner.y)) as i16;
        }
    }

    /// C: `TFB_BBox_RegisterPoint`.
    pub fn register_point(&mut self, x: i32, y: i32) {
        let x1 = i32::from(self.clip.corner.x);
        let y1 = i32::from(self.clip.corner.y);
        let x2 = x1 + i32::from(self.clip.extent.width) - 1;
        let y2 = y1 + i32::from(self.clip.extent.height) - 1;
        let x = x.max(x1).min(x2);
        let y = y.max(y1).min(y2);
        if !self.valid {
            self.valid = true;
            self.region = Rect::new(x as i16, y as i16, 1, 1);
            return;
        }
        let r = &mut self.region;
        let (rx, ry) = (i32::from(r.corner.x), i32::from(r.corner.y));
        let (ex, ey) = (i32::from(r.extent.width), i32::from(r.extent.height));
        let far_x = rx + ex - 1;
        let far_y = ry + ey - 1;
        if x < rx {
            r.corner.x = x as i16;
            r.extent.width = (ex + rx - x) as i16;
        }
        if y < ry {
            r.corner.y = y as i16;
            r.extent.height = (ey + ry - y) as i16;
        }
        if x > far_x {
            r.extent.width = (i32::from(r.extent.width) + x - far_x) as i16;
        }
        if y > far_y {
            r.extent.height = (i32::from(r.extent.height) + y - far_y) as i16;
        }
    }

    /// C: `TFB_BBox_RegisterRect`.
    pub fn register_rect(&mut self, r: Rect) {
        self.register_point(i32::from(r.corner.x), i32::from(r.corner.y));
        self.register_point(
            i32::from(r.corner.x) + i32::from(r.extent.width) - 1,
            i32::from(r.corner.y) + i32::from(r.extent.height) - 1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_intersect_reports_clipped_sides() {
        let mut out = Rect::default();
        let code = box_intersect(Rect::new(-2, 1, 5, 2), Rect::new(0, 0, 4, 4), &mut out);
        assert_eq!(out, Rect::new(0, 1, 3, 2));
        assert_eq!(code, LEFT);
        assert_eq!(
            box_intersect(Rect::new(9, 9, 1, 1), Rect::new(0, 0, 4, 4), &mut out),
            0
        );
        assert_eq!(
            box_union(Rect::new(0, 0, 2, 2), Rect::new(3, 1, 1, 4)),
            Rect::new(0, 0, 4, 5)
        );
    }

    #[test]
    fn bounding_box_grows_within_its_clip() {
        let mut bbox = BoundingBox::new();
        bbox.init(10, 10);
        bbox.register_rect(Rect::new(2, 3, 2, 2));
        bbox.register_point(-5, 20);
        assert!(bbox.valid);
        assert_eq!(bbox.region, Rect::new(0, 3, 4, 7));
        bbox.set_clip(Some(Rect::new(-1, 4, 20, 2)));
        assert_eq!(bbox.clip, Rect::new(0, 4, 10, 2));
    }

    #[test]
    fn clipped_lines_keep_half_pixel_rounding() {
        let mut line = BresenhamLine {
            first: Point::new(-4, 0),
            second: Point::new(8, 6),
            ..BresenhamLine::default()
        };
        let code = clip_line(Rect::new(0, 0, 5, 5), &mut line);
        assert_eq!(code & (LEFT | RIGHT | BOTTOM), LEFT | RIGHT);
        assert_eq!(
            (line.first, line.second),
            (Point::new(0, 2), Point::new(4, 4))
        );
    }
}
