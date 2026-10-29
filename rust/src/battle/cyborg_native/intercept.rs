//! Swept-frame intersection over value projections, without live record borrows.
use crate::battle::context_access::ai_legacy::{frame_rect, intersect};
use crate::battle::cyborg_native::intercept_window;
use crate::battle::velocity::VelocityDesc;
use crate::engine::battle::{Cx, Subject};
use crate::ships::native::abi::*;
fn current_velocity(v: &VelocityDesc) -> (i16, i16) {
    let (x, y) = v.get_current_components();
    (x as i16, y as i16)
}
fn next_velocity(v: &mut VelocityDesc, n: u16) -> (i16, i16) {
    let (x, y) = v.get_next_components(n);
    (x as i16, y as i16)
}
#[cfg(not(test))]
pub(crate) fn plot_intercept(
    _cx: &mut Cx<'_>,
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
        let ar = frame_rect(a.intersection().IntersectStamp.frame);
        let br = frame_rect(b.intersection().IntersectStamp.frame);
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
        let hit = intersect(&mut ca, &mut cb, MAX_TIME_VALUE as u16);
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
