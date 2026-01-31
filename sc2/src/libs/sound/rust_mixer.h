/*
 *  Rust Mixer FFI bindings header
 *  Used when USE_RUST_MIXER is defined.
 */

#ifndef RUST_MIXER_H
#define RUST_MIXER_H

#ifdef USE_RUST_MIXER

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Mixer object handle type */
typedef intptr_t mixer_Object;
typedef intptr_t mixer_IntVal;

/* Mixer formats - must match C mixer.h */
#define MIX_FORMAT_DUMMYID     0x00170000
#define MIX_FORMAT_BPC(f)      ((f) & 0xff)
#define MIX_FORMAT_CHANS(f)    (((f) >> 8) & 0xff)
#define MIX_FORMAT_BPC_MAX     2
#define MIX_FORMAT_CHANS_MAX   2
#define MIX_FORMAT_MAKE(b, c) \
		( MIX_FORMAT_DUMMYID | ((b) & 0xff) | (((c) & 0xff) << 8) )
#define MIX_FORMAT_SAMPSIZE(f) \
		( MIX_FORMAT_BPC(f) * MIX_FORMAT_CHANS(f) )

typedef enum
{
	MIX_FORMAT_MONO8 = MIX_FORMAT_MAKE (1, 1),
	MIX_FORMAT_STEREO8 = MIX_FORMAT_MAKE (1, 2),
	MIX_FORMAT_MONO16 = MIX_FORMAT_MAKE (2, 1),
	MIX_FORMAT_STEREO16 = MIX_FORMAT_MAKE (2, 2)
} mixer_Format;

/* Mixer errors (see OpenAL errors) */
enum
{
	MIX_NO_ERROR = 0,
	MIX_INVALID_NAME = 0xA001U,
	MIX_INVALID_ENUM = 0xA002U,
	MIX_INVALID_VALUE = 0xA003U,
	MIX_INVALID_OPERATION = 0xA004U,
	MIX_OUT_OF_MEMORY = 0xA005U,
	MIX_DRIVER_FAILURE = 0xA101U
};

/* Source properties (see OpenAL) */
typedef enum
{
	MIX_POSITION = 0x1004,
	MIX_LOOPING = 0x1007,
	MIX_BUFFER = 0x1009,
	MIX_GAIN = 0x100A,
	MIX_SOURCE_STATE = 0x1010,
	MIX_BUFFERS_QUEUED = 0x1015,
	MIX_BUFFERS_PROCESSED = 0x1016
} mixer_SourceProp;

/* Source state information */
typedef enum
{
	MIX_INITIAL = 0,
	MIX_STOPPED,
	MIX_PLAYING,
	MIX_PAUSED,
} mixer_SourceState;

/* Sound buffer properties */
typedef enum
{
	MIX_FREQUENCY = 0x2001,
	MIX_BITS = 0x2002,
	MIX_CHANNELS = 0x2003,
	MIX_SIZE = 0x2004,
	MIX_DATA = 0x2005
} mixer_BufferProp;

/* Buffer states */
typedef enum
{
	MIX_BUF_INITIAL = 0,
	MIX_BUF_FILLED,
	MIX_BUF_QUEUED,
	MIX_BUF_PLAYING,
	MIX_BUF_PROCESSED
} mixer_BufferState;

/* Quality */
typedef enum
{
	MIX_QUALITY_LOW = 0,
	MIX_QUALITY_MEDIUM,
	MIX_QUALITY_HIGH,
	MIX_QUALITY_DEFAULT = MIX_QUALITY_MEDIUM,
	MIX_QUALITY_COUNT
} mixer_Quality;

/* Flags */
typedef enum
{
	MIX_NOFLAGS = 0,
	MIX_FAKE_DATA = 1
} mixer_Flags;

/* Endian flags needed by audiodrv_sdl.c */
#ifdef WORDS_BIGENDIAN
#	define MIX_IS_BIG_ENDIAN   true
#	define MIX_WANT_BIG_ENDIAN true
#else
#	define MIX_IS_BIG_ENDIAN   false
#	define MIX_WANT_BIG_ENDIAN false
#endif

/* Rust FFI function declarations */
int rust_mixer_Init(unsigned int frequency, unsigned int format, unsigned int quality, unsigned int flags);
void rust_mixer_Uninit(void);
unsigned int rust_mixer_GetError(void);

