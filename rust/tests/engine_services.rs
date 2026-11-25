//! Resource and movie lifecycle through the engine's normal library build.
#![forbid(unsafe_code)]
use uqm_engine::graphics::present::GraphicsOptions;
use uqm_engine::strings::owned_table::LoadedTextTable;
use uqm_engine::video::legacy::*;

fn retained_playback_keeps_its_clip_after_descriptor_drop(content: &uqm_engine::resource::Content) {
    assert!(LoadedVideo::load(content, c"fixture.unknown").is_err());
    let absent = LoadedVideo::load(content, c"fixture.absent")
        .unwrap_or_else(|error| panic!("missing-file descriptor: {error}"));
    assert!(absent.play().is_err());
    drop(absent);
    let descriptor = LoadedVideo::load(content, c"fixture.video")
        .unwrap_or_else(|error| panic!("logo descriptor: {error}"));
    let playback_started = std::time::Instant::now();
    let playback = descriptor
        .play()
        .unwrap_or_else(|error| panic!("logo playback: {error}"));
    drop(descriptor);
    assert!(playback.is_playing());
    assert!(playback.process_frame());
    assert!(u128::from(playback.position()) <= playback_started.elapsed().as_millis());
    drop(playback);
    let broken = LoadedVideo::load(content, c"fixture.broken")
        .unwrap_or_else(|error| panic!("broken: {error}"));
    assert!(broken.play().is_err());
    let replaced = descriptor_playback(content, c"fixture.video");
    stop_player();
    assert!(!replaced.is_playing());
}

fn descriptor_playback(
    content: &uqm_engine::resource::Content,
    resource: &std::ffi::CStr,
) -> VideoPlayback {
    LoadedVideo::load(content, resource)
        .and_then(|video| video.play())
        .unwrap_or_else(|error| panic!("{resource:?} playback: {error}"))
}

fn retained_string_table_loads_owned_text(files: &uqm_engine::io::Dir) {
    assert!(LoadedTextTable::load_file(files, c"missing").is_err());
    let table = LoadedTextTable::load_file(files, c"strings.txt")
        .unwrap_or_else(|error| panic!("owned strings fixture did not load: {error}"));
    assert_eq!(table.len(), 2);
    assert_eq!(
        table
            .text(0)
            .unwrap_or_else(|| panic!("owned strings fixture has no first entry")),
        c"Hello, captain."
    );
}

#[test]
fn file_resources_and_movie_playback_share_the_live_content_directory() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&root).unwrap_or_else(|error| panic!("fixture root: {error}"));
    let directory =
        tempfile::tempdir_in(root).unwrap_or_else(|error| panic!("fixture directory: {error}"));
    std::fs::write(
        directory.path().join("video.rmp"),
        b"fixture.video = 3DOVID:logo.duk
fixture.absent = 3DOVID:missing.duk
fixture.broken = 3DOVID:broken.duk
",
    )
    .unwrap_or_else(|error| panic!("video resource index: {error}"));
    std::fs::write(directory.path().join("broken.duk"), b"bad").unwrap();
    std::fs::write(
        directory.path().join("strings.txt"),
        b"#(HELLO)\nHello, captain.\n#(BYE)\nGoodbye.\n",
    )
    .unwrap();
    let logo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sc2/content/addons/3dovideo/logo");
    for ext in ["duk", "frm", "hdr", "tbl"] {
        std::fs::copy(
            logo.join(format!("logo.{ext}")),
            directory.path().join(format!("logo.{ext}")),
        )
        .unwrap();
    }
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    uqm_platform::install(true);
    // SAFETY: this isolated test owns the repository, graphics state and every loaded resource.
    {
        let mount = uqm_engine::test_support::native_session::ContentMount::new(directory.path());
        let dir = mount.content_directory();
        retained_string_table_loads_owned_text(dir);
        let options = GraphicsOptions::default();
        uqm_engine::graphics::present::init_graphics(1, 0, options, 320, 240);
        mount.load_index("video.rmp");
        retained_playback_keeps_its_clip_after_descriptor_drop(&mount.content());
        assert!(uqm_engine::graphics::present::present_video(
            &[0xff00_00ff, 0xff00_ff00],
            1,
            2
        ));
        uqm_engine::sound::audio_device::shutdown();
        uqm_engine::graphics::present::uninit_graphics();
        drop(mount);
    }
    replay_audio_exports_follow_sample_completion_and_transport();
}

fn replay_audio_exports_follow_sample_completion_and_transport() {
    use uqm_engine::host as source;
    use uqm_engine::sound::audio_device::{self as audio, PcmClip};
    use uqm_rust::time::counter_abi::ONE_SECOND;

    let clip = |looping| {
        PcmClip::from_bytes(&[0u8; 2000], 1000, 1, 16, looping)
            .unwrap_or_else(|| panic!("silent clip"))
    };
    assert!(audio::start());
    let handle = audio::play(clip(false));
    assert_ne!(handle, 0);
    assert!(audio::is_playing(handle));
    source::advance_to(source::now_ticks() + ONE_SECOND - 1);
    assert!(audio::is_playing(handle));
    source::advance_to(source::now_ticks() + 1);
    assert!(!audio::is_playing(handle));
    let looping = audio::play(clip(true));
    assert_ne!(looping, 0);
    source::advance_to(source::now_ticks() + 2 * ONE_SECOND);
    assert!(audio::is_playing(looping));
    audio::stop(looping);
    assert!(!audio::is_playing(looping));
    let last = audio::play(clip(true));
    assert_ne!(last, 0);
    audio::shutdown();
    assert!(!audio::is_playing(last));
}
