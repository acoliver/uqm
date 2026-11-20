//! Loaded conversation assets and their ordered release authority.

use super::races::descriptor::CommDescriptor;
use super::sis_graphics::SubtitleCache;
use super::Comm;
use uqm_engine::graphics::{
    context::ContextLease, context::LoadedContext, drawable::LoadedDrawable, font::FontLease,
    font::LoadedFont, geometry::Color, palette_cycle::PaletteCycle,
};
use uqm_engine::sound::resource_types::LoadedMusic;

pub(crate) struct Assets {
    pub frame: LoadedDrawable,
    pub font: LoadedFont,
    pub palette: Option<PaletteCycle>,
    pub song: Option<LoadedMusic>,
    pub player: LoadedFont,
    pub cache_context: LoadedContext,
    pub cache_frame: LoadedDrawable,
    pub animation: Option<LoadedContext>,
    pub old_font: Option<FontLease>,
    pub intro_batch: Option<uqm_engine::graphics::context::BatchGuard>,
}

impl Assets {
    pub fn load(descriptor: &CommDescriptor, dimensions: (i32, i32)) -> Result<Self, &'static str> {
        let player = LoadedFont::load(c"font.player").ok_or("font.player failed to load")?;
        let frame = descriptor
            .alien_frame_res
            .and_then(LoadedDrawable::load)
            .ok_or("alien graphics failed to load")?;
        let font = descriptor
            .alien_font_res
            .and_then(LoadedFont::load)
            .ok_or("alien font failed to load")?;
        let palette = descriptor.alien_colormap_res.and_then(PaletteCycle::load);
        let song = if descriptor.alien_song_flags & 1 != 0 {
            descriptor.alien_alt_song_res.and_then(LoadedMusic::load)
        } else {
            None
        }
        .or_else(|| descriptor.alien_song_res.and_then(LoadedMusic::load));
        let cache_context = LoadedContext::create(c"TextCacheContext");
        let cache_frame = LoadedDrawable::create(
            2,
            (dimensions.0 - 78) as i16,
            (dimensions.1 - 13 - 107 - 15 + 2) as i16,
            1,
        )
        .ok_or("subtitle cache failed to allocate")?;
        let context = cache_context.lease();
        let target = cache_frame.lease().context_frame(0);
        let background = Color {
            r: 0,
            g: 0,
            b: (0x10 << 3) | (0x10 >> 2),
            a: 255,
        };
        context.set_frame(target.clone());
        context.set_background(background);
        context.with_selected(|| context.clear());
        target.transparent_color(background);
        Ok(Self {
            player,
            frame,
            font,
            palette,
            song,
            cache_context,
            cache_frame,
            animation: Some(LoadedContext::create(c"AnimContext")),
            old_font: None,
            intro_batch: None,
        })
    }

    pub fn subtitles(&self) -> SubtitleCache {
        SubtitleCache {
            target: self
                .animation
                .as_ref()
                .unwrap_or_else(|| panic!("animation context released"))
                .lease(),
            context: self.cache_context.lease(),
            frame: self.cache_frame.lease().context_frame(0),
            font: self.font.lease(),
        }
    }

    pub fn release(self, space: &ContextLease) {
        let Self {
            frame,
            font,
            palette,
            song,
            player,
            cache_context,
            cache_frame,
            animation,
            old_font: _,
            intro_batch,
        } = self;
        space.drain();
        drop(intro_batch);
        drop(animation);
        drop(song);
        drop(palette);
        drop(font);
        drop(frame);
        drop(cache_context);
        drop(cache_frame);
        drop(player);
    }
}

impl Comm {
    pub(crate) fn assets(&self) -> &Assets {
        self.display
            .assets
            .as_ref()
            .unwrap_or_else(|| panic!("conversation assets not loaded"))
    }

    pub(crate) fn animation_context(&self) -> ContextLease {
        self.conversation_display().subtitles.target.clone()
    }

    pub(crate) fn draw_speech_widgets(&mut self) {
        let radar = ContextLease::require(self.presentation.radar_context);
        let space = self.conversation_display().space.clone();
        radar.with_selected(|| super::oscill_abi::runtime::draw_oscilloscope(self));
        space.with_selected(|| super::oscill_abi::runtime::draw_slider(self));
    }
}
