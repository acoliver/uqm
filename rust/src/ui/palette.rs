//! Colors used by game menus, in the original RGB15 palette.

use crate::navigation::abi::Color;

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

pub(super) const MENU_HIGHLIGHT_COLOR: Color = rgb15(0x1F, 0x1F, 0x00);
pub(super) const MENU_TEXT_COLOR: Color = rgb15(0x06, 0x06, 0x06);
pub(super) const PCMENU_BACKGROUND_COLOR: Color = rgb15(0x00, 0x00, 0x15);
pub(super) const PCMENU_BOTTOM_RIGHT_BORDER_COLOR: Color = rgb15(0x06, 0x06, 0x06);
pub(super) const PCMENU_SELECTION_BACKGROUND_COLOR: Color = rgb15(0x0A, 0x0A, 0x1F);
pub(super) const PCMENU_SELECTION_TEXT_COLOR: Color = rgb15(0x0A, 0x1F, 0x1F);
pub(super) const PCMENU_TEXT_COLOR: Color = rgb15(0x00, 0x15, 0x15);
pub(super) const PCMENU_TOP_LEFT_BORDER_COLOR: Color = rgb15(0x0F, 0x0F, 0x0F);
pub(super) const SHADOWBOX_BACKGROUND_COLOR: Color = rgb15(0x14, 0x14, 0x14);
pub(super) const SHADOWBOX_DARK_COLOR: Color = rgb15(0x08, 0x08, 0x08);
pub(super) const SHADOWBOX_MEDIUM_COLOR: Color = rgb15(0x10, 0x10, 0x10);
pub(super) const THREEDOMENU_TEXT_COLOR: Color = rgb15(0x00, 0x1F, 0x00);
