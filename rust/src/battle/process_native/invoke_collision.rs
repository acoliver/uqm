//! Collision hooks retain the caller's pair ordering and point mutation order.

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
    let parent = cx.elements()[a].pParent as usize;
    match hook {
        CollisionHook::Native(callback) => {
            super::dispatch::invoke(callback as usize, parent, || callback(cx, a, pa, b, pb))
        }
        CollisionHook::Legacy(callback) => cx.invoke_legacy_collision_ids(callback, a, pa, b, pb),
    }
}

pub use crate::engine::battle::access::CollisionCall;

pub use crate::engine::battle::access::invoke_legacy_collision;

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
        let hook = cx.elements()[first]
            .collision_func
            .unwrap_or_else(|| panic!("{}", "solid element has no collision hook"));
        if index == 0 {
            invoke_collision(cx, hook, first, pa, second, pb);
        } else {
            invoke_collision(cx, hook, first, pb, second, pa);
        }
    }
}
