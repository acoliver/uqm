// @plan PLAN-20260225-AUDIO-HEART.P21
// @requirement REQ-CROSS-GENERAL-06
//
// Rust Audio Heart — C FFI prototypes
//
// When USE_RUST_AUDIO_HEART is defined, these Rust-implemented functions
// replace the C equivalents in sound.c, stream.c, trackplayer.c,
// music.c, sfx.c, and fileinst.c.

#ifndef AUDIO_HEART_RUST_H_
#define AUDIO_HEART_RUST_H_

#include "types.h"
#include "audiocore.h"
#include "decoders/decoder.h"
#include "libs/compiler.h"
#include "libs/callback.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Stream (from stream.h)
// ============================================================================

int InitStreamDecoder(void);
void UninitStreamDecoder(void);

void PlayStream(TFB_SoundSample *sample, uint32 source, bool looping,
                bool scope, bool rewind);
void StopStream(uint32 source);
void PauseStream(uint32 source);
void ResumeStream(uint32 source);
void SeekStream(uint32 source, uint32 pos);
BOOLEAN PlayingStream(uint32 source);

int GraphForegroundStream(uint8 *data, sint32 width, sint32 height,
                          bool wantSpeech);
bool SetMusicStreamFade(sint32 howLong, int endVolume);

// ============================================================================
// Sound (from sound.h)
// ============================================================================

void StopSource(int iSource);
void CleanSource(int iSource);

void SetSFXVolume(float volume);
void SetSpeechVolume(float volume);

TFB_SoundSample *TFB_CreateSoundSample(TFB_SoundDecoder *decoder,
                                        uint32 num_buffers,
                                        const TFB_SoundCallbacks *cb);
void TFB_DestroySoundSample(TFB_SoundSample *sample);
void TFB_SetSoundSampleData(TFB_SoundSample *sample, void *data);
void *TFB_GetSoundSampleData(TFB_SoundSample *sample);
void TFB_SetSoundSampleCallbacks(TFB_SoundSample *sample,
                                  const TFB_SoundCallbacks *cb);
TFB_SoundDecoder *TFB_GetSoundSampleDecoder(TFB_SoundSample *sample);

TFB_SoundTag *TFB_FindTaggedBuffer(TFB_SoundSample *sample,
                                    audio_Object buffer);
void TFB_ClearBufferTag(TFB_SoundTag *tag);
bool TFB_TagBuffer(TFB_SoundSample *sample, audio_Object buffer,
                   intptr_t data);

BOOLEAN InitSound(int argc, char *argv[]);
void UninitSound(void);
void StopSound(void);
BOOLEAN SoundPlaying(void);
void WaitForSoundEnd(COUNT channel);

DWORD FadeMusic(BYTE end_vol, SIZE TimeInterval);
void SetMusicVolume(int vol);

// ============================================================================
// Track Player (from trackplayer.h)
// ============================================================================

void SpliceTrack(UNICODE *filespec, UNICODE *textspec,
                 UNICODE *TimeStamp, CallbackFunction cb);
void SpliceMultiTrack(UNICODE *TrackNames[], UNICODE *TrackText);

void PlayTrack(void);
void StopTrack(void);
void JumpTrack(void);
void PauseTrack(void);
void ResumeTrack(void);
COUNT PlayingTrack(void);

void FastReverse_Smooth(void);
void FastForward_Smooth(void);
void FastReverse_Page(void);
void FastForward_Page(void);

int GetTrackPosition(int in_units);
const UNICODE *GetTrackSubtitle(void);

typedef struct tfb_soundchunk *SUBTITLE_REF;
SUBTITLE_REF GetFirstTrackSubtitle(void);
SUBTITLE_REF GetNextTrackSubtitle(SUBTITLE_REF LastRef);
const UNICODE *GetTrackSubtitleText(SUBTITLE_REF SubRef);

// ============================================================================
// Music (from music.c / sndintrn.h)
// ============================================================================

void PLRPlaySong(MUSIC_REF MusicRef, BOOLEAN Continuous, BYTE Priority);
void PLRStop(MUSIC_REF MusicRef);
BOOLEAN PLRPlaying(MUSIC_REF MusicRef);
void PLRSeek(MUSIC_REF MusicRef, uint32 pos);
void PLRPause(void);
void PLRResume(void);

void snd_PlaySpeech(MUSIC_REF SpeechRef);
void snd_StopSpeech(void);

// ============================================================================
// SFX (from sfx.c)
// ============================================================================

void PlayChannel(COUNT channel, SOUND snd, SoundPosition pos,
                 void *positional_object, BYTE priority);
void StopChannel(COUNT channel, BYTE priority);
BOOLEAN ChannelPlaying(COUNT channel);
void SetChannelVolume(COUNT channel, COUNT volume, BYTE priority);
void UpdateSoundPosition(COUNT channel, SoundPosition pos);
void *GetPositionalObject(COUNT channel);
void SetPositionalObject(COUNT channel, void *obj);

// ============================================================================
// File Loading (from fileinst.c)
// ============================================================================

void *LoadSoundFile(const char *filename);
void *LoadMusicFile(const char *filename);
void DestroySound(void *bank);
void DestroyMusic(void *music_ref);

#ifdef __cplusplus
}
#endif

#endif /* AUDIO_HEART_RUST_H_ */
