use std::sync::Mutex;

static CONTEXT: Mutex<()> = Mutex::new(());

pub fn run<R>(body: impl FnOnce() -> R) -> R {
    let _guard = CONTEXT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // SAFETY: the test mutex serializes all use of this binary's game authorities.
    unsafe { uqm_rust::app::bridge::with_battle(body) }
}

#[allow(dead_code)]
pub unsafe fn dispose(handle: *mut std::ffi::c_void) {
    // SAFETY: callers own the serialized fixture and pass its live optional allocation.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            if let Some(id) = cx.elements().id(handle) {
                cx.dispose(id);
            }
        });
    }
}
#[allow(dead_code)]
pub unsafe fn unlink(handle: *mut std::ffi::c_void) {
    // SAFETY: callers own the serialized fixture and pass a currently linked allocation.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let id = cx.elements().id(handle).unwrap();
            cx.unlink(id);
        });
    }
}

/// # Safety
/// The supplied pointer is a retained fixture record owned by the active battle.
pub unsafe fn put(handle: *mut std::ffi::c_void) {
    uqm_rust::app::bridge::enter(|cx| {
        let id = cx
            .elements()
            .id(handle)
            .unwrap_or_else(|| panic!("fixture element"));
        cx.put(id);
    });
}
/// Cast a retained fixture handle without dereferencing it.
pub fn element(handle: *mut std::ffi::c_void) -> *mut uqm_rust::engine::battle::record::ELEMENT {
    handle.cast()
}
