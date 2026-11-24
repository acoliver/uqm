//! Menu binding query — initialized-child production accessor
//!
//! Reads the same loaded `menu.<name>.N` resource strings as
//! `register_menu_controls` in `sc2/src/libs/input/sdl/input.c`, parses each
//! alternate through the production gesture parser ([`Gesture::parse`]), and
//! selects the first keyboard binding.
//!
//! This must be called from an initialized child (after the resource system
//! is initialized and the `menu.*` index is loaded). The caller never
//! assumes an SDL key before that.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P00

use super::gesture::Gesture;

/// The historical accessor formatted resource keys into a 40-byte buffer
/// with `snprintf(buf, 39, ...)`, so a key longer than 38 bytes is queried
/// truncated. Resource keys never approach this length in practice; the
/// truncation is preserved for exact behavioral parity.
const RESOURCE_KEY_MAX_BYTES: usize = 38;

/// Result of a menu binding query.
///
/// * `found` — a keyboard binding was found
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
/// every existing alternate, and selects the first keyboard gesture.
///
/// MUST be called from an initialized child after the resource system is
/// initialized and the `menu.*` index is loaded.
pub fn query_menu_binding(
    bindings: &crate::resource::settings::Bindings,
    menu_name: &str,
) -> MenuBindingResult {
    let mut result = MenuBindingResult::default();
    let mut index: u32 = 1;
    loop {
        let full_key = format!("menu.{menu_name}.{index}");
        let Some(spec) = bindings.string(truncate_resource_key(&full_key)) else {
            break;
        };
        let gesture = Gesture::parse(&spec);

        result.num_alternates += 1;

        if let (false, Some(Gesture::Key(key_code))) = (result.found, gesture) {
            result.found = true;
            result.binding_id = index;
            result.key_code = key_code;
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
    fn test_first_key_binding_selected() {
        let mut bindings = crate::resource::settings::Bindings::default();
        bindings.put_string("menu.zqk.1", "key Down");
        bindings.put_string("menu.zqk.2", "joystick 0 button 0");

        let result = query_menu_binding(&bindings, "zqk");
        assert!(result.found);
        assert_eq!(result.key_code, 0x4000_0051); // SDLK_DOWN
        assert_eq!(result.binding_id, 1);
        assert_eq!(result.num_alternates, 2);
    }

    #[test]
    fn test_joystick_first_selects_later_key() {
        let mut bindings = crate::resource::settings::Bindings::default();
        bindings.put_string("menu.zqj.1", "joystick 0 axis 1 positive");
        bindings.put_string("menu.zqj.2", "key Down");

        let result = query_menu_binding(&bindings, "zqj");
        assert!(result.found);
        assert_eq!(result.key_code, 0x4000_0051);
        assert_eq!(result.binding_id, 2);
        assert_eq!(result.num_alternates, 2);
    }

    #[test]
    fn test_missing_binding_returns_default() {
        let bindings = crate::resource::settings::Bindings::default();
        let result = query_menu_binding(&bindings, "nosuchcontrol");
        assert_eq!(result, MenuBindingResult::default());
    }

    #[test]
    fn test_missing_alternate_stops_iteration() {
        let mut bindings = crate::resource::settings::Bindings::default();
        bindings.put_string("menu.zqg.1", "key Down");
        // menu.zqg.2 is absent; menu.zqg.3 must not be consulted.
        bindings.put_string("menu.zqg.3", "key Down");

        let result = query_menu_binding(&bindings, "zqg");
        assert!(result.found);
        assert_eq!(result.binding_id, 1);
        assert_eq!(result.num_alternates, 1);
    }

    #[test]
    fn test_unparseable_spec_counts_but_is_not_selected() {
        let mut bindings = crate::resource::settings::Bindings::default();
        bindings.put_string("menu.zqu.1", "banana pudding");
        bindings.put_string("menu.zqu.2", "key Down");

        let result = query_menu_binding(&bindings, "zqu");
        assert!(result.found);
        assert_eq!(result.key_code, 0x4000_0051);
        assert_eq!(result.binding_id, 2);
        assert_eq!(result.num_alternates, 2);
    }
}
