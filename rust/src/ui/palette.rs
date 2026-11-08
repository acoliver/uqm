//! Colors used by game menus, in the original RGB15 palette.

use crate::graphics::native_abi::Color;

use super::rgb15;

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

pub(super) fn pilot_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 7] = [
        (10, 20, 24),
        (6, 16, 22),
        (3, 14, 20),
        (2, 12, 17),
        (1, 11, 15),
        (1, 9, 13),
        (0, 7, 11),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_colors_preserve_the_rgb15_channels() {
        for (color, expected) in [
            (MENU_HIGHLIGHT_COLOR, (255, 255, 0)),
            (MENU_TEXT_COLOR, (49, 49, 49)),
            (PCMENU_BACKGROUND_COLOR, (0, 0, 173)),
            (PCMENU_BOTTOM_RIGHT_BORDER_COLOR, (49, 49, 49)),
            (PCMENU_SELECTION_BACKGROUND_COLOR, (82, 82, 255)),
            (PCMENU_SELECTION_TEXT_COLOR, (82, 255, 255)),
            (PCMENU_TEXT_COLOR, (0, 173, 173)),
            (PCMENU_TOP_LEFT_BORDER_COLOR, (123, 123, 123)),
            (SHADOWBOX_BACKGROUND_COLOR, (165, 165, 165)),
            (SHADOWBOX_DARK_COLOR, (66, 66, 66)),
            (SHADOWBOX_MEDIUM_COLOR, (132, 132, 132)),
            (THREEDOMENU_TEXT_COLOR, (0, 255, 0)),
        ] {
            assert_eq!((color.r, color.g, color.b), expected);
            assert_eq!(color.a, 255);
        }
    }
}
