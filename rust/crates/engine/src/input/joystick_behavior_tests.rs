//! Tests that drive joystick bindings against real control-state words and
//! check which words change, and when.

use super::*;

fn addr(word: &mut i32) -> usize {
    word as *mut i32 as usize
}

#[test]
fn a_null_binding_ignores_writes() {
    let binding = JoyBinding::new(0);
    unsafe { binding.set(1) };
}

#[test]
fn removing_axis_bindings_reports_whether_anything_was_removed() {
    let mut axis = JoystickAxis::new();
    axis.add_neg(0x10);
    axis.add_pos(0x20);
    assert!(!axis.remove_neg(0x20));
    assert!(!axis.remove_pos(0x10));
    assert!(axis.remove_neg(0x10));
    assert!(axis.remove_pos(0x20));
    assert!(axis.neg.is_empty() && axis.pos.is_empty());
}

#[test]
fn axis_swings_from_one_side_to_the_other() {
    let (mut neg, mut pos) = (0i32, 0i32);
    let mut axis = JoystickAxis::new();
    axis.add_neg(addr(&mut neg));
    axis.add_pos(addr(&mut pos));
    unsafe {
        axis.handle_value(20_000, 10_000);
        assert_eq!((neg, pos, axis.polarity), (0, 1, 1));
        axis.handle_value(-20_000, 10_000);
        assert_eq!((neg, pos, axis.polarity), (1, 0, -1));
        axis.handle_value(20_000, 10_000);
        assert_eq!((neg, pos, axis.polarity), (0, 1, 1));
    }
}

#[test]
fn axis_only_writes_on_a_change_of_direction() {
    let (mut neg, mut pos) = (0i32, 0i32);
    let mut axis = JoystickAxis::new();
    axis.add_neg(addr(&mut neg));
    axis.add_pos(addr(&mut pos));
    unsafe {
        axis.handle_value(-20_000, 10_000);
        neg = 0; // the game consumed the press
        axis.handle_value(-25_000, 10_000);
        assert_eq!(neg, 0, "a held direction is not re-pressed");
        axis.handle_value(20_000, 10_000);
        pos = 0;
        axis.handle_value(30_000, 10_000);
        assert_eq!(pos, 0);
    }
}

#[test]
fn axis_dead_zone_edges_are_inclusive_and_release_a_held_direction() {
    let (mut neg, mut pos) = (0i32, 0i32);
    let mut axis = JoystickAxis::new();
    axis.add_neg(addr(&mut neg));
    axis.add_pos(addr(&mut pos));
    unsafe {
        axis.handle_value(10_000, 10_000);
        axis.handle_value(-10_000, 10_000);
        assert_eq!((neg, pos, axis.polarity), (0, 0, 0));
        axis.handle_value(10_001, 10_000);
        assert_eq!((neg, pos, axis.polarity), (0, 1, 1));
        axis.handle_value(-10_000, 10_000);
        assert_eq!((neg, pos, axis.polarity), (0, 0, 0));
        axis.handle_value(i16::MIN, 10_000);
        assert_eq!((neg, pos, axis.polarity), (1, 0, -1));
        axis.handle_value(0, 10_000);
        assert_eq!((neg, pos, axis.polarity), (0, 0, 0));
    }
}

#[test]
fn axis_reset_releases_both_directions_and_centers() {
    let (mut neg, mut pos) = (1i32, 1i32);
    let mut axis = JoystickAxis::new();
    axis.add_neg(addr(&mut neg));
    axis.add_pos(addr(&mut pos));
    axis.polarity = 1;
    unsafe { axis.reset() };
    assert_eq!((neg, pos, axis.polarity), (0, 0, 0));
}

#[test]
fn one_axis_direction_can_drive_several_words() {
    let (mut first, mut second) = (0i32, 0i32);
    let mut axis = JoystickAxis::new();
    axis.add_pos(addr(&mut first));
    axis.add_pos(addr(&mut second));
    unsafe { axis.handle_value(30_000, 10_000) };
    assert_eq!((first, second), (1, 1));
}

