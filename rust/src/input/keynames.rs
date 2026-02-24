//! Key name mappings
//!
//! Maps SDL keycodes to human-readable names and vice versa.
//! Based on the C keynames.c implementation for compatibility.

use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr};
use std::sync::LazyLock;

/// SDL2 keycode constants (for mapping SDL_Keycode values)
/// These match the SDL2 keysym definitions
pub mod sdl2_keys {
    pub const SDLK_BACKSPACE: i32 = 8;
    pub const SDLK_TAB: i32 = 9;
    pub const SDLK_CLEAR: i32 = 0x4000009C;
    pub const SDLK_RETURN: i32 = 13;
    pub const SDLK_PAUSE: i32 = 0x40000048;
    pub const SDLK_ESCAPE: i32 = 27;
    pub const SDLK_SPACE: i32 = 32;
    pub const SDLK_EXCLAIM: i32 = 33;
    pub const SDLK_QUOTEDBL: i32 = 34;
    pub const SDLK_HASH: i32 = 35;
    pub const SDLK_DOLLAR: i32 = 36;
    pub const SDLK_AMPERSAND: i32 = 38;
    pub const SDLK_QUOTE: i32 = 39;
    pub const SDLK_LEFTPAREN: i32 = 40;
    pub const SDLK_RIGHTPAREN: i32 = 41;
    pub const SDLK_ASTERISK: i32 = 42;
    pub const SDLK_PLUS: i32 = 43;
    pub const SDLK_COMMA: i32 = 44;
    pub const SDLK_MINUS: i32 = 45;
    pub const SDLK_PERIOD: i32 = 46;
    pub const SDLK_SLASH: i32 = 47;
    pub const SDLK_0: i32 = 48;
    pub const SDLK_1: i32 = 49;
    pub const SDLK_2: i32 = 50;
    pub const SDLK_3: i32 = 51;
    pub const SDLK_4: i32 = 52;
    pub const SDLK_5: i32 = 53;
    pub const SDLK_6: i32 = 54;
    pub const SDLK_7: i32 = 55;
    pub const SDLK_8: i32 = 56;
    pub const SDLK_9: i32 = 57;
    pub const SDLK_COLON: i32 = 58;
    pub const SDLK_SEMICOLON: i32 = 59;
    pub const SDLK_LESS: i32 = 60;
    pub const SDLK_EQUALS: i32 = 61;
    pub const SDLK_GREATER: i32 = 62;
    pub const SDLK_QUESTION: i32 = 63;
    pub const SDLK_AT: i32 = 64;
    pub const SDLK_LEFTBRACKET: i32 = 91;
    pub const SDLK_BACKSLASH: i32 = 92;
    pub const SDLK_RIGHTBRACKET: i32 = 93;
    pub const SDLK_CARET: i32 = 94;
    pub const SDLK_UNDERSCORE: i32 = 95;
    pub const SDLK_BACKQUOTE: i32 = 96;
    // Letters a-z are 97-122
    pub const SDLK_A: i32 = 97;
    pub const SDLK_B: i32 = 98;
    pub const SDLK_C: i32 = 99;
    pub const SDLK_D: i32 = 100;
    pub const SDLK_E: i32 = 101;
    pub const SDLK_F: i32 = 102;
    pub const SDLK_G: i32 = 103;
    pub const SDLK_H: i32 = 104;
    pub const SDLK_I: i32 = 105;
    pub const SDLK_J: i32 = 106;
    pub const SDLK_K: i32 = 107;
    pub const SDLK_L: i32 = 108;
    pub const SDLK_M: i32 = 109;
    pub const SDLK_N: i32 = 110;
    pub const SDLK_O: i32 = 111;
    pub const SDLK_P: i32 = 112;
    pub const SDLK_Q: i32 = 113;
    pub const SDLK_R: i32 = 114;
    pub const SDLK_S: i32 = 115;
    pub const SDLK_T: i32 = 116;
    pub const SDLK_U: i32 = 117;
    pub const SDLK_V: i32 = 118;
    pub const SDLK_W: i32 = 119;
    pub const SDLK_X: i32 = 120;
    pub const SDLK_Y: i32 = 121;
    pub const SDLK_Z: i32 = 122;
    pub const SDLK_DELETE: i32 = 127;
    // SDL2 scancodes | 0x40000000
    pub const SDLK_KP_0: i32 = 0x40000062;
    pub const SDLK_KP_1: i32 = 0x40000059;
    pub const SDLK_KP_2: i32 = 0x4000005A;
    pub const SDLK_KP_3: i32 = 0x4000005B;
    pub const SDLK_KP_4: i32 = 0x4000005C;
    pub const SDLK_KP_5: i32 = 0x4000005D;
    pub const SDLK_KP_6: i32 = 0x4000005E;
    pub const SDLK_KP_7: i32 = 0x4000005F;
    pub const SDLK_KP_8: i32 = 0x40000060;
    pub const SDLK_KP_9: i32 = 0x40000061;
    pub const SDLK_KP_PERIOD: i32 = 0x40000063;
    pub const SDLK_KP_DIVIDE: i32 = 0x40000054;
    pub const SDLK_KP_MULTIPLY: i32 = 0x40000055;
    pub const SDLK_KP_MINUS: i32 = 0x40000056;
    pub const SDLK_KP_PLUS: i32 = 0x40000057;
    pub const SDLK_KP_ENTER: i32 = 0x40000058;
    pub const SDLK_KP_EQUALS: i32 = 0x40000067;
    pub const SDLK_UP: i32 = 0x40000052;
    pub const SDLK_DOWN: i32 = 0x40000051;
    pub const SDLK_RIGHT: i32 = 0x4000004F;
    pub const SDLK_LEFT: i32 = 0x40000050;
    pub const SDLK_INSERT: i32 = 0x40000049;
    pub const SDLK_HOME: i32 = 0x4000004A;
    pub const SDLK_END: i32 = 0x4000004D;
    pub const SDLK_PAGEUP: i32 = 0x4000004B;
    pub const SDLK_PAGEDOWN: i32 = 0x4000004E;
    pub const SDLK_F1: i32 = 0x4000003A;
    pub const SDLK_F2: i32 = 0x4000003B;
    pub const SDLK_F3: i32 = 0x4000003C;
    pub const SDLK_F4: i32 = 0x4000003D;
    pub const SDLK_F5: i32 = 0x4000003E;
    pub const SDLK_F6: i32 = 0x4000003F;
    pub const SDLK_F7: i32 = 0x40000040;
    pub const SDLK_F8: i32 = 0x40000041;
    pub const SDLK_F9: i32 = 0x40000042;
    pub const SDLK_F10: i32 = 0x40000043;
    pub const SDLK_F11: i32 = 0x40000044;
    pub const SDLK_F12: i32 = 0x40000045;
    pub const SDLK_F13: i32 = 0x40000068;
    pub const SDLK_F14: i32 = 0x40000069;
    pub const SDLK_F15: i32 = 0x4000006A;
    pub const SDLK_RSHIFT: i32 = 0x400000E5;
    pub const SDLK_LSHIFT: i32 = 0x400000E1;
    pub const SDLK_RCTRL: i32 = 0x400000E4;
    pub const SDLK_LCTRL: i32 = 0x400000E0;
    pub const SDLK_RALT: i32 = 0x400000E6;
    pub const SDLK_LALT: i32 = 0x400000E2;
}

