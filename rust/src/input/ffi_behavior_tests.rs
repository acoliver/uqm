//! The C-facing VControl entry points driven the way the game's input layer
//! drives them: bindings registered against real control-state words, then
//! raw SDL events and key/joystick calls delivered, with the words, return
//! codes and recorded gesture checked.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::mem::zeroed;
use std::ptr;

use serial_test::serial;

use super::c_api::{vcontrol_dump_gesture, vcontrol_parse_gesture};
use super::ffi::*;
use super::vcontrol::VCONTROL;

const MASK: i32 = 0xFF;
const SPACE: c_int = 32;
const HAT_UP: u8 = 0x01;
const HAT_RIGHT: u8 = 0x02;
const HAT_DOWN: u8 = 0x04;
const HAT_LEFT: u8 = 0x08;

/// Starts and ends every test from an uninitialised input system, so global
/// state from other tests cannot leak in.
struct Input;

impl Input {
    fn new() -> Self {
        unsafe {
            rust_v_control_uninit();
            assert_eq!(rust_v_control_init(), 0);
        }
        Input
    }

    fn with_joystick(axes: c_int, buttons: c_int, hats: c_int) -> Self {
        let input = Self::new();
        unsafe {
            assert_eq!(
                rust_vcontrol_init_joystick(0, c"Pad".as_ptr(), axes, buttons, hats),
                0
            );
        }
        input
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        unsafe { rust_v_control_uninit() };
    }
}

fn parse(spec: &str) -> VCONTROL_GESTURE {
    let mut gesture: VCONTROL_GESTURE = unsafe { zeroed() };
    let spec = CString::new(spec).unwrap();
    unsafe { vcontrol_parse_gesture(&mut gesture, spec.as_ptr()) };
    gesture
}

fn dump(gesture: &mut VCONTROL_GESTURE, capacity: c_int) -> (c_int, String) {
    let mut buf = vec![0x7fu8; 64];
    let written =
        unsafe { vcontrol_dump_gesture(buf.as_mut_ptr() as *mut c_char, capacity, gesture) };
    let text = CStr::from_bytes_until_nul(&buf)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    (written, text)
}

fn last_gesture_type() -> c_int {
    unsafe { rust_vcontrol_get_last_gesture_type() }
}

fn key_event(kind: u32, sym: i32, repeat: u8) -> sdl2::sys::SDL_Event {
    let mut event: sdl2::sys::SDL_Event = unsafe { zeroed() };
    {
        event.key.type_ = kind;
        event.key.keysym.sym = sym;
        event.key.repeat = repeat;
    }
    event
}

fn deliver(event: &sdl2::sys::SDL_Event) {
    unsafe { rust_vcontrol_handle_event(event as *const _ as *const c_void) };
}

#[test]
#[serial]
fn the_control_system_reports_initialised_only_between_init_and_uninit() {
    unsafe {
        rust_v_control_uninit();
        assert!(!VCONTROL.read().is_initialized());
        assert_eq!(rust_v_control_init(), 0);
        assert!(VCONTROL.read().is_initialized());
        rust_v_control_uninit();
        assert!(!VCONTROL.read().is_initialized());
    }
}

#[test]
#[serial]
fn a_key_can_feed_two_targets_and_each_is_removed_on_its_own() {
    let _input = Input::new();
    let (mut a, mut b) = (0i32, 0i32);
    unsafe {
        assert_eq!(rust_vcontrol_add_key_binding(SPACE, &mut a), 0);
        assert_eq!(rust_vcontrol_add_key_binding(SPACE, &mut b), 0);
        rust_vcontrol_process_key_down(SPACE);
        assert_eq!((a & MASK, b & MASK), (1, 1));
        rust_vcontrol_process_key_up(SPACE);
        assert_eq!((a & MASK, b & MASK), (0, 0));

        assert_eq!(rust_vcontrol_remove_key_binding(SPACE, &mut a), 0);
        rust_vcontrol_process_key_down(SPACE);
        assert_eq!(
            (a & MASK, b & MASK),
            (0, 1),
            "only the remaining target moves"
        );
        assert_eq!(
            rust_vcontrol_remove_key_binding(SPACE, &mut a),
            -1,
            "a second removal finds nothing"
        );
    }
}

