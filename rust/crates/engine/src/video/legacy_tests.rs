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
    crate::time::source::install(true);
    let directory = crate::sound::decoder_vtable_tests::Directory::new();
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
    let screen = unsafe {
        let screen =
            sdl2_sys::SDL_CreateRGBSurface(0, 320, 240, 32, 0xff, 0xff00, 0xff0000, 0xff000000);
        assert!(!screen.is_null());
        crate::graphics::sdl::lifecycle_abi::SDL_Screen = screen.cast();
        crate::graphics::sdl::lifecycle_abi::GRAPHICS_BACKEND =
            ptr::addr_of_mut!(crate::graphics::sdl::lifecycle_abi::RUST_BACKEND);
        crate::io::handles::contentDir = directory.handle();
        assert_eq!(crate::sound::rodio_audio::rust_audio_init(), 1);
        assert_eq!(super::super::control::InitVideoPlayer(0), 1);
        InstallVideoResType();
        crate::resource::ffi_bridge::LoadResourceIndex(
            directory.handle().cast(),
            c"3dovideo.rmp".as_ptr(),
            ptr::null(),
        );
        screen
    };
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
        crate::io::handles::contentDir = ptr::null_mut();
        crate::graphics::sdl::lifecycle_abi::GRAPHICS_BACKEND = ptr::null_mut();
        crate::graphics::sdl::lifecycle_abi::SDL_Screen = ptr::null_mut();
        sdl2_sys::SDL_FreeSurface(screen);
    }
}
