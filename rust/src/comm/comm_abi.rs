//! Remaining live dialogue API and subtitle cache, ported from comm.c.
//! Copyright Paul Reiche, Fred Ford, 1992-2002. GPL-2.0-or-later.

use crate::comm::locdata::{CPoint, CRect, CStamp};
use crate::comm::Comm;

use super::sis_graphics::CText;
use std::ffi::{c_char, c_void};
use std::ptr::{null, null_mut};

use crate::graphics::context_abi::SetContext;

use crate::graphics::font_abi::font_DrawTracedText;

use crate::graphics::font_abi::GetContextFontLeading;

use crate::graphics::font_abi::SetContextFont;

use crate::graphics::font_abi::TextRect;

use crate::graphics::frame_abi::ClearDrawable;

use crate::graphics::frame_abi::DrawStamp;
use crate::sound::heart_ffi::get_track_subtitle as GetTrackSubtitle;

use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

/// # Safety
/// Text and next must be writable; text must reference a NUL-terminated string.
pub unsafe fn get_line_within_width(
    text: *mut CText,
    next: *mut *const c_char,
    width: i16,
    max_chars: u16,
) -> i32 {
    i32::from(super::text_layout::fit_line(
        &mut *text,
        &mut *next,
        width,
        max_chars,
        |text| {
            let mut rect = CRect::default();
            TextRect(
                std::ptr::from_mut(text).cast(),
                std::ptr::from_mut(&mut rect).cast(),
                null_mut(),
            );
            rect.width
        },
    ))
}

unsafe fn count_lines(comm: &mut Comm, text: &mut CText) -> i32 {
    let width = comm.data.alien_text_width as i16;
    SetContextFont((comm.data.alien_font).cast());
    let mut ptr = text.p_str;
    let mut count = 0;
    loop {
        count += 1;
        text.p_str = ptr;
        if get_line_within_width(text, &mut ptr, width, u16::MAX) != 0 {
            break;
        }
    }
    text.p_str = ptr;
    count
}

unsafe fn draw_subtitle(comm: &mut Comm, text: &mut CText) -> i16 {
    BatchGraphics();
    let mut stamp = CStamp {
        origin: CPoint { x: 0, y: 0 },
        frame: comm.display.text_cache_frame,
    };
    if comm.display.last_subtitle == text.p_str {
        DrawStamp(std::ptr::from_mut(&mut stamp).cast());
        UnbatchGraphics();
        return comm.display.last_baseline;
    }
    let old_context = SetContext(comm.display.text_cache_context);
    ClearDrawable();
    comm.display.last_subtitle = text.p_str;
    let width = comm.data.alien_text_width as i16;
    SetContextFont((comm.data.alien_font).cast());
    let mut leading = 0i16;
    GetContextFontLeading(&mut leading);
    let mut ptr = text.p_str;
    if comm.data.alien_text_valign & 3 != 0 {
        let lines = count_lines(comm, text);
        if comm.data.alien_text_valign == 2 {
            text.baseline.y = (i32::from(text.baseline.y) - i32::from(leading) * lines) as i16;
        } else if comm.data.alien_text_valign == 1 {
            text.baseline.y =
                (i32::from(text.baseline.y) - (i32::from(leading) * lines) / 2) as i16;
        }
        if text.baseline.y < 0 {
            text.baseline.y = 0;
        }
    }
    let mut max_chars = u16::MAX;
    loop {
        text.p_str = ptr;
        text.baseline.y = text.baseline.y.wrapping_add(leading);
        let eol = get_line_within_width(text, &mut ptr, width, max_chars);
        max_chars = max_chars.wrapping_sub(text.char_count).saturating_sub(1);
        font_DrawTracedText(
            std::ptr::from_mut(text).cast(),
            comm.data.alien_text_fcolor,
            comm.data.alien_text_bcolor,
        );
        if eol != 0 || max_chars == 0 {
            break;
        }
    }
    text.p_str = ptr;
    SetContext(old_context);
    DrawStamp(std::ptr::from_mut(&mut stamp).cast());
    comm.display.last_baseline = text.baseline.y;
    UnbatchGraphics();
    text.baseline.y
}

pub fn clear_subtitles(comm: &mut Comm) {
    comm.display.clear_subtitles = true;
    comm.display.last_subtitle = null();
    comm.display.subtitle.p_str = null();
    comm.display.subtitle.char_count = 0;
}

