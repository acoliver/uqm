/*
 *  Rust VControl bindings for C
 *
 *  When USE_RUST_INPUT is defined, these Rust implementations replace
 *  the C VControl functions.
 */

#ifndef LIBS_INPUT_SDL_RUST_VCONTROL_H_
#define LIBS_INPUT_SDL_RUST_VCONTROL_H_

#include "port.h"
#include SDL_INCLUDE(SDL.h)

#ifdef USE_RUST_INPUT

#if SDL_MAJOR_VERSION == 1
typedef SDLKey sdl_key_t;
#else
typedef SDL_Keycode sdl_key_t;
#endif

/* Gesture type enum - must match C version */
typedef enum {
	VCONTROL_NONE,
	VCONTROL_KEY,
	VCONTROL_JOYAXIS,
	VCONTROL_JOYBUTTON,
	VCONTROL_JOYHAT,
	NUM_VCONTROL_GESTURES
} VCONTROL_GESTURE_TYPE;

/* Gesture structure - must match C version */
typedef struct {
	VCONTROL_GESTURE_TYPE type;
	union {
		sdl_key_t key;
		struct { int port, index, polarity; } axis;
		struct { int port, index; } button;
		struct { int port, index; Uint8 dir; } hat;
	} gesture;
} VCONTROL_GESTURE;

/* Rust VControl FFI functions */
extern int rust_VControl_Init(void);
extern void rust_VControl_Uninit(void);
extern void rust_VControl_ResetInput(void);
extern void rust_VControl_BeginFrame(void);
extern void rust_VControl_RemoveAllBindings(void);

/* Key bindings */
extern int rust_VControl_AddKeyBinding(int symbol, int *target);
extern int rust_VControl_RemoveKeyBinding(int symbol, int *target);
extern void rust_VControl_ClearKeyBindings(void);
extern void rust_VControl_ProcessKeyDown(int symbol);
extern void rust_VControl_ProcessKeyUp(int symbol);

/* Joystick management */
extern int rust_VControl_InitJoystick(int index, const char *name, int num_axes, int num_buttons, int num_hats);
extern int rust_VControl_UninitJoystick(int index);
extern int rust_VControl_GetNumJoysticks(void);

/* Joystick bindings */
extern int rust_VControl_AddJoyAxisBinding(int port, int axis, int polarity, int *target);
extern int rust_VControl_RemoveJoyAxisBinding(int port, int axis, int polarity, int *target);
extern int rust_VControl_AddJoyButtonBinding(int port, int button, int *target);
extern int rust_VControl_RemoveJoyButtonBinding(int port, int button, int *target);
extern int rust_VControl_AddJoyHatBinding(int port, int which, unsigned char dir, int *target);
extern int rust_VControl_RemoveJoyHatBinding(int port, int which, unsigned char dir, int *target);
extern int rust_VControl_SetJoyThreshold(int port, int threshold);
extern int rust_VControl_ClearJoyBindings(int joy);

/* Joystick event processing */
extern void rust_VControl_ProcessJoyButtonDown(int port, int button);
extern void rust_VControl_ProcessJoyButtonUp(int port, int button);
extern void rust_VControl_ProcessJoyAxis(int port, int axis, int value);
extern void rust_VControl_ProcessJoyHat(int port, int which, unsigned char value);

/* Gesture tracking and handling */
extern void rust_VControl_ClearGesture(void);
extern int rust_VControl_GetLastGesture(VCONTROL_GESTURE *g);
extern void rust_VControl_HandleEvent(const SDL_Event *e);
extern int rust_VControl_AddGestureBinding(VCONTROL_GESTURE *g, int *target);
extern void rust_VControl_RemoveGestureBinding(VCONTROL_GESTURE *g, int *target);
extern void rust_VControl_ParseGesture(VCONTROL_GESTURE *g, const char *spec);
extern int rust_VControl_DumpGesture(char *buf, int n, VCONTROL_GESTURE *g);

/* Map the C VControl_* names to rust_VControl_* when USE_RUST_INPUT is enabled */
#define VControl_Init                   rust_VControl_Init
#define VControl_Uninit                 rust_VControl_Uninit
#define VControl_ResetInput             rust_VControl_ResetInput
#define VControl_BeginFrame             rust_VControl_BeginFrame
#define VControl_RemoveAllBindings      rust_VControl_RemoveAllBindings

#define VControl_AddKeyBinding(sym, tgt) rust_VControl_AddKeyBinding((int)(sym), (tgt))
#define VControl_RemoveKeyBinding(sym, tgt) rust_VControl_RemoveKeyBinding((int)(sym), (tgt))
#define VControl_ProcessKeyDown(sym)    rust_VControl_ProcessKeyDown((int)(sym))
#define VControl_ProcessKeyUp(sym)      rust_VControl_ProcessKeyUp((int)(sym))

#define VControl_AddJoyAxisBinding      rust_VControl_AddJoyAxisBinding
#define VControl_RemoveJoyAxisBinding   rust_VControl_RemoveJoyAxisBinding
#define VControl_SetJoyThreshold        rust_VControl_SetJoyThreshold
#define VControl_AddJoyButtonBinding    rust_VControl_AddJoyButtonBinding
#define VControl_RemoveJoyButtonBinding rust_VControl_RemoveJoyButtonBinding
#define VControl_AddJoyHatBinding       rust_VControl_AddJoyHatBinding
#define VControl_RemoveJoyHatBinding    rust_VControl_RemoveJoyHatBinding

#define VControl_ClearGesture           rust_VControl_ClearGesture
#define VControl_GetLastGesture         rust_VControl_GetLastGesture
#define VControl_HandleEvent            rust_VControl_HandleEvent
#define VControl_AddGestureBinding      rust_VControl_AddGestureBinding
#define VControl_RemoveGestureBinding   rust_VControl_RemoveGestureBinding
#define VControl_ParseGesture           rust_VControl_ParseGesture
#define VControl_DumpGesture            rust_VControl_DumpGesture

#define VControl_ProcessJoyButtonDown   rust_VControl_ProcessJoyButtonDown
#define VControl_ProcessJoyButtonUp     rust_VControl_ProcessJoyButtonUp
#define VControl_ProcessJoyAxis         rust_VControl_ProcessJoyAxis
#define VControl_ProcessJoyHat          rust_VControl_ProcessJoyHat

#endif /* USE_RUST_INPUT */

#endif /* LIBS_INPUT_SDL_RUST_VCONTROL_H_ */