#[test]
#[serial]
fn a_key_press_sets_a_start_bit_that_the_next_frame_clears() {
    let _input = Input::new();
    let mut target = 0i32;
    unsafe {
        rust_vcontrol_add_key_binding(SPACE, &mut target);
        rust_vcontrol_process_key_down(SPACE);
        assert_ne!(target & !MASK, 0, "the press is flagged as new this frame");
        rust_vcontrol_begin_frame();
        assert_eq!(target & !MASK, 0);
        assert_eq!(target & MASK, 1, "the key is still held");
    }
}

#[test]
#[serial]
fn reset_input_releases_keys_and_joystick_controls_alike() {
    let _input = Input::with_joystick(1, 1, 1);
    let (mut key, mut button, mut axis, mut hat) = (1i32, 1i32, 1i32, 1i32);
    unsafe {
        rust_vcontrol_add_key_binding(SPACE, &mut key);
        rust_vcontrol_add_joy_button_binding(0, 0, &mut button);
        rust_vcontrol_add_joy_axis_binding(0, 0, 1, &mut axis);
        rust_vcontrol_add_joy_hat_binding(0, 0, HAT_UP, &mut hat);
        rust_vcontrol_reset_input();
    }
    assert_eq!((key, button, axis, hat), (0, 0, 0, 0));
}

#[test]
#[serial]
fn the_joystick_registry_counts_and_bounds_its_pads() {
    let _input = Input::new();
    unsafe {
        assert_eq!(rust_vcontrol_get_num_joysticks(), 0);
        assert_eq!(rust_vcontrol_init_joystick(0, c"A".as_ptr(), 2, 4, 1), 0);
        assert_eq!(rust_vcontrol_init_joystick(3, ptr::null(), 0, 0, 0), 0);
        assert_eq!(rust_vcontrol_get_num_joysticks(), 2);
        assert_eq!(
            VCONTROL.read().get_joystick(3).unwrap().name,
            "Unknown",
            "a null name is replaced"
        );
        assert_eq!(VCONTROL.read().get_joystick(0).unwrap().name, "A");
        assert_eq!(rust_vcontrol_init_joystick(8, c"Z".as_ptr(), 1, 1, 1), -1);
        assert_eq!(rust_vcontrol_init_joystick(-1, c"Z".as_ptr(), 1, 1, 1), -1);
        assert_eq!(rust_vcontrol_uninit_joystick(3), 0);
        assert_eq!(rust_vcontrol_uninit_joystick(99), -1);
        assert_eq!(rust_vcontrol_get_num_joysticks(), 1);
    }
}

#[test]
#[serial]
fn a_name_that_is_not_utf8_falls_back_to_unknown() {
    let _input = Input::new();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        assert_eq!(
            rust_vcontrol_init_joystick(1, bad.as_ptr() as *const c_char, 1, 1, 1),
            0
        );
    }
    assert_eq!(VCONTROL.read().get_joystick(1).unwrap().name, "Unknown");
}

#[test]
#[serial]
fn button_bindings_report_added_duplicate_removed_and_missing() {
    let _input = Input::with_joystick(0, 4, 0);
    let mut target = 0i32;
    unsafe {
        assert_eq!(rust_vcontrol_add_joy_button_binding(0, 2, &mut target), 0);
        assert_eq!(rust_vcontrol_add_joy_button_binding(0, 2, &mut target), 1);
        assert_eq!(
            rust_vcontrol_add_joy_button_binding(5, 2, &mut target),
            -1,
            "no such pad"
        );

        rust_vcontrol_process_joy_button_down(0, 2);
        assert_eq!(target, 1);
        rust_vcontrol_process_joy_button_up(0, 2);
        assert_eq!(target, 0);
        rust_vcontrol_process_joy_button_down(0, 3);
        assert_eq!(target, 0, "another button does not reach it");
        rust_vcontrol_process_joy_button_down(0, 9);
        rust_vcontrol_process_joy_button_down(7, 0);

        assert_eq!(
            rust_vcontrol_remove_joy_button_binding(0, 2, &mut target),
            0
        );
        assert_eq!(
            rust_vcontrol_remove_joy_button_binding(0, 2, &mut target),
            1
        );
        assert_eq!(
            rust_vcontrol_remove_joy_button_binding(5, 2, &mut target),
            -1
        );
        rust_vcontrol_process_joy_button_down(0, 2);
        assert_eq!(target, 0);
    }
}

