use uqm_engine::configuration::{Configuration, Preferences};
use uqm_engine::input_port::{controls::InputControls, session::InputSession};

#[test]
fn session_and_application_share_preferences_without_sharing_other_owners() {
    let application = Configuration::default();
    let session = InputSession::new(InputControls::default(), application.clone());
    let other = Configuration::default();
    application.set_preferences(Preferences {
        menu: 2,
        gamma: 1.25,
        ..Preferences::default()
    });
    assert_eq!(session.preferences().menu, 2);
    session
        .configuration
        .update_preferences(|options| options.subtitles = 1);
    assert_eq!(application.preferences().subtitles, 1);
    assert_eq!(other.preferences(), Preferences::default());
}

#[test]
fn input_session_clones_keep_the_supplied_control_and_configuration_owners() {
    let controls = InputControls::default();
    let configuration = uqm_engine::configuration::Configuration::default();
    let session = InputSession::new(controls.clone(), configuration.clone());
    let retained = session.clone();
    drop(session);
    retained.set_menu_key(9, true);
    retained
        .configuration
        .update_preferences(|preferences| preferences.fonts = 2);
    assert_eq!(controls.immediate().menu[9], 1);
    assert_eq!(configuration.preferences().fonts, 2);
}
