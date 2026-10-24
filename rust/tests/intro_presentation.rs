//! Movie frames must survive the same input, pacing and expose boundaries as slides.
use std::{ffi::CString, path::Path, ptr, time::Duration};
use uqm_rust::graphics::{
    cmap_abi::FadeScreen,
    dcqueue_abi::TFB_FlushGraphics,
    geometry_ffi::TFB_BBox_RegisterRect,
    native_abi::{EXTENT, POINT, RECT},
    sdl::lifecycle_abi::{TFB_InitGraphics, TFB_SwapBuffers, TFB_UninitGraphics},
};
use uqm_rust::io::uio_bridge as uio;
use uqm_rust::mainloop::input_frame::rust_complete_input_frame;
use uqm_rust::video::{
    decoder::DukVideoDecoder,
    ffi::{
        tfb_play_video, tfb_process_video_frame, tfb_stop_video, TFB_VideoClip, TFB_VideoDecoder,
    },
    DUCK_FPS,
};

unsafe fn readback() -> Vec<u8> {
    let window = sdl2_sys::SDL_GetWindowFromID(1);
    assert!(!window.is_null());
    let renderer = sdl2_sys::SDL_GetRenderer(window);
    assert!(!renderer.is_null());
    let mut rgba = vec![0; 320 * 240 * 4];
    assert_eq!(
        sdl2_sys::SDL_RenderReadPixels(
            renderer,
            ptr::null(),
            sdl2_sys::SDL_PixelFormatEnum::SDL_PIXELFORMAT_ABGR8888 as u32,
            rgba.as_mut_ptr().cast(),
            320 * 4,
        ),
        0
    );
    rgba
}

#[test]
fn movie_frames_never_alternate_with_the_cleared_game_screen() {
    // Start on a visible part of the shipped movie rather than its intentional black lead-in.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let source = root.join("sc2/content/addons/3dovideo/logo");
    std::fs::create_dir_all(root.join("tmp")).unwrap();
    let fixture = tempfile::tempdir_in(root.join("tmp")).unwrap();
    for extension in ["hdr", "tbl", "duk"] {
        std::fs::copy(
            source.join(format!("logo.{extension}")),
            fixture.path().join(format!("logo.{extension}")),
        )
        .unwrap();
    }
    let offsets = std::fs::read(source.join("logo.frm")).unwrap();
    std::fs::write(fixture.path().join("logo.frm"), &offsets[100 * 4..]).unwrap();
    let mut decoder = DukVideoDecoder::open(fixture.path(), "logo").unwrap();
    let first = decoder.decode_frame(0).unwrap();
    assert!(first.data.iter().any(|p| p & 0x00ff_ffff != 0));
    let host = CString::new(fixture.path().to_str().unwrap()).unwrap();

    // This process owns its SDL instance; no native window or user config is used.
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    unsafe {
        assert_eq!(TFB_InitGraphics(0, 0, ptr::null(), 320, 240), 0);
        FadeScreen(253, 0);
        let screen = uqm_rust::graphics::ffi::rust_gfx_get_screen_surface(0);
        assert_eq!(sdl2_sys::SDL_FillRect(screen.cast(), ptr::null(), 0), 0);
        let bounds = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: 320,
                height: 240,
            },
        };
        TFB_BBox_RegisterRect(&bounds);

        uio::uio_init();
        let repo = uio::uio_openRepository(0);
        let mount = uio::uio_mountDir(
            repo,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            6,
            ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let dir = uio::uio_openDir(repo, c"/".as_ptr(), 0);
        assert!(!dir.is_null());
        let mut descriptor: TFB_VideoDecoder = std::mem::zeroed();
        descriptor.filename = c"logo.duk".as_ptr().cast_mut();
        descriptor.dir = dir.cast();
        let mut clip: TFB_VideoClip = std::mem::zeroed();
        clip.decoder = &mut descriptor;
        clip.loop_frame = u32::MAX;
        let x = (320 - first.width) / 2;
        let y = (240 - first.height) / 2;
        assert!(tfb_play_video(&mut clip, x, y));

        for _ in 0..3 {
            assert!(tfb_process_video_frame(&mut clip));
            let movie = readback();
            assert!(movie.as_chunks::<4>().0.iter().any(|p| p[..3] != [0, 0, 0]));
            for _ in 0..3 {
                rust_complete_input_frame(1);
                assert!(readback() == movie, "input boundary erased the movie");
                TFB_FlushGraphics();
                assert!(readback() == movie, "pacing flush erased the movie");
                TFB_SwapBuffers(2);
                assert!(readback() == movie, "expose erased the movie");
            }
            // Advance to the next scheduled movie frame, not a redraw of the current one.
            std::thread::sleep(Duration::from_secs_f64(1.0 / f64::from(DUCK_FPS)));
        }
        let movie = readback();
        FadeScreen(252, 0);
        TFB_SwapBuffers(1);
        assert!(readback()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [0, 0, 0]));
        FadeScreen(253, 0);
        TFB_SwapBuffers(1);
        assert!(
            readback() == movie,
            "fade restores the retained movie frame"
        );

        tfb_stop_video(&mut clip);
        assert_eq!(sdl2_sys::SDL_FillRect(screen.cast(), ptr::null(), 0), 0);
        TFB_SwapBuffers(1);
        assert!(readback()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [0, 0, 0]));
        uio::uio_closeDir(dir);
        uio::uio_closeRepository(repo);
        uio::uio_unInit();
        TFB_UninitGraphics();
    }
}
