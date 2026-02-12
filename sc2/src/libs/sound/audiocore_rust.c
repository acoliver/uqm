/*
 * Rust Audio Backend - implements audiocore API using rodio
 *
 * When USE_RUST_AUDIO is defined, this file provides the audiocore
 * functions by calling into the Rust rodio backend.
 */

#include "config.h"

#ifdef USE_RUST_AUDIO

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "types.h"
#include "audiocore.h"
#include "sound.h"
#include "sndintrn.h"
#include "libs/log.h"
#include "libs/threadlib.h"
#include "decoders/decoder.h"
#include "stream.h"

/* The globals that control the sound drivers. */
int snddriver, soundflags;

volatile bool audio_inited = false;

/*
 * Rust FFI declarations
 */
/* Note: audio_Object is uintptr_t which is 64-bit on ARM64 macOS */
extern sint32 rust_audio_backend_init(sint32 flags);
extern void rust_audio_backend_uninit(void);
extern sint32 rust_audio_get_error(void);
extern void rust_audio_gen_sources(uint32 n, audio_Object *psrcobj);
extern void rust_audio_delete_sources(uint32 n, audio_Object *psrcobj);
extern sint32 rust_audio_is_source(audio_Object srcobj);
extern void rust_audio_source_i(audio_Object srcobj, sint32 pname, audio_IntVal value);
extern void rust_audio_source_f(audio_Object srcobj, sint32 pname, float value);
extern void rust_audio_source_fv(audio_Object srcobj, sint32 pname, float *value);
extern void rust_audio_get_source_i(audio_Object srcobj, sint32 pname, audio_IntVal *value);
extern void rust_audio_get_source_f(audio_Object srcobj, sint32 pname, float *value);
extern void rust_audio_source_rewind(audio_Object srcobj);
extern void rust_audio_source_play(audio_Object srcobj);
extern void rust_audio_source_pause(audio_Object srcobj);
extern void rust_audio_source_stop(audio_Object srcobj);
extern void rust_audio_source_queue_buffers(audio_Object srcobj, uint32 n, audio_Object* pbufobj);
extern void rust_audio_source_unqueue_buffers(audio_Object srcobj, uint32 n, audio_Object* pbufobj);
extern void rust_audio_gen_buffers(uint32 n, audio_Object *pbufobj);
extern void rust_audio_delete_buffers(uint32 n, audio_Object *pbufobj);
extern sint32 rust_audio_is_buffer(audio_Object bufobj);
extern void rust_audio_get_buffer_i(audio_Object bufobj, sint32 pname, audio_IntVal *value);
extern void rust_audio_buffer_data(audio_Object bufobj, uint32 format, void* data, uint32 size, uint32 freq);



/*
 * Initialization
 */

sint32
initAudio (sint32 driver, sint32 flags)
{
	sint32 ret;
	TFB_DecoderFormats formats;
	
	log_add (log_Info, "initAudio: Using Rust rodio backend");
	
	(void)driver; /* We ignore the driver selection and always use rodio */
	
	ret = rust_audio_backend_init(flags);
	
	if (ret == 0)
	{
		log_add (log_Fatal, "Rust audio backend initialization failed.\n");
		exit (EXIT_FAILURE);
	}

	/* Initialize sound decoders - CRITICAL! 
	 * Without this, wava_formats is NULL and wav.c will crash */
	log_add (log_Info, "Initializing sound decoders.");
	formats.big_endian = false;
	formats.want_big_endian = false;
	formats.mono8 = audio_FORMAT_MONO8;
	formats.stereo8 = audio_FORMAT_STEREO8;
	formats.mono16 = audio_FORMAT_MONO16;
	formats.stereo16 = audio_FORMAT_STEREO16;
	if (SoundDecoder_Init (flags, &formats))
	{
		log_add (log_Error, "Sound decoders initialization failed.");
		rust_audio_backend_uninit();
		return -1;
	}
	log_add (log_Info, "Sound decoders initialized.");

	/* Initialize soundSource array - generate handles and create mutexes */
	{
		int i;
		for (i = 0; i < NUM_SOUNDSOURCES; ++i)
		{
			audio_GenSources (1, &soundSource[i].handle);
			soundSource[i].stream_mutex = CreateMutex ("Rust audio stream mutex", SYNC_CLASS_AUDIO);
		}
	}

	/* Initialize stream decoder thread */
	if (InitStreamDecoder ())
	{
		log_add (log_Error, "Stream decoder initialization failed.");
		rust_audio_backend_uninit();
		return -1;
	}

	atexit (unInitAudio);

	SetSFXVolume (sfxVolumeScale);
	SetSpeechVolume (speechVolumeScale);
	SetMusicVolume (musicVolume);
	
	audio_inited = true;
	
	return 0; /* Success */
}