#[test]
#[serial]
fn axis_bindings_follow_the_stick_through_the_dead_zone_threshold() {
    let _input = Input::with_joystick(2, 0, 0);
    let (mut neg, mut pos) = (0i32, 0i32);
    unsafe {
        assert_eq!(rust_vcontrol_add_joy_axis_binding(0, 1, -1, &mut neg), 0);
        assert_eq!(rust_vcontrol_add_joy_axis_binding(0, 1, -1, &mut neg), 1);
        assert_eq!(rust_vcontrol_add_joy_axis_binding(0, 1, 1, &mut pos), 0);
        assert_eq!(rust_vcontrol_add_joy_axis_binding(4, 1, 1, &mut pos), -1);

        rust_vcontrol_process_joy_axis(0, 1, 9_000);
        assert_eq!((neg, pos), (0, 0), "inside the default dead zone");
        assert_eq!(rust_vcontrol_set_joy_threshold(0, 5_000), 0);
        rust_vcontrol_process_joy_axis(0, 1, 9_000);
        assert_eq!((neg, pos), (0, 1), "outside the tighter dead zone");
        rust_vcontrol_process_joy_axis(0, 1, -9_000);
        assert_eq!((neg, pos), (1, 0));
        rust_vcontrol_process_joy_axis(0, 0, 30_000);
        assert_eq!((neg, pos), (1, 0), "another axis is unbound");
        assert_eq!(rust_vcontrol_set_joy_threshold(6, 5_000), -1);

        assert_eq!(rust_vcontrol_remove_joy_axis_binding(0, 1, -1, &mut neg), 0);
        assert_eq!(rust_vcontrol_remove_joy_axis_binding(0, 1, -1, &mut neg), 1);
        assert_eq!(
            rust_vcontrol_remove_joy_axis_binding(6, 1, -1, &mut neg),
            -1
        );
    }
}

#[test]
#[serial]
fn hat_bindings_fire_per_direction_and_release_when_the_hat_centres() {
    let _input = Input::with_joystick(0, 0, 1);
    let (mut up, mut right, mut down, mut left) = (0i32, 0i32, 0i32, 0i32);
    unsafe {
        assert_eq!(rust_vcontrol_add_joy_hat_binding(0, 0, HAT_UP, &mut up), 0);
        assert_eq!(
            rust_vcontrol_add_joy_hat_binding(0, 0, HAT_RIGHT, &mut right),
            0
        );
        assert_eq!(
            rust_vcontrol_add_joy_hat_binding(0, 0, HAT_DOWN, &mut down),
            0
        );
        assert_eq!(
            rust_vcontrol_add_joy_hat_binding(0, 0, HAT_LEFT, &mut left),
            0
        );
        assert_eq!(
            rust_vcontrol_add_joy_hat_binding(0, 0, HAT_LEFT, &mut left),
            1
        );
        assert_eq!(
            rust_vcontrol_add_joy_hat_binding(2, 0, HAT_LEFT, &mut left),
            -1
        );

        rust_vcontrol_process_joy_hat(0, 0, HAT_UP | HAT_RIGHT);
        assert_eq!((up, right, down, left), (1, 1, 0, 0));
        rust_vcontrol_process_joy_hat(0, 0, HAT_RIGHT | HAT_DOWN);
        assert_eq!((up, right, down, left), (0, 1, 1, 0));
        rust_vcontrol_process_joy_hat(0, 0, 0);
        assert_eq!((up, right, down, left), (0, 0, 0, 0));
        rust_vcontrol_process_joy_hat(0, 3, HAT_UP);
        rust_vcontrol_process_joy_hat(3, 0, HAT_UP);
        assert_eq!(up, 0, "unknown hats and pads are ignored");

        assert_eq!(
            rust_vcontrol_remove_joy_hat_binding(0, 0, HAT_UP, &mut up),
            0
        );
        assert_eq!(
            rust_vcontrol_remove_joy_hat_binding(0, 0, HAT_UP, &mut up),
            1
        );
        assert_eq!(
            rust_vcontrol_remove_joy_hat_binding(2, 0, HAT_UP, &mut up),
            -1
        );
        rust_vcontrol_process_joy_hat(0, 0, HAT_UP);
        assert_eq!(up, 0);
    }
}

