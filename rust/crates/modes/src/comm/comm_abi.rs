//! Supplied conversation state and retained subtitle rendering.

use super::locdata::CPoint;
use super::locdata::CRect;
use super::sis_graphics::suffix;
use super::sis_graphics::Text;
use super::Comm;
use std::ffi::CString;
use uqm_engine::graphics::{context::ContextLease, font::FontLease};
use uqm_engine::sound::trackplayer::{SubtitleIdentity, SubtitleSnapshot};

impl Comm {
    pub fn set_conversation_phrases(
        &mut self,
        phrases: Option<uqm_engine::strings::conversation::ConversationTable>,
    ) {
        self.display.phrases = phrases;
    }

    pub fn conversation_phrases(
        &self,
    ) -> Option<&uqm_engine::strings::conversation::ConversationTable> {
        self.display.phrases.as_ref()
    }
}

fn count_lines(comm: &Comm, text: &mut Text<'_>, font: &FontLease) -> i32 {
    let mut input = text.text;
    let mut count = 0;
    loop {
        count += 1;
        text.text = input;
        let fitted = text.fit(font, comm.data.definition.alien_text_width as i16, u16::MAX);
        input = suffix(input, fitted.next);
        if fitted.end_of_line {
            break;
        }
    }
    text.text = input;
    count
}

fn layout_subtitle(comm: &Comm, cache: &ContextLease, font: &FontLease, text: &mut Text<'_>) {
    let leading = font.leading();
    let mut input = text.text;
    let valign = comm.data.definition.alien_text_valign;
    if valign & 3 != 0 {
        let lines = count_lines(comm, text, font);
        if valign == 2 {
            text.baseline.y = (i32::from(text.baseline.y) - i32::from(leading) * lines) as i16;
        } else if valign == 1 {
            text.baseline.y =
                (i32::from(text.baseline.y) - (i32::from(leading) * lines) / 2) as i16;
        }
        text.baseline.y = text.baseline.y.max(0);
    }
    let mut remaining = u16::MAX;
    loop {
        text.text = input;
        text.baseline.y = text.baseline.y.wrapping_add(leading);
        let fitted = text.fit(
            font,
            comm.data.definition.alien_text_width as i16,
            remaining,
        );
        remaining = remaining.wrapping_sub(text.char_count).saturating_sub(1);
        input = suffix(input, fitted.next);
        let (line, _) = uqm_engine::strings::unicode::split_utf8_prefix(
            text.text.to_bytes(),
            text.char_count as usize,
        );
        let line = CString::from(
            line.iter()
                .copied()
                .map_while(std::num::NonZeroU8::new)
                .collect::<Vec<_>>(),
        );
        cache.draw_traced_text(
            font,
            &line,
            text.baseline,
            uqm_engine::graphics::geometry::TextAlign::from_c(text.align),
            comm.data.definition.alien_text_fcolor,
            comm.data.definition.alien_text_bcolor,
        );
        if fitted.end_of_line || remaining == 0 {
            break;
        }
    }
}

fn draw_subtitle(comm: &mut Comm, snapshot: &SubtitleSnapshot) -> i16 {
    let super::sis_graphics::SubtitleCache {
        target,
        context: cache,
        frame,
        font,
    } = comm.conversation_display().subtitles.clone();
    target.with_selected(|| {
        target.with_batch(|| {
            if comm.display.last_subtitle == Some(snapshot.identity) {
                target.draw_stamp(&frame, CPoint { x: 0, y: 0 });
                return comm.display.last_baseline;
            }
            let mut text = Text::new(
                &snapshot.text,
                comm.display.subtitle.baseline,
                comm.display.subtitle.align,
                u16::MAX,
            );
            cache.with_selected(|| {
                cache.clear();
                comm.display.last_subtitle = Some(snapshot.identity);
                cache.set_font(&font);
                layout_subtitle(comm, &cache, &font, &mut text);
            });
            target.draw_stamp(&frame, CPoint { x: 0, y: 0 });
            comm.display.last_baseline = text.baseline.y;
            text.baseline.y
        })
    })
}

pub fn clear_subtitles(comm: &mut Comm) {
    comm.display.clear_subtitles = true;
    comm.display.last_subtitle = None;
    comm.display.subtitle.current = None;
}

