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

// Prototypes for C helper functions in rust_bridge_ships.c
// that support the Rust FFI ship lifecycle entry points.

#ifndef UQM_RUST_BRIDGE_SHIPS_H_
#define UQM_RUST_BRIDGE_SHIPS_H_

#include "libs/compiler.h"
#include "races.h"
#include "element.h"

#if defined(__cplusplus)
extern "C" {
#endif

#ifdef USE_RUST_SHIPS

/* Lifecycle helpers called by Rust FFI -- see rust_bridge_ships.c */

BOOLEAN rust_bridge_spawn_element (STARSHIP *StarShipPtr,
		RACE_DESC *RDPtr, BYTE ship_mass, BYTE activity);

SIZE rust_bridge_init_battle_arena (void);

void rust_bridge_uninit_ships (void);

/* Layout verification -- all builds (layout mismatch = silent corruption) */
typedef struct
{
	size_t race_desc_size;
	size_t ship_data_offset;
	size_t ship_info_offset;
	size_t characteristics_offset;
	size_t ship_data_ship_offset;
	size_t ship_info_crew_offset;
	size_t ship_info_max_crew_offset;
	size_t characteristics_mass_offset;
} RACE_DESC_LAYOUT;

void rust_bridge_get_race_desc_layout (RACE_DESC_LAYOUT *out);

/* RaceDesc accessor functions (P05) -- used instead of direct field access
 * because Rust's RaceDesc is not #[repr(C)] and has different layout.
 * C calls these to safely read/write RaceDesc fields across the FFI boundary.
 * Defined in Rust: rust/src/ships/ffi.rs */

void *rust_race_desc_get_ship_frames (const void *rd);
BYTE rust_race_desc_get_ship_mass (const void *rd);
COUNT rust_race_desc_get_crew_level (const void *rd);
void rust_race_desc_set_crew_level (void *rd, COUNT crew);
COUNT rust_race_desc_get_max_crew (const void *rd);

/* Existing helpers */
BYTE uqm_get_current_activity_lobyte (void);

#endif /* USE_RUST_SHIPS */

#if defined(__cplusplus)
}
#endif

#endif /* UQM_RUST_BRIDGE_SHIPS_H_ */
