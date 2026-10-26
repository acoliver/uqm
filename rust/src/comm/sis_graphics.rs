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

use crate::comm::locdata::{CColor, CPoint, CRect};

// ============================================================================
// C ABI types matching gfxlib.h
// ============================================================================

/// C `TEXT` struct (gfxlib.h:226-231).
/// `{ POINT baseline; const UNICODE *pStr; TEXT_ALIGN align; COUNT CharCount; }`
///
/// TEXT_ALIGN is a C enum (int-sized). Layout: i16, i16, pad[4], ptr, i32, u16, pad[2].
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct CText {
    pub(crate) baseline: CPoint,
    _pad0: [u8; 4], // align pointer to 8 bytes
    pub(crate) p_str: *const std::ffi::c_char,
    pub(crate) align: std::ffi::c_int, // TEXT_ALIGN: 0=LEFT, 1=CENTER, 2=RIGHT
    pub(crate) char_count: u16,
    _pad1: [u8; 2], // align struct size to 8
}

crate::c_bindings::assert_c_layout!(
    CText,
    crate::c_bindings::global_state::TEXT {
        baseline: baseline,
        p_str: pStr,
        align: align,
        char_count: CharCount,
    }
);

impl CText {
    pub(crate) const fn new(p_str: *const std::ffi::c_char, char_count: u16) -> Self {
        Self {
            baseline: CPoint { x: 0, y: 0 },
            _pad0: [0; 4],
            p_str,
            align: 0,
            char_count,
            _pad1: [0; 2],
        }
    }
}
impl Default for CText {
    fn default() -> Self {
        Self::new(std::ptr::null(), 0)
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

unsafe fn scroll_indicator_stamp(
    frame: *mut crate::graphics::native_abi::frame_desc,
    origin_y: i16,
) -> Option<CStamp> {
    if frame.is_null() {
        return None;
    }
    let mut rect = std::mem::zeroed();
    crate::graphics::drawable_abi::GetFrameRect(frame, &mut rect);
    Some(CStamp {
        origin: CPoint {
            x: SIS_SCREEN_WIDTH - rect.extent.width - 1,
            y: origin_y,
        },
        frame: frame.cast(),
    })
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
const SIS_SCREEN_WIDTH: i16 = 242; // 320 - STATUS_WIDTH(64) - 14
const SIS_SCREEN_HEIGHT: i16 = 227; // 240 - 13
const TEXT_X_OFFS: i16 = 1; // comm.h
const PLAYER_TEXT_WIDTH: i16 = SIS_SCREEN_WIDTH - 8 - (TEXT_X_OFFS << 2);
#[cfg(not(test))]
const DELTA_Y_SUMMARY: i16 = 8;
#[cfg(not(test))]
const MAX_SUMM_ROWS: i16 = (SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT) / DELTA_Y_SUMMARY - 1;

const ALIGN_LEFT: std::ffi::c_int = 0;
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
    pub use crate::comm::ffi::rust_get_response_text;

    pub use crate::comm::comm_abi::comm_ClearSubtitles;

    pub use crate::comm::comm_abi::getLineWithinWidth;

    pub use crate::graphics::context_abi::SetContext;

    pub use crate::graphics::context_abi::SetContextForeGroundColor;

    pub use crate::graphics::font_abi::font_DrawText;

    pub use crate::graphics::font_abi::GetContextFontLeading;

    pub use crate::graphics::font_abi::SetContextFont;

    pub use crate::graphics::frame_abi::DrawFilledRectangle;

    pub use crate::graphics::frame_abi::DrawStamp;

    pub use crate::graphics::pixmap_abi::SetAbsFrameIndex;

    pub use crate::mainloop::input_abi::live::DoInput;
    pub use crate::sound::heart_ffi::get_first_track_subtitle as GetFirstTrackSubtitle;
    pub use crate::sound::heart_ffi::get_next_track_subtitle as GetNextTrackSubtitle;
    pub use crate::sound::heart_ffi::get_track_subtitle_text as GetTrackSubtitleText;

    pub use crate::strings::string_table::GetStringAddress;

    pub use crate::strings::string_table::SetAbsStringTableIndex;

    extern "C" {
        #[allow(dead_code)]
        pub static mut GlobData: crate::ships::native::abi::GLOBDATA;
        pub static mut SpaceContext: *mut crate::graphics::native_abi::context_desc;
        pub static mut ActivityFrame: *mut crate::graphics::native_abi::frame_desc;
        pub static mut TinyFont: *mut crate::graphics::native_abi::font_desc;
        pub static mut PulsedInputState:
            crate::c_bindings::controller_input::_controller_input_state;
        pub static mut GameStrings: *mut crate::strings::string_table::StringTableEntry;
    }
    pub use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};
    pub use crate::threading::pacing::linked::sleep_thread as SleepThread;
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
        pss.next_sub = c_bridge::GetFirstTrackSubtitle();
        pss.left_over = std::ptr::null();
        pss.initialized = 1;
        pss.input_func = Some(do_summary_page_cb);
        c_bridge::DoInput(state as *mut std::ffi::c_void, 0);
        return 1;
    }

