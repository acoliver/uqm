# Phase 6: Input/Control System Port to Rust

## Overview
Port `sc2/src/libs/input/sdl/vcontrol.c` (1300 lines) to Rust. The vcontrol system provides a virtual control abstraction layer that maps keyboard keys and joystick inputs to game actions.

## C Source Files
- `sc2/src/libs/input/sdl/vcontrol.c` - Main implementation
- `sc2/src/libs/input/sdl/vcontrol.h` - Public interface
- `sc2/src/libs/input/sdl/keynames.h` - Key name mappings

## Key Data Structures

### KeyBinding
```rust
pub struct KeyBinding {
    pub target: *mut i32,      // Pointer to control state variable
    pub keycode: Keycode,      // SDL keycode
    pub next: Option<Box<KeyBinding>>,  // Next binding in hash bucket
}
```

### JoystickAxis
```rust
pub struct JoystickAxis {
    pub neg: Option<Box<KeyBinding>>,
    pub pos: Option<Box<KeyBinding>>,
    pub polarity: i32,
}
```

### JoystickHat
```rust
pub struct JoystickHat {
    pub left: Option<Box<KeyBinding>>,
    pub right: Option<Box<KeyBinding>>,
    pub up: Option<Box<KeyBinding>>,
    pub down: Option<Box<KeyBinding>>,
    pub last: u8,
}
```

### Joystick
```rust
pub struct Joystick {
    pub stick: sdl2::joystick::Joystick,
    pub num_axes: i32,
    pub num_buttons: i32,
    pub num_hats: i32,
    pub threshold: i32,
    pub axes: Vec<JoystickAxis>,
    pub buttons: Vec<Option<Box<KeyBinding>>>,
    pub hats: Vec<JoystickHat>,
}
```

### VControl (main state)
```rust
pub struct VControl {
    pub joysticks: Vec<Joystick>,
    pub bindings: HashMap<Keycode, Vec<KeyBinding>>,  // Or fixed-size bucket array
    pub joycount: u32,
}
```

## Core Functions to Implement

### Initialization
- `VControl_Init()` - Initialize input system
- `VControl_Uninit()` - Cleanup
- `VControl_ResetStates()` - Reset all control states

### Keyboard Bindings
- `VControl_AddKeyBinding(key, target)` - Bind key to target
- `VControl_RemoveKeyBinding(key, target)` - Remove binding
- `VControl_ClearKeyBindings()` - Remove all keyboard bindings
- `VControl_HandleKeyDown(key)` - Process key press
- `VControl_HandleKeyUp(key)` - Process key release

### Joystick Management
- `VControl_InitJoystick(index) -> bool` - Initialize joystick
- `VControl_UninitJoystick(index)` - Release joystick
- `VControl_GetNumJoysticks() -> u32`

### Joystick Bindings
- `VControl_AddJoyButtonBinding(joy, button, target)`
- `VControl_RemoveJoyButtonBinding(joy, button, target)`
- `VControl_AddJoyAxisBinding(joy, axis, polarity, target)`
- `VControl_RemoveJoyAxisBinding(joy, axis, polarity, target)`
- `VControl_AddJoyHatBinding(joy, hat, direction, target)`
- `VControl_RemoveJoyHatBinding(joy, hat, direction, target)`
- `VControl_SetJoyThreshold(joy, threshold)`
- `VControl_HandleJoyButton(joy, button, pressed)`
- `VControl_HandleJoyAxis(joy, axis, value)`
- `VControl_HandleJoyHat(joy, hat, value)`

### Control Templates
- `VControl_LoadControlTemplate(name) -> Result`
- `VControl_SaveControlTemplate(name) -> Result`
- `VControl_GetControlTemplate(index) -> &Template`

### Event Processing
- `VControl_ProcessEvent(event)` - Process SDL event
- `VControl_ProcessEvents()` - Process all pending events

### Key Names
- `VControl_GetKeyName(key) -> &str`
- `VControl_GetKeyFromName(name) -> Option<Keycode>`
- `VControl_GetJoyButtonName(joy, button) -> String`
- `VControl_GetJoyAxisName(joy, axis) -> String`

