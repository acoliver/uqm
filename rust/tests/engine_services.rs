//! Resource and movie lifecycle through the engine's normal library build.
use std::ffi::CString;
use std::ptr;
use uqm_rust::graphics::sdl::lifecycle_abi::{
    GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics,
};
use uqm_rust::io::uio_bridge as uio;
use uqm_rust::resource::ffi_bridge::_cur_resfile_name;
use uqm_rust::strings::{sfileins::LoadStringTableFile, string_table::FreeStringTable};
use uqm_rust::video::{control::*, ffi::*, legacy::*};

#[test]
fn file_resources_and_movie_playback_share_the_live_content_directory() {
    let directory = tempfile::tempdir().unwrap();
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
    let host = CString::new(directory.path().to_str().unwrap()).unwrap();
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    uqm_rust::time::source::install(true);
    // SAFETY: this isolated test owns the repository, graphics state and every loaded resource.
    unsafe {
        uio::uio_init();
        let repository = uio::uio_openRepository(0);
        let mount = uio::uio_mountDir(
            repository,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            4,
            ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let dir = uio::uio_openDir(repository, c"/".as_ptr(), 0);
        assert!(!dir.is_null());
        uqm_rust::io::handles::contentDir = dir;
        assert!(LoadStringTableFile(dir.cast(), c"missing".as_ptr()).is_null());
        let table = LoadStringTableFile(dir.cast(), c"strings.txt".as_ptr());
        assert!(!table.is_null());
        assert_eq!((*table).size, 2);
        let first = &*(*table).strings;
        assert_eq!(
            std::slice::from_raw_parts(first.data.cast::<u8>(), first.length as usize),
            b"Hello, captain.\0"
        );
        assert!((*ptr::addr_of!(_cur_resfile_name)).is_null());
        FreeStringTable(table);
        _cur_resfile_name = c"busy".as_ptr();
        assert!(LoadStringTableFile(dir.cast(), c"strings.txt".as_ptr()).is_null());
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
        let mut legacy = LegacyVideo {
            video: c"logo.duk".as_ptr().cast_mut(),
            audio: ptr::null_mut(),
            speech: ptr::null_mut(),
            loop_frame: u32::MAX,
        };
        let reference = PlayLegacyVideo(&mut legacy);
        assert!(!reference.is_null());
        assert_eq!(PlayingLegacyVideo(reference), 1);
        StopLegacyVideo(reference);
        assert_eq!(PlayingLegacyVideo(ptr::null_mut()), 0);
        assert!(PlayLegacyVideo(ptr::null_mut()).is_null());
        StopLegacyVideo(ptr::null_mut());
        assert!(!rust_present_video_to_window(ptr::null(), 0, 0, 0));
        assert!(!rust_play_video(dir.cast(), ptr::null(), 0, 0, false));
        assert!(!rust_play_video(
            dir.cast(),
            c"absent.duk".as_ptr(),
            0,
            0,
            false
        ));
        std::fs::write(directory.path().join("broken.duk"), b"bad").unwrap();
        assert!(!rust_play_video(
            dir.cast(),
            c"broken.duk".as_ptr(),
            0,
            0,
            false
        ));
        let pixels = [
            255u8, 0, 0, 255, 99, 99, 99, 99, 0, 255, 0, 255, 88, 88, 88, 88,
        ];
        assert!(rust_present_video_to_window(pixels.as_ptr(), 1, 2, 8));
        UninitVideoPlayer();
        assert!(!rust_process_video_frame());
        uqm_rust::sound::rodio_audio::rust_audio_uninit();
        TFB_UninitGraphics();
        uqm_rust::io::handles::contentDir = ptr::null_mut();
        uio::uio_closeDir(dir);
        uio::uio_closeRepository(repository);
        uio::uio_unInit();
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
    use uqm_rust::time::{counter_abi::ONE_SECOND, source};

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
