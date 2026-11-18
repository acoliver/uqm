//! Fixed-width prediction over retained subject projections.

use super::flags::*;
use super::{Cx, Subject};
use crate::graphics::native_abi::COUNT;
use crate::math::battle_types::ONE_SHIFT;
use crate::math::velocity::VELOCITY_SHIFT;

fn c_size(v: i32) -> i16 {
    v as i16
}

fn velocity_to_world(v: i64) -> i16 {
    (v >> VELOCITY_SHIFT) as i16
}

/// Return the first turn at which the two swept rectangles can overlap.
///
/// This is the integer-only broad phase from `PlotIntercept`; the engine
/// narrow-phase collision callback is applied by the live implementation.
pub fn intercept_window(
    p0: (i16, i16),
    v0: (i16, i16),
    p1: (i16, i16),
    v1: (i16, i16),
    max_turns: u16,
    extents: [(i16, i16); 2],
) -> Option<(i16, i16)> {
    let (mut dx0, mut dy0) = v0;
    let (mut dx1, mut dy1) = v1;
    dx0 = velocity_to_world(i64::from(dx0) * i64::from(max_turns));
    dy0 = velocity_to_world(i64::from(dy0) * i64::from(max_turns));
    dx1 = velocity_to_world(i64::from(dx1) * i64::from(max_turns));
    dy1 = velocity_to_world(i64::from(dy1) * i64::from(max_turns));

    let x0 = i32::from(p0.0);
    let y0 = i32::from(p0.1);
    let x1 = i32::from(p1.0);
    let y1 = i32::from(p1.1);
    let mut t0y = i32::from(extents[0].1);
    let mut t1y = i32::from(extents[1].1);
    let diff_y = i32::from(c_size(y1 - y0));
    t0y = i32::from(c_size(diff_y - t0y + 1));
    t1y = i32::from(c_size(diff_y + t1y - 1));
    let mut rel_y = i32::from(c_size(i32::from(dy0) - i32::from(dy1)));

    if !axis_possible(t0y, t1y, rel_y) {
        return None;
    }
    let mut t0x = i32::from(extents[0].0);
    let mut t1x = i32::from(extents[1].0);
    let diff_x = i32::from(c_size(x1 - x0));
    t0x = i32::from(c_size(diff_x - t0x + 1));
    t1x = i32::from(c_size(diff_x + t1x - 1));
    let mut rel_x = i32::from(c_size(i32::from(dx0) - i32::from(dx1)));
    if !axis_possible(t0x, t1x, rel_x) {
        return None;
    }
    if rel_x == 0 && rel_y == 0 {
        return Some((0, 0));
    }

    if t1y < 0 {
        (t0y, t1y) = (i32::from(c_size(-t1y)), i32::from(c_size(-t0y)));
    } else if t0y <= 0 {
        if rel_y < 0 {
            t1y = i32::from(c_size(-t0y));
        }
        t0y = 0;
    }
    if rel_y < 0 {
        rel_y = i32::from(c_size(-rel_y));
    }
    if rel_y < t1y {
        t1y = rel_y;
    }
    if t1x < 0 {
        (t0x, t1x) = (i32::from(c_size(-t1x)), i32::from(c_size(-t0x)));
    } else if t0x <= 0 {
        if rel_x < 0 {
            t1x = i32::from(c_size(-t0x));
        }
        t0x = 0;
    }
    if rel_x < 0 {
        rel_x = i32::from(c_size(-rel_x));
    }
    if rel_x < t1x {
        t1x = rel_x;
    }

    let (begin, end, fract): (i64, i64, i64) = if rel_x == 0 {
        (i64::from(t0y), i64::from(t1y), i64::from(rel_y))
    } else if rel_y == 0 {
        (i64::from(t0x), i64::from(t1x), i64::from(rel_x))
    } else {
        let tx0 = i64::from(t0x) * i64::from(rel_y);
        let ty0 = i64::from(t0y) * i64::from(rel_x);
        let tx1 = i64::from(t1x) * i64::from(rel_y);
        let ty1 = i64::from(t1y) * i64::from(rel_x);
        (
            tx0.max(ty0),
            tx1.min(ty1),
            i64::from(rel_x) * i64::from(rel_y),
        )
    };
    let first = if begin * i64::from(max_turns) < fract {
        0
    } else {
        ((begin * i64::from(max_turns)) / fract) as i32
    } as i16;
    let last = if end >= fract {
        i32::from(max_turns) - 1
    } else {
        ((end * i64::from(max_turns)) / fract) as i32
    } as i16;
    if first <= last {
        Some((first, last))
    } else {
        None
    }
}

