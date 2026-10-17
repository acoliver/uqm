//! The battle ship picker of the full game (`pickship.c`).
//!
//! When the player has escorts, each battle opens a grid of the fleet's
//! ships. This module holds the picker's rules: how the cursor moves over the
//! grid (a flagship column in the middle, escorts on either side), which
//! escort a cell names, where its highlight goes and what its crew label
//! says. [`ffi`] drives the C drawing and input calls with those results and
//! exports the C entry points `GetEncounterStarShip` and `DrawArmadaPickShip`.

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod ffi;

/// `NUM_PICK_SHIP_ROWS`.
pub const ROWS: i16 = 2;
/// `NUM_PICK_SHIP_COLUMNS`: the grid has `COLUMNS + 1` cells per row, the
/// middle one being the flagship's.
pub const COLUMNS: i16 = 6;
pub const ICON_WIDTH: i16 = 16;
pub const ICON_HEIGHT: i16 = 16;
pub const FLAGSHIP_X_OFFS: i16 = 65;
pub const FLAGSHIP_Y_OFFS: i16 = 4;
pub const FLAGSHIP_WIDTH: i16 = 22;
pub const FLAGSHIP_HEIGHT: i16 = 48;
/// The grid column that holds the flagship (`NUM_PICK_SHIP_COLUMNS >> 1`).
pub const FLAGSHIP_COLUMN: i16 = COLUMNS >> 1;
/// `NO_ID`: the species id of a dead escort.
pub const NO_ID: u32 = 0;

/// The highlighted grid cell (`MENU_STATE.first_item`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub x: i16,
    pub y: i16,
}

impl Cursor {
    /// Where the picker opens: on the flagship.
    pub const START: Self = Self {
        x: FLAGSHIP_COLUMN,
        y: 0,
    };
}

/// The pulsed menu keys the picker reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Direction {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

/// The cursor after the pulsed keys, or `None` when it stays. Opposite keys
/// pressed together resolve to the later test of the C code: left beats
/// right, down beats up. Columns wrap through `0..=COLUMNS`, rows through
/// `0..ROWS`.
#[must_use]
pub fn moved(cursor: Cursor, keys: Direction) -> Option<Cursor> {
    let mut dx = 0;
    let mut dy = 0;
    if keys.right {
        dx = 1;
    }
    if keys.left {
        dx = -1;
    }
    if keys.up {
        dy = -1;
    }
    if keys.down {
        dy = 1;
    }

    let mut x = cursor.x + dx;
    let mut y = cursor.y + dy;
    if x == cursor.x && y == cursor.y {
        return None;
    }
    if x < 0 {
        x = COLUMNS;
    } else if x > COLUMNS {
        x = 0;
    }
    if y < 0 {
        y = ROWS - 1;
    } else if y == ROWS {
        y = 0;
    }
    Some(Cursor { x, y })
}

/// What a grid cell holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Flagship,
    /// The escort whose `STARSHIP.index` is this.
    Escort(u16),
}

#[must_use]
pub fn slot(cursor: Cursor) -> Slot {
    if cursor.x == FLAGSHIP_COLUMN {
        return Slot::Flagship;
    }
    let column = if cursor.x > FLAGSHIP_COLUMN {
        cursor.x - 1
    } else {
        cursor.x
    };
    Slot::Escort((cursor.y * COLUMNS + column) as u16)
}

/// A rectangle in the picker's context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
}

/// The highlight of a cell, for a picker frame whose corner is `frame`.
#[must_use]
pub fn highlight(frame: (i16, i16), cursor: Cursor) -> Area {
    if cursor.x == FLAGSHIP_COLUMN {
        return Area {
            x: frame.0 - 2 + FLAGSHIP_X_OFFS,
            y: frame.1 - 2 + FLAGSHIP_Y_OFFS,
            width: FLAGSHIP_WIDTH + 4,
            height: FLAGSHIP_HEIGHT + 4,
        };
    }
    let mut x = 5 + frame.0 - 2 + (ICON_WIDTH + 4) * cursor.x;
    if cursor.x > FLAGSHIP_COLUMN {
        x += FLAGSHIP_WIDTH - ICON_WIDTH;
    }
    Area {
        x,
        y: 16 + frame.1 - 2 + (ICON_HEIGHT + 4) * cursor.y,
        width: ICON_WIDTH + 4,
        height: ICON_HEIGHT + 4,
    }
}

/// The black strip behind the captain's name, for a picker frame whose
/// corner is `frame`.
#[must_use]
pub fn name_strip(frame: (i16, i16)) -> Area {
    Area {
        x: frame.0 + 6,
        y: frame.1 + 5,
        width: (ICON_WIDTH + 4) * 3 - 4,
        height: 7,
    }
}

/// The black strip behind the crew count: the name strip moved over the
/// right-hand icons.
#[must_use]
pub fn crew_strip(name: Area) -> Area {
    Area {
        x: name.x + (ICON_WIDTH + 4) * (FLAGSHIP_COLUMN + 1) + FLAGSHIP_WIDTH - ICON_WIDTH,
        ..name
    }
}

/// Where an escort's icon sits in the picker frame at `frame`.
#[must_use]
pub fn icon_origin(frame: (i16, i16), ship_index: u16) -> (i16, i16) {
    let column = (ship_index % COLUMNS as u16) as i16;
    let row = (ship_index / COLUMNS as u16) as i16;
    let mut x = frame.0 + 5 + (ICON_WIDTH + 4) * column;
    if column >= FLAGSHIP_COLUMN {
        x += FLAGSHIP_WIDTH + 4;
    }
    (x, frame.1 + 16 + (ICON_HEIGHT + 4) * row)
}

