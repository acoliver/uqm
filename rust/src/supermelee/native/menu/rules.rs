//! Cursor and control transitions of the Super Melee setup menu.
use crate::battle::{COMPUTER_CONTROL, HUMAN_CONTROL};

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
