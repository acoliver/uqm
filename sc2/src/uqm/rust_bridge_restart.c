//Copyright Paul Reiche, Fred Ford. 1992-2026

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

// @plan PLAN-20260707-RESTARTMENU.P04
//
// Rust->C bridge helpers for the UQM restart/menu system.
//
// Provides real linkable symbols for macros, globals, and static
// functions that the Rust FFI cannot call directly. Most restart.c
// functions are already linkable extern symbols and do NOT need
// wrappers here; the Rust side calls them directly via extern "C".
//
// Only the following categories need C wrappers:
//   - Macros: GET_GAME_STATE, SET_GAME_STATE, GLOBAL, GLOBAL_SIS
//   - Globals: PlayerControl, GamePaused, MouseButtonDown, PulsedInputState
//   - Static functions: DrawRestartMenuGraphic, DrawRestartMenu

#include "rust_bridge_restart.h"

#include "starcon.h"
#include "globdata.h"       // GLOBAL, GLOBAL_SIS, GET_GAME_STATE, SET_GAME_STATE
#include "setup.h"          // LastActivity, PlayerControl, GamePaused, race_q
#include "controls.h"       // PulsedInputState, GamePaused, KEY_MENU_*
#include "menustat.h"       // MENU_STATE
#include "gamestr.h"        // GAME_STRING, MAINMENU_STRING_BASE
#include "libs/inplib.h"    // MouseButtonDown
#include "libs/timelib.h"   // GetTimeCounter, SleepThreadUntil, SleepThread
#include "flash.h"          // FlashContext, Flash_* functions
#include "starmap.h"        // star_array

// ---------------------------------------------------------------------------
// Game-state byte accessors (specific to restart.c usage)
// ---------------------------------------------------------------------------

BYTE
uqm_get_utwig_bomb_on_ship (void)
{
	return GET_GAME_STATE (UTWIG_BOMB_ON_SHIP);
}

void
uqm_set_utwig_bomb_on_ship (BYTE v)
{
	SET_GAME_STATE (UTWIG_BOMB_ON_SHIP, v);
}

BYTE
uqm_get_utwig_bomb (void)
{
	return GET_GAME_STATE (UTWIG_BOMB);
}

// ---------------------------------------------------------------------------
// PlayerControl global accessors
// ---------------------------------------------------------------------------

void
uqm_set_player_control (uint8_t player, COUNT control)
{
	PlayerControl[player] = control;
}

// ---------------------------------------------------------------------------
// Input state accessors
// ---------------------------------------------------------------------------

// PulsedInputState.menu[] is an array of BOOLEAN indexed by KEY_MENU_*.
BOOLEAN
uqm_get_pulsed_menu_key (uint8_t key_index)
{
	return PulsedInputState.menu[key_index];
}

BOOLEAN
uqm_get_mouse_button_down (void)
{
	return MouseButtonDown;
}

// ---------------------------------------------------------------------------
// Time globals
// ---------------------------------------------------------------------------

TimeCount
uqm_get_time_counter (void)
{
	return GetTimeCounter ();
}

void
uqm_sleep_thread_until (TimeCount time)
{
	SleepThreadUntil (time);
}

void
uqm_sleep_thread (TimeCount duration)
{
	SleepThread (duration);
}

// ---------------------------------------------------------------------------
// GamePaused global
// ---------------------------------------------------------------------------

void
uqm_set_game_paused (BOOLEAN val)
{
	GamePaused = val;
}

// ---------------------------------------------------------------------------
// Race queue reinit
// ---------------------------------------------------------------------------

void
uqm_reinit_race_queues (void)
{
	ReinitQueue (&race_q[0]);
	ReinitQueue (&race_q[1]);
}

// ---------------------------------------------------------------------------
// Global array assignment (from StartGame, restart.c:398-405)
// ---------------------------------------------------------------------------

void
uqm_assign_star_planet_globals (void)
{
	extern STAR_DESC starmap_array[];
	extern const BYTE element_array[];
	extern const PlanetFrame planet_array[];

	star_array = starmap_array;
	Elements = element_array;
	PlanData = planet_array;
}

// ---------------------------------------------------------------------------
// DoPopupWindow with a string ID (from DoRestart mouse handler)
// ---------------------------------------------------------------------------

void
uqm_do_popup_window_msg (COUNT string_id)
{
	// string_id is an offset from MAINMENU_STRING_BASE.
	DoPopupWindow (GAME_STRING (MAINMENU_STRING_BASE + string_id));
}

// ---------------------------------------------------------------------------
// Rust callback trampoline for DoInput InputFunc
// ---------------------------------------------------------------------------
// DoInput calls pMS->InputFunc(pMS) each frame. When USE_RUST_RESTART is
// defined, we set InputFunc to this trampoline, which calls into Rust.
// The Rust state is stored in pMS->privData.

#ifdef USE_RUST_RESTART

extern BOOLEAN rust_do_restart_frame (MENU_STATE *pMS);

static BOOLEAN
rust_restart_input_func (MENU_STATE *pMS)
{
	return rust_do_restart_frame (pMS);
}

void
uqm_set_rust_input_func (MENU_STATE *pMS)
{
	pMS->InputFunc = rust_restart_input_func;
}


// ---------------------------------------------------------------------------
// MENU_STATE field accessor for Rust
// ---------------------------------------------------------------------------

FRAME
uqm_get_menu_cur_frame (MENU_STATE *pMS)
{
	return pMS->CurFrame;
}

void *
uqm_get_menu_priv_data (MENU_STATE *pMS)
{
	return pMS->privData;
}

// Field setters — Rust writes back to MENU_STATE so C drawing functions
// (DrawRestartMenu, DrawRestartMenuGraphic) see the correct values.
void
uqm_set_menu_flash_context (MENU_STATE *pMS, FlashContext *ctx)
{
	pMS->flashContext = ctx;
}

void
uqm_set_menu_initialized (MENU_STATE *pMS, SIZE val)
{
	pMS->Initialized = val;
}

void
uqm_set_menu_cur_state (MENU_STATE *pMS, BYTE state)
{
	pMS->CurState = state;
}

void
uqm_set_menu_cur_frame (MENU_STATE *pMS, FRAME frame)
{
	pMS->CurFrame = frame;
}

void
uqm_set_menu_h_music (MENU_STATE *pMS, MUSIC_REF handle)
{
	pMS->hMusic = handle;
}

// ---------------------------------------------------------------------------
// MENU_STATE lifecycle: create / set-privData / destroy
// (Rust calls these because MENU_STATE is a C struct that must be allocated
//  on the C heap with InputFunc pointing to the Rust trampoline.)
// ---------------------------------------------------------------------------

MENU_STATE *
uqm_create_menu_state (void)
{
	MENU_STATE *pMS = (MENU_STATE *)HCalloc (sizeof (MENU_STATE));
	if (pMS)
	{
		pMS->InputFunc = rust_restart_input_func;
	}
	return pMS;
}

void
uqm_set_menu_priv_data (MENU_STATE *pMS, void *data)
{
	if (pMS)
		pMS->privData = data;
}

void
uqm_destroy_menu_state (MENU_STATE *pMS)
{
	if (pMS)
	{
		HFree (pMS);
	}
}
#endif /* USE_RUST_RESTART */
