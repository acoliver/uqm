//! Menu binding query — initialized-child production accessor
//!
//! Reads the same loaded `menu.<name>.N` resource strings as
//! `register_menu_controls` in `sc2/src/libs/input/sdl/input.c`, parses each
//! alternate through the production gesture parser
//! ([`rust_VControl_ParseGesture`]), and selects the first keyboard
//! (`VCONTROL_KEY`) binding.
//!
//! This must be called from an initialized child (after the resource system
//! is initialized and the `menu.*` index is loaded). The caller never
//! assumes an SDL key before that.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P00

use std::ffi::CString;

use super::ffi::{
    rust_VControl_ParseGesture, GestureUnion, VCONTROL_GESTURE, VCONTROL_KEY, VCONTROL_NONE,
};
use crate::resource::ffi_bridge::{res_GetString, res_IsString};

/// The historical accessor formatted resource keys into a 40-byte buffer
/// with `snprintf(buf, 39, ...)`, so a key longer than 38 bytes is queried
/// truncated. Resource keys never approach this length in practice; the
/// truncation is preserved for exact behavioral parity.
const RESOURCE_KEY_MAX_BYTES: usize = 38;

/// Result of a menu binding query.
///
/// * `found` — a `VCONTROL_KEY` binding was found
/// * `key_code` — the SDL keycode of the binding (valid only if `found`)
/// * `binding_id` — stable identifier for the binding (1-based alternate index)
/// * `num_alternates` — total alternates found for this menu control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuBindingResult {
    pub found: bool,
    pub key_code: i32,
    pub binding_id: u32,
    pub num_alternates: u32,
}

/// Query the `menu.<name>.N` binding for a specific menu control.
///
/// Iterates `menu.<name>.1`, `menu.<name>.2`, ... exactly as
/// `register_menu_controls`, stops at the first missing resource string,
/// parses each existing string with the production gesture parser, counts
/// every existing alternate, and selects the first `VCONTROL_KEY` gesture.
///
/// MUST be called from an initialized child after the resource system is
/// initialized and the `menu.*` index is loaded.
pub fn query_menu_binding(menu_name: &str) -> MenuBindingResult {
    let mut result = MenuBindingResult::default();
    let mut index: u32 = 1;
    loop {
        let full_key = format!("menu.{menu_name}.{index}");
        let key = match CString::new(truncate_resource_key(&full_key)) {
            Ok(key) => key,
            // A menu name with an interior NUL can never address a loaded
            // resource string; treat it as "no binding".
            Err(_) => break,
        };

        // SAFETY: key is a valid, aligned, NUL-terminated C string.
        if unsafe { res_IsString(key.as_ptr()) } == 0 {
            break;
        }

        // SAFETY: res_GetString never returns null and the returned pointer
        // stays valid until the resource entry is modified or removed.
        let spec = unsafe { res_GetString(key.as_ptr()) };

        // The gesture starts from a known NONE state so an unparseable spec
        // is reported as VCONTROL_NONE rather than stale data.
        let mut gesture = VCONTROL_GESTURE {
            gesture_type: VCONTROL_NONE,
            gesture: GestureUnion { data: [0, 0, 0] },
        };
        // SAFETY: gesture is a valid, aligned out-pointer and spec is a
        // valid NUL-terminated C string.
        unsafe { rust_VControl_ParseGesture(&mut gesture, spec) };

        result.num_alternates += 1;

        if !result.found && gesture.gesture_type == VCONTROL_KEY {
            result.found = true;
            result.binding_id = index;
            // SAFETY: gesture_type is VCONTROL_KEY, so the key field of the
            // union is the active variant and reading it is defined.
            result.key_code = unsafe { gesture.gesture.key };
        }

        index += 1;
    }
    result
}

/// Truncate a resource key to the byte budget the historical accessor used,
/// never splitting a multi-byte UTF-8 character.
fn truncate_resource_key(full: &str) -> &str {
    if full.len() <= RESOURCE_KEY_MAX_BYTES {
        return full;
    }
    let mut end = RESOURCE_KEY_MAX_BYTES;
    while !full.is_char_boundary(end) {
        end -= 1;
    }
    &full[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ffi_bridge::{res_PutString, InitResourceSystem, UninitResourceSystem};
    use serial_test::serial;

    /// Reset the resource system to a pristine initialized state so tests do
    /// not observe each other's strings.
    fn reset_resource_state() {
        // SAFETY: called with no pointer arguments between serial tests.
        unsafe {
            UninitResourceSystem();
            let sentinel = InitResourceSystem();
            assert!(!sentinel.is_null());
        }
    }

    fn put_string(key: &str, value: &str) {
        let key_c = CString::new(key).unwrap();
        let value_c = CString::new(value).unwrap();
        // SAFETY: both arguments are valid NUL-terminated C strings.
        unsafe { res_PutString(key_c.as_ptr(), value_c.as_ptr()) };
    }

    /// The historical accessor's 38-byte truncation.
    #[test]
    fn test_truncate_resource_key() {
        assert_eq!(truncate_resource_key("menu.down.1"), "menu.down.1");
        let long = "m".repeat(64);
        assert_eq!(truncate_resource_key(&long).len(), 38);
        // Multi-byte characters are never split.
        let multibyte = "é".repeat(32);
        let truncated = truncate_resource_key(&multibyte);
        assert!(truncated.len() <= RESOURCE_KEY_MAX_BYTES);
        assert!(truncated.chars().all(|c| c == 'é'));
    }

    #[test]
    #[serial]
    fn test_first_key_binding_selected() {
        reset_resource_state();
        put_string("menu.zqk.1", "key Down");
        put_string("menu.zqk.2", "joystick 0 button 0");

        let result = query_menu_binding("zqk");
        assert!(result.found);
        assert_eq!(result.key_code, 0x4000_0051); // SDLK_DOWN
        assert_eq!(result.binding_id, 1);
        assert_eq!(result.num_alternates, 2);
    }

    #[test]
    #[serial]
    fn test_joystick_first_selects_later_key() {
        reset_resource_state();
        put_string("menu.zqj.1", "joystick 0 axis 1 positive");
        put_string("menu.zqj.2", "key Down");

        let result = query_menu_binding("zqj");
        assert!(result.found);
        assert_eq!(result.key_code, 0x4000_0051);
        assert_eq!(result.binding_id, 2);
        assert_eq!(result.num_alternates, 2);
    }

    #[test]
    #[serial]
    fn test_missing_binding_returns_default() {
        reset_resource_state();
        let result = query_menu_binding("nosuchcontrol");
        assert_eq!(result, MenuBindingResult::default());
    }

    #[test]
    #[serial]
    fn test_missing_alternate_stops_iteration() {
        reset_resource_state();
        put_string("menu.zqg.1", "key Down");
        // menu.zqg.2 is absent; menu.zqg.3 must not be consulted.
        put_string("menu.zqg.3", "key Down");

        let result = query_menu_binding("zqg");
        assert!(result.found);
        assert_eq!(result.binding_id, 1);
        assert_eq!(result.num_alternates, 1);
    }

    #[test]
    #[serial]
    fn test_unparseable_spec_counts_but_is_not_selected() {
        reset_resource_state();
        put_string("menu.zqu.1", "banana pudding");
        put_string("menu.zqu.2", "key Down");

        let result = query_menu_binding("zqu");
        assert!(result.found);
        assert_eq!(result.key_code, 0x4000_0051);
        assert_eq!(result.binding_id, 2);
        assert_eq!(result.num_alternates, 2);
    }
}
