//! Integration tests for Rust Input/VControl system
//!
//! These tests verify that the FFI layer correctly exposes the Rust
//! input system to C code, and that the integration with SDL events
//! works as expected.

use std::ffi::CString;

use serial_test::serial;

use uqm_rust::input::ffi::{
    rust_VControl_AddJoyAxisBinding, rust_VControl_AddJoyButtonBinding,
    rust_VControl_AddKeyBinding, rust_VControl_BeginFrame, rust_VControl_ClearGesture,
    rust_VControl_ClearKeyBindings, rust_VControl_GetLastGestureType,
    rust_VControl_GetNumJoysticks, rust_VControl_Init, rust_VControl_InitJoystick,
    rust_VControl_ProcessJoyAxis, rust_VControl_ProcessJoyButtonDown,
    rust_VControl_ProcessJoyButtonUp, rust_VControl_ProcessKeyDown, rust_VControl_ProcessKeyUp,
    rust_VControl_ResetInput, rust_VControl_SetJoyThreshold, rust_VControl_Uninit,
    rust_VControl_UninitJoystick,
};

#[test]
#[serial]
fn test_ffi_init_uninit() {
    unsafe {
        assert_eq!(rust_VControl_Init(), 0);
        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_key_binding() {
    unsafe {
        rust_VControl_Init();

        let mut target: i32 = 0;
        let keycode = 32; // Space key

        assert_eq!(rust_VControl_AddKeyBinding(keycode, &mut target), 0);

        // Simulate key down
        rust_VControl_ProcessKeyDown(keycode);
        assert_ne!(target, 0);

        // Key up sets the release bit for this frame; BeginFrame clears it.
        rust_VControl_ProcessKeyUp(keycode);
        rust_VControl_BeginFrame();
        assert_eq!(target, 0);

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_key_binding_persistence() {
    unsafe {
        rust_VControl_Init();

        let mut target: i32 = 0;
        let keycode = 32;

        rust_VControl_AddKeyBinding(keycode, &mut target);

        // Multiple key down events
        rust_VControl_ProcessKeyDown(keycode);
        rust_VControl_BeginFrame(); // Clear start bit
        rust_VControl_BeginFrame(); // Should still be 1 (key held)
        assert_eq!(target, 1);

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_joystick() {
    unsafe {
        rust_VControl_Init();

        let name = CString::new("Test Joystick").unwrap();
        assert_eq!(rust_VControl_InitJoystick(0, name.as_ptr(), 2, 10, 1), 0);

        assert_eq!(rust_VControl_GetNumJoysticks(), 1);

        let mut target: i32 = 0;
        assert_eq!(rust_VControl_AddJoyButtonBinding(0, 0, &mut target), 0);

        // Simulate button press
        rust_VControl_ProcessJoyButtonDown(0, 0);
        assert_eq!(target, 1);

        // Simulate button release
        rust_VControl_ProcessJoyButtonUp(0, 0);
        assert_eq!(target, 0);

        rust_VControl_UninitJoystick(0);
        assert_eq!(rust_VControl_GetNumJoysticks(), 0);

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_joystick_axis() {
    unsafe {
        rust_VControl_Init();

        let name = CString::new("Test Joy").unwrap();
        rust_VControl_InitJoystick(0, name.as_ptr(), 2, 0, 0);

        let mut neg_target: i32 = 0;
        let mut pos_target: i32 = 0;

        rust_VControl_AddJoyAxisBinding(0, 0, -1, &mut neg_target);
        rust_VControl_AddJoyAxisBinding(0, 0, 1, &mut pos_target);

        // Set threshold
        assert_eq!(rust_VControl_SetJoyThreshold(0, 5000), 0);

        // Push axis negative
        rust_VControl_ProcessJoyAxis(0, 0, -20000);
        assert_eq!(neg_target, 1);
        assert_eq!(pos_target, 0);

        // Center
        rust_VControl_ProcessJoyAxis(0, 0, 0);
        assert_eq!(neg_target, 0);
        assert_eq!(pos_target, 0);

        // Push axis positive
        rust_VControl_ProcessJoyAxis(0, 0, 20000);
        assert_eq!(neg_target, 0);
        assert_eq!(pos_target, 1);

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_reset_states() {
    unsafe {
        rust_VControl_Init();

        let mut target: i32 = 5;
        rust_VControl_AddKeyBinding(32, &mut target);

        rust_VControl_ResetInput();
        assert_eq!(target, 0);

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_clear_bindings() {
    unsafe {
        rust_VControl_Init();

        let mut target1: i32 = 0;
        let mut target2: i32 = 0;

        rust_VControl_AddKeyBinding(32, &mut target1);
        rust_VControl_AddKeyBinding(65, &mut target2);

        // Press keys
        rust_VControl_ProcessKeyDown(32);
        rust_VControl_ProcessKeyDown(65);
        assert_ne!(target1, 0);
        assert_ne!(target2, 0);

        // Clear bindings and reset the borrowed targets explicitly.
        rust_VControl_ClearKeyBindings();
        target1 = 0;
        target2 = 0;

        // Events for former bindings must leave both targets unchanged.
        rust_VControl_ProcessKeyUp(32);
        rust_VControl_ProcessKeyUp(65);
        rust_VControl_ProcessKeyDown(32);
        rust_VControl_ProcessKeyDown(65);
        assert_eq!(target1, 0);
        assert_eq!(target2, 0);

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_gesture_tracking() {
    unsafe {
        rust_VControl_Init();

        // No gesture initially
        rust_VControl_ClearGesture();
        assert_eq!(rust_VControl_GetLastGestureType(), 0);

        // Simulate key event (this would normally be done via HandleEvent)
        // For this test we just verify the FFI is callable
        rust_VControl_ClearGesture();

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}

#[test]
#[serial]
fn test_ffi_begin_frame() {
    unsafe {
        rust_VControl_Init();

        let mut target: i32 = 0;
        rust_VControl_AddKeyBinding(32, &mut target);

        // Key down includes both the held and newly-pressed bits.
        rust_VControl_ProcessKeyDown(32);
        assert_ne!(target, 0);

        // Begin frame clears start bit but key is still held
        // (In real usage, the key state would be tracked by SDL)
        rust_VControl_BeginFrame();

        rust_VControl_ClearKeyBindings();
        rust_VControl_Uninit();
    }
}
