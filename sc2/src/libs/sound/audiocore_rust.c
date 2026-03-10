/*
 * Rust Audio Backend - implements audiocore API using the Rust mixer
 *
 * When USE_RUST_AUDIO is defined, this file provides the audiocore
 * functions by calling into the Rust mixer (rust_mixer_*).
 * Audio output is handled by the mixer pump in stream.rs (rodio).
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
 * Enum translation table: maps audio_* enum indices to MIX_* values.
 * audio_* enums (from audiocore.h) are sequential integers starting at 0.
 * MIX_* values (from mixer.h) are the OpenAL-like hex constants.
 * Index order MUST match the audio_* enum declaration in audiocore.h.
 */
#define MIX_FORMAT_DUMMYID     0x00170000
#define MIX_FORMAT_MAKE_LOCAL(b, c) \
		( MIX_FORMAT_DUMMYID | ((b) & 0xff) | (((c) & 0xff) << 8) )

static const sint32 EnumLookup[audio_ENUM_SIZE] =
{
	/* Errors */
	0,          /* audio_NO_ERROR */
	0xA001,     /* audio_INVALID_NAME */
	0xA002,     /* audio_INVALID_ENUM */
	0xA003,     /* audio_INVALID_VALUE */
	0xA004,     /* audio_INVALID_OPERATION */
	0xA005,     /* audio_OUT_OF_MEMORY */
	0xA101,     /* audio_DRIVER_FAILURE */

	/* Source properties */
	0x1004,     /* audio_POSITION -> MIX_POSITION */
	0x1007,     /* audio_LOOPING -> MIX_LOOPING */
	0x1009,     /* audio_BUFFER -> MIX_BUFFER */
	0x100A,     /* audio_GAIN -> MIX_GAIN */
	0x1010,     /* audio_SOURCE_STATE -> MIX_SOURCE_STATE */
	0x1015,     /* audio_BUFFERS_QUEUED -> MIX_BUFFERS_QUEUED */
	0x1016,     /* audio_BUFFERS_PROCESSED -> MIX_BUFFERS_PROCESSED */

	/* Source state information */
	0,          /* audio_INITIAL -> MIX_INITIAL */
	1,          /* audio_STOPPED -> MIX_STOPPED */
	2,          /* audio_PLAYING -> MIX_PLAYING */
	3,          /* audio_PAUSED -> MIX_PAUSED */

	/* Sound buffer properties */
	0x2001,     /* audio_FREQUENCY -> MIX_FREQUENCY */
	0x2002,     /* audio_BITS -> MIX_BITS */
	0x2003,     /* audio_CHANNELS -> MIX_CHANNELS */
	0x2004,     /* audio_SIZE -> MIX_SIZE */

	/* Format constants */
	(sint32) MIX_FORMAT_MAKE_LOCAL(2, 1),  /* audio_FORMAT_MONO16 */
	(sint32) MIX_FORMAT_MAKE_LOCAL(2, 2),  /* audio_FORMAT_STEREO16 */
	(sint32) MIX_FORMAT_MAKE_LOCAL(1, 1),  /* audio_FORMAT_MONO8 */
	(sint32) MIX_FORMAT_MAKE_LOCAL(1, 2),  /* audio_FORMAT_STEREO8 */
};

/*
 * Rust mixer FFI declarations
 *
 * mixer_Object is intptr_t; audio_Object is uintptr_t.
 * Both are pointer-sized — safe to cast between them.
 */
typedef intptr_t mixer_Object;

extern int rust_mixer_Init(unsigned int frequency, unsigned int format,
		unsigned int quality, unsigned int flags);
extern void rust_mixer_Uninit(void);
extern unsigned int rust_mixer_GetError(void);
extern void rust_mixer_GenSources(unsigned int n, mixer_Object *psrcobj);
extern void rust_mixer_DeleteSources(unsigned int n, mixer_Object *psrcobj);
extern int rust_mixer_IsSource(mixer_Object srcobj);
extern void rust_mixer_Sourcei(mixer_Object srcobj, unsigned int pname, intptr_t value);
extern void rust_mixer_Sourcef(mixer_Object srcobj, unsigned int pname, float value);
extern void rust_mixer_Sourcefv(mixer_Object srcobj, unsigned int pname, float *value);
extern void rust_mixer_GetSourcei(mixer_Object srcobj, unsigned int pname, intptr_t *value);
extern void rust_mixer_GetSourcef(mixer_Object srcobj, unsigned int pname, float *value);
extern void rust_mixer_SourceRewind(mixer_Object srcobj);
extern void rust_mixer_SourcePlay(mixer_Object srcobj);
extern void rust_mixer_SourcePause(mixer_Object srcobj);
extern void rust_mixer_SourceStop(mixer_Object srcobj);
extern void rust_mixer_SourceQueueBuffers(mixer_Object srcobj, unsigned int n,
		mixer_Object *pbufobj);
extern void rust_mixer_SourceUnqueueBuffers(mixer_Object srcobj, unsigned int n,
		mixer_Object *pbufobj);
extern void rust_mixer_GenBuffers(unsigned int n, mixer_Object *pbufobj);
extern void rust_mixer_DeleteBuffers(unsigned int n, mixer_Object *pbufobj);
extern int rust_mixer_IsBuffer(mixer_Object bufobj);
extern void rust_mixer_GetBufferi(mixer_Object bufobj, unsigned int pname,
		intptr_t *value);
extern void rust_mixer_BufferData(mixer_Object bufobj, unsigned int format,
		void *data, unsigned int size, unsigned int freq);


/*
 * Initialization
 */

