//! hail_alien encounter orchestration.
//!
//! Implements the full alien encounter sequence replacing C's `hail_alien(comm)`.
//! Follows comm.c:1183–1308 step-by-step plus DoCommunication exit handling
//! from comm.c:1100–1138.
//!
//! @plan PLAN-20260326-COMMPT2.P07
//! @requirement REQ-HL-001

#[cfg(not(test))]
use crate::comm::locdata::resource_key;
use crate::comm::Comm;
#[cfg(not(test))]
use crate::engine::StoryKey;

use crate::engine::{battle::Cx, observe::Io};

// ============================================================================
// C bridge declarations (not compiled in test builds)
// ============================================================================

#[cfg(not(test))]
pub(super) mod c_bridge {
    use crate::comm::Comm;
    use std::ffi::c_int;

    pub use crate::graphics::context_abi::{
        CreateContextAux, DestroyContext, SetContext, SetContextBackGroundColor, SetContextClipRect,
    };
    pub use crate::graphics::drawable_abi::{
        CreateDrawable, DestroyDrawable, GetFrameRect, SetContextFGFrame, SetFrameTransparentColor,
    };
    pub use crate::graphics::font_abi::{DestroyFont, SetContextFont};
    pub use crate::graphics::gfx_common_abi::SetTransitionSource;
    pub use crate::graphics::pixmap_abi::{CaptureDrawable, ReleaseDrawable};
    pub use crate::sound::heart_ffi::stop_track as StopTrack;
    pub use crate::sound::heart_ffi::{fade_music as FadeMusic, stop_sound as StopSound};
    pub use crate::strings::sresins::LoadStringTableInstance;
    pub use crate::strings::string_table::{
        CaptureStringTable, DestroyStringTable, ReleaseStringTable,
    };
    pub use crate::ui::settings::stop_music;

    pub use crate::graphics::resgfx::LoadGraphicInstance;

    pub use crate::comm::comm_abi::set_anim_context;

    pub use crate::comm::comm_abi::set_text_cache_context;

    pub use crate::comm::comm_abi::set_text_cache_frame;

    pub use crate::graphics::frame_abi::ClearDrawable;

    pub use crate::graphics::cmap_abi::FlushColorXForms;

    pub use crate::comm::comm_abi::set_talking_finished;

    pub use crate::comm::comm_abi::setup_subtitle_text_from_comm_data;

    extern "C" {

        pub static mut ScreenWidth: c_int;
        pub static mut ScreenHeight: c_int;

    }
    pub use crate::graphics::gfx_common_abi::BatchGraphics;
    pub use crate::sound::heart_ffi::DestroyMusic;
    pub use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;

    use super::{CFontRelease, NORMAL_VOLUME};

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
        pub(super) unsafe fn finish(self, comm: &mut Comm, cx: &mut crate::engine::battle::Cx<'_>) {
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
            cx.with_legacy(|| {
                // AnimContext teardown (C lines 1126–1128)
                SetContext(space_ctx.cast());
                DestroyContext(anim_ctx);

                // FlushColorXForms, ClearSubtitles (C lines 1130–1131)
                FlushColorXForms();
                crate::comm::ffi::rust_clear_subtitles(comm);

                // Stop audio, fade music (C lines 1133–1136)
                stop_music();
                StopSound();
                StopTrack();
                let fade_end = FadeMusic(NORMAL_VOLUME as u8, 0);
                SleepThreadUntil(fade_end.wrapping_add(840 / 60));

                // ----------------------------------------------------------------
                // Step 16: Call post/uninit encounter funcs
                // ----------------------------------------------------------------
            });
            let activity = cx.world.activity();
            if (activity & 0x4000) == 0 && (activity & 0x1000) == 0 {
                if let Some(callback) = comm.data.post_encounter_func {
                    callback(comm, cx);
                }
            }
            if let Some(callback) = comm.data.uninit_encounter_func {
                callback(comm, cx);
            }

            // ----------------------------------------------------------------
            // Step 17: Restore context and font
            // ----------------------------------------------------------------
            cx.with_legacy(|| {
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
                comm.player_font.release(&mut CFontRelease);

                // ----------------------------------------------------------------
                // Steps 19–20: Clear CommData fields and pCurInputState
                // ----------------------------------------------------------------
                comm.data.definition.conversation_phrases_res = None;
                comm.data.conversation_phrases = std::ptr::null_mut();
                // Clear Rust-side comm_data (resources already destroyed above)
                comm.clear_comm_data();
                // pCurInputState was already cleared by c_RunEncounterDoInput.
            });
        }
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

/// Frees conversation fonts through the C graphics exports.
#[cfg(not(test))]
struct CFontRelease;

#[cfg(not(test))]
impl super::player_font::FontRelease for CFontRelease {
    fn drain_draw_queue(&mut self) {
        crate::mainloop::input_frame::drain_committed_draws();
    }