void
unInitAudio (void)
{
	int i;

	if (!audio_inited)
		return;

	audio_inited = false;

	UninitStreamDecoder ();

	for (i = 0; i < NUM_SOUNDSOURCES; ++i)
	{
		if (soundSource[i].sample && soundSource[i].sample->decoder)
		{
			StopStream (i);
		}
		if (soundSource[i].sbuffer)
		{
			void *sbuffer = soundSource[i].sbuffer;
			soundSource[i].sbuffer = NULL;
			HFree (sbuffer);
		}
		DestroyMutex (soundSource[i].stream_mutex);
		soundSource[i].stream_mutex = 0;

		audio_DeleteSources (1, &soundSource[i].handle);
	}

	SoundDecoder_Uninit ();
	rust_audio_backend_uninit();
}


/*
 * General
 */

sint32
audio_GetError (void)
{
	return rust_audio_get_error();
}


/*
 * Sources
 */

void
audio_GenSources (uint32 n, audio_Object *psrcobj)
{
	rust_audio_gen_sources(n, psrcobj);
}

void
audio_DeleteSources (uint32 n, audio_Object *psrcobj)
{
	rust_audio_delete_sources(n, psrcobj);
}

bool
audio_IsSource (audio_Object srcobj)
{
	return rust_audio_is_source(srcobj) != 0;
}

void
audio_Sourcei (audio_Object srcobj, audio_SourceProp pname,
		audio_IntVal value)
{
	rust_audio_source_i(srcobj, pname, value);
}

void
audio_Sourcef (audio_Object srcobj, audio_SourceProp pname,
		float value)
{
	rust_audio_source_f(srcobj, pname, value);
}

void
audio_Sourcefv (audio_Object srcobj, audio_SourceProp pname,
		float *value)
{
	rust_audio_source_fv(srcobj, pname, value);
}

void
audio_GetSourcei (audio_Object srcobj, audio_SourceProp pname,
		audio_IntVal *value)
{
	rust_audio_get_source_i(srcobj, pname, value);
}

void
audio_GetSourcef (audio_Object srcobj, audio_SourceProp pname,
		float *value)
{
	rust_audio_get_source_f(srcobj, pname, value);
}

void
audio_SourceRewind (audio_Object srcobj)
{
	rust_audio_source_rewind(srcobj);
}

void
audio_SourcePlay (audio_Object srcobj)
{
	rust_audio_source_play(srcobj);
}

void
audio_SourcePause (audio_Object srcobj)
{
	rust_audio_source_pause(srcobj);
}

void
audio_SourceStop (audio_Object srcobj)
{
	rust_audio_source_stop(srcobj);
}

void
audio_SourceQueueBuffers (audio_Object srcobj, uint32 n,
		audio_Object* pbufobj)
{
	rust_audio_source_queue_buffers(srcobj, n, pbufobj);
}

void
audio_SourceUnqueueBuffers (audio_Object srcobj, uint32 n,
		audio_Object* pbufobj)
{
	rust_audio_source_unqueue_buffers(srcobj, n, pbufobj);
}


/*
 * Buffers
 */

void
audio_GenBuffers (uint32 n, audio_Object *pbufobj)
{
	rust_audio_gen_buffers(n, pbufobj);
}

void
audio_DeleteBuffers (uint32 n, audio_Object *pbufobj)
{
	rust_audio_delete_buffers(n, pbufobj);
}

bool
audio_IsBuffer (audio_Object bufobj)
{
	return rust_audio_is_buffer(bufobj) != 0;
}

void
audio_GetBufferi (audio_Object bufobj, audio_BufferProp pname,
		audio_IntVal *value)
{
	rust_audio_get_buffer_i(bufobj, pname, value);
}

void
audio_BufferData (audio_Object bufobj, uint32 format, void* data,
		uint32 size, uint32 freq)
{
	rust_audio_buffer_data(bufobj, format, data, size, freq);
}

bool
audio_GetFormatInfo (uint32 format, int *channels, int *sample_size)
{
	switch (format)
	{
	case audio_FORMAT_MONO8:
		*channels = 1;
		*sample_size = sizeof (uint8);
		return true;

	case audio_FORMAT_STEREO8:
		*channels = 2;
		*sample_size = sizeof (uint8);
		return true;
	
	case audio_FORMAT_MONO16:
		*channels = 1;
		*sample_size = sizeof (sint16);
		return true;

	case audio_FORMAT_STEREO16:
		*channels = 2;
		*sample_size = sizeof (sint16);
		return true;
	}
	return false;
}

#endif /* USE_RUST_AUDIO */
