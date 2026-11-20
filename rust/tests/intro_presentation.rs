//! Movie frames must survive the same input, pacing and expose boundaries as slides.
use std::{ffi::CString, path::Path, ptr, time::Duration};
use uqm_rust::engine::test_support::native_probe::register_rect;
use uqm_rust::graphics::present::{swap_buffers, Redraw};
use uqm_rust::graphics::state::{draw_on_screen, fade_screen};
use uqm_rust::graphics::{dcq::Screen, geometry::Extent, geometry::Point, geometry::Rect};
use uqm_rust::mainloop::input_frame::rust_complete_input_frame;
use uqm_rust::video::{
    decoder::DukVideoDecoder,
    ffi::{tfb_play_video, tfb_process_video_frame, tfb_stop_video, ClipSource, TFB_VideoClip},
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

    // This process owns its SDL instance; no native window or user config is used.
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    let options = uqm_rust::graphics::present::GraphicsOptions::default();
    unsafe {
        uqm_rust::graphics::present::init_graphics(0, 0, options, 320, 240);
        fade_screen(253, 0);
        draw_on_screen(Screen::Main, |canvas| canvas.words_mut().fill(0));
        let bounds = Rect {
            corner: Point { x: 0, y: 0 },
            extent: Extent {
                width: 320,
                height: 240,
            },
        };
        register_rect(bounds);

        let mount = uqm_engine::test_support::native_session::ContentMount::new(fixture.path());
        let descriptor = ClipSource {
            w: 0,
            h: 0,
            length: 0.0,
            audio_synced: false,
            dir: uqm_rust::io::content_dir().unwrap(),
            filename: CString::new("logo.duk").unwrap(),
        };
        let mut descriptor = descriptor;
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
                let mut observer = uqm_rust::engine::observe::Unobserved;
                let mut io = uqm_rust::engine::observe::Io {
                    observer: &mut observer,
                };
                uqm_rust::engine::Game::new()
                    .with_context(|cx| rust_complete_input_frame(cx, 1, &mut io));
                assert!(readback() == movie, "input boundary erased the movie");
                uqm_rust::graphics::state::flush();
                assert!(readback() == movie, "pacing flush erased the movie");
                swap_buffers(Redraw::Expose);
                assert!(readback() == movie, "expose erased the movie");
            }
            // Advance to the next scheduled movie frame, not a redraw of the current one.
            std::thread::sleep(Duration::from_secs_f64(1.0 / f64::from(DUCK_FPS)));
        }
        let movie = readback();
        fade_screen(252, 0);
        swap_buffers(Redraw::Fading);
        assert!(readback()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [0, 0, 0]));
        fade_screen(253, 0);
        swap_buffers(Redraw::Fading);
        assert!(
            readback() == movie,
            "fade restores the retained movie frame"
        );

        tfb_stop_video(&mut clip);
        draw_on_screen(Screen::Main, |canvas| canvas.words_mut().fill(0));
        swap_buffers(Redraw::Fading);
        assert!(readback()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [0, 0, 0]));
        drop(mount);
        uqm_rust::graphics::present::uninit_graphics();
    }
}
