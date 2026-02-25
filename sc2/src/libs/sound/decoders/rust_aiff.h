/*
 *  Rust AIFF decoder header
 *  
 *  Provides extern declaration for the Rust-implemented AIFF decoder
 *  vtable. When USE_RUST_AIFF is defined, this decoder is used instead of
 *  the C aiffaud implementation.
 */

#ifndef LIBS_SOUND_DECODERS_RUST_AIFF_H_
#define LIBS_SOUND_DECODERS_RUST_AIFF_H_

#include "decoder.h"

#ifdef USE_RUST_AIFF

/*
 * Rust AIFF decoder vtable
 * Defined in rust/src/sound/aiff_ffi.rs and exported via staticlib
 */
extern TFB_SoundDecoderFuncs rust_aifa_DecoderVtbl;

#endif /* USE_RUST_AIFF */

#endif /* LIBS_SOUND_DECODERS_RUST_AIFF_H_ */
