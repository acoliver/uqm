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
#include "libs/sndlib.h"

/*
 * New Rust FFI (pure Rust player). This wrapper keeps the existing C-facing
 * vidplayer API intact.
 */
extern bool rust_play_video (uio_DirHandle *dir, const char *filename,
		uint32 x, uint32 y, bool looping);
extern bool rust_play_video_direct_window (uio_DirHandle *dir, const char *filename,
		uint32 window_width, uint32 window_height, bool looping);
extern void rust_stop_video (void);
extern bool rust_video_playing (void);
extern bool rust_process_video_frame (void);
extern uint32 rust_get_video_position (void);

bool
TFB_InitVideoPlayer (void)
{
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
	bool ok;

	if (!vid || !vid->decoder || !vid->decoder->filename)
		return false;

	log_add (log_Info, "RUST_VIDEO: TFB_PlayVideo %s", vid->decoder->filename);
	
	// Check if we have access to actual window dimensions for direct presentation
	extern int ScreenWidthActual, ScreenHeightActual;
	uint32 actual_width = (uint32)ScreenWidthActual;
	uint32 actual_height = (uint32)ScreenHeightActual;
	
	log_add (log_Info, "RUST_VIDEO: Using direct window presentation (actual %ux%u)",
				 actual_width, actual_height);
	// Call the enhanced Rust player that uses direct window presentation
	ok = rust_play_video_direct_window (vid->decoder->dir, vid->decoder->filename,
					     actual_width, actual_height,
					     vid->loop_frame != VID_NO_LOOP);
	
	log_add (log_Info, "RUST_VIDEO: rust_play_video_direct_window returned %d", ok);
	if (!ok)
		return false;
	
	// Start associated audio (if any) in the main audio path.
	if (vid->hAudio)
		PLRPlaySong (vid->hAudio, vid->loop_frame != VID_NO_LOOP, 1);
	if (vid->data)
		snd_PlaySpeech ((MUSIC_REF) vid->data);
	return true;
}

void
TFB_StopVideo (VIDEO_REF vid)
{
	if (vid && vid->hAudio)
		PLRStop (vid->hAudio);
	if (vid && vid->data)
		snd_StopSpeech ();
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
	bool ok;

	(void)vid;
	ok = rust_process_video_frame ();
	log_add (log_Info, "RUST_VIDEO: rust_process_video_frame -> %d", ok);
	return ok;
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
