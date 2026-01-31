//! C FFI bindings for VControl
//!
//! Exports C-compatible functions for the input system.
//! All exported functions use the `rust_` prefix to avoid symbol conflicts
//! with the C implementation. The C code uses #ifdef USE_RUST_INPUT to
//! choose between C and Rust implementations.

use std::ffi::{c_char, c_int, c_uchar, c_void, CStr};
use std::ptr;

use super::vcontrol::{Gesture, VCONTROL};

/// SDL HAT direction constants (matching SDL)
const SDL_HAT_UP: u8 = 0x01;
const SDL_HAT_RIGHT: u8 = 0x02;
const SDL_HAT_DOWN: u8 = 0x04;
const SDL_HAT_LEFT: u8 = 0x08;

/// VCONTROL_GESTURE_TYPE enum values (must match C)
const VCONTROL_NONE: c_int = 0;
const VCONTROL_KEY: c_int = 1;
const VCONTROL_JOYAXIS: c_int = 2;
const VCONTROL_JOYBUTTON: c_int = 3;
const VCONTROL_JOYHAT: c_int = 4;

/// C-compatible gesture structure
/// Must match the VCONTROL_GESTURE struct in rust_vcontrol.h
#[repr(C)]
pub struct VCONTROL_GESTURE {
    pub gesture_type: c_int,
    /// Union data - we use the largest variant size (3 ints for axis)
    pub data: [c_int; 3],
}

impl VCONTROL_GESTURE {
    /// Convert from Rust Gesture to C VCONTROL_GESTURE
    pub fn from_gesture(g: &Gesture) -> Self {
        match g {
            Gesture::Key(key) => VCONTROL_GESTURE {
                gesture_type: VCONTROL_KEY,
                data: [*key, 0, 0],
            },
            Gesture::JoyAxis { port, axis, polarity } => VCONTROL_GESTURE {
                gesture_type: VCONTROL_JOYAXIS,
                data: [*port as c_int, *axis, *polarity],
            },
            Gesture::JoyButton { port, button } => VCONTROL_GESTURE {
                gesture_type: VCONTROL_JOYBUTTON,
                data: [*port as c_int, *button, 0],
            },
            Gesture::JoyHat { port, hat, dir } => VCONTROL_GESTURE {
                gesture_type: VCONTROL_JOYHAT,
                data: [*port as c_int, *hat, *dir as c_int],
            },
        }
    }
    
    /// Convert from C VCONTROL_GESTURE to Rust Gesture
    pub fn to_gesture(&self) -> Option<Gesture> {
        match self.gesture_type {
            VCONTROL_KEY => Some(Gesture::Key(self.data[0])),
            VCONTROL_JOYAXIS => Some(Gesture::JoyAxis {
                port: self.data[0] as u32,
                axis: self.data[1],
                polarity: self.data[2],
            }),
            VCONTROL_JOYBUTTON => Some(Gesture::JoyButton {
                port: self.data[0] as u32,
                button: self.data[1],
            }),
            VCONTROL_JOYHAT => Some(Gesture::JoyHat {
                port: self.data[0] as u32,
                hat: self.data[1],
                dir: self.data[2] as u8,
            }),
            _ => None,
        }
    }
}

/// SDL_Event type constants (SDL2)
const SDL_KEYDOWN: u32 = 0x300;
const SDL_KEYUP: u32 = 0x301;
const SDL_JOYAXISMOTION: u32 = 0x600;
const SDL_JOYBALLMOTION: u32 = 0x601;
const SDL_JOYHATMOTION: u32 = 0x602;
const SDL_JOYBUTTONDOWN: u32 = 0x603;
const SDL_JOYBUTTONUP: u32 = 0x604;

