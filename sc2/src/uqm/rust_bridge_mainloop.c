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

// @plan PLAN-20260707-MAINLOOP.P02b
//
// Rust->C bridge helper functions for the UQM main loop.
//
// Provides real linkable symbols for macros, globals, and inline
// operations that the Rust FFI cannot call directly. The static-function
// wrappers (uqm_splash_with_bg_init_kernel, uqm_battle_with_frame_callback)
// live in starcon.c; this file holds everything that can be compiled in a
// separate translation unit because it only references extern globals and
// macros from public headers.

#include "rust_bridge_mainloop.h"

#include "starcon.h"     // MainExited
#include "globdata.h"    // GLOBAL, GLOBAL_SIS, GET_GAME_STATE, SET_GAME_STATE,
                        //   ACTIVITY, CurrentActivity, velocity
#include "save.h"        // NextActivity
#include "setup.h"       // LastActivity, SetPlayerInputAll
#include "sis.h"         // SetFlashRect
#include "velocity.h"    // ZeroVelocityComponents
#include "libs/misc.h"   // explode
#include "libs/log.h"    // log_add, log_Fatal

// ---------------------------------------------------------------------------
// Activity accessors
// ---------------------------------------------------------------------------
// Note: get_current_activity() is already defined in rust_bridge_macros.c
// (used by the battle module). Only define the rest here.

void
set_current_activity (UWORD v)
{
	GLOBAL (CurrentActivity) = v;
}

ACTIVITY
get_next_activity (void)
{
	return NextActivity;
}

void
set_next_activity (ACTIVITY v)
{
	NextActivity = v;
}

ACTIVITY
get_last_activity (void)
{
	return LastActivity;
}

void
set_last_activity (ACTIVITY v)
{
	LastActivity = v;
}

// ---------------------------------------------------------------------------
// Named game-state accessors (bit-packed via GET_GAME_STATE / SET_GAME_STATE)
// ---------------------------------------------------------------------------

BYTE
uqm_get_chmmr_bomb_state (void)
{
	return GET_GAME_STATE (CHMMR_BOMB_STATE);
}

void
uqm_set_chmmr_bomb_state (BYTE v)
{
	SET_GAME_STATE (CHMMR_BOMB_STATE, v);
}

BYTE
uqm_get_starbase_available (void)
{
	return GET_GAME_STATE (STARBASE_AVAILABLE);
}

BYTE
uqm_get_global_flags_and_data (void)
{
	return GET_GAME_STATE (GLOBAL_FLAGS_AND_DATA);
}

BYTE
uqm_get_kohr_ah_killed_all (void)
{
	return GET_GAME_STATE (KOHR_AH_KILLED_ALL);
}

COUNT
uqm_get_crew_enlisted (void)
{
	return GLOBAL_SIS (CrewEnlisted);
}

// ---------------------------------------------------------------------------
// Macro / global wrappers
// ---------------------------------------------------------------------------

void
uqm_zero_global_velocity (void)
{
	ZeroVelocityComponents (&GLOBAL (velocity));
}

void
uqm_set_flash_rect_null (void)
{
	SetFlashRect (NULL);
}

void
uqm_set_player_input_all_or_explode (void)
{
	if (!SetPlayerInputAll ())
	{
		log_add (log_Fatal, "Could not set player input.");
		explode ();  // Does not return.
	}
}

void
set_main_exited (BOOLEAN b)
{
	MainExited = b ? TRUE : FALSE;
}

/* Calls initAudio (snddriver, soundflags) — those two are C globals
 * (setup.c), so the Rust side cannot pass them directly. */
void
uqm_init_audio (void)
{
	extern sint32 initAudio (sint32 driver, sint32 flags);
	extern int snddriver, soundflags;
	initAudio (snddriver, soundflags);
}

// ---------------------------------------------------------------------------
// Directory-prep global accessors (options.c globals)
// ---------------------------------------------------------------------------

extern uio_Repository *repository;
extern uio_DirHandle *contentDir;
extern uio_DirHandle *configDir;
extern uio_DirHandle *saveDir;
extern uio_DirHandle *meleeDir;
extern uio_MountHandle *contentMountHandle;
extern char baseContentPath[];

uio_Repository *
uqm_get_repository (void) { return repository; }

uio_DirHandle *
uqm_get_config_dir (void) { return configDir; }

uio_DirHandle *
uqm_get_content_dir (void) { return contentDir; }

uio_MountHandle *
uqm_get_content_mount_handle (void) { return contentMountHandle; }

void
uqm_set_content_dir (uio_DirHandle *d) { contentDir = d; }

void
uqm_set_config_dir (uio_DirHandle *d) { configDir = d; }

void
uqm_set_save_dir (uio_DirHandle *d) { saveDir = d; }

void
uqm_set_melee_dir (uio_DirHandle *d) { meleeDir = d; }

void
uqm_set_content_mount_handle (uio_MountHandle *h) { contentMountHandle = h; }

void
uqm_set_base_content_path (const char *path)
{
	strncpy (baseContentPath, path, PATH_MAX - 1);
	baseContentPath[PATH_MAX - 1] = '\0';
}
