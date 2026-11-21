//! Collision hooks retain the caller's pair ordering and point mutation order.

use uqm_engine::battle::{CollisionHook, Cx, ElementId};
use uqm_engine::graphics::geometry::Point;

pub fn invoke_collision(
    cx: &mut Cx<'_>,
    hook: CollisionHook,
    a: ElementId,
    pa: &mut Point,
    b: ElementId,
    pb: &mut Point,
) {
    let parent = cx.element_parent_address(a) as usize;
    match hook {
        CollisionHook::Native(callback) => {
            super::dispatch::invoke(callback as usize, parent, || callback(cx, a, pa, b, pb))
        }
    }
}

pub fn invoke_pair(
    cx: &mut Cx<'_>,
    a: ElementId,
    pa: &mut Point,
    b: ElementId,
    pb: &mut Point,
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
