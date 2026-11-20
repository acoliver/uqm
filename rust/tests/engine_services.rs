//! Resource and movie lifecycle through the engine's normal library build.
use std::ptr;
use uqm_rust::graphics::present::GraphicsOptions;
use uqm_rust::strings::owned_table::LoadedTextTable;
use uqm_rust::video::{control::*, ffi::*, legacy::*};

fn retained_playback_keeps_its_clip_after_descriptor_drop() {
    assert!(LoadedVideo::load(c"fixture.unknown").is_none());
    let absent =
        LoadedVideo::load(c"fixture.absent").unwrap_or_else(|| panic!("missing-file descriptor"));
    assert!(absent.play().is_none());
    drop(absent);
    let descriptor =
        LoadedVideo::load(c"fixture.video").unwrap_or_else(|| panic!("logo descriptor"));
    let playback = descriptor.play().unwrap_or_else(|| panic!("logo playback"));
    drop(descriptor);
    assert!(playback.is_playing());
    assert!(playback.process_frame());
    drop(playback);
}

fn retained_string_table_loads_owned_text() {
    assert!(LoadedTextTable::load_file(c"missing").is_none());
    let table = LoadedTextTable::load_file(c"strings.txt")
        .unwrap_or_else(|| panic!("owned strings fixture did not load"));
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
",
    )
    .unwrap_or_else(|error| panic!("video resource index: {error}"));
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
    uqm_rust::engine::host::install(true);
    // SAFETY: this isolated test owns the repository, graphics state and every loaded resource.
    unsafe {
        let mount = uqm_engine::test_support::native_session::ContentMount::new(directory.path());
        let dir = uqm_rust::io::content_dir().unwrap();
        retained_string_table_loads_owned_text();
        let options = GraphicsOptions::default();
        uqm_rust::graphics::present::init_graphics(1, 0, options, 320, 240);
        assert_eq!(InitVideoPlayer(0), 1);
        assert_eq!(VidPlay(ptr::null_mut()), 0);
        assert_eq!(VidProcessFrame(), 0);
        assert_eq!(VidSeek(10), 0);
        assert_eq!(VidGetPosition(), 0);
        assert!(_init_video_file(c"missing.duk".as_ptr()).is_null());
        let clip = _init_video_file(c"logo.duk".as_ptr());
        assert!(!clip.is_null());
        let playback_started = std::time::Instant::now();
        assert_eq!(VidPlay(clip), 2);
        assert_eq!(VidPlaying(), clip);
        assert_eq!(VidProcessFrame(), 1);
        let position = VidGetPosition();
        assert_eq!(position, tfb_get_video_position(clip));
        assert!(u128::from(position) <= playback_started.elapsed().as_millis());
        assert_eq!(VidSeek(20), 0);
        VidStop();
        assert!(VidPlaying().is_null());
        assert_eq!(DestroyVideo(clip), 1);
        assert_eq!(DestroyVideo(ptr::null_mut()), 0);
        uqm_rust::resource::init();
        uqm_rust::resource::load_index(&dir, c"video.rmp", None);
        retained_playback_keeps_its_clip_after_descriptor_drop();
        assert!(VidPlaying().is_null());
        assert!(!rust_present_video_to_window(ptr::null(), 0, 0, 0));
        assert!(!rust_play_video(&dir, "absent.duk", 0, 0, false));
        std::fs::write(directory.path().join("broken.duk"), b"bad").unwrap();
        assert!(!rust_play_video(&dir, "broken.duk", 0, 0, false));
        let pixels = [
            255u8, 0, 0, 255, 99, 99, 99, 99, 0, 255, 0, 255, 88, 88, 88, 88,
        ];
        assert!(rust_present_video_to_window(pixels.as_ptr(), 1, 2, 8));
        UninitVideoPlayer();
        assert!(!rust_process_video_frame());
        uqm_rust::sound::rodio_audio::rust_audio_uninit();
        uqm_rust::graphics::present::uninit_graphics();
        drop(mount);
    }
    replay_audio_exports_follow_sample_completion_and_transport();
}

fn replay_audio_exports_follow_sample_completion_and_transport() {
    use uqm_rust::sound::rodio_audio::*;
    use uqm_rust::{engine::host as source, time::counter_abi::ONE_SECOND};

    let pcm = [0u8; 2000];
    // SAFETY: replay audio was selected before initialization; every byte slice stays live during calls.
    unsafe {
        assert_eq!(rust_audio_init(), 1);
        assert_eq!(rust_audio_play_raw(ptr::null(), 4, 1000, 1, 16, 0, 0), 0);
        assert_eq!(rust_audio_play_raw(pcm.as_ptr(), 0, 1000, 1, 16, 0, 0), 0);
        assert_eq!(
            rust_audio_play_raw(pcm.as_ptr(), pcm.len(), 0, 1, 16, 0, 0),
            0
        );
        assert_eq!(rust_audio_play_wav(ptr::null(), 1, 0, 0), 0);
        assert_eq!(rust_audio_play_ogg(pcm.as_ptr(), 0, 1, 0), 0);
        assert_eq!(rust_audio_play_ogg(pcm.as_ptr(), pcm.len(), 1, 0), 0);
        let handle = rust_audio_play_raw(pcm.as_ptr(), pcm.len(), 1000, 1, 16, 2, 0);
        assert_ne!(handle, 0);
        assert_eq!(rust_audio_is_playing(handle), 1);
        rust_audio_pause(handle);
        source::advance_to(source::now_ticks() + 2 * ONE_SECOND);
        assert_eq!(rust_audio_is_playing(handle), 1);
        rust_audio_resume(handle);
        source::advance_to(source::now_ticks() + ONE_SECOND);
        assert_eq!(rust_audio_is_playing(handle), 0);
        rust_audio_cleanup();
        let looping = rust_audio_play_raw(pcm.as_ptr(), pcm.len(), 1000, 1, 16, 7, 1);
        assert_ne!(looping, 0);
        source::advance_to(source::now_ticks() + 2 * ONE_SECOND);
        assert_eq!(rust_audio_is_playing(looping), 1);
        rust_audio_stop(looping);
        assert_eq!(rust_audio_is_playing(looping), 0);
        let last = play_raw_pcm(pcm.to_vec(), 1000, 1, 16, SoundCategory::Music, true);
        assert_ne!(last, 0);
        rust_audio_stop_all();
        assert_eq!(rust_audio_is_playing(last), 0);
        rust_audio_uninit();
        assert_eq!(rust_audio_is_playing(last), 0);
        assert_eq!(rust_audio_play_wav(pcm.as_ptr(), pcm.len(), 0, 0), 0);
    }
}