/// Initialize the VControl system
#[no_mangle]
pub extern "C" fn rust_VControl_Init() -> c_int {
    let mut vc = VCONTROL.write();
    match vc.init() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Uninitialize the VControl system
#[no_mangle]
pub extern "C" fn rust_VControl_Uninit() {
    let mut vc = VCONTROL.write();
    vc.uninit();
}

/// Reset all control states to 0
#[no_mangle]
pub extern "C" fn rust_VControl_ResetInput() {
    let mut vc = VCONTROL.write();
    unsafe {
        vc.reset_states();
    }
}

/// Add a keyboard binding
///
/// # Safety
/// `target` must be a valid pointer to an i32 that lives as long as the binding
#[no_mangle]
pub extern "C" fn rust_VControl_AddKeyBinding(symbol: c_int, target: *mut c_int) -> c_int {
    let mut vc = VCONTROL.write();
    if vc.add_key_binding(symbol, target as usize) {
        0
    } else {
        -1
    }
}

/// Remove a keyboard binding
#[no_mangle]
pub extern "C" fn rust_VControl_RemoveKeyBinding(symbol: c_int, target: *mut c_int) -> c_int {
    let mut vc = VCONTROL.write();
    if vc.remove_key_binding(symbol, target as usize) {
        0
    } else {
        -1
    }
}

/// Clear all keyboard bindings
#[no_mangle]
pub extern "C" fn rust_VControl_ClearKeyBindings() {
    let mut vc = VCONTROL.write();
    vc.clear_key_bindings();
}

/// Handle key down event
#[no_mangle]
pub extern "C" fn rust_VControl_ProcessKeyDown(symbol: c_int) {
    let vc = VCONTROL.read();
    unsafe {
        vc.handle_key_down(symbol);
    }
}

/// Handle key up event
#[no_mangle]
pub extern "C" fn rust_VControl_ProcessKeyUp(symbol: c_int) {
    let vc = VCONTROL.read();
    unsafe {
        vc.handle_key_up(symbol);
    }
}

/// Initialize a joystick
///
/// # Safety
/// `name` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn rust_VControl_InitJoystick(
    index: c_int,
    name: *const c_char,
    num_axes: c_int,
    num_buttons: c_int,
    num_hats: c_int,
) -> c_int {
    let name_str = if name.is_null() {
        "Unknown".to_string()
    } else {
        CStr::from_ptr(name)
            .to_str()
            .unwrap_or("Unknown")
            .to_string()
    };

    let mut vc = VCONTROL.write();
    match vc.init_joystick(index as u32, name_str, num_axes, num_buttons, num_hats) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Uninitialize a joystick
#[no_mangle]
pub extern "C" fn rust_VControl_UninitJoystick(index: c_int) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.uninit_joystick(index as u32) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Get number of initialized joysticks
#[no_mangle]
pub extern "C" fn rust_VControl_GetNumJoysticks() -> c_int {
    let vc = VCONTROL.read();
    vc.num_joysticks() as c_int
}

/// Add a joystick axis binding
#[no_mangle]
pub extern "C" fn rust_VControl_AddJoyAxisBinding(
    port: c_int,
    axis: c_int,
    polarity: c_int,
    target: *mut c_int,
) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.add_joy_axis_binding(port as u32, axis, polarity, target as usize) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(_) => -1,
    }
}

/// Remove a joystick axis binding
#[no_mangle]
pub extern "C" fn rust_VControl_RemoveJoyAxisBinding(
    port: c_int,
    axis: c_int,
    polarity: c_int,
    target: *mut c_int,
) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.remove_joy_axis_binding(port as u32, axis, polarity, target as usize) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(_) => -1,
    }
}

/// Add a joystick button binding
#[no_mangle]
pub extern "C" fn rust_VControl_AddJoyButtonBinding(
    port: c_int,
    button: c_int,
    target: *mut c_int,
) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.add_joy_button_binding(port as u32, button, target as usize) {
        Ok(true) => 0,
        Ok(false) => 1, // Already exists
        Err(_) => -1,
    }
}

/// Remove a joystick button binding
#[no_mangle]
pub extern "C" fn rust_VControl_RemoveJoyButtonBinding(
    port: c_int,
    button: c_int,
    target: *mut c_int,
) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.remove_joy_button_binding(port as u32, button, target as usize) {
        Ok(true) => 0,
        Ok(false) => 1, // Not found
        Err(_) => -1,
    }
}

/// Add a joystick hat binding
#[no_mangle]
pub extern "C" fn rust_VControl_AddJoyHatBinding(
    port: c_int,
    which: c_int,
    dir: c_uchar,
    target: *mut c_int,
) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.add_joy_hat_binding(port as u32, which, dir, target as usize) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(_) => -1,
    }
}

/// Remove a joystick hat binding
#[no_mangle]
pub extern "C" fn rust_VControl_RemoveJoyHatBinding(
    port: c_int,
    which: c_int,
    dir: c_uchar,
    target: *mut c_int,
) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.remove_joy_hat_binding(port as u32, which, dir, target as usize) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(_) => -1,
    }
}

