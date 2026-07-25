/*
 *  Rust Communication System wrapper
 *  
 *  Wraps Rust-implemented communication functions when USE_RUST_COMM is defined.
 *  This file provides C-callable wrapper functions that delegate to the Rust
 *  implementation via the FFI bindings declared in rust_comm.h.
 *
 *  @plan PLAN-20260314-COMM.P05b
 */

#include <stdio.h>
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

/* c_get_conversation_phrase, c_get_commander_name, c_get_ship_name — PORTED to Rust */

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

/* c_get_phrase_sound_clip, c_get_phrase_timestamp — PORTED to Rust */


void
c_SpliceTrack (UNICODE *filespec, UNICODE *textspec,
		UNICODE *timestamp, CallbackFunction cb)
{
	fprintf (stderr, "[DBG] c_SpliceTrack: file=%p text=%p ts=%p cb=%p\n",
		(void *)filespec, (void *)textspec, (void *)timestamp, (void *)cb);
	SpliceTrack (filespec, textspec, timestamp, cb);
	fprintf (stderr, "[DBG] c_SpliceTrack: done\n");
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
#include "controls.h"                  /* PulsedInputState, CurrentInputState, DoInput */
#include "encount.h"                   /* InitEncounter, UninitEncounter */
#include "gamestr.h"                   /* GAME_STRING, FEEDBACK_STRING_BASE */
#include "ifontres.h"                  /* PLAYER_FONT */
#include "nameref.h"                   /* LoadFont, LoadGraphicInstance */
#include "options.h"                   /* optSmoothScroll, OPT_PC, OPT_3DO */
#include "oscill.h"                    /* InitOscilloscope, DrawOscilloscope, etc. */
#include "settings.h"                  /* PlayMusic, StopMusic */
#include "sis.h"                       /* DrawSISFrame, DrawSISMessage, etc. */
#include "sounds.h"                    /* SetMenuSounds, MENU_SOUND_FLAGS */
#include "setup.h"                     /* ActivityFrame, LastActivity, TinyFont */
#include "libs/graphics/gfx_common.h"  /* ScreenTransition */
#include "libs/sndlib.h"               /* SetMusicVolume, FadeMusic */
#include "libs/strlib.h"               /* STR_BULLET, STR_MIDDLE_DOT */

/* Forward declaration needed because c_DrawSISComWindow is defined later
 * in this file but called by the rendering bridge functions above it. */
void c_DrawSISComWindow (void);


/* ---- Subtitle state bridges -----------------------------------------------
 * @plan PLAN-20260325-COMMPT3.P06
 * @requirement REQ-SD-001..REQ-SD-003
 * @pseudocode 002-subtitle-display-fix lines 39-54
 *
 * c_ClearSubtitles / c_CheckSubtitles / c_RedrawSubtitles are called from
 * Rust via FFI.  They forward to comm_* functions defined in comm.c
 * (inside #ifdef USE_RUST_COMM), breaking the former circular
 * rust_comm.c → rust_* → rust_comm.c routing. */


/* ---- InitSpeechGraphics ---------------------------------------------------
 * comm.c's InitSpeechGraphics() is guarded; here we replicate its logic. */

void
c_InitSpeechGraphics (void)
{
	c_InitOscilloscope (9);
	InitSlider (0, SLIDER_Y, SIS_SCREEN_WIDTH,
			SetAbsFrameIndex (ActivityFrame, (COUNT)5),
			SetAbsFrameIndex (ActivityFrame, (COUNT)2));
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


/* UpdateAnimations: under USE_RUST_COMM, ProcessCommAnimations routes to Rust
 * via rust_ProcessCommAnimations_cb().  We still need a context switch and
 * RedrawSubtitles call to drive rendering. */

void
c_UpdateAnimations (int seeking)
{
	CONTEXT OldContext;
	BOOLEAN change;
	BOOLEAN do_clear;

	do_clear = c_GetClearSubtitles ();
	OldContext = SetContext (c_GetAnimContext ());
	BatchGraphics ();
	change = ProcessCommAnimations (do_clear, (BOOLEAN)seeking);
	if (change || do_clear)
		comm_RedrawSubtitles ();
	UnbatchGraphics ();
	c_ResetClearSubtitles ();
	SetContext (OldContext);
}

/* c_RunCommAnimFrame: one animation + sleep cycle, matching C runCommAnimFrame(). */

void
c_RunCommAnimFrame (void)
{
	c_UpdateAnimations (FALSE);
	SleepThread (ONE_SECOND / 40);
}

/* ---- C-side TalkSegue (DoInput-driven) -----------------------------------
 *
 * Replaces Rust's blocking while-loop with a proper DoInput-driven loop
 * that yields to UQM cooperative threading via SleepThreadUntil.
 * This avoids holding the Rust COMM_STATE lock during blocking operations.
 *
 * Closely matches comm.c DoTalkSegue/TalkSegue but uses the bridge functions
 * available under USE_RUST_COMM.
 */

#include "controls.h"     /* PulsedInputState, CurrentInputState */
#include "settings.h"     /* optSmoothScroll, OPT_PC, OPT_3DO */

/* Forward declarations for functions in comm.c USE_RUST_COMM block */
extern void comm_CheckSubtitles (void);
extern void comm_ClearSubtitles (void);

/* TALKING_STATE for DoInput-driven talk segue */
typedef struct c_talking_state
{
	BOOLEAN (*InputFunc) (struct c_talking_state *);
	TimeCount NextTime;
	COUNT waitTrack;
	BOOLEAN rewind;
	BOOLEAN seeking;
	BOOLEAN ended;
} C_TALKING_STATE;

static BOOLEAN
c_DoTalkSegue (C_TALKING_STATE *pTS)
{
	BOOLEAN left = FALSE;
	BOOLEAN right = FALSE;
	COUNT curTrack;
	fprintf (stderr, "[DBG] c_DoTalkSegue: enter seeking=%d ended=%d\n",
		pTS->seeking, pTS->ended);

	if (GLOBAL (CurrentActivity) & CHECK_ABORT)
	{
		pTS->ended = TRUE;
		return FALSE;
	}

	if (PulsedInputState.menu[KEY_MENU_CANCEL])
	{
		JumpTrack ();
		pTS->ended = TRUE;
		return FALSE;
	}

	if (optSmoothScroll == OPT_PC)
	{
		left = PulsedInputState.menu[KEY_MENU_LEFT] != 0;
		right = PulsedInputState.menu[KEY_MENU_RIGHT] != 0;
	}
	else if (optSmoothScroll == OPT_3DO)
	{
		left = CurrentInputState.menu[KEY_MENU_LEFT] != 0;
		right = CurrentInputState.menu[KEY_MENU_RIGHT] != 0;
	}

	fprintf (stderr, "[DBG] c_DoTalkSegue: opt=%d left=%d right=%d curL=%d curR=%d pulL=%d pulR=%d\n",
		optSmoothScroll, left, right,
		CurrentInputState.menu[KEY_MENU_LEFT], CurrentInputState.menu[KEY_MENU_RIGHT],
		PulsedInputState.menu[KEY_MENU_LEFT], PulsedInputState.menu[KEY_MENU_RIGHT]);

	if (right)
	{
		SetSliderImage (SetAbsFrameIndex (ActivityFrame, 3));
		if (optSmoothScroll == OPT_PC)
			FastForward_Page ();
		else if (optSmoothScroll == OPT_3DO)
			FastForward_Smooth ();
		pTS->seeking = TRUE;
	}
	else if (left || pTS->rewind)
	{
		pTS->rewind = FALSE;
		SetSliderImage (SetAbsFrameIndex (ActivityFrame, 4));
		if (optSmoothScroll == OPT_PC)
			FastReverse_Page ();
		else if (optSmoothScroll == OPT_3DO)
			FastReverse_Smooth ();
		pTS->seeking = TRUE;
	}
	else if (pTS->seeking)
	{
		pTS->seeking = FALSE;
		SetSliderImage (SetAbsFrameIndex (ActivityFrame, 2));
	}
	else
	{
		comm_CheckSubtitles ();
	}

	c_UpdateAnimations (pTS->seeking);
	c_UpdateSpeechGraphics ();

	curTrack = PlayingTrack ();
	pTS->ended = !pTS->seeking && !curTrack;

	SleepThreadUntil (pTS->NextTime);
	pTS->NextTime = GetTimeCounter () + ONE_SECOND / 60;

	return pTS->seeking || (curTrack && curTrack <= pTS->waitTrack);
}

/*
 * C-side TalkSegue: runs the talk segue via DoInput with proper frame pacing.
 * Returns TRUE if playback reached its natural end.
 */
int
c_RunTalkSegue (unsigned int wait_track)
{
	C_TALKING_STATE talkingState;

	fprintf (stderr, "[DBG] c_RunTalkSegue: wait_track=%u\n", wait_track);

	/* Transition animation to talking state */
	if (wantTalkingAnim () && haveTalkingAnim ())
	{
		fprintf (stderr, "[DBG] c_RunTalkSegue: have talking anim\n");
		if (haveTransitionAnim ())
			setRunIntroAnim ();
		setRunTalkingAnim ();
		while (runningIntroAnim ())
			c_RunCommAnimFrame ();
	}
	else
	{
		fprintf (stderr, "[DBG] c_RunTalkSegue: NO talking anim (want=%d have=%d)\n",
			wantTalkingAnim (), haveTalkingAnim ());
	}

	memset (&talkingState, 0, sizeof talkingState);

	if (wait_track == 0)
	{
		wait_track = (COUNT)~0;
		talkingState.rewind = TRUE;
		fprintf (stderr, "[DBG] c_RunTalkSegue: rewind mode\n");
	}
	else if (!PlayingTrack ())
	{
		fprintf (stderr, "[DBG] c_RunTalkSegue: calling PlayTrack()\n");
		PlayTrack ();
		fprintf (stderr, "[DBG] c_RunTalkSegue: PlayTrack() done, PlayingTrack()=%d\n",
			PlayingTrack ());
	}
	else
	{
		fprintf (stderr, "[DBG] c_RunTalkSegue: already playing track=%d\n",
			PlayingTrack ());
	}

	SetMenuSounds (MENU_SOUND_NONE, MENU_SOUND_NONE);
	talkingState.InputFunc = c_DoTalkSegue;
	talkingState.waitTrack = (COUNT)wait_track;

	fprintf (stderr, "[DBG] c_RunTalkSegue: entering DoInput (waitTrack=%u)\n",
		(unsigned)talkingState.waitTrack);
	DoInput (&talkingState, FALSE);
	fprintf (stderr, "[DBG] c_RunTalkSegue: DoInput returned (ended=%d)\n",
		talkingState.ended);

	comm_ClearSubtitles ();

	if (talkingState.ended)
	{
		SetSliderImage (SetAbsFrameIndex (ActivityFrame, 8));
	}

	/* Transition back to silent */
	if (runningTalkingAnim ())
		setStopTalkingAnim ();
	while (runningTalkingAnim ())
		c_RunCommAnimFrame ();

	return talkingState.ended;
}


/* ---- Feedback / Response display -----------------------------------------
 * C-rendering hooks called from Rust's response_ui.rs.
 * comm.c's static implementations are guarded behind #ifndef USE_RUST_COMM,
 * so we replicate equivalent draw logic here using the same C primitives.
 *
 * @plan PLAN-20260326-COMMPT2.P05
 */

/* Shared text width for player response area, matching comm.c add_text(-1/-2).
 * TEXT_X_OFFS is 1; the expression is SIS_SCREEN_WIDTH - 8 - (TEXT_X_OFFS<<2). */
#define PLAYER_TEXT_WIDTH ((SIZE)(SIS_SCREEN_WIDTH - 8 - (TEXT_X_OFFS << 2)))

/* Draw word-wrapped player text starting at *pText.
 * Mirrors the drawing loop in comm.c add_text() for status <= -2.
 * The caller sets pText->baseline.y to the first line Y coordinate.
 * Returns the baseline.y of the line after the last one drawn.
 *
 * @plan PLAN-20260326-COMMPT2.P05
 * @requirement REQ-RB-001, REQ-RB-002
 */
static COORD
draw_player_text_wrapped (TEXT *pText)
{
	const char *pStr;
	const char *next;
	SIZE leading;
	COUNT maxchars;
	BOOLEAN eol;

	GetContextFontLeading (&leading);

	pStr = pText->pStr;
	maxchars = (COUNT)~0;

	/* Subtract one leading so the first loop iteration restores to
	 * the requested baseline.y — mirrors add_text()'s pre-adjustment. */
	pText->baseline.y -= leading;

	do
	{
		pText->pStr = pStr;
		pText->baseline.y += leading;
		eol = getLineWithinWidth (pText, &next, PLAYER_TEXT_WIDTH, maxchars);
		maxchars -= pText->CharCount;
		if (maxchars != 0)
			--maxchars;
		pStr = next;

		if (pText->baseline.y < SIS_SCREEN_HEIGHT)
			font_DrawText (pText);
	} while (!eol && maxchars);

	return pText->baseline.y;
}

/* Last response-list window state, captured from c_RefreshResponses.
 * Used to restore the list after summary overlay closes.
 */
static unsigned char last_top_response = 0;
static unsigned char last_num_responses = 0;
static unsigned char last_cur_response = 0;

/* Render the player's selected response text in the SIS comm window.
 * Replicates comm.c FeedbackPlayerPhrase() for the USE_RUST_COMM path.
 *
 * @plan PLAN-20260326-COMMPT2.P05
 * @requirement REQ-RB-001, REQ-RB-004
 */
void
c_FeedbackPlayerPhrase (const char *text)
{
	CONTEXT OldContext;
	FONT PlayerFont, OldFont;

	OldContext = SetContext (SpaceContext);

	BatchGraphics ();
	c_DrawSISComWindow ();

	if (text && text[0])
	{
		TEXT ct;
		const char *pStr;
		const char *next;
		SIZE leading;
		COUNT maxchars;
		BOOLEAN eol;

		PlayerFont = LoadFont (PLAYER_FONT);
		OldFont = SetContextFont (PlayerFont);

		ct.baseline.x = SIS_SCREEN_WIDTH >> 1;
		ct.baseline.y = SLIDER_Y + SLIDER_HEIGHT + 13;
		ct.align = ALIGN_CENTER;
		ct.CharCount = (COUNT)~0;
		ct.pStr = GAME_STRING (FEEDBACK_STRING_BASE);
		SetContextForeGroundColor (COMM_RESPONSE_INTRO_TEXT_COLOR);
		font_DrawText (&ct);

		/* Feedback text: centered, word-wrapped, no bullet (add_text(-4)).
		 * The add_text(-4) path does NOT pre-subtract leading, so the
		 * first draw lands at baseline.y + leading from the initial value. */
		ct.baseline.y += 16;
		ct.align = ALIGN_CENTER;
		ct.pStr = text;
		SetContextForeGroundColor (COMM_FEEDBACK_TEXT_COLOR);

		GetContextFontLeading (&leading);
		pStr = ct.pStr;
		maxchars = (COUNT)~0;

		do
		{
			ct.pStr = pStr;
			ct.baseline.y += leading;
			eol = getLineWithinWidth (&ct, &next, PLAYER_TEXT_WIDTH, maxchars);
			maxchars -= ct.CharCount;
			if (maxchars != 0)
				--maxchars;
			pStr = next;
			if (ct.baseline.y < SIS_SCREEN_HEIGHT)
				font_DrawText (&ct);
		} while (!eol && maxchars);

		SetContextFont (OldFont);
		DestroyFont (PlayerFont);
	}

	UnbatchGraphics ();
	SetContext (OldContext);
}

/* Render the response list in the SIS comm window.
 * Replicates comm.c RefreshResponses() for the USE_RUST_COMM path.
 * Response text is fetched from Rust via rust_GetResponseText().
 *
 * @plan PLAN-20260326-COMMPT2.P05
 * @requirement REQ-RB-002, REQ-RB-004
 */
void
c_RefreshResponses (unsigned char top, unsigned char num_responses,
		unsigned char cur_response)
{
	CONTEXT OldContext;
	FONT PlayerFont, OldFont;
	SIZE leading;
	COORD y;
	unsigned char response;
	STAMP s;
	char text_buf[1024];

	/* Track latest list window so summary can restore responses on return. */
	last_top_response = top;
	last_num_responses = num_responses;
	last_cur_response = cur_response;

	OldContext = SetContext (SpaceContext);
	PlayerFont = LoadFont (PLAYER_FONT);
	OldFont = SetContextFont (PlayerFont);
	GetContextFontLeading (&leading);

	BatchGraphics ();
	c_DrawSISComWindow ();

	y = SLIDER_Y + SLIDER_HEIGHT + 1;
	for (response = top; response < num_responses; ++response)
	{
		TEXT rt;
		TEXT bullet;

		if (!rust_GetResponseText ((int)response, text_buf, sizeof (text_buf)))
			continue;

		rt.pStr = text_buf;
		rt.CharCount = (COUNT)~0;
		rt.baseline.x = TEXT_X_OFFS + 8;
		rt.baseline.y = y + leading;
		rt.align = ALIGN_LEFT;

		if (response == cur_response)
			SetContextForeGroundColor (COMM_PLAYER_TEXT_HIGHLIGHT_COLOR);
		else
			SetContextForeGroundColor (COMM_PLAYER_TEXT_NORMAL_COLOR);

		bullet = rt;
		bullet.baseline.x -= 8;
		bullet.pStr = STR_BULLET;
		font_DrawText (&bullet);

		y = draw_player_text_wrapped (&rt);
	}

	s.frame = 0;
	if (top)
	{
		s.origin.y = SLIDER_Y + SLIDER_HEIGHT + 1;
		s.frame = SetAbsFrameIndex (ActivityFrame, 6);
	}
	else if (y > SIS_SCREEN_HEIGHT)
	{
		s.origin.y = SIS_SCREEN_HEIGHT - 2;
		s.frame = SetAbsFrameIndex (ActivityFrame, 7);
	}

	if (s.frame)
	{
		RECT r;

		GetFrameRect (s.frame, &r);
		s.origin.x = SIS_SCREEN_WIDTH - r.extent.width - 1;
		DrawStamp (&s);
	}

	UnbatchGraphics ();

	SetContextFont (OldFont);
	DestroyFont (PlayerFont);
	SetContext (OldContext);
}

/* State for the conversation summary page loop.
 * First field must be an InputFunc pointer for DoInput compatibility. */
typedef struct summary_loop_state
{
	BOOLEAN (*InputFunc) (struct summary_loop_state *pSS);
	BOOLEAN Initialized;
	BOOLEAN PrintNext;
	SUBTITLE_REF NextSub;
	const UNICODE *LeftOver;
} SUMMARY_LOOP_STATE;

/* Draw one page of the conversation history into SpaceContext.
 * Advances pSS->NextSub and pSS->LeftOver; returns FALSE when done.
 *
 * @plan PLAN-20260326-COMMPT2.P05
 * @requirement REQ-RB-003
 */
static BOOLEAN
do_summary_page (SUMMARY_LOOP_STATE *pSS)
{
#define DELTA_Y_SUMMARY 8
#define MAX_SUMM_ROWS ((SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT) \
		/ DELTA_Y_SUMMARY) - 1

	if (!pSS->Initialized)
	{
		pSS->PrintNext = TRUE;
		pSS->NextSub = c_GetFirstTrackSubtitle ();
		pSS->LeftOver = NULL;
		pSS->Initialized = TRUE;
		pSS->InputFunc = do_summary_page;
		DoInput (pSS, FALSE);
		return TRUE;
	}

	if (GLOBAL (CurrentActivity) & CHECK_ABORT)
		return FALSE;

	if (PulsedInputState.menu[KEY_MENU_SELECT]
			|| PulsedInputState.menu[KEY_MENU_CANCEL]
			|| PulsedInputState.menu[KEY_MENU_RIGHT])
	{
		if (pSS->NextSub)
		{
			pSS->PrintNext = TRUE;
		}
		else
		{
			return FALSE;
		}
	}
	else if (pSS->PrintNext)
	{
		RECT r;
		TEXT t;
		int row;
		SIZE tw;

		SetContext (SpaceContext);

		r.corner.x = 0;
		r.corner.y = SLIDER_Y + SLIDER_HEIGHT;
		r.extent.width = SIS_SCREEN_WIDTH;
		r.extent.height = SIS_SCREEN_HEIGHT - r.corner.y;
		SetContextForeGroundColor (COMM_HISTORY_BACKGROUND_COLOR);
		DrawFilledRectangle (&r);

		SetContextForeGroundColor (COMM_HISTORY_TEXT_COLOR);
		SetContextFont (TinyFont);

		tw = r.extent.width - 2 - 2;
		t.baseline.x = 2;
		t.align = ALIGN_LEFT;
		t.baseline.y = SLIDER_Y + SLIDER_HEIGHT + DELTA_Y_SUMMARY;

		for (row = 0; row < MAX_SUMM_ROWS && pSS->NextSub; ++row,
				pSS->NextSub = c_GetNextTrackSubtitle (pSS->NextSub))
		{
			const char *next = NULL;

			if (pSS->LeftOver)
			{
				t.pStr = pSS->LeftOver;
				pSS->LeftOver = NULL;
			}
			else
			{
				t.pStr = c_GetTrackSubtitleText (pSS->NextSub);
				if (!t.pStr)
					continue;
			}

			t.CharCount = (COUNT)~0;
			for (; row < MAX_SUMM_ROWS &&
					!getLineWithinWidth (&t, &next, tw, (COUNT)~0);
					++row)
			{
				font_DrawText (&t);
				t.baseline.y += DELTA_Y_SUMMARY;
				t.pStr = next;
				t.CharCount = (COUNT)~0;
			}

			if (row >= MAX_SUMM_ROWS)
			{
				pSS->LeftOver = next;
				break;
			}

			font_DrawText (&t);
			t.baseline.y += DELTA_Y_SUMMARY;
		}

		if (row >= MAX_SUMM_ROWS && (pSS->NextSub || pSS->LeftOver))
		{
			TEXT mt;
			UNICODE buffer[80];

			mt.baseline.x = SIS_SCREEN_WIDTH >> 1;
			mt.baseline.y = t.baseline.y;
			mt.align = ALIGN_CENTER;
			snprintf (buffer, sizeof (buffer), "%s%s%s",
					STR_MIDDLE_DOT,
					GAME_STRING (FEEDBACK_STRING_BASE + 1),
					STR_MIDDLE_DOT);
			mt.pStr = buffer;
			SetContextForeGroundColor (COMM_MORE_TEXT_COLOR);
			font_DrawText (&mt);
		}

		pSS->PrintNext = FALSE;
	}
	else
	{
		SleepThread (ONE_SECOND / 20);
	}

	return TRUE;
}

/* Show the conversation summary overlay.
 * Replicates comm.c SelectConversationSummary() semantics for USE_RUST_COMM:
 * 1) show current player phrase context,
 * 2) display summary pages,
 * 3) restore response list,
 * 4) clear subtitles for redraw.
 *
 * @plan PLAN-20260326-COMMPT2.P05
 * @requirement REQ-RB-003, REQ-RB-004
 */
void
c_SelectConversationSummary (void)
{
	SUMMARY_LOOP_STATE SummaryState;
	char text_buf[1024];

	/* FeedbackPlayerPhrase(pES->phrase_buf) equivalent using current selected response text. */
	if (last_num_responses > 0
			&& rust_GetResponseText ((int)last_cur_response, text_buf, sizeof (text_buf)))
	{
		c_FeedbackPlayerPhrase (text_buf);
	}

	SummaryState.Initialized = FALSE;
	do_summary_page (&SummaryState);

	/* RefreshResponses(pES) equivalent. */
	if (last_num_responses > 0)
	{
		c_RefreshResponses (last_top_response, last_num_responses, last_cur_response);
	}

	/* clear_subtitles = TRUE equivalent in Rust path. */
	c_ClearSubtitles ();
}

void
c_DrawSISComWindow (void)
{
	/* DrawSISComWindow equivalent body copied for USE_RUST_COMM path.
	 * The original function in comm.c is static, so this bridge carries
	 * the same rendering operations directly. */
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


/* ---- Input bridges ------------------------------------------------------- */


/* c_GetPulsedMenuKey, c_GetCurrentMenuKey, c_HasTransitionAnim,
 * c_GetLastActivityAbortFlag, c_ClearLastActivityLoadFlag, c_GetOptSmoothScroll
 * — PORTED to Rust (direct PulsedInputState/CurrentInputState/LastActivity/
 * optSmoothScroll/CommData access) */

/* ---- Resource destroy bridges ------------------------------------------- */

#include "libs/gfxlib.h"


/*
 * ============================================================================
 *  Resource Bridge (P06, @plan PLAN-20260326-COMMPT2.P06)
 *
 *  Thin wrappers so Rust can load, capture, release, and manage C-side
 *  graphics/audio/string resources without knowing internal type layouts.
 *  All handles returned as uintptr_t; zero means load failure.
 * ============================================================================
 */

#include "libs/reslib.h"   /* RESOURCE typedef (const char *) */
#include "libs/strlib.h"   /* STRING_TABLE, CaptureStringTable, etc. */
#include "libs/sndlib.h"   /* MUSIC_REF, LoadMusicInstance */
#include "units.h"         /* SIS_ORG_X/Y, SIS_SCREEN_WIDTH/HEIGHT */
#include "controls.h"      /* DoInput */

/* ---- Resource load bridges, graphics forwarders, CommData accessor bridges ---
 * All DEAD CODE (never called from Rust). Removed:
 * c_LoadGraphic, c_LoadFont, c_LoadColorMap, c_LoadMusic, c_LoadStringTable,
 * c_CaptureDrawable, c_CaptureColorMap, c_CaptureStringTable,
 * c_ReleaseDrawable, c_ReleaseColorMap, c_ReleaseStringTable,
 * c_CreateContext, c_SetContext, c_SetContextClipRect, c_ClearContextClipRect,
 * c_SetContextBackGroundColor, c_SetContextFont, c_CreateDrawable,
 * c_SetFrameTransparentColor, c_GetFrameRect, c_SetTransitionSource,
 * c_GetScreen, c_GetSpaceContext,
 * c_GetCommDataAlienFrameRes, c_GetCommDataAlienFontRes, c_GetCommDataAlienColorMapRes,
 * c_GetCommDataAlienSongRes, c_GetCommDataAlienAltSongRes, c_GetCommDataAlienSongFlags,
 * c_GetCommDataConversationPhrasesRes, c_SetCommDataAlienFrame, c_SetCommDataAlienFont,
 * c_SetCommDataAlienColorMap, c_SetCommDataAlienSong, c_SetCommDataConversationPhrases,
 * c_ClearCommDataConversationPhrasesRes, c_ClearCommDataConversationPhrases,
 * c_GetCommConversationPhrases */
/* ---- Encounter function call bridges ------------------------------------- */
/* c_CallInitEncounterFunc, c_CallPostEncounterFunc, c_CallUninitEncounterFunc — PORTED to Rust */

/* ---- Game-state / layout query bridges ----------------------------------- */
/* @plan PLAN-20260326-COMMPT2.P06 */

/* c_IsStarbaseConversation — PORTED to Rust (direct game_state_keys access) */

/* c_GetGameString, c_GetPlanetName, c_GetSISOrigin, c_GetPlayerFontRes,
 * c_GetWantPixmap, c_GetCommWndRect, c_SetCommWndRect — PORTED to Rust */

/* ---- HailAlien encounter loop bridge ------------------------------------- */
/* @plan PLAN-20260326-COMMPT2.P07 @requirement REQ-DI-001 */

#include "libs/sndlib.h" /* StopSound */

/*
 * Minimal input state for the Rust encounter loop.
 * The first field (InputFunc) is the only requirement for DoInput compatibility;
 * the remaining fields provide the same layout used by comm.c ENCOUNTER_STATE
 * so that pCurInputState = &rust_es is valid for C code that reads phrase_buf.
 *
 * @plan PLAN-20260326-COMMPT2.P07
 */
typedef struct rust_encounter_state
{
	/* First field: required by DoInput() */
	BOOLEAN (*InputFunc) (struct rust_encounter_state *pES);

	/* Mirrors comm.c ENCOUNTER_STATE fields used by bridge code */
	COUNT  Initialized;
	TimeCount NextTime;
	BYTE   num_responses;
	BYTE   cur_response;
	BYTE   top_response;
	/* phrase_buf: accessed by c_ClearPhraseBuf and c_SelectConversationSummary */
	UNICODE phrase_buf[1024];
} RUST_ENCOUNTER_STATE;

/*
 * C-side callback that DoInput invokes each frame for the encounter loop.
 * Delegates to rust_DoCommunication(), which implements the Rust-side
 * DoCommunication state machine.
 */
static BOOLEAN
rust_do_communication_cb (RUST_ENCOUNTER_STATE *pES)
{
	(void)pES;
	return (BOOLEAN)rust_DoCommunication ();
}

/* ---- Last-replay DoInput (no responses available) ----------------------- */

typedef struct last_replay_state_rust
{
	BOOLEAN (*InputFunc) (struct last_replay_state_rust *);
	TimeCount NextTime;
	TimeCount TimeOut;
} LAST_REPLAY_STATE_RUST;

static BOOLEAN
c_DoLastReplay (LAST_REPLAY_STATE_RUST *pLRS)
{
	if (GLOBAL (CurrentActivity) & CHECK_ABORT)
		return FALSE;

	if (GetTimeCounter () > pLRS->TimeOut)
		return FALSE;

	if (PulsedInputState.menu[KEY_MENU_CANCEL]
		&& LOBYTE (GLOBAL (CurrentActivity)) != WON_LAST_BATTLE)
	{
		FadeMusic (usingSpeech ? (NORMAL_VOLUME / 2) : NORMAL_VOLUME, ONE_SECOND);
		c_SelectConversationSummary ();
		pLRS->TimeOut = FadeMusic (0, ONE_SECOND * 2) + ONE_SECOND / 60;
	}
	else if (PulsedInputState.menu[KEY_MENU_LEFT])
	{
		/* Replay handled via SelectReplay if available; for now use summary */
		c_SelectConversationSummary ();
		pLRS->TimeOut = FadeMusic (0, ONE_SECOND * 2) + ONE_SECOND / 60;
	}

	c_UpdateAnimations (0);

	SleepThreadUntil (pLRS->NextTime);
	pLRS->NextTime = GetTimeCounter () + (ONE_SECOND / 40);

	return TRUE;
}

void
c_RunLastReplay (int timeout)
{
	LAST_REPLAY_STATE_RUST replayState;
	memset (&replayState, 0, sizeof replayState);
	replayState.TimeOut = (TimeCount)timeout + ONE_SECOND / 60;
	replayState.InputFunc = c_DoLastReplay;
	DoInput (&replayState, FALSE);
}


/*
 * Allocate a RUST_ENCOUNTER_STATE on the stack, wire InputFunc to the Rust
 * DoCommunication callback, register it as pCurInputState, run DoInput,
 * then clear pCurInputState.  Rust calls this from hail_alien() to drive
 * the encounter loop.
 *
 * @plan PLAN-20260326-COMMPT2.P07
 * @requirement REQ-DI-001, REQ-HL-001
 */
void
c_RunEncounterDoInput (void)
{
	RUST_ENCOUNTER_STATE ES;
	memset (&ES, 0, sizeof ES);
	ES.InputFunc = rust_do_communication_cb;
	/* Register as pCurInputState so comm-internal bridge code can access
	 * phrase_buf via c_ClearPhraseBuf and c_SetCurInputState. */
	c_SetCurInputState (&ES);
	SetMenuSounds (MENU_SOUND_UP | MENU_SOUND_DOWN, MENU_SOUND_SELECT);
	DoInput (&ES, FALSE);
	c_SetCurInputState (NULL);
}

#endif /* USE_RUST_COMM */
