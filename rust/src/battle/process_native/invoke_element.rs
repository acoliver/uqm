//! One observation per hook and a context handoff only for legacy callbacks.

use crate::engine::battle::record::ELEMENT;
use crate::engine::battle::{Cx, ElementHook, ElementId};

pub use crate::engine::battle::access::invoke_legacy_element;

/// # Safety
/// Element and callback must be live on the game thread, with no record borrow.
pub unsafe fn invoke_race_element<E>(
    cx: &mut Cx<'_>,
    callback: unsafe extern "C" fn(*mut E),
    element: *mut E,
) {
    // SAFETY: live element layouts have the asserted identical parent offset.
    let parent = unsafe { (*element.cast::<ELEMENT>()).pParent as usize };
    // SAFETY: publication suspends cx until the callback returns.
    unsafe {
        super::dispatch::invoke_legacy(cx, callback as usize, parent, || callback(element));
    }
}

pub fn invoke_element(cx: &mut Cx<'_>, hook: ElementHook, element: ElementId) {
    match hook {
        ElementHook::Native(callback) => {
            let parent = cx.elements()[element].pParent as usize;
            super::dispatch::invoke(callback as usize, parent, || callback(cx, element));
        }
        ElementHook::Legacy(callback) => cx.invoke_legacy_element_id(callback, element),
    }
}
