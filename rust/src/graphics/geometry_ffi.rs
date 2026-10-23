//! Rust implementations of the legacy graphics geometry ABI.

use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Extent {
    pub width: i16,
    pub height: i16,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Rect {
    pub corner: Point,
    pub extent: Extent,
}
#[repr(C)]
pub struct Bbox {
    pub valid: i32,
    pub region: Rect,
    pub clip: Rect,
}
#[repr(C)]
pub struct BresenhamLine {
    pub first: Point,
    pub second: Point,
    pub abs_delta_x: i16,
    pub abs_delta_y: i16,
    pub error_term: i16,
    pub end_points_exchanged: u8,
    pub intersect_code: u8,
}

const LEFT: u8 = 1;
const TOP: u8 = 2;
const RIGHT: u8 = 4;
const BOTTOM: u8 = 8;
const NOCLIP: u8 = 128;
const ALL_SIDES: u8 = LEFT | TOP | RIGHT | BOTTOM;

pub static mut TFB_B_BOX: Bbox = Bbox {
    valid: 0,
    region: Rect {
        corner: Point { x: 0, y: 0 },
        extent: Extent {
            width: 0,
            height: 0,
        },
    },
    clip: Rect {
        corner: Point { x: 0, y: 0 },
        extent: Extent {
            width: 0,
            height: 0,
        },
    },
};
static mut MAX_WIDTH: i32 = 0;
static mut MAX_HEIGHT: i32 = 0;

/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn TFB_BBox_Init(width: i32, height: i32) {
    MAX_WIDTH = width;
    MAX_HEIGHT = height;
    (*std::ptr::addr_of_mut!(TFB_B_BOX)).clip.extent = Extent {
        width: width as i16,
        height: height as i16,
    };
}
/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn TFB_BBox_Reset() {
    (*std::ptr::addr_of_mut!(TFB_B_BOX)).valid = 0;
}
/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn TFB_BBox_SetClipRect(r: *const Rect) {
    let b = &mut *std::ptr::addr_of_mut!(TFB_B_BOX);
    if r.is_null() {
        b.clip = Rect {
            corner: Point { x: 0, y: 0 },
            extent: Extent {
                width: MAX_WIDTH as i16,
                height: MAX_HEIGHT as i16,
            },
        };
        return;
    }
    b.clip = *r;
    b.clip.corner.x = b.clip.corner.x.max(0);
    b.clip.corner.y = b.clip.corner.y.max(0);
    if i32::from(b.clip.corner.x) + i32::from(b.clip.extent.width) > MAX_WIDTH {
        b.clip.extent.width = (MAX_WIDTH - i32::from(b.clip.corner.x)) as i16;
    }
    if i32::from(b.clip.corner.y) + i32::from(b.clip.extent.height) > MAX_HEIGHT {
        b.clip.extent.height = (MAX_HEIGHT - i32::from(b.clip.corner.y)) as i16;
    }
}
/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn TFB_BBox_RegisterPoint(x: i32, y: i32) {
    let b = &mut *std::ptr::addr_of_mut!(TFB_B_BOX);
    let mut x = x;
    let mut y = y;
    let x1 = i32::from(b.clip.corner.x);
    let y1 = i32::from(b.clip.corner.y);
    let x2 = x1 + i32::from(b.clip.extent.width) - 1;
    let y2 = y1 + i32::from(b.clip.extent.height) - 1;
    x = x.max(x1).min(x2);
    y = y.max(y1).min(y2);
    if b.valid == 0 {
        b.valid = 1;
        b.region = Rect {
            corner: Point {
                x: x as i16,
                y: y as i16,
            },
            extent: Extent {
                width: 1,
                height: 1,
            },
        };
        return;
    }
    let (rx, ry) = (i32::from(b.region.corner.x), i32::from(b.region.corner.y));
    let (ex, ey) = (
        i32::from(b.region.extent.width),
        i32::from(b.region.extent.height),
    );
    let far_x = rx + ex - 1;
    let far_y = ry + ey - 1;
    if x < rx {
        b.region.corner.x = x as i16;
        b.region.extent.width = (ex + rx - x) as i16;
    }
    if y < ry {
        b.region.corner.y = y as i16;
        b.region.extent.height = (ey + ry - y) as i16;
    }
    if x > far_x {
        b.region.extent.width = (i32::from(b.region.extent.width) + x - far_x) as i16;
    }
    if y > far_y {
        b.region.extent.height = (i32::from(b.region.extent.height) + y - far_y) as i16;
    }
}
/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn TFB_BBox_RegisterRect(r: *const Rect) {
    let r = *r;
    TFB_BBox_RegisterPoint(i32::from(r.corner.x), i32::from(r.corner.y));
    TFB_BBox_RegisterPoint(
        i32::from(r.corner.x) + i32::from(r.extent.width) - 1,
        i32::from(r.corner.y) + i32::from(r.extent.height) - 1,
    );
}
extern "C" {
    fn TFB_DrawCanvas_GetExtent(canvas: *mut c_void, size: *mut Extent);
}
/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn TFB_BBox_RegisterCanvas(canvas: *mut c_void, x: i32, y: i32) {
    let mut r = Rect {
        corner: Point {
            x: x as i16,
            y: y as i16,
        },
        extent: Extent::default(),
    };
    TFB_DrawCanvas_GetExtent(canvas, &mut r.extent);
    TFB_BBox_RegisterRect(&r);
}

/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn BoxIntersect(a: *mut Rect, b: *mut Rect, out: *mut Rect) -> u8 {
    let (a, b) = (*a, *b);
    let mut code = NOCLIP;
    let mut x = i32::from(a.corner.x) - i32::from(b.corner.x);
    let mut w = i32::from(a.extent.width);
    let w2 = i32::from(b.extent.width);
    let delta = w2 - x;
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
    let h2 = i32::from(b.extent.height);
    let delta = h2 - y;
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
        corner: Point {
            x: (x + i32::from(b.corner.x)) as i16,
            y: (y + i32::from(b.corner.y)) as i16,
        },
        extent: Extent {
            width: w as i16,
            height: h as i16,
        },
    };
    code
}
/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn BoxUnion(a: *mut Rect, b: *mut Rect, out: *mut Rect) {
    let (a, b) = (*a, *b);
    let x = i32::from(a.corner.x).min(i32::from(b.corner.x));
    let y = i32::from(a.corner.y).min(i32::from(b.corner.y));
    *out = Rect {
        corner: Point {
            x: x as i16,
            y: y as i16,
        },
        extent: Extent {
            width: ((i32::from(a.corner.x) + i32::from(a.extent.width))
                .max(i32::from(b.corner.x) + i32::from(b.extent.width))
                - x) as i16,
            height: ((i32::from(a.corner.y) + i32::from(a.extent.height))
                .max(i32::from(b.corner.y) + i32::from(b.extent.height))
                - y) as i16,
        },
    };
}

