/*
 * Rust VControl compatibility header
 *
 * When USE_RUST_INPUT is defined, this header provides macro mappings
 * from the C VControl_* API to the Rust rust_VControl_* FFI functions.
 * This allows existing C code to work with minimal changes.
 *
 * The pattern follows existing rust_oggaud.h approach: extern declarations
 * for Rust vtable/functions when feature flag is enabled.
 */

#ifndef LIBS_INPUT_SDL_RUST_VCONTROL_H_
#define LIBS_INPUT_SDL_RUST_VCONTROL_H_

#include "port.h"
#include SDL_INCLUDE(SDL.h)

#ifdef USE_RUST_INPUT

/* Include the Rust FFI declarations */
#include "rust_input.h"

#ifdef __cplusplus
extern "C" {
#endif

/* === C VControl API -> Rust Mapping === */

#if SDL_MAJOR_VERSION == 1
typedef SDLKey sdl_key_t;
#else
typedef SDL_Keycode sdl_key_t;
#endif

/* VCONTROL_GESTURE structure and types - must match C vcontrol.h */
typedef enum {
	VCONTROL_NONE,
	VCONTROL_KEY,
	VCONTROL_JOYAXIS,
	VCONTROL_JOYBUTTON,
	VCONTROL_JOYHAT,
	NUM_VCONTROL_GESTURES
} VCONTROL_GESTURE_TYPE;

/* This struct must be layout-compatible with both C and Rust FFI.
 * The union is named both 'gesture' for C input.c and 'data' as an alias.
 * The 'type' and 'gesture_type' fields share the same location. */
typedef struct {
	union {
		VCONTROL_GESTURE_TYPE type;
		VCONTROL_GESTURE_TYPE gesture_type;
	};
	union {
		struct {
			sdl_key_t key;
		};
		struct { int port, index, polarity; } axis;
		struct { int port, index; } button;
		struct { int port, index; Uint8 dir; } hat;
		int data[3];
	} gesture;
} VCONTROL_GESTURE;

/* Map C API calls to Rust FFI functions */
#define VControl_Init() rust_VControl_Init()
#define VControl_Uninit() rust_VControl_Uninit()
#define VControl_ResetInput() rust_VControl_ResetInput()
#define VControl_BeginFrame() rust_VControl_BeginFrame()

/* Keyboard bindings */
#define VControl_AddKeyBinding(sym, target) \
	rust_VControl_AddKeyBinding((sym), (target))
#define VControl_RemoveKeyBinding(sym, target) \
	rust_VControl_RemoveKeyBinding((sym), (target))

/* Joystick management */
#define VControl_InitJoystick(idx, name, axes, buttons, hats) \
	rust_VControl_InitJoystick((idx), (name), (axes), (buttons), (hats))
#define VControl_UninitJoystick(idx) \
	rust_VControl_UninitJoystick(idx)
#define VControl_GetNumJoysticks() \
	rust_VControl_GetNumJoysticks()

/* Joystick bindings */
#define VControl_AddJoyAxisBinding(port, axis, pol, target) \
	rust_VControl_AddJoyAxisBinding((port), (axis), (pol), (target))
#define VControl_RemoveJoyAxisBinding(port, axis, pol, target) \
	rust_VControl_RemoveJoyAxisBinding((port), (axis), (pol), (target))
#define VControl_SetJoyThreshold(port, thresh) \
	rust_VControl_SetJoyThreshold((port), (thresh))
#define VControl_AddJoyButtonBinding(port, button, target) \
	rust_VControl_AddJoyButtonBinding((port), (button), (target))
#define VControl_RemoveJoyButtonBinding(port, button, target) \
	rust_VControl_RemoveJoyButtonBinding((port), (button), (target))
#define VControl_AddJoyHatBinding(port, hat, dir, target) \
	rust_VControl_AddJoyHatBinding((port), (hat), (dir), (target))
#define VControl_RemoveJoyHatBinding(port, hat, dir, target) \
	rust_VControl_RemoveJoyHatBinding((port), (hat), (dir), (target))

/* Event processing */
#define VControl_ProcessKeyDown(sym) rust_VControl_ProcessKeyDown(sym)
#define VControl_ProcessKeyUp(sym) rust_VControl_ProcessKeyUp(sym)
#define VControl_ProcessJoyButtonDown(port, button) \
	rust_VControl_ProcessJoyButtonDown((port), (button))
#define VControl_ProcessJoyButtonUp(port, button) \
	rust_VControl_ProcessJoyButtonUp((port), (button))
#define VControl_ProcessJoyAxis(port, axis, value) \
	rust_VControl_ProcessJoyAxis((port), (axis), (value))
#define VControl_ProcessJoyHat(port, hat, value) \
	rust_VControl_ProcessJoyHat((port), (hat), (value))

/* Gesture tracking */
#define VControl_ClearGesture() rust_VControl_ClearGesture()
#define VControl_GetLastGestureType() rust_VControl_GetLastGestureType()

/* General */
#define VControl_RemoveAllBindings() rust_VControl_RemoveAllBindings()

/* VControl_HandleEvent needs special handling - implemented separately */
void VControl_HandleEvent(const SDL_Event *e);

/* Gesture parsing/dumping use Rust keyname functions */
extern int rust_VControl_name2code(const char *name);
extern const char *rust_VControl_code2name(int code);

/* C compatibility wrappers using Rust keynames */
#define VControl_name2code(name) rust_VControl_name2code(name)
#define VControl_code2name(code) rust_VControl_code2name(code)

/* VControl_AddGestureBinding needs wrapper for gesture struct */
int VControl_AddGestureBinding(VCONTROL_GESTURE *g, int *target);
void VControl_RemoveGestureBinding(VCONTROL_GESTURE *g, int *target);
int VControl_GetLastGesture(VCONTROL_GESTURE *g);
void VControl_ParseGesture(VCONTROL_GESTURE *g, const char *spec);
int VControl_DumpGesture(char *buf, int n, VCONTROL_GESTURE *g);

#ifdef __cplusplus
}
#endif

#endif /* USE_RUST_INPUT */

/* VControl constants (always defined, regardless of Rust/C choice) */
#define VCONTROL_STARTBIT 0x100
#define VCONTROL_MASK     0x0FF

#endif /* LIBS_INPUT_SDL_RUST_VCONTROL_H_ */
