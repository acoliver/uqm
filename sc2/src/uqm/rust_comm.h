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

/* Track Management */
int rust_StartTrack(void);
void rust_StopTrack(void);
void rust_RewindTrack(void);
void rust_JumpTrack(float offset);
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
int rust_DoResponsePhrase(unsigned int response_ref, const char *text, void (*func)(void));
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

#ifdef __cplusplus
}
#endif

#endif /* USE_RUST_COMM */

#endif /* RUST_COMM_H */
