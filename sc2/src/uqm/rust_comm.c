/*
 *  Rust Communication System wrapper
 *  
 *  Wraps Rust-implemented communication functions when USE_RUST_COMM is defined.
 *  This file provides C-callable wrapper functions that delegate to the Rust
 *  implementation via the FFI bindings declared in rust_comm.h.
 *
 *  @plan PLAN-20260314-COMM.P05b
 */

#include <string.h>
#include <stdint.h>

#define COMM_INTERNAL
#include "comm.h"

#ifdef USE_RUST_COMM
#include "rust_comm.h"

/* Initialize communication system using Rust implementation */
void
init_communication (void)
{
	rust_InitCommunication ();
}

/* Uninitialize communication system using Rust implementation */
void
uninit_communication (void)
{
	rust_UninitCommunication ();
}

/*
 * ============================================================================
 *  Trackplayer C-side wrapper seam (@plan PLAN-20260314-COMM.P05b)
 *
 *  Thin wrappers around the authoritative C trackplayer in
 *  sc2/src/libs/sound/trackplayer.c.  Rust comm calls these via FFI
 *  so it never depends on legacy symbol shapes directly.
 * ============================================================================
 */

#include "libs/sound/trackplayer.h"
#include "commanim.h"  /* ANIMATION_DESC, MAX_ANIMATIONS */

/*
 * ============================================================================
 *  LOCDATA field accessor seam (P03 gap-fill, @plan PLAN-20260314-COMM.P03)
 *
 *  Thin accessors so Rust can read LOCDATA fields without knowing the
 *  struct layout.  The LOCDATA pointer is passed as void* from Rust.
 * ============================================================================
 */

typedef void (*VoidFunc)(void);
typedef COUNT (*CountFunc)(void);

/* Pack a Color (RGBA each u8) into a uint32 for FFI */
static uint32_t
color_to_u32 (Color c)
{
	return ((uint32_t)c.r << 24) | ((uint32_t)c.g << 16)
			| ((uint32_t)c.b << 8) | (uint32_t)c.a;
}

/* Copy an ANIMATION_DESC into the Rust-side AnimationDescData layout */
static void
anim_desc_to_ffi (const ANIMATION_DESC *src, void *out)
{
	/* AnimationDescData layout (repr(C)):
	 *   u16 start_index, u8 num_frames, u8 anim_flags,
	 *   u16 base_frame_rate, u16 random_frame_rate,
	 *   u16 base_restart_rate, u16 random_restart_rate,
	 *   u32 block_mask
	 */
	unsigned char *p = (unsigned char *)out;
	uint16_t u16v;
	uint32_t u32v;

	u16v = (uint16_t)src->StartIndex;
	memcpy (p, &u16v, 2); p += 2;
	*p++ = (unsigned char)src->NumFrames;
	*p++ = (unsigned char)src->AnimFlags;
	u16v = (uint16_t)src->BaseFrameRate;
	memcpy (p, &u16v, 2); p += 2;
	u16v = (uint16_t)src->RandomFrameRate;
	memcpy (p, &u16v, 2); p += 2;
	u16v = (uint16_t)src->BaseRestartRate;
	memcpy (p, &u16v, 2); p += 2;
	u16v = (uint16_t)src->RandomRestartRate;
	memcpy (p, &u16v, 2); p += 2;
	u32v = (uint32_t)src->BlockMask;
	memcpy (p, &u32v, 4);
}

VoidFunc
c_locdata_get_init_func (const void *locdata)
{
	return (VoidFunc)((const LOCDATA *)locdata)->init_encounter_func;
}

VoidFunc
c_locdata_get_post_func (const void *locdata)
{
	return (VoidFunc)((const LOCDATA *)locdata)->post_encounter_func;
}

CountFunc
c_locdata_get_uninit_func (const void *locdata)
{
	return ((const LOCDATA *)locdata)->uninit_encounter_func;
}

const char *
c_locdata_get_alien_frame_res (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienFrameRes;
}

const char *
c_locdata_get_alien_font_res (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienFontRes;
}

const char *
c_locdata_get_alien_colormap_res (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienColorMapRes;
}

const char *
c_locdata_get_alien_song_res (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienSongRes;
}

const char *
c_locdata_get_alien_alt_song_res (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienAltSongRes;
}

const char *
c_locdata_get_conversation_phrases_res (const void *locdata)
{
	return ((const LOCDATA *)locdata)->ConversationPhrasesRes;
}

uint32_t
c_locdata_get_text_fcolor (const void *locdata)
{
	return color_to_u32 (((const LOCDATA *)locdata)->AlienTextFColor);
}

uint32_t
c_locdata_get_text_bcolor (const void *locdata)
{
	return color_to_u32 (((const LOCDATA *)locdata)->AlienTextBColor);
}

int16_t
c_locdata_get_text_baseline_x (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienTextBaseline.x;
}

int16_t
c_locdata_get_text_baseline_y (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienTextBaseline.y;
}

uint16_t
c_locdata_get_text_width (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienTextWidth;
}

uint32_t
c_locdata_get_text_align (const void *locdata)
{
	return (uint32_t)((const LOCDATA *)locdata)->AlienTextAlign;
}

