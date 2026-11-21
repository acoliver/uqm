//! Movie frames must survive the same input, pacing and expose boundaries as slides.
#![forbid(unsafe_code)]
use std::{cell::RefCell, path::Path, rc::Rc, time::Duration};
use uqm_engine::graphics::present::{swap_buffers, PresentObserver, Redraw};
use uqm_engine::graphics::presentation_diagnostics::PresentationSample;
use uqm_engine::graphics::presented_frame::PresentedFrame;
use uqm_engine::graphics::state::{draw_on_screen, fade_screen};
use uqm_engine::graphics::{dcq::Screen, geometry::Extent, geometry::Point, geometry::Rect};
use uqm_engine::test_support::native_probe::register_rect;
use uqm_engine::video::{decoder::DukVideoDecoder, legacy::LoadedVideo, DUCK_FPS};
use uqm_rust::mainloop::input_frame::rust_complete_input_frame;

/// The last frame the window presented, read back by the display.
#[derive(Clone, Default)]
struct Window(Rc<RefCell<Option<Vec<u8>>>>);

impl PresentObserver for Window {
    fn suppress_present(&mut self) -> bool {
        false
    }
    fn capture(&mut self, count: u64, _: &[PresentationSample]) -> Option<u64> {
        Some(count)
    }
    fn presented(&mut self, frame: Option<PresentedFrame>) {
        *self.0.borrow_mut() = Some(
            frame
                .unwrap_or_else(|| panic!("the display reads every frame back"))
                .rgba,
        );
    }
}

impl Window {
    fn shown(&self) -> Vec<u8> {
        self.0
            .borrow()
            .clone()
            .unwrap_or_else(|| panic!("a frame was presented"))
    }
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
    std::fs::write(
        fixture.path().join("video.rmp"),
        b"fixture.logo = 3DOVID:logo.duk\n",
    )
    .unwrap();
    let mut decoder = DukVideoDecoder::open(fixture.path(), "logo").unwrap();
    let first = decoder.decode_frame(0).unwrap();
    assert!(first.data.iter().any(|p| p & 0x00ff_ffff != 0));

    // This process owns its SDL instance; no native window or user config is used.
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    uqm_platform::install(false);
    let window = Window::default();
    let options = uqm_engine::graphics::present::GraphicsOptions {
        observer: Some(Box::new(window.clone())),
        ..Default::default()
    };
    {
        uqm_engine::graphics::present::init_graphics(0, 0, options, 320, 240);
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
        uqm_engine::resource::load_index(
            &uqm_engine::io::content_dir().unwrap(),
            c"video.rmp",
            None,
        );
        let playback = LoadedVideo::load(c"fixture.logo")
            .and_then(|video| video.play())
            .expect("logo playback");

        for _ in 0..3 {
            assert!(playback.process_frame());
            let movie = window.shown();
            assert!(movie.as_chunks::<4>().0.iter().any(|p| p[..3] != [0, 0, 0]));
            for _ in 0..3 {
                let mut observer = uqm_engine::observe::Unobserved;
                let mut io = uqm_engine::observe::Io {
                    observer: &mut observer,
                };
                uqm_engine::Game::new()
                    .with_context(|cx| rust_complete_input_frame(cx, 1, &mut io));
                assert!(window.shown() == movie, "input boundary erased the movie");
                uqm_engine::graphics::state::flush();
                assert!(window.shown() == movie, "pacing flush erased the movie");
                swap_buffers(Redraw::Expose);
                assert!(window.shown() == movie, "expose erased the movie");
            }
            // Advance to the next scheduled movie frame, not a redraw of the current one.
            std::thread::sleep(Duration::from_secs_f64(1.0 / f64::from(DUCK_FPS)));
        }
        let movie = window.shown();
        fade_screen(252, 0);
        swap_buffers(Redraw::Fading);
        assert!(window
            .shown()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [0, 0, 0]));
        fade_screen(253, 0);
        swap_buffers(Redraw::Fading);
        assert!(
            window.shown() == movie,
            "fade restores the retained movie frame"
        );

        drop(playback);
        draw_on_screen(Screen::Main, |canvas| canvas.words_mut().fill(0));
        swap_buffers(Redraw::Fading);
        assert!(window
            .shown()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[..3] == [0, 0, 0]));
        drop(mount);
        uqm_engine::graphics::present::uninit_graphics();
    }
}