void rust_mixer_GenSources(unsigned int n, mixer_Object *psrcobj);
void rust_mixer_DeleteSources(unsigned int n, mixer_Object *psrcobj);
int rust_mixer_IsSource(mixer_Object srcobj);

void rust_mixer_Sourcei(mixer_Object srcobj, unsigned int property, mixer_IntVal value);
void rust_mixer_Sourcef(mixer_Object srcobj, unsigned int property, float value);
void rust_mixer_Sourcefv(mixer_Object srcobj, unsigned int property, float *value);
void rust_mixer_GetSourcei(mixer_Object srcobj, unsigned int property, mixer_IntVal *value);
void rust_mixer_GetSourcef(mixer_Object srcobj, unsigned int property, float *value);

void rust_mixer_SourcePlay(mixer_Object srcobj);
void rust_mixer_SourcePause(mixer_Object srcobj);
void rust_mixer_SourceStop(mixer_Object srcobj);
void rust_mixer_SourceRewind(mixer_Object srcobj);

void rust_mixer_SourceQueueBuffers(mixer_Object srcobj, unsigned int n, mixer_Object *pbufobj);
void rust_mixer_SourceUnqueueBuffers(mixer_Object srcobj, unsigned int n, mixer_Object *pbufobj);

void rust_mixer_GenBuffers(unsigned int n, mixer_Object *pbufobj);
void rust_mixer_DeleteBuffers(unsigned int n, mixer_Object *pbufobj);
int rust_mixer_IsBuffer(mixer_Object bufobj);

void rust_mixer_BufferData(mixer_Object bufobj, unsigned int format, const void *data, unsigned int size, unsigned int freq);
void rust_mixer_GetBufferi(mixer_Object bufobj, unsigned int property, mixer_IntVal *value);

void rust_mixer_MixChannels(void *userdata, unsigned char *stream, int len);
void rust_mixer_MixFake(void *userdata, unsigned char *stream, int len);

unsigned int rust_mixer_GetFrequency(void);
unsigned int rust_mixer_GetFormat(void);

/* Redirect mixer_* calls to rust_mixer_* */
#define mixer_Init(freq, fmt, qual, flags) rust_mixer_Init((freq), (fmt), (qual), (flags))
#define mixer_Uninit() rust_mixer_Uninit()
#define mixer_GetError() rust_mixer_GetError()

#define mixer_GenSources(n, p) rust_mixer_GenSources((n), (p))
#define mixer_DeleteSources(n, p) rust_mixer_DeleteSources((n), (p))
#define mixer_IsSource(s) rust_mixer_IsSource((s))

#define mixer_Sourcei(s, pn, v) rust_mixer_Sourcei((s), (pn), (v))
#define mixer_Sourcef(s, pn, v) rust_mixer_Sourcef((s), (pn), (v))
#define mixer_Sourcefv(s, pn, v) rust_mixer_Sourcefv((s), (pn), (v))
#define mixer_GetSourcei(s, pn, v) rust_mixer_GetSourcei((s), (pn), (v))
#define mixer_GetSourcef(s, pn, v) rust_mixer_GetSourcef((s), (pn), (v))

#define mixer_SourcePlay(s) rust_mixer_SourcePlay((s))
#define mixer_SourcePause(s) rust_mixer_SourcePause((s))
#define mixer_SourceStop(s) rust_mixer_SourceStop((s))
#define mixer_SourceRewind(s) rust_mixer_SourceRewind((s))

#define mixer_SourceQueueBuffers(s, n, p) rust_mixer_SourceQueueBuffers((s), (n), (p))
#define mixer_SourceUnqueueBuffers(s, n, p) rust_mixer_SourceUnqueueBuffers((s), (n), (p))

#define mixer_GenBuffers(n, p) rust_mixer_GenBuffers((n), (p))
#define mixer_DeleteBuffers(n, p) rust_mixer_DeleteBuffers((n), (p))
#define mixer_IsBuffer(b) rust_mixer_IsBuffer((b))

#define mixer_BufferData(b, fmt, d, sz, f) rust_mixer_BufferData((b), (fmt), (d), (sz), (f))
#define mixer_GetBufferi(b, pn, v) rust_mixer_GetBufferi((b), (pn), (v))

#define mixer_MixChannels(ud, s, l) rust_mixer_MixChannels((ud), (s), (l))
#define mixer_MixFake(ud, s, l) rust_mixer_MixFake((ud), (s), (l))

#ifdef __cplusplus
}
#endif

#endif /* USE_RUST_MIXER */

#endif /* RUST_MIXER_H */
