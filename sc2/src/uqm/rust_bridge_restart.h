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
// Prototypes for C wrapper functions that expose macros, globals, and
// static functions used by the restart/menu system, so the Rust FFI
// can link against real symbols.

#ifndef UQM_RUST_BRIDGE_RESTART_H_
#define UQM_RUST_BRIDGE_RESTART_H_

#include "libs/compiler.h"
#include "libs/timelib.h"    // TimeCount
#include "libs/gfxlib.h"     // FRAME, DRAWABLE
#include "libs/sndlib.h"     // MUSIC_REF
#include "flash.h"           // FlashContext
#include "menustat.h"        // MENU_STATE (full struct definition)

#if defined(__cplusplus)
extern "C" {
#endif

// Static compile-time checks for FFI ABI correctness.
_Static_assert (sizeof (UWORD) == 2, "UWORD must be 2 bytes for FFI");
_Static_assert (sizeof (BOOLEAN) == sizeof (int), "BOOLEAN must match int for FFI");
_Static_assert (sizeof (BYTE) == 1, "BYTE must be 1 byte for FFI");

// --- Game-state byte accessors (restart.c specific) ----------------------

BYTE uqm_get_utwig_bomb_on_ship (void);
void uqm_set_utwig_bomb_on_ship (BYTE v);
BYTE uqm_get_utwig_bomb (void);

// --- PlayerControl global ------------------------------------------------

void uqm_set_player_control (uint8_t player, COUNT control);

// --- Input state accessors -----------------------------------------------

// key_index must be one of KEY_MENU_SELECT, KEY_MENU_UP, etc. (controls.h)
BOOLEAN uqm_get_pulsed_menu_key (uint8_t key_index);
BOOLEAN uqm_get_mouse_button_down (void);

// --- Time globals --------------------------------------------------------

TimeCount uqm_get_time_counter (void);
void uqm_sleep_thread_until (TimeCount time);
void uqm_sleep_thread (TimeCount duration);

// --- GamePaused global ---------------------------------------------------

void uqm_set_game_paused (BOOLEAN val);

// --- Race queue reinit ---------------------------------------------------

void uqm_reinit_race_queues (void);

// --- Global array assignment (StartGame) ---------------------------------

void uqm_assign_star_planet_globals (void);

// --- Popup window --------------------------------------------------------

void uqm_do_popup_window_msg (COUNT string_id);

// --- MENU_STATE field accessor ---------------------------------------------

struct menu_state;
FRAME uqm_get_menu_cur_frame (struct menu_state *pMS);
void *uqm_get_menu_priv_data (struct menu_state *pMS);
void uqm_set_menu_flash_context (struct menu_state *pMS, FlashContext *ctx);
void uqm_set_menu_initialized (struct menu_state *pMS, SIZE val);
void uqm_set_menu_cur_state (struct menu_state *pMS, BYTE state);
void uqm_set_menu_cur_frame (struct menu_state *pMS, FRAME frame);
void uqm_set_menu_h_music (struct menu_state *pMS, MUSIC_REF handle);

struct menu_state *uqm_create_menu_state (void);
void uqm_set_menu_priv_data (struct menu_state *pMS, void *data);
void uqm_destroy_menu_state (struct menu_state *pMS);

// --- Rust callback setup (only when USE_RUST_RESTART is defined) ---------

#ifdef USE_RUST_RESTART

// Sets MENU_STATE.InputFunc to the Rust trampoline. The Rust side must
// set pMS->privData before calling this.
struct menu_state;
void uqm_set_rust_input_func (struct menu_state *pMS);

#endif /* USE_RUST_RESTART */

#if defined(__cplusplus)
}
#endif

#endif  /* UQM_RUST_BRIDGE_RESTART_H_ */
