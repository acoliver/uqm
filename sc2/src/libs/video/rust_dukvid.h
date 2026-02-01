/*
 *  Rust DukVid decoder header
 *  
 *  Provides extern declaration for the Rust-implemented DukVid decoder
 *  vtable. When USE_RUST_VIDEO is defined, this decoder is used instead of
 *  the C implementation.
 */

#ifndef LIBS_VIDEO_RUST_DUKVID_H_
#define LIBS_VIDEO_RUST_DUKVID_H_

#include "videodec.h"

#ifdef USE_RUST_VIDEO

/*
 * Rust DukVid decoder vtable
 * Defined in rust/src/video/ffi.rs and exported via staticlib
 */
extern TFB_VideoDecoderFuncs rust_dukv_DecoderVtbl;

#endif /* USE_RUST_VIDEO */

#endif /* LIBS_VIDEO_RUST_DUKVID_H_ */
