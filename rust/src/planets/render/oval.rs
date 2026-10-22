//! `oval.c`'s midpoint ellipses, including its quadrant culling and batch order.
//!
//! Coordinates and extents retain the C signed 16-bit narrowing. The decision
//! terms use 64 bits, matching `long` on the supported native targets. Clipping
//! is a graphics dependency: the linked adapter must use `_clip_line`, whose
//! Bresenham rounding decides whether a quadrant is submitted at all.

use crate::comm::locdata::{CPoint, CRect};

/// The graphics operations consumed by the oval leaf. Batch entries are in
/// submission order, including repeated axis points and repeated centre rows.
pub trait OvalGfx {
    fn viewport(&self) -> CRect;
    fn line_visible(&mut self, viewport: &CRect, first: CPoint, second: CPoint) -> bool;
    fn draw_line(&mut self, first: CPoint, second: CPoint);
    fn points(&mut self, points: &[CPoint]);
    fn filled_rows(&mut self, rows: &[CRect; 2]);
}

fn corners(rect: &CRect) -> (CPoint, CPoint) {
    (
        rect.corner,
        CPoint {
            x: (i32::from(rect.corner.x) + i32::from(rect.width) - 1) as i16,
            y: (i32::from(rect.corner.y) + i32::from(rect.height) - 1) as i16,
        },
    )
}

fn degenerate(rect: &CRect, gfx: &mut impl OvalGfx) -> bool {
    let (first, mut second) = corners(rect);
    if second.x <= first.x || second.y <= first.y {
        second.x = second.x.max(first.x);
        second.y = second.y.max(first.y);
        gfx.draw_line(first, second);
        true
    } else {
        false
    }
}

fn quadrants(rect: &CRect, gfx: &mut impl OvalGfx) -> u8 {
    let (first, second) = corners(rect);
    let middle = CPoint {
        x: ((i32::from(first.x) + i32::from(second.x)) >> 1) as i16,
        y: ((i32::from(first.y) + i32::from(second.y)) >> 1) as i16,
    };
    let viewport = gfx.viewport();
    let mut visible = 0;
    for (quad, x, y, end_y) in [
        (0, second.x, first.y, middle.y),
        (1, first.x, first.y, middle.y),
        (2, first.x, second.y, middle.y),
        (3, second.x, second.y, middle.y),
    ] {
        if (y >= 0 && y < viewport.height && x >= 0 && x < viewport.width)
            || gfx.line_visible(&viewport, CPoint { x: middle.x, y }, CPoint { x, y: end_y })
        {
            visible |= 1 << quad;
        }
    }
    visible
}

struct Ellipse {
    x: i16,
    y: i16,
    center: CPoint,
    a_squared: i64,
    b_squared: i64,
    dx: i64,
    dy: i64,
    d: i64,
}

impl Ellipse {
    fn new(rect: &CRect) -> Self {
        let a = rect.width >> 1;
        let b = rect.height >> 1;
        let a_squared = i64::from(a) * i64::from(a);
        let b_squared = i64::from(b) * i64::from(b);
        let dy = 2 * a_squared * i64::from(b);
        Self {
            x: 0,
            y: b,
            center: CPoint {
                x: a.wrapping_add(rect.corner.x),
                y: b.wrapping_add(rect.corner.y),
            },
            a_squared,
            b_squared,
            dx: 0,
            dy,
            d: b_squared - (dy >> 1) + (a_squared >> 2),
        }
    }

    fn step_x(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.dx += 2 * self.b_squared;
    }

    fn step_y(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.dy -= 2 * self.a_squared;
    }

    fn switch_region(&mut self) {
        self.d += ((((self.a_squared - self.b_squared) * 3) >> 1) - (self.dx + self.dy)) >> 1;
    }

