/*
 *  Rust DukAud decoder header
 *  
 *  Provides extern declaration for the Rust-implemented DukAud decoder
 *  vtable. When USE_RUST_DUKAUD is defined, this decoder is used instead of
 *  the C dukaud implementation.
 */

#ifndef LIBS_SOUND_DECODERS_RUST_DUKAUD_H_
#define LIBS_SOUND_DECODERS_RUST_DUKAUD_H_

#include "decoder.h"

#ifdef USE_RUST_DUKAUD

/*
 * Rust DukAud decoder vtable
 * Defined in rust/src/sound/dukaud_ffi.rs and exported via staticlib
 */
extern TFB_SoundDecoderFuncs rust_duka_DecoderVtbl;

#endif /* USE_RUST_DUKAUD */

#endif /* LIBS_SOUND_DECODERS_RUST_DUKAUD_H_ */
