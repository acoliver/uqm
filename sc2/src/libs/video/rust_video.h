/*
 *  Rust Video Player header
 *  
 *  Provides extern declarations for the Rust-implemented video player.
 *  When USE_RUST_VIDEO is defined, this player is used instead of
 *  the C DukVid implementation.
 */

#ifndef LIBS_VIDEO_RUST_VIDEO_H_
#define LIBS_VIDEO_RUST_VIDEO_H_

#include "types.h"
#include <stddef.h>

#ifdef USE_RUST_VIDEO

#include "libs/uio.h"

/*
 * Pure Rust video player FFI.
 * Implemented in rust/src/video/ffi.rs.
 */
extern bool rust_play_video (uio_DirHandle *dir, const char *filename,
		uint32 x, uint32 y, bool looping);
extern void rust_stop_video (void);
extern bool rust_video_playing (void);
extern bool rust_process_video_frame (void);
extern uint32 rust_get_video_position (void);

#endif /* USE_RUST_VIDEO */

#endif /* LIBS_VIDEO_RUST_VIDEO_H_ */