#[test]
fn hat_rejects_unknown_directions() {
    let mut hat = JoystickHat::new();
    assert!(!hat.add_binding(HatDirection::CENTERED, 0x10));
    assert!(!hat.add_binding(HatDirection::RIGHTUP, 0x10));
    assert!(!hat.remove_binding(HatDirection::RIGHTUP, 0x10));
    assert!(hat.up.is_empty() && hat.right.is_empty());
}

#[test]
fn hat_bindings_are_unique_per_direction() {
    let mut hat = JoystickHat::new();
    for direction in [
        HatDirection::UP,
        HatDirection::DOWN,
        HatDirection::LEFT,
        HatDirection::RIGHT,
    ] {
        assert!(hat.add_binding(direction, 0x10));
        assert!(!hat.add_binding(direction, 0x10));
        assert!(hat.add_binding(direction, 0x20));
    }
    assert_eq!(hat.up.len(), 2);
    assert_eq!(hat.down.len(), 2);
    assert_eq!(hat.left.len(), 2);
    assert_eq!(hat.right.len(), 2);
    assert!(hat.remove_binding(HatDirection::LEFT, 0x10));
    assert!(!hat.remove_binding(HatDirection::LEFT, 0x10));
    assert_eq!(hat.left.len(), 1);
    assert_eq!(hat.right.len(), 2);
}

#[test]
fn hat_diagonals_press_both_directions_and_release_one_at_a_time() {
    let (mut up, mut down, mut left, mut right) = (0i32, 0i32, 0i32, 0i32);
    let mut hat = JoystickHat::new();
    hat.add_binding(HatDirection::UP, addr(&mut up));
    hat.add_binding(HatDirection::DOWN, addr(&mut down));
    hat.add_binding(HatDirection::LEFT, addr(&mut left));
    hat.add_binding(HatDirection::RIGHT, addr(&mut right));
    unsafe {
        hat.handle_value(HatDirection::RIGHTUP);
        assert_eq!((up, down, left, right), (1, 0, 0, 1));
        hat.handle_value(HatDirection::UP);
        assert_eq!((up, down, left, right), (1, 0, 0, 0));
        hat.handle_value(HatDirection::LEFTDOWN);
        assert_eq!((up, down, left, right), (0, 1, 1, 0));
        hat.handle_value(HatDirection::CENTERED);
        assert_eq!((up, down, left, right), (0, 0, 0, 0));
        assert_eq!(hat.last, HatDirection::CENTERED);
    }
}

#[test]
fn hat_only_writes_the_directions_that_changed() {
    let (mut up, mut right) = (0i32, 0i32);
    let mut hat = JoystickHat::new();
    hat.add_binding(HatDirection::UP, addr(&mut up));
    hat.add_binding(HatDirection::RIGHT, addr(&mut right));
    unsafe {
        hat.handle_value(HatDirection::UP);
        up = 0; // consumed by the game
        hat.handle_value(HatDirection::RIGHTUP);
        assert_eq!(up, 0, "up was already held");
        assert_eq!(right, 1);
    }
}

#[test]
fn hat_reset_releases_everything() {
    let (mut up, mut down, mut left, mut right) = (1i32, 1i32, 1i32, 1i32);
    let mut hat = JoystickHat::new();
    hat.add_binding(HatDirection::UP, addr(&mut up));
    hat.add_binding(HatDirection::DOWN, addr(&mut down));
    hat.add_binding(HatDirection::LEFT, addr(&mut left));
    hat.add_binding(HatDirection::RIGHT, addr(&mut right));
    hat.last = HatDirection::LEFTUP;
    unsafe { hat.reset() };
    assert_eq!((up, down, left, right), (0, 0, 0, 0));
    assert_eq!(hat.last, HatDirection::CENTERED);
}

#[test]
fn joystick_starts_open_with_one_slot_per_control() {
    let stick = Joystick::new(3, "pad".to_string(), 2, 5, 1);
    assert_eq!(stick.index, 3);
    assert!(stick.is_open);
    assert_eq!(stick.threshold, 10_000);
    assert_eq!(
        (stick.axes.len(), stick.buttons.len(), stick.hats.len()),
        (2, 5, 1)
    );
}

