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

/* Full alliance-name lookup matching C NPCPhrase_cb branch 4.
 * @plan PLAN-20260326-COMMPT2.P04 @requirement REQ-NP-001
 *
 * adjusted_index = phrase_id - GLOBAL_ALLIANCE_NAME (already done by caller).
 * Writes into buf (size buf_len) with optional CommanderName append (state==3).
 * Returns buf, or NULL if buf is too small or phrases unavailable.
 */
const unsigned char *
c_get_alliance_name_full (int adjusted_index, char *buf, int buf_len)
{
	COUNT i;
	STRING S;
	const UNICODE *src;

	if (!CommData.ConversationPhrases || !buf || buf_len <= 0)
		return NULL;

	i = GET_GAME_STATE (NEW_ALLIANCE_NAME);
	S = SetAbsStringTableIndex (CommData.ConversationPhrases,
			(adjusted_index - 1) + i);
	src = (const UNICODE *)GetStringAddress (S);
	if (!src)
		return NULL;

	strncpy (buf, src, (size_t)buf_len - 1);
	buf[buf_len - 1] = '\0';

	if (i == 3)
	{
		const UNICODE *cname = GLOBAL_SIS (CommanderName);
		if (cname)
		{
			size_t used = strlen (buf);
			strncat (buf + used, cname, (size_t)buf_len - used - 1);
			buf[buf_len - 1] = '\0';
		}
	}

	return (const unsigned char *)buf;
}

/* Return the sound-clip pointer for a phrase index (0-based into table).
 * @plan PLAN-20260326-COMMPT2.P04 @requirement REQ-NP-001
 */
void *
c_get_phrase_sound_clip (const void *phrases, int index)
{
	STRING S;
	if (!phrases || index < 0)
		return NULL;
	S = SetAbsStringTableIndex ((STRING_TABLE)phrases, index);
	return GetStringSoundClip (S);
}

/* Return the timestamp pointer for a phrase index (0-based into table).
 * @plan PLAN-20260326-COMMPT2.P04 @requirement REQ-NP-001
 */
