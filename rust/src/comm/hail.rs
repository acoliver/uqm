//! HailAlien encounter orchestration.
//!
//! Implements the full alien encounter sequence replacing C's `HailAlien()`.
//! Follows comm.c:1183–1308 step-by-step plus DoCommunication exit handling
//! from comm.c:1100–1138.
//!
//! @plan PLAN-20260326-COMMPT2.P07
//! @requirement REQ-HL-001

// ============================================================================
// C bridge declarations (not compiled in test builds)
// ============================================================================

#[cfg(not(test))]
mod c_bridge {
    use super::super::locdata::CLocData;
    use std::ffi::{c_char, c_int, c_uint, c_void};

    // The C global CommData (LOCDATA struct) — accessed directly without bridge functions
    extern "C" {
        pub static mut CommData: CLocData;
    }

    extern "C" {
        // Resource loading — accepts RESOURCE (const char *) as *const c_char
        pub fn LoadGraphicInstance(res: *const c_char) -> *mut c_void;
        pub fn LoadMusicInstance(res: *const c_char) -> *mut c_void;
        pub fn LoadStringTableInstance(res: *const c_char) -> *mut c_void;

        // Capture (converts raw handle to ref-counted handle)
        pub fn CaptureDrawable(handle: *mut c_void) -> *mut c_void;
        pub fn CaptureStringTable(handle: *mut c_void) -> *mut c_void;

        // Resource destruction
        pub fn DestroyDrawable(handle: *mut c_void);
        pub fn DestroyFont(handle: *mut c_void) -> c_int;
        #[allow(clashing_extern_declarations)]
        pub fn DestroyMusic(handle: *mut c_void) -> c_int;
        pub fn DestroyStringTable(handle: *mut c_void) -> c_int;
        pub fn ReleaseStringTable(handle: *mut c_void) -> *mut c_void;

        // Context management
        pub fn CreateContextAux(name: *const c_char) -> *mut c_void;
        pub fn DestroyContext(ctx: *mut c_void) -> c_int;
        pub fn SetContext(ctx: *mut c_void) -> *mut c_void;
        pub fn SetContextFGFrame(frame: *mut c_void) -> *mut c_void;
        pub fn SetContextClipRect(x: c_int, y: c_int, w: c_int, h: c_int);
        #[allow(clashing_extern_declarations)]
        pub fn SetContextBackGroundColor(r: c_int, g: c_int, b: c_int);
        pub fn SetContextFont(font: *mut c_void) -> *mut c_void;

        // comm.c static variable setters
        pub fn c_SetAnimContext(ctx: *mut c_void);
        pub fn c_SetTextCacheContext(ctx: *mut c_void);
        pub fn c_SetTextCacheFrame(frame: *mut c_void);

        // Drawable management
        pub fn CreateDrawable(dtype: u8, w: i16, h: i16, nframes: u16) -> *mut c_void;
        pub fn SetFrameTransparentColor(frame: *mut c_void, r: c_int, g: c_int, b: c_int);
        pub fn ClearDrawable();
        #[allow(clashing_extern_declarations)]
        pub fn GetFrameRect(
            frame: *mut c_void,
            x: *mut c_int,
            y: *mut c_int,
            w: *mut c_int,
            h: *mut c_int,
        );

        // Graphics batching
        pub fn BatchGraphics();

        // Transitions
        pub fn SetTransitionSource(src: *mut c_void);

        // SIS drawing
        pub fn DrawSISFrame();
        pub fn DrawSISMessage(msg: *const c_char);
        pub fn DrawSISTitle(title: *const c_char);
        pub fn c_DrawSISComWindow();

        // Encounter loop — runs DoInput with rust_DoCommunication as InputFunc
        pub fn c_RunEncounterDoInput();

        // Audio teardown
        pub fn StopMusic();
        pub fn StopSound();
        pub fn StopTrack();
        pub fn FadeMusic(vol: u8, duration: i16) -> c_uint;
        pub fn SleepThreadUntil(time: c_uint);
        pub fn FlushColorXForms();

        // Activity flags — read C-side CurrentActivity via existing bridge
        pub fn get_current_activity() -> u16;
        pub fn c_SetLastActivityCheckLoad();

        // Screen / context globals
        pub static mut Screen: *mut c_void;
        pub static mut SpaceContext: *mut c_void;

        // Encounter functions
        pub fn c_CallInitEncounterFunc();
        pub fn c_CallPostEncounterFunc();
        pub fn c_CallUninitEncounterFunc();

        // Comm-internal static variable accessors
        pub fn c_SetTalkingFinished(finished: c_int);
        pub fn c_SetupSubtitleTextFromCommData();
        pub fn c_ClearPhraseBuf();

        // Game-state / layout queries
        pub fn c_IsStarbaseConversation() -> c_int;
        pub fn c_GetPlanetName() -> *const c_char;
        pub fn c_GetGameString(base: c_int, offset: c_int) -> *const c_char;
        pub fn c_GetCommWndRect(x: *mut c_int, y: *mut c_int, w: *mut c_int, h: *mut c_int);
        pub fn c_SetCommWndRect(x: c_int, y: c_int, w: c_int, h: c_int);

        // Dimension constants
        pub fn c_GetSISOrigin(x: *mut c_int, y: *mut c_int);
        pub fn c_GetPlayerFontRes() -> *const c_char;
        pub fn c_GetWantPixmap() -> c_uint;

        // C runtime globals for screen dimensions
        pub static mut ScreenWidth: c_int;
        pub static mut ScreenHeight: c_int;
    }
}

