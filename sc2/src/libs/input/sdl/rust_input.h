/*
 * Rust Input/VControl FFI Header
 *
 * Provides extern declarations for the Rust-implemented input system.
 * When USE_RUST_INPUT is defined, these functions are used instead of
 * the C implementation in vcontrol.c.
 *
 * The Rust implementation provides:
 * - Virtual Control abstraction for keyboard/joystick binding
 * - Gesture tracking for input configuration
 * - Thread-safe state management via RwLock
 * - Compatible with existing SDL event handling
 */

#ifndef LIBS_INPUT_SDL_RUST_INPUT_H_
#define LIBS_INPUT_SDL_RUST_INPUT_H_

#include "port.h"
#include SDL_INCLUDE(SDL.h)

#ifdef __cplusplus
extern "C" {
#endif

/* === Initialization === */

/**
 * Initialize the VControl input system
 * Returns 0 on success, -1 on failure
 */
int rust_VControl_Init(void);

/**
 * Uninitialize the VControl input system
 */
void rust_VControl_Uninit(void);

/* === State Management === */

/**
 * Reset all input states to 0
 * Clears all bound target variables
 */
void rust_VControl_ResetInput(void);

/**
 * Begin a new input frame
 * Clears start bits (VCONTROL_STARTBIT) for all bindings
 */
void rust_VControl_BeginFrame(void);

/* === Keyboard Bindings === */

/**
 * Add a keyboard binding
 * @param symbol SDL keycode
 * @param target Pointer to i32 variable to update
 * @returns 0 on success, -1 on failure
 */
int rust_VControl_AddKeyBinding(int symbol, int *target);

/**
 * Remove a keyboard binding
 * @param symbol SDL keycode
 * @param target Pointer to bound variable
 * @returns 0 on success, -1 on failure
 */
int rust_VControl_RemoveKeyBinding(int symbol, int *target);

/**
 * Clear all keyboard bindings
 */
void rust_VControl_ClearKeyBindings(void);

/* === Keyboard Event Handling === */

/**
 * Handle key down event
 * @param symbol SDL keycode
 */
void rust_VControl_ProcessKeyDown(int symbol);

/**
 * Handle key up event
 * @param symbol SDL keycode
 */
void rust_VControl_ProcessKeyUp(int symbol);

/* === Joystick Management === */

/**
 * Initialize a joystick
 * @param index Joystick index (0-7)
 * @param name Joystick name string
 * @param num_axes Number of axes
 * @param num_buttons Number of buttons
 * @param num_hats Number of hats
 * @returns 0 on success, -1 on failure
 */
int rust_VControl_InitJoystick(int index, const char *name,
	int num_axes, int num_buttons, int num_hats);

/**
 * Uninitialize a joystick
 * @param index Joystick index
 * @returns 0 on success, -1 on failure
 */
int rust_VControl_UninitJoystick(int index);

/**
 * Get number of initialized joysticks
 * @returns Number of joysticks
 */
int rust_VControl_GetNumJoysticks(void);

/* === Joystick Bindings === */

/**
 * Add joystick axis binding
 * @param port Joystick port number
 * @param axis Axis index
 * @param polarity -1 for negative, 1 for positive
 * @param target Pointer to i32 variable to update
 * @returns 0 on success, 1 if already exists, -1 on error
 */
int rust_VControl_AddJoyAxisBinding(int port, int axis, int polarity,
	int *target);

/**
 * Remove joystick axis binding
 * @param port Joystick port number
 * @param axis Axis index
 * @param polarity -1 for negative, 1 for positive
 * @param target Pointer to bound variable
 * @returns 0 on success, 1 if not found, -1 on error
 */
int rust_VControl_RemoveJoyAxisBinding(int port, int axis, int polarity,
	int *target);

/**
 * Add joystick button binding
 * @param port Joystick port number
 * @param button Button index
 * @param target Pointer to i32 variable to update
 * @returns 0 on success, 1 if already exists, -1 on error
 */
int rust_VControl_AddJoyButtonBinding(int port, int button, int *target);

/**
 * Remove joystick button binding
 * @param port Joystick port number
 * @param button Button index
 * @param target Pointer to bound variable
 * @returns 0 on success, 1 if not found, -1 on error
 */
int rust_VControl_RemoveJoyButtonBinding(int port, int button, int *target);

/**
 * Add joystick hat binding
 * @param port Joystick port number
 * @param which Hat index
 * @param dir Hat direction (SDL_HAT_UP/DOWN/LEFT/RIGHT)
 * @param target Pointer to i32 variable to update
 * @returns 0 on success, 1 if already exists, -1 on error
 */
int rust_VControl_AddJoyHatBinding(int port, int which, unsigned char dir,
	int *target);

/**
 * Remove joystick hat binding
 * @param port Joystick port number
 * @param which Hat index
 * @param dir Hat direction
 * @param target Pointer to bound variable
 * @returns 0 on success, 1 if not found, -1 on error
 */
int rust_VControl_RemoveJoyHatBinding(int port, int which, unsigned char dir,
	int *target);

/**
 * Set joystick axis threshold (dead zone)
 * @param port Joystick port number
 * @param threshold Threshold value (0-32767)
 * @returns 0 on success, -1 on error
 */
int rust_VControl_SetJoyThreshold(int port, int threshold);

/**
 * Clear all bindings for a joystick
 * @param joy Joystick port number
 * @returns 0 on success, -1 on error
 */
int rust_VControl_ClearJoyBindings(int joy);

/* === Joystick Event Handling === */

/**
 * Handle joystick button down event
 * @param port Joystick port number
 * @param button Button index
 */
void rust_VControl_ProcessJoyButtonDown(int port, int button);

/**
 * Handle joystick button up event
 * @param port Joystick port number
 * @param button Button index
 */
void rust_VControl_ProcessJoyButtonUp(int port, int button);

/**
 * Handle joystick axis event
 * @param port Joystick port number
 * @param axis Axis index
 * @param value Axis value (-32768 to 32767)
 */
void rust_VControl_ProcessJoyAxis(int port, int axis, int value);

/**
 * Handle joystick hat event
 * @param port Joystick port number
 * @param which Hat index
 * @param value Hat position (SDL_HAT_*)
 */
void rust_VControl_ProcessJoyHat(int port, int which, unsigned char value);

/* === Gesture Tracking === */

/**
 * Clear the last gesture
 */
void rust_VControl_ClearGesture(void);

/**
 * Get the type of the last gesture
 * @returns 0=NONE, 1=KEY, 2=JOYAXIS, 3=JOYBUTTON, 4=JOYHAT
 */
int rust_VControl_GetLastGestureType(void);

/* === General === */

/**
 * Remove all bindings (keyboard and joystick)
 */
void rust_VControl_RemoveAllBindings(void);

/* === SDL Event Handling === */

/**
 * Handle an SDL event
 * @param e SDL_Event pointer
 */
void rust_VControl_HandleEvent(const void *e);

/* === Gesture Struct Functions === */

/* Forward declaration - VCONTROL_GESTURE struct is defined in rust_vcontrol.h */
struct VCONTROL_GESTURE_s;
typedef struct VCONTROL_GESTURE_s VCONTROL_GESTURE_FFI;

/**
 * Get the last gesture
 * @param g Pointer to gesture struct to fill
 * @returns 1 if gesture available, 0 otherwise
 */
int rust_VControl_GetLastGesture(void *g);

/**
 * Add a gesture binding
 * @param g Pointer to gesture struct
 * @param target Pointer to i32 variable to update
 * @returns 0 on success, -1 on failure
 */
int rust_VControl_AddGestureBinding(void *g, int *target);

/**
 * Remove a gesture binding
 * @param g Pointer to gesture struct
 * @param target Pointer to bound variable
 */
void rust_VControl_RemoveGestureBinding(void *g, int *target);

/**
 * Parse a gesture from string specification
 * @param g Pointer to gesture struct to fill
 * @param spec Gesture string specification
 */
void rust_VControl_ParseGesture(void *g, const char *spec);

/**
 * Dump gesture to string buffer
 * @param buf Buffer to write to
 * @param n Buffer size
 * @param g Pointer to gesture struct
 * @returns Number of characters written
 */
int rust_VControl_DumpGesture(char *buf, int n, void *g);

/* === Key Name Functions === */

/**
 * Convert key name to SDL keycode
 * @param name Key name string
 * @returns SDL keycode, or 0 if not found
 */
int rust_VControl_name2code(const char *name);

/**
 * Convert SDL keycode to key name
 * @param code SDL keycode
 * @returns Key name string (static)
 */
const char *rust_VControl_code2name(int code);

#ifdef __cplusplus
}
#endif

#endif /* LIBS_INPUT_SDL_RUST_INPUT_H_ */
