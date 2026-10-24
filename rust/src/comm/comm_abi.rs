//! Remaining live dialogue API and subtitle cache, ported from comm.c.
//! Copyright Paul Reiche, Fred Ford, 1992-2002. GPL-2.0-or-later.

use super::locdata::{CPoint, CRect, CStamp, COMM_DATA};
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
#[allow(clashing_extern_declarations)]
extern "C" {
    static optSubtitles: i32;
}
use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

#[no_mangle]
pub static mut shared_phrase_buf: [c_char; 2048] = [0; 2048];
#[no_mangle]
pub static mut CommWndRect: CRect = CRect {
    corner: CPoint { x: 7, y: 10 },
    width: 0,
    height: 0,
};

static mut ANIM_CONTEXT: *mut c_void = null_mut();
static mut TEXT_CACHE_CONTEXT: *mut crate::graphics::native_abi::context_desc = null_mut();
static mut TEXT_CACHE_FRAME: *mut c_void = null_mut();
static mut CLEAR_SUBTITLES: bool = false;
static mut SUBTITLE: CText = CText::new(null(), 0);
static mut LAST_SUBTITLE: *const c_char = null();
static mut LAST_BASELINE: i16 = 0;

#[repr(C)]
struct ResponseEntry {
    reference: u16,
    text: CText,
    callback: Option<extern "C" fn(u16)>,
}
#[repr(C)]
pub(super) struct EncounterState {
    pub(super) input: Option<unsafe extern "C" fn(*mut EncounterState) -> i32>,
    initialized: u16,
    next_time: u32,
    count: u8,
    current: u8,
    top: u8,
    responses: [ResponseEntry; 8],
    phrase: [c_char; 1024],
}
static mut CURRENT_INPUT: *mut EncounterState = null_mut();

/// # Safety
/// Text and next must be writable; text must reference a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn getLineWithinWidth(
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

unsafe fn count_lines(text: &mut CText) -> i32 {
    let width = COMM_DATA.alien_text_width as i16;
    SetContextFont((COMM_DATA.alien_font).cast());
    let mut ptr = text.p_str;
    let mut count = 0;
    loop {
        count += 1;
        text.p_str = ptr;
        if getLineWithinWidth(text, &mut ptr, width, u16::MAX) != 0 {
            break;
        }
    }
    text.p_str = ptr;
    count
}

unsafe fn draw_subtitle(text: &mut CText) -> i16 {
    BatchGraphics();
    let mut stamp = CStamp {
        origin: CPoint { x: 0, y: 0 },
        frame: TEXT_CACHE_FRAME,
    };
    if LAST_SUBTITLE == text.p_str {
        DrawStamp(std::ptr::from_mut(&mut stamp).cast());
        UnbatchGraphics();
        return LAST_BASELINE;
    }
    let old_context = SetContext(TEXT_CACHE_CONTEXT);
    ClearDrawable();
    LAST_SUBTITLE = text.p_str;
    let width = COMM_DATA.alien_text_width as i16;
    SetContextFont((COMM_DATA.alien_font).cast());
    let mut leading = 0i16;
    GetContextFontLeading(&mut leading);
    let mut ptr = text.p_str;
    if COMM_DATA.alien_text_valign & 3 != 0 {
        let lines = count_lines(text);
        if COMM_DATA.alien_text_valign == 2 {
            text.baseline.y = (i32::from(text.baseline.y) - i32::from(leading) * lines) as i16;
        } else if COMM_DATA.alien_text_valign == 1 {
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
        let eol = getLineWithinWidth(text, &mut ptr, width, max_chars);
        max_chars = max_chars.wrapping_sub(text.char_count).saturating_sub(1);
        font_DrawTracedText(
            std::ptr::from_mut(text).cast(),
            COMM_DATA.alien_text_fcolor,
            COMM_DATA.alien_text_bcolor,
        );
        if eol != 0 || max_chars == 0 {
            break;
        }
    }
    text.p_str = ptr;
    SetContext(old_context);
    DrawStamp(std::ptr::from_mut(&mut stamp).cast());
    LAST_BASELINE = text.baseline.y;
    UnbatchGraphics();
    text.baseline.y
}

/// # Safety
/// Game thread only; invalidates the cached subtitle before the next redraw.
#[no_mangle]
pub unsafe extern "C" fn comm_ClearSubtitles() {
    CLEAR_SUBTITLES = true;
    LAST_SUBTITLE = null();
    SUBTITLE.p_str = null();
    SUBTITLE.char_count = 0;
}

/// # Safety
/// Track player and CommData must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn comm_CheckSubtitles() {
    let text = GetTrackSubtitle();
    let baseline = COMM_DATA.alien_text_baseline;
    let align = COMM_DATA.alien_text_align as i32;
    if text != SUBTITLE.p_str
        || SUBTITLE.baseline.x != baseline.x
        || SUBTITLE.baseline.y != baseline.y
        || SUBTITLE.align != align
    {
        CLEAR_SUBTITLES = true;
        SUBTITLE.baseline = baseline;
        SUBTITLE.align = align;
        if SUBTITLE.p_str == text {
            crate::log_abi::log_message(
                crate::log_abi::LOG_WARNING,
                "Dialog text and location changed out of sync",
            );
        }
        SUBTITLE.p_str = text;
        SUBTITLE.char_count = if text.is_null() { 0 } else { u16::MAX };
    }
}