/// # Safety
/// Track player and CommData must be initialized on the game thread.
pub unsafe fn check_subtitles(comm: &mut Comm) {
    comm.apply_phrase_events();
    let text = GetTrackSubtitle();
    let baseline = comm.data.alien_text_baseline;
    let align = comm.data.alien_text_align as i32;
    if text != comm.display.subtitle.p_str
        || comm.display.subtitle.baseline.x != baseline.x
        || comm.display.subtitle.baseline.y != baseline.y
        || comm.display.subtitle.align != align
    {
        comm.display.clear_subtitles = true;
        comm.display.subtitle.baseline = baseline;
        comm.display.subtitle.align = align;
        if comm.display.subtitle.p_str == text {
            crate::log_abi::log_message(
                crate::log_abi::LOG_WARNING,
                "Dialog text and location changed out of sync",
            );
        }
        comm.display.subtitle.p_str = text;
        comm.display.subtitle.char_count = if text.is_null() { 0 } else { u16::MAX };
    }
}

/// # Safety
/// Subtitle cache and current drawing context must be live on the game thread.
pub unsafe fn comm_redraw_subtitles(comm: &mut Comm, enabled: bool) {
    if !enabled {
        return;
    }
    if !comm.display.subtitle.p_str.is_null() {
        let mut text = comm.display.subtitle;
        draw_subtitle(comm, &mut text);
    }
}

/// # Safety
/// Called from the game thread, outside a COMM_STATE lock.
pub unsafe fn set_talking_finished(comm: &mut Comm, finished: i32) {
    super::ffi::rust_set_talking_finished(comm, finished);
}
pub fn setup_subtitle_text_from_comm_data(comm: &mut Comm) {
    comm.display.subtitle.baseline = comm.data.alien_text_baseline;
    comm.display.subtitle.align = comm.data.alien_text_align as i32;
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
/// # Safety
/// Game thread only.
pub unsafe fn clear_subtitles_pending(comm: &mut Comm) -> i32 {
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
        comm.data.alien_talk_desc.anim_flags &= !16;
    } else {
        comm.data.alien_talk_desc.anim_flags |= 16;
    }
}
pub fn set_comm_intro_mode(comm: &mut Comm, mode: u32, duration: u32) {
    super::ffi::set_comm_intro_mode(comm, mode, duration);
}

pub(crate) struct RuntimeState {
    pub(crate) phrase_buf: [c_char; 2048],
    pub(crate) window: CRect,
    pub(crate) anim_context: *mut c_void,
    pub(crate) text_cache_context: *mut crate::graphics::native_abi::context_desc,
    pub(crate) text_cache_frame: *mut c_void,
    pub(crate) clear_subtitles: bool,
    pub(crate) subtitle: CText,
    pub(crate) last_subtitle: *const c_char,
    pub(crate) last_baseline: i16,
}
impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            phrase_buf: [0; 2048],
            window: CRect {
                corner: CPoint { x: 7, y: 10 },
                width: 0,
                height: 0,
            },
            anim_context: null_mut(),
            text_cache_context: null_mut(),
            text_cache_frame: null_mut(),
            clear_subtitles: false,
            subtitle: CText::new(null(), 0),
            last_subtitle: null(),
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
        fn move_baseline(comm: &mut Comm) {
            comm.data.alien_text_baseline.y = 23;
        }
        let mut first = Comm::new();
        let second = Comm::new();
        first.data.alien_text_baseline.x = 17;
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
            assert_eq!(first.data.alien_talk_desc.anim_flags & 16, 16);
            enable_talking_anim(&mut first, 1);
            assert_eq!(first.data.alien_talk_desc.anim_flags & 16, 0);
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
            first.display.phrase_buf[0] = b'X' as c_char;
            set_anim_context(&mut first, context.cast());
            set_text_cache_context(&mut first, context.cast());
            set_text_cache_frame(&mut first, frame.cast());
            assert_eq!(anim_context(&mut first), context.cast());
            assert_eq!(first.display.text_cache_context, context);
            assert_eq!(first.display.text_cache_frame, frame.cast());
            assert!(second.display.anim_context.is_null());
            assert!(second.display.text_cache_context.is_null());
            assert!(second.display.text_cache_frame.is_null());
            assert_eq!(second.display.phrase_buf[0], 0);
            drop(first);
            assert_eq!(drawable_abi::DestroyDrawable(drawable), 1);
            assert_eq!(context_abi::DestroyContext(context), 1);
        }
    }
}