fn axis_possible(t0: i32, t1: i32, velocity: i32) -> bool {
    (t0 <= 0 && t1 >= 0) || (t0 > 0 && velocity >= t0) || (t1 < 0 && velocity <= t1)
}

fn current_velocity(v: &crate::math::velocity::VelocityDesc) -> (i16, i16) {
    let (x, y) = v.get_current_components();
    (x as i16, y as i16)
}

pub fn plot_intercept(
    cx: &mut Cx<'_>,
    a: &Subject,
    b: &Subject,
    mut turns: COUNT,
    margin: COUNT,
) -> COUNT {
    if (a.flags | b.flags) & FINITE_LIFE as u16 != 0 {
        if a.flags & FINITE_LIFE as u16 == 0 {
            turns = turns.min(b.life);
        } else if b.flags & FINITE_LIFE as u16 == 0 {
            turns = turns.min(a.life);
        } else {
            turns = turns.min(a.life).min(b.life);
        }
    }
    let (avx, avy) = current_velocity(&a.velocity);
    let (bvx, bvy) = current_velocity(&b.velocity);
    let (ha, hb, pos_a, pos_b) = if margin != 0 {
        let m = i32::from(margin);
        (
            (1, 1),
            ((m << 1) as i16, (m << 1) as i16),
            (a.current.location.x, a.current.location.y),
            (
                b.current.location.x.wrapping_sub(m as i16),
                b.current.location.y.wrapping_sub(m as i16),
            ),
        )
    } else {
        let ar = cx.frame_rect(cx.subject_intersect_frame(a));
        let br = cx.frame_rect(cx.subject_intersect_frame(b));
        let ax = (ar.corner.x as i32).wrapping_shl(ONE_SHIFT) as i16;
        let ay = (ar.corner.y as i32).wrapping_shl(ONE_SHIFT) as i16;
        let bx = (br.corner.x as i32).wrapping_shl(ONE_SHIFT) as i16;
        let by = (br.corner.y as i32).wrapping_shl(ONE_SHIFT) as i16;
        (
            (
                (i32::from(ar.extent.width) << ONE_SHIFT) as i16,
                (i32::from(ar.extent.height) << ONE_SHIFT) as i16,
            ),
            (
                (i32::from(br.extent.width) << ONE_SHIFT) as i16,
                (i32::from(br.extent.height) << ONE_SHIFT) as i16,
            ),
            (
                a.current.location.x.wrapping_add(ax),
                a.current.location.y.wrapping_add(ay),
            ),
            (
                b.current.location.x.wrapping_add(bx),
                b.current.location.y.wrapping_add(by),
            ),
        )
    };
    let Some((first, last)) =
        intercept_window(pos_a, (avx, avy), pos_b, (bvx, bvy), turns, [ha, hb])
    else {
        return 0;
    };
    if margin != 0 {
        return (first as u16).wrapping_add(1);
    }

    let mut ca = a.intersection();
    let mut cb = b.intersection();
    let mut pa = a.current.location;
    let mut pb = b.current.location;
    let mut va = a.velocity;
    let mut vb = b.velocity;
    if first != 0 {
        let (x, y) = next_velocity(&mut va, first as u16);
        pa.x = pa.x.wrapping_add(x);
        pa.y = pa.y.wrapping_add(y);
        ca.EndPoint.x = pa.x >> ONE_SHIFT;
        ca.EndPoint.y = pa.y >> ONE_SHIFT;
        let (x, y) = next_velocity(&mut vb, first as u16);
        pb.x = pb.x.wrapping_add(x);
        pb.y = pb.y.wrapping_add(y);
        cb.EndPoint.x = pb.x >> ONE_SHIFT;
        cb.EndPoint.y = pb.y >> ONE_SHIFT;
    }
    let mut turn = first;
    loop {
        turn = turn.wrapping_add(1);
        let (x, y) = next_velocity(&mut va, 1);
        pa.x = pa.x.wrapping_add(x);
        pa.y = pa.y.wrapping_add(y);
        let (x, y) = next_velocity(&mut vb, 1);
        pb.x = pb.x.wrapping_add(x);
        pb.y = pb.y.wrapping_add(y);
        ca.IntersectStamp.origin = ca.EndPoint;
        ca.EndPoint.x = pa.x >> ONE_SHIFT;
        ca.EndPoint.y = pa.y >> ONE_SHIFT;
        cb.IntersectStamp.origin = cb.EndPoint;
        cb.EndPoint.x = pb.x >> ONE_SHIFT;
        cb.EndPoint.y = pb.y >> ONE_SHIFT;
        let hit = cx.projected_intersect(&mut ca, &mut cb, MAX_TIME_VALUE as u16);
        if hit != 0 {
            if hit == 1 && turn == 1 && (a.flags | b.flags) & APPEARING as u16 != 0 {
                ca.EndPoint.x = pa.x >> ONE_SHIFT;
                ca.EndPoint.y = pa.y >> ONE_SHIFT;
                cb.EndPoint.x = pb.x >> ONE_SHIFT;
                cb.EndPoint.y = pb.y >> ONE_SHIFT;
            } else {
                return turn as u16;
            }
        }
        if turn >= last {
            break;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intersect_window_handles_stationary_and_relative_motion() {
        assert_eq!(
            intercept_window((0, 0), (0, 0), (0, 0), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((0, 0))
        );
        assert_eq!(
            intercept_window((0, 0), (640, 0), (50, 0), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((2, 2))
        );
        assert_eq!(
            intercept_window((0, 0), (0, 0), (100, 100), (0, 0), 10, [(1, 1), (1, 1)]),
            None
        );
    }
    #[test]
    fn intercept_requires_overlapping_time_windows_on_both_axes() {
        assert_eq!(
            intercept_window((0, 0), (320, 640), (100, 100), (0, 0), 20, [(1, 1), (1, 1)]),
            None
        );
        assert_eq!(
            intercept_window((0, 0), (320, 320), (100, 100), (0, 0), 20, [(1, 1), (1, 1)]),
            Some((10, 10))
        );
    }
    #[test]
    fn intercept_handles_negative_motion_and_one_stationary_axis() {
        for (position, velocity) in [(100, 320), (-100, -320)] {
            assert_eq!(
                intercept_window(
                    (0, 0),
                    (velocity, 0),
                    (position, 0),
                    (-velocity, 0),
                    10,
                    [(1, 1), (1, 1)]
                ),
                Some((5, 5))
            );
        }
        assert_eq!(
            intercept_window((0, 0), (0, 320), (0, 50), (0, 0), 10, [(1, 1), (1, 1)]),
            Some((5, 5))
        );
    }
    #[test]
    fn intercept_rejects_receding_objects_and_preserves_initial_overlap() {
        assert_eq!(
            intercept_window((0, 0), (-320, 0), (100, 0), (320, 0), 10, [(1, 1), (1, 1)]),
            None
        );
        assert_eq!(
            intercept_window((0, 0), (0, 0), (3, 3), (0, 0), 0, [(4, 4), (4, 4)]),
            Some((0, 0))
        );
    }
    #[test]
    fn intercept_narrows_turn_endpoints_to_signed_size_before_comparing() {
        assert_eq!(
            intercept_window((0, 0), (16, 0), (19000, 0), (0, 0), 40000, [(1, 1), (1, 1)]),
            Some((-27536, -27536))
        );
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;
    #[test]
    fn conversion_widths_follow_signed_size_and_unsigned_count() {
        assert_eq!(c_size(32768), i16::MIN);
        assert_eq!(
            velocity_to_world(i64::from(i16::MAX) * 64),
            (i32::from(i16::MAX) * 2) as i16
        );
        assert_eq!(crate::math::normalize_angle(-1), 63);
        assert_eq!(crate::math::facing_to_angle(15), 60);
    }
}

pub fn next_velocity(v: &mut crate::math::velocity::VelocityDesc, turns: u16) -> (i16, i16) {
    let (x, y) = v.get_next_components(turns);
    (x as i16, y as i16)
}
