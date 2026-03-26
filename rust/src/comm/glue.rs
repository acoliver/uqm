// Script-facing glue functions — NPCPhrase, NPCNumber, construct_response
// @plan PLAN-20260314-COMM.P04
// @requirement DS-REQ-005, DS-REQ-006, DS-REQ-007, DS-REQ-008, DS-REQ-009, DS-REQ-010

use std::ffi::c_void;

/// Callback type for phrase completion notification.
pub type PhraseCallback = Option<unsafe extern "C" fn()>;

// Special phrase indices matching C commglue.h
/// Index 0 is a no-op (DS-REQ-007)
pub const PHRASE_NOOP: i32 = 0;
/// Negative indices are alliance name references (DS-REQ-006)
pub const GLOBAL_PLAYER_NAME: i32 = -1;
pub const GLOBAL_SHIP_NAME: i32 = -2;

/// Resolve a raw pointer from C to a Rust String, or None if null.
///
/// # Safety
/// `ptr` must be a valid null-terminated C string, or null.
unsafe fn ptr_to_string(ptr: *const u8) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let cstr = std::ffi::CStr::from_ptr(ptr as *const std::ffi::c_char);
        Some(cstr.to_string_lossy().into_owned())
    }
}

/// Resolve a phrase index to its text.
///
/// Returns `None` for index 0 (no-op) or if the phrase table is null.
///
/// # Safety
/// `phrases_handle` must be a valid conversation phrases handle or null.
#[cfg(not(test))]
pub unsafe fn resolve_phrase(phrases_handle: *const c_void, index: i32) -> Option<String> {
    extern "C" {
        fn c_get_conversation_phrase(phrases: *const c_void, index: i32) -> *const u8;
        fn c_get_commander_name() -> *const u8;
        fn c_get_ship_name() -> *const u8;
        fn c_get_alliance_name(index: i32) -> *const u8;
    }

    if index == PHRASE_NOOP {
        return None;
    }

    unsafe {
        let ptr = if index == GLOBAL_PLAYER_NAME {
            c_get_commander_name()
        } else if index == GLOBAL_SHIP_NAME {
            c_get_ship_name()
        } else if index < 0 {
            c_get_alliance_name(index)
        } else if !phrases_handle.is_null() {
            c_get_conversation_phrase(phrases_handle, index)
        } else {
            return None;
        };

        ptr_to_string(ptr)
    }
}

/// Test-mode resolve_phrase: returns placeholder strings without calling C.
#[cfg(test)]
pub unsafe fn resolve_phrase(phrases_handle: *const c_void, index: i32) -> Option<String> {
    if index == PHRASE_NOOP {
        return None;
    }
    if index == GLOBAL_PLAYER_NAME {
        return Some("Commander".to_string());
    }
    if index == GLOBAL_SHIP_NAME {
        return Some("Vindicator".to_string());
    }
    if index < 0 {
        return Some(format!("Alliance{}", -index));
    }
    if phrases_handle.is_null() {
        return None;
    }
    Some(format!("Phrase#{}", index))
}

/// Build a composite response text from multiple phrase fragments.
///
/// Concatenates resolved texts with a single space between fragments.
/// Skips fragments that resolve to None.
///
/// # Safety
/// `phrases_handle` must be valid for the encounter lifetime.
pub unsafe fn construct_response(phrases_handle: *const c_void, fragments: &[i32]) -> String {
    let mut result = String::new();
    for &idx in fragments {
        if let Some(text) = unsafe { resolve_phrase(phrases_handle, idx) } {
            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(&text);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_index_returns_none() {
        unsafe {
            let result = resolve_phrase(std::ptr::null(), PHRASE_NOOP);
            assert!(result.is_none());
        }
    }

    #[test]
    fn null_phrases_returns_none_for_positive() {
        unsafe {
            let result = resolve_phrase(std::ptr::null(), 42);
            assert!(result.is_none());
        }
    }

    #[test]
    fn global_player_name_resolves() {
        unsafe {
            let result = resolve_phrase(std::ptr::null(), GLOBAL_PLAYER_NAME);
            assert_eq!(result, Some("Commander".to_string()));
        }
    }

    #[test]
    fn global_ship_name_resolves() {
        unsafe {
            let result = resolve_phrase(std::ptr::null(), GLOBAL_SHIP_NAME);
            assert_eq!(result, Some("Vindicator".to_string()));
        }
    }

    #[test]
    fn negative_index_is_alliance() {
        unsafe {
            let result = resolve_phrase(std::ptr::null(), -5);
            assert_eq!(result, Some("Alliance5".to_string()));
        }
    }

    #[test]
    fn positive_index_with_handle() {
        unsafe {
            // Use a non-null sentinel as the phrases handle
            let fake_handle = 0x1234 as *const c_void;
            let result = resolve_phrase(fake_handle, 7);
            assert_eq!(result, Some("Phrase#7".to_string()));
        }
    }

    #[test]
    fn construct_response_concatenates() {
        unsafe {
            let fake_handle = 0x1234 as *const c_void;
            let result = construct_response(fake_handle, &[1, 2, 3]);
            assert_eq!(result, "Phrase#1 Phrase#2 Phrase#3");
        }
    }

    #[test]
    fn construct_response_skips_noops() {
        unsafe {
            let fake_handle = 0x1234 as *const c_void;
            let result = construct_response(fake_handle, &[1, 0, 3]);
            assert_eq!(result, "Phrase#1 Phrase#3");
        }
    }

    #[test]
    fn construct_response_empty_fragments() {
        unsafe {
            let fake_handle = 0x1234 as *const c_void;
            let result = construct_response(fake_handle, &[]);
            assert_eq!(result, "");
        }
    }

    #[test]
    fn construct_response_with_special_indices() {
        unsafe {
            let fake_handle = 0x1234 as *const c_void;
            let result = construct_response(fake_handle, &[1, GLOBAL_PLAYER_NAME, 3]);
            assert_eq!(result, "Phrase#1 Commander Phrase#3");
        }
    }

    #[test]
    fn special_indices_recognized() {
        assert_eq!(PHRASE_NOOP, 0);
        assert_eq!(GLOBAL_PLAYER_NAME, -1);
        assert_eq!(GLOBAL_SHIP_NAME, -2);
    }
}