    let activity = crate::mainloop::ffi::get_current_activity().0;
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
    c_bridge::DrawFilledRectangle(std::ptr::from_mut(&mut rect).cast());

    c_bridge::SetContextForeGroundColor(COMM_HISTORY_TEXT_COLOR);
    c_bridge::SetContextFont((c_bridge::TinyFont).cast());

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
            t.p_str = c_bridge::GetTrackSubtitleText(pss.next_sub);
            if t.p_str.is_null() {
                pss.next_sub = c_bridge::GetNextTrackSubtitle(pss.next_sub);
                continue;
            }
        }

        t.char_count = u16::MAX;
        while row < MAX_SUMM_ROWS
            && c_bridge::getLineWithinWidth(&mut t, &mut next, tw, u16::MAX) == 0
        {
            c_bridge::font_DrawText(std::ptr::from_mut(&mut t).cast());
            t.baseline.y += DELTA_Y_SUMMARY;
            row += 1;
            t.p_str = next;
            t.char_count = u16::MAX;
        }

        if row >= MAX_SUMM_ROWS {
            pss.left_over = next;
            break;
        }

        c_bridge::font_DrawText(std::ptr::from_mut(&mut t).cast());
        t.baseline.y += DELTA_Y_SUMMARY;
        row += 1;
        pss.next_sub = c_bridge::GetNextTrackSubtitle(pss.next_sub);
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
        c_bridge::font_DrawText(std::ptr::from_mut(&mut mt).cast());
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
        let ch = (*src.add(i)).to_ne_bytes()[0];
        if ch == 0 {
            break;
        }
        buf[offset + i] = ch;
        i += 1;
    }
    offset + i
}

const ONE_SECOND: u32 = 840;

/// Convert 5-bit color component to 8-bit (matches C CC5TO8 macro).
const fn cc5to8(c: u8) -> u8 {
    (c << 3) | (c >> 2)
}

/// Build a Color from RGB15 components (matches C MAKE_RGB15).
const fn make_rgb15(r: u8, g: u8, b: u8) -> CColor {
    CColor {
        r: cc5to8(r),
        g: cc5to8(g),
        b: cc5to8(b),
        a: 0xFF,
    }
}

