//! Race dispatch entry points. Legacy record decoding lives in the context accessors.
/// # Safety
/// The closure calls legacy code on the published thread without retaining records.
pub unsafe fn invoke_context<R>(callback: usize, parent: usize, call: impl FnOnce() -> R) -> R {
    // SAFETY: enter removes publication while deriving the nested handoff.
    unsafe {
        crate::app::bridge::enter(|cx| super::dispatch::invoke_legacy(cx, callback, parent, call))
    }
}
