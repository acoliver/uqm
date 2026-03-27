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

/* HailAlien bridge (P11) */
void rust_HailAlien(void);
int rust_AlienTalkSegue(unsigned int wait_track);


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
unsigned int c_FadeMusic(int vol, int duration);
void c_StopMusic(void);
void c_SetMenuSounds(unsigned int up_down, unsigned int select);

void c_SetMenuSounds(unsigned int up_down, unsigned int select);

/* @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-AT-001 */
int c_HasTransitionAnim(void);

/* Alliance name lookup with full i==3 CommanderName concatenation.
 * @plan PLAN-20260326-COMMPT2.P04 @requirement REQ-NP-001 */
const unsigned char *c_get_alliance_name_full(int adjusted_index, char *buf, int buf_len);

/* Sound-clip and timestamp for a 0-based phrase-table index.
 * @plan PLAN-20260326-COMMPT2.P04 @requirement REQ-NP-001 */
void *c_get_phrase_sound_clip(const void *phrases, int index);
void *c_get_phrase_timestamp(const void *phrases, int index);

#ifdef __cplusplus
}
#endif

#endif /* USE_RUST_COMM */

#endif /* RUST_COMM_H */
