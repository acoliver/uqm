use super::*;
use uqm_engine::configuration::runtime::{AudioSettings, DisplaySettings};

#[test]
fn reads_every_supplied_option_without_ambient_runtime_state() {
    let input = InputSession::default();
    input.configuration.update_preferences(|p| {
        p.menu = 1;
        p.fonts = 1;
        p.coarse_scan = 1;
        p.smooth_scroll = 1;
        p.intro = 1;
        p.shield = 1;
        p.melee_scale = 3;
        p.stereo_sfx = 1;
        p.music_3do = 1;
        p.music_remix = 1;
        p.speech = 1;
        p.keep_aspect_ratio = 1;
        p.subtitles = 1;
        p.gamma = 1.0;
    });
    input.set_player_control(0, 4);
    input.set_player_control(1, 5);
    let mut range = GammaRange::new(0.4, 2.5);
    let options = get_global_options(
        &input,
        DisplaySettings {
            driver: 0,
            flags: 263,
            width: 1024,
            height: 768,
        },
        AudioSettings {
            driver: 2,
            flags: 1,
            music: 0.76,
            sfx: 0.52,
            speech: 0.99,
        },
        None,
        &mut range,
    );
    assert_eq!(
        options.choices(),
        [3, 0, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 1, 4, 5, 0, 1, 1, 1]
    );
    assert_eq!(&options.sliders()[..3], &[75, 50, 100]);
    assert_eq!(range.slider_to_gamma(options.gamma), 1.0);
}

#[test]
fn resolutions_scaler_precedence_audio_defaults_and_explicit_driver_round_trip() {
    let input = InputSession::default();
    let mut range = GammaRange::new(0.4, 2.5);
    for (width, height, driver, res, choice) in [
        (320, 240, 1, 0, 0),
        (640, 480, 1, 1, 0),
        (800, 600, 0, 2, 0),
        (1024, 768, 0, 3, 0),
        (1280, 960, 0, 4, 0),
        (960, 720, 0, 5, 0),
        (1234, 987, 0, 6, 0),
        (320, 200, 0, 6, 0),
        (640, 480, 0, 1, 1),
        (320, 200, 1, 0, 0),
    ] {
        let display = DisplaySettings {
            driver,
            flags: 0,
            width,
            height,
        };
        let opts = get_global_options(&input, display, AudioSettings::default(), None, &mut range);
        assert_eq!((opts.res, opts.driver, opts.scaler), (res, choice, 2));
        for always in [false, true] {
            let opts = get_global_options(
                &input,
                display,
                AudioSettings::default(),
                Some(always),
                &mut range,
            );
            assert_eq!(opts.driver, u32::from(always));
        }
    }
    for (flags, scaler) in [(128, 1), (512, 3), (256 | 512 | 128, 2)] {
        let opts = get_global_options(
            &input,
            DisplaySettings {
                flags,
                ..Default::default()
            },
            AudioSettings::default(),
            None,
            &mut range,
        );
        assert_eq!(opts.scaler, scaler);
    }
    for (driver, flags, expected_driver, quality) in
        [(1, 4, 0, 0), (0, 2, 1, 1), (2, 1, 2, 2), (88, 5, 0, 2)]
    {
        let opts = get_global_options(
            &input,
            DisplaySettings::default(),
            AudioSettings {
                driver,
                flags,
                ..Default::default()
            },
            None,
            &mut range,
        );
        assert_eq!((opts.adriver, opts.aquality), (expected_driver, quality));
    }
}