uint32_t
c_locdata_get_text_valign (const void *locdata)
{
	return (uint32_t)((const LOCDATA *)locdata)->AlienTextValign;
}

uint32_t
c_locdata_get_song_flags (const void *locdata)
{
	return (uint32_t)((const LOCDATA *)locdata)->AlienSongFlags;
}

uint32_t
c_locdata_get_num_animations (const void *locdata)
{
	return (uint32_t)((const LOCDATA *)locdata)->NumAnimations;
}

void
c_locdata_get_ambient_anim (const void *locdata, uint32_t index, void *out)
{
	const LOCDATA *ld = (const LOCDATA *)locdata;
	if (index < (uint32_t)ld->NumAnimations && index < MAX_ANIMATIONS)
		anim_desc_to_ffi (&ld->AlienAmbientArray[index], out);
}

void
c_locdata_get_transition_desc (const void *locdata, void *out)
{
	anim_desc_to_ffi (&((const LOCDATA *)locdata)->AlienTransitionDesc, out);
}

void
c_locdata_get_talk_desc (const void *locdata, void *out)
{
	anim_desc_to_ffi (&((const LOCDATA *)locdata)->AlienTalkDesc, out);
}

const void *
c_locdata_get_number_speech (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienNumberSpeech;
}

void *
c_locdata_get_alien_frame (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienFrame;
}

void *
c_locdata_get_alien_font (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienFont;
}

void *
c_locdata_get_alien_colormap (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienColorMap;
}

void *
c_locdata_get_alien_song (const void *locdata)
{
	return ((const LOCDATA *)locdata)->AlienSong;
}

void *
c_locdata_get_conversation_phrases (const void *locdata)
{
	return ((const LOCDATA *)locdata)->ConversationPhrases;
}

/*
 * ============================================================================
 *  Glue-layer accessors (P04, @plan PLAN-20260314-COMM.P04)
 *
 *  Let Rust resolve conversation phrases, player name, ship name,
 *  alliance name without depending on UQM global layout.
 * ============================================================================
 */

#include "globdata.h"
#include "commglue.h"

/* Forward decl — init_race lives in commglue.c */
extern LOCDATA *init_race (CONVERSATION comm_id);

const void *
c_init_race (int comm_id)
{
	return init_race ((CONVERSATION)comm_id);
}

const unsigned char *
c_get_conversation_phrase (const void *phrases, int index)
{
	STRING s;
	if (!phrases || index <= 0)
		return NULL;
	s = SetAbsStringTableIndex ((STRING)phrases, index - 1);
	return (const unsigned char *)GetStringAddress (s);
}

const unsigned char *
c_get_commander_name (void)
{
	return (const unsigned char *)GLOBAL_SIS (CommanderName);
}

const unsigned char *
c_get_ship_name (void)
{
	return (const unsigned char *)GLOBAL_SIS (ShipName);
}

const unsigned char *
c_get_alliance_name (int index)
{
	/* Alliance name variants live in CommData.ConversationPhrases
	 * at negative indices offset from GLOBAL_ALLIANCE_NAME.
	 * For the bridge, just return the raw phrase text —
	 * commander-name concatenation is done in Rust's construct_response. */
	COUNT i;
	STRING S;

	i = GET_GAME_STATE (NEW_ALLIANCE_NAME);
	S = SetAbsStringTableIndex (CommData.ConversationPhrases, index + i);
	return (const unsigned char *)GetStringAddress (S);
}

void
c_SpliceTrack (UNICODE *filespec, UNICODE *textspec,
		UNICODE *timestamp, CallbackFunction cb)
{
	SpliceTrack (filespec, textspec, timestamp, cb);
}

void
c_SpliceMultiTrack (UNICODE *track_names[], UNICODE *track_text)
{
	SpliceMultiTrack (track_names, track_text);
}

void
c_PlayTrack (void)
{
	PlayTrack ();
}

void
c_StopTrack (void)
{
	StopTrack ();
}

void
c_JumpTrack (void)
{
	JumpTrack ();
}

COUNT
c_PlayingTrack (void)
{
	return PlayingTrack ();
}

void
c_PauseTrack (void)
{
	PauseTrack ();
}

void
c_ResumeTrack (void)
{
	ResumeTrack ();
}

const UNICODE *
c_GetTrackSubtitle (void)
{
	return GetTrackSubtitle ();
}

SUBTITLE_REF
c_GetFirstTrackSubtitle (void)
{
	return GetFirstTrackSubtitle ();
}

SUBTITLE_REF
c_GetNextTrackSubtitle (SUBTITLE_REF last_ref)
{
	return GetNextTrackSubtitle (last_ref);
}

const UNICODE *
c_GetTrackSubtitleText (SUBTITLE_REF sub_ref)
{
	return GetTrackSubtitleText (sub_ref);
}

void
c_FastForward_Page (void)
{
	FastForward_Page ();
}

void
c_FastForward_Smooth (void)
{
	FastForward_Smooth ();
}

void
c_FastReverse_Page (void)
{
	FastReverse_Page ();
}

void
c_FastReverse_Smooth (void)
{
	FastReverse_Smooth ();
}

int
c_GetTrackPosition (int in_units)
{
	return GetTrackPosition (in_units);
}

#endif /* USE_RUST_COMM */
