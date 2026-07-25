/*
 *  Rust Communication System wrapper
 *
 *  Provides C bridge functions for the USE_RUST_COMM path. All other
 *  communication logic has been ported to Rust. This file contains only
 *  the 5 remaining C graphics bridge functions that use complex C
 *  graphics primitives (font_DrawText, DrawFilledRectangle, etc.).
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

/* Initialize communication system using Rust implementation.
 * Called from C code (setup.c) — must remain here as a C entry point. */
void
init_communication (void)
{
	rust_InitCommunication ();
}

/* Uninitialize communication system using Rust implementation. */
void
uninit_communication (void)
{
	rust_UninitCommunication ();
}

#include "globdata.h"
#include "colors.h"                    /* COMM_PLAYER_BACKGROUND_COLOR */
#include "controls.h"                  /* PulsedInputState, DoInput */
#include "gamestr.h"                   /* GAME_STRING, FEEDBACK_STRING_BASE */
#include "ifontres.h"                  /* PLAYER_FONT */
#include "nameref.h"                   /* LoadFont */
#include "settings.h"                  /* optSmoothScroll, OPT_PC, OPT_3DO */
#include "sis.h"                       /* SIS_SCREEN_WIDTH/HEIGHT, SLIDER_Y */
#include "setup.h"                     /* ActivityFrame, TinyFont */
#include "libs/sndlib.h"               /* FadeMusic */
#include "libs/strlib.h"               /* STR_BULLET, STR_MIDDLE_DOT */
#include "libs/gfxlib.h"

/* Extern declarations for trackplayer subtitle functions */
extern SUBTITLE_REF c_GetFirstTrackSubtitle (void);
extern SUBTITLE_REF c_GetNextTrackSubtitle (SUBTITLE_REF last_ref);
extern const UNICODE *c_GetTrackSubtitleText (SUBTITLE_REF sub_ref);
extern void comm_ClearSubtitles (void);

#define PLAYER_TEXT_WIDTH ((SIZE)(SIS_SCREEN_WIDTH - 8 - (TEXT_X_OFFS << 2)))

/* Last response-list window state, captured from c_RefreshResponses.
 * Used to restore the list after summary overlay closes.
 */
static unsigned char last_top_response = 0;
static unsigned char last_num_responses = 0;
static unsigned char last_cur_response = 0;

/* ---- draw_player_text_wrapped ------------------------------------------- */

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

/* ---- c_DrawSISComWindow ------------------------------------------------- */

void
c_DrawSISComWindow (void)
{
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

/* ---- c_FeedbackPlayerPhrase --------------------------------------------- */

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

/* ---- c_RefreshResponses ------------------------------------------------- */

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

/* ---- c_SelectConversationSummary ---------------------------------------- */

typedef struct summary_loop_state
{
	BOOLEAN (*InputFunc) (struct summary_loop_state *pSS);
	BOOLEAN Initialized;
	BOOLEAN PrintNext;
	SUBTITLE_REF NextSub;
	const UNICODE *LeftOver;
} SUMMARY_LOOP_STATE;

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

void
c_SelectConversationSummary (void)
{
	SUMMARY_LOOP_STATE SummaryState;
	char text_buf[1024];

	if (last_num_responses > 0
			&& rust_GetResponseText ((int)last_cur_response, text_buf, sizeof (text_buf)))
	{
		c_FeedbackPlayerPhrase (text_buf);
	}

	SummaryState.Initialized = FALSE;
	do_summary_page (&SummaryState);

	if (last_num_responses > 0)
	{
		c_RefreshResponses (last_top_response, last_num_responses, last_cur_response);
	}

	c_ClearSubtitles ();
}

#endif /* USE_RUST_COMM */