sint32
initAudio (sint32 driver, sint32 flags)
{
	sint32 ret;
	TFB_DecoderFormats formats;
	unsigned int mixer_format;

	log_add (log_Info, "initAudio: Using Rust mixer backend");

	(void)driver; /* We always use the Rust mixer */

	/*
	 * Initialize the Rust mixer: 44100 Hz, stereo 16-bit, medium quality.
	 * The mixer_format encoding matches MIX_FORMAT_MAKE(2, 2):
	 *   bytes_per_channel=2, channels=2 → (2 << 8) | 2 = 0x0202
	 */
	if (flags & audio_QUALITY_HIGH)
		mixer_format = 0x0202; /* stereo 16-bit */
	else if (flags & audio_QUALITY_LOW)
		mixer_format = 0x0202; /* stereo 16-bit (same for now) */
	else
		mixer_format = 0x0202; /* stereo 16-bit default */

	ret = rust_mixer_Init (44100, mixer_format, 1 /* medium */, 0 /* none */);

	if (ret == 0)
	{
		log_add (log_Fatal, "Rust mixer initialization failed.\n");
		exit (EXIT_FAILURE);
	}

	/* Initialize sound decoders */
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
		rust_mixer_Uninit();
		return -1;
	}
	log_add (log_Info, "Sound decoders initialized.");

	/* Initialize soundSource array — generate mixer handles and create mutexes */
	{
		int i;
		for (i = 0; i < NUM_SOUNDSOURCES; ++i)
		{
			audio_GenSources (1, &soundSource[i].handle);
			soundSource[i].stream_mutex = CreateMutex (
					"Rust mixer stream mutex", SYNC_CLASS_AUDIO);
		}
	}

	/* Initialize stream decoder thread */
	if (InitStreamDecoder ())
	{
		log_add (log_Error, "Stream decoder initialization failed.");
		rust_mixer_Uninit();
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
	rust_mixer_Uninit();
}


/*
 * General
 */

sint32
audio_GetError (void)
{
	return (sint32) rust_mixer_GetError();
}


/*
 * Sources — cast audio_Object (uintptr_t) ↔ mixer_Object (intptr_t)
 */

void
audio_GenSources (uint32 n, audio_Object *psrcobj)
{
	rust_mixer_GenSources(n, (mixer_Object *) psrcobj);
}

void
audio_DeleteSources (uint32 n, audio_Object *psrcobj)
{
	rust_mixer_DeleteSources(n, (mixer_Object *) psrcobj);
}

bool
audio_IsSource (audio_Object srcobj)
{
	return rust_mixer_IsSource((mixer_Object) srcobj) != 0;
}

void
audio_Sourcei (audio_Object srcobj, audio_SourceProp pname,
		audio_IntVal value)
{
	rust_mixer_Sourcei((mixer_Object) srcobj, EnumLookup[pname], value);
}

void
audio_Sourcef (audio_Object srcobj, audio_SourceProp pname,
		float value)
{
	rust_mixer_Sourcef((mixer_Object) srcobj, EnumLookup[pname], value);
}

void
audio_Sourcefv (audio_Object srcobj, audio_SourceProp pname,
		float *value)
{
	rust_mixer_Sourcefv((mixer_Object) srcobj, EnumLookup[pname], value);
}

void
audio_GetSourcei (audio_Object srcobj, audio_SourceProp pname,
		audio_IntVal *value)
{
	rust_mixer_GetSourcei((mixer_Object) srcobj, EnumLookup[pname], (intptr_t *) value);
}

void
audio_GetSourcef (audio_Object srcobj, audio_SourceProp pname,
		float *value)
{
	rust_mixer_GetSourcef((mixer_Object) srcobj, EnumLookup[pname], value);
}

void
audio_SourceRewind (audio_Object srcobj)
{
	rust_mixer_SourceRewind((mixer_Object) srcobj);
}

void
audio_SourcePlay (audio_Object srcobj)
{
	rust_mixer_SourcePlay((mixer_Object) srcobj);
}

void
audio_SourcePause (audio_Object srcobj)
{
	rust_mixer_SourcePause((mixer_Object) srcobj);
}

void
audio_SourceStop (audio_Object srcobj)
{
	rust_mixer_SourceStop((mixer_Object) srcobj);
}

void
audio_SourceQueueBuffers (audio_Object srcobj, uint32 n,
		audio_Object* pbufobj)
{
	rust_mixer_SourceQueueBuffers((mixer_Object) srcobj, n,
			(mixer_Object *) pbufobj);
}

void
audio_SourceUnqueueBuffers (audio_Object srcobj, uint32 n,
		audio_Object* pbufobj)
{
	rust_mixer_SourceUnqueueBuffers((mixer_Object) srcobj, n,
			(mixer_Object *) pbufobj);
}


/*
 * Buffers
 */

void
audio_GenBuffers (uint32 n, audio_Object *pbufobj)
{
	rust_mixer_GenBuffers(n, (mixer_Object *) pbufobj);
}

void
audio_DeleteBuffers (uint32 n, audio_Object *pbufobj)
{
	rust_mixer_DeleteBuffers(n, (mixer_Object *) pbufobj);
}

bool
audio_IsBuffer (audio_Object bufobj)
{
	return rust_mixer_IsBuffer((mixer_Object) bufobj) != 0;
}

void
audio_GetBufferi (audio_Object bufobj, audio_BufferProp pname,
		audio_IntVal *value)
{
	rust_mixer_GetBufferi((mixer_Object) bufobj, EnumLookup[pname], (intptr_t *) value);
}

void
audio_BufferData (audio_Object bufobj, uint32 format, void* data,
		uint32 size, uint32 freq)
{
	rust_mixer_BufferData((mixer_Object) bufobj, EnumLookup[format], data, size, freq);
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
