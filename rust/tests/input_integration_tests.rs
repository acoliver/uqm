//! Integration tests for Rust Input/VControl system
//!
//! These tests verify that the FFI layer correctly exposes the Rust
//! input system to C code, and that the integration with SDL events
//! works as expected.

use std::ffi::CString;

use serial_test::serial;

use uqm_rust::input::ffi::{
    rust_v_control_init, rust_v_control_uninit, rust_vcontrol_add_joy_axis_binding,
    rust_vcontrol_add_joy_button_binding, rust_vcontrol_add_key_binding, rust_vcontrol_begin_frame,
    rust_vcontrol_clear_gesture, rust_vcontrol_clear_key_bindings,
    rust_vcontrol_get_last_gesture_type, rust_vcontrol_get_num_joysticks,
    rust_vcontrol_init_joystick, rust_vcontrol_process_joy_axis,
    rust_vcontrol_process_joy_button_down, rust_vcontrol_process_joy_button_up,
    rust_vcontrol_process_key_down, rust_vcontrol_process_key_up, rust_vcontrol_reset_input,
    rust_vcontrol_set_joy_threshold, rust_vcontrol_uninit_joystick,
};

#[test]
#[serial]
fn test_ffi_init_uninit() {
    unsafe {
        assert_eq!(rust_v_control_init(), 0);
        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_key_binding() {
    unsafe {
        rust_v_control_init();

        let mut target: i32 = 0;
        let keycode = 32; // Space key

        assert_eq!(rust_vcontrol_add_key_binding(keycode, &mut target), 0);

        // Simulate key down
        rust_vcontrol_process_key_down(keycode);
        assert_ne!(target, 0);

        // Key up sets the release bit for this frame; BeginFrame clears it.
        rust_vcontrol_process_key_up(keycode);
        rust_vcontrol_begin_frame();
        assert_eq!(target, 0);

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_key_binding_persistence() {
    unsafe {
        rust_v_control_init();

        let mut target: i32 = 0;
        let keycode = 32;

        rust_vcontrol_add_key_binding(keycode, &mut target);

        // Multiple key down events
        rust_vcontrol_process_key_down(keycode);
        rust_vcontrol_begin_frame(); // Clear start bit
        rust_vcontrol_begin_frame(); // Should still be 1 (key held)
        assert_eq!(target, 1);

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_joystick() {
    unsafe {
        rust_v_control_init();

        let name = CString::new("Test Joystick").unwrap();
        assert_eq!(rust_vcontrol_init_joystick(0, name.as_ptr(), 2, 10, 1), 0);

        assert_eq!(rust_vcontrol_get_num_joysticks(), 1);

        let mut target: i32 = 0;
        assert_eq!(rust_vcontrol_add_joy_button_binding(0, 0, &mut target), 0);

        // Simulate button press
        rust_vcontrol_process_joy_button_down(0, 0);
        assert_eq!(target, 1);

        // Simulate button release
        rust_vcontrol_process_joy_button_up(0, 0);
        assert_eq!(target, 0);

        rust_vcontrol_uninit_joystick(0);
        assert_eq!(rust_vcontrol_get_num_joysticks(), 0);

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_joystick_axis() {
    unsafe {
        rust_v_control_init();

        let name = CString::new("Test Joy").unwrap();
        rust_vcontrol_init_joystick(0, name.as_ptr(), 2, 0, 0);

        let mut neg_target: i32 = 0;
        let mut pos_target: i32 = 0;

        rust_vcontrol_add_joy_axis_binding(0, 0, -1, &mut neg_target);
        rust_vcontrol_add_joy_axis_binding(0, 0, 1, &mut pos_target);

        // Set threshold
        assert_eq!(rust_vcontrol_set_joy_threshold(0, 5000), 0);

        // Push axis negative
        rust_vcontrol_process_joy_axis(0, 0, -20000);
        assert_eq!(neg_target, 1);
        assert_eq!(pos_target, 0);

        // Center
        rust_vcontrol_process_joy_axis(0, 0, 0);
        assert_eq!(neg_target, 0);
        assert_eq!(pos_target, 0);

        // Push axis positive
        rust_vcontrol_process_joy_axis(0, 0, 20000);
        assert_eq!(neg_target, 0);
        assert_eq!(pos_target, 1);

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_reset_states() {
    unsafe {
        rust_v_control_init();

        let mut target: i32 = 5;
        rust_vcontrol_add_key_binding(32, &mut target);

        rust_vcontrol_reset_input();
        assert_eq!(target, 0);

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_clear_bindings() {
    unsafe {
        rust_v_control_init();

        let mut target1: i32 = 0;
        let mut target2: i32 = 0;

        rust_vcontrol_add_key_binding(32, &mut target1);
        rust_vcontrol_add_key_binding(65, &mut target2);

        // Press keys
        rust_vcontrol_process_key_down(32);
        rust_vcontrol_process_key_down(65);
        assert_ne!(target1, 0);
        assert_ne!(target2, 0);

        // Clear bindings and reset the borrowed targets explicitly.
        rust_vcontrol_clear_key_bindings();
        target1 = 0;
        target2 = 0;

        // Events for former bindings must leave both targets unchanged.
        rust_vcontrol_process_key_up(32);
        rust_vcontrol_process_key_up(65);
        rust_vcontrol_process_key_down(32);
        rust_vcontrol_process_key_down(65);
        assert_eq!(target1, 0);
        assert_eq!(target2, 0);

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_gesture_tracking() {
    unsafe {
        rust_v_control_init();

        // No gesture initially
        rust_vcontrol_clear_gesture();
        assert_eq!(rust_vcontrol_get_last_gesture_type(), 0);

        // Simulate key event (this would normally be done via HandleEvent)
        // For this test we just verify the FFI is callable
        rust_vcontrol_clear_gesture();

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}

#[test]
#[serial]
fn test_ffi_begin_frame() {
    unsafe {
        rust_v_control_init();

        let mut target: i32 = 0;
        rust_vcontrol_add_key_binding(32, &mut target);

        // Key down includes both the held and newly-pressed bits.
        rust_vcontrol_process_key_down(32);
        assert_ne!(target, 0);

        // Begin frame clears start bit but key is still held
        // (In real usage, the key state would be tracked by SDL)
        rust_vcontrol_begin_frame();

        rust_vcontrol_clear_key_bindings();
        rust_v_control_uninit();
    }
}
