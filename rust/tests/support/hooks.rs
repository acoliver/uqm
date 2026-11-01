#![allow(dead_code)]
use uqm_rust::engine::battle::{CollisionHook, ElementHook};

pub enum CollisionCall {
    Hook,
}

/// # Safety
/// The fixture retains the pool element and publishes its battle context.
pub unsafe fn invoke_element_pointer<E>(hook: ElementHook, element: *mut E) {
    // SAFETY: fixture storage stays live until native dispatch returns.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let id = cx
                .elements()
                .id(element.cast())
                .unwrap_or_else(|| panic!("fixture element is allocated"));
            uqm_rust::battle::process_native::invoke_element::invoke_element(cx, hook, id);
        });
    }
}

/// # Safety
/// Both elements and collision points remain live through the synchronous callback.
pub unsafe fn invoke_collision_pointer<E, P>(
    hook: CollisionHook,
    a: *mut E,
    pa: *mut P,
    b: *mut E,
    pb: *mut P,
    _: CollisionCall,
) {
    // SAFETY: the fixture owns the published context, elements and writable points.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let a = cx
                .elements()
                .id(a.cast())
                .unwrap_or_else(|| panic!("first collision element is allocated"));
            let b = cx
                .elements()
                .id(b.cast())
                .unwrap_or_else(|| panic!("second collision element is allocated"));
            uqm_rust::battle::process_native::invoke_collision::invoke_collision(
                cx,
                hook,
                a,
                &mut *pa.cast(),
                b,
                &mut *pb.cast(),
            );
        });
    }
}