const COMM_PLAYER_TEXT_NORMAL_COLOR: CColor = make_rgb15(0x00, 0x14, 0x14);
const COMM_PLAYER_TEXT_HIGHLIGHT_COLOR: CColor = make_rgb15(0x1A, 0x1A, 0x1A);
const COMM_PLAYER_BACKGROUND_COLOR: CColor = make_rgb15(0x00, 0x00, 0x14);
const COMM_RESPONSE_INTRO_TEXT_COLOR: CColor = make_rgb15(0x0A, 0x0C, 0x1F);
const COMM_FEEDBACK_TEXT_COLOR: CColor = make_rgb15(0x12, 0x14, 0x4F);
#[cfg(not(test))]
const COMM_HISTORY_BACKGROUND_COLOR: CColor = make_rgb15(0x00, 0x05, 0x00);
#[cfg(not(test))]
const COMM_HISTORY_TEXT_COLOR: CColor = make_rgb15(0x00, 0x10, 0x00);
#[cfg(not(test))]
const COMM_MORE_TEXT_COLOR: CColor = make_rgb15(0x00, 0x17, 0x00);

/// STR_BULLET #define: "\xE2\x80\xA2" (•)
const STR_BULLET_BYTES: &[u8] = b"\xE2\x80\xA2\0";

/// STR_MIDDLE_DOT #define: "\xC2\xB7" (·)
#[cfg(not(test))]
const STR_MIDDLE_DOT_BYTES: &[u8] = b"\xC2\xB7\0";

/// FEEDBACK_STRING_BASE from gamestr.h
const FEEDBACK_STRING_BASE: i32 = 0x0100;

/// Inline GAME_STRING(i) macro: GetStringAddress(SetAbsStringTableIndex(GameStrings, i))
#[cfg(not(test))]
unsafe fn game_string(id: i32) -> *const std::ffi::c_char {
    let table = c_bridge::SetAbsStringTableIndex((c_bridge::GameStrings).cast(), id as u16);
    c_bridge::GetStringAddress(table)
}

// ============================================================================
// Drawing surface for the SIS communication window
// ============================================================================