/// The fields of an escort `STARSHIP` the lookup reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Escort {
    pub index: u16,
    /// `SpeciesID != NO_ID`.
    pub alive: bool,
}

/// The position of the live escort with this `index`; dead escorts cannot be
/// picked.
#[must_use]
pub fn find_escort(escorts: &[Escort], ship_index: u16) -> Option<usize> {
    escorts
        .iter()
        .position(|escort| escort.index == ship_index && escort.alive)
}

/// The crew text under the picker: nothing for no crew, the count when the
/// ship is full, `crew/max` otherwise.
#[must_use]
pub fn crew_label(crew: u16, max_crew: u16) -> Option<String> {
    if crew == 0 {
        None
    } else if crew >= max_crew {
        Some(format!("{crew}"))
    } else {
        Some(format!("{crew}/{max_crew}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(up: bool, down: bool, left: bool, right: bool) -> Direction {
        Direction {
            up,
            down,
            left,
            right,
        }
    }

    #[test]
    fn opens_on_the_flagship_column() {
        assert_eq!(slot(Cursor::START), Slot::Flagship);
    }

    #[test]
    fn no_key_leaves_the_cursor() {
        assert_eq!(moved(Cursor::START, Direction::default()), None);
    }

    #[test]
    fn horizontal_moves_wrap_through_the_flagship_column() {
        let at = |x| Cursor { x, y: 0 };
        assert_eq!(moved(at(3), keys(false, false, false, true)), Some(at(4)));
        assert_eq!(moved(at(6), keys(false, false, false, true)), Some(at(0)));
        assert_eq!(moved(at(0), keys(false, false, true, false)), Some(at(6)));
        assert_eq!(moved(at(3), keys(false, false, true, false)), Some(at(2)));
    }

    #[test]
    fn vertical_moves_wrap_over_two_rows() {
        let at = |y| Cursor { x: 1, y };
        assert_eq!(moved(at(0), keys(false, true, false, false)), Some(at(1)));
        assert_eq!(moved(at(1), keys(false, true, false, false)), Some(at(0)));
        assert_eq!(moved(at(0), keys(true, false, false, false)), Some(at(1)));
        assert_eq!(moved(at(1), keys(true, false, false, false)), Some(at(0)));
    }

    #[test]
    fn opposite_keys_resolve_to_left_and_down() {
        let start = Cursor { x: 2, y: 0 };
        assert_eq!(
            moved(start, keys(true, true, true, true)),
            Some(Cursor { x: 1, y: 1 })
        );
    }

    #[test]
    fn cells_right_of_the_flagship_skip_its_column() {
        assert_eq!(slot(Cursor { x: 0, y: 0 }), Slot::Escort(0));
        assert_eq!(slot(Cursor { x: 2, y: 0 }), Slot::Escort(2));
        assert_eq!(slot(Cursor { x: 4, y: 0 }), Slot::Escort(3));
        assert_eq!(slot(Cursor { x: 6, y: 0 }), Slot::Escort(5));
        assert_eq!(slot(Cursor { x: 0, y: 1 }), Slot::Escort(6));
        assert_eq!(slot(Cursor { x: 6, y: 1 }), Slot::Escort(11));
    }

    #[test]
    fn highlight_of_the_flagship_covers_its_tall_cell() {
        assert_eq!(
            highlight((100, 50), Cursor::START),
            Area {
                x: 163,
                y: 52,
                width: 26,
                height: 52
            }
        );
    }

    #[test]
    fn highlight_of_an_escort_matches_the_icon_it_surrounds() {
        for (cursor, index) in [
            (Cursor { x: 0, y: 0 }, 0),
            (Cursor { x: 2, y: 1 }, 8),
            (Cursor { x: 4, y: 0 }, 3),
            (Cursor { x: 6, y: 1 }, 11),
        ] {
            let area = highlight((100, 50), cursor);
            let (x, y) = icon_origin((100, 50), index);
            assert_eq!((area.x, area.y), (x - 2, y - 2), "{cursor:?}");
            assert_eq!((area.width, area.height), (20, 20));
        }
    }

    #[test]
    fn icon_origin_leaves_room_for_the_flagship() {
        assert_eq!(icon_origin((0, 0), 0), (5, 16));
        assert_eq!(icon_origin((0, 0), 2), (45, 16));
        assert_eq!(icon_origin((0, 0), 3), (5 + 60 + 26, 16));
        assert_eq!(icon_origin((0, 0), 11), (5 + 100 + 26, 36));
    }

    #[test]
    fn strips_follow_the_frame() {
        let name = name_strip((10, 20));
        assert_eq!(
            name,
            Area {
                x: 16,
                y: 25,
                width: 56,
                height: 7
            }
        );
        assert_eq!(crew_strip(name).x, 16 + 80 + 6);
        assert_eq!(crew_strip(name).y, 25);
    }

    #[test]
    fn dead_escorts_cannot_be_picked() {
        let escorts = [
            Escort {
                index: 4,
                alive: false,
            },
            Escort {
                index: 4,
                alive: true,
            },
            Escort {
                index: 5,
                alive: true,
            },
        ];
        assert_eq!(find_escort(&escorts, 4), Some(1));
        assert_eq!(find_escort(&escorts, 5), Some(2));
        assert_eq!(find_escort(&escorts, 6), None);
        assert_eq!(find_escort(&escorts[..1], 4), None);
    }

    #[test]
    fn crew_label_shows_a_full_ship_as_one_number() {
        assert_eq!(crew_label(0, 12), None);
        assert_eq!(crew_label(12, 12).as_deref(), Some("12"));
        assert_eq!(crew_label(14, 12).as_deref(), Some("14"));
        assert_eq!(crew_label(5, 12).as_deref(), Some("5/12"));
    }
}
