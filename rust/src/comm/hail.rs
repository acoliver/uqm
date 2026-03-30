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
    use std::ffi::{c_char, c_int, c_uint};

    extern "C" {
        // Resource loading — accepts RESOURCE (const char *) as *const c_char
        pub fn c_LoadFont(res: *const c_char) -> usize;
        pub fn c_LoadGraphic(res: *const c_char) -> usize;
        pub fn c_LoadColorMap(res: *const c_char) -> usize;
        pub fn c_LoadMusic(res: *const c_char) -> usize;
        pub fn c_LoadStringTable(res: *const c_char) -> usize;

        // Capture (converts raw handle to ref-counted handle)
        pub fn c_CaptureDrawable(handle: usize) -> usize;
        pub fn c_CaptureColorMap(handle: usize) -> usize;
        pub fn c_CaptureStringTable(handle: usize) -> usize;

        // Release (converts ref-counted handle back to raw; must precede Destroy)
        pub fn c_ReleaseDrawable(handle: usize) -> usize;
        pub fn c_ReleaseColorMap(handle: usize) -> usize;
        pub fn c_ReleaseStringTable(handle: usize) -> usize;

        // Resource destruction (use raw handles returned by Release)
        pub fn c_DestroyDrawable(handle: usize);
        pub fn c_DestroyFont(handle: usize);
        pub fn c_DestroyColorMap(handle: usize);
        pub fn c_DestroyMusic(handle: usize);
        pub fn c_DestroyStringTable(handle: usize);

        // Context management
        pub fn c_CreateContext(name: *const c_char) -> usize;
        pub fn c_DestroyContext(ctx: usize);
        pub fn c_SetContext(ctx: usize) -> usize;
        pub fn c_SetContextFGFrame(frame: usize);
        pub fn c_SetContextClipRect(x: c_int, y: c_int, w: c_int, h: c_int);
        pub fn c_SetContextBackGroundColor(r: c_int, g: c_int, b: c_int);
        pub fn c_SetContextFont(font: usize) -> usize;

        // comm.c static variable setters
        pub fn c_SetAnimContext(ctx: usize);
        pub fn c_SetTextCacheContext(ctx: usize);
        pub fn c_SetTextCacheFrame(frame: usize);

        // Drawable management
        pub fn c_CreateDrawable(dtype: c_uint, w: c_int, h: c_int, nframes: c_int) -> usize;
        pub fn c_SetFrameTransparentColor(frame: usize, r: c_int, g: c_int, b: c_int);
        pub fn c_ClearDrawable();
        pub fn c_GetFrameRect(
            frame: usize,
            x: *mut c_int,
            y: *mut c_int,
            w: *mut c_int,
            h: *mut c_int,
        );

        // Graphics batching
        pub fn c_BatchGraphics();

        // Transitions
        pub fn c_SetTransitionSource(src: usize);

        // SIS drawing
        pub fn c_DrawSISFrame();
        pub fn c_DrawSISMessage(msg: *const c_char);
        pub fn c_DrawSISTitle(title: *const c_char);
        pub fn c_DrawSISComWindow();

        // Encounter loop — runs DoInput with rust_DoCommunication as InputFunc
        pub fn c_RunEncounterDoInput();

        // Audio teardown
        pub fn c_StopMusic();
        pub fn c_StopSound();
        pub fn c_StopTrack();
        pub fn c_FadeMusic(vol: c_int, duration: c_int) -> c_uint;
        pub fn c_SleepThreadUntil(time: c_uint);
        pub fn c_FlushColorXForms();

        // Activity flags
        pub fn c_SetLastActivityCheckLoad();
        pub fn c_CheckAbort() -> c_int;
        pub fn c_CheckLoad() -> c_int;

        // Screen / context accessors
        pub fn c_GetScreen() -> usize;
        pub fn c_GetSpaceContext() -> usize;

        // CommData accessors
        pub fn c_GetCommDataAlienFrameRes() -> *const c_char;
        pub fn c_GetCommDataAlienFontRes() -> *const c_char;
        pub fn c_GetCommDataAlienColorMapRes() -> *const c_char;
        pub fn c_GetCommDataAlienSongRes() -> *const c_char;
        pub fn c_GetCommDataAlienAltSongRes() -> *const c_char;
        pub fn c_GetCommDataAlienSongFlags() -> c_uint;
        pub fn c_GetCommDataConversationPhrasesRes() -> *const c_char;
        pub fn c_SetCommDataAlienFrame(frame: usize);
        pub fn c_SetCommDataAlienFont(font: usize);
        pub fn c_SetCommDataAlienColorMap(cmap: usize);
        pub fn c_SetCommDataAlienSong(song: usize);
        pub fn c_SetCommDataConversationPhrases(phrases: usize);
        pub fn c_ClearCommDataConversationPhrasesRes();
        pub fn c_ClearCommDataConversationPhrases();

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
        pub fn c_WonLastBattle() -> c_int;
        pub fn c_GetCommWndRect(x: *mut c_int, y: *mut c_int, w: *mut c_int, h: *mut c_int);
        pub fn c_SetCommWndRect(x: c_int, y: c_int, w: c_int, h: c_int);

        // Dimension constants
        pub fn c_GetSISScreenWidth() -> c_int;
        pub fn c_GetSISScreenHeight() -> c_int;
        pub fn c_GetSliderY() -> c_int;
        pub fn c_GetSliderHeight() -> c_int;
        pub fn c_GetSISOrigin(x: *mut c_int, y: *mut c_int);
        pub fn c_GetPlayerFontRes() -> *const c_char;
        pub fn c_GetWantPixmap() -> c_uint;
    }
}