#[test]
fn joystick_refuses_bindings_for_controls_it_does_not_have() {
    let mut stick = Joystick::new(0, "pad".to_string(), 2, 3, 1);
    assert!(!stick.add_button_binding(-1, 0x10));
    assert!(!stick.add_button_binding(3, 0x10));
    assert!(!stick.remove_button_binding(-1, 0x10));
    assert!(!stick.remove_button_binding(3, 0x10));
    assert!(!stick.add_axis_binding(-1, 1, 0x10));
    assert!(!stick.add_axis_binding(2, 1, 0x10));
    assert!(!stick.remove_axis_binding(-1, 1, 0x10));
    assert!(!stick.remove_axis_binding(2, 1, 0x10));
    assert!(!stick.add_hat_binding(-1, HatDirection::UP, 0x10));
    assert!(!stick.add_hat_binding(1, HatDirection::UP, 0x10));
    assert!(!stick.remove_hat_binding(-1, HatDirection::UP, 0x10));
    assert!(!stick.remove_hat_binding(1, HatDirection::UP, 0x10));
    assert!(stick.buttons.iter().all(|b| b.is_empty()));
}

#[test]
fn joystick_binding_add_and_remove_round_trip() {
    let mut stick = Joystick::new(0, "pad".to_string(), 1, 2, 1);
    assert!(stick.add_button_binding(1, 0x10));
    assert!(!stick.add_button_binding(1, 0x10));
    assert!(stick.add_axis_binding(0, -1, 0x20));
    assert!(stick.add_axis_binding(0, 1, 0x30));
    assert!(stick.add_hat_binding(0, HatDirection::DOWN, 0x40));
    assert_eq!(stick.axes[0].neg.len(), 1);
    assert_eq!(stick.axes[0].pos.len(), 1);
    assert_eq!(stick.hats[0].down.len(), 1);

    assert!(stick.remove_button_binding(1, 0x10));
    assert!(!stick.remove_button_binding(1, 0x10));
    assert!(stick.remove_axis_binding(0, -1, 0x20));
    assert!(!stick.remove_axis_binding(0, -1, 0x20));
    assert!(stick.remove_axis_binding(0, 1, 0x30));
    assert!(stick.remove_hat_binding(0, HatDirection::DOWN, 0x40));
    assert!(stick.buttons[1].is_empty());
    assert!(stick.hats[0].down.is_empty());
}

#[test]
fn joystick_threshold_is_clamped_to_the_axis_range() {
    let mut stick = Joystick::new(0, "pad".to_string(), 1, 0, 0);
    stick.set_threshold(-5);
    assert_eq!(stick.threshold, 0);
    stick.set_threshold(40_000);
    assert_eq!(stick.threshold, 32_767);
    stick.set_threshold(5_000);
    assert_eq!(stick.threshold, 5_000);
}

#[test]
fn joystick_routes_events_to_the_matching_control_only() {
    let (mut button0, mut button1, mut axis_neg, mut axis_pos, mut hat_left) =
        (0i32, 0i32, 0i32, 0i32, 0i32);
    let mut stick = Joystick::new(0, "pad".to_string(), 2, 2, 1);
    stick.add_button_binding(0, addr(&mut button0));
    stick.add_button_binding(1, addr(&mut button1));
    stick.add_axis_binding(1, -1, addr(&mut axis_neg));
    stick.add_axis_binding(1, 1, addr(&mut axis_pos));
    stick.add_hat_binding(0, HatDirection::LEFT, addr(&mut hat_left));
    stick.set_threshold(2_000);
    unsafe {
        stick.handle_button(1, true);
        assert_eq!((button0, button1), (0, 1));
        stick.handle_button(1, false);
        assert_eq!(button1, 0);

        stick.handle_axis(0, 30_000);
        assert_eq!((axis_neg, axis_pos), (0, 0), "axis 0 has no bindings");
        stick.handle_axis(1, -3_000);
        assert_eq!((axis_neg, axis_pos), (1, 0), "threshold of 2000 applies");
        stick.handle_axis(1, -1_500);
        assert_eq!(axis_neg, 0);

        stick.handle_hat(0, HatDirection::LEFT);
        assert_eq!(hat_left, 1);
    }
}

