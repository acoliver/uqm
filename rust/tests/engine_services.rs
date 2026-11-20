//! Resource and movie lifecycle through the engine's normal library build.
use std::ffi::CString;
use std::ptr;
use uqm_rust::graphics::sdl::lifecycle_abi::{
    GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics,
};
use uqm_rust::resource::ffi_bridge::_cur_resfile_name;
use uqm_rust::strings::{owned_table::LoadedTextTable, sfileins::load_string_table_file};
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
        assert!((*ptr::addr_of!(_cur_resfile_name)).is_null());
        _cur_resfile_name = c"busy".as_ptr();
        assert!(load_string_table_file(&dir, c"strings.txt".as_ptr()).is_null());
        _cur_resfile_name = ptr::null();
        let options = GraphicsOptions::default();
        assert_eq!(
            TFB_InitGraphics(1, 0, ptr::from_ref(&options).cast(), 320, 240),
            0
        );
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
        uqm_rust::resource::ffi_bridge::InitResourceSystem();
        uqm_rust::resource::ffi_bridge::load_resource_index(
            &dir,
            c"video.rmp".as_ptr(),
            ptr::null(),
        );
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
        TFB_UninitGraphics();
        drop(mount);
    }
    replay_audio_exports_follow_sample_completion_and_transport();
}

#[test]
fn string_bank_deduplicates_across_chunks_and_preserves_split_tails() {
    use std::ffi::CStr;
    use uqm_rust::resource::stringbank::*;

    // SAFETY: this test owns the bank and all returned pointers remain live until it is freed.
    unsafe {
        let bank = StringBank_Create();
        assert!(!bank.is_null());
        let alpha = StringBank_AddString(bank, c"alpha".as_ptr());
        assert_ne!(alpha, StringBank_AddString(bank, c"alpha".as_ptr()));
        assert_eq!(alpha, StringBank_AddOrFindString(bank, c"alpha".as_ptr()));
        assert_eq!(
            alpha,
            StringBank_AddOrFindSubstring(bank, c"alphabet".as_ptr(), 5)
        );
        let beta = StringBank_AddSubstring(bank, c"betamax".as_ptr(), 4);
        assert_eq!(CStr::from_ptr(beta), c"beta");
        for i in 0..300 {
            let value = CString::new(format!("bank-entry-{i:04}")).unwrap();
            let saved = StringBank_AddString(bank, value.as_ptr());
            assert_eq!(CStr::from_ptr(saved), value.as_c_str());
        }
        assert_eq!(CStr::from_ptr(alpha), c"alpha");
        assert_eq!(alpha, StringBank_AddOrFindString(bank, c"alpha".as_ptr()));
        let tail = StringBank_AddOrFindString(bank, c"bank-entry-0299".as_ptr());
        assert_eq!(CStr::from_ptr(tail), c"bank-entry-0299");
        let too_long = CString::new(vec![b'x'; 2048]).unwrap();
        assert!(StringBank_AddString(bank, too_long.as_ptr()).is_null());
        let mut pieces = [ptr::null(); 4];
        assert_eq!(
            SplitString(
                c"alpha,,alpha,tail,rest".as_ptr(),
                b',' as _,
                4,
                pieces.as_mut_ptr(),
                bank
            ),
            4
        );
        assert_eq!(pieces[0], alpha);
        assert_eq!(pieces[2], alpha);
        assert_eq!(CStr::from_ptr(pieces[1]), c"");
        assert_eq!(CStr::from_ptr(pieces[3]), c"tail,rest");
        assert_eq!(
            SplitString(
                c"whole,tail".as_ptr(),
                b',' as _,
                1,
                pieces.as_mut_ptr(),
                bank
            ),
            1
        );
        assert_eq!(CStr::from_ptr(pieces[0]), c"whole,tail");
        StringBank_Free(bank);
        StringBank_Free(ptr::null_mut());
    }
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