/// The graphics operations the communication window renderers use.
///
/// Text drawn through [`CommWindowGfx::draw_text`] is queued: the draw command
/// keeps pointers into the context font until the draw queue is consumed, so a
/// font must stay alive until then.
pub(crate) trait CommWindowGfx {
    fn current_activity(&self) -> u16;
    fn space_context(&self) -> *mut std::ffi::c_void;
    fn set_context(&mut self, ctx: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn set_context_font(&mut self, font: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn set_foreground_color(&mut self, color: CColor);
    fn font_leading(&mut self) -> i16;
    fn batch(&mut self);
    fn unbatch(&mut self);
    fn fill_rect(&mut self, rect: &mut CRect);
    fn draw_text(&mut self, text: &mut CText);
    fn line_within_width(
        &mut self,
        text: &mut CText,
        next: &mut *const std::ffi::c_char,
        width: i16,
        max_chars: u16,
    ) -> std::ffi::c_int;
    fn game_string(&mut self, id: i32) -> *const std::ffi::c_char;
    /// Copies response `index` into `buf`; false when there is no such text.
    fn response_text(&mut self, index: u8, buf: &mut [u8]) -> bool;
    /// Draws activity frame `frame_index` at the right edge, `origin_y` down.
    fn draw_scroll_indicator(&mut self, frame_index: u16, origin_y: i16);
    /// The player font of the running conversation. It is owned by the
    /// conversation, not by the renderer: queued draws still read it after
    /// the renderer returns.
    fn conversation_player_font(&mut self) -> *mut std::ffi::c_void;
}

// ============================================================================
// Ported functions
// ============================================================================

/// Draw the SIS communication window background.
/// Ported from `c_DrawSISComWindow()` in rust_comm.c.
pub(crate) fn draw_sis_com_window_with<G: CommWindowGfx>(gfx: &mut G) {
    let activity = gfx.current_activity();
    if activity as u8 != WON_LAST_BATTLE {
        let space = gfx.space_context();
        let old_ctx = gfx.set_context(space);
        let mut rect = CRect {
            corner: CPoint {
                x: 0,
                y: SLIDER_Y + SLIDER_HEIGHT,
            },
            width: SIS_SCREEN_WIDTH,
            height: SIS_SCREEN_HEIGHT - (SLIDER_Y + SLIDER_HEIGHT),
        };
        gfx.set_foreground_color(COMM_PLAYER_BACKGROUND_COLOR);
        gfx.fill_rect(&mut rect);
        gfx.set_context(old_ctx);
    }
}

/// Render the player's selected response text in the SIS comm window.
/// Ported from `c_FeedbackPlayerPhrase()` in rust_comm.c.
///
/// # Safety
/// `text` must be a valid null-terminated C string or null.
pub(crate) unsafe fn feedback_player_phrase_with<G: CommWindowGfx>(
    gfx: &mut G,
    text: *const std::ffi::c_char,
) {
    let space = gfx.space_context();
    let old_ctx = gfx.set_context(space);

    gfx.batch();
    draw_sis_com_window_with(gfx);

    if !text.is_null() && *text != 0 {
        let player_font = gfx.conversation_player_font();
        let old_font = gfx.set_context_font(player_font);

        let mut ct = CText::default();
        ct.baseline.x = SIS_SCREEN_WIDTH >> 1;
        ct.baseline.y = SLIDER_Y + SLIDER_HEIGHT + 13;
        ct.align = ALIGN_CENTER;
        ct.char_count = u16::MAX;
        ct.p_str = gfx.game_string(FEEDBACK_STRING_BASE);
        gfx.set_foreground_color(COMM_RESPONSE_INTRO_TEXT_COLOR);
        gfx.draw_text(&mut ct);

        ct.baseline.y += 16;
        ct.align = ALIGN_CENTER;
        ct.p_str = text;
        gfx.set_foreground_color(COMM_FEEDBACK_TEXT_COLOR);

        let leading = gfx.font_leading();

        let mut p_str = text;
        let mut maxchars: u16 = u16::MAX;

        loop {
            let mut next: *const std::ffi::c_char = std::ptr::null();
            ct.p_str = p_str;
            ct.baseline.y += leading;
            let eol = gfx.line_within_width(&mut ct, &mut next, PLAYER_TEXT_WIDTH, maxchars);
            maxchars = maxchars.wrapping_sub(ct.char_count);
            maxchars = maxchars.saturating_sub(1);
            p_str = next;
            if ct.baseline.y < SIS_SCREEN_HEIGHT {
                gfx.draw_text(&mut ct);
            }
            if eol != 0 || maxchars == 0 {
                break;
            }
        }

        gfx.set_context_font(old_font);
    }

    gfx.unbatch();
    gfx.set_context(old_ctx);
}

/// Render the response list in the SIS comm window.
/// Ported from `c_RefreshResponses()` in rust_comm.c.
pub(crate) fn refresh_responses_with<G: CommWindowGfx>(
    gfx: &mut G,
    top: u8,
    num_responses: u8,
    cur_response: u8,
) {
    let space = gfx.space_context();
    let old_ctx = gfx.set_context(space);
    let player_font = gfx.conversation_player_font();
    let old_font = gfx.set_context_font(player_font);

    let leading = gfx.font_leading();

    gfx.batch();
    draw_sis_com_window_with(gfx);

    let mut y: i16 = SLIDER_Y + SLIDER_HEIGHT + 1;
    let mut response = top;
    let mut text_buf = [0u8; 1024];

    while response < num_responses {
        if !gfx.response_text(response, &mut text_buf) {
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
            gfx.set_foreground_color(COMM_PLAYER_TEXT_HIGHLIGHT_COLOR);
        } else {
            gfx.set_foreground_color(COMM_PLAYER_TEXT_NORMAL_COLOR);
        }

        let mut bullet = rt;
        bullet.baseline.x -= 8;
        bullet.p_str = STR_BULLET_BYTES.as_ptr() as *const std::ffi::c_char;
        gfx.draw_text(&mut bullet);

        y = draw_player_text_wrapped(gfx, &mut rt, leading);

        response += 1;
    }

    // Scroll indicator
    if top != 0 {
        gfx.draw_scroll_indicator(6, SLIDER_Y + SLIDER_HEIGHT + 1);
    } else if y > SIS_SCREEN_HEIGHT {
        gfx.draw_scroll_indicator(7, SIS_SCREEN_HEIGHT - 2);
    }

    gfx.unbatch();
    gfx.set_context_font(old_font);
    gfx.set_context(old_ctx);
}

/// Word-wrap and draw player text, returning the final baseline.y.
fn draw_player_text_wrapped<G: CommWindowGfx>(gfx: &mut G, text: &mut CText, leading: i16) -> i16 {
    let mut p_str = text.p_str;
    let mut maxchars: u16 = u16::MAX;

    text.baseline.y -= leading;

    loop {
        let mut next: *const std::ffi::c_char = std::ptr::null();
        text.p_str = p_str;
        text.baseline.y += leading;
        let eol = gfx.line_within_width(text, &mut next, PLAYER_TEXT_WIDTH, maxchars);
        maxchars = maxchars.wrapping_sub(text.char_count);
        maxchars = maxchars.saturating_sub(1);
        p_str = next;

        if text.baseline.y < SIS_SCREEN_HEIGHT {
            gfx.draw_text(text);
        }
        if eol != 0 || maxchars == 0 {
            break;
        }
    }

    text.baseline.y
}

/// The game's drawing surface: the C graphics exports.
#[cfg(not(test))]
struct CommWindow;

#[cfg(not(test))]
impl CommWindowGfx for CommWindow {
    fn current_activity(&self) -> u16 {
        crate::mainloop::ffi::get_current_activity().0
    }

    fn space_context(&self) -> *mut std::ffi::c_void {
        // SAFETY: set once at startup, read on the game thread.
        unsafe { (c_bridge::SpaceContext).cast() }
    }

    fn set_context(&mut self, ctx: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
        // SAFETY: `ctx` is a live context handle owned by the game.
        unsafe { (c_bridge::SetContext((ctx).cast())).cast() }
    }

    fn set_context_font(&mut self, font: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
        // SAFETY: `font` is a live font handle or the previous context font.
        unsafe { (c_bridge::SetContextFont((font).cast())).cast() }
    }

    fn set_foreground_color(&mut self, color: CColor) {
        // SAFETY: plain value call.
        unsafe { c_bridge::SetContextForeGroundColor(color) };
    }

    fn font_leading(&mut self) -> i16 {
        let mut leading: i16 = 0;
        // SAFETY: `leading` is valid for the call.
        unsafe { c_bridge::GetContextFontLeading(&mut leading) };
        leading
    }

    fn batch(&mut self) {
        // SAFETY: plain call.
        unsafe { c_bridge::BatchGraphics() };
    }

    fn unbatch(&mut self) {
        // SAFETY: plain call.
        unsafe { c_bridge::UnbatchGraphics() };
    }

    fn fill_rect(&mut self, rect: &mut CRect) {
        // SAFETY: `rect` is valid for the call.
        unsafe { c_bridge::DrawFilledRectangle(std::ptr::from_mut(rect).cast()) };
    }

    fn draw_text(&mut self, text: &mut CText) {
        // SAFETY: `text` and its string are valid for the call.
        unsafe { c_bridge::font_DrawText(std::ptr::from_mut(text).cast()) };
    }

    fn line_within_width(
        &mut self,
        text: &mut CText,
        next: &mut *const std::ffi::c_char,
        width: i16,
        max_chars: u16,
    ) -> std::ffi::c_int {
        // SAFETY: `text` and `next` are valid for the call.
        unsafe { c_bridge::getLineWithinWidth(text, next, width, max_chars) }
    }

    fn game_string(&mut self, id: i32) -> *const std::ffi::c_char {
        // SAFETY: `GameStrings` is loaded before any communication starts.
        unsafe { game_string(id) }
    }

    fn response_text(&mut self, index: u8, buf: &mut [u8]) -> bool {
        // SAFETY: `buf` is valid for `buf.len()` bytes.
        unsafe {
            c_bridge::rust_get_response_text(
                std::ffi::c_int::from(index),
                buf.as_mut_ptr() as *mut std::ffi::c_char,
                buf.len()
                    .try_into()
                    .expect("response buffer length fits i32"),
            ) != 0
        }
    }

    fn draw_scroll_indicator(&mut self, frame_index: u16, origin_y: i16) {
        // SAFETY: `ActivityFrame` is loaded before any communication starts.
        unsafe {
            let frame = c_bridge::SetAbsFrameIndex((c_bridge::ActivityFrame).cast(), frame_index);
            if let Some(mut stamp) = scroll_indicator_stamp(frame.cast(), origin_y) {
                c_bridge::DrawStamp(std::ptr::from_mut(&mut stamp).cast());
            }
        }
    }

    fn conversation_player_font(&mut self) -> *mut std::ffi::c_void {
        crate::comm::player_font::CONVERSATION_PLAYER_FONT.current()
    }
}

/// Draw the SIS communication window background.
#[cfg(not(test))]
pub unsafe fn draw_sis_com_window() {
    draw_sis_com_window_with(&mut CommWindow);
}

/// Render the player's selected response text in the SIS comm window.
///
/// # Safety
/// Caller must ensure `text` is a valid null-terminated C string or null,
/// and that the graphics subsystem is initialized.
#[cfg(not(test))]
pub unsafe fn feedback_player_phrase(text: *const std::ffi::c_char) {
    feedback_player_phrase_with(&mut CommWindow, text);
}

/// Render the response list in the SIS comm window.
#[cfg(not(test))]
pub unsafe fn refresh_responses(top: u8, num_responses: u8, cur_response: u8) {
    LAST_TOP_RESPONSE = top;
    LAST_NUM_RESPONSES = num_responses;
    LAST_CUR_RESPONSE = cur_response;
    refresh_responses_with(&mut CommWindow, top, num_responses, cur_response);
}

/// Show conversation history summary page.
/// Ported from `c_SelectConversationSummary()` in rust_comm.c.
#[cfg(not(test))]
pub unsafe fn select_conversation_summary() {
    let mut text_buf = [0u8; 1024];

    if LAST_NUM_RESPONSES > 0
        && c_bridge::rust_get_response_text(
            LAST_CUR_RESPONSE as std::ffi::c_int,
            text_buf.as_mut_ptr() as *mut std::ffi::c_char,
            text_buf
                .len()
                .try_into()
                .expect("response buffer length fits i32"),
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
    state.next_sub = c_bridge::GetFirstTrackSubtitle();
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
    fn scroll_indicator_uses_the_native_frame_width_and_preserves_its_origin_y() {
        use crate::graphics::native_abi::{frame_desc, EXTENT, HOT_SPOT};

        unsafe {
            let mut frame: frame_desc = std::mem::zeroed();
            frame.Bounds = EXTENT {
                width: 13,
                height: 9,
            };
            frame.HotSpot = HOT_SPOT { x: -4, y: 2 };
            let stamp = scroll_indicator_stamp(&mut frame, 225).unwrap();
            assert_eq!(stamp.origin.x, 228);
            assert_eq!(stamp.origin.y, 225);
            assert_eq!(stamp.frame, std::ptr::from_mut(&mut frame).cast());
            assert_eq!((frame.Bounds.width, frame.Bounds.height), (13, 9));
            assert_eq!((frame.HotSpot.x, frame.HotSpot.y), (-4, 2));
        }
    }

    #[test]
    fn missing_scroll_indicator_frame_is_not_drawn() {
        assert!(unsafe { scroll_indicator_stamp(std::ptr::null_mut(), 1) }.is_none());
    }

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
        assert_eq!(SIS_SCREEN_WIDTH, 242);
        assert_eq!(SIS_SCREEN_HEIGHT, 227);
        assert_eq!(CHECK_ABORT, 0x4000);
        assert_eq!(ONE_SECOND, 840);
    }
}
