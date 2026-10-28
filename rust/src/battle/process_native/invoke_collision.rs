//! Collision hooks retain the caller's pair ordering and point mutation order.

use crate::engine::battle::record::ELEMENT;
use crate::engine::battle::{CollisionHook, Cx, ElementId};
use crate::graphics::native_abi::POINT;

pub fn invoke_collision(
    cx: &mut Cx<'_>,
    hook: CollisionHook,
    a: ElementId,
    pa: &mut POINT,
    b: ElementId,
    pb: &mut POINT,
) {
    let parent = cx.b.elements[a].pParent as usize;
    match hook {
        CollisionHook::Native(callback) => {
            super::dispatch::invoke(callback as usize, parent, || callback(cx, a, pa, b, pb))
        }
        CollisionHook::Legacy(callback) => {
            let first = cx.b.elements.raw(a).cast::<ELEMENT>();
            let second = cx.b.elements.raw(b).cast::<ELEMENT>();
            // SAFETY: checked ids and publication suspend the context borrow.
            unsafe {
                super::dispatch::invoke_legacy(cx, callback as usize, parent, || {
                    callback(first, pa, second, pb)
                });
            }
        }
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
    // SAFETY: the caller is a published boundary and retains no record borrow.
    unsafe {
        crate::app::bridge::enter(|cx| match hook {
            CollisionHook::Legacy(callback) => {
                let parent = (*a.cast::<ELEMENT>()).pParent as usize;
                super::dispatch::invoke_legacy(cx, callback as usize, parent, || {
                    callback(a, pa, b, pb)
                });
            }
            CollisionHook::Native(callback) => {
                let a =
                    cx.b.elements
                        .id(a.cast())
                        .expect("null first collision element");
                let b =
                    cx.b.elements
                        .id(b.cast())
                        .expect("null second collision element");
                invoke_collision(
                    cx,
                    CollisionHook::Native(callback),
                    a,
                    &mut *pa.cast(),
                    b,
                    &mut *pb.cast(),
                );
            }
        });
    }
}

pub fn invoke_pair(
    cx: &mut Cx<'_>,
    a: ElementId,
    pa: &mut POINT,
    b: ElementId,
    pb: &mut POINT,
    test_is_player: bool,
) {
    for index in super::dispatch::collision_order(test_is_player) {
        let (first, second) = if index == 0 { (a, b) } else { (b, a) };
        let hook = cx.b.elements[first]
            .collision_func
            .expect("solid element has no collision hook");
        if index == 0 {
            invoke_collision(cx, hook, first, pa, second, pb);
        } else {
            invoke_collision(cx, hook, first, pb, second, pa);
        }
    }
}
