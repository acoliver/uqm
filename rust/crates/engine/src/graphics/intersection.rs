//! Fixed-point collision walker, ported from intersec.c (GPL-2.0-or-later).
use super::drawable::FrameId;
use super::geometry::{Extent, HotSpot, Point, Rect};
use super::image::ImageId;
use super::state;

/// A stamp: a frame at an origin (C: `STAMP`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stamp {
    pub origin: Point,
    pub frame: FrameId,
}

/// A moving stamp's sweep for collision tests (C: `INTERSECT_CONTROL`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct IntersectControl {
    pub last_time_val: u16,
    pub EndPoint: Point,
    pub IntersectStamp: Stamp,
}

/// What the sweep reads of a stamp's frame.
#[derive(Clone, Copy, Debug)]
struct Shape {
    hot_spot: HotSpot,
    bounds: Extent,
    image: Option<ImageId>,
}

const TIME_SHIFT: u32 = 8;
const TIME_SCALE: i16 = 1 << TIME_SHIFT;
const MAX_TIME_VALUE: u16 = TIME_SCALE as u16 + 1;

type ImageIntersect<'a> =
    dyn FnMut(Option<ImageId>, Point, Option<ImageId>, Point, &Rect) -> bool + 'a;

// SIZE and COUNT in the C algorithm are signed/unsigned 16-bit. Wrapping
// operations preserve assignment narrowing, including the fast-forward COUNT.
fn frame_intersect(
    (control0, frame0, r0): (&mut IntersectControl, &Shape, &mut Rect),
    (control1, frame1, r1): (&mut IntersectControl, &Shape, &mut Rect),
    (mut t0, t1): (u16, u16),
    images_intersect: &mut ImageIntersect<'_>,
) -> u16 {
    let mut box0 = Rect {
        corner: r0.corner,
        extent: frame0.bounds,
    };
    let mut box1 = Rect {
        corner: r1.corner,
        extent: frame1.bounds,
    };

    let mut dx_0 = r0.extent.width;
    let mut dy_0 = r0.extent.height;
    let xincr0 = if dx_0 >= 0 {
        1i16
    } else {
        dx_0 = dx_0.wrapping_neg();
        -1
    };
    let yincr0 = if dy_0 >= 0 {
        1i16
    } else {
        dy_0 = dy_0.wrapping_neg();
        -1
    };
    let cycle0 = dx_0.max(dy_0);
    let mut xerror0 = cycle0;
    let mut yerror0 = cycle0;

    let mut dx_1 = r1.extent.width;
    let mut dy_1 = r1.extent.height;
    let xincr1 = if dx_1 >= 0 {
        1i16
    } else {
        dx_1 = dx_1.wrapping_neg();
        -1
    };
    let yincr1 = if dy_1 >= 0 {
        1i16
    } else {
        dy_1 = dy_1.wrapping_neg();
        -1
    };
    let cycle1 = dx_1.max(dy_1);
    let mut xerror1 = cycle1;
    let mut yerror1 = cycle1;

    let mut check0 = false;
    let mut check1 = false;
    let mut time_error0;
    let mut time_error1;
    let mut check_first = false;
    if t0 <= 1 {
        time_error0 = 0i16;
        time_error1 = 0i16;
        if t0 == 0 {
            t0 += 1;
            // C jumps directly to CheckFirstIntersection, bypassing last_time_val.
            check_first = true;
        }
    } else {
        let mut start = (cycle0 as u16).wrapping_mul(t0 - 1);
        time_error0 = (start & (TIME_SCALE as u16 - 1)) as i16;
        start >>= TIME_SHIFT;
        if start > 0 {
            let error = i64::from(xerror0) - i64::from(dx_0) * i64::from(start);
            if error > 0 {
                xerror0 = error as i16;
            } else {
                let delta = (error / i64::from(cycle0)) as i16;
                let delta = delta.wrapping_neg().wrapping_add(1);
                box0.corner.x = box0.corner.x.wrapping_add(xincr0.wrapping_mul(delta));
                xerror0 = (error + i64::from(cycle0) * i64::from(delta)) as i16;
            }
            let error = i64::from(yerror0) - i64::from(dy_0) * i64::from(start);
            if error > 0 {
                yerror0 = error as i16;
            } else {
                let delta = (error / i64::from(cycle0)) as i16;
                let delta = delta.wrapping_neg().wrapping_add(1);
                box0.corner.y = box0.corner.y.wrapping_add(yincr0.wrapping_mul(delta));
                yerror0 = (error + i64::from(cycle0) * i64::from(delta)) as i16;
            }
            r0.corner = box0.corner;
        }

        let mut start = (cycle1 as u16).wrapping_mul(t0 - 1);
        time_error1 = (start & (TIME_SCALE as u16 - 1)) as i16;
        start >>= TIME_SHIFT;
        if start > 0 {
            let error = i64::from(xerror1) - i64::from(dx_1) * i64::from(start);
            if error > 0 {
                xerror1 = error as i16;
            } else {
                let delta = (error / i64::from(cycle1)) as i16;
                let delta = delta.wrapping_neg().wrapping_add(1);
                box1.corner.x = box1.corner.x.wrapping_add(xincr1.wrapping_mul(delta));
                xerror1 = (error + i64::from(cycle1) * i64::from(delta)) as i16;
            }
            let error = i64::from(yerror1) - i64::from(dy_1) * i64::from(start);
            if error > 0 {
                yerror1 = error as i16;
            } else {
                let delta = (error / i64::from(cycle1)) as i16;
                let delta = delta.wrapping_neg().wrapping_add(1);
                box1.corner.y = box1.corner.y.wrapping_add(yincr1.wrapping_mul(delta));
                yerror1 = (error + i64::from(cycle1) * i64::from(delta)) as i16;
            }
            r1.corner = box1.corner;
        }
    }

    if !check_first {
        control0.last_time_val = t0;
        control1.last_time_val = t0;
    }
    loop {
        if !check_first {
            t0 = t0.wrapping_add(1);
            time_error0 = time_error0.wrapping_add(cycle0);
            if time_error0 >= TIME_SCALE {
                xerror0 = xerror0.wrapping_sub(dx_0);
                if xerror0 <= 0 {
                    box0.corner.x = box0.corner.x.wrapping_add(xincr0);
                    xerror0 = xerror0.wrapping_add(cycle0);
                }
                yerror0 = yerror0.wrapping_sub(dy_0);
                if yerror0 <= 0 {
                    box0.corner.y = box0.corner.y.wrapping_add(yincr0);
                    yerror0 = yerror0.wrapping_add(cycle0);
                }
                check0 = true;
                time_error0 = time_error0.wrapping_sub(TIME_SCALE);
            }

            time_error1 = time_error1.wrapping_add(cycle1);
            if time_error1 >= TIME_SCALE {
                xerror1 = xerror1.wrapping_sub(dx_1);
                if xerror1 <= 0 {
                    box1.corner.x = box1.corner.x.wrapping_add(xincr1);
                    xerror1 = xerror1.wrapping_add(cycle1);
                }
                yerror1 = yerror1.wrapping_sub(dy_1);
                if yerror1 <= 0 {
                    box1.corner.y = box1.corner.y.wrapping_add(yincr1);
                    yerror1 = yerror1.wrapping_add(cycle1);
                }
                check1 = true;
                time_error1 = time_error1.wrapping_sub(TIME_SCALE);
            }
        }

        if check_first || check0 || check1 {
            let mut overlap = Rect::default();
            if super::bbox::box_intersect(box0, box1, &mut overlap) != 0
                && images_intersect(
                    frame0.image,
                    box0.corner,
                    frame1.image,
                    box1.corner,
                    &overlap,
                )
            {
                return t0;
            }
            if check0 {
                r0.corner = box0.corner;
                control0.last_time_val = t0;
                check0 = false;
            }
            if check1 {
                r1.corner = box1.corner;
                control1.last_time_val = t0;
                check1 = false;
            }
        }
        check_first = false;
        if t0 > t1 {
            break;
        }
    }
    0
}