use sdl2_keys::*;

/// Keyname entry for the lookup table
struct KeyName {
    name: &'static str,
    code: i32,
}

/// Key name table matching the C keynames.c implementation
/// The names are case-insensitive when compared but formatted nicely for output
static KEYNAMES: &[KeyName] = &[
    KeyName {
        name: "Backspace",
        code: SDLK_BACKSPACE,
    },
    KeyName {
        name: "Tab",
        code: SDLK_TAB,
    },
    KeyName {
        name: "Clear",
        code: SDLK_CLEAR,
    },
    KeyName {
        name: "Return",
        code: SDLK_RETURN,
    },
    KeyName {
        name: "Pause",
        code: SDLK_PAUSE,
    },
    KeyName {
        name: "Escape",
        code: SDLK_ESCAPE,
    },
    KeyName {
        name: "Space",
        code: SDLK_SPACE,
    },
    KeyName {
        name: "!",
        code: SDLK_EXCLAIM,
    },
    KeyName {
        name: "\"",
        code: SDLK_QUOTEDBL,
    },
    KeyName {
        name: "Hash",
        code: SDLK_HASH,
    },
    KeyName {
        name: "$",
        code: SDLK_DOLLAR,
    },
    KeyName {
        name: "&",
        code: SDLK_AMPERSAND,
    },
    KeyName {
        name: "'",
        code: SDLK_QUOTE,
    },
    KeyName {
        name: "(",
        code: SDLK_LEFTPAREN,
    },
    KeyName {
        name: ")",
        code: SDLK_RIGHTPAREN,
    },
    KeyName {
        name: "*",
        code: SDLK_ASTERISK,
    },
    KeyName {
        name: "+",
        code: SDLK_PLUS,
    },
    KeyName {
        name: ",",
        code: SDLK_COMMA,
    },
    KeyName {
        name: "-",
        code: SDLK_MINUS,
    },
    KeyName {
        name: ".",
        code: SDLK_PERIOD,
    },
    KeyName {
        name: "/",
        code: SDLK_SLASH,
    },
    KeyName {
        name: "0",
        code: SDLK_0,
    },
    KeyName {
        name: "1",
        code: SDLK_1,
    },
    KeyName {
        name: "2",
        code: SDLK_2,
    },
    KeyName {
        name: "3",
        code: SDLK_3,
    },
    KeyName {
        name: "4",
        code: SDLK_4,
    },
    KeyName {
        name: "5",
        code: SDLK_5,
    },
    KeyName {
        name: "6",
        code: SDLK_6,
    },
    KeyName {
        name: "7",
        code: SDLK_7,
    },
    KeyName {
        name: "8",
        code: SDLK_8,
    },
    KeyName {
        name: "9",
        code: SDLK_9,
    },
    KeyName {
        name: ":",
        code: SDLK_COLON,
    },
    KeyName {
        name: ";",
        code: SDLK_SEMICOLON,
    },
    KeyName {
        name: "<",
        code: SDLK_LESS,
    },
    KeyName {
        name: "=",
        code: SDLK_EQUALS,
    },
    KeyName {
        name: ">",
        code: SDLK_GREATER,
    },
    KeyName {
        name: "?",
        code: SDLK_QUESTION,
    },
    KeyName {
        name: "@",
        code: SDLK_AT,
    },
    KeyName {
        name: "[",
        code: SDLK_LEFTBRACKET,
    },
    KeyName {
        name: "\\",
        code: SDLK_BACKSLASH,
    },
    KeyName {
        name: "]",
        code: SDLK_RIGHTBRACKET,
    },
    KeyName {
        name: "^",
        code: SDLK_CARET,
    },
    KeyName {
        name: "_",
        code: SDLK_UNDERSCORE,
    },
    KeyName {
        name: "`",
        code: SDLK_BACKQUOTE,
    },
    KeyName {
        name: "a",
        code: SDLK_A,
    },
    KeyName {
        name: "b",
        code: SDLK_B,
    },
    KeyName {
        name: "c",
        code: SDLK_C,
    },
    KeyName {
        name: "d",
        code: SDLK_D,
    },
    KeyName {
        name: "e",
        code: SDLK_E,
    },
    KeyName {
        name: "f",
        code: SDLK_F,
    },
    KeyName {
        name: "g",
        code: SDLK_G,
    },
    KeyName {
        name: "h",
        code: SDLK_H,
    },
    KeyName {
        name: "i",
        code: SDLK_I,
    },
    KeyName {
        name: "j",
        code: SDLK_J,
    },
    KeyName {
        name: "k",
        code: SDLK_K,
    },
    KeyName {
        name: "l",
        code: SDLK_L,
    },
    KeyName {
        name: "m",
        code: SDLK_M,
    },
    KeyName {
        name: "n",
        code: SDLK_N,
    },
    KeyName {
        name: "o",
        code: SDLK_O,
    },
    KeyName {
        name: "p",
        code: SDLK_P,
    },
    KeyName {
        name: "q",
        code: SDLK_Q,
    },
    KeyName {
        name: "r",
        code: SDLK_R,
    },
    KeyName {
        name: "s",
        code: SDLK_S,
    },
    KeyName {
        name: "t",
        code: SDLK_T,
    },
    KeyName {
        name: "u",
        code: SDLK_U,
    },
    KeyName {
        name: "v",
        code: SDLK_V,
    },
    KeyName {
        name: "w",
        code: SDLK_W,
    },
    KeyName {
        name: "x",
        code: SDLK_X,
    },
    KeyName {
        name: "y",
        code: SDLK_Y,
    },
    KeyName {
        name: "z",
        code: SDLK_Z,
    },
    KeyName {
        name: "Delete",
        code: SDLK_DELETE,
    },
    // SDL2 keypad
    KeyName {
        name: "Keypad-0",
        code: SDLK_KP_0,
    },
    KeyName {
        name: "Keypad-1",
        code: SDLK_KP_1,
    },
    KeyName {
        name: "Keypad-2",
        code: SDLK_KP_2,
    },
    KeyName {
        name: "Keypad-3",
        code: SDLK_KP_3,
    },
    KeyName {
        name: "Keypad-4",
        code: SDLK_KP_4,
    },
    KeyName {
        name: "Keypad-5",
        code: SDLK_KP_5,
    },
    KeyName {
        name: "Keypad-6",
        code: SDLK_KP_6,
    },
    KeyName {
        name: "Keypad-7",
        code: SDLK_KP_7,
    },
    KeyName {
        name: "Keypad-8",
        code: SDLK_KP_8,
    },
    KeyName {
        name: "Keypad-9",
        code: SDLK_KP_9,
    },
    KeyName {
        name: "Keypad-.",
        code: SDLK_KP_PERIOD,
    },
    KeyName {
        name: "Keypad-/",
        code: SDLK_KP_DIVIDE,
    },
    KeyName {
        name: "Keypad-*",
        code: SDLK_KP_MULTIPLY,
    },
    KeyName {
        name: "Keypad--",
        code: SDLK_KP_MINUS,
    },
    KeyName {
        name: "Keypad-+",
        code: SDLK_KP_PLUS,
    },
    KeyName {
        name: "Keypad-Enter",
        code: SDLK_KP_ENTER,
    },
    KeyName {
        name: "Keypad-=",
        code: SDLK_KP_EQUALS,
    },
    KeyName {
        name: "Up",
        code: SDLK_UP,
    },
    KeyName {
        name: "Down",
        code: SDLK_DOWN,
    },
    KeyName {
        name: "Right",
        code: SDLK_RIGHT,
    },
    KeyName {
        name: "Left",
        code: SDLK_LEFT,
    },
    KeyName {
        name: "Insert",
        code: SDLK_INSERT,
    },
    KeyName {
        name: "Home",
        code: SDLK_HOME,
    },
    KeyName {
        name: "End",
        code: SDLK_END,
    },
    KeyName {
        name: "PageUp",
        code: SDLK_PAGEUP,
    },
    KeyName {
        name: "PageDown",
        code: SDLK_PAGEDOWN,
    },
    KeyName {
        name: "F1",
        code: SDLK_F1,
    },
    KeyName {
        name: "F2",
        code: SDLK_F2,
    },
    KeyName {
        name: "F3",
        code: SDLK_F3,
    },
    KeyName {
        name: "F4",
        code: SDLK_F4,
    },
    KeyName {
        name: "F5",
        code: SDLK_F5,
    },
    KeyName {
        name: "F6",
        code: SDLK_F6,
    },
    KeyName {
        name: "F7",
        code: SDLK_F7,
    },
    KeyName {
        name: "F8",
        code: SDLK_F8,
    },
    KeyName {
        name: "F9",
        code: SDLK_F9,
    },
    KeyName {
        name: "F10",
        code: SDLK_F10,
    },
    KeyName {
        name: "F11",
        code: SDLK_F11,
    },
    KeyName {
        name: "F12",
        code: SDLK_F12,
    },
    KeyName {
        name: "F13",
        code: SDLK_F13,
    },
    KeyName {
        name: "F14",
        code: SDLK_F14,
    },
    KeyName {
        name: "F15",
        code: SDLK_F15,
    },
    KeyName {
        name: "RightShift",
        code: SDLK_RSHIFT,
    },
    KeyName {
        name: "LeftShift",
        code: SDLK_LSHIFT,
    },
    KeyName {
        name: "RightControl",
        code: SDLK_RCTRL,
    },
    KeyName {
        name: "LeftControl",
        code: SDLK_LCTRL,
    },
    KeyName {
        name: "RightAlt",
        code: SDLK_RALT,
    },
    KeyName {
        name: "LeftAlt",
        code: SDLK_LALT,
    },
    // Sentinel - must be last, code 0
    KeyName {
        name: "Unknown",
        code: 0,
    },
];