#[test]
#[serial]
fn clearing_one_pad_leaves_the_keyboard_and_other_pads_bound() {
    let _input = Input::with_joystick(1, 2, 1);
    unsafe {
        assert_eq!(rust_vcontrol_init_joystick(1, c"Two".as_ptr(), 1, 2, 1), 0);
    }
    let (mut key, mut b0, mut b1) = (0i32, 0i32, 0i32);
    unsafe {
        rust_vcontrol_add_key_binding(SPACE, &mut key);
        rust_vcontrol_add_joy_button_binding(0, 0, &mut b0);
        rust_vcontrol_add_joy_button_binding(1, 0, &mut b1);
        assert_eq!(rust_vcontrol_clear_joy_bindings(0), 0);
        assert_eq!(rust_vcontrol_clear_joy_bindings(6), -1);
        rust_vcontrol_process_joy_button_down(0, 0);
        rust_vcontrol_process_joy_button_down(1, 0);
        rust_vcontrol_process_key_down(SPACE);
    }
    assert_eq!(b0, 0);
    assert_eq!(b1, 1);
    assert_eq!(key & MASK, 1);
}

#[test]
#[serial]
fn removing_all_bindings_silences_keys_and_every_pad() {
    let _input = Input::with_joystick(1, 1, 1);
    let (mut key, mut button) = (0i32, 0i32);
    unsafe {
        rust_vcontrol_add_key_binding(SPACE, &mut key);
        rust_vcontrol_add_joy_button_binding(0, 0, &mut button);
        rust_vcontrol_remove_all_bindings();
        rust_vcontrol_process_key_down(SPACE);
        rust_vcontrol_process_joy_button_down(0, 0);
    }
    assert_eq!((key, button), (0, 0));
    assert_eq!(
        unsafe { rust_vcontrol_get_num_joysticks() },
        1,
        "the pad itself stays"
    );
}

#[test]
#[serial]
fn gesture_conversion_round_trips_every_kind() {
    use super::vcontrol::Gesture;
    let gestures = [
        Gesture::Key(SPACE),
        Gesture::JoyAxis {
            port: 2,
            axis: 3,
            polarity: -1,
        },
        Gesture::JoyButton { port: 1, button: 7 },
        Gesture::JoyHat {
            port: 3,
            hat: 1,
            dir: HAT_LEFT,
        },
    ];
    for gesture in gestures {
        let c = VCONTROL_GESTURE::from_gesture(&gesture);
        assert_eq!(c.to_gesture(), Some(gesture));
    }
    let none = VCONTROL_GESTURE {
        gesture_type: VCONTROL_NONE,
        gesture: GestureUnion { data: [0; 3] },
    };
    assert_eq!(none.to_gesture(), None);
    let unknown = VCONTROL_GESTURE {
        gesture_type: 99,
        gesture: GestureUnion { data: [0; 3] },
    };
    assert_eq!(unknown.to_gesture(), None);
}

#[test]
#[serial]
fn keyboard_events_drive_bound_targets_and_remember_the_gesture() {
    let _input = Input::new();
    let mut target = 0i32;
    unsafe { rust_vcontrol_add_key_binding(SPACE, &mut target) };
    assert_eq!(last_gesture_type(), 0);

    deliver(&key_event(0x300, SPACE, 0));
    assert_eq!(target & MASK, 1);
    assert_eq!(last_gesture_type(), 1);

    let mut out: VCONTROL_GESTURE = unsafe { zeroed() };
    assert_eq!(unsafe { rust_vcontrol_get_last_gesture(&mut out) }, 1);
    assert_eq!(out.gesture_type, VCONTROL_KEY);
    assert_eq!(unsafe { out.gesture.key }, SPACE);

    deliver(&key_event(0x301, SPACE, 0));
    assert_eq!(target & MASK, 0);

    unsafe { rust_vcontrol_clear_gesture() };
    assert_eq!(last_gesture_type(), 0);
    assert_eq!(unsafe { rust_vcontrol_get_last_gesture(&mut out) }, 0);
}

#[test]
#[serial]
fn a_repeated_key_down_neither_presses_again_nor_counts_as_a_gesture() {
    let _input = Input::new();
    let mut target = 0i32;
    unsafe { rust_vcontrol_add_key_binding(SPACE, &mut target) };
    deliver(&key_event(0x300, SPACE, 1));
    assert_eq!(target & MASK, 0);
    assert_eq!(last_gesture_type(), 0);
}

