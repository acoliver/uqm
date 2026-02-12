/*
 * Rust VControl wrapper implementation
 *
 * This file provides C wrapper functions that integrate with the Rust
 * FFI layer when USE_RUST_INPUT is defined. It handles VControl_HandleEvent
 * and gesture tracking functions that need special handling beyond simple
 * macro mappings.
 *
 * When USE_RUST_INPUT is not defined, this file is not compiled.
 */

#ifdef USE_RUST_INPUT

#include "port.h"
#include SDL_INCLUDE(SDL.h)
#include "rust_vcontrol.h"
#include <stdio.h>

/* Declare rust_bridge_log from Rust */
extern int rust_bridge_log(const char *message);

/* === VControl_HandleEvent wrapper === */
/* This function calls the Rust FFI rust_VControl_HandleEvent which
 * processes SDL events and updates bound targets */

void VControl_HandleEvent(const SDL_Event *e)
{
	if (e == NULL)
		return;

	/* Debug log key events */
	if (e->type == SDL_KEYDOWN || e->type == SDL_KEYUP) {
		char buf[128];
		snprintf(buf, sizeof(buf), "C_VCONTROL: %s sym=0x%X",
			e->type == SDL_KEYDOWN ? "KeyDown" : "KeyUp",
			e->key.keysym.sym);
		rust_bridge_log(buf);
	}

	/* The Rust implementation expects a const void* for SDL_Event */
	rust_VControl_HandleEvent((const void *)e);
}

/* === VControl_AddGestureBinding wrapper === */
/* Converts C VCONTROL_GESTURE struct to Rust representation */

int VControl_AddGestureBinding(VCONTROL_GESTURE *g, int *target)
{
	if (g == NULL || target == NULL)
		return -1;

	/* The Rust FFI expects a VCONTROL_GESTURE pointer */
	return rust_VControl_AddGestureBinding(g, target);
}

/* === VControl_RemoveGestureBinding wrapper === */

void VControl_RemoveGestureBinding(VCONTROL_GESTURE *g, int *target)
{
	if (g == NULL || target == NULL)
		return;

	rust_VControl_RemoveGestureBinding(g, target);
}

/* === VControl_GetLastGesture wrapper === */
/* Gets the last gesture from Rust implementation */

int VControl_GetLastGesture(VCONTROL_GESTURE *g)
{
	if (g == NULL)
		return 0;

	return rust_VControl_GetLastGesture(g);
}

/* === VControl_ParseGesture wrapper === */
/* Parses gesture string specification using Rust implementation */

void VControl_ParseGesture(VCONTROL_GESTURE *g, const char *spec)
{
	if (g == NULL || spec == NULL)
		return;

	rust_VControl_ParseGesture(g, spec);
}

/* === VControl_DumpGesture wrapper === */
/* Dumps gesture to string buffer using Rust implementation */

int VControl_DumpGesture(char *buf, int n, VCONTROL_GESTURE *g)
{
	if (buf == NULL || g == NULL || n <= 0)
		return 0;

	return rust_VControl_DumpGesture(buf, n, g);
}

#endif /* USE_RUST_INPUT */
