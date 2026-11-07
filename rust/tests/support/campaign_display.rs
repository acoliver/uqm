//! Display resources for campaign callbacks that also redraw their status label.
use std::{ffi::CString, ptr};
use uqm_rust::{
    engine::Game,
    graphics::{
        context_abi::{CreateContextAux, DestroyContext, SetContext},
        drawable_abi::LoadedDrawable,
        native_abi::CONTEXT,
        native_font::LoadedFont,
        sdl::lifecycle_abi::{GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics},
    },
    io::uio_bridge as uio,
};

/// Run process-global Flash fixtures without retaining allocations on another test's thread.
pub fn own_graphics_process(case: &str) -> bool {
    let case = case.strip_prefix("uqm_rust::").unwrap_or(case);
    const CHILD: &str = "UQM_CAMPAIGN_DISPLAY_CASE";
    if std::env::var(CHILD).as_deref() == Ok(case) {
        return true;
    }
    let status = std::process::Command::new(
        std::env::current_exe()
            .unwrap_or_else(|error| panic!("test executable unavailable: {error}")),
    )
    .args(["--exact", case, "--nocapture"])
    .env(CHILD, case)
    .status()
    .unwrap_or_else(|error| panic!("campaign panel child failed: {error}"));
    assert!(status.success(), "campaign panel child: {status}");
    false
}

pub struct CampaignDisplay {
    context: CONTEXT,
    previous: CONTEXT,
    _screen: Option<LoadedDrawable>,
}

impl CampaignDisplay {
    pub fn install(game: &mut Game) -> Result<Self, Box<dyn std::error::Error>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
        std::fs::create_dir_all(&root)?;
        let directory = tempfile::tempdir_in(root.canonicalize()?)?;
        std::fs::create_dir(directory.path().join("font"))?;
        for (code, width) in [(0x41, 1), (0x31, 1), (0x32, 2), (0x2e, 1), (0x3a, 1)] {
            image::RgbaImage::from_pixel(width, 1, image::Rgba([255, 255, 255, 255]))
                .save(directory.path().join(format!("font/{code:04x}.png")))?;
        }
        std::fs::write(
            directory.path().join("fixture.rmp"),
            "fixture.tiny = FONTRES:font\n",
        )?;
        let host = CString::new(
            directory
                .path()
                .to_str()
                .ok_or("fixture path is not UTF-8")?,
        )?;
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
        // SAFETY: the serialized fixture owns SDL and the temporary resource mount.
        unsafe {
            assert_eq!(
                TFB_InitGraphics(
                    1,
                    0,
                    ptr::from_ref(&GraphicsOptions::default()).cast(),
                    320,
                    240
                ),
                0
            );
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
                6,
                ptr::null_mut(),
            );
            assert!(!mount.is_null());
            let content = uio::uio_openDir(repository, c"/".as_ptr(), 0);
            let previous_content = uqm_rust::mainloop::dir_prep::contentDir;
            uqm_rust::mainloop::dir_prep::contentDir = content.cast();
            uqm_rust::resource::ffi_bridge::InitResourceSystem();
            uqm_rust::resource::ffi_bridge::LoadResourceIndex(
                content.cast(),
                c"fixture.rmp".as_ptr(),
                ptr::null(),
            );
            game.kernel.install_tiny_font(
                LoadedFont::load(c"fixture.tiny").ok_or("tiny-font fixture did not load")?,
            );
            game.kernel.install_micro_font(
                LoadedFont::load(c"fixture.tiny").ok_or("micro-font fixture did not load")?,
            );
            game.kernel.install_battle_font(
                LoadedFont::load(c"fixture.tiny").ok_or("battle-font fixture did not load")?,
            );
            uqm_rust::mainloop::dir_prep::contentDir = previous_content;
            uio::uio_closeDir(content);
            uio::uio_closeRepository(repository);
            uio::uio_unInit();
            #[cfg(feature = "debug-process")]
            let context = CreateContextAux(c"campaign callback fixture".as_ptr());
            #[cfg(not(feature = "debug-process"))]
            let context = CreateContextAux();
            let previous = SetContext(context);
            let screen =
                LoadedDrawable::create(8, 320, 240, 1).ok_or("screen fixture allocation failed")?;
            game.kernel.status_context = context;
            game.kernel.off_screen_context = context;
            game.kernel.screen = screen
                .lease()
                .frame(0)
                .ok_or("fixture screen has no frame")?;
            let count = (uqm_rust::navigation::abi::MONTHS_STRING_BASE as usize + 12)
                .max(uqm_rust::navigation::abi::STATUS_STRING_BASE as usize + 3);
            game.kernel.install_game_text(
                uqm_rust::strings::owned_table::LoadedTextTable::fixture(vec![
                    b"A\0".to_vec();
                    count
                ]),
            );
            Ok(Self {
                context,
                previous,
                _screen: Some(screen),
            })
        }
    }
    pub fn finish(self, game: Game) {
        // SAFETY: the fixture retains its queued drawing resources until this flush.
        unsafe { uqm_rust::graphics::dcqueue_abi::TFB_FlushGraphics() };
        drop(game);
    }
}

impl Drop for CampaignDisplay {
    fn drop(&mut self) {
        // SAFETY: the serialized fixture restores selection before releasing its context.
        unsafe {
            SetContext(self.previous);
            DestroyContext(self.context);
            drop(self._screen.take());
            TFB_UninitGraphics();
        }
    }
}