#[test]
#[serial]
fn joystick_events_drive_bindings_and_only_deliberate_moves_become_gestures() {
    let _input = Input::with_joystick(2, 2, 1);
    let (mut button, mut axis_pos, mut hat_up) = (0i32, 0i32, 0i32);
    unsafe {
        rust_vcontrol_add_joy_button_binding(0, 1, &mut button);
        rust_vcontrol_add_joy_axis_binding(0, 0, 1, &mut axis_pos);
        rust_vcontrol_add_joy_hat_binding(0, 0, HAT_UP, &mut hat_up);
    }

    let mut event: sdl2::sys::SDL_Event = unsafe { zeroed() };
    {
        event.jaxis.type_ = 0x600;
        event.jaxis.which = 0;
        event.jaxis.axis = 0;
        event.jaxis.value = 12_000;
    }
    deliver(&event);
    assert_eq!(axis_pos, 1);
    assert_eq!(
        last_gesture_type(),
        0,
        "a small push is not a deliberate gesture"
    );

    event.jaxis.value = 20_000;
    deliver(&event);
    assert_eq!(last_gesture_type(), 2);
    let mut out: VCONTROL_GESTURE = unsafe { zeroed() };
    unsafe { rust_vcontrol_get_last_gesture(&mut out) };
    unsafe {
        assert_eq!(
            (
                out.gesture.axis.port,
                out.gesture.axis.index,
                out.gesture.axis.polarity
            ),
            (0, 0, 1)
        );
    }

    event.jaxis.value = -20_000;
    deliver(&event);
    unsafe { rust_vcontrol_get_last_gesture(&mut out) };
    assert_eq!(unsafe { out.gesture.axis.polarity }, -1);

    let mut hat: sdl2::sys::SDL_Event = unsafe { zeroed() };
    {
        hat.jhat.type_ = 0x602;
        hat.jhat.which = 0;
        hat.jhat.hat = 0;
        hat.jhat.value = HAT_UP;
    }
    deliver(&hat);
    assert_eq!(hat_up, 1);
    assert_eq!(last_gesture_type(), 4);
    unsafe { rust_vcontrol_get_last_gesture(&mut out) };
    unsafe { assert_eq!((out.gesture.hat.index, out.gesture.hat.dir), (0, HAT_UP)) };

    let mut press: sdl2::sys::SDL_Event = unsafe { zeroed() };
    {
        press.jbutton.type_ = 0x603;
        press.jbutton.which = 0;
        press.jbutton.button = 1;
    }
    deliver(&press);
    assert_eq!(button, 1);
    assert_eq!(last_gesture_type(), 3);
    press.jbutton.type_ = 0x604;
    deliver(&press);
    assert_eq!(button, 0);
}

#[test]
#[serial]
fn events_of_other_kinds_and_null_events_change_nothing() {
    let _input = Input::new();
    let mut event: sdl2::sys::SDL_Event = unsafe { zeroed() };
    event.type_ = 0x400;
    deliver(&event);
    unsafe { rust_vcontrol_handle_event(ptr::null()) };
    assert_eq!(last_gesture_type(), 0);
}

#[test]
#[serial]
fn getting_the_last_gesture_into_nowhere_reports_nothing() {
    let _input = Input::new();
    assert_eq!(
        unsafe { rust_vcontrol_get_last_gesture(ptr::null_mut()) },
        0
    );
}

