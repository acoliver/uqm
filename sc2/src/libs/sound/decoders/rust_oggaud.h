/*
 *  Rust Ogg Vorbis decoder header
 *  
 *  Provides extern declaration for the Rust-implemented Ogg Vorbis decoder
 *  vtable. When USE_RUST_OGG is defined, this decoder is used instead of
 *  the C libvorbis/tremor implementation.
 */

#ifndef LIBS_SOUND_DECODERS_RUST_OGGAUD_H_
#define LIBS_SOUND_DECODERS_RUST_OGGAUD_H_

#include "decoder.h"

#ifdef USE_RUST_OGG

/*
 * Rust Ogg Vorbis decoder vtable
 * Defined in rust/src/sound/ffi.rs and exported via staticlib
 */
extern TFB_SoundDecoderFuncs rust_ova_DecoderVtbl;

#endif /* USE_RUST_OGG */

#endif /* LIBS_SOUND_DECODERS_RUST_OGGAUD_H_ */
