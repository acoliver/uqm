//! Supplied conversation state and retained subtitle rendering.

use super::{
    locdata::{CPoint, CRect},
    sis_graphics::{suffix, Text},
    Comm,
};
use crate::graphics::{
    context_abi::{ContextFrame, ContextLease},
    native_font::FontLease,
};
use crate::sound::trackplayer::{SubtitleIdentity, SubtitleSnapshot};
use std::{
    ffi::{c_void, CString},
    ptr::{null_mut, NonNull},
};

impl Comm {
    pub fn set_conversation_phrases(
        &mut self,
        phrases: Option<crate::strings::conversation::ConversationTable>,
    ) {
        self.display.phrases = phrases;
    }

    pub fn conversation_phrases(&self) -> Option<&crate::strings::conversation::ConversationTable> {
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
        let line: String = text
            .text
            .to_str()
            .expect("subtitle UTF-8")
            .chars()
            .take(text.char_count as usize)
            .collect();
        let line = CString::new(line).expect("subtitle line excludes NUL");
        cache.draw_traced_text(
            font,
            &line,
            text.baseline,
            text.align as u32,
            comm.data.definition.alien_text_fcolor,
            comm.data.definition.alien_text_bcolor,
        );
        if fitted.end_of_line || remaining == 0 {
            break;
        }
    }
}

