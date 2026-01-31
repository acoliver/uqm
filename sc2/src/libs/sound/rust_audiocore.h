/*
 * Rust Audio Backend - audiocore-compatible API using rodio
 *
 * This header provides the C interface to the Rust rodio-based audio backend.
 * It implements the same API as audiocore.h but uses rodio for playback.
 */

#ifndef LIBS_SOUND_RUST_AUDIOCORE_H_
#define LIBS_SOUND_RUST_AUDIOCORE_H_

#include "types.h"

#ifdef USE_RUST_AUDIO

/* Types matching audiocore.h */
typedef uint32 audio_Object;
typedef int32 audio_IntVal;

/* Initialization */
extern int32 rust_audio_backend_init(int32 flags);
extern void rust_audio_backend_uninit(void);

/* Sources */
extern void rust_audio_gen_sources(uint32 n, audio_Object *psrcobj);
extern void rust_audio_delete_sources(uint32 n, audio_Object *psrcobj);
extern int32 rust_audio_is_source(audio_Object srcobj);
extern void rust_audio_source_i(audio_Object srcobj, int32 pname, audio_IntVal value);
extern void rust_audio_source_f(audio_Object srcobj, int32 pname, float value);
extern void rust_audio_source_fv(audio_Object srcobj, int32 pname, float *value);
extern void rust_audio_get_source_i(audio_Object srcobj, int32 pname, audio_IntVal *value);
extern void rust_audio_get_source_f(audio_Object srcobj, int32 pname, float *value);
extern void rust_audio_source_rewind(audio_Object srcobj);
extern void rust_audio_source_play(audio_Object srcobj);
extern void rust_audio_source_pause(audio_Object srcobj);
extern void rust_audio_source_stop(audio_Object srcobj);
extern void rust_audio_source_queue_buffers(audio_Object srcobj, uint32 n, audio_Object* pbufobj);
extern void rust_audio_source_unqueue_buffers(audio_Object srcobj, uint32 n, audio_Object* pbufobj);

/* Buffers */
extern void rust_audio_gen_buffers(uint32 n, audio_Object *pbufobj);
extern void rust_audio_delete_buffers(uint32 n, audio_Object *pbufobj);
extern int32 rust_audio_is_buffer(audio_Object bufobj);
extern void rust_audio_get_buffer_i(audio_Object bufobj, int32 pname, audio_IntVal *value);
extern void rust_audio_buffer_data(audio_Object bufobj, uint32 format, void* data, uint32 size, uint32 freq);

/* Error handling */
extern int32 rust_audio_get_error(void);

/*
 * When USE_RUST_AUDIO is defined, redirect audiocore calls to rust_audio functions.
 * This allows existing code to work without modification.
 */

#define initAudio(driver, flags) rust_audio_backend_init(flags)
#define unInitAudio() rust_audio_backend_uninit()

#define audio_GetError() rust_audio_get_error()

#define audio_GenSources(n, p) rust_audio_gen_sources(n, p)
#define audio_DeleteSources(n, p) rust_audio_delete_sources(n, p)
#define audio_IsSource(s) rust_audio_is_source(s)
#define audio_Sourcei(s, pn, v) rust_audio_source_i(s, pn, v)
#define audio_Sourcef(s, pn, v) rust_audio_source_f(s, pn, v)
#define audio_Sourcefv(s, pn, v) rust_audio_source_fv(s, pn, v)
#define audio_GetSourcei(s, pn, v) rust_audio_get_source_i(s, pn, v)
#define audio_GetSourcef(s, pn, v) rust_audio_get_source_f(s, pn, v)
#define audio_SourceRewind(s) rust_audio_source_rewind(s)
#define audio_SourcePlay(s) rust_audio_source_play(s)
#define audio_SourcePause(s) rust_audio_source_pause(s)
#define audio_SourceStop(s) rust_audio_source_stop(s)
#define audio_SourceQueueBuffers(s, n, p) rust_audio_source_queue_buffers(s, n, p)
#define audio_SourceUnqueueBuffers(s, n, p) rust_audio_source_unqueue_buffers(s, n, p)

#define audio_GenBuffers(n, p) rust_audio_gen_buffers(n, p)
#define audio_DeleteBuffers(n, p) rust_audio_delete_buffers(n, p)
#define audio_IsBuffer(b) rust_audio_is_buffer(b)
#define audio_GetBufferi(b, pn, v) rust_audio_get_buffer_i(b, pn, v)
#define audio_BufferData(b, fmt, d, sz, f) rust_audio_buffer_data(b, fmt, d, sz, f)

#endif /* USE_RUST_AUDIO */

#endif /* LIBS_SOUND_RUST_AUDIOCORE_H_ */
