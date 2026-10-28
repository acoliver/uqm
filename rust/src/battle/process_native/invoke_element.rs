//! One observation and context publication for each legacy element hook.

use crate::engine::battle::record::ELEMENT;
use crate::engine::battle::{Cx, ElementHook, ElementId};

/// # Safety
/// The pointer is a live legacy element; no record borrow crosses this call.
pub unsafe fn invoke_legacy_element<E>(
    hook: ElementHook<unsafe extern "C" fn(*mut E)>,
    element: *mut E,
) {
    let ElementHook::Legacy(callback) = hook;
    // SAFETY: the legacy entry has the currently published game context.
    unsafe {
        crate::app::bridge::enter(|cx| invoke_race_element(cx, callback, element));
    }
}

/// # Safety
/// Element and callback must be live on the game thread, with no record borrow.
pub unsafe fn invoke_race_element<E>(
    cx: &mut Cx<'_>,
    callback: unsafe extern "C" fn(*mut E),
    element: *mut E,
) {
    // SAFETY: both live element layouts have the asserted, identical parent offset.
    let parent = unsafe { (*element.cast::<ELEMENT>()).pParent as usize };
    // SAFETY: publication suspends cx until the callback returns.
    unsafe {
        super::dispatch::invoke(callback as usize, parent, || {
            crate::app::bridge::publish(cx, || callback(element))
        });
    }
}

/// # Safety
/// The element's resources and legacy hook must be live.
pub unsafe fn invoke_element(cx: &mut Cx<'_>, hook: ElementHook, element: ElementId) {
    let pointer = cx.b.elements.raw(element).cast::<ELEMENT>();
    let ElementHook::Legacy(callback) = hook;
    // SAFETY: the checked id belongs to the context's live pool.
    unsafe {
        invoke_race_element(cx, callback, pointer);
    }
}
