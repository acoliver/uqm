#![allow(dead_code)]
use uqm_engine::battle::{CollisionHook, ElementHook, ElementId};
use uqm_engine::graphics::geometry::Point;

pub fn invoke_element(hook: ElementHook, element: ElementId) {
    crate::battle_context::with(|cx| {
        uqm_modes::battle::process_native::invoke_element::invoke_element(cx, hook, element);
    });
}

pub fn invoke_collision(
    hook: CollisionHook,
    a: ElementId,
    pa: &mut Point,
    b: ElementId,
    pb: &mut Point,
) {
    crate::battle_context::with(|cx| {
        uqm_modes::battle::process_native::invoke_collision::invoke_collision(
            cx, hook, a, pa, b, pb,
        );
    });
}
