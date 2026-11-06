//! Geometry and colors for game-options rendering.

use crate::navigation::abi::{Color, EXTENT, POINT, RECT, SHIP_NAME_HEIGHT, SHIP_NAME_WIDTH};
use crate::ui::rgb15;

pub(super) const CAPTAIN_NAME_RECT: RECT = RECT {
    corner: POINT { x: 3, y: 10 },
    extent: EXTENT {
        width: (SHIP_NAME_WIDTH - 2) as i16,
        height: SHIP_NAME_HEIGHT as i16,
    },
};
pub(super) const SHIP_NAME_RECT: RECT = RECT {
    corner: POINT { x: 2, y: 20 },
    extent: EXTENT {
        width: SHIP_NAME_WIDTH as i16,
        height: SHIP_NAME_HEIGHT as i16,
    },
};

pub(super) const CAPTAIN_NAME_TEXT_COLOR: Color = rgb15(0x16, 0x0b, 0x1f);

pub(super) const BLACK_COLOR: Color = rgb15(0, 0, 0);
