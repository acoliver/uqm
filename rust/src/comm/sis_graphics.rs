//! SIS Communication Window Graphics
//!
//! Ported from `rust_comm.c` — the 5 remaining C bridge functions that
//! render text and UI elements in the SIS communication window during
//! alien encounters. These use the C graphics primitives directly via
//! `extern "C"` declarations and `#[repr(C)]` types matching the C ABI.
//!
//! # Ported functions
//!
//! - `draw_sis_com_window()` ← `c_DrawSISComWindow()`
//! - `feedback_player_phrase(text)` ← `c_FeedbackPlayerPhrase()`
//! - `refresh_responses(top, count, cur)` ← `c_RefreshResponses()`
//! - `select_conversation_summary()` ← `c_SelectConversationSummary()`
//! - `do_summary_page(state)` ← `do_summary_page()` (internal helper)
//!
//! @plan PLAN-20260314-COMM.P05b

// These functions are direct translations of C code that accesses
// mutable statics and uses field reassignment after Default initialization.
// The patterns are inherent to C ABI compatibility.
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::field_reassign_with_default)]
#![allow(static_mut_refs)]

use crate::comm::locdata::CPoint;

#[cfg(not(test))]
use crate::comm::locdata::{CColor, CRect};

// ============================================================================
// C ABI types matching gfxlib.h
// ============================================================================

/// C `TEXT` struct (gfxlib.h:226-231).
/// `{ POINT baseline; const UNICODE *pStr; TEXT_ALIGN align; COUNT CharCount; }`
///
/// TEXT_ALIGN is a C enum (int-sized). Layout: i16, i16, pad[4], ptr, i32, u16, pad[2].
#[repr(C)]
#[derive(Clone, Copy)]
struct CText {
    baseline: CPoint,
    _pad0: [u8; 4], // align pointer to 8 bytes
    p_str: *const std::ffi::c_char,
    align: std::ffi::c_int, // TEXT_ALIGN: 0=LEFT, 1=CENTER, 2=RIGHT
    char_count: u16,
    _pad1: [u8; 2], // align struct size to 8
}

impl Default for CText {
    fn default() -> Self {
        Self {
            baseline: CPoint::default(),
            _pad0: [0; 4],
            p_str: std::ptr::null(),
            align: 0,
            char_count: 0,
            _pad1: [0; 2],
        }
    }
}

/// C `STAMP` struct (gfxlib.h:161-164).
/// `{ POINT origin; FRAME frame; }`
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CStamp {
    origin: CPoint,
    frame: *mut std::ffi::c_void,
}

/// State for the conversation summary page loop (DoInput-compatible).
/// First field MUST be a function pointer for DoInput compatibility.
#[repr(C)]
struct SummaryLoopState {
    input_func: Option<unsafe extern "C" fn(*mut SummaryLoopState) -> std::ffi::c_int>,
    initialized: std::ffi::c_int,
    print_next: std::ffi::c_int,
    next_sub: *mut std::ffi::c_void, // SUBTITLE_REF
    left_over: *const std::ffi::c_char,
}

impl Default for SummaryLoopState {
    fn default() -> Self {
        Self {
            input_func: None,
            initialized: 0,
            print_next: 0,
            next_sub: std::ptr::null_mut(),
            left_over: std::ptr::null(),
        }
    }
}

// ============================================================================
// Constants (matching comm.h / sis.h / gamestr.h)
// ============================================================================
const WON_LAST_BATTLE: u8 = 5;
const SLIDER_Y: i16 = 107;
const SLIDER_HEIGHT: i16 = 15;
const SIS_SCREEN_WIDTH: i16 = 306; // 320 - 14
const SIS_SCREEN_HEIGHT: i16 = 227; // 240 - 13
#[cfg(not(test))]
const TEXT_X_OFFS: i16 = 7; // SIS_ORG_X
#[cfg(not(test))]
const PLAYER_TEXT_WIDTH: i16 = SIS_SCREEN_WIDTH - 8 - (TEXT_X_OFFS << 2);
#[cfg(not(test))]
const DELTA_Y_SUMMARY: i16 = 8;
#[cfg(not(test))]
const MAX_SUMM_ROWS: i16 = (SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT) / DELTA_Y_SUMMARY - 1;