void *
c_get_phrase_timestamp (const void *phrases, int index)
{
	STRING S;
	if (!phrases || index < 0)
		return NULL;
	S = SetAbsStringTableIndex ((STRING_TABLE)phrases, index);
	return GetStringTimeStamp (S);
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

/*
 * ============================================================================
 *  Graphics / Input / Music / Game-state bridge wrappers (P11)
 *
 *  Called from Rust via FFI when Rust needs to trigger C-side rendering,
 *  sound, animation, or game-state operations.
 *  All names match the extern "C" declarations in the Rust source exactly.
 * ============================================================================
 */

#include "colors.h"                    /* COMM_PLAYER_BACKGROUND_COLOR */
#include "controls.h"                  /* PulsedInputState, CurrentInputState */
#include "encount.h"                   /* InitEncounter, UninitEncounter */
#include "options.h"                   /* optSmoothScroll, OPT_PC, OPT_3DO */
#include "oscill.h"                    /* InitOscilloscope, DrawOscilloscope, etc. */
#include "settings.h"                  /* PlayMusic, StopMusic */
#include "sis.h"                       /* DrawSISFrame, DrawSISMessage, etc. */
#include "sounds.h"                    /* SetMenuSounds, MENU_SOUND_FLAGS */
#include "setup.h"                     /* ActivityFrame, LastActivity */
#include "libs/graphics/gfx_common.h"  /* ScreenTransition */
#include "libs/sndlib.h"               /* SetMusicVolume, FadeMusic */

/* ---- Oscilloscope / Slider ----------------------------------------------- */

void
c_InitOscilloscope (unsigned int frame)
{
	InitOscilloscope (SetAbsFrameIndex (ActivityFrame, (COUNT)frame));
}

void
c_InitSlider (int x, int y, int w, unsigned int bg_frame,
		unsigned int cursor_frame)
{
	InitSlider (x, y, w,
			SetAbsFrameIndex (ActivityFrame, (COUNT)bg_frame),
			SetAbsFrameIndex (ActivityFrame, (COUNT)cursor_frame));
}

void
c_DrawOscilloscope (void)
{
	DrawOscilloscope ();
}

void
c_DrawSlider (void)
{
	DrawSlider ();
}

void
c_SetSliderImage (unsigned int frame)
{
	SetSliderImage (SetAbsFrameIndex (ActivityFrame, (COUNT)frame));
}

/* ---- Subtitle state bridges -----------------------------------------------
 * Under USE_RUST_COMM subtitle state is owned by Rust (P10).
 * c_ClearSubtitles / c_CheckSubtitles / c_RedrawSubtitles are called from
 * Rust code.  comm.c's static definitions are guarded by
 * #ifndef USE_RUST_COMM so we forward to the Rust-side implementations. */

void
c_ClearSubtitles (void)
{
	rust_ClearSubtitles ();
}

void
c_CheckSubtitles (void)
{
	rust_CheckSubtitles ();
}

void
c_RedrawSubtitles (void)
{
	rust_RedrawSubtitles ();
}

/* ---- InitSpeechGraphics ---------------------------------------------------
 * comm.c's InitSpeechGraphics() is guarded; here we replicate its logic. */

void
c_InitSpeechGraphics (void)
{
	c_InitOscilloscope (9);
	c_InitSlider (0, SLIDER_Y, SIS_SCREEN_WIDTH, 5, 2);
}

/* ---- UpdateSpeechGraphics -------------------------------------------------
 * Rate-limited oscilloscope + slider update.  Under USE_RUST_COMM the
 * static C version is guarded; call C drawing primitives directly. */

void
c_UpdateSpeechGraphics (void)
{
	static TimeCount NextTime;
	CONTEXT OldContext;

	if (GetTimeCounter () < NextTime)
		return;

	NextTime = GetTimeCounter () + (ONE_SECOND / 32);

	OldContext = SetContext (RadarContext);
	DrawOscilloscope ();
	SetContext (SpaceContext);
	DrawSlider ();
	SetContext (OldContext);
}

/* ---- DrawAlienFrame -------------------------------------------------------
 * Initial draw of the alien frame at the start of an encounter. */

void
c_DrawAlienFrame (void)
{
	DrawAlienFrame (NULL, 0, TRUE);
}

/* ---- CommIntroTransition --------------------------------------------------
 * Perform the intro screen transition.  Under USE_RUST_COMM the static C
 * CommIntroTransition() is guarded; call the equivalent logic here. */

void
c_CommIntroTransition (void)
{
	/* Replicate comm.c CommIntroTransition() for the USE_RUST_COMM path.
	 * rust_GetCommIntroMode() returns the mode set via rust_SetCommIntroMode(). */
	unsigned int mode = rust_GetCommIntroMode ();

	if (mode == CIM_CROSSFADE_SCREEN)
	{
		ScreenTransition (3, NULL);
		UnbatchGraphics ();
	}
	else if (mode == CIM_CROSSFADE_SPACE)
	{
		RECT r;
		r.corner.x = SIS_ORG_X;
		r.corner.y = SIS_ORG_Y;
		r.extent.width = SIS_SCREEN_WIDTH;
		r.extent.height = SIS_SCREEN_HEIGHT;
		ScreenTransition (3, &r);
		UnbatchGraphics ();
	}
	else if (mode == CIM_CROSSFADE_WINDOW)
	{
		ScreenTransition (3, &CommWndRect);
		UnbatchGraphics ();
	}
	else
	{
		/* CIM_FADE_IN_SCREEN or unknown — unbatch to avoid lockup */
		UnbatchGraphics ();
	}

	rust_SetCommIntroMode (CIM_DEFAULT);
}

/* ---- Animation state bridges --------------------------------------------- */

#include "commanim.h"  /* ProcessCommAnimations, InitCommAnimations, etc. */

int
c_WantTalkingAnim (void)
{
	return wantTalkingAnim () ? 1 : 0;
}

int
c_HaveTalkingAnim (void)
{
	return haveTalkingAnim () ? 1 : 0;
}

void
c_SetRunTalkingAnim (void)
{
	setRunTalkingAnim ();
}

void
c_SetStopTalkingAnim (void)
{
	setStopTalkingAnim ();
}

void
c_SetRunIntroAnim (void)
{
	setRunIntroAnim ();
}

int
c_RunningIntroAnim (void)
{
	return runningIntroAnim () ? 1 : 0;
}

int
c_RunningTalkingAnim (void)
{
	return runningTalkingAnim () ? 1 : 0;
}

void
c_InitCommAnimations (void)
{
	InitCommAnimations ();
}

/* UpdateAnimations: under USE_RUST_COMM, ProcessCommAnimations routes to Rust
 * via rust_ProcessCommAnimations_cb().  We still need a context switch and
 * RedrawSubtitles call to drive rendering. */

void
c_UpdateAnimations (int seeking)
{
	/* ProcessCommAnimations is provided by commanim.c which under
	 * USE_RUST_COMM delegates to rust_ProcessCommAnimations_cb(). */
	BOOLEAN change;
	change = ProcessCommAnimations (FALSE, (BOOLEAN)seeking);
	if (change)
		rust_RedrawSubtitles ();
}

/* c_RunCommAnimFrame: one animation + sleep cycle, matching C runCommAnimFrame(). */

void
c_RunCommAnimFrame (void)
{
	c_UpdateAnimations (FALSE);
	SleepThread (ONE_SECOND / 40);
}

/* ---- Feedback / Response display -----------------------------------------
 * These are C-rendering hooks called from Rust's response_ui.rs.
 * comm.c's static implementations are guarded; stubs for now. */

void
c_FeedbackPlayerPhrase (const char *text)
{
	/* P11: Stub.  Full rendering wired in a later phase when the
	 * Rust encounter loop drives the C comm window directly. */
	(void)text;
}

void
c_RefreshResponses (unsigned char top, unsigned char num_responses,
		unsigned char cur_response)
{
	/* P11: Stub.  Response rendering stays in C's RefreshResponses()
	 * until the Rust encounter loop is fully wired. */
	(void)top;
	(void)num_responses;
	(void)cur_response;
}

void
c_SelectConversationSummary (void)
{
	/* P11: Stub.  Conversation summary overlay driven by Rust in P12+. */
}

void
c_DrawSISComWindow (void)
{
	/* P11: delegate to C DrawSISComWindow — it is not guarded since it
	 * is also used by C mode.  But it is declared static in comm.c so
	 * we cannot call it from here.  Replicate the body. */
	if (LOBYTE (GLOBAL (CurrentActivity)) != WON_LAST_BATTLE)
	{
		RECT r;
		CONTEXT OldContext;

		OldContext = SetContext (SpaceContext);
		r.corner.x = 0;
		r.corner.y = SLIDER_Y + SLIDER_HEIGHT;
		r.extent.width = SIS_SCREEN_WIDTH;
		r.extent.height = SIS_SCREEN_HEIGHT - r.corner.y;
		SetContextForeGroundColor (COMM_PLAYER_BACKGROUND_COLOR);
		DrawFilledRectangle (&r);
		SetContext (OldContext);
	}
}

/* ---- Music bridges -------------------------------------------------------
 * The Rust extern declarations use the non-suffixed names. */

void
c_PlayMusic (void *song, int looping, int priority)
{
	PlayMusic ((MUSIC_REF)song, (BOOLEAN)looping, (BYTE)priority);
}

unsigned int
c_FadeMusic (int vol, int duration)
{
	return (unsigned int)FadeMusic ((BYTE)vol, (SIZE)duration);
}

void
c_StopMusic (void)
{
	StopMusic ();
}

void
c_SetMusicVolume (unsigned int vol)
{
	SetMusicVolume ((COUNT)vol);
}

/* ---- Colormap bridge ----------------------------------------------------- */

void
c_SetColorMap (void *colormap)
{
	SetColorMap (GetColorMapAddress ((COLORMAP)colormap));
}

/* ---- Input bridges ------------------------------------------------------- */

int
c_GetPulsedMenuKey (int key_index)
{
	return PulsedInputState.menu[key_index];
}

int
c_GetCurrentMenuKey (int key_index)
{
	return CurrentInputState.menu[key_index];
}

void
c_SetMenuSounds (unsigned int up_down, unsigned int select)
{
	SetMenuSounds ((MENU_SOUND_FLAGS)up_down, (MENU_SOUND_FLAGS)select);
}

/* ---- Game-state bridges -------------------------------------------------- */

int
c_CheckAbort (void)
{
	return (GLOBAL (CurrentActivity) & CHECK_ABORT) ? 1 : 0;
}

int
c_WonLastBattle (void)
{
	return (LOBYTE (GLOBAL (CurrentActivity)) == WON_LAST_BATTLE) ? 1 : 0;
}

/* @plan PLAN-20260326-COMMPT2.P03 @requirement REQ-AT-001 */
int
c_HasTransitionAnim (void)
{
	return CommData.AlienTransitionDesc.NumFrames > 0 ? 1 : 0;
}

int
c_GetLastActivityAbortFlag (void)
{
	return (LastActivity & CHECK_ABORT) ? 1 : 0;
}

void
c_ClearLastActivityLoadFlag (void)
{
	LastActivity &= ~CHECK_LOAD;
}

int
c_GetOptSmoothScroll (void)
{
	return optSmoothScroll;
}

unsigned int
c_FadeOutMusicForReplay (void)
{
	return (unsigned int)FadeMusic (0, (SIZE)(ONE_SECOND * 2));
}

/* ---- Resource destroy bridges ------------------------------------------- */

#include "libs/gfxlib.h"

void
c_DestroyDrawable (unsigned int handle)
{
	DestroyDrawable (ReleaseDrawable ((FRAME)(uintptr_t)handle));
}

void
c_DestroyFont (unsigned int handle)
{
	DestroyFont ((FONT)(uintptr_t)handle);
}

void
c_DestroyColorMap (unsigned int handle)
{
	DestroyColorMap (ReleaseColorMap ((COLORMAP)(uintptr_t)handle));
}

void
c_DestroyMusic (unsigned int handle)
{
	DestroyMusic ((MUSIC_REF)(uintptr_t)handle);
}

void
c_DestroyStringTable (unsigned int handle)
{
	DestroyStringTable (ReleaseStringTable ((STRING_TABLE)(uintptr_t)handle));
}

#endif /* USE_RUST_COMM */
