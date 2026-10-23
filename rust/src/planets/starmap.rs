//! Star catalog lookup and cluster naming, ported from starmap.c.

#![allow(non_snake_case)]

use crate::comm::dispatch::CPoint;
use crate::savegame::runtime::star::{CStarDesc, NUM_HYPER_VORTICES, NUM_SOLAR_SYSTEMS};
use std::ptr;

#[no_mangle]
pub static mut star_array: *mut CStarDesc = ptr::null_mut();
#[no_mangle]
pub static mut CurStarDescPtr: *mut CStarDesc = ptr::null_mut();

unsafe fn find_star(
    base: *mut CStarDesc,
    last: *mut CStarDesc,
    point: CPoint,
    xb: i16,
    yb: i16,
    mut hi: i16,
) -> *mut CStarDesc {
    let mut lo = if last.is_null() {
        0
    } else {
        (last.offset_from(base) + 1) as i16
    };
    if !last.is_null() {
        if lo > hi {
            return ptr::null_mut();
        }
        hi = lo;
    }
    let (min_y, max_y) = if yb <= 0 {
        (point.y, point.y)
    } else {
        (point.y.wrapping_sub(yb), point.y.wrapping_add(yb))
    };
    while lo < hi {
        let mid = (lo + hi) >> 1;
        if (*base.offset(mid as isize)).star_pt.y >= min_y {
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
    }
    let mut at = base.offset(lo as isize);
    if yb < 0 || (*at).star_pt.y <= max_y {
        let (min_x, max_x) = if xb <= 0 {
            (point.x, point.x)
        } else {
            (point.x.wrapping_sub(xb), point.x.wrapping_add(xb))
        };
        loop {
            if (yb < 0 || (*at).star_pt.y >= min_y)
                && (xb < 0 || ((*at).star_pt.x >= min_x && (*at).star_pt.x <= max_x))
            {
                return at;
            }
            at = at.add(1);
            if (*at).star_pt.y > max_y {
                break;
            }
        }
    }
    ptr::null_mut()
}

/// # Safety
/// Catalog must be initialized; `last` is NULL or a catalog entry, `point` is readable.
#[no_mangle]
pub unsafe extern "C" fn FindStar(
    last: *mut CStarDesc,
    point: *mut CPoint,
    xb: i16,
    yb: i16,
) -> *mut CStarDesc {
    let (base, hi) = if crate::state::ffi::rust_get_game_state_bits(42, 43) <= 1 {
        (star_array, NUM_SOLAR_SYSTEMS as i16 - 1)
    } else {
        (
            star_array.add(NUM_SOLAR_SYSTEMS + 1),
            NUM_HYPER_VORTICES as i16,
        )
    };
    find_star(base, last, *point, xb, yb, hi)
}

#[cfg(not(test))]
mod naming {
    use super::*;
    use std::ffi::{c_char, c_void};
    extern "C" {
        static mut GameStrings: *mut c_void;
        fn SetAbsStringTableIndex(table: *mut c_void, index: u16) -> *mut c_void;
        fn GetStringAddress(entry: *mut c_void) -> *const c_char;
    }

    /// # Safety
    /// `star` is readable, and `buffer` has space for the prefix and cluster name.
    #[no_mangle]
    pub unsafe extern "C" fn GetClusterName(star: *const CStarDesc, buffer: *mut c_char) {
        let mut out = buffer;
        if (*star).prefix != 0 {
            let mut text = GetStringAddress(SetAbsStringTableIndex(
                GameStrings,
                crate::c_bindings::global_state::STAR_NUMBER_BASE as u16 + (*star).prefix as u16
                    - 1,
            ));
            if !text.is_null() {
                loop {
                    *out = *text;
                    out = out.add(1);
                    let end = *text == 0;
                    text = text.add(1);
                    if end {
                        break;
                    }
                }
                *out.sub(1) = b' ' as c_char;
            }
        }
        let mut text =
            GetStringAddress(SetAbsStringTableIndex(GameStrings, (*star).postfix as u16));
        if text.is_null() {
            *out = 0;
        } else {
            loop {
                *out = *text;
                out = out.add(1);
                let end = *text == 0;
                text = text.add(1);
                if end {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(x: i16, y: i16) -> CStarDesc {
        CStarDesc {
            star_pt: CPoint { x, y },
            star_type: 0,
            index: 0,
            prefix: 0,
            postfix: 0,
        }
    }

    #[test]
    fn bounds_are_inclusive_and_iteration_resumes_after_last_match() {
        let mut stars = [star(5, 10), star(8, 10), star(9, 20), star(9999, 32767)];
        let base = stars.as_mut_ptr();
        let point = CPoint { x: 7, y: 10 };
        unsafe {
            assert_eq!(find_star(base, ptr::null_mut(), point, 2, 0, 2), base);
            assert_eq!(find_star(base, base, point, 2, 0, 2), base.add(1));
            assert!(find_star(base, base.add(1), point, 2, 0, 2).is_null());
            assert!(find_star(base, base.add(2), point, -1, -1, 2).is_null());
            assert_eq!(
                find_star(base, ptr::null_mut(), CPoint { x: 9, y: 20 }, 0, 0, 2),
                base.add(2)
            );
            assert!(find_star(base, ptr::null_mut(), CPoint { x: 6, y: 10 }, 0, 0, 2).is_null());
        }
    }
}
