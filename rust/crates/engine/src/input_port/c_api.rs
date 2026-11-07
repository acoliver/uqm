//! Unprefixed VControl ABI retained for C callers.

use std::ffi::{c_char, c_int, c_void};

use super::ffi::{self, VCONTROL_GESTURE};

/// # Safety
/// `event` must be null or point to an SDL2 event.
pub unsafe extern "C" fn vcontrol_handle_event(event: *const c_void) {
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
    ffi::rust_vcontrol_handle_event((event as *const sdl2::sys::SDL_Event).cast());
}

/// # Safety
/// Non-null pointers must address a gesture and a target that outlives its binding.
pub unsafe extern "C" fn vcontrol_add_gesture_binding(
    gesture: *mut VCONTROL_GESTURE,
    target: *mut c_int,
) -> c_int {
    if gesture.is_null() || target.is_null() {
        return -1;
    }
    ffi::rust_vcontrol_add_gesture_binding(gesture, target)
}

/// # Safety
/// Non-null pointers must address the gesture and target of a live binding.
pub unsafe extern "C" fn vcontrol_remove_gesture_binding(
    gesture: *mut VCONTROL_GESTURE,
    target: *mut c_int,
) {
    ffi::rust_vcontrol_remove_gesture_binding(gesture, target);
}

/// # Safety
/// `gesture` must be null or writable for one gesture.
pub unsafe extern "C" fn vcontrol_get_last_gesture(gesture: *mut VCONTROL_GESTURE) -> c_int {
    ffi::rust_vcontrol_get_last_gesture(gesture)
}

/// # Safety
/// Non-null arguments must be a writable gesture and a NUL-terminated specification.
pub unsafe extern "C" fn vcontrol_parse_gesture(
    gesture: *mut VCONTROL_GESTURE,
    spec: *const c_char,
) {
    ffi::rust_vcontrol_parse_gesture(gesture, spec);
}

/// # Safety
/// Non-null `buffer` must hold `size` bytes and `gesture` must point to a gesture.
pub unsafe extern "C" fn vcontrol_dump_gesture(
    buffer: *mut c_char,
    size: c_int,
    gesture: *mut VCONTROL_GESTURE,
) -> c_int {
    ffi::rust_vcontrol_dump_gesture(buffer, size, gesture)
}
