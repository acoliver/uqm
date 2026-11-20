//! Super Melee's live C ABI, ported from the original GPL-2.0-or-later UI.
pub mod battle_picker;
pub(crate) mod build_picker;
pub(crate) mod load_teams;
pub mod menu;
pub(crate) mod protocol;

pub(crate) const MELEE_PICK_MASK_PMAP_ANIM: &[u8; 23] = b"graphics.meleepickship\0";
pub(crate) const MELEE_SCREEN_PMAP_ANIM: &[u8; 19] = b"graphics.meleemenu\0";
pub(crate) const MELEE_MUSIC: &[u8; 16] = b"music.meleemenu\0";