#[test]
#[serial]
fn gesture_bindings_register_and_unregister_each_kind_of_control() {
    let _input = Input::with_joystick(1, 1, 1);
    let (mut key, mut axis, mut button, mut hat) = (0i32, 0i32, 0i32, 0i32);
    unsafe {
        let mut g = parse("key Space");
        assert_eq!(rust_vcontrol_add_gesture_binding(&mut g, &mut key), 0);
        let mut g_axis = parse("joystick 0 axis 0 positive");
        assert_eq!(rust_vcontrol_add_gesture_binding(&mut g_axis, &mut axis), 0);
        let mut g_button = parse("joystick 0 button 0");
        assert_eq!(
            rust_vcontrol_add_gesture_binding(&mut g_button, &mut button),
            0
        );
        let mut g_hat = parse("joystick 0 hat 0 left");
        assert_eq!(rust_vcontrol_add_gesture_binding(&mut g_hat, &mut hat), 0);

        rust_vcontrol_process_key_down(SPACE);
        rust_vcontrol_process_joy_axis(0, 0, 30_000);
        rust_vcontrol_process_joy_button_down(0, 0);
        rust_vcontrol_process_joy_hat(0, 0, HAT_LEFT);
        assert_eq!((key & MASK, axis, button, hat), (1, 1, 1, 1));

        rust_vcontrol_remove_gesture_binding(&mut g, &mut key);
        rust_vcontrol_remove_gesture_binding(&mut g_axis, &mut axis);
        rust_vcontrol_remove_gesture_binding(&mut g_button, &mut button);
        rust_vcontrol_remove_gesture_binding(&mut g_hat, &mut hat);
        (key, axis, button, hat) = (0, 0, 0, 0);
        rust_vcontrol_process_key_down(SPACE);
        rust_vcontrol_process_joy_axis(0, 0, -30_000);
        rust_vcontrol_process_joy_axis(0, 0, 30_000);
        rust_vcontrol_process_joy_button_down(0, 0);
        rust_vcontrol_process_joy_hat(0, 0, 0);
        rust_vcontrol_process_joy_hat(0, 0, HAT_LEFT);
        assert_eq!(
            (key, axis, button, hat),
            (0, 0, 0, 0),
            "all four were unbound"
        );
    }
}

#[test]
#[serial]
fn gesture_bindings_refuse_null_unknown_and_unbindable_requests() {
    let _input = Input::new();
    let mut target = 0i32;
    unsafe {
        let mut none = parse("");
        assert_eq!(
            rust_vcontrol_add_gesture_binding(&mut none, &mut target),
            -1
        );
        assert_eq!(
            rust_vcontrol_add_gesture_binding(ptr::null_mut(), &mut target),
            -1
        );
        assert_eq!(
            rust_vcontrol_add_gesture_binding(&mut none, ptr::null_mut()),
            -1
        );
        // No such pad is initialised.
        let mut button = parse("joystick 0 button 0");
        assert_eq!(
            rust_vcontrol_add_gesture_binding(&mut button, &mut target),
            -1
        );
        let mut axis = parse("joystick 0 axis 0 negative");
        assert_eq!(
            rust_vcontrol_add_gesture_binding(&mut axis, &mut target),
            -1
        );
        let mut hat = parse("joystick 0 hat 0 up");
        assert_eq!(rust_vcontrol_add_gesture_binding(&mut hat, &mut target), -1);

        rust_vcontrol_remove_gesture_binding(ptr::null_mut(), &mut target);
        rust_vcontrol_remove_gesture_binding(&mut none, ptr::null_mut());
        rust_vcontrol_remove_gesture_binding(&mut none, &mut target);
    }
}

#[test]
#[serial]
fn parsing_reads_every_documented_gesture_form() {
    let key = parse("key Space");
    assert_eq!(key.gesture_type, VCONTROL_KEY);
    assert_eq!(unsafe { key.gesture.key }, SPACE);
    assert_eq!(
        parse("KEY space").gesture_type,
        VCONTROL_KEY,
        "case is ignored"
    );

    let negative = parse("joystick 2 axis 3 negative");
    assert_eq!(negative.gesture_type, 2);
    unsafe {
        assert_eq!(
            (
                negative.gesture.axis.port,
                negative.gesture.axis.index,
                negative.gesture.axis.polarity
            ),
            (2, 3, -1)
        );
    }
    let positive = parse("Joystick 0 Axis 1 Positive");
    unsafe { assert_eq!(positive.gesture.axis.polarity, 1) };

    let button = parse("joystick 1 button 9");
    assert_eq!(button.gesture_type, 3);
    unsafe {
        assert_eq!(
            (button.gesture.button.port, button.gesture.button.index),
            (1, 9)
        )
    };

    for (word, dir) in [
        ("up", HAT_UP),
        ("down", HAT_DOWN),
        ("left", HAT_LEFT),
        ("right", HAT_RIGHT),
    ] {
        let hat = parse(&format!("joystick 0 hat 2 {word}"));
        assert_eq!(hat.gesture_type, 4, "{word}");
        unsafe {
            assert_eq!(
                (hat.gesture.hat.index, hat.gesture.hat.dir),
                (2, dir),
                "{word}"
            )
        };
    }
}

