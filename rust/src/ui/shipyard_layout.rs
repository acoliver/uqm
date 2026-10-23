//! Hangar positions and cursor traversal for the shipped PC layout.
#![allow(non_snake_case)]

use crate::navigation::abi::{BYTE, EXTENT, POINT, RECT, SBYTE};

// Mirrors the shipped C build, which does not define USE_3DO_HANGAR.
pub(crate) const USE_3DO_HANGAR: bool = false;
pub(crate) const HANGAR_SHIPS_ROW: u8 = if USE_3DO_HANGAR { 4 } else { 6 };
pub(crate) const HANGAR_Y: i16 = if USE_3DO_HANGAR { 64 } else { 88 };
pub(crate) const HANGAR_DY: i16 = if USE_3DO_HANGAR { 44 } else { 84 };
pub(crate) const HANGAR_X_COORDS: &[i16] = if USE_3DO_HANGAR {
    &[19, 60, 116, 157]
} else {
    &[0, 38, 76, 131, 169, 207]
};
pub(crate) const HANGAR_ROWS: u8 = 12 / HANGAR_SHIPS_ROW;
pub(crate) const SHIP_WIN_WIDTH: i16 = 34;
pub(crate) const SHIP_WIN_HEIGHT: i16 = SHIP_WIN_WIDTH + 6;

pub(crate) const fn DMS_GetEscortShipRect(slot: BYTE) -> RECT {
    let row = slot / HANGAR_SHIPS_ROW;
    let col = slot % HANGAR_SHIPS_ROW;
    RECT {
        corner: POINT {
            x: HANGAR_X_COORDS[col as usize],
            y: HANGAR_Y + HANGAR_DY * row as i16,
        },
        extent: EXTENT {
            width: SHIP_WIN_WIDTH,
            height: SHIP_WIN_HEIGHT,
        },
    }
}

pub(crate) const fn DMS_MoveCursor(state: BYTE, dx: SBYTE, dy: SBYTE) -> BYTE {
    let mut row = (state & 0x0f) / HANGAR_SHIPS_ROW;
    let mut col = (state & 0x0f) % HANGAR_SHIPS_ROW;
    let mut flagship = state >> 4 != 0;
    if dy != 0 {
        if flagship {
            row = HANGAR_ROWS;
        }
        row = ((row as i32 + (HANGAR_ROWS + 1) as i32 + dy as i32) % (HANGAR_ROWS + 1) as i32)
            as BYTE;
        flagship = row == HANGAR_ROWS;
        if flagship {
            row = 0;
        }
    } else if dx != 0 && !flagship {
        col =
            ((col as i32 + HANGAR_SHIPS_ROW as i32 + dx as i32) % HANGAR_SHIPS_ROW as i32) as BYTE;
    }
    (row * HANGAR_SHIPS_ROW + col) | if flagship { 0xf0 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_wraps_through_flagship_and_keeps_column() {
        for col in 0..HANGAR_SHIPS_ROW {
            let flagship = 0xf0 | col;
            let bottom = (HANGAR_ROWS - 1) * HANGAR_SHIPS_ROW + col;
            assert_eq!(DMS_MoveCursor(col, 0, -1), flagship);
            assert_eq!(DMS_MoveCursor(flagship, 0, -1), bottom);
            assert_eq!(DMS_MoveCursor(bottom, 0, 1), flagship);
            assert_eq!(DMS_MoveCursor(flagship, 0, 1), col);
            assert_eq!(DMS_MoveCursor(flagship, -1, 0), flagship);
            assert_eq!(DMS_MoveCursor(flagship, 1, 0), flagship);
        }
    }

    #[test]
    fn cursor_wraps_horizontally_and_vertical_input_takes_priority() {
        for row in 0..HANGAR_ROWS {
            let left = row * HANGAR_SHIPS_ROW;
            let right = left + HANGAR_SHIPS_ROW - 1;
            assert_eq!(DMS_MoveCursor(left, -1, 0), right);
            assert_eq!(DMS_MoveCursor(right, 1, 0), left);
        }
        for slot in 0..12 {
            assert_eq!(DMS_MoveCursor(slot, 0, 0), slot);
            assert_eq!(DMS_MoveCursor(DMS_MoveCursor(slot, 0, 1), 0, -1), slot);
            assert_eq!(DMS_MoveCursor(slot, 1, -1), DMS_MoveCursor(slot, 0, -1));
        }
    }

    #[test]
    fn escort_rectangles_follow_hangar_slot_order() {
        for slot in 0..12 {
            let r = DMS_GetEscortShipRect(slot);
            assert_eq!(
                r.corner.x,
                HANGAR_X_COORDS[(slot % HANGAR_SHIPS_ROW) as usize]
            );
            assert_eq!(
                r.corner.y,
                HANGAR_Y + HANGAR_DY * (slot / HANGAR_SHIPS_ROW) as i16
            );
            assert_eq!(r.extent.width, 34);
            assert_eq!(r.extent.height, 40);
        }
    }
}
