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

/* New includes for lifecycle helpers (P00) */
#include "ship.h"       /* ship_preprocess, ship_postprocess, collision */
#include "tactrans.h"   /* ship_death, new_ship */
#include "init.h"       /* InitSpace, UninitSpace, NUM_PLAYERS, NUM_SIDES */
#include "build.h"      /* queue operations, Build, race_q, Lock/UnlockStarShip */
#include "hyper.h"      /* LoadHyperspace, FreeHyperspace, inHQSpace */
#include "process.h"    /* CalculateGravity, TimeSpaceMatterConflict */
#include "setup.h"      /* SpaceContext, StatusContext, ScreenContext, Screen */
#include "encount.h"    /* UpdateShipFragCrew, FleetIsInfinite */
#include "cons_res.h"   /* free_gravity_well */
#include "libs/log.h"   /* log_add for debug assertions (C3) */

/* Own header -- prototype declarations */
#include "rust_bridge_ships.h"

#ifdef USE_RUST_SHIPS

// ---------------------------------------------------------------------------
// P05 — Compile-time sanity checks for RACE_DESC layout
// ---------------------------------------------------------------------------
// These catch gross layout errors (empty struct, reordered top-level fields).
// The authoritative runtime check is in Rust's verify_race_desc_layout().

#include <stddef.h>

_Static_assert (sizeof (RACE_DESC) > 0,
		"RACE_DESC must be non-empty");
_Static_assert (offsetof (RACE_DESC, ship_info)
		< offsetof (RACE_DESC, ship_data),
		"ship_info must precede ship_data in RACE_DESC");
_Static_assert (offsetof (RACE_DESC, ship_data)
		> offsetof (RACE_DESC, characteristics),
		"ship_data must follow characteristics in RACE_DESC");

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

// ===========================================================================
// P00 — Lifecycle helpers
// ===========================================================================

/* Copied from init.c:CountCrewElements() which is static in its origin TU.
 * STALENESS WARNING: If init.c's CountCrewElements() is modified, this copy
 * must be updated manually. Grep for "rust_bridge_CountCrewElements" to find
 * this copy. Last synced with init.c line 252. */
static COUNT
rust_bridge_CountCrewElements (void)
{
	COUNT result;
	HELEMENT hElement, hNextElement;

	result = 0;
	for (hElement = GetHeadElement ();
			hElement != 0; hElement = hNextElement)
	{
		ELEMENT *ElementPtr;

		LockElement (hElement, &ElementPtr);
		hNextElement = GetSuccElement (ElementPtr);
		if (ElementPtr->state_flags & CREW_OBJECT)
			++result;

		UnlockElement (hElement);
	}

	return result;
}