/// SDL keycode to name mapping (lazy initialized from KEYNAMES)
static KEY_NAMES: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    KEYNAMES
        .iter()
        .filter(|k| k.code != 0)
        .map(|k| (k.code, k.name))
        .collect()
});

/// Name to SDL keycode mapping (reverse lookup)
static NAME_TO_KEY: LazyLock<HashMap<&'static str, i32>> = LazyLock::new(|| {
    KEYNAMES
        .iter()
        .filter(|k| k.code != 0)
        .map(|k| (k.name, k.code))
        .collect()
});

/// Get the human-readable name for a keycode
/// This matches the C VControl_code2name behavior
pub fn key_name(keycode: i32) -> &'static str {
    // Linear search like C implementation
    for k in KEYNAMES.iter() {
        if k.code == keycode || k.code == 0 {
            return k.name;
        }
    }
    "Unknown"
}

/// Get the keycode for a key name (case-insensitive)
/// This matches the C VControl_name2code behavior
pub fn key_from_name(name: &str) -> Option<i32> {
    // Case-insensitive linear search like C strcasecmp
    for k in KEYNAMES.iter() {
        if k.code == 0 {
            return None; // Sentinel reached
        }
        if k.name.eq_ignore_ascii_case(name) {
            return Some(k.code);
        }
    }
    None
}