## Key Buckets
The C code uses 512 hash buckets for keyboard bindings:
```rust
const KEYBOARD_INPUT_BUCKETS: usize = 512;

fn key_to_bucket(key: Keycode) -> usize {
    (key.into_i32() as usize) % KEYBOARD_INPUT_BUCKETS
}
```

## Joystick Axis Handling
```rust
fn handle_axis(axis: &mut JoystickAxis, value: i16, threshold: i32) {
    let threshold = threshold as i16;
    
    if value < -threshold {
        // Negative direction
        if axis.polarity >= 0 {
            if let Some(ref neg) = axis.neg {
                *neg.target = 1;
            }
        }
        axis.polarity = -1;
    } else if value > threshold {
        // Positive direction
        if axis.polarity <= 0 {
            if let Some(ref pos) = axis.pos {
                *pos.target = 1;
            }
        }
        axis.polarity = 1;
    } else {
        // Dead zone - release both
        if axis.polarity != 0 {
            if let Some(ref neg) = axis.neg {
                *neg.target = 0;
            }
            if let Some(ref pos) = axis.pos {
                *pos.target = 0;
            }
        }
        axis.polarity = 0;
    }
}
```

## Hat Direction Constants
```rust
pub mod HatDirection {
    pub const CENTERED: u8 = 0;
    pub const UP: u8 = 1;
    pub const RIGHT: u8 = 2;
    pub const DOWN: u8 = 4;
    pub const LEFT: u8 = 8;
}
```

## Thread Safety
Input handling must be thread-safe since events can come from multiple sources. Use `parking_lot::RwLock` for the main VControl state.

## Test Plan (TDD)

### Unit Tests
1. `test_vcontrol_init_uninit` - Initialize and cleanup
2. `test_add_key_binding` - Add keyboard binding
3. `test_remove_key_binding` - Remove keyboard binding
4. `test_key_down_up` - Key press and release
5. `test_multiple_bindings_same_key` - Multiple targets per key
6. `test_clear_bindings` - Clear all bindings
7. `test_joystick_init` - Initialize joystick
8. `test_joy_button_binding` - Joystick button binding
9. `test_joy_axis_binding` - Joystick axis binding
10. `test_joy_axis_threshold` - Dead zone handling
11. `test_joy_hat_binding` - Hat switch binding
12. `test_joy_hat_diagonal` - Diagonal hat positions
13. `test_key_name_lookup` - Get key name
14. `test_key_from_name` - Parse key name
15. `test_reset_states` - Reset all control states
16. `test_template_save_load` - Save and load templates

### Integration Tests
1. `test_process_sdl_keyboard_event` - Handle SDL keyboard event
2. `test_process_sdl_joystick_event` - Handle SDL joystick event
3. `test_full_event_loop` - Complete event processing

## File Structure
```
rust/src/input/
├── mod.rs              (public exports)
├── vcontrol.rs         (main VControl struct)
├── keyboard.rs         (keyboard bindings)
├── joystick.rs         (joystick handling)
├── keynames.rs         (key name mappings)
├── templates.rs        (control templates)
└── ffi.rs              (C FFI bindings)
```

## FFI Functions to Export
```rust
#[no_mangle]
pub extern "C" fn VControl_Init() -> c_int;

#[no_mangle]
pub extern "C" fn VControl_Uninit();

#[no_mangle]
pub extern "C" fn VControl_AddKeyBinding(
    key: c_int,
    target: *mut c_int
) -> c_int;

#[no_mangle]
pub extern "C" fn VControl_HandleKeyDown(key: c_int);

#[no_mangle]
pub extern "C" fn VControl_HandleKeyUp(key: c_int);
// ... etc
```

## Dependencies
- `sdl2` crate for joystick and keyboard handling
- `parking_lot` for thread-safe access

## Acceptance Criteria
1. All unit tests pass
2. Keyboard input works correctly
3. Joystick input works (if available)
4. Dead zone handling is correct
5. Multiple bindings per key work
6. FFI bindings work with C code
7. No input lag or missed events
