//! Three live parallax star planes, with the original sort and wrap order.

use crate::ships::native::abi::{Color, POINT};

#[repr(C)]
pub struct STAR_BLOCK {
    min_star_index: u16,
    num_stars: u16,
    star_array: *mut POINT,
    pmin_star: *mut POINT,
    plast_star: *mut POINT,
}

impl STAR_BLOCK {
    unsafe fn sort(&mut self) {
        for i in 0..self.num_stars as usize {
            for j in (i + 1..self.num_stars as usize).rev() {
                if (*self.star_array.add(i)).y > (*self.star_array.add(j)).y {
                    std::ptr::swap(self.star_array.add(i), self.star_array.add(j));
                }
            }
        }
        self.min_star_index = 0;
        self.pmin_star = self.star_array;
        self.plast_star = self.star_array.add(self.num_stars as usize - 1);
    }

    unsafe fn translate(&mut self, index: usize, dx: i16, dy: i16) {
        let point = &mut *self.star_array.add(index);
        point.y = point.y.wrapping_add(dy);
        point.x = point.x.wrapping_add(dx);
    }

    unsafe fn wrap(&mut self, plane: i16, dx: i16, mut dy: i16, log_height: i32) {
        let mut i = self.min_star_index as usize;
        let mut point = i as isize;
        let count = self.num_stars as usize;
        if dy < 0 {
            let first = i;
            dy = dy.wrapping_neg();
            let offset = ((log_height << plane) - i32::from(dy)) as i16;
            while (*self.star_array.offset(point)).y < dy {
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
            self.pmin_star = self.star_array.offset(point);
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
                while (*self.star_array.offset(point)).y >= offset {
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
                self.pmin_star = self.star_array.offset(point + 1);
                self.min_star_index = (i + 1) as u16;
                if self.min_star_index == self.num_stars {
                    self.min_star_index = 0;
                    self.pmin_star = self.star_array;
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

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::*;
    use crate::ships::native::{abi::*, frame_index};
    use std::ptr;

    const COUNTS: [usize; 3] = [30, 60, 90];
    const ZERO: POINT = POINT { x: 0, y: 0 };
    static mut LOG_STARS: [POINT; 180] = [ZERO; 180];
    #[no_mangle]
    pub static mut SpaceOrg: POINT = ZERO;
    pub static mut STAR_BLOCK: [super::STAR_BLOCK; 3] = [
        super::STAR_BLOCK {
            min_star_index: 0,
            num_stars: 30,
            star_array: (&raw mut LOG_STARS).cast::<POINT>(),
            pmin_star: ptr::null_mut(),
            plast_star: ptr::null_mut(),
        },
        super::STAR_BLOCK {
            min_star_index: 0,
            num_stars: 60,
            star_array: unsafe { (&raw mut LOG_STARS).cast::<POINT>().add(30) },
            pmin_star: ptr::null_mut(),
            plast_star: ptr::null_mut(),
        },
        super::STAR_BLOCK {
            min_star_index: 0,
            num_stars: 90,
            star_array: unsafe { (&raw mut LOG_STARS).cast::<POINT>().add(90) },
            pmin_star: ptr::null_mut(),
            plast_star: ptr::null_mut(),
        },
    ];
    extern "C" {
        static mut zoom_out: i16;
    }

    use crate::engine::battle::legacy::{DisplayFreeList, DisplayLinks};

    unsafe fn prim(index: usize) -> *mut PRIMITIVE {
        (&raw mut DisplayArray).cast::<PRIMITIVE>().add(index)
    }
    unsafe fn stars(index: usize) -> *mut POINT {
        (&raw mut LOG_STARS).cast::<POINT>().add(index)
    }
    unsafe fn blocks(index: usize) -> *mut super::STAR_BLOCK {
        (&raw mut STAR_BLOCK).cast::<super::STAR_BLOCK>().add(index)
    }
    unsafe fn width() -> i32 {
        (ScreenWidth - 64) << (ONE_SHIFT + MAX_REDUCTION)
    }
    unsafe fn height() -> i32 {
        ScreenHeight << (ONE_SHIFT + MAX_REDUCTION)
    }

    /// # Safety
    /// Star frames and primitive free list must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn InitGalaxy() {
        let mut links = u32::MAX;
        let mut factor = ONE_SHIFT + MAX_REDUCTION + (BACKGROUND_SHIFT - 3);
        for i in 0..180 {
            let p = DisplayFreeList;
            DisplayFreeList = ((*prim(p as usize)).Links >> 16) as u16;
            if i == 30 || i == 90 {
                factor += 1;
            }
            (*stars(i)).x =
                ((TFB_Random() as u16 as i32 % (ScreenWidth - 64)) as i16).wrapping_shl(factor);
            (*stars(i)).y =
                ((TFB_Random() as u16 as i32 % ScreenHeight) as i16).wrapping_shl(factor);
            if i < 90 {
                (*prim(p as usize)).Type = gfx_object_STAMP_PRIM as u8;
                (*prim(p as usize)).color = color(11, 11, 31);
                (*prim(p as usize)).Object.Stamp.frame = stars_in_space;
            } else {
                (*prim(p as usize)).Type = gfx_object_POINT_PRIM as u8;
                (*prim(p as usize)).color = if inHQSpace() == 0 {
                    color(21, 21, 21)
                } else if crate::state::ffi::rust_get_game_state(c"ARILOU_SPACE_SIDE".as_ptr()) <= 1
                {
                    color(20, 0, 0)
                } else {
                    color(0, 14, 0)
                };
            }
            let previous = links as u16;
            InsertPrim(&mut links, p, previous);
        }
        for i in 0..3 {
            (*blocks(i)).sort();
        }
    }

    /// # Safety
    /// InitGalaxy must have completed; all display primitives and frames remain live.
    #[no_mangle]
    pub unsafe extern "C" fn MoveGalaxy(view: u32, mut dx: i16, mut dy: i16) {
        // VIEW_STABLE=0, VIEW_CHANGE=2; TFB_SCALE_STEP=0.
        if view != 0 {
            let mut reduction = zoom_out as u16;
            let step = optMeleeScale == 0;
            if view == 2 {
                if inHQSpace() != 0 {
                    let mut p = 0;
                    for (plane, offset) in [58, 26].into_iter().enumerate() {
                        for _ in 0..COUNTS[plane] {
                            (*prim(p)).Object.Stamp.frame = SetAbsFrameIndex(
                                stars_in_space,
                                (TFB_Random() & 31) as u16 + offset,
                            );
                            p += 1;
                        }
                    }
                } else {
                    let mut frames = [IncFrameIndex(stars_in_space), stars_in_space];
                    let types = if step {
                        if reduction > 0 {
                            frames[0] = frames[1];
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
                        for _ in 0..COUNTS[plane] {
                            (*prim(p)).Type = types[plane] as u8;
                            (*prim(p)).Object.Stamp.frame = frames[plane];
                            p += 1;
                        }
                    }
                }
            }
            if inHQSpace() != 0 {
                for i in 0..90 {
                    let frame = (*prim(i)).Object.Stamp.frame;
                    let base = frame_index(frame).wrapping_sub(26);
                    (*prim(i)).Object.Stamp.frame = SetAbsFrameIndex(
                        frame,
                        ((base & !31) + (base.wrapping_add(1) & 31)).wrapping_add(26),
                    );
                }
                dx = dx.wrapping_shl(3);
                dy = dy.wrapping_shl(3);
            }
            for plane in (0..3).rev() {
                (*blocks(plane)).wrap(plane as i16, dx, dy, height());
            }
            if inHQSpace() == 0 {
                dx = SpaceOrg.x;
                dy = SpaceOrg.y;
                if step {
                    reduction = reduction.wrapping_add(ONE_SHIFT as u16);
                } else {
                    reduction = reduction.wrapping_shl(ONE_SHIFT);
                }
            } else {
                dx = (((width() >> 1) as i16 as i32)
                    - (width() >> ((MAX_REDUCTION + 1) - MAX_VIS_REDUCTION)))
                    as i16;
                dy = (((height() >> 1) as i16 as i32)
                    - (height() >> ((MAX_REDUCTION + 1) - MAX_VIS_REDUCTION)))
                    as i16;
                reduction = if step {
                    (MAX_VIS_REDUCTION + ONE_SHIFT) as u16
                } else {
                    (MAX_ZOOM_OUT << ONE_SHIFT) as u16
                };
            }
            let mut p = 0;
            let mut wrap = width();
            for count in COUNTS {
                let projected = projected_coords(*stars(p), dx, dy, reduction, step);
                let displayed = (*prim(p)).Object.Point;
                if view != 2
                    && i32::from(displayed.x) == projected.0
                    && i32::from(displayed.y) == projected.1
                {
                    break;
                }
                for _ in 0..count {
                    (*stars(p)).x = wrap_x((*stars(p)).x, wrap);
                    (*prim(p)).Object.Point = project(*stars(p), dx, dy, reduction, step);
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
        DisplayLinks = 180 - 1;
    }
}

#[cfg(not(test))]
pub use engine::{InitGalaxy, MoveGalaxy, SpaceOrg};

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    fn block(points: &mut [POINT]) -> STAR_BLOCK {
        STAR_BLOCK {
            min_star_index: 0,
            num_stars: points.len() as u16,
            star_array: points.as_mut_ptr(),
            pmin_star: ptr::null_mut(),
            plast_star: ptr::null_mut(),
        }
    }

    #[test]
    fn sort_preserves_the_original_equal_y_swap_order() {
        let mut points = [
            POINT { x: 0, y: 5 },
            POINT { x: 1, y: 5 },
            POINT { x: 2, y: 1 },
        ];
        unsafe {
            block(&mut points).sort();
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
        let mut block = block(&mut points);
        unsafe {
            block.sort();
            block.wrap(0, 3, -2, 10);
            assert_eq!(points.map(|p| (p.x, p.y)), [(3, 9), (4, 2), (5, 7)]);
            assert_eq!(block.min_star_index, 1);
            block.wrap(0, -3, 2, 10);
            assert_eq!(points.map(|p| (p.x, p.y)), [(0, 1), (1, 4), (2, 9)]);
            assert_eq!(block.min_star_index, 0);
            block.wrap(0, 1, 0, 10);
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
            let mut block = block(&mut points);
            unsafe {
                block.sort();
                for dy in [-3, 1, 8, -7, 0, 2, -1] {
                    block.wrap(plane, 3, dy, 10);
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