    fn points(&self, visible: u8, gfx: &mut impl OvalGfx) {
        let right = self.center.x.wrapping_add(self.x);
        let left = self.center.x.wrapping_sub(self.x);
        let top = self.center.y.wrapping_sub(self.y);
        let bottom = self.center.y.wrapping_add(self.y);
        let all = [
            CPoint { x: right, y: top },
            CPoint { x: left, y: top },
            CPoint { x: left, y: bottom },
            CPoint {
                x: right,
                y: bottom,
            },
        ];
        let mut points = [CPoint::default(); 4];
        let mut count = 0;
        // SetPrimNextLink prepends each visible quadrant to the C list.
        for quad in (0..4).rev() {
            if visible & (1 << quad) != 0 {
                points[count] = all[quad];
                count += 1;
            }
        }
        gfx.points(&points[..count]);
    }

    fn rows(&self, gfx: &mut impl OvalGfx) {
        let row = CRect {
            corner: CPoint {
                x: self.center.x.wrapping_sub(self.x),
                y: self.center.y.wrapping_add(self.y),
            },
            width: ((i32::from(self.x) << 1) + 1) as i16,
            height: 1,
        };
        gfx.filled_rows(&[
            row,
            CRect {
                corner: CPoint {
                    y: self.center.y.wrapping_sub(self.y),
                    ..row.corner
                },
                ..row
            },
        ]);
    }
}

/// Outline with one submitted step followed by `num_off_pixels` skipped steps.
/// This preserves the original even-extent asymmetry and duplicate axis points.
pub fn draw_oval(rect: &CRect, num_off_pixels: u8, gfx: &mut impl OvalGfx) {
    if degenerate(rect, gfx) {
        return;
    }
    let visible = quadrants(rect, gfx);
    if visible == 0 {
        return;
    }
    let mut ellipse = Ellipse::new(rect);
    let mut off = 0u16;
    let mut submit = |ellipse: &Ellipse, gfx: &mut _| {
        let draw = off == 0;
        off = off.wrapping_sub(1);
        if draw {
            ellipse.points(visible, gfx);
            off = u16::from(num_off_pixels);
        }
    };
    while ellipse.dx < ellipse.dy {
        submit(&ellipse, gfx);
        if ellipse.d > 0 {
            ellipse.step_y();
            ellipse.d -= ellipse.dy;
        }
        ellipse.step_x();
        ellipse.d += ellipse.b_squared + ellipse.dx;
    }
    ellipse.switch_region();
    while ellipse.y >= 0 {
        submit(&ellipse, gfx);
        if ellipse.d < 0 {
            ellipse.step_x();
            ellipse.d += ellipse.dx;
        }
        ellipse.step_y();
        ellipse.d += ellipse.a_squared - ellipse.dy;
    }
}