BOOLEAN
rust_bridge_spawn_element (STARSHIP *StarShipPtr, RACE_DESC *RDPtr,
		BYTE ship_mass, BYTE activity)
{
	HELEMENT hShip;

#ifndef NDEBUG
	/* L1: Verify Rust-provided activity matches C global.
	 * Catches timing drift between Rust's read and C's use. */
	assert (activity == LOBYTE (GLOBAL (CurrentActivity))
		&& "activity parameter does not match GLOBAL(CurrentActivity)");
#endif

	/* --- Branch A vs B: hShip==0 means fresh alloc, hShip!=0 means reuse --- */
	hShip = StarShipPtr->hShip;
	if (hShip == 0)
	{
		/* Branch A: fresh allocation */
		hShip = AllocElement ();
		if (hShip != 0)
			InsertElement (hShip, GetHeadElement ());
	}
	/* Branch B: hShip != 0 -- reuse existing element handle.
	 * No AllocElement/InsertElement needed; element is already in display list.
	 * All fields below are still overwritten unconditionally (C2 parity). */

	StarShipPtr->hShip = hShip;
	if (StarShipPtr->hShip != 0)
	{
		/* Common path for BOTH branches -- all fields set unconditionally */
		ELEMENT *ShipElementPtr;

		LockElement (hShip, &ShipElementPtr);

		ShipElementPtr->playerNr = StarShipPtr->playerNr;
		ShipElementPtr->crew_level = 0;
		ShipElementPtr->mass_points = ship_mass;
		ShipElementPtr->state_flags = APPEARING | PLAYER_SHIP | IGNORE_SIMILAR;
		ShipElementPtr->turn_wait = 0;
		ShipElementPtr->thrust_wait = 0;
		ShipElementPtr->life_span = NORMAL_LIFE;
		ShipElementPtr->colorCycleIndex = 0;

		SetPrimType (&DisplayArray[ShipElementPtr->PrimIndex], STAMP_PRIM);
		ShipElementPtr->current.image.farray =
				(FRAME *)rust_race_desc_get_ship_frames (RDPtr);

		if (ShipElementPtr->playerNr == NPC_PLAYER_NUM
				&& activity == IN_LAST_BATTLE)
		{
			/* Sa-Matra special case */
#ifndef NDEBUG
			assert (ShipElementPtr->playerNr == NPC_PLAYER_NUM
				&& "Sa-Matra path requires NPC_PLAYER_NUM");
#endif
			StarShipPtr->ShipFacing = 0;
			ShipElementPtr->current.image.frame =
					SetAbsFrameIndex (
					((FRAME *)rust_race_desc_get_ship_frames (RDPtr))[0],
					StarShipPtr->ShipFacing);
			ShipElementPtr->current.location.x = LOG_SPACE_WIDTH >> 1;
			ShipElementPtr->current.location.y = LOG_SPACE_HEIGHT >> 1;
			++ShipElementPtr->life_span;
		}
		else
		{
			StarShipPtr->ShipFacing = NORMALIZE_FACING (TFB_Random ());
			if (inHQSpace ())
			{
				/* Only one ship is ever spawned in HyperSpace -- flagship */
				COUNT facing = GLOBAL (ShipFacing);
				/* Solar system reentry test depends on ShipFacing != 0 */
				if (facing > 0)
					--facing;

				StarShipPtr->ShipFacing = facing;
			}
			ShipElementPtr->current.image.frame =
					SetAbsFrameIndex (
					((FRAME *)rust_race_desc_get_ship_frames (RDPtr))[0],
					StarShipPtr->ShipFacing);
			do
			{
				ShipElementPtr->current.location.x =
						WRAP_X (DISPLAY_ALIGN_X (TFB_Random ()));
				ShipElementPtr->current.location.y =
						WRAP_Y (DISPLAY_ALIGN_Y (TFB_Random ()));
			} while (CalculateGravity (ShipElementPtr)
					|| TimeSpaceMatterConflict (ShipElementPtr));
		}

		/* Callbacks -- set in BOTH branches (C2 parity) */
		ShipElementPtr->preprocess_func = ship_preprocess;
		ShipElementPtr->postprocess_func = ship_postprocess;
		ShipElementPtr->death_func = ship_death;
		ShipElementPtr->collision_func = collision;
		ZeroVelocityComponents (&ShipElementPtr->velocity);

		SetElementStarShip (ShipElementPtr, StarShipPtr);
		ShipElementPtr->hTarget = 0;

		UnlockElement (hShip);
	}

	return (hShip != 0);
}

SIZE
rust_bridge_init_battle_arena (void)
{
	SIZE num_ships;

	InitSpace ();

	SetContext (StatusContext);
	SetContext (SpaceContext);

	InitDisplayList ();
	InitGalaxy ();

	if (inHQSpace ())
	{
		ReinitQueue (&race_q[0]);
		ReinitQueue (&race_q[1]);

		/* Inlined from init.c:BuildSIS() -- static in origin file.
		 * STALENESS WARNING: If init.c's BuildSIS() is modified, this
		 * inlined copy must be updated manually. Grep for "BuildSIS"
		 * in rust_bridge_ships.c to find this copy.
		 * Last synced with init.c line 164. */
		{
			HSTARSHIP hStarShip;
			STARSHIP *StarShipPtr;

			hStarShip = Build (&race_q[0], SIS_SHIP_ID);
			if (hStarShip)
			{
				StarShipPtr = LockStarShip (&race_q[0], hStarShip);
				StarShipPtr->playerNr = RPG_PLAYER_NUM;
				StarShipPtr->captains_name_index = 0;
				UnlockStarShip (&race_q[0], hStarShip);
			}
		}

		LoadHyperspace ();

		num_ships = 1;
	}
	else
	{
		COUNT i;
		RECT r;

		SetContextFGFrame (Screen);
		r.corner.x = SAFE_X;
		r.corner.y = SAFE_Y;
		r.extent.width = SPACE_WIDTH;
		r.extent.height = SPACE_HEIGHT;
		SetContextClipRect (&r);

		SetContextBackGroundColor (BLACK_COLOR);
		{
			CONTEXT OldContext;

			OldContext = SetContext (ScreenContext);

			SetContextBackGroundColor (BLACK_COLOR);
			ClearDrawable ();

			SetContext (OldContext);
		}

		if (LOBYTE (GLOBAL (CurrentActivity)) == IN_LAST_BATTLE)
			free_gravity_well ();
		else
		{
#define NUM_ASTEROIDS 5
			for (i = 0; i < NUM_ASTEROIDS; ++i)
				spawn_asteroid (NULL);
#define NUM_PLANETS 1
			for (i = 0; i < NUM_PLANETS; ++i)
				spawn_planet ();
		}

		num_ships = NUM_SIDES;
	}

	return (num_ships);
}