// ============================================================================
// AlienSongFlags constant — matches LDASF_USE_ALTERNATE from comm.h
// ============================================================================

/// LDASF_USE_ALTERNATE: try AlienAltSongRes before AlienSongRes.
#[cfg(not(test))]
const LDASF_USE_ALTERNATE: u32 = 0x0001;

// ============================================================================
// STARBASE_STRING_BASE — matches gamestr.h
// ============================================================================

/// Game string base index for starbase strings (from gamestr.h).
#[cfg(not(test))]
const STARBASE_STRING_BASE: i32 = 0x0200;

// ============================================================================
// NORMAL_VOLUME — matches libs/sndlib.h
// ============================================================================
#[cfg(not(test))]
const NORMAL_VOLUME: i32 = 128;

// ============================================================================
// hail_alien — full encounter orchestration
// ============================================================================

/// Run the full alien encounter sequence.
///
/// Implements C's `HailAlien()` (comm.c:1183–1308) plus the
/// `DoCommunication` exit-handling path (comm.c:1100–1138).
///
/// # Safety
/// Must be called from the game thread with CommData fully initialized.
///
/// @plan PLAN-20260326-COMMPT2.P07
/// @requirement REQ-HL-001
pub unsafe fn hail_alien() {
    #[cfg(not(test))]
    {
        use c_bridge::*;
        use std::ptr;

        // ----------------------------------------------------------------
        // Step 1: Encounter state initialization
        // Reset Rust-side comm state for the new encounter, then sync
        // C statics (pCurInputState, TalkingFinished) via bridges.
        // ----------------------------------------------------------------
        eprintln!("[DBG] hail_alien: clearing COMM_STATE");
        super::state::COMM_STATE.write().clear();
        {
            let s = super::state::COMM_STATE.read();
            eprintln!(
                "[DBG] hail_alien: after clear, talking_finished={}",
                s.is_talking_finished()
            );
        }
        c_SetTalkingFinished(0);

        // ----------------------------------------------------------------
        // Step 2: Load PlayerFont
        // ----------------------------------------------------------------
        let player_font_res = c_GetPlayerFontRes();
        let player_font = LoadGraphicInstance(player_font_res);

        // ----------------------------------------------------------------
        // Step 3: Load and set alien resources
        // ----------------------------------------------------------------

        // AlienFrame: load → capture → set
        let alien_frame_raw = LoadGraphicInstance(unsafe { c_bridge::CommData.alien_frame_res });
        let alien_frame = CaptureDrawable(alien_frame_raw);
        unsafe { c_bridge::CommData.alien_frame = alien_frame };

        // AlienFont: load → set (not captured, direct Destroy on exit)
        let alien_font = LoadGraphicInstance(unsafe { c_bridge::CommData.alien_font_res });
        unsafe { c_bridge::CommData.alien_font = alien_font };

        // AlienColorMap: load → capture → set
        let alien_cmap_raw =
            LoadStringTableInstance(unsafe { c_bridge::CommData.alien_colormap_res });
        let alien_cmap = CaptureStringTable(alien_cmap_raw);
        unsafe { c_bridge::CommData.alien_colormap = alien_cmap };

        // AlienSong: alt-song fallback then primary
        let song_flags = unsafe { c_bridge::CommData.alien_song_flags };
        let alt_song_res = unsafe { c_bridge::CommData.alien_alt_song_res };
        let alien_song = if (song_flags & LDASF_USE_ALTERNATE) != 0 && !alt_song_res.is_null() {
            let alt = LoadMusicInstance(alt_song_res);
            if !alt.is_null() {
                alt
            } else {
                LoadMusicInstance(unsafe { c_bridge::CommData.alien_song_res })
            }
        } else {
            LoadMusicInstance(unsafe { c_bridge::CommData.alien_song_res })
        };
        unsafe { c_bridge::CommData.alien_song = alien_song };

        // ConversationPhrases: load → capture → set
        let phrases_raw =
            LoadStringTableInstance(unsafe { c_bridge::CommData.conversation_phrases_res });
        let phrases = CaptureStringTable(phrases_raw);
        unsafe { c_bridge::CommData.conversation_phrases = phrases };

        // Populate COMM_STATE.comm_data so Rust-side NPCPhrase can
        // resolve conversation phrases without calling back into C.
        {
            let comm_data = super::types::CommData {
                conversation_phrases: phrases,
                alien_frame,
                alien_font,
                alien_color_map: alien_cmap,
                alien_song,
                ..Default::default()
            };
            super::state::COMM_STATE.write().set_comm_data(comm_data);
        }

        // ----------------------------------------------------------------
        // Step 4: Subtitle text setup
        // ----------------------------------------------------------------
        c_SetupSubtitleTextFromCommData();

        // ----------------------------------------------------------------
        // Step 5: TextCacheContext setup
        // ----------------------------------------------------------------
        let text_cache_ctx_name = c"TextCacheContext".as_ptr() as *const _;
        let text_cache_ctx = CreateContextAux(text_cache_ctx_name);

        let sis_w = unsafe { c_bridge::ScreenWidth - 78 };
        let sis_h = unsafe { c_bridge::ScreenHeight - 13 };
        let slider_y = 107;
        let slider_h = 15;
        let cache_height = sis_h - slider_y - slider_h + 2;

        let want_pixmap = c_GetWantPixmap();
        let cache_frame_raw =
            CreateDrawable(want_pixmap as u8, sis_w as i16, cache_height as i16, 1);
        let text_cache_frame = CaptureDrawable(cache_frame_raw);

        c_SetTextCacheContext(text_cache_ctx);
        c_SetTextCacheFrame(text_cache_frame);
        SetContext(text_cache_ctx);
        SetContextFGFrame(text_cache_frame);
        // TextBack = BUILD_COLOR(MAKE_RGB15(0x00, 0x00, 0x10), 0x00)
        SetContextBackGroundColor(0x00, 0x00, 0x10);
        ClearDrawable();
        SetFrameTransparentColor(text_cache_frame, 0x00, 0x00, 0x10);

        // ----------------------------------------------------------------
        // Step 6: Clear phrase buffer
        // ----------------------------------------------------------------
        c_ClearPhraseBuf();

        // ----------------------------------------------------------------
        // Step 7: Set SpaceContext and save old font
        // ----------------------------------------------------------------
        let space_ctx = unsafe { SpaceContext };
        SetContext(space_ctx);
        let old_font = SetContextFont(player_font);

        // ----------------------------------------------------------------
        // Step 8: Create AnimContext and configure
        // ----------------------------------------------------------------
        let anim_ctx_name = c"AnimContext".as_ptr() as *const _;
        let anim_ctx = CreateContextAux(anim_ctx_name);
        c_SetAnimContext(anim_ctx);
        SetContext(anim_ctx);
        let screen = unsafe { Screen };
        SetContextFGFrame(screen);

        let mut _frame_x: i32 = 0;
        let mut _frame_y: i32 = 0;
        let mut _frame_w: i32 = 0;
        let mut frame_h: i32 = 0;
        GetFrameRect(
            alien_frame,
            ptr::addr_of_mut!(_frame_x),
            ptr::addr_of_mut!(_frame_y),
            ptr::addr_of_mut!(_frame_w),
            ptr::addr_of_mut!(frame_h),
        );

        // CommWndRect.extent = { SIS_SCREEN_WIDTH, frame_h }
        // CommWndRect.corner stays at its current value for WON_LAST_BATTLE
        let mut wnd_x: i32 = 0;
        let mut wnd_y: i32 = 0;
        let mut _wnd_w: i32 = 0;
        let mut _wnd_h: i32 = 0;
        c_GetCommWndRect(
            ptr::addr_of_mut!(wnd_x),
            ptr::addr_of_mut!(wnd_y),
            ptr::addr_of_mut!(_wnd_w),
            ptr::addr_of_mut!(_wnd_h),
        );
        c_SetCommWndRect(wnd_x, wnd_y, sis_w, frame_h);

        // ----------------------------------------------------------------
        // Steps 9–10: Transition, batch, draw SIS UI
        // ----------------------------------------------------------------
        SetTransitionSource(ptr::null_mut());
        BatchGraphics();

        if (c_bridge::get_current_activity() & 0xFF) == 5 {
            // WON_LAST_BATTLE branch: set clip to current CommWndRect corner
            let mut cx: i32 = 0;
            let mut cy: i32 = 0;
            let mut cw: i32 = 0;
            let mut ch: i32 = 0;
            c_GetCommWndRect(
                ptr::addr_of_mut!(cx),
                ptr::addr_of_mut!(cy),
                ptr::addr_of_mut!(cw),
                ptr::addr_of_mut!(ch),
            );
            SetContextClipRect(cx, cy, cw, ch);
        } else {
            // Normal branch
            let mut org_x: i32 = 0;
            let mut org_y: i32 = 0;
            c_GetSISOrigin(ptr::addr_of_mut!(org_x), ptr::addr_of_mut!(org_y));

            let mut _wnd_w2: i32 = 0;
            let mut _wnd_h2: i32 = 0;
            c_GetCommWndRect(
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::addr_of_mut!(_wnd_w2),
                ptr::addr_of_mut!(_wnd_h2),
            );
            SetContextClipRect(org_x, org_y, _wnd_w2, _wnd_h2);
            // Update CommWndRect.corner to SIS origin
            c_SetCommWndRect(org_x, org_y, _wnd_w2, _wnd_h2);

            DrawSISFrame();

            if c_IsStarbaseConversation() != 0 {
                // Talking to allied Starbase
                // GAME_STRING(STARBASE_STRING_BASE + 1) = "Starbase Commander"
                let msg = c_GetGameString(STARBASE_STRING_BASE, 1);
                DrawSISMessage(msg);
                // GAME_STRING(STARBASE_STRING_BASE + 0) = "Starbase"
                let title = c_GetGameString(STARBASE_STRING_BASE, 0);
                DrawSISTitle(title);
            } else {
                // Default titles: NULL message + planet name
                DrawSISMessage(ptr::null());
                let planet_name = c_GetPlanetName();
                DrawSISTitle(planet_name);
            }
        }

        // DrawSISComWindow is unconditional (C line 1278)
        c_DrawSISComWindow();

        // ----------------------------------------------------------------
        // Step 11: Set CHECK_LOAD flag, call encounter funcs, run DoInput
        // ----------------------------------------------------------------
        c_SetLastActivityCheckLoad();
        c_CallInitEncounterFunc();

        // Run the encounter loop: DoInput with rust_DoCommunication as InputFunc.
        // c_RunEncounterDoInput allocates ENCOUNTER_STATE, wires InputFunc,
        // registers pCurInputState, runs DoInput, then clears pCurInputState.
        c_RunEncounterDoInput();

        // ----------------------------------------------------------------
        // DoCommunication exit handling (C lines 1126–1136):
        // These operations execute when DoCommunication returns FALSE.
        // In the Rust path, c_RunEncounterDoInput has already returned,
        // meaning the DoInput loop has finished. The teardown that C does
        // inside the final DoCommunication iteration (AnimContext destroy,
        // FlushColorXForms, ClearSubtitles, stop audio) is performed here.
        // ----------------------------------------------------------------

        // AnimContext teardown (C lines 1126–1128)
        SetContext(space_ctx);
        DestroyContext(anim_ctx);

        // FlushColorXForms, ClearSubtitles (C lines 1130–1131)
        FlushColorXForms();
        super::ffi::rust_ClearSubtitles();

        // Stop audio, fade music (C lines 1133–1136)
        StopMusic();
        StopSound();
        StopTrack();
        let fade_end = FadeMusic(NORMAL_VOLUME as u8, 0);
        // ONE_SECOND/60 ≈ 16ms at 60Hz; FadeMusic returns TimeCount
        // The sleep ensures the fade completes before teardown.
        // We approximate ONE_SECOND/60 as 1 tick unit (C uses GetTimeCounter units).
        SleepThreadUntil(fade_end + 1);

        // ----------------------------------------------------------------
        // Step 16: Call post/uninit encounter funcs
        // ----------------------------------------------------------------
        let activity = c_bridge::get_current_activity();
        if (activity & 0x4000) == 0 && (activity & 0x1000) == 0 {
            c_CallPostEncounterFunc();
        }
        c_CallUninitEncounterFunc();

        // ----------------------------------------------------------------
        // Step 17: Restore context and font
        // ----------------------------------------------------------------
        SetContext(space_ctx);
        SetContextFont(old_font);

        // ----------------------------------------------------------------
        // Step 18: Destroy all resources in exact C order
        // Captured resources: Release first, then Destroy the raw handle.
        // Non-captured resources: Destroy directly.
        // Order: ConversationPhrases → AlienSong → AlienColorMap →
        //        AlienFont → AlienFrame → TextCacheContext → TextCacheFrame
        //        → PlayerFont
        // ----------------------------------------------------------------

        // c_Destroy* for captured resources (Drawable, ColorMap, StringTable)
        // already do Release+Destroy internally.  Non-captured resources
        // (Font, Music) go straight to Destroy.

        // ConversationPhrases — captured; DestroyStringTable does Release+Destroy
        DestroyStringTable(phrases);

        // AlienSong — not captured, direct Destroy
        DestroyMusic(alien_song);

        // AlienColorMap — captured; release before destroying the table
        DestroyStringTable(ReleaseStringTable(alien_cmap));

        // AlienFont — not captured, direct Destroy
        DestroyFont(alien_font);

        // AlienFrame — captured; DestroyDrawable does Release+Destroy
        DestroyDrawable(alien_frame);

        // TextCacheContext — context, direct destroy
        DestroyContext(text_cache_ctx);

        // TextCacheFrame — captured; DestroyDrawable does Release+Destroy
        DestroyDrawable(text_cache_frame);

        // PlayerFont — not captured, direct Destroy
        DestroyFont(player_font);

        // ----------------------------------------------------------------
        // Steps 19–20: Clear CommData fields and pCurInputState
        // ----------------------------------------------------------------
        unsafe { c_bridge::CommData.conversation_phrases_res = std::ptr::null() };
        unsafe { c_bridge::CommData.conversation_phrases = std::ptr::null_mut() };
        // Clear Rust-side comm_data (resources already destroyed above)
        super::state::COMM_STATE.write().clear_comm_data();
        // pCurInputState was already cleared by c_RunEncounterDoInput
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    /// hail_alien is a no-op in test builds (all C bridge calls are gated
    /// behind #[cfg(not(test))]). The public function must exist and be callable.
    #[test]
    fn test_hail_alien_compiles() {
        // Verify the function exists and is callable in test context.
        // The actual logic is gated behind cfg(not(test)) and requires
        // the full C game runtime, so we just confirm it compiles.
        unsafe {
            // In test mode this is a no-op — no C runtime needed.
            super::hail_alien();
        }
    }
}
