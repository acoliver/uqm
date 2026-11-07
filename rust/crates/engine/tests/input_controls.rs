use uqm_engine::input_port::controls::InputControls;

#[test]
fn provider_buffer_keeps_its_address_when_the_input_owner_moves() {
    let input = InputControls::default();
    let address = input.immediate_ptr();
    let input = Box::new(input);
    assert_eq!(input.immediate_ptr(), address);
    input.set_menu_key(9, true);
    assert_eq!(input.immediate().menu[9], 1);
}

#[test]
fn input_owners_do_not_share_keys_templates_or_pause_exit_state() {
    let first = InputControls::default();
    let second = InputControls::default();
    first.set_player_control(0, 3);
    first.set_player_key(0, 4, true);
    first.set_paused(true);
    first.set_exit_requested(true);
    assert_eq!(first.immediate().key[3][4], 1);
    assert_eq!(second.immediate().key[3][4], 0);
    assert_eq!(second.player_controls(), [0, 0]);
    assert!(first.paused());
    assert!(first.exit_requested());
    assert!(!second.paused());
    assert!(!second.exit_requested());
    assert_ne!(first.immediate_ptr(), second.immediate_ptr());
}

#[test]
fn sampled_frames_and_menu_sound_masks_remain_local() {
    let first = InputControls::default();
    let second = InputControls::default();
    first.set_menu_key(9, true);
    first.set_menu_sounds(0x12, 0x34);
    let keys = first.immediate();
    first.advance(keys, keys, 1);
    assert_eq!(first.current().menu[9], 1);
    assert_eq!(first.pulsed().menu[9], 1);
    assert_eq!(second.current().menu[9], 0);
    assert_eq!(second.pulsed().menu[9], 0);
    assert_eq!(first.menu_sounds(), (0x12, 0x34));
    assert_eq!(second.menu_sounds(), (0, 0));
}

#[test]
fn retained_input_handles_share_the_provider_buffer_and_live_snapshots() {
    let owner = InputControls::default();
    let observer = owner.clone();
    assert_eq!(owner.immediate_ptr(), observer.immediate_ptr());
    observer.set_menu_key(9, true);
    let keys = owner.immediate();
    owner.advance(keys, keys, 7);
    assert_eq!(observer.current().menu[9], 1);
    observer.clear_menu_pulse(9);
    assert_eq!(owner.pulsed().menu[9], 0);
    drop(owner);
    assert_eq!(observer.current().menu[9], 1);
}

#[test]
fn moving_a_handle_to_another_thread_does_not_admit_provider_access() {
    let owner = InputControls::default();
    let observer = owner.clone();
    assert!(std::thread::spawn(move || observer.current())
        .join()
        .is_err());
    assert_eq!(owner.current().menu[9], 0);
}

#[test]
fn retained_handles_share_callback_installation_and_restoration() {
    fn hook(_: &mut dyn FnMut(uqm_engine::observe_port::CheckpointView<'_>), _: &InputControls) {}
    let input = InputControls::default();
    let retained = input.clone();
    let other = InputControls::default();
    assert!(input.replace_callback(Some(hook)).is_none());
    assert!(retained.callback().is_some());
    assert!(other.callback().is_none());
    let saved = retained.replace_callback(None);
    assert!(saved.is_some());
    assert!(input.callback().is_none());
    assert!(input.replace_callback(saved).is_none());
    assert!(retained.callback().is_some());
}

#[test]
fn rejected_injection_is_sticky_only_for_its_retained_owner() {
    let first = InputControls::default();
    let retained = first.clone();
    let other = InputControls::default();
    retained.reject_injection();
    assert!(first.injection_rejected());
    assert!(!other.injection_rejected());
}