/// C-compatible wrapper for key_from_name
/// Returns 0 if not found (matches C behavior)
#[no_mangle]
pub extern "C" fn rust_VControl_name2code(name: *const c_char) -> c_int {
    if name.is_null() {
        return 0;
    }

    match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(name_str) => key_from_name(name_str).unwrap_or(0),
        Err(_) => 0,
    }
}

/// C-compatible null-terminated key name strings
/// These must be static C strings for FFI use
static CSTR_KEYNAMES: &[&'static [u8]] = &[
    b"Backspace\0",
    b"Tab\0",
    b"Clear\0",
    b"Return\0",
    b"Pause\0",
    b"Escape\0",
    b"Space\0",
    b"!\0",
    b"\"\0",
    b"Hash\0",
    b"$\0",
    b"&\0",
    b"'\0",
    b"(\0",
    b")\0",
    b"*\0",
    b"+\0",
    b",\0",
    b"-\0",
    b".\0",
    b"/\0",
    b"0\0",
    b"1\0",
    b"2\0",
    b"3\0",
    b"4\0",
    b"5\0",
    b"6\0",
    b"7\0",
    b"8\0",
    b"9\0",
    b":\0",
    b";\0",
    b"<\0",
    b"=\0",
    b">\0",
    b"?\0",
    b"@\0",
    b"[\0",
    b"\\\0",
    b"]\0",
    b"^\0",
    b"_\0",
    b"`\0",
    b"a\0",
    b"b\0",
    b"c\0",
    b"d\0",
    b"e\0",
    b"f\0",
    b"g\0",
    b"h\0",
    b"i\0",
    b"j\0",
    b"k\0",
    b"l\0",
    b"m\0",
    b"n\0",
    b"o\0",
    b"p\0",
    b"q\0",
    b"r\0",
    b"s\0",
    b"t\0",
    b"u\0",
    b"v\0",
    b"w\0",
    b"x\0",
    b"y\0",
    b"z\0",
    b"Delete\0",
    b"Keypad-0\0",
    b"Keypad-1\0",
    b"Keypad-2\0",
    b"Keypad-3\0",
    b"Keypad-4\0",
    b"Keypad-5\0",
    b"Keypad-6\0",
    b"Keypad-7\0",
    b"Keypad-8\0",
    b"Keypad-9\0",
    b"Keypad-.\0",
    b"Keypad-/\0",
    b"Keypad-*\0",
    b"Keypad--\0",
    b"Keypad-+\0",
    b"Keypad-Enter\0",
    b"Keypad-=\0",
    b"Up\0",
    b"Down\0",
    b"Right\0",
    b"Left\0",
    b"Insert\0",
    b"Home\0",
    b"End\0",
    b"PageUp\0",
    b"PageDown\0",
    b"F1\0",
    b"F2\0",
    b"F3\0",
    b"F4\0",
    b"F5\0",
    b"F6\0",
    b"F7\0",
    b"F8\0",
    b"F9\0",
    b"F10\0",
    b"F11\0",
    b"F12\0",
    b"F13\0",
    b"F14\0",
    b"F15\0",
    b"RightShift\0",
    b"LeftShift\0",
    b"RightControl\0",
    b"LeftControl\0",
    b"RightAlt\0",
    b"LeftAlt\0",
    b"Unknown\0",
];

