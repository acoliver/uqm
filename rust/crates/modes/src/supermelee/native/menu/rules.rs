//! Cursor and control transitions of the Super Melee setup menu.
use crate::battle::{COMPUTER_CONTROL, HUMAN_CONTROL};

pub(in crate::supermelee::native) fn slot_bounds(
    side: u16,
    row: u16,
    col: u16,
) -> (i16, i16, i16, i16) {
    (
        2 + col as i16 * 35,
        21 + side as i16 * 92 + row as i16 * 35,
        34,
        34,
    )
}

pub(in crate::supermelee::native) fn popup_hotspot(
    side: u16,
    width: i16,
    height: i16,
) -> (i16, i16) {
    let (x, y, _, _) = slot_bounds(0, 0, 0);
    let (last_x, last_y, last_width, last_height) = slot_bounds(1, 1, 6);
    let ships_width = last_x + last_width - x;
    let ships_height = last_y + last_height - y;
    (
        -(x + ((ships_width - width) >> 1)),
        if side != 0 {
            -y
        } else {
            -(y + ships_height - height)
        },
    )
}

pub(super) fn name_editor_buffer(name: &std::ffi::CStr) -> [u8; 31] {
    let mut buffer = [0; 31];
    let name = name.to_bytes();
    let length = name.len().min(buffer.len() - 1);
    buffer[..length].copy_from_slice(&name[..length]);
    buffer
}

pub(super) fn advance_cursor(row: u16, col: u16) -> (u16, u16) {
    let col = col + 1;
    if col != 7 {
        (row, col)
    } else if row + 1 < 2 {
        (row + 1, 0)
    } else {
        (1, 6)
    }
}

pub(super) fn move_cursor(
    (mut side, mut row, mut col): (u16, u16, u16),
    left: bool,
    right: bool,
    up: bool,
    down: bool,
) -> (u16, u16, u16) {
    if left {
        col = col.saturating_sub(1);
    } else if right && col < 6 {
        col += 1;
    }
    if up {
        if row > 0 {
            row -= 1;
        } else if side != 0 {
            row = 2;
            side = 0;
        }
    } else if down {
        row += 1;
        if row == 3 {
            if side == 1 {
                row = 2;
            } else {
                row = 0;
                side = 1;
            }
        }
    }
    (side, row, col)
}

pub(super) fn next_option(option: u8, up: bool, down: bool) -> u8 {
    if up {
        option.saturating_sub(1)
    } else if down && option != 9 {
        option + 1
    } else {
        option
    }
}

pub(super) fn next_control(control: u8) -> u8 {
    let human = HUMAN_CONTROL | 16;
    let computer = COMPUTER_CONTROL;
    if control == human {
        computer | 16
    } else if control == computer | 16 {
        computer | 32
    } else if control == computer | 32 {
        computer | 64
    } else {
        human
    }
}

pub(super) fn network_status_side(side: usize, control: u8) -> isize {
    if control & crate::battle::NETWORK_CONTROL != 0 {
        side as isize
    } else {
        -1
    }
}

#[cfg(test)]
mod layout_tests {
    #[test]
    fn ship_slots_and_popup_keep_the_legacy_union_and_alignment() {
        assert_eq!(super::slot_bounds(0, 0, 0), (2, 21, 34, 34));
        assert_eq!(super::slot_bounds(1, 1, 6), (212, 148, 34, 34));
        assert_eq!(super::popup_hotspot(0, 104, 98), (-72, -84));
        assert_eq!(super::popup_hotspot(1, 104, 98), (-72, -21));
        assert_eq!(super::popup_hotspot(0, 101, 97), (-73, -85));
        assert_eq!(super::popup_hotspot(1, 101, 97), (-73, -21));
    }
}