pub fn check_subtitles(comm: &mut Comm) {
    comm.apply_phrase_events();
    let current = uqm_engine::sound::trackplayer::subtitle_snapshot();
    let identity = current.as_ref().map(|snapshot| snapshot.identity);
    let previous = comm
        .display
        .subtitle
        .current
        .as_ref()
        .map(|snapshot| snapshot.identity);
    let baseline = comm.data.definition.alien_text_baseline;
    let align = comm.data.definition.alien_text_align as i32;
    if identity != previous
        || comm.display.subtitle.baseline.x != baseline.x
        || comm.display.subtitle.baseline.y != baseline.y
        || comm.display.subtitle.align != align
    {
        comm.display.clear_subtitles = true;
        comm.display.subtitle.baseline = baseline;
        comm.display.subtitle.align = align;
        if previous == identity {
            uqm_engine::log_abi::log_message(
                uqm_engine::logging::LogLevel::Warning,
                "Dialog text and location changed out of sync",
            );
        }
        comm.display.subtitle.current = current;
    }
}

pub fn comm_redraw_subtitles(comm: &mut Comm, enabled: bool) {
    if enabled {
        if let Some(current) = comm.display.subtitle.current.clone() {
            draw_subtitle(comm, &current);
        }
    }
}

pub fn setup_subtitle_text_from_comm_data(comm: &mut Comm) {
    comm.display.subtitle.baseline = comm.data.definition.alien_text_baseline;
    comm.display.subtitle.align = comm.data.definition.alien_text_align as i32;
}

pub fn clear_subtitles_pending(comm: &mut Comm) -> i32 {
    i32::from(comm.display.clear_subtitles)
}
pub fn reset_clear_subtitles(comm: &mut Comm) {
    comm.display.clear_subtitles = false;
}

/// Run the conversation's speech segue for `track` on the supplied game owners.
pub fn alien_talk_segue(comm: &mut Comm, cx: &mut uqm_engine::battle::Cx<'_>, track: u16) {
    {
        let runtime = cx.kernel.input_runtime();
        let mut observer = runtime.observer();
        let mut io = uqm_engine::observe::Io {
            observer: &mut *observer,
        };
        super::ffi::observe_responses(comm, &mut io);
        super::ffi::rust_alien_talk_segue(comm, runtime.input(), cx, &mut io, track.into());
    }
}

#[derive(Default)]
pub(crate) struct SubtitleState {
    pub(crate) current: Option<SubtitleSnapshot>,
    pub(crate) baseline: CPoint,
    pub(crate) align: i32,
}