#[cfg(not(test))]
const ALIGN_LEFT: std::ffi::c_int = 0;
#[cfg(not(test))]
const ALIGN_CENTER: std::ffi::c_int = 1;

// CHECK_ABORT = 0x4000
const CHECK_ABORT: u16 = 0x4000;

// Key constants (controls.h)
#[cfg(not(test))]
const KEY_MENU_SELECT: usize = 9;
#[cfg(not(test))]
const KEY_MENU_CANCEL: usize = 10;
#[cfg(not(test))]
const KEY_MENU_RIGHT: usize = 8;

// OPT_PC = 0x02 (from options.h)

// ============================================================================
// C extern declarations (not compiled in test builds)
// ============================================================================

#[cfg(not(test))]
mod c_bridge {
    use super::*;

    #[allow(clashing_extern_declarations)]
    extern "C" {
        // C globals
        #[allow(dead_code)]
        pub static mut GlobData: crate::comm::locdata::CGlobData;
        pub static mut SpaceContext: *mut std::ffi::c_void;
        pub static mut ActivityFrame: *mut std::ffi::c_void;
        pub static mut TinyFont: *mut std::ffi::c_void;

        // Activity accessor (replaces GlobData.Game_state.CurrentActivity)
        pub fn get_current_activity() -> u16;

        // Input state (controls.h)
        pub static mut PulsedInputState: CInputState;

        // Graphics context
        pub fn SetContext(ctx: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        pub fn SetContextForeGroundColor(color: CColor) -> CColor;
        pub fn SetContextFont(font: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        pub fn GetContextFontLeading(leading: *mut i16);

        // Graphics primitives
        pub fn font_DrawText(text: *mut CText);
        pub fn getLineWithinWidth(
            text: *mut CText,
            next: *mut *const std::ffi::c_char,
            width: i16,
            maxchars: u16,
        ) -> std::ffi::c_int;
        pub fn DrawFilledRectangle(rect: *mut CRect);
        pub fn DrawStamp(stamp: *mut CStamp);
        pub fn GetFrameRect(
            frame: *mut std::ffi::c_void,
            x: *mut std::ffi::c_int,
            y: *mut std::ffi::c_int,
            w: *mut std::ffi::c_int,
            h: *mut std::ffi::c_int,
        );

        // Batching
        pub fn BatchGraphics();
        pub fn UnbatchGraphics();

        // Font/resource management
        pub fn LoadFont(font_ref: *const std::ffi::c_char) -> *mut std::ffi::c_void;
        pub fn DestroyFont(font: *mut std::ffi::c_void) -> std::ffi::c_int;
        pub fn SetAbsFrameIndex(frame: *mut std::ffi::c_void, index: u16) -> *mut std::ffi::c_void;

        // Input
        pub fn DoInput(state: *mut std::ffi::c_void, reset: std::ffi::c_int);
        pub fn SleepThread(ticks: std::ffi::c_int);

        // Game strings — GAME_STRING(i) macro = GetStringAddress(SetAbsStringTableIndex(GameStrings, i))
        pub static mut GameStrings: *mut std::ffi::c_void;
        pub fn SetAbsStringTableIndex(
            table: *mut std::ffi::c_void,
            index: std::ffi::c_int,
        ) -> *mut std::ffi::c_void;
        pub fn GetStringAddress(s: *mut std::ffi::c_void) -> *const std::ffi::c_char;

        // Player font constant (ifontres.h) — PLAYER_FONT = "font.player"
        // Not a variable, it's a #define string

        // Colors — computed from #define macros in colors.h/gfxlib.h
        // MAKE_RGB15(r,g,b) = { CC5TO8(r), CC5TO8(g), CC5TO8(b), 0xFF }
        // BUILD_COLOR(col, idx) = col (palette index ignored)

        // Rust response text accessor
        pub fn rust_GetResponseText(
            index: std::ffi::c_int,
            buf: *mut std::ffi::c_char,
            buf_len: usize,
        ) -> std::ffi::c_int;

        // Trackplayer subtitle functions (trackplayer.c)
        pub fn c_GetFirstTrackSubtitle() -> *mut std::ffi::c_void;
        pub fn c_GetNextTrackSubtitle(last: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        pub fn c_GetTrackSubtitleText(sub: *mut std::ffi::c_void) -> *const std::ffi::c_char;
        pub fn comm_ClearSubtitles();
    }
}

/// C `INPUT_STATE` struct — just the menu array we need.
/// The real struct is larger but we only access `menu[16]`.
#[repr(C)]
#[allow(dead_code)]
struct CInputState {
    menu: [u8; 16],
}

// ============================================================================
// Last response window state (module-level)
// ============================================================================

#[cfg(not(test))]
static mut LAST_TOP_RESPONSE: u8 = 0;
#[cfg(not(test))]
static mut LAST_NUM_RESPONSES: u8 = 0;
#[cfg(not(test))]
static mut LAST_CUR_RESPONSE: u8 = 0;

// ============================================================================
// do_summary_page — DoInput callback for summary page scrolling
// ============================================================================

#[cfg(not(test))]
unsafe extern "C" fn do_summary_page_cb(state: *mut SummaryLoopState) -> std::ffi::c_int {
    let pss = &mut *state;

    if pss.initialized == 0 {
        pss.print_next = 1;
        pss.next_sub = c_bridge::c_GetFirstTrackSubtitle();
        pss.left_over = std::ptr::null();
        pss.initialized = 1;
        pss.input_func = Some(do_summary_page_cb);
        c_bridge::DoInput(state as *mut std::ffi::c_void, 0);
        return 1;
    }

    let activity = c_bridge::get_current_activity();
    if (activity & CHECK_ABORT) != 0 {
        return 0;
    }

    let menu = &c_bridge::PulsedInputState.menu;
    if menu[KEY_MENU_SELECT] != 0 || menu[KEY_MENU_CANCEL] != 0 || menu[KEY_MENU_RIGHT] != 0 {
        if !pss.next_sub.is_null() {
            pss.print_next = 1;
        } else {
            return 0;
        }
    } else if pss.print_next != 0 {
        draw_summary_page_contents(pss);
        pss.print_next = 0;
    } else {
        c_bridge::SleepThread(ONE_SECOND / 20);
    }

    1
}

#[cfg(not(test))]
unsafe fn draw_summary_page_contents(pss: &mut SummaryLoopState) {
    let old_ctx = c_bridge::SetContext(c_bridge::SpaceContext);

    let mut rect = CRect {
        corner: CPoint {
            x: 0,
            y: SLIDER_Y + SLIDER_HEIGHT,
        },
        width: SIS_SCREEN_WIDTH,
        height: SIS_SCREEN_HEIGHT - (SLIDER_Y + SLIDER_HEIGHT),
    };
    c_bridge::SetContextForeGroundColor(COMM_HISTORY_BACKGROUND_COLOR);
    c_bridge::DrawFilledRectangle(&mut rect);

    c_bridge::SetContextForeGroundColor(COMM_HISTORY_TEXT_COLOR);
    c_bridge::SetContextFont(c_bridge::TinyFont);

    let tw = rect.width - 2 - 2;
    let mut t = CText::default();
    t.baseline.x = 2;
    t.align = ALIGN_LEFT;
    t.baseline.y = SLIDER_Y + SLIDER_HEIGHT + DELTA_Y_SUMMARY;

    let mut row: i16 = 0;
    while row < MAX_SUMM_ROWS && !pss.next_sub.is_null() {
        let mut next: *const std::ffi::c_char = std::ptr::null();

        if !pss.left_over.is_null() {
            t.p_str = pss.left_over;
            pss.left_over = std::ptr::null();
        } else {
            t.p_str = c_bridge::c_GetTrackSubtitleText(pss.next_sub);
            if t.p_str.is_null() {
                pss.next_sub = c_bridge::c_GetNextTrackSubtitle(pss.next_sub);
                continue;
            }
        }

        t.char_count = u16::MAX;
        while row < MAX_SUMM_ROWS
            && c_bridge::getLineWithinWidth(&mut t, &mut next, tw, u16::MAX) == 0
        {
            c_bridge::font_DrawText(&mut t);
            t.baseline.y += DELTA_Y_SUMMARY;
            row += 1;
            t.p_str = next;
            t.char_count = u16::MAX;
        }

        if row >= MAX_SUMM_ROWS {
            pss.left_over = next;
            break;
        }

        c_bridge::font_DrawText(&mut t);
        t.baseline.y += DELTA_Y_SUMMARY;
        row += 1;
        pss.next_sub = c_bridge::c_GetNextTrackSubtitle(pss.next_sub);
    }

    if row >= MAX_SUMM_ROWS && (!pss.next_sub.is_null() || !pss.left_over.is_null()) {
        let mut buffer = [0u8; 80];
        let bullet = STR_MIDDLE_DOT_BYTES.as_ptr() as *const std::ffi::c_char;
        let more_str = game_string(FEEDBACK_STRING_BASE + 1);
        let more_str_c = STR_MIDDLE_DOT_BYTES.as_ptr() as *const std::ffi::c_char;

        // Build "·MORE·" string
        let blen = copy_cstr(bullet, &mut buffer, 0);
        let mlen = copy_cstr(more_str, &mut buffer, blen);
        let _ = copy_cstr(more_str_c, &mut buffer, blen + mlen);

        let mut mt = CText::default();
        mt.baseline.x = SIS_SCREEN_WIDTH >> 1;
        mt.baseline.y = t.baseline.y;
        mt.align = ALIGN_CENTER;
        mt.p_str = buffer.as_ptr() as *const std::ffi::c_char;
        c_bridge::SetContextForeGroundColor(COMM_MORE_TEXT_COLOR);
        c_bridge::font_DrawText(&mut mt);
    }

    c_bridge::SetContext(old_ctx);
}

/// Copy a C string into a buffer at offset, returning new offset.
#[cfg(not(test))]
unsafe fn copy_cstr(src: *const std::ffi::c_char, buf: &mut [u8; 80], offset: usize) -> usize {
    if src.is_null() || offset >= buf.len() {
        return offset;
    }
    let mut i = 0;
    while offset + i < buf.len() - 1 {
        let ch = *src.add(i) as u8;
        if ch == 0 {
            break;
        }
        buf[offset + i] = ch;
        i += 1;
    }
    offset + i
}

const ONE_SECOND: std::ffi::c_int = 840;

/// Convert 5-bit color component to 8-bit (matches C CC5TO8 macro).
#[cfg(not(test))]
const fn cc5to8(c: u8) -> u8 {
    (c << 3) | (c >> 2)
}

/// Build a Color from RGB15 components (matches C MAKE_RGB15).
#[cfg(not(test))]
const fn make_rgb15(r: u8, g: u8, b: u8) -> CColor {
    CColor {
        r: cc5to8(r),
        g: cc5to8(g),
        b: cc5to8(b),
        a: 0xFF,
    }
}

#[cfg(not(test))]
const COMM_PLAYER_TEXT_NORMAL_COLOR: CColor = make_rgb15(0x00, 0x14, 0x14);
#[cfg(not(test))]
const COMM_PLAYER_TEXT_HIGHLIGHT_COLOR: CColor = make_rgb15(0x1A, 0x1A, 0x1A);
#[cfg(not(test))]
const COMM_PLAYER_BACKGROUND_COLOR: CColor = make_rgb15(0x00, 0x00, 0x14);
#[cfg(not(test))]
const COMM_RESPONSE_INTRO_TEXT_COLOR: CColor = make_rgb15(0x0A, 0x0C, 0x1F);
#[cfg(not(test))]
const COMM_FEEDBACK_TEXT_COLOR: CColor = make_rgb15(0x12, 0x14, 0x4F);
#[cfg(not(test))]
const COMM_HISTORY_BACKGROUND_COLOR: CColor = make_rgb15(0x00, 0x05, 0x00);
#[cfg(not(test))]
const COMM_HISTORY_TEXT_COLOR: CColor = make_rgb15(0x00, 0x10, 0x00);
#[cfg(not(test))]
const COMM_MORE_TEXT_COLOR: CColor = make_rgb15(0x00, 0x17, 0x00);

/// PLAYER_FONT #define: "font.player"
#[cfg(not(test))]
const PLAYER_FONT_STR: &[u8] = b"font.player\0";

/// STR_BULLET #define: "\xE2\x80\xA2" (•)
#[cfg(not(test))]
const STR_BULLET_BYTES: &[u8] = b"\xE2\x80\xA2\0";

/// STR_MIDDLE_DOT #define: "\xC2\xB7" (·)
#[cfg(not(test))]
const STR_MIDDLE_DOT_BYTES: &[u8] = b"\xC2\xB7\0";

/// FEEDBACK_STRING_BASE from gamestr.h
#[cfg(not(test))]
const FEEDBACK_STRING_BASE: i32 = 0x0100;

/// Inline GAME_STRING(i) macro: GetStringAddress(SetAbsStringTableIndex(GameStrings, i))
#[cfg(not(test))]
unsafe fn game_string(id: i32) -> *const std::ffi::c_char {
    let table = c_bridge::SetAbsStringTableIndex(c_bridge::GameStrings, id as std::ffi::c_int);
    c_bridge::GetStringAddress(table)
}

// ============================================================================
// Ported functions
// ============================================================================

/// Draw the SIS communication window background.
/// Ported from `c_DrawSISComWindow()` in rust_comm.c.
#[cfg(not(test))]
pub unsafe fn draw_sis_com_window() {
    let activity = c_bridge::get_current_activity();
    if activity as u8 != WON_LAST_BATTLE {
        let old_ctx = c_bridge::SetContext(c_bridge::SpaceContext);
        let mut rect = CRect {
            corner: CPoint {
                x: 0,
                y: SLIDER_Y + SLIDER_HEIGHT,
            },
            width: SIS_SCREEN_WIDTH,
            height: SIS_SCREEN_HEIGHT - (SLIDER_Y + SLIDER_HEIGHT),
        };
        c_bridge::SetContextForeGroundColor(COMM_PLAYER_BACKGROUND_COLOR);
        c_bridge::DrawFilledRectangle(&mut rect);
        c_bridge::SetContext(old_ctx);
    }
}

/// Render the player's selected response text in the SIS comm window.
/// Ported from `c_FeedbackPlayerPhrase()` in rust_comm.c.
///
/// # Safety
/// Caller must ensure `text` is a valid null-terminated C string or null,
/// and that the graphics subsystem is initialized.
#[cfg(not(test))]
pub unsafe fn feedback_player_phrase(text: *const std::ffi::c_char) {
    let old_ctx = c_bridge::SetContext(c_bridge::SpaceContext);

    c_bridge::BatchGraphics();
    draw_sis_com_window();

    if !text.is_null() && *text != 0 {
        let player_font = c_bridge::LoadFont(PLAYER_FONT_STR.as_ptr() as *const std::ffi::c_char);
        let old_font = c_bridge::SetContextFont(player_font);

        let mut ct = CText::default();
        ct.baseline.x = SIS_SCREEN_WIDTH >> 1;
        ct.baseline.y = SLIDER_Y + SLIDER_HEIGHT + 13;
        ct.align = ALIGN_CENTER;
        ct.char_count = u16::MAX;
        ct.p_str = game_string(FEEDBACK_STRING_BASE);
        c_bridge::SetContextForeGroundColor(COMM_RESPONSE_INTRO_TEXT_COLOR);
        c_bridge::font_DrawText(&mut ct);

        ct.baseline.y += 16;
        ct.align = ALIGN_CENTER;
        ct.p_str = text;
        c_bridge::SetContextForeGroundColor(COMM_FEEDBACK_TEXT_COLOR);

        let mut leading: i16 = 0;
        c_bridge::GetContextFontLeading(&mut leading);

        let mut p_str = text;
        let mut maxchars: u16 = u16::MAX;

        loop {
            let mut next: *const std::ffi::c_char = std::ptr::null();
            ct.p_str = p_str;
            ct.baseline.y += leading;
            let eol = c_bridge::getLineWithinWidth(&mut ct, &mut next, PLAYER_TEXT_WIDTH, maxchars);
            maxchars = maxchars.wrapping_sub(ct.char_count);
            maxchars = maxchars.saturating_sub(1);
            p_str = next;
            if ct.baseline.y < SIS_SCREEN_HEIGHT {
                c_bridge::font_DrawText(&mut ct);
            }
            if eol != 0 || maxchars == 0 {
                break;
            }
        }

        c_bridge::SetContextFont(old_font);
        c_bridge::DestroyFont(player_font);
    }

    c_bridge::UnbatchGraphics();
    c_bridge::SetContext(old_ctx);
}

/// Render the response list in the SIS comm window.
/// Ported from `c_RefreshResponses()` in rust_comm.c.
#[cfg(not(test))]
pub unsafe fn refresh_responses(top: u8, num_responses: u8, cur_response: u8) {
    LAST_TOP_RESPONSE = top;
    LAST_NUM_RESPONSES = num_responses;
    LAST_CUR_RESPONSE = cur_response;

    let old_ctx = c_bridge::SetContext(c_bridge::SpaceContext);
    let player_font = c_bridge::LoadFont(PLAYER_FONT_STR.as_ptr() as *const std::ffi::c_char);
    let old_font = c_bridge::SetContextFont(player_font);

    let mut leading: i16 = 0;
    c_bridge::GetContextFontLeading(&mut leading);

    c_bridge::BatchGraphics();
    draw_sis_com_window();

    let mut y: i16 = SLIDER_Y + SLIDER_HEIGHT + 1;
    let mut response = top;
    let mut text_buf = [0u8; 1024];

    while response < num_responses {
        if c_bridge::rust_GetResponseText(
            response as std::ffi::c_int,
            text_buf.as_mut_ptr() as *mut std::ffi::c_char,
            text_buf.len(),
        ) == 0
        {
            response += 1;
            continue;
        }

        let mut rt = CText::default();
        rt.p_str = text_buf.as_ptr() as *const std::ffi::c_char;
        rt.char_count = u16::MAX;
        rt.baseline.x = TEXT_X_OFFS + 8;
        rt.baseline.y = y + leading;
        rt.align = ALIGN_LEFT;

        if response == cur_response {
            c_bridge::SetContextForeGroundColor(COMM_PLAYER_TEXT_HIGHLIGHT_COLOR);
        } else {
            c_bridge::SetContextForeGroundColor(COMM_PLAYER_TEXT_NORMAL_COLOR);
        }

        let mut bullet = rt;
        bullet.baseline.x -= 8;
        bullet.p_str = STR_BULLET_BYTES.as_ptr() as *const std::ffi::c_char;
        c_bridge::font_DrawText(&mut bullet);

        y = draw_player_text_wrapped(&mut rt, leading);

        response += 1;
    }

    // Scroll indicator
    let mut stamp = CStamp::default();
    stamp.frame = std::ptr::null_mut();

    if top != 0 {
        stamp.origin.y = SLIDER_Y + SLIDER_HEIGHT + 1;
        stamp.frame = c_bridge::SetAbsFrameIndex(c_bridge::ActivityFrame, 6);
    } else if y > SIS_SCREEN_HEIGHT {
        stamp.origin.y = SIS_SCREEN_HEIGHT - 2;
        stamp.frame = c_bridge::SetAbsFrameIndex(c_bridge::ActivityFrame, 7);
    }

    if !stamp.frame.is_null() {
        let mut rw: std::ffi::c_int = 0;
        let mut rh: std::ffi::c_int = 0;
        let mut rx: std::ffi::c_int = 0;
        let mut ry: std::ffi::c_int = 0;
        c_bridge::GetFrameRect(stamp.frame, &mut rx, &mut ry, &mut rw, &mut rh);
        stamp.origin.x = SIS_SCREEN_WIDTH - rw as i16 - 1;
        c_bridge::DrawStamp(&mut stamp);
    }

    c_bridge::UnbatchGraphics();
    c_bridge::SetContextFont(old_font);
    c_bridge::DestroyFont(player_font);
    c_bridge::SetContext(old_ctx);
}

/// Word-wrap and draw player text, returning the final baseline.y.
#[cfg(not(test))]
unsafe fn draw_player_text_wrapped(text: &mut CText, leading: i16) -> i16 {
    let mut p_str = text.p_str;
    let mut maxchars: u16 = u16::MAX;

    text.baseline.y -= leading;

    loop {
        let mut next: *const std::ffi::c_char = std::ptr::null();
        text.p_str = p_str;
        text.baseline.y += leading;
        let eol = c_bridge::getLineWithinWidth(text, &mut next, PLAYER_TEXT_WIDTH, maxchars);
        maxchars = maxchars.wrapping_sub(text.char_count);
        maxchars = maxchars.saturating_sub(1);
        p_str = next;

        if text.baseline.y < SIS_SCREEN_HEIGHT {
            c_bridge::font_DrawText(text);
        }
        if eol != 0 || maxchars == 0 {
            break;
        }
    }

    text.baseline.y
}

/// Show conversation history summary page.
/// Ported from `c_SelectConversationSummary()` in rust_comm.c.
#[cfg(not(test))]
pub unsafe fn select_conversation_summary() {
    let mut text_buf = [0u8; 1024];

    if LAST_NUM_RESPONSES > 0
        && c_bridge::rust_GetResponseText(
            LAST_CUR_RESPONSE as std::ffi::c_int,
            text_buf.as_mut_ptr() as *mut std::ffi::c_char,
            text_buf.len(),
        ) != 0
    {
        feedback_player_phrase(text_buf.as_ptr() as *const std::ffi::c_char);
    }

    let mut state = SummaryLoopState::default();
    state.initialized = 0;
    do_summary_page(&mut state);

    if LAST_NUM_RESPONSES > 0 {
        refresh_responses(LAST_TOP_RESPONSE, LAST_NUM_RESPONSES, LAST_CUR_RESPONSE);
    }

    c_bridge::comm_ClearSubtitles();
}

#[cfg(not(test))]
unsafe fn do_summary_page(state: &mut SummaryLoopState) {
    state.print_next = 1;
    state.next_sub = c_bridge::c_GetFirstTrackSubtitle();
    state.left_over = std::ptr::null();
    state.initialized = 1;
    state.input_func = Some(do_summary_page_cb);
    c_bridge::DoInput(state as *mut SummaryLoopState as *mut std::ffi::c_void, 0);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctext_layout_matches_c() {
        // C TEXT struct: POINT(4) + pad(4) + ptr(8) + int(4) + u16(2) + pad(2) = 24 bytes
        assert_eq!(std::mem::size_of::<CText>(), 24);
    }

    #[test]
    fn cstamp_layout_matches_c() {
        // C STAMP struct: POINT(4) + pad(4) + ptr(8) = 16 bytes
        assert_eq!(std::mem::size_of::<CStamp>(), 16);
    }

    #[test]
    fn summary_loop_state_has_function_pointer_first() {
        // First field must be a function pointer for DoInput compatibility
        let offset = std::mem::offset_of!(SummaryLoopState, input_func);
        assert_eq!(offset, 0);
    }

    #[test]
    fn constants_match_c() {
        assert_eq!(WON_LAST_BATTLE, 5);
        assert_eq!(SLIDER_Y, 107);
        assert_eq!(SLIDER_HEIGHT, 15);
        assert_eq!(SIS_SCREEN_WIDTH, 306);
        assert_eq!(SIS_SCREEN_HEIGHT, 227);
        assert_eq!(CHECK_ABORT, 0x4000);
        assert_eq!(ONE_SECOND, 840);
    }
}