/// Filled ellipse as paired single-height rectangles, bottom row first.
/// The filled variant does not perform the outline's quadrant culling.
pub fn draw_filled_oval(rect: &CRect, gfx: &mut impl OvalGfx) {
    if degenerate(rect, gfx) {
        return;
    }
    let mut ellipse = Ellipse::new(rect);
    while ellipse.dx < ellipse.dy {
        if ellipse.d > 0 {
            ellipse.rows(gfx);
            ellipse.step_y();
            ellipse.d -= ellipse.dy;
        }
        ellipse.step_x();
        ellipse.d += ellipse.b_squared + ellipse.dx;
    }
    ellipse.switch_region();
    while ellipse.y >= 0 {
        ellipse.rows(gfx);
        if ellipse.d < 0 {
            ellipse.step_x();
            ellipse.d += ellipse.dx;
        }
        ellipse.step_y();
        ellipse.d += ellipse.a_squared - ellipse.dy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recording {
        batches: Vec<Vec<(i16, i16)>>,
        rows: Vec<Vec<(i16, i16, i16, i16)>>,
        lines: Vec<((i16, i16), (i16, i16))>,
        clips: Vec<((i16, i16), (i16, i16))>,
        visible: bool,
    }

    impl OvalGfx for Recording {
        fn viewport(&self) -> CRect {
            CRect {
                width: 242,
                height: 227,
                ..CRect::default()
            }
        }
        fn line_visible(&mut self, _: &CRect, first: CPoint, second: CPoint) -> bool {
            self.clips.push(((first.x, first.y), (second.x, second.y)));
            self.visible
        }
        fn draw_line(&mut self, first: CPoint, second: CPoint) {
            self.lines.push(((first.x, first.y), (second.x, second.y)));
        }
        fn points(&mut self, points: &[CPoint]) {
            self.batches
                .push(points.iter().map(|p| (p.x, p.y)).collect());
        }
        fn filled_rows(&mut self, rows: &[CRect; 2]) {
            self.rows.push(
                rows.iter()
                    .map(|r| (r.corner.x, r.corner.y, r.width, r.height))
                    .collect(),
            );
        }
    }

    fn rect(x: i16, y: i16, width: i16, height: i16) -> CRect {
        CRect {
            corner: CPoint { x, y },
            width,
            height,
        }
    }

    #[test]
    fn degenerates_draw_one_clamped_line_without_clipping() {
        for (width, height, end) in [
            (0, 0, (5, 7)),
            (-8, 9, (5, 15)),
            (9, -8, (13, 7)),
            (1, 9, (5, 15)),
        ] {
            for filled in [false, true] {
                let mut gfx = Recording::default();
                let r = rect(5, 7, width, height);
                if filled {
                    draw_filled_oval(&r, &mut gfx);
                } else {
                    draw_oval(&r, 255, &mut gfx);
                }
                assert_eq!(gfx.lines, [((5, 7), end)]);
                assert!(gfx.batches.is_empty() && gfx.rows.is_empty() && gfx.clips.is_empty());
            }
        }
    }

    #[test]
    fn quadrant_order_and_axis_duplicates_are_preserved() {
        let mut gfx = Recording::default();
        draw_oval(&rect(10, 20, 5, 5), 0, &mut gfx);
        assert_eq!(gfx.batches[0], [(12, 24), (12, 24), (12, 20), (12, 20)]);
        assert_eq!(
            gfx.batches.last().unwrap(),
            &[(14, 22), (10, 22), (10, 22), (14, 22)]
        );
        assert!(gfx.clips.is_empty());
    }

    #[test]
    fn dotted_cadence_continues_across_regions() {
        let r = rect(20, 20, 32, 17);
        let mut solid = Recording::default();
        draw_oval(&r, 0, &mut solid);
        for skip in [1, 2, 7, 255] {
            let mut dotted = Recording::default();
            draw_oval(&r, skip, &mut dotted);
            assert_eq!(
                dotted.batches,
                solid
                    .batches
                    .iter()
                    .step_by(usize::from(skip) + 1)
                    .cloned()
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn invisible_quadrants_stop_before_emitting_points() {
        let mut gfx = Recording::default();
        draw_oval(&rect(-50, -50, 20, 20), 0, &mut gfx);
        assert_eq!(gfx.clips.len(), 4);
        assert!(gfx.batches.is_empty());
    }

    #[test]
    fn clipping_keeps_only_the_visible_quadrants() {
        let mut gfx = Recording::default();
        draw_oval(&rect(-2, 20, 5, 5), 0, &mut gfx);
        assert_eq!(gfx.clips, [((0, 20), (-2, 22)), ((0, 24), (-2, 22))]);
        assert_eq!(gfx.batches[0], [(0, 24), (0, 20)]);
    }

    #[test]
    fn filled_rows_include_duplicate_centre_and_ignore_quadrant_clip() {
        let mut gfx = Recording::default();
        draw_filled_oval(&rect(10, 20, 5, 5), &mut gfx);
        assert_eq!(gfx.rows.last().unwrap(), &[(10, 22, 5, 1), (10, 22, 5, 1)]);
        assert!(gfx.clips.is_empty());
        let mut outside = Recording::default();
        draw_filled_oval(&rect(-100, -100, 5, 5), &mut outside);
        assert!(!outside.rows.is_empty());
    }

    #[test]
    fn signed_coordinate_narrowing_matches_the_c_degenerate_branch() {
        let mut gfx = Recording::default();
        draw_oval(&rect(i16::MAX - 1, 7, 8, 5), 0, &mut gfx);
        assert_eq!(gfx.lines, [((i16::MAX - 1, 7), (i16::MAX - 1, 11))]);
    }
}
