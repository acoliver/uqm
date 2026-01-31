/*
 * Rust Audio System
 *
 * Simple audio API using rodio for playback.
 * Replaces the OpenAL-style mixer with a simpler interface.
 */

#ifndef RUST_AUDIO_H
#define RUST_AUDIO_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Sound categories for volume control */
#define RUST_AUDIO_MUSIC  0
#define RUST_AUDIO_SFX    1
#define RUST_AUDIO_SPEECH 2

/* Initialize the audio system. Returns 1 on success, 0 on failure. */
int rust_audio_init(void);

/* Shutdown the audio system */
void rust_audio_uninit(void);

/* Play a WAV file from memory. Returns handle (>0) on success, 0 on failure.
 * data: pointer to WAV file data (including header)
 * len: length of data in bytes
 * category: RUST_AUDIO_MUSIC, RUST_AUDIO_SFX, or RUST_AUDIO_SPEECH
 * looping: non-zero to loop, 0 for one-shot
 */
uint32_t rust_audio_play_wav(const uint8_t *data, size_t len, int category, int looping);

/* Play an OGG file from memory. Returns handle (>0) on success, 0 on failure. */
uint32_t rust_audio_play_ogg(const uint8_t *data, size_t len, int category, int looping);

/* Play raw PCM audio data. Returns handle (>0) on success, 0 on failure.
 * sample_rate: e.g., 44100
 * channels: 1 for mono, 2 for stereo
 * bits_per_sample: 8 or 16
 */
uint32_t rust_audio_play_raw(const uint8_t *data, size_t len,
                              uint32_t sample_rate, uint16_t channels,
                              uint16_t bits_per_sample, int category, int looping);

/* Stop a playing sound */
void rust_audio_stop(uint32_t handle);

/* Pause a playing sound */
void rust_audio_pause(uint32_t handle);

/* Resume a paused sound */
void rust_audio_resume(uint32_t handle);

/* Set volume for a specific sound (0.0 - 1.0) */
void rust_audio_set_volume(uint32_t handle, float volume);

/* Set master volume (0.0 - 1.0) */
void rust_audio_set_master_volume(float volume);

/* Set music volume (0.0 - 1.0) */
void rust_audio_set_music_volume(float volume);

/* Set SFX volume (0.0 - 1.0) */
void rust_audio_set_sfx_volume(float volume);

/* Set speech volume (0.0 - 1.0) */
void rust_audio_set_speech_volume(float volume);

/* Check if a sound is still playing. Returns 1 if playing, 0 if stopped/finished. */
int rust_audio_is_playing(uint32_t handle);

/* Stop all sounds */
void rust_audio_stop_all(void);

/* Cleanup finished sounds (call periodically) */
void rust_audio_cleanup(void);

#ifdef __cplusplus
}
#endif

#endif /* RUST_AUDIO_H */
