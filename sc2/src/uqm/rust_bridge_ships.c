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

// Rust→C Bridge Helper Functions for Ships Subsystem
// Provides C wrapper functions for macros, globals, and inline operations
// that Rust FFI cannot call directly.

#include "globdata.h"
#include "libs/compiler.h"
#include "element.h"
#include "weapon.h"
#include "intel.h"
#include "races.h"
#include "collide.h"
#include "colors.h"
#include "status.h"
#include "sounds.h"
#include "units.h"
#include "libs/mathlib.h"
#include "libs/sndlib.h"

// Returns LOBYTE(GLOBAL(CurrentActivity))
// Used by rust_ships_spawn() and rust_ships_init() to determine game mode
BYTE
uqm_get_current_activity_lobyte(void)
{
	return LOBYTE(GLOBAL(CurrentActivity));
}

// ---------------------------------------------------------------------------
// Element operations (C macros over disp_q — need real function symbols)
// ---------------------------------------------------------------------------

void
rust_bridge_PutElement (HELEMENT h)
{
	PutElement (h);
}

void
rust_bridge_InsertElement (HELEMENT h, HELEMENT after)
{
	InsertElement (h, after);
}

HELEMENT
rust_bridge_GetHeadElement (void)
{
	return GetHeadElement ();
}

HELEMENT
rust_bridge_GetTailElement (void)
{
	return GetTailElement ();
}

void
rust_bridge_LockElement (HELEMENT h, ELEMENT **ppe)
{
	LockElement (h, ppe);
}

void
rust_bridge_UnlockElement (HELEMENT h)
{
	UnlockElement (h);
}

HELEMENT
rust_bridge_GetPredElement (ELEMENT *e)
{
	return GetPredElement (e);
}

HELEMENT
rust_bridge_GetSuccElement (ELEMENT *e)
{
	return GetSuccElement (e);
}

UWORD
rust_bridge_GetFrameIndex (FRAME f)
{
	return GetFrameIndex (f);
}

// ---------------------------------------------------------------------------
// Weapon creation
// ---------------------------------------------------------------------------

HELEMENT
rust_bridge_initialize_missile (MISSILE_BLOCK *block)
{
	return initialize_missile (block);
}

HELEMENT
rust_bridge_initialize_laser (LASER_BLOCK *block)
{
	return initialize_laser (block);
}

// ---------------------------------------------------------------------------
// AI / Intelligence
// ---------------------------------------------------------------------------

void
rust_bridge_ship_intelligence (ELEMENT *ShipPtr,
		EVALUATE_DESC *ObjectsOfConcern, COUNT ConcernCounter)
{
	ship_intelligence (ShipPtr, ObjectsOfConcern, ConcernCounter);
}

// ---------------------------------------------------------------------------
// Sound helpers (SetAbsSoundIndex is a macro)
// ---------------------------------------------------------------------------

uintptr_t
rust_bridge_SetAbsSoundIndex (uintptr_t sounds, COUNT index)
{
	return (uintptr_t)SetAbsSoundIndex ((SOUND)sounds, index);
}

// ---------------------------------------------------------------------------
// Coordinate conversion macros
// ---------------------------------------------------------------------------

SDWORD
rust_bridge_DISPLAY_TO_WORLD (SDWORD x)
{
	return DISPLAY_TO_WORLD (x);
}

SDWORD
rust_bridge_WORLD_TO_DISPLAY (SDWORD x)
{
	return WORLD_TO_DISPLAY (x);
}

COUNT
rust_bridge_NORMALIZE_FACING (COUNT f)
{
	return NORMALIZE_FACING (f);
}

COUNT
rust_bridge_FACING_TO_ANGLE (COUNT f)
{
	return FACING_TO_ANGLE (f);
}

SDWORD
rust_bridge_SINE (COUNT angle, SIZE magnitude)
{
	return SINE (angle, magnitude);
}

SDWORD
rust_bridge_COSINE (COUNT angle, SIZE magnitude)
{
	return COSINE (angle, magnitude);
}

COUNT
rust_bridge_ARCTAN (SDWORD dx, SDWORD dy)
{
	return ARCTAN (dx, dy);
}

SDWORD
rust_bridge_WRAP_X (SDWORD x)
{
	return WRAP_X (x);
}

SDWORD
rust_bridge_WRAP_Y (SDWORD y)
{
	return WRAP_Y (y);
}

// ---------------------------------------------------------------------------
// Element state flag helpers (CollidingElement, OBJECT_CLOAKED are macros)
// ---------------------------------------------------------------------------

BOOLEAN
rust_bridge_CollidingElement (ELEMENT *e)
{
	return CollidingElement (e);
}

BOOLEAN
rust_bridge_OBJECT_CLOAKED (ELEMENT *e)
{
	return OBJECT_CLOAKED (e);
}

// ---------------------------------------------------------------------------
// DeltaEnergy / DeltaCrew (not macros but in ship.c, may need linking)
// ---------------------------------------------------------------------------

BOOLEAN
rust_bridge_DeltaEnergy (ELEMENT *ElementPtr, SIZE energy_delta)
{
	return DeltaEnergy (ElementPtr, energy_delta);
}

BOOLEAN
rust_bridge_DeltaCrew (ELEMENT *ElementPtr, SIZE crew_delta)
{
	return DeltaCrew (ElementPtr, crew_delta);
}

// ---------------------------------------------------------------------------
// Starship element association (macros in element.h)
// ---------------------------------------------------------------------------

void
rust_bridge_GetElementStarShip (ELEMENT *e, STARSHIP **ss)
{
	GetElementStarShip (e, ss);
}

void
rust_bridge_SetElementStarShip (ELEMENT *e, STARSHIP *ss)
{
	SetElementStarShip (e, ss);
}

// ---------------------------------------------------------------------------
// Misc helpers ships need
// ---------------------------------------------------------------------------

SIZE
rust_bridge_TrackShip (ELEMENT *e, COUNT *pfacing)
{
	return TrackShip (e, pfacing);
}

void
rust_bridge_Untarget (ELEMENT *e)
{
	Untarget (e);
}

FRAME
rust_bridge_ModifySilhouette (ELEMENT *e, STAMP *s, BYTE flags)
{
	return ModifySilhouette (e, s, flags);
}

void
rust_bridge_ProcessSound (uintptr_t sound, ELEMENT *source)
{
	ProcessSound ((SOUND)sound, source);
}

HELEMENT
rust_bridge_weapon_collision (ELEMENT *e0, POINT *p0, ELEMENT *e1, POINT *p1)
{
	return weapon_collision (e0, p0, e1, p1);
}

// Note: CrewDied wrapper deferred — no such C function exists.
// Ships handle crew death through DeltaCrew and death_func callbacks.
