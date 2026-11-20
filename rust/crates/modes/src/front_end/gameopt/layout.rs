//! Geometry and colors for game-options rendering.

use crate::graphics::{geometry::Color, geometry::Extent, geometry::Point, geometry::Rect};
use crate::starbase::ids::{SHIP_NAME_HEIGHT, SHIP_NAME_WIDTH};
use crate::ui::rgb15;

pub(super) const CAPTAIN_NAME_RECT: Rect = Rect {
    corner: Point { x: 3, y: 10 },
    extent: Extent {
        width: SHIP_NAME_WIDTH - 2,
        height: SHIP_NAME_HEIGHT,
    },
};
pub(super) const SHIP_NAME_RECT: Rect = Rect {
    corner: Point { x: 2, y: 20 },
    extent: Extent {
        width: SHIP_NAME_WIDTH,
        height: SHIP_NAME_HEIGHT,
    },
};

pub(super) const CAPTAIN_NAME_TEXT_COLOR: Color = rgb15(0x16, 0x0b, 0x1f);

pub(super) const BLACK_COLOR: Color = rgb15(0, 0, 0);

pub(super) fn name_style(captain: bool) -> (Rect, Color, Color) {
    if captain {
        (
            CAPTAIN_NAME_RECT,
            rgb15(0x0a, 0x0a, 0x1f),
            rgb15(0x0a, 0x1f, 0x1f),
        )
    } else {
        (SHIP_NAME_RECT, rgb15(0x0f, 0, 0), rgb15(0x1f, 0x0a, 0))
    }
}
