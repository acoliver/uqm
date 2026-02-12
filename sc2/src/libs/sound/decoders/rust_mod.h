/*
 * Rust MOD decoder header
 *
 * Declares the Rust MOD decoder vtable for tracker music playback.
 * When USE_RUST_MOD is defined, this decoder is used instead of
 * the MikMod-based modaud.c implementation.
 */

#ifndef RUST_MOD_H
#define RUST_MOD_H

#include "decoder.h"

#ifdef USE_RUST_MOD

/* The Rust MOD decoder vtable - defined in rust/src/sound/mod_ffi.rs */
extern TFB_SoundDecoderFuncs rust_mod_DecoderVtbl;

#define moda_DecoderVtbl rust_mod_DecoderVtbl

#endif /* USE_RUST_MOD */

#endif /* RUST_MOD_H */