fn draw_subtitle(comm: &mut Comm, snapshot: &SubtitleSnapshot) -> i16 {
    let target = ContextLease::require(comm.display.anim_context.cast());
    let frame = ContextFrame::retain(Some(
        NonNull::new(comm.display.text_cache_frame.cast()).expect("subtitle cache frame"),
    ));
    target.with_selected(|| {
        target.with_batch(|| {
            if comm.display.last_subtitle == Some(snapshot.identity) {
                target.draw_stamp(&frame, CPoint { x: 0, y: 0 });
                return comm.display.last_baseline;
            }
            let cache = ContextLease::require(comm.display.text_cache_context);
            let font = FontLease::retain(
                NonNull::new(comm.data.alien_font.cast()).expect("conversation alien font"),
            );
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
    let current = crate::sound::trackplayer::subtitle_snapshot();
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
            crate::log_abi::log_message(
                crate::log_abi::LOG_WARNING,
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

pub fn set_talking_finished(comm: &mut Comm, finished: i32) {
    super::ffi::rust_set_talking_finished(comm, finished);
}
pub fn setup_subtitle_text_from_comm_data(comm: &mut Comm) {
    comm.display.subtitle.baseline = comm.data.definition.alien_text_baseline;
    comm.display.subtitle.align = comm.data.definition.alien_text_align as i32;
}

/// # Safety
/// Context must remain live while registered; game thread only.
pub unsafe fn set_anim_context(comm: &mut Comm, context: *mut c_void) {
    comm.display.anim_context = context;
}
/// # Safety
/// Context must remain live while registered; game thread only.
pub unsafe fn set_text_cache_context(comm: &mut Comm, context: *mut c_void) {
    comm.display.text_cache_context = (context).cast();
}
/// # Safety
/// Frame must remain live while registered; game thread only.
pub unsafe fn set_text_cache_frame(comm: &mut Comm, frame: *mut c_void) {
    comm.display.text_cache_frame = frame;
}
pub fn anim_context(comm: &mut Comm) -> *mut c_void {
    comm.display.anim_context
}
pub fn clear_subtitles_pending(comm: &mut Comm) -> i32 {
    i32::from(comm.display.clear_subtitles)
}
pub fn reset_clear_subtitles(comm: &mut Comm) {
    comm.display.clear_subtitles = false;
}

/// # Safety
/// The game thread must have an active published communication context.
pub unsafe fn alien_talk_segue(
    comm: &mut Comm,
    cx: &mut crate::engine::battle::Cx<'_>,
    track: u16,
) {
    {
        let runtime = cx.kernel.input_runtime();
        let mut observer = runtime.observer();
        let mut io = crate::engine::observe::Io {
            observer: &mut *observer,
        };
        super::ffi::observe_responses(comm, &mut io);
        super::ffi::rust_alien_talk_segue(comm, runtime.input(), cx, &mut io, track.into());
    }
}

pub unsafe fn enable_talking_anim(comm: &mut Comm, enable: i32) {
    if enable != 0 {
        comm.data.definition.alien_talk_desc.anim_flags &= !16;
    } else {
        comm.data.definition.alien_talk_desc.anim_flags |= 16;
    }
}
pub fn set_comm_intro_mode(comm: &mut Comm, mode: u32, duration: u32) {
    super::ffi::set_comm_intro_mode(comm, mode, duration);
}

#[derive(Default)]
pub(crate) struct SubtitleState {
    pub(crate) current: Option<SubtitleSnapshot>,
    pub(crate) baseline: CPoint,
    pub(crate) align: i32,
}

pub(crate) struct RuntimeState {
    pub(crate) phrases: Option<crate::strings::conversation::ConversationTable>,
    pub(crate) game_strings: Option<crate::strings::owned_table::LoadedTextTable>,
    pub(crate) window: CRect,
    pub(crate) anim_context: *mut c_void,
    pub(crate) text_cache_context: *mut crate::graphics::native_abi::context_desc,
    pub(crate) text_cache_frame: *mut c_void,
    pub(crate) clear_subtitles: bool,
    pub(crate) subtitle: SubtitleState,
    pub(crate) last_subtitle: Option<SubtitleIdentity>,
    pub(crate) last_baseline: i16,
}
impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            phrases: None,
            game_strings: None,
            window: CRect {
                corner: CPoint { x: 7, y: 10 },
                width: 0,
                height: 0,
            },
            anim_context: null_mut(),
            text_cache_context: null_mut(),
            text_cache_frame: null_mut(),
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
        let _: fn(&mut Comm) -> *mut c_void = anim_context;
        let _: fn(&mut Comm) = reset_clear_subtitles;
        let _: fn(&mut Comm) -> i32 = clear_subtitles_pending;
        fn move_baseline(comm: &mut Comm) {
            comm.data.definition.alien_text_baseline.y = 23;
        }
        let mut first = Comm::new();
        let second = Comm::new();
        first.data.definition.alien_text_baseline.x = 17;
        first.queue_phrase_callback(Some(move_baseline)).unwrap()(0);
        // SAFETY: the serialized fixture owns both sessions and uses unloaded, null resource handles.
        unsafe {
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
            enable_talking_anim(&mut first, 0);
            assert_eq!(first.data.definition.alien_talk_desc.anim_flags & 16, 16);
            enable_talking_anim(&mut first, 1);
            assert_eq!(first.data.definition.alien_talk_desc.anim_flags & 16, 0);
            set_talking_finished(&mut first, 1);
        }
        set_comm_intro_mode(&mut first, 2, 840);
        assert_eq!(first.fade_time(), 840);
        assert!(!second.is_talking_finished());
        assert_eq!(second.display.subtitle.baseline.x, 0);
        assert_eq!(second.display.subtitle.baseline.y, 0);
        crate::comm::with_test_context(|cx| {
            crate::comm::input::fixture::install(cx);
            // SAFETY: the fixture installed its input driver in the supplied exclusive Cx.
            unsafe {
                alien_talk_segue(&mut first, cx, 1);
            }
        });
        assert!(!first.first_talk_call);
    }

    #[test]
    #[serial_test::serial]
    fn render_registrations_borrow_resources_for_only_the_supplied_session() {
        use crate::graphics::{context_abi, drawable_abi, pixmap_abi};
        // SAFETY: the serialized fixture creates its own drawable/context and frees them after borrowed registrations end.
        unsafe {
            #[cfg(feature = "debug-process")]
            let context = context_abi::CreateContextAux(c"comm-session".as_ptr());
            #[cfg(not(feature = "debug-process"))]
            let context = context_abi::CreateContextAux();
            let drawable = drawable_abi::CreateDrawable(0, 0, 0, 1);
            let frame = pixmap_abi::CaptureDrawable(drawable);
            assert!(!context.is_null());
            assert!(!frame.is_null());
            let mut first = Comm::new();
            let second = Comm::new();
            assert!(first.add_response(1, "X", None));
            set_anim_context(&mut first, context.cast());
            set_text_cache_context(&mut first, context.cast());
            set_text_cache_frame(&mut first, frame.cast());
            assert_eq!(anim_context(&mut first), context.cast());
            assert_eq!(first.display.text_cache_context, context);
            assert_eq!(first.display.text_cache_frame, frame.cast());
            assert!(second.display.anim_context.is_null());
            assert!(second.display.text_cache_context.is_null());
            assert!(second.display.text_cache_frame.is_null());
            assert_eq!(second.responses().count(), 0);
            drop(first);
            assert_eq!(drawable_abi::DestroyDrawable(drawable), 1);
            assert_eq!(context_abi::DestroyContext(context), 1);
        }
    }
    #[test]
    #[serial_test::serial]
    fn subtitle_bridge_renders_and_reuses_the_real_cache_without_test_substitutes() {
        super::super::sis_graphics_font_tests::with_display("comm::comm_abi::owner_tests::subtitle_bridge_renders_and_reuses_the_real_cache_without_test_substitutes", |comm, context| {
            use crate::graphics::{context_abi::LoadedContext, drawable_abi::LoadedDrawable, native_abi::{Color, POINT}};
            use crate::sound::trackplayer;
            trackplayer::stop_track().unwrap();
            trackplayer::splice_track(Some("subtitle"), Some("A A"), None, None, vec![]).unwrap();
            let snapshot = trackplayer::subtitle_history().remove(0);
            trackplayer::stop_track().unwrap();
            let cache = LoadedContext::create(c"conversation subtitle fixture");
            let drawable = LoadedDrawable::create(8, 242, 122, 1).unwrap();
            let cache_context = cache.lease();
            cache_context.set_frame(drawable.lease().context_frame(0));
            comm.display.anim_context = context.as_raw().cast();
            comm.display.text_cache_context = cache_context.as_raw();
            comm.display.text_cache_frame = drawable.lease().context_frame(0).as_raw().cast();
            comm.data.alien_font = comm.presentation.tiny_font.cast();
            comm.data.definition.alien_text_width = 100;
            comm.data.definition.alien_text_fcolor = Color { r: 255, g: 255, b: 255, a: 255 };
            comm.display.subtitle.baseline = POINT { x: 10, y: 20 };
            comm.display.subtitle.current = Some(snapshot.clone());
            super::super::subtitle_display::SubtitleDisplay::redraw(comm, false);
            assert_eq!(comm.display.last_subtitle, None);
            super::super::subtitle_display::SubtitleDisplay::redraw(comm, true);
            context.flush();
            assert_eq!(comm.display.last_subtitle, Some(snapshot.identity));
            let pixels = || (0..122).flat_map(|y| (0..242).map(move |x| POINT { x, y })).map(|point| drawable.lease().context_frame(0).pixel(point)).map(|pixel| (pixel.r, pixel.g, pixel.b, pixel.a)).collect::<Vec<_>>();
            let original = pixels();
            assert!(original.contains(&(255, 255, 255, 255)));
            comm_redraw_subtitles(comm, true);
            context.flush();
            assert_eq!(pixels(), original);
            clear_subtitles(comm);
            assert!(comm.display.subtitle.current.is_none());
            assert_eq!(comm.display.last_subtitle, None);
            let leading = FontLease::retain(NonNull::new(comm.data.alien_font.cast()).unwrap()).leading();
            for (valign, expected) in [(0, 20 + leading), (1, 20 - leading / 2 + leading), (2, 20), (3, 20 + leading)] {
                clear_subtitles(comm);
                comm.data.definition.alien_text_valign = valign;
                comm.display.subtitle.current = Some(snapshot.clone());
                comm_redraw_subtitles(comm, true);
                assert_eq!(comm.display.last_baseline, expected);
            }
            comm.data.alien_font = std::ptr::null_mut();
        });
    }
}
