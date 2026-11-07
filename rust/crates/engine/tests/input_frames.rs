use uqm_engine::input_port::frames::{InputFrames, InputState};

fn held_menu(index: usize) -> InputState {
    let mut state = InputState::default();
    state.menu[index] = 1;
    state
}

#[test]
fn repeat_state_belongs_to_each_input_owner() {
    let mut first = InputFrames::default();
    let mut second = InputFrames::default();
    first.configure_repeat(70, 420, 70, false, 0);
    second.configure_repeat(1, 2, 1, false, 0);
    let down = held_menu(9);
    first.advance(down, down, 10);
    second.advance(down, down, 10);
    assert_eq!(first.pulsed.menu[9], 1);
    assert_eq!(second.pulsed.menu[9], 1);
    first.advance(down, down, 12);
    second.advance(down, down, 12);
    assert_eq!(first.current.menu[9], 1);
    assert_eq!(first.pulsed.menu[9], 0);
    assert_eq!(second.pulsed.menu[9], 1);
    assert_eq!(first.repeat_config(), (70, 420, 70, false));
}

#[test]
fn gestalt_uses_post_frame_keys_and_the_pre_frame_snapshot() {
    let mut frames = InputFrames::default();
    frames.configure_repeat(70, 420, 70, true, 0);
    let before = held_menu(9);
    frames.advance(before, InputState::default(), 10);
    assert_eq!(frames.current.menu[9], 1);
    assert_eq!(frames.pulsed.menu[9], 0);
    frames.advance(before, before, 11);
    assert_eq!(frames.pulsed.menu[9], 1);
    frames.advance(before, before, 12);
    assert_eq!(frames.pulsed.menu[9], 0);
}

#[test]
fn clearing_pulses_keeps_current_state_and_repeat_configuration() {
    let mut frames = InputFrames::default();
    frames.configure_repeat(70, 420, 70, false, 0);
    let mut down = InputState::default();
    down.key[1][4] = 1;
    frames.advance(down, down, 10);
    frames.clear_menu_state();
    assert_eq!(frames.current.key[1][4], 1);
    assert_eq!(frames.pulsed.key[1][4], 0);
    assert_eq!(frames.repeat_config(), (70, 420, 70, false));
    frames.advance(down, down, 11);
    assert_eq!(frames.pulsed.key[1][4], 1);
}

#[test]
fn repeat_elapsed_time_wraps_and_acceleration_stops_at_the_minimum() {
    let mut frames = InputFrames::default();
    frames.configure_repeat(4, 12, 4, false, u32::MAX - 7);
    let down = held_menu(9);
    frames.advance(down, down, u32::MAX - 7);
    for (now, pulse) in [(3, 0), (4, 1), (11, 0), (12, 1), (15, 0), (16, 1), (20, 1)] {
        frames.advance(down, down, now);
        assert_eq!(frames.pulsed.menu[9], pulse, "tick {now}");
    }
}

#[test]
fn independent_keys_do_not_restart_each_others_repeat_clock() {
    let mut frames = InputFrames::default();
    frames.configure_repeat(4, 12, 4, false, 0);
    let first = held_menu(9);
    frames.advance(first, first, 0);
    let mut both = first;
    both.menu[10] = 1;
    frames.advance(both, both, 8);
    assert_eq!(frames.pulsed.menu[9], 0);
    assert_eq!(frames.pulsed.menu[10], 1);
    frames.advance(both, both, 12);
    assert_eq!(frames.pulsed.menu[9], 1);
    assert_eq!(frames.pulsed.menu[10], 0);
}
