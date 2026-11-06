//! Three live parallax star planes, with the original sort and wrap order.

#[cfg(not(test))]
use crate::battle::context_access::ContextOps;
use crate::ships::native::abi::{Color, POINT};

pub use crate::engine::battle::kernel::StarPlane as STAR_BLOCK;

fn projected_coords(point: POINT, dx: i16, dy: i16, reduction: u16, step: bool) -> (i32, i32) {
    let x = i32::from(point.x) - i32::from(dx);
    let y = i32::from(point.y) - i32::from(dy);
    if step {
        (x >> reduction, y >> reduction)
    } else {
        (
            (x << 8) / i32::from(reduction),
            (y << 8) / i32::from(reduction),
        )
    }
}

fn project(point: POINT, dx: i16, dy: i16, reduction: u16, step: bool) -> POINT {
    let (x, y) = projected_coords(point, dx, dy, reduction, step);
    POINT {
        x: x as i16,
        y: y as i16,
    }
}

fn wrap_x(x: i16, width: i32) -> i16 {
    let x = i32::from(x);
    (if x < 0 {
        x + width
    } else if x >= width {
        x - width
    } else {
        x
    }) as i16
}

const fn color(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

impl STAR_BLOCK {
    pub(crate) fn sort(&mut self) {
        for i in 0..self.points.len() {
            for j in (i + 1..self.points.len()).rev() {
                if self.points[i].y > self.points[j].y {
                    self.points.swap(i, j);
                }
            }
        }
        self.min_star_index = 0;
    }

    pub(crate) fn translate(&mut self, index: usize, dx: i16, dy: i16) {
        let point = &mut self.points[index];
        point.y = point.y.wrapping_add(dy);
        point.x = point.x.wrapping_add(dx);
    }

    pub(crate) fn wrap(&mut self, plane: i16, dx: i16, mut dy: i16, log_height: i32) {
        let mut i = self.min_star_index as usize;
        let mut point = i as isize;
        let count = self.points.len();
        if dy < 0 {
            let first = i;
            dy = dy.wrapping_neg();
            let offset = ((log_height << plane) - i32::from(dy)) as i16;
            while self.points[point as usize].y < dy {
                self.translate(point as usize, dx, offset);
                i += 1;
                if i < count {
                    point += 1;
                } else {
                    i = 0;
                    point = 0;
                }
                if i == first {
                    return;
                }
            }
            self.min_star_index = i as u16;
            if first <= i {
                i = count - i;
                loop {
                    self.translate(point as usize, dx, dy.wrapping_neg());
                    point += 1;
                    i -= 1;
                    if i == 0 {
                        break;
                    }
                }
                point = 0;
            }
            if first > i {
                i = first - i;
                loop {
                    self.translate(point as usize, dx, dy.wrapping_neg());
                    point += 1;
                    i -= 1;
                    if i == 0 {
                        break;
                    }
                }
            }
        } else {
            point -= 1;
            if i == 0 {
                i = count - 1;
                point = i as isize;
            } else {
                i -= 1;
            }
            let last = i;
            if dy > 0 {
                let offset = ((log_height << plane) - i32::from(dy)) as i16;
                while self.points[point as usize].y >= offset {
                    self.translate(point as usize, dx, offset.wrapping_neg());
                    if i > 0 {
                        i -= 1;
                        point -= 1;
                    } else {
                        i = count - 1;
                        point = i as isize;
                    }
                    if i == last {
                        return;
                    }
                }
                self.min_star_index = (i + 1) as u16;
                if usize::from(self.min_star_index) == self.points.len() {
                    self.min_star_index = 0;
                }
            }
            if last >= i {
                i += 1;
                loop {
                    self.translate(point as usize, dx, dy);
                    point -= 1;
                    i -= 1;
                    if i == 0 {
                        break;
                    }
                }
                i = count - 1;
                point = i as isize;
            }
            if last < i {
                i -= last;
                loop {
                    self.translate(point as usize, dx, dy);
                    point -= 1;
                    i -= 1;
                    if i == 0 {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::engine::battle::Cx;
    use crate::ships::native::abi::*;

    const COUNTS: [usize; 3] = [30, 60, 90];

    pub fn init_galaxy(cx: &mut Cx<'_>) {
        let (screen_width, screen_height) = cx.screen_dimensions();
        let stars = crate::engine::battle::frames::FrameSelection::retain(cx.kernel.stars_in_space);
        let mut factor = ONE_SHIFT + MAX_REDUCTION + (BACKGROUND_SHIFT - 3);
        let mut p = 0;
        for (plane, count) in COUNTS.into_iter().enumerate() {
            for index in 0..count {
                assert_eq!(cx.display_pool().alloc(), Some(p as u16));
                if p == 30 || p == 90 {
                    factor += 1;
                }
                let point = POINT {
                    x: ((cx.random() as u16 as i32 % (screen_width - 64)) as i16)
                        .wrapping_shl(factor),
                    y: ((cx.random() as u16 as i32 % screen_height) as i16).wrapping_shl(factor),
                };
                cx.kernel.star_planes[plane].points[index] = point;
                cx.kernel.star_planes[plane].displayed[index] = POINT { x: 0, y: 0 };
                let tint = if p < 90 {
                    color(11, 11, 31)
                } else if cx.world.activity() as u8 != 3 {
                    color(21, 21, 21)
                } else if cx
                    .world
                    .story_state(crate::engine::StoryKey::ARILOU_SPACE_SIDE)
                    <= 1
                {
                    color(20, 0, 0)
                } else {
                    color(0, 14, 0)
                };
                let primitive = cx.display_primitive(p as u16);
                primitive.kind = if p < 90 {
                    gfx_object_STAMP_PRIM
                } else {
                    gfx_object_POINT_PRIM
                } as u8;
                primitive.color = tint;
                if p < 90 {
                    primitive.set_frame(stars.clone());
                    cx.kernel.star_planes[plane].frames[index] = stars.clone();
                }
                p += 1;
            }
            cx.kernel.star_planes[plane].sort();
        }
    }

    /// # Safety
    /// InitGalaxy must have completed; all display primitives and frames remain live.
    fn change_star_frames(cx: &mut Cx<'_>, step: bool, reduction: u16) {
        let star_frame =
            crate::engine::battle::frames::FrameSelection::retain(cx.kernel.stars_in_space);
        if cx.world.activity() as u8 == 3 {
            let mut p = 0;
            for (plane, offset) in [58, 26].into_iter().enumerate() {
                for index in 0..COUNTS[plane] {
                    let random = cx.random();
                    let frame = cx.absolute_frame(star_frame.view(), (random & 31) as u16 + offset);
                    cx.display_primitive(p as u16).set_frame(frame.clone());
                    cx.kernel.star_planes[plane].frames[index] = frame;
                    p += 1;
                }
            }
        } else {
            let mut frames = [cx.advance_frame(star_frame.view()), star_frame];
            let types = if step {
                if reduction > 0 {
                    frames[0] = frames[1].clone();
                    [gfx_object_STAMP_PRIM, gfx_object_POINT_PRIM]
                } else {
                    [gfx_object_STAMP_PRIM; 2]
                }
            } else if reduction > 1 << ZOOM_SHIFT {
                [gfx_object_POINT_PRIM; 2]
            } else {
                [gfx_object_STAMP_PRIM, gfx_object_POINT_PRIM]
            };
            let mut p = 0;
            for plane in 0..2 {
                for index in 0..COUNTS[plane] {
                    cx.display_primitive(p as u16).kind = types[plane] as u8;
                    cx.display_primitive(p as u16)
                        .set_frame(frames[plane].clone());
                    cx.kernel.star_planes[plane].frames[index] = frames[plane].clone();
                    p += 1;
                }
            }
        }
    }

    pub fn move_galaxy(cx: &mut Cx<'_>, view: u32, mut dx: i16, mut dy: i16, scale: i32) {
        let (screen_width, screen_height) = cx.screen_dimensions();
        let width = (screen_width - 64) << (ONE_SHIFT + MAX_REDUCTION);
        let height = screen_height << (ONE_SHIFT + MAX_REDUCTION);
        // VIEW_STABLE=0, VIEW_CHANGE=2; TFB_SCALE_STEP=0.
        if view != 0 {
            let mut reduction = cx.b.zoom_out as u16;
            let step = scale == 0;
            if view == 2 {
                change_star_frames(cx, step, reduction);
            }
            if cx.world.activity() as u8 == 3 {
                for (plane, count) in COUNTS.into_iter().enumerate().take(2) {
                    let offset = if plane == 0 { 0 } else { 30 };
                    for index in 0..count {
                        let frame = cx.kernel.star_planes[plane].frames[index].clone();
                        let base = cx.frame_number(frame.view()).wrapping_sub(26);
                        let frame = cx.absolute_frame(
                            frame.view(),
                            ((base & !31) + (base.wrapping_add(1) & 31)).wrapping_add(26),
                        );
                        cx.display_primitive((offset + index) as u16)
                            .set_frame(frame.clone());
                        cx.kernel.star_planes[plane].frames[index] = frame;
                    }
                }
                dx = dx.wrapping_shl(3);
                dy = dy.wrapping_shl(3);
            }
            for plane in (0..3).rev() {
                cx.kernel.star_planes[plane].wrap(plane as i16, dx, dy, height);
            }
            if cx.world.activity() as u8 != 3 {
                dx = cx.kernel.space_origin.x;
                dy = cx.kernel.space_origin.y;
                if step {
                    reduction = reduction.wrapping_add(ONE_SHIFT as u16);
                } else {
                    reduction = reduction.wrapping_shl(ONE_SHIFT);
                }
            } else {
                dx = (((width >> 1) as i16 as i32)
                    - (width >> ((MAX_REDUCTION + 1) - MAX_VIS_REDUCTION)))
                    as i16;
                dy = (((height >> 1) as i16 as i32)
                    - (height >> ((MAX_REDUCTION + 1) - MAX_VIS_REDUCTION)))
                    as i16;
                reduction = if step {
                    (MAX_VIS_REDUCTION + ONE_SHIFT) as u16
                } else {
                    (MAX_ZOOM_OUT << ONE_SHIFT) as u16
                };
            }
            let mut p = 0;
            let mut wrap = width;
            for (plane, count) in COUNTS.into_iter().enumerate() {
                let projected = projected_coords(
                    cx.kernel.star_planes[plane].points[0],
                    dx,
                    dy,
                    reduction,
                    step,
                );
                let displayed = cx.kernel.star_planes[plane].displayed[0];
                if view != 2
                    && i32::from(displayed.x) == projected.0
                    && i32::from(displayed.y) == projected.1
                {
                    break;
                }
                for index in 0..count {
                    let plane = &mut cx.kernel.star_planes[plane];
                    plane.points[index].x = wrap_x(plane.points[index].x, wrap);
                    let point = project(plane.points[index], dx, dy, reduction, step);
                    plane.displayed[index] = point;
                    cx.display_primitive(p as u16).set_point(point);
                    p += 1;
                }
                if step {
                    reduction = reduction.wrapping_add(1);
                } else {
                    reduction = reduction.wrapping_shl(1);
                }
                wrap <<= 1;
                dx = dx.wrapping_shl(1);
                dy = dy.wrapping_shl(1);
            }
        }
        cx.display_pool().background_order(180);
    }
}

#[cfg(not(test))]
pub use engine::{init_galaxy, move_galaxy};

#[cfg(test)]
mod tests {
    use super::*;
    fn block(points: &[POINT]) -> STAR_BLOCK {
        let mut plane = STAR_BLOCK::new(points.len());
        plane.points.copy_from_slice(points);
        plane
    }

    #[test]
    fn sort_preserves_the_original_equal_y_swap_order() {
        let mut points = [
            POINT { x: 0, y: 5 },
            POINT { x: 1, y: 5 },
            POINT { x: 2, y: 1 },
        ];
        {
            let mut plane = block(&points);
            plane.sort();
            points.copy_from_slice(&plane.points);
        }
        assert_eq!(points.map(|p| p.x), [2, 1, 0]);
    }

    #[test]
    fn vertical_wrap_rotates_the_sorted_start_both_directions() {
        let mut points = [
            POINT { x: 0, y: 1 },
            POINT { x: 1, y: 4 },
            POINT { x: 2, y: 9 },
        ];
        let mut block = block(&points);
        {
            block.sort();
            block.wrap(0, 3, -2, 10);
            points.copy_from_slice(&block.points);
            assert_eq!(points.map(|p| (p.x, p.y)), [(3, 9), (4, 2), (5, 7)]);
            assert_eq!(block.min_star_index, 1);
            block.wrap(0, -3, 2, 10);
            points.copy_from_slice(&block.points);
            assert_eq!(points.map(|p| (p.x, p.y)), [(0, 1), (1, 4), (2, 9)]);
            assert_eq!(block.min_star_index, 0);
            block.wrap(0, 1, 0, 10);
            points.copy_from_slice(&block.points);
            assert_eq!(points.map(|p| p.x), [1, 2, 3]);
        }
    }

    #[test]
    fn repeated_wraps_match_each_points_translation_across_cyclic_segments() {
        for plane in 0..3 {
            let height = 10 << plane;
            let mut points = [
                POINT { x: 0, y: 0 },
                POINT { x: 1, y: 2 },
                POINT { x: 2, y: 5 },
                POINT { x: 3, y: 9 },
            ];
            let mut expected = points;
            let mut block = block(&points);
            {
                block.sort();
                for dy in [-3, 1, 8, -7, 0, 2, -1] {
                    block.wrap(plane, 3, dy, 10);
                    points.copy_from_slice(&block.points);
                    for point in &mut expected {
                        point.x += 3;
                        point.y = (point.y + dy).rem_euclid(height);
                    }
                    assert_eq!(points.map(|p| (p.x, p.y)), expected.map(|p| (p.x, p.y)));
                }
            }
        }
    }

    #[test]
    fn projection_preserves_shift_vs_signed_division_rounding() {
        let p = POINT { x: -1, y: 15 };
        let step = project(p, 0, 0, 2, true);
        let smooth = project(p, 0, 0, 1024, false);
        assert_eq!((step.x, step.y), (-1, 3));
        assert_eq!((smooth.x, smooth.y), (0, 3));
        assert_eq!(wrap_x(-1, 8192), 8191);
        assert_eq!(wrap_x(8192, 8192), 0);
        assert_eq!(color(11, 11, 31).b, 255);
    }
}