// Bresenham clipping uses the same half-pixel rounding as the original line walker.
/// # Safety
/// Pointer arguments must be valid for the duration of the call; graphics ABI
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn _clip_line(clip: *const Rect, line: *mut BresenhamLine) -> u8 {
    let clip = *clip;
    let l = &mut *line;
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
        l.end_points_exchanged = 1;
    } else {
        l.end_points_exchanged = 0;
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

#[repr(C)]
pub struct FrameDesc {
    kind: u16,
    index: u16,
    hot_spot: Point,
    bounds: Extent,
    image: *mut c_void,
    parent: *mut c_void,
}
#[repr(C)]
pub struct Stamp {
    origin: Point,
    frame: *mut FrameDesc,
}
#[repr(C)]
pub struct IntersectControl {
    last_time_val: u16,
    end_point: Point,
    stamp: Stamp,
}

extern "C" {
    static mut _GraphicsStatusFlags: u8;
    fn TFB_DrawImage_Intersect(
        img1: *mut c_void,
        img1org: Point,
        img2: *mut c_void,
        img2org: Point,
        rect: *const Rect,
    ) -> u8;
}

#[path = "intersection.rs"]
mod intersection;

/// # Safety
/// Controls and their frames must be valid for the duration of the call;
/// callers must obey the single graphics-thread access contract.
#[no_mangle]
pub unsafe extern "C" fn DrawablesIntersect(
    a: *mut IntersectControl,
    b: *mut IntersectControl,
    max_time: u16,
) -> u16 {
    intersection::drawables_intersect(
        a,
        b,
        max_time,
        _GraphicsStatusFlags & 4 != 0,
        |image0, origin0, image1, origin1, rect| {
            TFB_DrawImage_Intersect(image0, origin0, image1, origin1, rect) != 0
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn box_intersection_clips_edges_and_rejects_empty() {
        unsafe {
            let mut a = Rect {
                corner: Point { x: -2, y: 1 },
                extent: Extent {
                    width: 5,
                    height: 4,
                },
            };
            let mut b = Rect {
                corner: Point { x: 0, y: 0 },
                extent: Extent {
                    width: 3,
                    height: 3,
                },
            };
            let mut out = Rect::default();
            assert_eq!(
                BoxIntersect(&mut a, &mut b, &mut out),
                LEFT | RIGHT | BOTTOM
            );
            assert_eq!(
                out,
                Rect {
                    corner: Point { x: 0, y: 1 },
                    extent: Extent {
                        width: 3,
                        height: 2
                    }
                }
            );
            a.extent.width = 0;
            assert_eq!(BoxIntersect(&mut a, &mut b, &mut out), 0);
        }
    }
    #[test]
    fn union_handles_negative_origins() {
        unsafe {
            let mut a = Rect {
                corner: Point { x: -3, y: 2 },
                extent: Extent {
                    width: 4,
                    height: 2,
                },
            };
            let mut b = Rect {
                corner: Point { x: 0, y: -2 },
                extent: Extent {
                    width: 2,
                    height: 5,
                },
            };
            let mut out = Rect::default();
            BoxUnion(&mut a, &mut b, &mut out);
            assert_eq!(
                out,
                Rect {
                    corner: Point { x: -3, y: -2 },
                    extent: Extent {
                        width: 5,
                        height: 6
                    }
                }
            );
        }
    }
    #[test]
    fn line_clipping_keeps_edge_endpoints() {
        unsafe {
            let clip = Rect {
                corner: Point { x: 0, y: 0 },
                extent: Extent {
                    width: 8,
                    height: 8,
                },
            };
            let mut line = BresenhamLine {
                first: Point { x: -3, y: 3 },
                second: Point { x: 7, y: 3 },
                abs_delta_x: 0,
                abs_delta_y: 0,
                error_term: 0,
                end_points_exchanged: 0,
                intersect_code: 0,
            };
            assert_eq!(_clip_line(&clip, &mut line), LEFT);
            assert_eq!(line.first, Point { x: 0, y: 3 });
            assert_eq!(line.second, Point { x: 7, y: 3 });
        }
    }
}