pub(crate) struct RuntimeState {
    pub(crate) phrases: Option<uqm_engine::strings::conversation::ConversationTable>,
    pub(crate) resources: Option<super::sis_graphics::ConversationResources>,
    pub(crate) window: CRect,
    pub(crate) assets: Option<super::assets::Assets>,
    pub(crate) clear_subtitles: bool,
    pub(crate) subtitle: SubtitleState,
    pub(crate) last_subtitle: Option<SubtitleIdentity>,
    pub(crate) last_baseline: i16,
}
impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            phrases: None,
            resources: None,
            window: CRect {
                corner: CPoint { x: 7, y: 10 },
                width: 0,
                height: 0,
            },
            assets: None,
            clear_subtitles: false,
            subtitle: SubtitleState::default(),
            last_subtitle: None,
            last_baseline: 0,
        }
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn subtitle_invalidation_drains_only_the_supplied_sessions_callbacks() {
        let _: fn(&mut Comm) = clear_subtitles;
        let _: fn(&mut Comm) = setup_subtitle_text_from_comm_data;
        let _: fn(&mut Comm) = reset_clear_subtitles;
        let _: fn(&mut Comm) -> i32 = clear_subtitles_pending;
        fn move_baseline(comm: &mut Comm) {
            comm.data.definition.alien_text_baseline.y = 23;
        }
        let mut first = Comm::new();
        let second = Comm::new();
        first.data.definition.alien_text_baseline.x = 17;
        first.queue_phrase_callback(Some(move_baseline)).unwrap()(0);
        setup_subtitle_text_from_comm_data(&mut first);
        assert_eq!(first.display.subtitle.baseline.x, 17);
        check_subtitles(&mut first);
        assert_eq!(first.display.subtitle.baseline.y, 23);
        clear_subtitles(&mut first);
        assert_eq!(clear_subtitles_pending(&mut first), 1);
        comm_redraw_subtitles(&mut first, false);
        assert_eq!(clear_subtitles_pending(&mut first), 1);
        reset_clear_subtitles(&mut first);
        assert_eq!(clear_subtitles_pending(&mut first), 0);
        first.set_talking_animation(false);
        assert_eq!(first.data.definition.alien_talk_desc.anim_flags & 16, 16);
        first.set_talking_animation(true);
        assert_eq!(first.data.definition.alien_talk_desc.anim_flags & 16, 0);
        first.set_talking_finished(true);
        first.set_intro_transition(super::super::CommIntroMode::from(2), 840);
        assert_eq!(first.fade_time(), 840);
        assert!(!second.is_talking_finished());
        assert_eq!(second.display.subtitle.baseline.x, 0);
        assert_eq!(second.display.subtitle.baseline.y, 0);
        crate::comm::with_test_context(|cx| {
            crate::comm::input::fixture::install(cx);
            alien_talk_segue(&mut first, cx, 1);
        });
        assert!(!first.first_talk_call);
    }

    #[test]
    #[serial_test::serial]
    fn subtitle_bridge_renders_and_reuses_the_real_cache_without_test_substitutes() {
        super::super::sis_graphics_font_tests::with_display("comm::comm_abi::owner_tests::subtitle_bridge_renders_and_reuses_the_real_cache_without_test_substitutes", |comm, context| {
            use uqm_engine::graphics::context::LoadedContext;
use uqm_engine::graphics::{drawable::LoadedDrawable, geometry::Color, geometry::Point};
            use uqm_engine::sound::trackplayer;
            trackplayer::stop_track().unwrap();
            trackplayer::splice_track(Some("subtitle"), Some("A A"), None, None, vec![]).unwrap();
            let snapshot = trackplayer::subtitle_history().remove(0);
            trackplayer::stop_track().unwrap();
            let cache = LoadedContext::create(c"conversation subtitle fixture");
            let drawable = LoadedDrawable::create(8, 242, 122, 1).unwrap();
            let cache_context = cache.lease();
            cache_context.set_frame(drawable.lease().context_frame(0));
            comm.display.resources.as_mut().unwrap().display.as_mut().unwrap().subtitles = super::super::sis_graphics::SubtitleCache {
                target: context.clone(), context: cache_context.clone(),
                frame: drawable.lease().context_frame(0),
                font: FontLease::retain(comm.presentation.tiny_font.unwrap()),
            };
            comm.data.definition.alien_text_width = 100;
            comm.data.definition.alien_text_fcolor = Color { r: 255, g: 255, b: 255, a: 255 };
            comm.display.subtitle.baseline = Point { x: 10, y: 20 };
            comm.display.subtitle.current = Some(snapshot.clone());
            super::super::subtitle_display::SubtitleDisplay::redraw(comm, false);
            assert_eq!(comm.display.last_subtitle, None);
            super::super::subtitle_display::SubtitleDisplay::redraw(comm, true);
            context.flush();
            assert_eq!(comm.display.last_subtitle, Some(snapshot.identity));
            let pixels = || (0..122).flat_map(|y| (0..242).map(move |x| Point { x, y })).map(|point| drawable.lease().context_frame(0).pixel(point)).map(|pixel| (pixel.r, pixel.g, pixel.b, pixel.a)).collect::<Vec<_>>();
            let original = pixels();
            assert!(original.contains(&(255, 255, 255, 255)));
            comm_redraw_subtitles(comm, true);
            context.flush();
            assert_eq!(pixels(), original);
            clear_subtitles(comm);
            assert!(comm.display.subtitle.current.is_none());
            assert_eq!(comm.display.last_subtitle, None);
            let leading = FontLease::retain(comm.presentation.tiny_font.unwrap()).leading();
            for (valign, expected) in [(0, 20 + leading), (1, 20 - leading / 2 + leading), (2, 20), (3, 20 + leading)] {
                clear_subtitles(comm);
                comm.data.definition.alien_text_valign = valign;
                comm.display.subtitle.current = Some(snapshot.clone());
                comm_redraw_subtitles(comm, true);
                assert_eq!(comm.display.last_baseline, expected);
            }
        });
    }
}