/// Set joystick axis threshold (dead zone)
#[no_mangle]
pub extern "C" fn rust_VControl_SetJoyThreshold(port: c_int, threshold: c_int) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.set_joy_threshold(port as u32, threshold) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Handle joystick button down event
#[no_mangle]
pub extern "C" fn rust_VControl_ProcessJoyButtonDown(port: c_int, button: c_int) {
    let vc = VCONTROL.read();
    unsafe {
        vc.handle_joy_button(port as u32, button, true);
    }
}

/// Handle joystick button up event
#[no_mangle]
pub extern "C" fn rust_VControl_ProcessJoyButtonUp(port: c_int, button: c_int) {
    let vc = VCONTROL.read();
    unsafe {
        vc.handle_joy_button(port as u32, button, false);
    }
}

/// Handle joystick axis event
#[no_mangle]
pub extern "C" fn rust_VControl_ProcessJoyAxis(port: c_int, axis: c_int, value: c_int) {
    let mut vc = VCONTROL.write();
    unsafe {
        vc.handle_joy_axis(port as u32, axis, value as i16);
    }
}

/// Handle joystick hat event
#[no_mangle]
pub extern "C" fn rust_VControl_ProcessJoyHat(port: c_int, which: c_int, value: c_uchar) {
    let mut vc = VCONTROL.write();
    unsafe {
        vc.handle_joy_hat(port as u32, which, value);
    }
}

