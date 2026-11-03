//! Gravity and spawn overlap checks over the live ordered element pool.

use crate::engine::battle::{Cx, ElementId};
use crate::ships::native::abi::{
    DISAPPEARING, INTERSECT_CONTROL, NONSOLID, PLAYER_SHIP, PRE_PROCESS,
};

fn gravity_mass(mass: u8) -> bool {
    mass > 100
}

pub(crate) fn calculate_gravity(
    cx: &mut Cx<'_>,
    element: ElementId,
    dimensions: (i32, i32),
) -> bool {
    let elem = cx.element(element);
    let has_gravity = elem.flags & (NONSOLID | DISAPPEARING) as u16 == 0
        && gravity_mass(elem.mass.wrapping_add(1));
    let mut next = cx.elements().head();
    while let Some(test) = next {
        next = cx.elements().succ(test);
        let other = cx.element(test);
        if test == element || other.flags & (NONSOLID | DISAPPEARING) as u16 != 0 {
            continue;
        }
        let test_has_gravity = gravity_mass(other.mass.wrapping_add(1));
        if test_has_gravity == has_gravity {
            continue;
        }
        let (a, b) = if elem.flags & PRE_PROCESS as u16 == 0 {
            (elem.current.location, other.current.location)
        } else {
            (elem.next.location, other.next.location)
        };
        let (width, height) = dimensions;
        let dx = crate::math::wrap_delta(a.x - b.x, (width - 64) << 5);
        let dy = crate::math::wrap_delta(a.y - b.y, height << 5);
        let ax = i32::from(dx).unsigned_abs() >> 2;
        let ay = i32::from(dy).unsigned_abs() >> 2;
        if ax > 255 || ay > 255 || ax * ax + ay * ay > 255 * 255 {
            continue;
        }
        if test_has_gravity {
            return true;
        }
        let angle = crate::math::arctan(dx, dy);
        cx.element_mut(test).velocity.delta_components(
            super::battle_types::cosine(angle, 1 << 5),
            super::battle_types::sine(angle, 1 << 5),
        );
    }
    false
}

pub(crate) fn control(
    cx: &Cx<'_>,
    id: ElementId,
    frame: crate::engine::battle::views::FrameSelection,
) -> INTERSECT_CONTROL {
    let current = cx.element(id).current;
    let origin = crate::graphics::native_abi::POINT {
        x: current.location.x >> 2,
        y: current.location.y >> 2,
    };
    INTERSECT_CONTROL {
        last_time_val: 0,
        EndPoint: origin,
        IntersectStamp: crate::graphics::native_abi::STAMP {
            origin,
            frame: frame.raw(),
        },
    }
}

pub(crate) fn time_space_matter_conflict(
    cx: &Cx<'_>,
    element: ElementId,
    mut intersects: impl FnMut(ElementId) -> bool,
) -> bool {
    let mut next = cx.elements().head();
    while let Some(test) = next {
        next = cx.elements().succ(test);
        let state = cx.element(test);
        if test == element
            || (state.flags & (NONSOLID | DISAPPEARING) as u16 != 0
                && state.flags & PLAYER_SHIP as u16 == 0)
        {
            continue;
        }
        if intersects(test) {
            return true;
        }
    }
    false
}
