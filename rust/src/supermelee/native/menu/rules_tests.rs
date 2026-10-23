use super::rules::{advance_cursor, move_cursor, next_control, next_option};

#[test]
fn slot_advance_wraps_first_row_and_clamps_at_last_slot() {
    assert_eq!(advance_cursor(0, 5), (0, 6));
    assert_eq!(advance_cursor(0, 6), (1, 0));
    assert_eq!(advance_cursor(1, 6), (1, 6));
}

#[test]
fn vertical_movement_crosses_teams_via_the_name_row() {
    assert_eq!(move_cursor((0, 2, 3), false, false, false, true), (1, 0, 3));
    assert_eq!(move_cursor((1, 0, 3), false, false, true, false), (0, 2, 3));
    assert_eq!(move_cursor((0, 0, 3), false, false, true, false), (0, 0, 3));
    assert_eq!(move_cursor((1, 2, 3), false, false, false, true), (1, 2, 3));
}

#[test]
fn cursor_clamps_and_left_up_take_precedence() {
    assert_eq!(move_cursor((0, 1, 3), true, true, true, true), (0, 0, 2));
    assert_eq!(move_cursor((0, 0, 0), true, false, false, false), (0, 0, 0));
    assert_eq!(move_cursor((0, 1, 6), false, true, false, false), (0, 1, 6));
}

#[test]
fn menu_navigation_clamps_and_prioritizes_up() {
    assert_eq!(next_option(0, true, false), 0);
    assert_eq!(next_option(9, false, true), 9);
    assert_eq!(next_option(4, true, true), 3);
    assert_eq!(next_option(4, false, true), 5);
}

#[test]
fn controls_cycle_through_all_computer_ratings() {
    use crate::battle::{COMPUTER_CONTROL, HUMAN_CONTROL, NETWORK_CONTROL};
    let human = HUMAN_CONTROL | 16;
    let computer = COMPUTER_CONTROL;
    assert_eq!(next_control(human), computer | 16);
    assert_eq!(next_control(computer | 16), computer | 32);
    assert_eq!(next_control(computer | 32), computer | 64);
    assert_eq!(next_control(computer | 64), human);
    assert_eq!(next_control(NETWORK_CONTROL | 16), human);
}
