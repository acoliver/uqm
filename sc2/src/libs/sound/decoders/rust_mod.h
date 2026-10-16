/*
 * Rust MOD decoder header
 *
 * Declares the Rust MOD decoder vtable for tracker music playback.
 */

#ifndef RUST_MOD_H
#define RUST_MOD_H

#include "decoder.h"

/* The Rust MOD decoder vtable - defined in rust/src/sound/mod_ffi.rs */
extern TFB_SoundDecoderFuncs rust_mod_DecoderVtbl;

#endif /* RUST_MOD_H */
