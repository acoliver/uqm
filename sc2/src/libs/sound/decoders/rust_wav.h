/*
 *  Rust WAV decoder header
 *  
 *  Provides extern declaration for the Rust-implemented WAV decoder
 *  vtable. When USE_RUST_WAV is defined, this decoder is used instead of
 *  the C implementation.
 */

#ifndef LIBS_SOUND_DECODERS_RUST_WAV_H_
#define LIBS_SOUND_DECODERS_RUST_WAV_H_

#include "decoder.h"

#ifdef USE_RUST_WAV

/*
 * Rust WAV decoder vtable
 * Defined in rust/src/sound/wav_ffi.rs and exported via staticlib
 */
extern TFB_SoundDecoderFuncs rust_wav_DecoderVtbl;

#endif /* USE_RUST_WAV */

#endif /* LIBS_SOUND_DECODERS_RUST_WAV_H_ */
