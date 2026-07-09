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
// Prototypes for C wrapper functions that expose static functions,
// macros, and globals used by the UQM main loop, so the Rust FFI can
// link against real symbols.
//
// Static-function wrappers (uqm_splash_with_bg_init_kernel,
// uqm_battle_with_frame_callback) are defined in starcon.c because they
// reference static functions in that translation unit.
// All other wrappers are defined in rust_bridge_mainloop.c.

#ifndef UQM_RUST_BRIDGE_MAINLOOP_H_
#define UQM_RUST_BRIDGE_MAINLOOP_H_

#include "libs/compiler.h"

#if defined(__cplusplus)
extern "C" {
#endif

/* Compile-time ABI sanity checks. ACTIVITY is typedef'd to UWORD and
 * must be exactly 2 bytes so the Rust FFI can read/write it as a u16. */
_Static_assert (sizeof (UWORD) == 2, "UWORD must be 2 bytes for FFI");
_Static_assert (sizeof (BOOLEAN) == sizeof (int), "BOOLEAN must match int for FFI");

/* --- Static-function wrappers (defined in starcon.c) --------------------- */

/* Calls SplashScreen (BackgroundInitKernel). */
void uqm_splash_with_bg_init_kernel (void);

/* Calls Battle (&on_battle_frame). */
void uqm_battle_with_frame_callback (void);

/* --- Activity accessors -------------------------------------------------- */
/* ACTIVITY is typedef'd to UWORD in globdata.h; using UWORD here
 * avoids a heavy include dependency for this bridge header. */

UWORD get_current_activity (void);
void set_current_activity (UWORD v);

UWORD get_next_activity (void);
void set_next_activity (UWORD v);

UWORD get_last_activity (void);
void set_last_activity (UWORD v);

/* --- Named game-state accessors (GET_GAME_STATE / SET_GAME_STATE) -------- */

BYTE uqm_get_chmmr_bomb_state (void);
void uqm_set_chmmr_bomb_state (BYTE v);

BYTE uqm_get_starbase_available (void);
BYTE uqm_get_global_flags_and_data (void);
BYTE uqm_get_kohr_ah_killed_all (void);

/* GLOBAL_SIS (CrewEnlisted) as a COUNT (UWORD). */
COUNT uqm_get_crew_enlisted (void);

/* --- Macro / global wrappers -------------------------------------------- */

/* Wraps ZeroVelocityComponents (&GLOBAL (velocity)). */
void uqm_zero_global_velocity (void);

/* Wraps SetFlashRect (NULL). */
void uqm_set_flash_rect_null (void);

/* Calls SetPlayerInputAll (); on failure logs a fatal message and calls
 * explode () (does not return). Mirrors the C main loop's behavior. */
void uqm_set_player_input_all_or_explode (void);

/* Sets the MainExited global to the given BOOLEAN value. */
void set_main_exited (BOOLEAN b);

/* Calls initAudio (snddriver, soundflags). */
void uqm_init_audio (void);

/* --- Directory-prep global accessors (options.c globals) ---------------- */

struct uio_Repository;
struct uio_DirHandle;
struct uio_MountHandle;

struct uio_Repository *uqm_get_repository (void);
struct uio_DirHandle *uqm_get_config_dir (void);
struct uio_DirHandle *uqm_get_content_dir (void);
struct uio_MountHandle *uqm_get_content_mount_handle (void);

void uqm_set_content_dir (struct uio_DirHandle *d);
void uqm_set_config_dir (struct uio_DirHandle *d);
void uqm_set_save_dir (struct uio_DirHandle *d);
void uqm_set_melee_dir (struct uio_DirHandle *d);
void uqm_set_content_mount_handle (struct uio_MountHandle *h);
void uqm_set_base_content_path (const char *path);

#if defined(__cplusplus)
}
#endif

#endif  /* UQM_RUST_BRIDGE_MAINLOOP_H_ */