/// Clear all bindings for a joystick
#[no_mangle]
pub extern "C" fn rust_VControl_ClearJoyBindings(joy: c_int) -> c_int {
    let mut vc = VCONTROL.write();
    match vc.clear_joy_bindings(joy as u32) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Remove all bindings
#[no_mangle]
pub extern "C" fn rust_VControl_RemoveAllBindings() {
    let mut vc = VCONTROL.write();
    vc.clear_key_bindings();
    // Clear joystick bindings for all ports
    for port in 0..16 {
        let _ = vc.clear_joy_bindings(port);
    }
}

/// Begin a new input frame (clear start bits)
#[no_mangle]
pub extern "C" fn rust_VControl_BeginFrame() {
    let mut vc = VCONTROL.write();
    unsafe {
        vc.begin_frame();
    }
}

/// Clear the last gesture
#[no_mangle]
pub extern "C" fn rust_VControl_ClearGesture() {
    let mut vc = VCONTROL.write();
    vc.clear_gesture();
}

/// Get the type of the last gesture
/// Returns: 0=NONE, 1=KEY, 2=JOYAXIS, 3=JOYBUTTON, 4=JOYHAT
#[no_mangle]
pub extern "C" fn rust_VControl_GetLastGestureType() -> c_int {
    let vc = VCONTROL.read();
    match vc.get_last_gesture() {
        Some(g) => match g {
            Gesture::Key(_) => 1,
            Gesture::JoyAxis { .. } => 2,
            Gesture::JoyButton { .. } => 3,
            Gesture::JoyHat { .. } => 4,
        },
        None => 0,
    }
}

/// Get the last gesture (fills in the provided VCONTROL_GESTURE struct)
/// Returns 1 if a gesture was available, 0 otherwise
#[no_mangle]
pub extern "C" fn rust_VControl_GetLastGesture(g: *mut VCONTROL_GESTURE) -> c_int {
    if g.is_null() {
        return 0;
    }
    
    let vc = VCONTROL.read();
    match vc.get_last_gesture() {
        Some(gesture) => {
            let c_gesture = VCONTROL_GESTURE::from_gesture(gesture);
            unsafe {
                *g = c_gesture;
            }
            1
        }
        None => 0,
    }
}

/// Handle an SDL event
/// This processes the event and updates bound targets accordingly
#[no_mangle]
pub unsafe extern "C" fn rust_VControl_HandleEvent(e: *const c_void) {
    if e.is_null() {
        return;
    }
    
    // SDL_Event is a union, but we only care about the type field (first 4 bytes)
    // and then the specific event data based on type
    let event_type = *(e as *const u32);
    
    match event_type {
        SDL_KEYDOWN => {
            // SDL_KeyboardEvent: type(4) + timestamp(4) + windowID(4) + state(1) + repeat(1) + padding(2) + keysym
            // keysym: scancode(4) + sym(4) + mod(2) + unused(4)
            // We want keysym.sym which is at offset 20
            let sym = *((e as *const u8).add(20) as *const i32);
            let repeat = *((e as *const u8).add(13));
            
            if repeat == 0 {
                let vc = VCONTROL.read();
                vc.handle_key_down(sym);
                drop(vc);
                
                // Track gesture
                let mut vc = VCONTROL.write();
                vc.set_last_gesture(Gesture::Key(sym));
            }
        }
        SDL_KEYUP => {
            let sym = *((e as *const u8).add(20) as *const i32);
            let vc = VCONTROL.read();
            vc.handle_key_up(sym);
        }
        SDL_JOYAXISMOTION => {
            // SDL_JoyAxisEvent: type(4) + timestamp(4) + which(4) + axis(1) + padding(3) + value(2)
            let which = *((e as *const u8).add(8) as *const i32);
            let axis = *((e as *const u8).add(12)) as i32;
            let value = *((e as *const u8).add(16) as *const i16);
            
            let mut vc = VCONTROL.write();
            vc.handle_joy_axis(which as u32, axis, value);
            
            // Track gesture for significant axis movements
            if value > 15000 || value < -15000 {
                let polarity = if value < 0 { -1 } else { 1 };
                vc.set_last_gesture(Gesture::JoyAxis {
                    port: which as u32,
                    axis,
                    polarity,
                });
            }
        }
        SDL_JOYHATMOTION => {
            // SDL_JoyHatEvent: type(4) + timestamp(4) + which(4) + hat(1) + value(1)
            let which = *((e as *const u8).add(8) as *const i32);
            let hat = *((e as *const u8).add(12)) as i32;
            let value = *((e as *const u8).add(13));
            
            let mut vc = VCONTROL.write();
            vc.handle_joy_hat(which as u32, hat, value);
            vc.set_last_gesture(Gesture::JoyHat {
                port: which as u32,
                hat,
                dir: value,
            });
        }
        SDL_JOYBUTTONDOWN => {
            // SDL_JoyButtonEvent: type(4) + timestamp(4) + which(4) + button(1) + state(1)
            let which = *((e as *const u8).add(8) as *const i32);
            let button = *((e as *const u8).add(12)) as i32;
            
            let vc = VCONTROL.read();
            vc.handle_joy_button(which as u32, button, true);
            drop(vc);
            
            let mut vc = VCONTROL.write();
            vc.set_last_gesture(Gesture::JoyButton {
                port: which as u32,
                button,
            });
        }
        SDL_JOYBUTTONUP => {
            let which = *((e as *const u8).add(8) as *const i32);
            let button = *((e as *const u8).add(12)) as i32;
            
            let vc = VCONTROL.read();
            vc.handle_joy_button(which as u32, button, false);
        }
        _ => {}
    }
}

/// Add a gesture binding
/// Maps a gesture to a target address
#[no_mangle]
pub extern "C" fn rust_VControl_AddGestureBinding(g: *mut VCONTROL_GESTURE, target: *mut c_int) -> c_int {
    if g.is_null() || target.is_null() {
        return -1;
    }
    
    let gesture = unsafe { &*g };
    let mut vc = VCONTROL.write();
    
    match gesture.gesture_type {
        VCONTROL_KEY => {
            if vc.add_key_binding(gesture.data[0], target as usize) {
                0
            } else {
                -1
            }
        }
        VCONTROL_JOYAXIS => {
            match vc.add_joy_axis_binding(
                gesture.data[0] as u32,
                gesture.data[1],
                gesture.data[2],
                target as usize,
            ) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
        VCONTROL_JOYBUTTON => {
            match vc.add_joy_button_binding(
                gesture.data[0] as u32,
                gesture.data[1],
                target as usize,
            ) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
        VCONTROL_JOYHAT => {
            match vc.add_joy_hat_binding(
                gesture.data[0] as u32,
                gesture.data[1],
                gesture.data[2] as u8,
                target as usize,
            ) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
        _ => -1,
    }
}

/// Remove a gesture binding
#[no_mangle]
pub extern "C" fn rust_VControl_RemoveGestureBinding(g: *mut VCONTROL_GESTURE, target: *mut c_int) {
    if g.is_null() || target.is_null() {
        return;
    }
    
    let gesture = unsafe { &*g };
    let mut vc = VCONTROL.write();
    
    match gesture.gesture_type {
        VCONTROL_KEY => {
            vc.remove_key_binding(gesture.data[0], target as usize);
        }
        VCONTROL_JOYAXIS => {
            let _ = vc.remove_joy_axis_binding(
                gesture.data[0] as u32,
                gesture.data[1],
                gesture.data[2],
                target as usize,
            );
        }
        VCONTROL_JOYBUTTON => {
            let _ = vc.remove_joy_button_binding(
                gesture.data[0] as u32,
                gesture.data[1],
                target as usize,
            );
        }
        VCONTROL_JOYHAT => {
            let _ = vc.remove_joy_hat_binding(
                gesture.data[0] as u32,
                gesture.data[1],
                gesture.data[2] as u8,
                target as usize,
            );
        }
        _ => {}
    }
}

/// Parse a gesture from a string specification
/// Format: "key KEYNAME" or "joystick N axis M positive/negative" or 
///         "joystick N button M" or "joystick N hat M up/down/left/right"
#[no_mangle]
pub unsafe extern "C" fn rust_VControl_ParseGesture(g: *mut VCONTROL_GESTURE, spec: *const c_char) {
    if g.is_null() || spec.is_null() {
        return;
    }
    
    let spec_str = match CStr::from_ptr(spec).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    
    // Default to NONE
    (*g).gesture_type = VCONTROL_NONE;
    (*g).data = [0, 0, 0];
    
    let tokens: Vec<&str> = spec_str.split_whitespace().collect();
    if tokens.is_empty() {
        return;
    }
    
    if tokens[0].eq_ignore_ascii_case("key") && tokens.len() >= 2 {
        // Parse key binding: "key KEYNAME"
        // We need to call the C function VControl_name2code
        extern "C" {
            fn VControl_name2code(name: *const c_char) -> c_int;
        }
        
        let key_name = std::ffi::CString::new(tokens[1]).unwrap();
        let keycode = VControl_name2code(key_name.as_ptr());
        
        if keycode != 0 {
            (*g).gesture_type = VCONTROL_KEY;
            (*g).data[0] = keycode;
        }
    } else if tokens[0].eq_ignore_ascii_case("joystick") && tokens.len() >= 4 {
        // Parse joystick binding
        let joy_num: c_int = match tokens[1].parse() {
            Ok(n) => n,
            Err(_) => return,
        };
        
        if tokens[2].eq_ignore_ascii_case("axis") && tokens.len() >= 5 {
            // "joystick N axis M positive/negative"
            let axis_num: c_int = match tokens[3].parse() {
                Ok(n) => n,
                Err(_) => return,
            };
            
            let polarity = if tokens[4].eq_ignore_ascii_case("positive") {
                1
            } else if tokens[4].eq_ignore_ascii_case("negative") {
                -1
            } else {
                return;
            };
            
            (*g).gesture_type = VCONTROL_JOYAXIS;
            (*g).data[0] = joy_num;
            (*g).data[1] = axis_num;
            (*g).data[2] = polarity;
        } else if tokens[2].eq_ignore_ascii_case("button") {
            // "joystick N button M"
            let button_num: c_int = match tokens[3].parse() {
                Ok(n) => n,
                Err(_) => return,
            };
            
            (*g).gesture_type = VCONTROL_JOYBUTTON;
            (*g).data[0] = joy_num;
            (*g).data[1] = button_num;
        } else if tokens[2].eq_ignore_ascii_case("hat") && tokens.len() >= 5 {
            // "joystick N hat M up/down/left/right"
            let hat_num: c_int = match tokens[3].parse() {
                Ok(n) => n,
                Err(_) => return,
            };
            
            let dir = if tokens[4].eq_ignore_ascii_case("up") {
                SDL_HAT_UP
            } else if tokens[4].eq_ignore_ascii_case("down") {
                SDL_HAT_DOWN
            } else if tokens[4].eq_ignore_ascii_case("left") {
                SDL_HAT_LEFT
            } else if tokens[4].eq_ignore_ascii_case("right") {
                SDL_HAT_RIGHT
            } else {
                return;
            };
            
            (*g).gesture_type = VCONTROL_JOYHAT;
            (*g).data[0] = joy_num;
            (*g).data[1] = hat_num;
            (*g).data[2] = dir as c_int;
        }
    }
}

/// Dump a gesture to a string buffer
/// Returns the number of characters written (excluding null terminator)
#[no_mangle]
pub unsafe extern "C" fn rust_VControl_DumpGesture(buf: *mut c_char, n: c_int, g: *mut VCONTROL_GESTURE) -> c_int {
    if buf.is_null() || g.is_null() || n <= 0 {
        return 0;
    }
    
    let gesture = &*g;
    let result = match gesture.gesture_type {
        VCONTROL_KEY => {
            // Get key name from C function
            extern "C" {
                fn VControl_code2name(code: c_int) -> *const c_char;
            }
            
            let name_ptr = VControl_code2name(gesture.data[0]);
            if name_ptr.is_null() {
                String::new()
            } else {
                let name = CStr::from_ptr(name_ptr).to_string_lossy();
                format!("key {}", name)
            }
        }
        VCONTROL_JOYAXIS => {
            let polarity = if gesture.data[2] > 0 { "positive" } else { "negative" };
            format!("joystick {} axis {} {}", gesture.data[0], gesture.data[1], polarity)
        }
        VCONTROL_JOYBUTTON => {
            format!("joystick {} button {}", gesture.data[0], gesture.data[1])
        }
        VCONTROL_JOYHAT => {
            let dir = match gesture.data[2] as u8 {
                SDL_HAT_UP => "up",
                SDL_HAT_DOWN => "down",
                SDL_HAT_LEFT => "left",
                SDL_HAT_RIGHT => "right",
                _ => "unknown",
            };
            format!("joystick {} hat {} {}", gesture.data[0], gesture.data[1], dir)
        }
        _ => {
            *buf = 0;
            return 0;
        }
    };
    
    let bytes = result.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), (n - 1) as usize);
    
    ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
    *buf.add(copy_len) = 0;
    
    copy_len as c_int
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn cleanup() {
        rust_VControl_Uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_init_uninit() {
        cleanup();
        assert_eq!(rust_VControl_Init(), 0);

        let vc = VCONTROL.read();
        assert!(vc.is_initialized());
        drop(vc);

        rust_VControl_Uninit();

        let vc = VCONTROL.read();
        assert!(!vc.is_initialized());
    }

    #[test]
    #[serial]
    fn test_ffi_key_binding() {
        cleanup();
        rust_VControl_Init();

        let mut target: i32 = 0;
        assert_eq!(rust_VControl_AddKeyBinding(32, &mut target), 0);

        rust_VControl_ProcessKeyDown(32);
        assert_eq!(target, 1);

        rust_VControl_ProcessKeyUp(32);
        assert_eq!(target, 0);

        assert_eq!(rust_VControl_RemoveKeyBinding(32, &mut target), 0);
        cleanup();
    }

    #[test]
    #[serial]
    fn test_ffi_clear_key_bindings() {
        cleanup();
        rust_VControl_Init();

        let mut target: i32 = 0;
        rust_VControl_AddKeyBinding(32, &mut target);
        rust_VControl_ClearKeyBindings();

        rust_VControl_ProcessKeyDown(32);
        assert_eq!(target, 0); // Should not be affected

        cleanup();
    }

    #[test]
    #[serial]
    fn test_ffi_joystick() {
        cleanup();
        rust_VControl_Init();

        let name = c"Test Joystick";
        unsafe {
            assert_eq!(rust_VControl_InitJoystick(0, name.as_ptr(), 2, 10, 1), 0);
        }

        assert_eq!(rust_VControl_GetNumJoysticks(), 1);

        let mut target: i32 = 0;
        assert_eq!(rust_VControl_AddJoyButtonBinding(0, 0, &mut target), 0);

        rust_VControl_ProcessJoyButtonDown(0, 0);
        assert_eq!(target, 1);

        rust_VControl_ProcessJoyButtonUp(0, 0);
        assert_eq!(target, 0);

        assert_eq!(rust_VControl_UninitJoystick(0), 0);
        assert_eq!(rust_VControl_GetNumJoysticks(), 0);

        cleanup();
    }

    #[test]
    #[serial]
    fn test_ffi_joy_axis() {
        cleanup();
        rust_VControl_Init();

        let name = c"Test";
        unsafe {
            rust_VControl_InitJoystick(0, name.as_ptr(), 2, 0, 0);
        }

        let mut neg: i32 = 0;
        let mut pos: i32 = 0;

        rust_VControl_AddJoyAxisBinding(0, 0, -1, &mut neg);
        rust_VControl_AddJoyAxisBinding(0, 0, 1, &mut pos);

        rust_VControl_ProcessJoyAxis(0, 0, -20000);
        assert_eq!(neg, 1);
        assert_eq!(pos, 0);

        rust_VControl_ProcessJoyAxis(0, 0, 0);
        assert_eq!(neg, 0);
        assert_eq!(pos, 0);

        rust_VControl_ProcessJoyAxis(0, 0, 20000);
        assert_eq!(neg, 0);
        assert_eq!(pos, 1);

        cleanup();
    }

    #[test]
    #[serial]
    fn test_ffi_reset_states() {
        cleanup();
        rust_VControl_Init();

        let mut target: i32 = 5;
        rust_VControl_AddKeyBinding(32, &mut target);

        rust_VControl_ResetInput();
        assert_eq!(target, 0);

        cleanup();
    }
}
