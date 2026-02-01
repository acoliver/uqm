/*
 *  Rust Video Player wrapper
 *
 *  When USE_RUST_VIDEO is defined, this file routes the legacy C video-player
 *  API (vidplayer.c) into the pure Rust video player.
 */

#ifdef USE_RUST_VIDEO

#include "video.h"
#include "vidplayer.h"
#include "vidintrn.h"
#include "rust_video.h"
#include "libs/log.h"

/*
 * New Rust FFI (pure Rust player). This wrapper keeps the existing C-facing
 * vidplayer API intact.
 */
extern bool rust_play_video (uio_DirHandle *dir, const char *filename,
		uint32 x, uint32 y, bool looping);
extern void rust_stop_video (void);
extern bool rust_video_playing (void);
extern bool rust_process_video_frame (void);
extern uint32 rust_get_video_position (void);

bool
TFB_InitVideoPlayer (void)
{
	/* Rust video player requires no initialization. */
	return true;
}

void
TFB_UninitVideoPlayer (void)
{
	/* Clean up any playing video. */
	rust_stop_video ();
}

bool
TFB_PlayVideo (VIDEO_REF vid, uint32 x, uint32 y)
{
	if (!vid || !vid->decoder || !vid->decoder->filename)
		return false;

	return rust_play_video (vid->decoder->dir, vid->decoder->filename, x, y,
			vid->loop_frame != VID_NO_LOOP);
}

void
TFB_StopVideo (VIDEO_REF vid)
{
	(void)vid;
	rust_stop_video ();
}

bool
TFB_VideoPlaying (VIDEO_REF vid)
{
	(void)vid;
	return rust_video_playing ();
}

bool
TFB_ProcessVideoFrame (VIDEO_REF vid)
{
	(void)vid;
	return rust_process_video_frame ();
}

uint32
TFB_GetVideoPosition (VIDEO_REF vid)
{
	(void)vid;
	return rust_get_video_position ();
}

bool
TFB_SeekVideo (VIDEO_REF vid, uint32 pos)
{
	(void)vid;
	(void)pos;
	/* Not implemented by pure Rust player yet. */
	return false;
}

#endif /* USE_RUST_VIDEO */