#[test]
#[serial_test::serial]
fn writes_every_preference_and_persists_restart_only_audio_options() {
    let settings = Settings;
    let input = InputSession::default();
    let range = GammaRange::new(0.4, 2.5);
    for value in [0, 1] {
        let mut choices = [value; 24];
        choices[18] = 4;
        choices[19] = 5;
        let options =
            GlobalOptions::from_widgets(&choices, [20, 40, 60, range.gamma_to_slider(1.0)]);
        set_global_options(&input, &options, &range, &settings);
        let p = input.preferences();
        let console = if value == 1 { 1 } else { 2 };
        assert_eq!(
            [
                p.menu,
                p.fonts,
                p.coarse_scan,
                p.smooth_scroll,
                p.intro,
                p.shield
            ],
            [console; 6]
        );
        assert_eq!(
            [
                p.subtitles,
                p.stereo_sfx,
                p.music_3do,
                p.music_remix,
                p.speech,
                p.keep_aspect_ratio
            ],
            [value as u32; 6]
        );
        assert_eq!(p.melee_scale, if value == 1 { 3 } else { 0 });
        assert_eq!(input.player_controls(), [4, 5]);
        for key in [
            "subtitles",
            "iconicscan",
            "smoothscroll",
            "3domusic",
            "remixmusic",
            "speech",
            "3domovies",
            "showfps",
            "smoothmelee",
            "positionalsfx",
            "pulseshield",
            "keepaspectratio",
        ] {
            assert_eq!(settings.boolean(&format!("config.{key}")), Some(value == 1));
        }
        for key in ["textmenu", "textgradients"] {
            assert_eq!(settings.boolean(&format!("config.{key}")), Some(value == 0));
        }
        assert_eq!(settings.integer("config.gamma"), Some(1000));
        assert_eq!(settings.integer("config.player1control"), Some(4));
        assert_eq!(settings.integer("config.player2control"), Some(5));
        assert_eq!(
            settings.string("config.audiodriver").as_deref(),
            Some(if value == 0 { "none" } else { "mixsdl" })
        );
        assert_eq!(
            settings.string("config.audioquality").as_deref(),
            Some(if value == 0 { "low" } else { "medium" })
        );
        for (key, volume) in [("musicvol", 20), ("sfxvol", 40), ("speechvol", 60)] {
            assert_eq!(settings.integer(&format!("config.{key}")), Some(volume));
        }
    }
}

#[test]
#[serial_test::serial]
fn display_mapping_preserves_custom_dimensions_and_unrelated_flags() {
    let settings = Settings;
    let original = DisplaySettings {
        driver: 0,
        flags: 2 | 1024 | 1016,
        width: 1234,
        height: 987,
    };
    for (res, width, height, driver) in [
        (0, 320, 240, 1),
        (1, 640, 480, 1),
        (2, 800, 600, 0),
        (3, 1024, 768, 0),
        (4, 1280, 960, 0),
        (5, 960, 720, 0),
        (6, 1234, 987, 0),
    ] {
        for (scaler, flag, name) in [
            (0, 0, "no"),
            (1, 128, "hq"),
            (2, 256, "xbrz3"),
            (3, 512, "xbrz4"),
        ] {
            let options = GlobalOptions {
                res,
                scaler,
                driver: 1,
                fullscreen: 1,
                scanlines: 1,
                ..Default::default()
            };
            let result = super::super::options::prepare_display(&options, original, &settings);
            assert_eq!(
                result,
                DisplaySettings {
                    driver,
                    flags: 2 | 1024 | 1 | 4 | flag,
                    width,
                    height
                }
            );
            assert_eq!(settings.integer("config.reswidth"), Some(width));
            assert_eq!(settings.integer("config.resheight"), Some(height));
            assert_eq!(settings.boolean("config.alwaysgl"), Some(true));
            assert_eq!(settings.boolean("config.usegl"), Some(driver == 0));
            assert_eq!(settings.string("config.scaler").as_deref(), Some(name));
        }
    }
}

#[test]
fn gamma_rejection_adopts_the_reached_bound_and_keeps_unity_exact() {
    for direction in [-1, 1] {
        let mut range = GammaRange::new(0.4, 2.5);
        let mut value = range.gamma_to_slider(1.0);
        let before = range.slider_to_gamma(value);
        let mut attempted = 0.0;
        range.step(&mut value, direction, |gamma| {
            attempted = gamma;
            false
        });
        assert_eq!(value, if direction < 0 { 0 } else { 100 });
        assert!((range.slider_to_gamma(value) - before).abs() < 0.001);
        assert_eq!(attempted < before, direction < 0);
        assert_eq!(range.slider_to_gamma(range.gamma_to_slider(1.0)), 1.0);
    }
    let mut range = GammaRange::new(0.4, 2.5);
    let mut value = 50;
    range.step(&mut value, 1, |_| true);
    assert_eq!(value, 51);
    value = 0;
    range.step(&mut value, -1, |_| true);
    assert_eq!(value, 0);
    value = 100;
    range.step(&mut value, 1, |_| true);
    assert_eq!(value, 100);
}
