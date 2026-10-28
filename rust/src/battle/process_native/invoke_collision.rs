//! Collision hooks retain the caller's pair ordering and point mutation order.

use crate::engine::battle::record::ELEMENT;
use crate::engine::battle::{CollisionHook, Cx, ElementId};
use crate::graphics::native_abi::POINT;

/// # Safety
/// Both ids and their callback resources must be live. No element borrow is retained.
pub unsafe fn invoke_collision(
    cx: &mut Cx<'_>,
    hook: CollisionHook,
    a: ElementId,
    pa: &mut POINT,
    b: ElementId,
    pb: &mut POINT,
) {
    let first = cx.b.elements.raw(a).cast::<ELEMENT>();
    let second = cx.b.elements.raw(b).cast::<ELEMENT>();
    let parent = cx.b.elements[a].pParent as usize;
    let CollisionHook::Legacy(callback) = hook;
    crate::collections::observation::observe(callback as usize, parent);
    // SAFETY: ids have been checked and publication suspends the context borrow.
    unsafe {
        crate::app::bridge::publish(cx, || callback(first, pa, second, pb));
    }
}

/// # Safety
/// Pointers name live legacy elements and collision points on the game thread.
pub unsafe fn invoke_legacy_collision<E, P>(
    hook: CollisionHook<unsafe extern "C" fn(*mut E, *mut P, *mut E, *mut P)>,
    a: *mut E,
    pa: *mut P,
    b: *mut E,
    pb: *mut P,
) {
    let CollisionHook::Legacy(callback) = hook;
    // SAFETY: the two element layouts share the asserted parent offset.
    let parent = unsafe { (*a.cast::<ELEMENT>()).pParent as usize };
    crate::collections::observation::observe(callback as usize, parent);
    // SAFETY: the caller is a published legacy boundary and retains no record borrow.
    unsafe {
        crate::app::bridge::enter(|cx| crate::app::bridge::publish(cx, || callback(a, pa, b, pb)));
    }
}

/// # Safety
/// Same contract as invoke_legacy_collision, for the layout-checked observation record.
pub unsafe fn invoke_legacy_collision_const<E, P>(
    hook: CollisionHook<unsafe extern "C" fn(*mut E, *const P, *mut E, *const P)>,
    a: *mut E,
    pa: *const P,
    b: *mut E,
    pb: *const P,
) {
    let CollisionHook::Legacy(callback) = hook;
    // SAFETY: the mirrored record's parent offset is layout-checked.
    let parent = unsafe { (*a.cast::<ELEMENT>()).pParent as usize };
    crate::collections::observation::observe(callback as usize, parent);
    // SAFETY: the caller is a published legacy boundary and retains no record borrow.
    unsafe {
        crate::app::bridge::enter(|cx| crate::app::bridge::publish(cx, || callback(a, pa, b, pb)));
    }
}

/// # Safety
/// Both elements and points are live for this collision. Callbacks may replace hooks.
pub unsafe fn invoke_pair(
    cx: &mut Cx<'_>,
    a: ElementId,
    pa: &mut POINT,
    b: ElementId,
    pb: &mut POINT,
    test_is_player: bool,
) {
    let pairs = [
        (a, &raw mut *pa, b, &raw mut *pb),
        (b, &raw mut *pb, a, &raw mut *pa),
    ];
    for index in super::dispatch::collision_order(test_is_player) {
        let (first, point, second, other_point) = pairs[index];
        let hook = cx.b.elements[first]
            .collision_func
            .unwrap_or_else(|| panic!("solid element has no collision hook"));
        // SAFETY: each invocation ends before the next hook is reread.
        unsafe {
            invoke_collision(cx, hook, first, &mut *point, second, &mut *other_point);
        }
    }
}
