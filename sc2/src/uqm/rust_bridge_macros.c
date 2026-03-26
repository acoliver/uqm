// Rust→C Bridge: Wrapper functions for macros and static-inline functions
//
// The Rust FFI declares these as extern "C" fn, but in C they are
// preprocessor macros or static inline functions that produce no linkable
// symbol.  This file provides real function symbols that Rust can link
// against.

#include "races.h"
#include "element.h"
#include "collide.h"
#include "displist.h"
#include "nameref.h"
#include "globdata.h"
#include "units.h"
#include "libs/strlib.h"
#include "libs/sndlib.h"
#include "libs/gfxlib.h"

// Undefine macros so we can define real functions with the same names
#undef InitIntersectStartPoint
#undef InitIntersectEndPoint
#undef GetElementStarShip
#undef SetElementStarShip
#undef GetLinkSize
#undef LoadGraphic
#undef LoadSound
#undef LoadMusic
#undef LoadStringTable
#undef CaptureSound
#undef ReleaseSound

// --- collide.h macros ---

void
InitIntersectStartPoint (ELEMENT *eptr)
{
	eptr->IntersectControl.IntersectStamp.origin.x =
			WORLD_TO_DISPLAY (eptr->current.location.x);
	eptr->IntersectControl.IntersectStamp.origin.y =
			WORLD_TO_DISPLAY (eptr->current.location.y);
}

void
InitIntersectEndPoint (ELEMENT *eptr)
{
	eptr->IntersectControl.EndPoint.x =
			WORLD_TO_DISPLAY (eptr->next.location.x);
	eptr->IntersectControl.EndPoint.y =
			WORLD_TO_DISPLAY (eptr->next.location.y);
}

// --- element.h macros ---

void
GetElementStarShip (const ELEMENT *e, STARSHIP **ppsd)
{
	*ppsd = e->pParent;
}

void
SetElementStarShip (ELEMENT *e, STARSHIP *psd)
{
	e->pParent = psd;
}

// --- displist.h static inlines / macros ---

void *
rust_bridge_LockLink (const QUEUE *pq, HLINK h)
{
	(void)pq;
	return (void *)h;
}

void
rust_bridge_UnlockLink (const QUEUE *pq, HLINK h)
{
	(void)pq;
	(void)h;
}

COUNT
rust_bridge_GetLinkSize (const QUEUE *pq)
{
	return (COUNT)(pq->object_size);
}

// --- nameref.h macros (Load* wrappers) ---

void *
LoadGraphic (RESOURCE res)
{
	return LoadGraphicInstance (res);
}

SOUND_REF
LoadSound (RESOURCE res)
{
	return LoadSoundInstance (res);
}

MUSIC_REF
LoadMusic (RESOURCE res)
{
	return LoadMusicInstance (res);
}

STRING_TABLE
LoadStringTable (RESOURCE res)
{
	return LoadStringTableInstance (res);
}

// --- sndlib.h macros ---

STRING
CaptureSound (SOUND_REF sound)
{
	return CaptureStringTable ((STRING_TABLE)sound);
}

STRING_TABLE
ReleaseSound (STRING sound)
{
	return ReleaseStringTable (sound);
}

// --- globdata.h: get_current_activity ---
// Rust battle/c_bridge.rs declares get_current_activity() -> u16.

UWORD
get_current_activity (void)
{
	return GLOBAL (CurrentActivity);
}