/// C: `DrawablesIntersect`: the first tick (1-based) within `max_time` at
/// which the two sweeping stamps' pixels overlap, or zero; on a hit both
/// end points move to the last positions before it.
pub fn drawables_intersect(
    control0: &mut IntersectControl,
    control1: &mut IntersectControl,
    max_time: u16,
) -> u16 {
    state::with(|g| {
        let shape = |frame: FrameId| {
            (!frame.is_null()).then(|| {
                let frame = g.frame(frame);
                Shape {
                    hot_spot: frame.hot_spot,
                    bounds: frame.bounds,
                    image: frame.image,
                }
            })
        };
        let shapes = (
            shape(control0.IntersectStamp.frame),
            shape(control1.IntersectStamp.frame),
        );
        let active = g.context_active();
        sweep(
            control0,
            control1,
            shapes,
            max_time,
            active,
            &mut |a, ao, b, bo, area| {
                let image = |id: Option<ImageId>| {
                    id.unwrap_or_else(|| panic!("collision frame without an image"))
                };
                super::image::intersect(&g.images, image(a), ao, image(b), bo, *area)
            },
        )
    })
}

fn sweep(
    control0: &mut IntersectControl,
    control1: &mut IntersectControl,
    shapes: (Option<Shape>, Option<Shape>),
    mut max_time: u16,
    context_active: bool,
    images_intersect: &mut ImageIntersect<'_>,
) -> u16 {
    if !context_active || max_time == 0 {
        return 0;
    }
    if max_time > MAX_TIME_VALUE {
        max_time = MAX_TIME_VALUE;
    }

    control0.last_time_val = 0;
    control1.last_time_val = 0;
    let mut r0 = Rect {
        corner: control0.IntersectStamp.origin,
        extent: Extent::default(),
    };
    let mut r1 = Rect {
        corner: control1.IntersectStamp.origin,
        extent: Extent::default(),
    };
    r0.extent.width = control0.EndPoint.x.wrapping_sub(r0.corner.x);
    r0.extent.height = control0.EndPoint.y.wrapping_sub(r0.corner.y);
    r1.extent.width = control1.EndPoint.x.wrapping_sub(r1.corner.x);
    r1.extent.height = control1.EndPoint.y.wrapping_sub(r1.corner.y);

    let Some(frame0) = shapes.0 else {
        return 0;
    };
    r0.corner.x = r0.corner.x.wrapping_sub(frame0.hot_spot.x);
    r0.corner.y = r0.corner.y.wrapping_sub(frame0.hot_spot.y);
    let Some(frame1) = shapes.1 else {
        return 0;
    };
    r1.corner.x = r1.corner.x.wrapping_sub(frame1.hot_spot.x);
    r1.corner.y = r1.corner.y.wrapping_sub(frame1.hot_spot.y);

    let mut dy = r1.corner.y.wrapping_sub(r0.corner.y);
    let mut time_y_0 = dy.wrapping_sub(frame0.bounds.height).wrapping_add(1);
    let mut time_y_1 = dy.wrapping_add(frame1.bounds.height).wrapping_sub(1);
    dy = r0.extent.height.wrapping_sub(r1.extent.height);
    if (time_y_0 <= 0 && time_y_1 >= 0)
        || (time_y_0 > 0 && dy >= time_y_0)
        || (time_y_1 < 0 && dy <= time_y_1)
    {
        let mut dx = r1.corner.x.wrapping_sub(r0.corner.x);
        let mut time_x_0 = dx.wrapping_sub(frame0.bounds.width).wrapping_add(1);
        let mut time_x_1 = dx.wrapping_add(frame1.bounds.width).wrapping_sub(1);
        dx = r0.extent.width.wrapping_sub(r1.extent.width);
        if (time_x_0 <= 0 && time_x_1 >= 0)
            || (time_x_0 > 0 && dx >= time_x_0)
            || (time_x_1 < 0 && dx <= time_x_1)
        {
            if dx == 0 && dy == 0 {
                time_y_0 = 0;
                time_y_1 = 0;
            } else {
                if time_y_1 < 0 {
                    let t = time_y_0;
                    time_y_0 = time_y_1.wrapping_neg();
                    time_y_1 = t.wrapping_neg();
                } else if time_y_0 <= 0 {
                    if dy < 0 {
                        time_y_1 = time_y_0.wrapping_neg();
                    }
                    time_y_0 = 0;
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                if dy < time_y_1 {
                    time_y_1 = dy;
                }
                time_y_0 = time_y_0.wrapping_sub(1);
                time_y_1 = time_y_1.wrapping_add(1);

                if time_x_1 < 0 {
                    let t = time_x_0;
                    time_x_0 = time_x_1.wrapping_neg();
                    time_x_1 = t.wrapping_neg();
                } else if time_x_0 <= 0 {
                    if dx < 0 {
                        time_x_1 = time_x_0.wrapping_neg();
                    }
                    time_x_0 = 0;
                }
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dx < time_x_1 {
                    time_x_1 = dx;
                }
                time_x_0 = time_x_0.wrapping_sub(1);
                time_x_1 = time_x_1.wrapping_add(1);

                let (mut time_beg, mut time_end, fract);
                if dx == 0 {
                    time_beg = i64::from(time_y_0);
                    time_end = i64::from(time_y_1);
                    fract = i64::from(dy);
                } else if dy == 0 {
                    time_beg = i64::from(time_x_0);
                    time_end = i64::from(time_x_1);
                    fract = i64::from(dx);
                } else {
                    let time_x = i64::from(time_x_0) * i64::from(dy);
                    let time_y = i64::from(time_y_0) * i64::from(dx);
                    time_beg = time_x.max(time_y);
                    let time_x = i64::from(time_x_1) * i64::from(dy);
                    let time_y = i64::from(time_y_1) * i64::from(dx);
                    time_end = time_x.min(time_y);
                    fract = i64::from(dx) * i64::from(dy);
                }
                time_beg <<= TIME_SHIFT;
                time_y_0 = if time_beg < fract {
                    0
                } else {
                    (time_beg / fract) as i16
                };
                if time_end >= fract {
                    time_y_1 = (max_time - 1) as i16;
                } else {
                    time_end <<= TIME_SHIFT;
                    time_y_1 = if time_end >= fract * i64::from(max_time) {
                        (max_time - 1) as i16
                    } else {
                        (((time_end + fract - 1) / fract) as i16).wrapping_sub(1)
                    };
                }
            }

            if time_y_0 <= time_y_1 {
                let intersect_time = frame_intersect(
                    (control0, &frame0, &mut r0),
                    (control1, &frame1, &mut r1),
                    (time_y_0 as u16, time_y_1 as u16),
                    images_intersect,
                );
                if intersect_time != 0 {
                    control0.EndPoint.x = r0.corner.x.wrapping_add(frame0.hot_spot.x);
                    control0.EndPoint.y = r0.corner.y.wrapping_add(frame0.hot_spot.y);
                    control1.EndPoint.x = r1.corner.x.wrapping_add(frame1.hot_spot.x);
                    control1.EndPoint.y = r1.corner.y.wrapping_add(frame1.hot_spot.y);
                    return intersect_time;
                }
            }
        }
    }
    0
}

/// A collision sweep holding its frame while the sweep updates its end
/// point.
pub struct FrameIntersection {
    control: IntersectControl,
    frame: super::drawable::ContextFrame,
}

impl FrameIntersection {
    pub fn new(frame: super::drawable::ContextFrame, origin: Point) -> Self {
        Self {
            control: IntersectControl {
                last_time_val: 0,
                EndPoint: origin,
                IntersectStamp: Stamp {
                    origin,
                    frame: frame.frame(),
                },
            },
            frame,
        }
    }

    pub fn set_frame(&mut self, frame: super::drawable::ContextFrame, origin: Point) {
        self.control.IntersectStamp = Stamp {
            origin,
            frame: frame.frame(),
        };
        self.control.EndPoint = origin;
        self.frame = frame;
    }

    pub fn intersect(&mut self, other: &mut Self, max_time: u16) -> u16 {
        drawables_intersect(&mut self.control, &mut other.control, max_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: i16, y: i16) -> Point {
        Point { x, y }
    }

    fn frame(width: i16, height: i16) -> Shape {
        Shape {
            hot_spot: point(0, 0),
            bounds: Extent { width, height },
            image: None,
        }
    }

    struct Probe<'a> {
        control: IntersectControl,
        shape: Option<&'a Shape>,
    }

    impl std::ops::Deref for Probe<'_> {
        type Target = IntersectControl;
        fn deref(&self) -> &IntersectControl {
            &self.control
        }
    }

    impl std::ops::DerefMut for Probe<'_> {
        fn deref_mut(&mut self) -> &mut IntersectControl {
            &mut self.control
        }
    }

    fn control(shape: &Shape, origin: Point, end_point: Point) -> Probe<'_> {
        Probe {
            control: IntersectControl {
                last_time_val: 99,
                EndPoint: end_point,
                IntersectStamp: Stamp {
                    origin,
                    frame: FrameId::NULL,
                },
            },
            shape: Some(shape),
        }
    }

    fn intersect(a: &mut Probe<'_>, b: &mut Probe<'_>, max: u16) -> u16 {
        let shapes = (a.shape.copied(), b.shape.copied());
        sweep(
            &mut a.control,
            &mut b.control,
            shapes,
            max,
            true,
            &mut |_, _, _, _, _| true,
        )
    }

    #[test]
    fn stationary_overlap_returns_one_and_preserves_zero_last_time() {
        let fa = frame(2, 2);
        let fb = frame(2, 2);
        let mut a = control(&fa, point(0, 0), point(0, 0));
        let mut b = control(&fb, point(1, 1), point(1, 1));
        assert_eq!(intersect(&mut a, &mut b, 257), 1);
        assert_eq!((a.last_time_val, b.last_time_val), (0, 0));
        assert_eq!((a.EndPoint, b.EndPoint), (point(0, 0), point(1, 1)));
    }

    #[test]
    fn moving_point_returns_collision_tick_but_previous_noncolliding_endpoint() {
        // cycle=4: pixel steps occur at 65,129,193,257. Search begins at 64.
        let fa = frame(1, 1);
        let fb = frame(1, 1);
        let mut a = control(&fa, point(0, 0), point(4, 0));
        let mut b = control(&fb, point(2, 0), point(2, 0));
        assert_eq!(intersect(&mut a, &mut b, 257), 129);
        assert_eq!((a.last_time_val, b.last_time_val), (65, 64));
        assert_eq!((a.EndPoint, b.EndPoint), (point(1, 0), point(2, 0)));
    }

    #[test]
    fn reverse_motion_has_the_same_time_and_previous_endpoint() {
        let fa = frame(1, 1);
        let fb = frame(1, 1);
        let mut a = control(&fa, point(4, 0), point(0, 0));
        let mut b = control(&fb, point(2, 0), point(2, 0));
        assert_eq!(intersect(&mut a, &mut b, 257), 129);
        assert_eq!((a.last_time_val, b.last_time_val), (65, 64));
        assert_eq!(a.EndPoint, point(3, 0));
    }

    #[test]
    fn simultaneous_steps_are_tested_together_after_fast_forward() {
        // Relative cycle=8 sets t0=96. Each walker starts at its first pixel.
        let fa = frame(1, 1);
        let fb = frame(1, 1);
        let mut a = control(&fa, point(0, 0), point(4, 0));
        let mut b = control(&fb, point(4, 0), point(0, 0));
        assert_eq!(intersect(&mut a, &mut b, 257), 129);
        assert_eq!((a.last_time_val, b.last_time_val), (96, 96));
        assert_eq!((a.EndPoint, b.EndPoint), (point(1, 0), point(3, 0)));
    }

    #[test]
    fn bresenham_minor_axis_does_not_use_linear_sampling() {
        // cycle=5, errors=(5,5): (1,0) at 53, (2,0) at 104, (3,1) at 155.
        // Linear sampling invents (2,1) at 129, but the C walker never visits it.
        let fa = frame(1, 1);
        let fb = frame(1, 1);
        let mut a = control(&fa, point(0, 0), point(5, 2));
        let mut b = control(&fb, point(2, 1), point(2, 1));
        assert_eq!(intersect(&mut a, &mut b, 257), 0);
        assert_eq!((a.last_time_val, b.last_time_val), (104, 51));
        assert_eq!((a.EndPoint, b.EndPoint), (point(5, 2), point(2, 1)));
    }

    #[test]
    fn touching_box_edges_are_not_overlapping_pixels() {
        let fa = frame(2, 2);
        let fb = frame(2, 2);
        let mut a = control(&fa, point(0, 0), point(0, 0));
        let mut b = control(&fb, point(2, 0), point(2, 0));
        assert_eq!(intersect(&mut a, &mut b, 257), 0);
        a.EndPoint = point(2, 0);
        // First one-pixel move is at 129; the boxes now share a column.
        assert_eq!(intersect(&mut a, &mut b, 257), 129);
        assert_eq!((a.last_time_val, b.last_time_val), (0, 0));
        assert_eq!(a.EndPoint, point(0, 0));
    }

    #[test]
    fn time_limit_is_inclusive_and_clamped_to_257() {
        for (max, expected) in [(128, 0), (129, 129), (257, 129), (u16::MAX, 129)] {
            let fa = frame(1, 1);
            let fb = frame(1, 1);
            let mut a = control(&fa, point(0, 0), point(4, 0));
            let mut b = control(&fb, point(2, 0), point(2, 0));
            assert_eq!(intersect(&mut a, &mut b, max), expected);
        }
    }

    #[test]
    fn fast_forward_keeps_error_remainders() {
        // t0=192: start=8*191=1528, five steps plus time_error=248.
        // The next pixels are x=6 at 193 and x=7 at 225.
        let fa = frame(1, 1);
        let fb = frame(1, 1);
        let mut a = control(&fa, point(0, 0), point(8, 0));
        let mut b = control(&fb, point(7, 0), point(7, 0));
        assert_eq!(intersect(&mut a, &mut b, 257), 225);
        assert_eq!((a.last_time_val, b.last_time_val), (193, 192));
        assert_eq!(a.EndPoint, point(6, 0));
    }

    #[test]
    fn inactive_context_and_zero_limit_do_not_touch_controls() {
        for (active, max) in [(false, 257), (true, 0)] {
            let shape = Some(frame(1, 1));
            assert_eq!(
                sweep(
                    &mut IntersectControl::default(),
                    &mut IntersectControl::default(),
                    (shape, shape),
                    max,
                    active,
                    &mut |_, _, _, _, _| panic!("early exit must not test pixels"),
                ),
                0
            );
        }
        let fa = frame(1, 1);
        let fb = frame(1, 1);
        let mut a = control(&fa, point(0, 0), point(4, 0));
        let mut b = control(&fb, point(2, 0), point(2, 0));
        assert_eq!(intersect(&mut a, &mut b, 0), 0);
        assert_eq!((a.last_time_val, b.last_time_val), (99, 99));
        a.shape = None;
        assert_eq!(intersect(&mut a, &mut b, 257), 0);
        assert_eq!((a.last_time_val, b.last_time_val), (0, 0));
    }
    #[test]
    fn diagonal_hit_applies_hotspots_and_updates_only_after_missed_steps() {
        // (dx,dy)=(4,2): (1,0) at 65, then (2,1) at 129.
        let mut fa = frame(1, 1);
        let mut fb = frame(1, 1);
        fa.hot_spot = point(2, 3);
        fb.hot_spot = point(2, 3);
        let mut a = control(&fa, point(10, 20), point(14, 22));
        let mut b = control(&fb, point(12, 21), point(12, 21));
        assert_eq!(intersect(&mut a, &mut b, 257), 129);
        assert_eq!((a.last_time_val, b.last_time_val), (65, 64));
        assert_eq!((a.EndPoint, b.EndPoint), (point(11, 20), point(12, 21)));
    }

    #[test]
    fn vertical_motion_in_both_directions_uses_the_same_ticks() {
        for (start, end, previous) in [(0, 4, 1), (4, 0, 3)] {
            let fa = frame(1, 1);
            let fb = frame(1, 1);
            let mut a = control(&fa, point(0, start), point(0, end));
            let mut b = control(&fb, point(0, 2), point(0, 2));
            assert_eq!(intersect(&mut a, &mut b, 257), 129);
            assert_eq!((a.last_time_val, b.last_time_val), (65, 64));
            assert_eq!(a.EndPoint, point(0, previous));
        }
    }

    #[test]
    fn same_velocity_checks_only_the_initial_pixels() {
        let fa = frame(2, 2);
        let fb = frame(2, 2);
        let mut a = control(&fa, point(0, 0), point(4, 2));
        let mut b = control(&fb, point(1, 1), point(5, 3));
        let mut checks = 0;
        assert_eq!(
            sweep(
                &mut a.control,
                &mut b.control,
                (Some(fa), Some(fb)),
                257,
                true,
                &mut |_, _, _, _, _| {
                    checks += 1;
                    false
                }
            ),
            0
        );
        assert_eq!(checks, 1);
        assert_eq!((a.last_time_val, b.last_time_val), (0, 0));
        assert_eq!((a.EndPoint, b.EndPoint), (point(4, 2), point(5, 3)));
    }

    #[test]
    fn bounding_rejection_does_not_call_pixel_intersection() {
        for target in [point(8, 0), point(0, 8)] {
            let fa = frame(1, 1);
            let fb = frame(1, 1);
            let mut a = control(&fa, point(0, 0), point(-4, -4));
            let mut b = control(&fb, target, target);
            assert_eq!(
                sweep(
                    &mut a.control,
                    &mut b.control,
                    (Some(fa), Some(fb)),
                    257,
                    true,
                    &mut |_, _, _, _, _| panic!("disjoint swept boxes")
                ),
                0
            );
            assert_eq!((a.last_time_val, b.last_time_val), (0, 0));
        }
    }

    #[test]
    fn frame_fast_forward_narrows_count_and_advances_only_once_per_tick() {
        // COUNT(512*255)=65024, then >>8 is 254. At 257 the walker
        // advances to 255, not 510 or 512, despite cycle exceeding 256.
        let fa = frame(1, 1);
        let fb = frame(1, 1);
        let mut a = control(&fa, point(0, 0), point(512, 0));
        let mut b = control(&fb, point(255, 0), point(255, 0));
        let mut r0 = Rect {
            corner: point(0, 0),
            extent: Extent {
                width: 512,
                height: 0,
            },
        };
        let mut r1 = Rect {
            corner: point(255, 0),
            extent: Extent::default(),
        };
        assert_eq!(
            frame_intersect(
                (&mut a.control, &fa, &mut r0),
                (&mut b.control, &fb, &mut r1),
                (256, 256),
                &mut |_, _, _, _, _| true,
            ),
            257
        );
        assert_eq!(r0.corner, point(254, 0));
        assert_eq!((a.last_time_val, b.last_time_val), (256, 256));
    }
}
