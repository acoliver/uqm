//! Race-vtable calls remain legacy until their race is converted.
use crate::collections::queue::HLink;
use crate::engine::battle::record::{ELEMENT, EVALUATE_DESC};

/// # Safety
/// The closure calls legacy code on the published game thread without retaining records.
pub unsafe fn invoke_context<R>(callback: usize, parent: usize, call: impl FnOnce() -> R) -> R {
    // SAFETY: enter removes publication while deriving the nested handoff.
    unsafe {
        crate::app::bridge::enter(|cx| {
            super::dispatch::invoke(callback, parent, || crate::app::bridge::publish(cx, call))
        })
    }
}

/// # Safety
/// The callback and element/weapon storage are live on the published game thread.
pub unsafe fn invoke_weapon(
    callback: unsafe extern "C" fn(*mut ELEMENT, *mut HLink) -> u16,
    parent: usize,
    element: *mut ELEMENT,
    weapons: *mut HLink,
) -> u16 {
    // SAFETY: callback storage is live for the published call.
    unsafe { invoke_context(callback as usize, parent, || callback(element, weapons)) }
}

/// # Safety
/// The callback and concern records are live on the published game thread.
pub unsafe fn invoke_intelligence(
    callback: unsafe extern "C" fn(*mut ELEMENT, *mut EVALUATE_DESC, u16),
    parent: usize,
    element: *mut ELEMENT,
    concerns: *mut EVALUATE_DESC,
    count: u16,
) {
    // SAFETY: concern storage is live for the published call.
    unsafe {
        invoke_context(callback as usize, parent, || {
            callback(element, concerns, count)
        })
    }
}
