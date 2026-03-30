/*
 *  Rust Communication FFI bindings header
 *  Used when USE_RUST_COMM is defined.
 */

#ifndef RUST_COMM_H
#define RUST_COMM_H

#ifdef USE_RUST_COMM

#ifdef __cplusplus
extern "C" {
#endif

/* Initialization */
int rust_InitCommunication(void);
void rust_UninitCommunication(void);
int rust_IsCommInitialized(void);
void rust_ClearCommunication(void);

/* HailAlien bridge (P07) */
void rust_HailAlien(void);
int rust_AlienTalkSegue(unsigned int wait_track);
int rust_DoCommunication(void);


/* NPCPhrase routing (P11) */
void rust_NPCPhrase_cb(int index, void (*cb)(void));
void rust_NPCPhrase_splice(int index);

/* Track Management */
int rust_StartTrack(void);
void rust_StopTrack(void);
void rust_RewindTrack(void);
void rust_JumpTrack(void);
void rust_SeekTrack(float position);
float rust_CommitTrack(void);
int rust_WaitTrack(void);
float rust_GetTrackPosition(void);
float rust_GetTrackLength(void);
void rust_SpliceTrack(unsigned int audio_handle, const char *text, float start_time, float duration);
void rust_SpliceTrackText(const char *text, float start_time, float duration);
void rust_ClearTrack(void);

/* Subtitle Management */
const char *rust_GetSubtitle(void);
void rust_SetSubtitlesEnabled(int enabled);
int rust_AreSubtitlesEnabled(void);

/* Response System */
int rust_DoResponsePhrase(unsigned int response_ref, const char *text, void (*func)(unsigned int));
void rust_DisplayResponses(void);
void rust_ClearResponses(void);
int rust_SelectNextResponse(void);
int rust_SelectPrevResponse(void);
int rust_GetSelectedResponse(void);
int rust_GetResponseCount(void);
int rust_GetResponseText(int index, char *buf, int buf_len);
unsigned int rust_ExecuteResponse(void);

/* Animation Management */
void rust_StartCommAnimation(unsigned int index);
void rust_StopCommAnimation(unsigned int index);
void rust_StartAllCommAnimations(void);
void rust_StopAllCommAnimations(void);
void rust_PauseCommAnimations(void);
void rust_ResumeCommAnimations(void);
unsigned int rust_GetCommAnimationFrame(unsigned int index);
/* C-bridge signature matching commanim.c (clear=FullRedraw, paused=paused) */
int rust_ProcessCommAnimations_cb(int clear, int paused);
int rust_WantTalkingAnim(void);
int rust_HaveTalkingAnim(void);
int rust_HaveTransitionAnim(void);
void rust_SetRunIntroAnim(int run);
void rust_SetRunTalkingAnim(int run);
void rust_SetStopTalkingAnim(void);
int rust_RunningIntroAnim(void);
int rust_RunningTalkingAnim(void);

/* Oscilloscope */
void rust_AddOscilloscopeSamples(const short *samples, unsigned int count);
void rust_UpdateOscilloscope(void);
unsigned char rust_GetOscilloscopeY(unsigned int x);
void rust_ClearOscilloscope(void);

/* State Queries */
int rust_IsTalking(void);
int rust_IsTalkingFinished(void);
void rust_SetTalkingFinished(int finished);
unsigned int rust_GetCommIntroMode(void);
void rust_SetCommIntroMode(unsigned int mode);
unsigned int rust_GetCommFadeTime(void);
void rust_SetCommFadeTime(unsigned int time);
int rust_IsCommInputPaused(void);
void rust_SetCommInputPaused(int paused);
void rust_UpdateCommunication(float delta_time);

/* Track Player */
unsigned int rust_PlayingTrack(void);
void rust_FastForward_Page(void);
void rust_FastForward_Smooth(void);
void rust_FastReverse_Page(void);
void rust_FastReverse_Smooth(void);

/* Subtitle state (P10/P11) */
void rust_ClearSubtitles(void);
void rust_CheckSubtitles(void);
void rust_RedrawSubtitles(void);

/* Phrase state (P04/P11) */
int rust_PhraseEnabled(int index);
void rust_DisablePhrase(int index);

/* Segue state (P04/P11) */
void rust_SetSegue(unsigned int segue);
unsigned int rust_GetSegue(void);
unsigned int rust_GetBattleSegue(void);

/*
 * ============================================================================
 *  C-side trackplayer wrapper seam (@plan PLAN-20260314-COMM.P05b)
 *
 *  Thin wrappers consumed by Rust comm FFI — backed by the authoritative
 *  C trackplayer in sc2/src/libs/sound/trackplayer.c.
 * ============================================================================
 */
#include "libs/compiler.h"
#include "libs/callback.h"
#include "libs/sound/trackplayer.h"

void c_SpliceTrack(UNICODE *filespec, UNICODE *textspec,
                   UNICODE *timestamp, CallbackFunction cb);
void c_SpliceMultiTrack(UNICODE *track_names[], UNICODE *track_text);
void c_PlayTrack(void);
void c_StopTrack(void);
void c_JumpTrack(void);
COUNT c_PlayingTrack(void);
void c_PauseTrack(void);
void c_ResumeTrack(void);
const UNICODE *c_GetTrackSubtitle(void);
SUBTITLE_REF c_GetFirstTrackSubtitle(void);
SUBTITLE_REF c_GetNextTrackSubtitle(SUBTITLE_REF last_ref);
const UNICODE *c_GetTrackSubtitleText(SUBTITLE_REF sub_ref);
void c_FastForward_Page(void);
void c_FastForward_Smooth(void);
void c_FastReverse_Page(void);
void c_FastReverse_Smooth(void);
int c_GetTrackPosition(int in_units);

/*
 * ============================================================================
 *  C-side graphics/input/music bridge wrappers (P11)
 *  Called from Rust via FFI when Rust needs to trigger C-side rendering.
 * ============================================================================
 */
#include "oscill.h"

void c_InitOscilloscope(unsigned int frame);
void c_DrawOscilloscope(void);
void c_DrawSlider(void);
void c_SetSliderImage(unsigned int frame);
void c_ClearSubtitles(void);
void c_CheckSubtitles(void);
void c_RedrawSubtitles(void);

/* Subtitle bridge stubs — bodies in comm.c, behaviour added in P08.
 * @plan PLAN-20260325-COMMPT3.P06
 * @requirement REQ-SD-001..REQ-SD-003 */
void comm_ClearSubtitles(void);
void comm_CheckSubtitles(void);
void comm_RedrawSubtitles(void);

unsigned int c_FadeMusic(int vol, int duration);
void c_StopMusic(void);
void c_SetMenuSounds(unsigned int up_down, unsigned int select);

/* Colormap + Music bridge functions sourced from CommData.
 * @plan PLAN-20260325-COMMPT3.P05
 * @requirement REQ-CM-001, REQ-CM-002, REQ-MU-001, REQ-MU-002
 * @pseudocode 001-colormap-music-bridges lines 01-15 */
void c_SetColorMapFromCommData(void);
void c_PlayAlienMusic(void);

/* @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-AT-001 */
int c_HasTransitionAnim(void);

/* Alliance name lookup with full i==3 CommanderName concatenation.
 * @plan PLAN-20260326-COMMPT2.P04 @requirement REQ-NP-001 */
const unsigned char *c_get_alliance_name_full(int adjusted_index, char *buf, int buf_len);

/* Sound-clip and timestamp for a 0-based phrase-table index.
 * @plan PLAN-20260326-COMMPT2.P04 @requirement REQ-NP-001 */
void *c_get_phrase_sound_clip(const void *phrases, int index);
void *c_get_phrase_timestamp(const void *phrases, int index);

/*
 * ============================================================================
 *  Resource Bridge (P06, @plan PLAN-20260326-COMMPT2.P06)
 *
 *  Thin wrappers for resource load/capture/release, context management,
 *  drawable management, batching, transitions, SIS drawing, DoInput,
 *  and CommData/game-state accessors called from Rust.
 * ============================================================================
 */
#include <stdint.h>

/* Resource load bridges — @requirement REQ-HL-002 */
uintptr_t c_LoadGraphic(const char *res);
uintptr_t c_LoadFont(const char *res);
uintptr_t c_LoadColorMap(const char *res);
uintptr_t c_LoadMusic(const char *res);
uintptr_t c_LoadStringTable(const char *res);

/* Capture/Release bridges — @requirement REQ-HL-002 */
uintptr_t c_CaptureDrawable(uintptr_t handle);
uintptr_t c_CaptureColorMap(uintptr_t handle);
uintptr_t c_CaptureStringTable(uintptr_t handle);
uintptr_t c_ReleaseDrawable(uintptr_t handle);
uintptr_t c_ReleaseColorMap(uintptr_t handle);
uintptr_t c_ReleaseStringTable(uintptr_t handle);

/* Context management bridges — @requirement REQ-HL-003 */
uintptr_t c_CreateContext(const char *name);
void      c_DestroyContext(uintptr_t ctx);
uintptr_t c_SetContext(uintptr_t ctx);
void      c_SetContextFGFrame(uintptr_t frame);
void      c_SetContextClipRect(int x, int y, int w, int h);
void      c_ClearContextClipRect(void);
void      c_SetContextBackGroundColor(int r, int g, int b);
uintptr_t c_SetContextFont(uintptr_t font);

/* Drawable management bridges — @requirement REQ-HL-003 */
uintptr_t c_CreateDrawable(unsigned int type, int w, int h, int num_frames);
void      c_SetFrameTransparentColor(uintptr_t frame, int r, int g, int b);
void      c_ClearDrawable(void);
void      c_GetFrameRect(uintptr_t frame, int *x, int *y, int *w, int *h);

/* Graphics batching bridges */
void c_BatchGraphics(void);
void c_UnbatchGraphics(void);

/* Transition bridges */
void c_SetTransitionSource(uintptr_t rect_ptr);
void c_ScreenTransition(int num_frames, uintptr_t rect_ptr);

/* SIS drawing bridges — @requirement REQ-HL-007 */
void c_DrawSISFrame(void);
void c_DrawSISMessage(const char *msg);
void c_DrawSISTitle(const char *title);

/* DoInput bridge — @requirement REQ-DI-001 */
void c_DoInput(void *state, int exclusive);

/* Screen / context accessor bridges */
uintptr_t c_GetScreen(void);
uintptr_t c_GetSpaceContext(void);
void      c_SetLastActivityCheckLoad(void);

/* CommData accessor bridges */
const char  *c_GetCommDataAlienFrameRes(void);
const char  *c_GetCommDataAlienFontRes(void);
const char  *c_GetCommDataAlienColorMapRes(void);
const char  *c_GetCommDataAlienSongRes(void);
const char  *c_GetCommDataAlienAltSongRes(void);
unsigned int c_GetCommDataAlienSongFlags(void);
const char  *c_GetCommDataConversationPhrasesRes(void);
void         c_SetCommDataAlienFrame(uintptr_t frame);
void         c_SetCommDataAlienFont(uintptr_t font);
void         c_SetCommDataAlienColorMap(uintptr_t cmap);
void         c_SetCommDataAlienSong(uintptr_t song);
void         c_SetCommDataConversationPhrases(uintptr_t phrases);
void         c_ClearCommDataConversationPhrasesRes(void);
void         c_ClearCommDataConversationPhrases(void);
const void * c_GetCommConversationPhrases(void);


/* Encounter function call bridges */
void c_CallInitEncounterFunc(void);
void c_CallPostEncounterFunc(void);
void c_CallUninitEncounterFunc(void);

/* Game-state / layout query bridges */
int          c_IsStarbaseConversation(void);
const char  *c_GetGameString(int base, int offset);
const char  *c_GetPlanetName(void);
int          c_CheckLoad(void);
int          c_GetSISScreenWidth(void);
int          c_GetSISScreenHeight(void);
int          c_GetSliderY(void);
int          c_GetSliderHeight(void);
void         c_GetSISOrigin(int *x, int *y);
const char  *c_GetPlayerFontRes(void);
unsigned int c_GetWantPixmap(void);

/* CommWndRect accessor bridges */
void c_GetCommWndRect(int *x, int *y, int *w, int *h);
void c_SetCommWndRect(int x, int y, int w, int h);

/* HailAlien encounter loop bridge (@plan PLAN-20260326-COMMPT2.P07) */
void c_RunEncounterDoInput(void);

/* Audio teardown bridges (@plan PLAN-20260326-COMMPT2.P07) */
void c_StopSound(void);
void c_SleepThreadUntil(unsigned int time);
void c_FlushColorXForms(void);
unsigned int c_GetTimeCounter(void);
void c_SleepThread(unsigned int duration);
int c_RunTalkSegue(unsigned int wait_track);
void rust_UpdateSpeechGraphics(void);

/* Comm-internal static variable accessors (implemented in comm.c) */
void c_SetTalkingFinished(int finished);
void c_SetupSubtitleTextFromCommData(void);
void c_SetCurInputState(void *state);
void c_ClearPhraseBuf(void);

/* AnimContext / TextCache setters (statics in comm.c) */
void c_SetAnimContext(uintptr_t ctx);
void c_SetTextCacheContext(uintptr_t ctx);
void c_SetTextCacheFrame(uintptr_t frame);
CONTEXT c_GetAnimContext(void);
BOOLEAN c_GetClearSubtitles(void);
void c_ResetClearSubtitles(void);


#ifdef __cplusplus
}
#endif

#endif /* USE_RUST_COMM */

#endif /* RUST_COMM_H */
