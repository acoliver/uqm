//! One observation per hook and a context handoff only for legacy callbacks.

use crate::engine::battle::record::ELEMENT;
use crate::engine::battle::{Cx, ElementHook, ElementId};

/// # Safety
/// The pointer is a live element; no record borrow crosses this call.
pub unsafe fn invoke_legacy_element<E>(
    hook: ElementHook<unsafe extern "C" fn(*mut E)>,
    element: *mut E,
) {
    // SAFETY: this legacy boundary owns the currently published context.
    unsafe {
        crate::app::bridge::enter(|cx| match hook {
            ElementHook::Legacy(callback) => invoke_race_element(cx, callback, element),
            ElementHook::Native(callback) => {
                let id =
                    cx.b.elements
                        .id(element.cast())
                        .expect("null native element");
                invoke_element(cx, ElementHook::Native(callback), id);
            }
        });
    }
}

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
            let parent = cx.b.elements[element].pParent as usize;
            super::dispatch::invoke(callback as usize, parent, || callback(cx, element));
        }
        ElementHook::Legacy(callback) => {
            let pointer = cx.b.elements.raw(element).cast::<ELEMENT>();
            // SAFETY: the checked id belongs to this context's live pool.
            unsafe {
                invoke_race_element(cx, callback, pointer);
            }
        }
    }
}