/// C-compatible wrapper for key_name
/// Returns pointer to static null-terminated string
///
/// # Safety
/// The returned pointer is valid for the lifetime of the program
#[no_mangle]
pub extern "C" fn rust_VControl_code2name(code: c_int) -> *const c_char {
    // Find index in KEYNAMES by matching code
    for (i, k) in KEYNAMES.iter().enumerate() {
        if k.code == code || k.code == 0 {
            // Return the corresponding C string (null-terminated)
            if i < CSTR_KEYNAMES.len() {
                return CSTR_KEYNAMES[i].as_ptr() as *const c_char;
            }
            break;
        }
    }
    // Return "Unknown" which is the last entry
    CSTR_KEYNAMES[CSTR_KEYNAMES.len() - 1].as_ptr() as *const c_char
}

/// Get joystick button name
pub fn joy_button_name(joy_index: u32, button: i32) -> String {
    format!("Joy{}Button{}", joy_index, button)
}

/// Get joystick axis name
pub fn joy_axis_name(joy_index: u32, axis: i32, polarity: i32) -> String {
    let dir = if polarity < 0 { "-" } else { "+" };
    format!("Joy{}Axis{}{}", joy_index, axis, dir)
}

/// Get joystick hat name
pub fn joy_hat_name(joy_index: u32, hat: i32, direction: u8) -> String {
    let dir = match direction {
        1 => "Up",
        2 => "Right",
        4 => "Down",
        8 => "Left",
        _ => "?",
    };
    format!("Joy{}Hat{}{}", joy_index, hat, dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_name_known() {
        assert_eq!(key_name(32), "Space");
        assert_eq!(key_name(27), "Escape");
        assert_eq!(key_name(13), "Return");
        assert_eq!(key_name(97), "a"); // Lowercase in KEYNAMES table
    }

    #[test]
    fn test_key_name_unknown() {
        assert_eq!(key_name(99999), "Unknown");
    }

    #[test]
    fn test_key_from_name_exact() {
        assert_eq!(key_from_name("Space"), Some(32));
        assert_eq!(key_from_name("Escape"), Some(27));
        assert_eq!(key_from_name("a"), Some(97)); // Match KEYNAMES table
    }

    #[test]
    fn test_key_from_name_case_insensitive() {
        assert_eq!(key_from_name("space"), Some(32));
        assert_eq!(key_from_name("ESCAPE"), Some(27));
    }

    #[test]
    fn test_key_from_name_not_found() {
        assert_eq!(key_from_name("NotAKey"), None);
    }

    #[test]
    fn test_joy_button_name() {
        assert_eq!(joy_button_name(0, 5), "Joy0Button5");
        assert_eq!(joy_button_name(1, 0), "Joy1Button0");
    }

    #[test]
    fn test_joy_axis_name() {
        assert_eq!(joy_axis_name(0, 0, -1), "Joy0Axis0-");
        assert_eq!(joy_axis_name(0, 1, 1), "Joy0Axis1+");
    }

    #[test]
    fn test_joy_hat_name() {
        assert_eq!(joy_hat_name(0, 0, 1), "Joy0Hat0Up");
        assert_eq!(joy_hat_name(0, 0, 2), "Joy0Hat0Right");
        assert_eq!(joy_hat_name(0, 0, 4), "Joy0Hat0Down");
        assert_eq!(joy_hat_name(0, 0, 8), "Joy0Hat0Left");
    }

    #[test]
    fn test_function_keys() {
        assert_eq!(key_name(0x4000003A), "F1");
        assert_eq!(key_name(0x40000045), "F12");
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(key_name(0x4000004F), "Right");
        assert_eq!(key_name(0x40000050), "Left");
        assert_eq!(key_name(0x40000051), "Down");
        assert_eq!(key_name(0x40000052), "Up");
    }

    #[test]
    fn test_modifier_keys() {
        assert_eq!(key_name(0x400000E0), "LeftControl");
        assert_eq!(key_name(0x400000E1), "LeftShift");
        assert_eq!(key_name(0x400000E2), "LeftAlt");
    }
}
