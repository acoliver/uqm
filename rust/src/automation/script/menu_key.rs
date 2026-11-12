use serde::Deserialize;

// ===========================================================================
//  MenuKey — six typed variants (REQ-SCRIPT-005)
// ===========================================================================

/// The six menu control variants exposed to automation, mapped from the C
/// `controls.h` enum indices.
///
/// The indices match `KEY_MENU_UP..KEY_MENU_CANCEL` (5..=10):
///
/// | Variant | C enum constant | Index |
/// |---------|-----------------|-------|
/// | Up      | `KEY_MENU_UP`     | 5     |
/// | Down    | `KEY_MENU_DOWN`   | 6     |
/// | Left    | `KEY_MENU_LEFT`   | 7     |
/// | Right   | `KEY_MENU_RIGHT`  | 8     |
/// | Select  | `KEY_MENU_SELECT` | 9     |
/// | Cancel  | `KEY_MENU_CANCEL` | 10    |
///
/// Unknown or numeric-only identifiers are rejected; only the six closed
/// string names are accepted.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P01
/// @requirement REQ-SCRIPT-005
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MenuKey {
    Up = 5,
    Down = 6,
    Left = 7,
    Right = 8,
    Select = 9,
    Cancel = 10,
    /// Removes the selected ship in the Super Melee fleet editor.
    Delete = 18,
}

impl MenuKey {
    /// All variants, in `controls.h` enum order.
    pub const ALL: [MenuKey; 7] = [
        MenuKey::Up,
        MenuKey::Down,
        MenuKey::Left,
        MenuKey::Right,
        MenuKey::Select,
        MenuKey::Cancel,
        MenuKey::Delete,
    ];

    /// The stable `controls.h` index for this key.
    #[must_use]
    #[inline]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// The canonical lowercase string name, matching `menu.<name>.N`
    /// resource keys in `sc2/content/menu.key`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            MenuKey::Up => "up",
            MenuKey::Down => "down",
            MenuKey::Left => "left",
            MenuKey::Right => "right",
            MenuKey::Select => "select",
            MenuKey::Cancel => "cancel",
            MenuKey::Delete => "delete",
        }
    }

    /// Parse a key from its canonical string name.
    ///
    /// Returns `None` for any value that is not exactly one of the six names.
    /// Numeric or unknown identifiers are rejected (REQ-SCRIPT-005).
    #[must_use]
    pub fn from_name(name: &str) -> Option<MenuKey> {
        match name {
            "up" => Some(MenuKey::Up),
            "down" => Some(MenuKey::Down),
            "left" => Some(MenuKey::Left),
            "right" => Some(MenuKey::Right),
            "select" => Some(MenuKey::Select),
            "cancel" => Some(MenuKey::Cancel),
            "delete" => Some(MenuKey::Delete),
            _ => None,
        }
    }

    /// Checked conversion from a raw index. Returns `None` for any index that
    /// is not a scriptable menu key.
    #[must_use]
    pub const fn from_index(index: u8) -> Option<MenuKey> {
        match index {
            5 => Some(MenuKey::Up),
            6 => Some(MenuKey::Down),
            7 => Some(MenuKey::Left),
            8 => Some(MenuKey::Right),
            9 => Some(MenuKey::Select),
            10 => Some(MenuKey::Cancel),
            18 => Some(MenuKey::Delete),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for MenuKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        MenuKey::from_name(&name).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "unknown menu key '{name}'; expected one of: up, down, left, right, select, cancel, delete"
            ))
        })
    }
}
