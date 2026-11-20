use super::*;
use std::sync::Arc;

#[test]
fn shipped_spin_retains_video_and_both_audio_tracks_after_descriptor_drop() {
    const CHILD: &str = "UQM_ENGINE_RETAINED_SHIP_SPIN_CHILD";
    const TEST: &str = "video::legacy::tests::shipped_spin_retains_video_and_both_audio_tracks_after_descriptor_drop";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    crate::host::install(true);
    let directory = crate::io::fixture::TempMount::new().as_content();
    let source = std::path::Path::new(env!("UQM_RUST_ROOT")).join("../sc2/content/addons/3dovideo");
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
        directory.write(
            &format!("{ships}/{name}"),
            &std::fs::read(source.join("ships").join(name)).unwrap(),
        );
    }
    directory.write(
        "3dovideo.rmp",
        &std::fs::read(source.join("3dovideo.rmp")).unwrap(),
    );
    // SAFETY: the isolated child owns this real content mount, video player and screen until teardown.
    unsafe {
        crate::graphics::state::init_headless(crate::graphics::state::VideoMode::default());
        assert_eq!(crate::sound::rodio_audio::rust_audio_init(), 1);
        assert_eq!(super::super::control::InitVideoPlayer(0), 1);
        crate::resource::load_index(&directory.dir(), c"3dovideo.rmp", None);
    }
    let descriptor = LoadedVideo::load(c"slides.spins.00").expect("shipped spin descriptor");
    let playback = descriptor.play().expect("shipped spin playback");
    let music = Arc::downgrade(&playback.audio.as_ref().expect("spin.aif").playback().0);
    let speech = Arc::downgrade(&playback.speech.as_ref().expect("ship00.aif").playback().0);
    drop(descriptor);
    assert!(playback.is_playing());
    assert!(music.upgrade().is_some());
    assert!(speech.upgrade().is_some());
    playback.seek(100);
    assert!(playback.process_frame());
    drop(playback);
    assert!(music.upgrade().is_none());
    assert!(speech.upgrade().is_none());
    // SAFETY: playback has released its clip and audio before the child tears down their providers.
    unsafe {
        assert!(super::super::control::VidPlaying().is_null());
        super::super::control::UninitVideoPlayer();
        crate::sound::rodio_audio::rust_audio_uninit();
        crate::graphics::state::uninit_headless();
    }
}
