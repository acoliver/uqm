//! Display resources for campaign callbacks that also redraw their status label.
use crate::{
    graphics::{
        context::{self, ContextId, LoadedContext},
        drawable::LoadedDrawable,
        font::LoadedFont,
        present::{init_graphics, uninit_graphics, GraphicsOptions},
    },
    world::Game,
};

/// Entries in the fixture's game-text table. It must reach past the highest
/// months and status string ids that campaign callbacks read.
pub const GAME_TEXT_ENTRIES: usize = 470;

pub struct CampaignDisplay {
    context: Option<LoadedContext>,
    previous: Option<ContextId>,
    _screen: Option<LoadedDrawable>,
}

impl CampaignDisplay {
    pub fn install(game: &mut Game) -> Result<Self, Box<dyn std::error::Error>> {
        Self::install_with_resources(game, |_| {})
    }

    pub fn install_with_resources(
        game: &mut Game,
        load: impl FnOnce(&crate::resource::Content),
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::install_prepared(game, load, |_| {})
    }

    pub fn install_prepared(
        game: &mut Game,
        load: impl FnOnce(&crate::resource::Content),
        draw: impl FnOnce(&mut Game),
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::install_prepared_with_options(game, load, draw, GraphicsOptions::default())
    }

    pub fn install_prepared_with_options(
        game: &mut Game,
        load: impl FnOnce(&crate::resource::Content),
        draw: impl FnOnce(&mut Game),
        options: GraphicsOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tmp");
        std::fs::create_dir_all(&root)?;
        let directory = tempfile::tempdir_in(root.canonicalize()?)?;
        std::fs::create_dir(directory.path().join("font"))?;
        for (code, width) in [(0x41, 1), (0x31, 1), (0x32, 2), (0x2e, 1), (0x3a, 1)] {
            image::RgbaImage::from_pixel(width, 1, image::Rgba([255, 255, 255, 255]))
                .save(directory.path().join(format!("font/{code:04x}.png")))?;
        }
        image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
            .save(directory.path().join("effect.png"))?;
        std::fs::write(directory.path().join("effect.ani"), "effect.png -1 0 0 0\n")?;
        let mut comm_frames = String::new();
        for (index, color) in [[180, 0, 0, 255], [0, 180, 0, 255], [0, 0, 180, 255]]
            .into_iter()
            .enumerate()
        {
            let name = format!("comm-{index}.png");
            image::RgbaImage::from_pixel(242, 100, image::Rgba(color))
                .save(directory.path().join(&name))?;
            comm_frames.push_str(&format!("{name} -1 0 0 0\n"));
        }
        std::fs::write(directory.path().join("comm.ani"), comm_frames)?;
        std::fs::write(
            directory.path().join("fixture.rmp"),
            "fixture.tiny = FONTRES:font\nfont.player = FONTRES:font\nfixture.comm.font = FONTRES:font\nfixture.comm.graphics = GFXRES:comm.ani\nfont.lander = FONTRES:font\ngraphics.landerfonteffect = GFXRES:effect.ani\ncredits.credits = STRTAB:credits.txt\ncredits.background = GFXRES:effect.ani\ngraphics.newgame = GFXRES:effect.ani\ncredits.font.pt13 = FONTRES:font\ncredits.font.pt17 = FONTRES:font\ncredits.font.pt45 = FONTRES:font\ngraphics.flagshipskeleton = GFXRES:effect.ani\ngraphics.outfitmodules = GFXRES:effect.ani\n",
        )?;
        std::fs::write(
            directory.path().join("credits.txt"),
            "#(credits)\n13 L/10 A\n",
        )?;
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
        init_graphics(1, 0, options, 320, 240);
        {
            let content = crate::test_support::native_session::ContentMount::new(directory.path());
            let content = content.load_index("fixture.rmp");
            game.kernel.admit_content(content.clone());
            game.kernel.install_tiny_font(
                LoadedFont::load(&content, c"fixture.tiny")
                    .ok_or("tiny-font fixture did not load")?,
            );
            game.kernel.install_micro_font(
                LoadedFont::load(&content, c"fixture.tiny")
                    .ok_or("micro-font fixture did not load")?,
            );
            game.kernel.install_battle_font(
                LoadedFont::load(&content, c"fixture.tiny")
                    .ok_or("battle-font fixture did not load")?,
            );
            load(&content);
            let owner = LoadedContext::create(c"campaign callback fixture");
            let context = owner.context();
            let previous = context::select(Some(context));
            let screen =
                LoadedDrawable::create(8, 320, 240, 1).ok_or("screen fixture allocation failed")?;
            game.kernel.status_context = Some(context);
            game.kernel.off_screen_context = Some(context);
            game.kernel.screen = screen.first_frame();
            game.kernel
                .install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(vec![
                    b"A\0".to_vec();
                    GAME_TEXT_ENTRIES
                ]));
            draw(game);
            drop(content);
            Ok(Self {
                context: Some(owner),
                previous,
                _screen: Some(screen),
            })
        }
    }
    pub fn finish(&self, game: Game) {
        crate::graphics::state::flush();
        drop(game);
    }
}

impl Drop for CampaignDisplay {
    fn drop(&mut self) {
        context::select(self.previous);
        drop(self.context.take());
        drop(self._screen.take());
        uninit_graphics();
    }
}
