use super::{CreditsControl, CreditsKeys};

#[test]
fn independent_credits_controls_start_at_the_original_forward_rate() {
    let mut first = CreditsControl::new(false);
    let second = CreditsControl::new(false);
    assert_eq!((first.rate, second.rate), (9, 9));
    assert!(first.running && second.running);
    assert!(first.input(
        CreditsKeys {
            down: true,
            ..Default::default()
        },
        false,
        0
    ));
    assert_eq!(first.rate, 11);
    assert_eq!(second.rate, 9);
}

#[test]
fn speed_changes_keep_down_priority_and_both_original_bounds() {
    let mut control = CreditsControl::new(false);
    assert!(control.input(
        CreditsKeys {
            up: true,
            down: true,
            ..Default::default()
        },
        false,
        0
    ));
    assert_eq!(control.rate, 11);
    control.rate = 129;
    assert!(control.input(
        CreditsKeys {
            down: true,
            ..Default::default()
        },
        false,
        0
    ));
    assert_eq!(control.rate, 130);
    control.rate = -129;
    assert!(control.input(
        CreditsKeys {
            up: true,
            ..Default::default()
        },
        false,
        0
    ));
    assert_eq!(control.rate, -130);
}

#[test]
fn victory_cancel_waits_for_scrolling_to_finish_but_abort_does_not() {
    let mut control = CreditsControl::new(true);
    let cancel = CreditsKeys {
        cancel: true,
        ..Default::default()
    };
    assert!(control.input(cancel, false, 0));
    control.running = false;
    assert!(!control.input(cancel, false, 0));
    let mut control = CreditsControl::new(true);
    assert!(!control.input(CreditsKeys::default(), true, 0));
    assert_eq!(control.rate, 9);
}

#[test]
fn normal_credits_close_only_after_the_strict_five_second_deadline() {
    let mut control = CreditsControl::new(false);
    control.running = false;
    assert!(control.input(CreditsKeys::default(), false, 100));
    assert_eq!(control.close_time_out, 4300);
    assert!(control.input(CreditsKeys::default(), false, 4300));
    assert!(!control.input(CreditsKeys::default(), false, 4301));
    let mut control = CreditsControl::new(true);
    control.running = false;
    assert!(control.input(CreditsKeys::default(), false, 100));
    assert_eq!(control.close_time_out, 0);
}

#[test]
fn keys_read_the_supplied_session_pulse_without_global_input() {
    use uqm_engine::input::{
        frames::{InputState, KEY_MENU_SELECT, KEY_MENU_UP},
        InputSession,
    };
    let input = InputSession::default();
    let mut pressed = InputState::default();
    pressed.menu[KEY_MENU_UP as usize] = 1;
    pressed.menu[KEY_MENU_SELECT as usize] = 1;
    input.advance(pressed, pressed, 0);
    let keys = CreditsKeys::read(&input);
    assert_eq!(
        (keys.up, keys.select, keys.down, keys.cancel),
        (true, true, false, false)
    );
}
