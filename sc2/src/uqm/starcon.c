//Copyright Paul Reiche, Fred Ford. 1992-2002

/*
 *  This program is free software; you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation; either version 2 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program; if not, write to the Free Software
 *  Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA 02111-1307, USA.
 */

#include <stdlib.h>

#include "comm.h"
#include "battle.h"
#include "fmv.h"
#include "gameev.h"
#include "types.h"
#include "globdata.h"
#include "resinst.h"
#include "restart.h"
#include "starbase.h"
#include "save.h"
#include "setup.h"
#include "master.h"
#include "controls.h"
#include "starcon.h"
#include "clock.h"
		// for GameClockTick()
#include "hyper.h"
		// for SeedUniverse()
#include "planets/planets.h"
		// for ExploreSolarSys()
#include "uqmdebug.h"
#include "libs/tasklib.h"
#include "libs/log.h"
#include "libs/gfxlib.h"
#include "libs/graphics/gfx_common.h"
#include "libs/graphics/tfb_draw.h"
#include "libs/misc.h"

#include "uqmversion.h"
#include "options.h"

volatile int MainExited = FALSE;
#ifdef DEBUG_SLEEP
uint32 mainThreadId;
extern uint32 SDL_ThreadID(void);
#endif

// Open or close the periodically occuring QuasiSpace portal.
// It changes the appearant portal size when necessary.
static void
checkArilouGate (void)
{
	BYTE counter;

	counter = GET_GAME_STATE (ARILOU_SPACE_COUNTER);
	if (GET_GAME_STATE (ARILOU_SPACE) == OPENING)
	{	// The portal is opening or fully open
		if (counter < 9)
			++counter;
	}
	else
	{	// The portal is closing or fully closed
		if (counter > 0)
			--counter;
	}
	SET_GAME_STATE (ARILOU_SPACE_COUNTER, counter);
}

// Battle frame callback function.
static void
on_battle_frame (void)
{
	GameClockTick ();
	checkArilouGate ();

	if (!(GLOBAL (CurrentActivity) & (CHECK_ABORT | CHECK_LOAD)))
		SeedUniverse ();

	DrawAutoPilotMessage (FALSE);
}

static void
BackgroundInitKernel (DWORD TimeOut)
{
	LoadMasterShipList (TaskSwitch);
	TaskSwitch ();
	InitGameKernel ();

	while ((GetTimeCounter () <= TimeOut) &&
	       !(GLOBAL (CurrentActivity) & CHECK_ABORT))
	{
		UpdateInputState ();
		TaskSwitch ();
	}
}

// @plan PLAN-20260707-MAINLOOP.P02b
// Exported wrapper for Rust FFI -- calls static BackgroundInitKernel.
// Must live in starcon.c because BackgroundInitKernel is static here.
void
uqm_splash_with_bg_init_kernel (void)
{
	SplashScreen (BackgroundInitKernel);
}

// @plan PLAN-20260707-MAINLOOP.P02b
// Exported wrapper for Rust FFI -- calls Battle with static on_battle_frame.
// Must live in starcon.c because on_battle_frame is static here.
void
uqm_battle_with_frame_callback (void)
{
	Battle (&on_battle_frame);
}

// Executes on the main() thread
void
SignalStopMainThread (void)
{
	GamePaused = FALSE;
	GLOBAL (CurrentActivity) |= CHECK_ABORT;
	TaskSwitch ();
}

// Executes on the main() thread
void
ProcessUtilityKeys (void)
{
	if (ImmediateInputState.menu[KEY_ABORT])
	{
		log_showBox (false, false);
		exit (EXIT_SUCCESS);
	}
	
	if (ImmediateInputState.menu[KEY_FULLSCREEN])
	{
		int flags = GfxFlags ^ TFB_GFXFLAGS_FULLSCREEN;
		// clear ImmediateInputState so we don't repeat this next frame
		FlushInput ();
		TFB_DrawScreen_ReinitVideo (GraphicsDriver, flags, ScreenWidthActual,
				ScreenHeightActual);
	}

#if defined(DEBUG) || defined(USE_DEBUG_KEY)
	{	// Only call the debug func on the rising edge of
		// ImmediateInputState[KEY_DEBUG] so it does not execute repeatedly.
		// This duplicates the PulsedInputState somewhat, but we cannot
		// use PulsedInputState here because it is meant for another thread.
		static int debugKeyState;

		if (ImmediateInputState.menu[KEY_DEBUG] && debugKeyState == 0)
		{
			debugKeyPressed ();
		}
		debugKeyState = ImmediateInputState.menu[KEY_DEBUG];
	}
#endif  /* DEBUG */
}

/* TODO: Remove these declarations once threading is gone. */
extern int snddriver, soundflags;

int
Starcon2Main (void *threadArg)
{
	/* The game loop body is implemented entirely in Rust.
	 * C main() still owns startup, the main-thread event pump, and
	 * subsystem teardown after MainExited. */
	extern int rust_game_loop (void);
	(void) threadArg;
	return rust_game_loop ();
}