// ============================================================================
// AlienSongFlags constant — matches LDASF_USE_ALTERNATE from comm.h
// ============================================================================

/// LDASF_USE_ALTERNATE: try AlienAltSongRes before AlienSongRes.
const LDASF_USE_ALTERNATE: u32 = 0x0001;

// ============================================================================
// STARBASE_STRING_BASE — matches gamestr.h
// ============================================================================

/// Game string base index for starbase strings (from gamestr.h).
const STARBASE_STRING_BASE: i32 = 0x0200;

// ============================================================================
// NORMAL_VOLUME — matches libs/sndlib.h
// ============================================================================
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
            eprintln!("[DBG] hail_alien: after clear, talking_finished={}", s.is_talking_finished());
        }
        c_SetTalkingFinished(0);

        // ----------------------------------------------------------------
        // Step 2: Load PlayerFont
        // ----------------------------------------------------------------
        let player_font_res = c_GetPlayerFontRes();
        let player_font = c_LoadFont(player_font_res);

        // ----------------------------------------------------------------
        // Step 3: Load and set alien resources
        // ----------------------------------------------------------------

        // AlienFrame: load → capture → set
        let alien_frame_raw = c_LoadGraphic(c_GetCommDataAlienFrameRes());
        let alien_frame = c_CaptureDrawable(alien_frame_raw);
        c_SetCommDataAlienFrame(alien_frame);

        // AlienFont: load → set (not captured, direct Destroy on exit)
        let alien_font = c_LoadFont(c_GetCommDataAlienFontRes());
        c_SetCommDataAlienFont(alien_font);

        // AlienColorMap: load → capture → set
        let alien_cmap_raw = c_LoadColorMap(c_GetCommDataAlienColorMapRes());
        let alien_cmap = c_CaptureColorMap(alien_cmap_raw);
        c_SetCommDataAlienColorMap(alien_cmap);

        // AlienSong: alt-song fallback then primary
        let song_flags = c_GetCommDataAlienSongFlags();
        let alt_song_res = c_GetCommDataAlienAltSongRes();
        let alien_song = if (song_flags & LDASF_USE_ALTERNATE) != 0 && !alt_song_res.is_null() {
            let alt = c_LoadMusic(alt_song_res);
            if alt != 0 {
                alt
            } else {
                c_LoadMusic(c_GetCommDataAlienSongRes())
            }
        } else {
            c_LoadMusic(c_GetCommDataAlienSongRes())
        };
        c_SetCommDataAlienSong(alien_song);

        // ConversationPhrases: load → capture → set
        let phrases_raw = c_LoadStringTable(c_GetCommDataConversationPhrasesRes());
        let phrases = c_CaptureStringTable(phrases_raw);
        c_SetCommDataConversationPhrases(phrases);

        // Populate COMM_STATE.comm_data so Rust-side NPCPhrase can
        // resolve conversation phrases without calling back into C.
        {
            let mut comm_data = super::types::CommData::default();
            comm_data.conversation_phrases = phrases as *mut std::ffi::c_void;
            comm_data.alien_frame = alien_frame as *mut std::ffi::c_void;
            comm_data.alien_font = alien_font as *mut std::ffi::c_void;
            comm_data.alien_color_map = alien_cmap as *mut std::ffi::c_void;
            comm_data.alien_song = alien_song as *mut std::ffi::c_void;
            super::state::COMM_STATE.write().set_comm_data(comm_data);
        }

        // ----------------------------------------------------------------
        // Step 4: Subtitle text setup
        // ----------------------------------------------------------------
        c_SetupSubtitleTextFromCommData();

        // ----------------------------------------------------------------
        // Step 5: TextCacheContext setup
        // ----------------------------------------------------------------
        let text_cache_ctx_name = b"TextCacheContext\0".as_ptr() as *const _;
        let text_cache_ctx = c_CreateContext(text_cache_ctx_name);

        let sis_w = c_GetSISScreenWidth();
        let sis_h = c_GetSISScreenHeight();
        let slider_y = c_GetSliderY();
        let slider_h = c_GetSliderHeight();
        let cache_height = sis_h - slider_y - slider_h + 2;

        let want_pixmap = c_GetWantPixmap();
        let cache_frame_raw = c_CreateDrawable(want_pixmap, sis_w, cache_height, 1);
        let text_cache_frame = c_CaptureDrawable(cache_frame_raw);

        c_SetTextCacheContext(text_cache_ctx);
        c_SetTextCacheFrame(text_cache_frame);
        c_SetContext(text_cache_ctx);
        c_SetContextFGFrame(text_cache_frame);
        // TextBack = BUILD_COLOR(MAKE_RGB15(0x00, 0x00, 0x10), 0x00)
        c_SetContextBackGroundColor(0x00, 0x00, 0x10);
        c_ClearDrawable();
        c_SetFrameTransparentColor(text_cache_frame, 0x00, 0x00, 0x10);

        // ----------------------------------------------------------------
        // Step 6: Clear phrase buffer
        // ----------------------------------------------------------------
        c_ClearPhraseBuf();

        // ----------------------------------------------------------------
        // Step 7: Set SpaceContext and save old font
        // ----------------------------------------------------------------
        let space_ctx = c_GetSpaceContext();
        c_SetContext(space_ctx);
        let old_font = c_SetContextFont(player_font);

        // ----------------------------------------------------------------
        // Step 8: Create AnimContext and configure
        // ----------------------------------------------------------------
        let anim_ctx_name = b"AnimContext\0".as_ptr() as *const _;
        let anim_ctx = c_CreateContext(anim_ctx_name);
        c_SetAnimContext(anim_ctx);
        c_SetContext(anim_ctx);
        let screen = c_GetScreen();
        c_SetContextFGFrame(screen);

        let mut _frame_x: i32 = 0;
        let mut _frame_y: i32 = 0;
        let mut _frame_w: i32 = 0;
        let mut frame_h: i32 = 0;
        c_GetFrameRect(
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
        c_SetTransitionSource(0);
        c_BatchGraphics();

        if c_WonLastBattle() != 0 {
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
            c_SetContextClipRect(cx, cy, cw, ch);
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
            c_SetContextClipRect(org_x, org_y, _wnd_w2, _wnd_h2);
            // Update CommWndRect.corner to SIS origin
            c_SetCommWndRect(org_x, org_y, _wnd_w2, _wnd_h2);

            c_DrawSISFrame();

            if c_IsStarbaseConversation() != 0 {
                // Talking to allied Starbase
                // GAME_STRING(STARBASE_STRING_BASE + 1) = "Starbase Commander"
                let msg = c_GetGameString(STARBASE_STRING_BASE, 1);
                c_DrawSISMessage(msg);
                // GAME_STRING(STARBASE_STRING_BASE + 0) = "Starbase"
                let title = c_GetGameString(STARBASE_STRING_BASE, 0);
                c_DrawSISTitle(title);
            } else {
                // Default titles: NULL message + planet name
                c_DrawSISMessage(ptr::null());
                let planet_name = c_GetPlanetName();
                c_DrawSISTitle(planet_name);
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
        c_SetContext(space_ctx);
        c_DestroyContext(anim_ctx);

        // FlushColorXForms, ClearSubtitles (C lines 1130–1131)
        c_FlushColorXForms();
        super::ffi::rust_ClearSubtitles();

        // Stop audio, fade music (C lines 1133–1136)
        c_StopMusic();
        c_StopSound();
        c_StopTrack();
        let fade_end = c_FadeMusic(NORMAL_VOLUME, 0);
        // ONE_SECOND/60 ≈ 16ms at 60Hz; c_FadeMusic returns TimeCount
        // The sleep ensures the fade completes before teardown.
        // We approximate ONE_SECOND/60 as 1 tick unit (C uses GetTimeCounter units).
        c_SleepThreadUntil(fade_end + 1);

        // ----------------------------------------------------------------
        // Step 16: Call post/uninit encounter funcs
        // ----------------------------------------------------------------
        if c_CheckAbort() == 0 && c_CheckLoad() == 0 {
            c_CallPostEncounterFunc();
        }
        c_CallUninitEncounterFunc();

        // ----------------------------------------------------------------
        // Step 17: Restore context and font
        // ----------------------------------------------------------------
        c_SetContext(space_ctx);
        c_SetContextFont(old_font);

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

        // ConversationPhrases — captured; c_DestroyStringTable does Release+Destroy
        c_DestroyStringTable(phrases);

        // AlienSong — not captured, direct Destroy
        c_DestroyMusic(alien_song);

        // AlienColorMap — captured; c_DestroyColorMap does Release+Destroy
        c_DestroyColorMap(alien_cmap);

        // AlienFont — not captured, direct Destroy
        c_DestroyFont(alien_font);

        // AlienFrame — captured; c_DestroyDrawable does Release+Destroy
        c_DestroyDrawable(alien_frame);

        // TextCacheContext — context, direct destroy
        c_DestroyContext(text_cache_ctx);

        // TextCacheFrame — captured; c_DestroyDrawable does Release+Destroy
        c_DestroyDrawable(text_cache_frame);

        // PlayerFont — not captured, direct Destroy
        c_DestroyFont(player_font);

        // ----------------------------------------------------------------
        // Steps 19–20: Clear CommData fields and pCurInputState
        // ----------------------------------------------------------------
        c_ClearCommDataConversationPhrasesRes();
        c_ClearCommDataConversationPhrases();
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
