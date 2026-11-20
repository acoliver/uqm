use super::*;
use crate::io::fixture::TempMount;
use std::path::Path;
use std::sync::Arc;

/// Run `test` in a child process: each owns the process platform, the
/// content mount, the screen and the one movie slot.
fn in_child(test: &str, body: impl FnOnce()) {
    const CHILD: &str = "UQM_ENGINE_VIDEO_PLAYBACK_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    crate::host::install_test(true);
    crate::graphics::state::init_headless(crate::graphics::state::VideoMode::default());
    body();
    stop_player();
    crate::sound::audio_device::shutdown();
    crate::graphics::state::uninit_headless();
}

fn addon(path: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .join("../sc2/content/addons/3dovideo")
            .join(path),
    )
    .unwrap()
}

fn logo_content() -> TempMount {
    let mount = TempMount::new().as_content();
    for extension in ["duk", "frm", "hdr", "tbl"] {
        let name = format!("logo.{extension}");
        mount.write(&name, &addon(&format!("logo/{name}")));
    }
    mount.write(
        "video.rmp",
        b"fixture.video = 3DOVID:logo.duk
fixture.absent = 3DOVID:missing.duk
fixture.unsuffixed = 3DOVID:logo
fixture.other = 3DOVID:logo.mpg
fixture.broken = 3DOVID:broken.duk
",
    );
    mount.write("broken.duk", b"bad");
    crate::resource::load_index(&mount.dir(), c"video.rmp", None);
    mount
}

fn started() -> bool {
    current().started
}

#[test]
fn a_movie_plays_reports_and_stops_on_drop() {
    in_child(
        "video::legacy::tests::a_movie_plays_reports_and_stops_on_drop",
        || {
            let _content = logo_content();
            assert!(LoadedVideo::load(c"fixture.unknown").is_none());
            let descriptor = LoadedVideo::load(c"fixture.video").expect("logo descriptor");
            let playback = descriptor.play().expect("logo playback");
            drop(descriptor);
            assert!(started());
            assert!(playback.is_playing());
            assert_eq!(playback.position(), 0);
            playback.seek(100);
            assert!(playback.process_frame());
            drop(playback);
            assert!(!started());
            assert!(current().player.is_none());
        },
    );
}

#[test]
fn missing_unsupported_and_corrupt_movies_do_not_start() {
    in_child(
        "video::legacy::tests::missing_unsupported_and_corrupt_movies_do_not_start",
        || {
            let _content = logo_content();
            for resource in [
                c"fixture.absent",
                c"fixture.unsuffixed",
                c"fixture.other",
                c"fixture.broken",
            ] {
                let descriptor = LoadedVideo::load(resource).expect("descriptor");
                assert!(descriptor.play().is_none(), "{resource:?}");
                assert!(!started());
            }
        },
    );
}

#[test]
fn starting_a_movie_stops_the_one_playing_and_stopping_the_player_ends_it() {
    in_child(
        "video::legacy::tests::starting_a_movie_stops_the_one_playing_and_stopping_the_player_ends_it",
        || {
            let _content = logo_content();
            let descriptor = LoadedVideo::load(c"fixture.video").expect("logo descriptor");
            let first = descriptor.play().expect("first playback");
            let second = descriptor.play().expect("second playback");
            assert!(second.is_playing());
            drop(first);
            assert!(!second.is_playing());
            assert!(!second.process_frame());
            drop(second);
            let third = descriptor.play().expect("third playback");
            stop_player();
            assert!(!third.is_playing());
            assert_eq!(third.position(), 0);
        },
    );
}

#[test]
fn shipped_spin_retains_video_and_both_audio_tracks_after_descriptor_drop() {
    in_child(
        "video::legacy::tests::shipped_spin_retains_video_and_both_audio_tracks_after_descriptor_drop",
        || {
            let directory = TempMount::new().as_content();
            let ships = "addons/3dovideo/ships";
            std::fs::create_dir_all(directory.path_of(ships)).unwrap();
            for name in [
                "ship00.duk",
                "ship00.frm",
                "ship00.hdr",
                "ship00.tbl",
                "ship00.aif",
                "spin.aif",
            ] {
                directory.write(&format!("{ships}/{name}"), &addon(&format!("ships/{name}")));
            }
            directory.write("3dovideo.rmp", &addon("3dovideo.rmp"));
            assert!(crate::sound::audio_device::start());
            crate::resource::load_index(&directory.dir(), c"3dovideo.rmp", None);
            let descriptor =
                LoadedVideo::load(c"slides.spins.00").expect("shipped spin descriptor");
            let playback = descriptor.play().expect("shipped spin playback");
            let music = Arc::downgrade(&playback.audio.as_ref().expect("spin.aif").playback().0);
            let speech =
                Arc::downgrade(&playback.speech.as_ref().expect("ship00.aif").playback().0);
            drop(descriptor);
            assert!(playback.is_playing());
            assert!(music.upgrade().is_some());
            assert!(speech.upgrade().is_some());
            assert!(playback.process_frame());
            drop(playback);
            assert!(music.upgrade().is_none());
            assert!(speech.upgrade().is_none());
            assert!(!started());
        },
    );
}