    fn destroy_font(&mut self, font: *mut std::ffi::c_void) {
        // SAFETY: `font` is the conversation font, released exactly once.
        unsafe { c_bridge::DestroyFont(font.cast()) };
    }
}

/// Check if this is a starbase conversation by reading C game state.
/// Matches: GET_GAME_STATE(GLOBAL_FLAGS_AND_DATA) == 0xFF && GET_GAME_STATE(STARBASE_AVAILABLE)
#[cfg(not(test))]
fn is_starbase_conversation(world: &crate::engine::World) -> bool {
    // Direct game state access (replaces c_IsStarbaseConversation)
    let global_flags = world.story_state(StoryKey::GLOBAL_FLAGS_AND_DATA);
    let starbase_available = world.story_state(StoryKey::STARBASE_AVAILABLE);
    global_flags == 0xFF && starbase_available != 0
}

/// Draw the player communication panel using the shared C-parity renderer.
#[cfg(not(test))]
fn draw_sis_com_window(comm: &mut Comm, activity: u16) {
    super::sis_graphics::draw_sis_com_window(comm, activity);
}

// ============================================================================
// NORMAL_VOLUME — matches libs/sndlib.h
// ============================================================================
#[cfg(not(test))]
const NORMAL_VOLUME: i32 = 128;

// ============================================================================
// hail_alien — full encounter orchestration
// ============================================================================

#[cfg(not(test))]
unsafe fn load_alien_song(
    descriptor: &super::races::descriptor::CommDescriptor,
) -> *mut std::ffi::c_void {
    let alt_song = resource_key(descriptor.alien_alt_song_res);
    if descriptor.alien_song_flags & LDASF_USE_ALTERNATE != 0 && !alt_song.is_null() {
        let song = crate::sound::resource_types::LoadMusicInstance(alt_song);
        if !song.is_null() {
            return song;
        }
    }
    crate::sound::resource_types::LoadMusicInstance(resource_key(descriptor.alien_song_res))
}

/// Run the full alien encounter sequence.
///
/// Implements C's `hail_alien(comm)` (comm.c:1183–1308) plus the
/// `DoCommunication` exit-handling path (comm.c:1100–1138).
///
/// # Safety
/// Must be called from the game thread with CommData fully initialized.
///
/// @plan PLAN-20260326-COMMPT2.P07
/// @requirement REQ-HL-001
pub unsafe fn hail_alien(_comm: &mut Comm, cx: &mut Cx<'_>, io: &mut Io<'_>) {
    #[cfg(test)]
    let _ = (cx, io);
    #[cfg(not(test))]
    use crate::engine::observe::{CheckpointView, InputView};
    #[cfg(not(test))]
    {
        let input = cx.kernel.input_runtime().input().clone();
        use c_bridge::*;
        use std::ptr;

        // ----------------------------------------------------------------
        // Step 1: Encounter state initialization
        // Reset Rust-side comm state for the new encounter, then sync
        // C statics (pCurInputState, TalkingFinished) via bridges.
        // ----------------------------------------------------------------
        let activity = cx.world.activity();
        let resources = cx.with_legacy(|| {
            eprintln!("[DBG] HailAlien: clearing COMM_STATE");
            _comm.reset_for_encounter();
            {
                let s = &*_comm;
                eprintln!(
                    "[DBG] HailAlien: after clear, talking_finished={}",
                    s.is_talking_finished()
                );
            }
            set_talking_finished(_comm, 0);

            // ----------------------------------------------------------------
            // Step 2: Load PlayerFont
            // ----------------------------------------------------------------
            let player_font = LoadGraphicInstance(c"font.player".as_ptr());
            // The conversation owns this font: the response and feedback renderers
            // borrow it, and it is freed only at teardown after queued draws drain.
            _comm
                .player_font
                .install(player_font)
                .unwrap_or_else(|error| panic!("HailAlien: {error}"));

            // ----------------------------------------------------------------
            // Step 3: Load and set alien resources
            // ----------------------------------------------------------------

            // AlienFrame: load → capture → set
            let descriptor = &_comm.data.definition;
            let alien_frame_raw = LoadGraphicInstance(resource_key(descriptor.alien_frame_res));
            let alien_frame = CaptureDrawable(alien_frame_raw.cast());
            _comm.data.alien_frame = alien_frame.cast();

            // AlienFont: load → set (not captured, direct Destroy on exit)
            let alien_font = LoadGraphicInstance(resource_key(descriptor.alien_font_res));
            _comm.data.alien_font = alien_font;

            // AlienColorMap: load → capture → set
            let alien_cmap_raw =
                LoadStringTableInstance(resource_key(descriptor.alien_colormap_res));
            let alien_cmap = CaptureStringTable(alien_cmap_raw);
            _comm.data.alien_colormap = alien_cmap.cast();

            // AlienSong: alt-song fallback then primary
            let alien_song = load_alien_song(descriptor);
            _comm.data.alien_song = alien_song;

            // ConversationPhrases: load → capture → set
            let phrases_raw =
                LoadStringTableInstance(resource_key(descriptor.conversation_phrases_res));
            let phrases = CaptureStringTable(phrases_raw);
            _comm.data.conversation_phrases = phrases.cast();

            // Populate COMM_STATE.comm_data so Rust-side NPCPhrase can
            // resolve conversation phrases without calling back into C.
            {
                let comm_data = super::types::CommData {
                    conversation_phrases: phrases.cast(),
                    alien_frame: alien_frame.cast(),
                    alien_font,
                    alien_color_map: alien_cmap.cast(),
                    alien_song,
                    ..Default::default()
                };
                _comm.set_comm_data(comm_data);
            }

            // ----------------------------------------------------------------
            // Step 4: Subtitle text setup
            // ----------------------------------------------------------------
            setup_subtitle_text_from_comm_data(_comm);

            // ----------------------------------------------------------------
            // Step 5: TextCacheContext setup
            // ----------------------------------------------------------------
            let text_cache_ctx_name = c"TextCacheContext".as_ptr() as *const _;
            let text_cache_ctx = create_named_context(text_cache_ctx_name);

            let sis_w = unsafe { c_bridge::ScreenWidth - 78 };
            let sis_h = unsafe { c_bridge::ScreenHeight - 13 };
            let slider_y = 107;
            let slider_h = 15;
            let cache_height = sis_h - slider_y - slider_h + 2;

            let want_pixmap: std::ffi::c_uint = 2; // WANT_PIXMAP = 1 << 1
            let cache_frame_raw =
                CreateDrawable(want_pixmap as u8, sis_w as i16, cache_height as i16, 1);
            let text_cache_frame = CaptureDrawable(cache_frame_raw);

            set_text_cache_context(_comm, text_cache_ctx.cast());
            set_text_cache_frame(_comm, text_cache_frame.cast());
            SetContext(text_cache_ctx);
            SetContextFGFrame(text_cache_frame);
            // TextBack = BUILD_COLOR(MAKE_RGB15(0x00, 0x00, 0x10), 0x00)
            let text_back = crate::graphics::native_abi::Color {
                r: 0,
                g: 0,
                b: (0x10 << 3) | (0x10 >> 2),
                a: 0xff,
            };
            SetContextBackGroundColor(text_back);
            ClearDrawable();
            SetFrameTransparentColor(text_cache_frame, text_back);

            // ----------------------------------------------------------------
            // Step 6: Clear phrase buffer
            // ----------------------------------------------------------------

            // ----------------------------------------------------------------
            // Step 7: Set SpaceContext and save old font
            // ----------------------------------------------------------------
            let space_ctx = { _comm.presentation.space_context };
            SetContext(space_ctx.cast());
            let old_font = SetContextFont(player_font.cast());

            // ----------------------------------------------------------------
            // Step 8: Create AnimContext and configure
            // ----------------------------------------------------------------
            let anim_ctx = create_named_context(c"AnimContext".as_ptr().cast());
            set_anim_context(_comm, anim_ctx.cast());
            SetContext(anim_ctx);
            let screen = { _comm.presentation.screen };
            SetContextFGFrame(screen.cast());

            let mut frame_rect = crate::comm::locdata::CRect::default();
            GetFrameRect(alien_frame, ptr::addr_of_mut!(frame_rect).cast());

            // CommWndRect.extent = { SIS_SCREEN_WIDTH, frame_rect.height }
            // CommWndRect.corner stays at its current value for WON_LAST_BATTLE
            let wnd_x = { _comm.display.window.corner.x as i32 };
            let wnd_y = { _comm.display.window.corner.y as i32 };
            {
                _comm.display.window.corner.x = wnd_x as i16;
                _comm.display.window.corner.y = wnd_y as i16;
                _comm.display.window.width = sis_w as i16;
                _comm.display.window.height = frame_rect.height;
            }

            if let Some(window) = &_comm.options.window {
                window.set(_comm.display.window);
            }

            // ----------------------------------------------------------------
            // Steps 9–10: Transition, batch, draw SIS UI
            // ----------------------------------------------------------------
            SetTransitionSource(ptr::null());
            BatchGraphics();

            if (activity & 0xFF) == 5 || _comm.options.window.is_some() {
                // WON_LAST_BATTLE branch: set clip to current CommWndRect
                unsafe {
                    SetContextClipRect((&raw mut _comm.display.window).cast());
                }
            } else {
                // Normal branch: set clip to SIS origin + CommWndRect size
                unsafe {
                    _comm.display.window.corner.x = 7; // SIS_ORG_X
                    _comm.display.window.corner.y = 10; // SIS_ORG_Y
                    SetContextClipRect((&raw mut _comm.display.window).cast());
                }
            }

            c_bridge::HailResources {
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
            }
        });

        if activity & 0xff != 5 {
            let title = is_starbase_conversation(cx.world).then_some(STARBASE_STRING_BASE as usize);
            let dimensions = cx.screen_dimensions();
            crate::ui::border::draw_conversation_frame(cx.kernel, cx.world, dimensions, title);
        }
        cx.with_legacy(|| draw_sis_com_window(_comm, activity));

        cx.kernel.last_activity |= 0x1000;
        if let Some(callback) = _comm.data.init_encounter_func {
            callback(_comm, cx);
            super::ffi::observe_responses(_comm, io);
        }

        // Run the encounter loop: DoInput with rust_do_communication as InputFunc.
        // Allocates ENCOUNTER_STATE, wires InputFunc, registers pCurInputState,
        // runs DoInput, then clears pCurInputState.
        {
            const MENU_SOUND_UP: u16 = 1 << 0;
            const MENU_SOUND_DOWN: u16 = 1 << 1;
            const MENU_SOUND_SELECT: u16 = 1 << 4;
            cx.with_legacy(|| {
                input.set_menu_sounds(MENU_SOUND_UP | MENU_SOUND_DOWN, MENU_SOUND_SELECT);
            });
            io.observer
                .checkpoint(&CheckpointView::CommunicationStarted);
            _comm.run_input(cx, io, false, |_comm, cx, io| {
                crate::comm::ffi::rust_do_communication(_comm, &input, cx, io) != 0
            });
            io.observer.checkpoint(&CheckpointView::CommunicationEnded);
        }

        // ----------------------------------------------------------------
        // DoCommunication exit handling (C lines 1126–1136):
        // These operations execute when DoCommunication returns FALSE.
        // In the Rust path, c_RunEncounterDoInput has already returned,
        // meaning the DoInput loop has finished. The teardown that C does
        // inside the final DoCommunication iteration (AnimContext destroy,
        // FlushColorXForms, ClearSubtitles, stop audio) is performed here.
        // ----------------------------------------------------------------

        resources.finish(_comm, cx);
        let campaign = crate::engine::observe::CampaignView::capture(cx.world, cx.kernel);
        cx.with_legacy(|| {
            _comm.admit_observer(io, &InputView::CommunicationCompleted, &Some(campaign))
        });
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::comm::Comm;
    /// hail_alien is a no-op in test builds (all C bridge calls are gated
    /// behind #[cfg(not(test))]). The public function must exist and be callable.
    #[test]
    fn test_hail_alien_compiles() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        // Verify the function exists and is callable in test context.
        // The actual logic is gated behind cfg(not(test)) and requires
        // the full C game runtime, so we just confirm it compiles.
        let mut game = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let mut cx = crate::engine::battle::Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        let mut observer = crate::engine::observe::Unobserved;
        let mut io = crate::engine::observe::Io {
            observer: &mut observer,
        };
        unsafe { super::hail_alien(comm, &mut cx, &mut io) };
    }
}

#[cfg(not(test))]
unsafe fn create_named_context(
    name: *const std::ffi::c_char,
) -> *mut crate::graphics::native_abi::context_desc {
    #[cfg(feature = "debug-process")]
    {
        c_bridge::CreateContextAux(name)
    }
    #[cfg(not(feature = "debug-process"))]
    {
        let _ = name;
        c_bridge::CreateContextAux()
    }
}
