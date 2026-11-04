//! Conversation resources released after the typed encounter session stops.
use super::c_bridge;
use super::c_bridge::*;
use super::{CFontRelease, NORMAL_VOLUME};
use crate::comm::player_font::CONVERSATION_PLAYER_FONT;

pub(super) struct HailResources {
    pub(super) space_ctx: *mut crate::graphics::native_abi::context_desc,
    pub(super) anim_ctx: *mut crate::graphics::native_abi::context_desc,
    pub(super) old_font: *mut crate::graphics::native_abi::font_desc,
    pub(super) phrases: *mut crate::strings::string_table::StringTableEntry,
    pub(super) alien_song: *mut std::ffi::c_void,
    pub(super) alien_cmap: *mut crate::strings::string_table::StringTableEntry,
    pub(super) alien_font: *mut std::ffi::c_void,
    pub(super) alien_frame: *mut crate::graphics::native_abi::frame_desc,
    pub(super) text_cache_ctx: *mut crate::graphics::native_abi::context_desc,
    pub(super) text_cache_frame: *mut crate::graphics::native_abi::frame_desc,
}

impl HailResources {
    pub(super) unsafe fn finish(self, activity: impl FnOnce() -> u16) {
        let Self {
            space_ctx,
            anim_ctx,
            old_font,
            phrases,
            alien_song,
            alien_cmap,
            alien_font,
            alien_frame,
            text_cache_ctx,
            text_cache_frame,
        } = self;
        // AnimContext teardown (C lines 1126–1128)
        SetContext(space_ctx.cast());
        DestroyContext(anim_ctx);

        // FlushColorXForms, ClearSubtitles (C lines 1130–1131)
        FlushColorXForms();
        crate::comm::ffi::rust_clear_subtitles();

        // Stop audio, fade music (C lines 1133–1136)
        stop_music();
        StopSound();
        StopTrack();
        let fade_end = FadeMusic(NORMAL_VOLUME as u8, 0);
        SleepThreadUntil(fade_end.wrapping_add(840 / 60));

        // ----------------------------------------------------------------
        // Step 16: Call post/uninit encounter funcs
        // ----------------------------------------------------------------
        let activity = activity();
        if (activity & 0x4000) == 0 && (activity & 0x1000) == 0 {
            if let Some(callback) = c_bridge::CommData.post_encounter_func {
                callback();
            }
        }
        if let Some(callback) = c_bridge::CommData.uninit_encounter_func {
            callback();
        }

        // ----------------------------------------------------------------
        // Step 17: Restore context and font
        // ----------------------------------------------------------------
        SetContext(space_ctx.cast());
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

        // ConversationPhrases — captured; Release to get back STRING_TABLE, then Destroy
        DestroyStringTable(ReleaseStringTable(phrases));

        // AlienSong — not captured, direct Destroy
        DestroyMusic(alien_song);

        // AlienColorMap — captured; release before destroying the table
        DestroyStringTable(ReleaseStringTable(alien_cmap));

        // AlienFont — not captured, direct Destroy
        DestroyFont(alien_font.cast());

        // AlienFrame — captured; ReleaseDrawable to get back raw handle, then DestroyDrawable
        DestroyDrawable(ReleaseDrawable(alien_frame));

        // TextCacheContext — context, direct destroy
        DestroyContext(text_cache_ctx);

        // TextCacheFrame — captured; ReleaseDrawable to get back raw handle, then DestroyDrawable
        DestroyDrawable(ReleaseDrawable(text_cache_frame));

        // PlayerFont — not captured; queued response/feedback glyph draws may
        // still read it, so the release drains the draw queue first.
        CONVERSATION_PLAYER_FONT.release(&mut CFontRelease);

        // ----------------------------------------------------------------
        // Steps 19–20: Clear CommData fields and pCurInputState
        // ----------------------------------------------------------------
        unsafe { c_bridge::CommData.conversation_phrases_res = std::ptr::null() };
        unsafe { c_bridge::CommData.conversation_phrases = std::ptr::null_mut() };
        // Clear Rust-side comm_data (resources already destroyed above)
        crate::comm::state::COMM_STATE.write().clear_comm_data();
        // pCurInputState was already cleared by c_RunEncounterDoInput.
    }
}