#[test]
fn joystick_ignores_events_for_missing_controls() {
    let mut word = 7i32;
    let mut stick = Joystick::new(0, "pad".to_string(), 1, 1, 1);
    stick.add_button_binding(0, addr(&mut word));
    unsafe {
        stick.handle_button(-1, true);
        stick.handle_button(1, true);
        stick.handle_axis(-1, 30_000);
        stick.handle_axis(1, 30_000);
        stick.handle_hat(-1, HatDirection::UP);
        stick.handle_hat(1, HatDirection::UP);
    }
    assert_eq!(word, 7);
}

#[test]
fn reset_all_releases_buttons_axes_and_hats() {
    let (mut button, mut neg, mut pos, mut up) = (1i32, 1i32, 1i32, 1i32);
    let mut stick = Joystick::new(0, "pad".to_string(), 1, 1, 1);
    stick.add_button_binding(0, addr(&mut button));
    stick.add_axis_binding(0, -1, addr(&mut neg));
    stick.add_axis_binding(0, 1, addr(&mut pos));
    stick.add_hat_binding(0, HatDirection::UP, addr(&mut up));
    stick.axes[0].polarity = -1;
    stick.hats[0].last = HatDirection::UP;
    unsafe { stick.reset_all() };
    assert_eq!((button, neg, pos, up), (0, 0, 0, 0));
    assert_eq!(stick.axes[0].polarity, 0);
    assert_eq!(stick.hats[0].last, HatDirection::CENTERED);
}

#[test]
fn begin_frame_keeps_only_the_low_byte_of_every_bound_word() {
    let mut words = [0x1234i32; 7];
    let [button, neg, pos, up, down, left, right] = &mut words;
    let mut stick = Joystick::new(0, "pad".to_string(), 1, 1, 1);
    stick.add_button_binding(0, addr(button));
    stick.add_axis_binding(0, -1, addr(neg));
    stick.add_axis_binding(0, 1, addr(pos));
    stick.add_hat_binding(0, HatDirection::UP, addr(up));
    stick.add_hat_binding(0, HatDirection::DOWN, addr(down));
    stick.add_hat_binding(0, HatDirection::LEFT, addr(left));
    stick.add_hat_binding(0, HatDirection::RIGHT, addr(right));
    unsafe { stick.begin_frame() };
    assert_eq!(words, [0x34; 7]);
}

#[test]
fn begin_frame_leaves_unbound_words_alone() {
    let mut bound = 0x1FFi32;
    let unbound = 0x1FFi32;
    let mut stick = Joystick::new(0, "pad".to_string(), 0, 1, 0);
    stick.add_button_binding(0, addr(&mut bound));
    unsafe { stick.begin_frame() };
    assert_eq!(bound, 0xFF);
    assert_eq!(unbound, 0x1FF);
}

#[test]
fn clear_bindings_drops_every_binding_but_keeps_the_controls() {
    let mut stick = Joystick::new(0, "pad".to_string(), 2, 2, 2);
    stick.add_button_binding(0, 0x10);
    stick.add_axis_binding(1, -1, 0x20);
    stick.add_axis_binding(1, 1, 0x30);
    for direction in [
        HatDirection::UP,
        HatDirection::DOWN,
        HatDirection::LEFT,
        HatDirection::RIGHT,
    ] {
        stick.add_hat_binding(1, direction, 0x40);
    }
    stick.clear_bindings();
    assert_eq!(
        (stick.axes.len(), stick.buttons.len(), stick.hats.len()),
        (2, 2, 2)
    );
    assert!(stick.buttons.iter().all(|b| b.is_empty()));
    assert!(stick
        .axes
        .iter()
        .all(|a| a.neg.is_empty() && a.pos.is_empty()));
    assert!(stick.hats.iter().all(|h| {
        h.up.is_empty() && h.down.is_empty() && h.left.is_empty() && h.right.is_empty()
    }));
}