/// # Safety
/// Subtitle cache and current drawing context must be live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn comm_RedrawSubtitles() {
    if optSubtitles == 0 {
        return;
    }
    if !SUBTITLE.p_str.is_null() {
        let mut text = SUBTITLE;
        draw_subtitle(&mut text);
    }
}

/// # Safety
/// Called from the game thread, outside a COMM_STATE lock.
#[no_mangle]
pub unsafe extern "C" fn c_SetTalkingFinished(finished: i32) {
    super::ffi::rust_set_talking_finished(finished);
}
/// # Safety
/// CommData must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn c_SetupSubtitleTextFromCommData() {
    SUBTITLE.baseline = COMM_DATA.alien_text_baseline;
    SUBTITLE.align = COMM_DATA.alien_text_align as i32;
}
/// # Safety
/// State must be null or a live C encounter state until cleared.
#[no_mangle]
pub unsafe extern "C" fn c_SetCurInputState(state: *mut c_void) {
    CURRENT_INPUT = state.cast();
}
/// # Safety
/// Game thread only; the registered encounter state must still be live.
#[no_mangle]
pub unsafe extern "C" fn c_ClearPhraseBuf() {
    if !CURRENT_INPUT.is_null() {
        (*CURRENT_INPUT).phrase[0] = 0;
    }
}
/// # Safety
/// Context must remain live while registered; game thread only.
#[no_mangle]
pub unsafe extern "C" fn c_SetAnimContext(context: *mut c_void) {
    ANIM_CONTEXT = context;
}
/// # Safety
/// Context must remain live while registered; game thread only.
#[no_mangle]
pub unsafe extern "C" fn c_SetTextCacheContext(context: *mut c_void) {
    TEXT_CACHE_CONTEXT = (context).cast();
}
/// # Safety
/// Frame must remain live while registered; game thread only.
#[no_mangle]
pub unsafe extern "C" fn c_SetTextCacheFrame(frame: *mut c_void) {
    TEXT_CACHE_FRAME = frame;
}
/// # Safety
/// Game thread only.
#[no_mangle]
pub unsafe extern "C" fn c_GetAnimContext() -> *mut c_void {
    ANIM_CONTEXT
}
/// # Safety
/// Game thread only.
#[no_mangle]
pub unsafe extern "C" fn c_GetClearSubtitles() -> i32 {
    i32::from(CLEAR_SUBTITLES)
}
/// # Safety
/// Game thread only.
#[no_mangle]
pub unsafe extern "C" fn c_ResetClearSubtitles() {
    CLEAR_SUBTITLES = false;
}

/// # Safety
/// Called from the game thread with an initialized communication system.
#[no_mangle]
pub unsafe extern "C" fn AlienTalkSegue(track: u16) {
    super::ffi::rust_alien_talk_segue(track.into());
}
/// # Safety
/// CommData must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn EnableTalkingAnim(enable: i32) {
    if enable != 0 {
        COMM_DATA.alien_talk_desc.anim_flags &= !16;
    } else {
        COMM_DATA.alien_talk_desc.anim_flags |= 16;
    }
}
#[no_mangle]
pub extern "C" fn SetCommIntroMode(mode: u32, duration: u32) {
    super::ffi::set_comm_intro_mode(mode, duration);
}
/// # Safety
/// Game thread only, with game state and resources initialized.
#[no_mangle]
pub unsafe extern "C" fn InitCommunication(conversation: u32) -> u16 {
    super::dispatch::init_communication(conversation)
}
/// # Safety
/// Game thread only, with game state and resources initialized.
#[no_mangle]
pub unsafe extern "C" fn RaceCommunication() {
    super::dispatch::rust_race_communication();
}