#[test]
#[serial]
fn parsing_malformed_specs_leaves_the_gesture_empty() {
    for spec in [
        "",
        "   ",
        "key",
        "key NoSuchKeyAnywhere",
        "mouse 1",
        "joystick",
        "joystick x button 1",
        "joystick 0 button x",
        "joystick 0 button",
        "joystick 0 axis x positive",
        "joystick 0 axis 1 sideways",
        "joystick 0 axis 1",
        "joystick 0 hat x up",
        "joystick 0 hat 1 diagonal",
        "joystick 0 hat 1",
        "joystick 0 trackball 1 up",
    ] {
        let gesture = parse(spec);
        assert_eq!(gesture.gesture_type, VCONTROL_NONE, "{spec:?}");
        assert_eq!(unsafe { gesture.gesture.data }, [0, 0, 0], "{spec:?}");
    }
}

#[test]
#[serial]
fn parsing_clears_whatever_the_gesture_held_before() {
    let mut gesture = parse("key Space");
    let spec = c"nonsense";
    unsafe { vcontrol_parse_gesture(&mut gesture, spec.as_ptr()) };
    assert_eq!(gesture.gesture_type, VCONTROL_NONE);
    unsafe {
        rust_vcontrol_parse_gesture(ptr::null_mut(), spec.as_ptr());
        rust_vcontrol_parse_gesture(&mut gesture, ptr::null());
    }
    let bad = [0xffu8, 0];
    let mut held = parse("key Space");
    unsafe { rust_vcontrol_parse_gesture(&mut held, bad.as_ptr() as *const c_char) };
    assert_eq!(
        held.gesture_type, VCONTROL_KEY,
        "an undecodable spec is ignored outright"
    );
}

#[test]
#[serial]
fn dumping_writes_the_text_form_that_parsing_reads() {
    for spec in [
        "key Space",
        "joystick 2 axis 3 negative",
        "joystick 0 axis 1 positive",
        "joystick 1 button 9",
        "joystick 0 hat 2 up",
        "joystick 0 hat 2 down",
        "joystick 0 hat 0 left",
        "joystick 3 hat 1 right",
    ] {
        let mut gesture = parse(spec);
        let (written, text) = dump(&mut gesture, 64);
        assert_eq!(text, spec);
        assert_eq!(written as usize, spec.len());
    }
}

#[test]
#[serial]
fn dumping_truncates_to_the_buffer_and_always_terminates() {
    let mut gesture = parse("joystick 2 axis 3 negative");
    let (written, text) = dump(&mut gesture, 9);
    assert_eq!((written, text.as_str()), (8, "joystick"));
    let (written, text) = dump(&mut gesture, 1);
    assert_eq!((written, text.as_str()), (0, ""));
}

#[test]
#[serial]
fn dumping_an_empty_or_unreadable_request_writes_nothing() {
    let mut empty = parse("");
    let mut buf = [0x55u8; 4];
    unsafe {
        assert_eq!(
            rust_vcontrol_dump_gesture(buf.as_mut_ptr() as *mut c_char, 4, &mut empty),
            0
        );
        assert_eq!(buf[0], 0, "an empty gesture terminates the buffer");
        let mut key = parse("key Space");
        buf = [0x55; 4];
        assert_eq!(rust_vcontrol_dump_gesture(ptr::null_mut(), 4, &mut key), 0);
        assert_eq!(
            rust_vcontrol_dump_gesture(buf.as_mut_ptr() as *mut c_char, 4, ptr::null_mut()),
            0
        );
        assert_eq!(
            rust_vcontrol_dump_gesture(buf.as_mut_ptr() as *mut c_char, 0, &mut key),
            0
        );
        assert_eq!(buf, [0x55; 4]);
    }
}

#[test]
#[serial]
fn a_hat_gesture_with_an_odd_direction_dumps_as_unknown() {
    let mut gesture = parse("joystick 0 hat 0 up");
    gesture.gesture.hat.dir = HAT_UP | HAT_DOWN;
    let (_, text) = dump(&mut gesture, 64);
    assert_eq!(text, "joystick 0 hat 0 unknown");
}

#[test]
#[serial]
fn shutting_the_input_system_down_forgets_the_last_gesture() {
    let _input = Input::new();
    deliver(&key_event(0x300, SPACE, 0));
    assert_eq!(last_gesture_type(), 1);
    unsafe {
        rust_v_control_uninit();
        assert_eq!(rust_v_control_init(), 0);
    }
    assert_eq!(
        last_gesture_type(),
        0,
        "a new session starts without a gesture"
    );
}
