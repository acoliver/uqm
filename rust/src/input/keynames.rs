//! Key name mappings
//!
//! Maps SDL keycodes to human-readable names and vice versa.

use std::collections::HashMap;
use std::sync::LazyLock;

/// SDL keycode to name mapping
static KEY_NAMES: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Common keys
    m.insert(0, "Unknown");
    m.insert(8, "Backspace");
    m.insert(9, "Tab");
    m.insert(13, "Return");
    m.insert(27, "Escape");
    m.insert(32, "Space");
    m.insert(39, "'");
    m.insert(44, ",");
    m.insert(45, "-");
    m.insert(46, ".");
    m.insert(47, "/");

    // Numbers
    m.insert(48, "0");
    m.insert(49, "1");
    m.insert(50, "2");
    m.insert(51, "3");
    m.insert(52, "4");
    m.insert(53, "5");
    m.insert(54, "6");
    m.insert(55, "7");
    m.insert(56, "8");
    m.insert(57, "9");

    m.insert(59, ";");
    m.insert(61, "=");

    // Letters (lowercase)
    m.insert(97, "A");
    m.insert(98, "B");
    m.insert(99, "C");
    m.insert(100, "D");
    m.insert(101, "E");
    m.insert(102, "F");
    m.insert(103, "G");
    m.insert(104, "H");
    m.insert(105, "I");
    m.insert(106, "J");
    m.insert(107, "K");
    m.insert(108, "L");
    m.insert(109, "M");
    m.insert(110, "N");
    m.insert(111, "O");
    m.insert(112, "P");
    m.insert(113, "Q");
    m.insert(114, "R");
    m.insert(115, "S");
    m.insert(116, "T");
    m.insert(117, "U");
    m.insert(118, "V");
    m.insert(119, "W");
    m.insert(120, "X");
    m.insert(121, "Y");
    m.insert(122, "Z");

    m.insert(91, "[");
    m.insert(92, "\\");
    m.insert(93, "]");
    m.insert(96, "`");
    m.insert(127, "Delete");

    // Function keys (SDL scancodes | 0x40000000)
    m.insert(0x4000003A, "F1");
    m.insert(0x4000003B, "F2");
    m.insert(0x4000003C, "F3");
    m.insert(0x4000003D, "F4");
    m.insert(0x4000003E, "F5");
    m.insert(0x4000003F, "F6");
    m.insert(0x40000040, "F7");
    m.insert(0x40000041, "F8");
    m.insert(0x40000042, "F9");
    m.insert(0x40000043, "F10");
    m.insert(0x40000044, "F11");
    m.insert(0x40000045, "F12");

    // Navigation keys
    m.insert(0x40000049, "Insert");
    m.insert(0x4000004A, "Home");
    m.insert(0x4000004B, "PageUp");
    m.insert(0x4000004D, "End");
    m.insert(0x4000004E, "PageDown");

    // Arrow keys
    m.insert(0x4000004F, "Right");
    m.insert(0x40000050, "Left");
    m.insert(0x40000051, "Down");
    m.insert(0x40000052, "Up");

    // Numpad
    m.insert(0x40000053, "NumLock");
    m.insert(0x40000054, "Keypad /");
    m.insert(0x40000055, "Keypad *");
    m.insert(0x40000056, "Keypad -");
    m.insert(0x40000057, "Keypad +");
    m.insert(0x40000058, "Keypad Enter");
    m.insert(0x40000059, "Keypad 1");
    m.insert(0x4000005A, "Keypad 2");
    m.insert(0x4000005B, "Keypad 3");
    m.insert(0x4000005C, "Keypad 4");
    m.insert(0x4000005D, "Keypad 5");
    m.insert(0x4000005E, "Keypad 6");
    m.insert(0x4000005F, "Keypad 7");
    m.insert(0x40000060, "Keypad 8");
    m.insert(0x40000061, "Keypad 9");
    m.insert(0x40000062, "Keypad 0");
    m.insert(0x40000063, "Keypad .");

    // Modifier keys
    m.insert(0x400000E0, "Left Ctrl");
    m.insert(0x400000E1, "Left Shift");
    m.insert(0x400000E2, "Left Alt");
    m.insert(0x400000E3, "Left GUI");
    m.insert(0x400000E4, "Right Ctrl");
    m.insert(0x400000E5, "Right Shift");
    m.insert(0x400000E6, "Right Alt");
    m.insert(0x400000E7, "Right GUI");

    m
});

/// Name to SDL keycode mapping (reverse lookup)
static NAME_TO_KEY: LazyLock<HashMap<&'static str, i32>> = LazyLock::new(|| {
    KEY_NAMES
        .iter()
        .map(|(&code, &name)| (name, code))
        .collect()
});

/// Get the human-readable name for a keycode
pub fn key_name(keycode: i32) -> &'static str {
    KEY_NAMES.get(&keycode).copied().unwrap_or("Unknown")
}

/// Get the keycode for a key name (case-insensitive)
pub fn key_from_name(name: &str) -> Option<i32> {
    // Try exact match first
    if let Some(&code) = NAME_TO_KEY.get(name) {
        return Some(code);
    }

    // Try case-insensitive match
    let upper = name.to_uppercase();
    for (&n, &code) in NAME_TO_KEY.iter() {
        if n.to_uppercase() == upper {
            return Some(code);
        }
    }

    None
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
        assert_eq!(key_name(97), "A");
    }

    #[test]
    fn test_key_name_unknown() {
        assert_eq!(key_name(99999), "Unknown");
    }

    #[test]
    fn test_key_from_name_exact() {
        assert_eq!(key_from_name("Space"), Some(32));
        assert_eq!(key_from_name("Escape"), Some(27));
        assert_eq!(key_from_name("A"), Some(97));
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
        assert_eq!(key_name(0x400000E0), "Left Ctrl");
        assert_eq!(key_name(0x400000E1), "Left Shift");
        assert_eq!(key_name(0x400000E2), "Left Alt");
    }
}
