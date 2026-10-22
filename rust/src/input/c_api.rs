//! Unprefixed VControl ABI retained for C callers.

use std::ffi::{c_char, c_int, c_void};

use super::ffi::{self, VCONTROL_GESTURE};

/// # Safety
/// `event` must be null or point to an SDL2 event.
#[no_mangle]
pub unsafe extern "C" fn VControl_HandleEvent(event: *const c_void) {
    if event.is_null() {
        return;
    }
    let event = &*event.cast::<sdl2::sys::SDL_Event>();
    match event.type_ {
        0x300 | 0x301 => crate::bridge_log::rust_bridge_log_msg(&format!(
            "C_VCONTROL: {} sym=0x{:X}",
            if event.type_ == 0x300 {
                "KeyDown"
            } else {
                "KeyUp"
            },
            event.key.keysym.sym
        )),
        _ => {}
    }
    ffi::rust_VControl_HandleEvent((event as *const sdl2::sys::SDL_Event).cast());
}

/// # Safety
/// Non-null pointers must address a gesture and a target that outlives its binding.
#[no_mangle]
pub unsafe extern "C" fn VControl_AddGestureBinding(
    gesture: *mut VCONTROL_GESTURE,
    target: *mut c_int,
) -> c_int {
    if gesture.is_null() || target.is_null() {
        return -1;
    }
    ffi::rust_VControl_AddGestureBinding(gesture, target)
}

/// # Safety
/// Non-null pointers must address the gesture and target of a live binding.
#[no_mangle]
pub unsafe extern "C" fn VControl_RemoveGestureBinding(
    gesture: *mut VCONTROL_GESTURE,
    target: *mut c_int,
) {
    ffi::rust_VControl_RemoveGestureBinding(gesture, target);
}

/// # Safety
/// `gesture` must be null or writable for one gesture.
#[no_mangle]
pub unsafe extern "C" fn VControl_GetLastGesture(gesture: *mut VCONTROL_GESTURE) -> c_int {
    ffi::rust_VControl_GetLastGesture(gesture)
}

/// # Safety
/// Non-null arguments must be a writable gesture and a NUL-terminated specification.
#[no_mangle]
pub unsafe extern "C" fn VControl_ParseGesture(
    gesture: *mut VCONTROL_GESTURE,
    spec: *const c_char,
) {
    ffi::rust_VControl_ParseGesture(gesture, spec);
}

/// # Safety
/// Non-null `buffer` must hold `size` bytes and `gesture` must point to a gesture.
#[no_mangle]
pub unsafe extern "C" fn VControl_DumpGesture(
    buffer: *mut c_char,
    size: c_int,
    gesture: *mut VCONTROL_GESTURE,
) -> c_int {
    ffi::rust_VControl_DumpGesture(buffer, size, gesture)
}