void
rust_bridge_uninit_ships (void)
{
	COUNT crew_retrieved;
	int i;
	HELEMENT hElement, hNextElement;
	STARSHIP *SPtr[NUM_PLAYERS];

	StopSound ();

	UninitSpace ();

	for (i = 0; i < NUM_PLAYERS; ++i)
		SPtr[i] = 0;

	crew_retrieved = rust_bridge_CountCrewElements ();

#ifndef NDEBUG
	/* C3: Log state at entry for debugging desync between Rust and C */
	log_add (log_Debug, "rust_bridge_uninit_ships: crew_retrieved=%u",
			(unsigned)crew_retrieved);
#endif

	for (hElement = GetHeadElement ();
			hElement != 0; hElement = hNextElement)
	{
		ELEMENT *ElementPtr;

		/* C3 Guard 1: Lock element and validate pointer */
		LockElement (hElement, &ElementPtr);
		hNextElement = GetSuccElement (ElementPtr);
		if ((ElementPtr->state_flags & PLAYER_SHIP)
				|| ElementPtr->death_func == new_ship)
		{
			STARSHIP *StarShipPtr;

			/* C3 Guard 2: Extract starship pointer from element */
			GetElementStarShip (ElementPtr, &StarShipPtr);

			/* C3 Guard 3: Validate StarShipPtr before ANY dereference.
			 * MANDATORY -- not optional diagnostics. Fires in all builds.
			 * This handles: partial spawn failure, stale element refs,
			 * panic-path desync where element exists but starship is gone. */
			if (StarShipPtr == NULL)
			{
#ifndef NDEBUG
				log_add (log_Debug,
						"rust_bridge_uninit_ships: null StarShipPtr, "
						"skipping element");
#endif
				UnlockElement (hElement);
				continue;
			}

			/* C3 Guard 4: Validate RaceDescPtr before field access.
			 * MANDATORY -- not optional diagnostics. Fires in all builds.
			 * This handles: descriptor already freed, init failure before
			 * descriptor was set, double-uninit where first pass freed it. */
			if (StarShipPtr->RaceDescPtr == NULL)
			{
#ifndef NDEBUG
				log_add (log_Debug,
						"rust_bridge_uninit_ships: null RaceDescPtr on "
						"StarShipPtr=%p, skipping", (void *)StarShipPtr);
#endif
				UnlockElement (hElement);
				continue;
			}

			/* C3: All guards passed -- use accessor functions for
			 * RaceDesc field access (layout differs between C and Rust) */
			{
				COUNT crew =
						rust_race_desc_get_crew_level (StarShipPtr->RaceDescPtr);
				COUNT max_crew =
						rust_race_desc_get_max_crew (StarShipPtr->RaceDescPtr);

				if (crew)
				{
					if (crew_retrieved >= max_crew - crew)
						crew = max_crew;
					else
						crew += crew_retrieved;
				}

				rust_race_desc_set_crew_level (StarShipPtr->RaceDescPtr, crew);
				StarShipPtr->crew_level = crew;
			}
			SPtr[StarShipPtr->playerNr] = StarShipPtr;
			free_ship (StarShipPtr->RaceDescPtr, TRUE, TRUE);
			/* Post-free nulling -- prevents double-free on same element */
			StarShipPtr->RaceDescPtr = 0;
		}
		UnlockElement (hElement);
	}

	GLOBAL (CurrentActivity) &= ~IN_BATTLE;

	if (LOBYTE (GLOBAL (CurrentActivity)) == IN_ENCOUNTER
			&& !(GLOBAL (CurrentActivity) & CHECK_ABORT))
	{
		for (i = NUM_PLAYERS - 1; i >= 0; --i)
		{
			if (SPtr[i] && !FleetIsInfinite (i))
				UpdateShipFragCrew (SPtr[i]);
		}
	}

	if (LOBYTE (GLOBAL (CurrentActivity)) != IN_ENCOUNTER)
	{
		for (i = 0; i < NUM_PLAYERS; i++)
			ReinitQueue (&race_q[i]);

		if (inHQSpace ())
			FreeHyperspace ();
	}

#ifndef NDEBUG
	log_add (log_Debug, "rust_bridge_uninit_ships: teardown complete");
#endif
}

void
rust_bridge_get_race_desc_layout (RACE_DESC_LAYOUT *out)
{
	out->race_desc_size = sizeof (RACE_DESC);
	out->ship_data_offset = offsetof (RACE_DESC, ship_data);
	out->ship_info_offset = offsetof (RACE_DESC, ship_info);
	out->characteristics_offset = offsetof (RACE_DESC, characteristics);
	out->ship_data_ship_offset = offsetof (DATA_STUFF, ship);
	out->ship_info_crew_offset = offsetof (SHIP_INFO, crew_level);
	out->ship_info_max_crew_offset = offsetof (SHIP_INFO, max_crew);
	out->characteristics_mass_offset = offsetof (CHARACTERISTIC_STUFF, ship_mass);
}

#endif /* USE_RUST_SHIPS */
