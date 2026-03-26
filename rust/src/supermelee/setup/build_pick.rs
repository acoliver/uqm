// SuperMelee Ship Picker — fleet-edit selection logic
// @plan PLAN-20260314-SUPERMELEE.P07
// @requirement fleet-edit ship pick

use crate::supermelee::types::{MeleeShip, NUM_MELEE_SHIPS, NUM_PICK_COLS, NUM_PICK_ROWS};

// ---------------------------------------------------------------------------
// Picker state
// ---------------------------------------------------------------------------

/// Tracks the state of the 5×5 ship picker grid.
///
/// The picker grid is `NUM_PICK_ROWS × NUM_PICK_COLS` (5×5 = 25 cells).
/// Each cell maps to a `MeleeShip` by grid index (row * cols + col).
/// Index 0..24 → the 25 MeleeShip races. Navigation wraps.
#[derive(Debug, Clone)]
pub struct ShipPicker {
    /// Current highlighted row (0..NUM_PICK_ROWS-1).
    pub row: usize,
    /// Current highlighted column (0..NUM_PICK_COLS-1).
    pub col: usize,
}

impl Default for ShipPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl ShipPicker {
    pub fn new() -> Self {
        Self { row: 0, col: 0 }
    }

    /// Returns the grid index (0..24) of the currently highlighted cell.
    pub fn index(&self) -> usize {
        self.row * NUM_PICK_COLS + self.col
    }

    /// Returns the `MeleeShip` at the current grid position, if valid.
    pub fn selected_ship(&self) -> Option<MeleeShip> {
        let idx = self.index();
        if idx < NUM_MELEE_SHIPS {
            MeleeShip::from_u8(idx as u8)
        } else {
            None
        }
    }

    /// Move up (wraps around).
    pub fn move_up(&mut self) {
        self.row = if self.row == 0 {
            NUM_PICK_ROWS - 1
        } else {
            self.row - 1
        };
    }

    /// Move down (wraps around).
    pub fn move_down(&mut self) {
        self.row = (self.row + 1) % NUM_PICK_ROWS;
    }

    /// Move left (wraps around).
    pub fn move_left(&mut self) {
        self.col = if self.col == 0 {
            NUM_PICK_COLS - 1
        } else {
            self.col - 1
        };
    }

    /// Move right (wraps around).
    pub fn move_right(&mut self) {
        self.col = (self.col + 1) % NUM_PICK_COLS;
    }
}

// ---------------------------------------------------------------------------
// Picker result
// ---------------------------------------------------------------------------

/// Outcome of a ship-picker interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickResult {
    /// User confirmed selection of a ship.
    Confirmed(MeleeShip),
    /// User cancelled — no change to team.
    Cancelled,
}

/// Resolves a picker confirmation into a ship assignment.
///
/// - If `result` is `Confirmed`, returns the chosen ship.
/// - If `result` is `Cancelled`, returns `None`.
pub fn resolve_pick(result: &PickResult) -> Option<MeleeShip> {
    match result {
        PickResult::Confirmed(ship) => Some(*ship),
        PickResult::Cancelled => None,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_default_position_is_top_left() {
        unsafe {
            let picker = ShipPicker::new();
            assert_eq!(picker.row, 0);
            assert_eq!(picker.col, 0);
            assert_eq!(picker.index(), 0);
        }
    }

    #[test]
    fn picker_navigation_changes_highlighted_ship() {
        unsafe {
            let mut picker = ShipPicker::new();
            // Move right → col 1
            picker.move_right();
            assert_eq!(picker.col, 1);
            assert_eq!(picker.index(), 1);

            // Move down → row 1, col 1
            picker.move_down();
            assert_eq!(picker.index(), NUM_PICK_COLS + 1);

            // Verify it maps to a real ship
            assert!(picker.selected_ship().is_some());
        }
    }

    #[test]
    fn picker_navigation_wraps_up() {
        unsafe {
            let mut picker = ShipPicker::new();
            picker.move_up(); // wraps to last row
            assert_eq!(picker.row, NUM_PICK_ROWS - 1);
        }
    }

    #[test]
    fn picker_navigation_wraps_down() {
        unsafe {
            let mut picker = ShipPicker::new();
            for _ in 0..NUM_PICK_ROWS {
                picker.move_down();
            }
            assert_eq!(picker.row, 0); // wrapped back
        }
    }

    #[test]
    fn picker_navigation_wraps_left() {
        unsafe {
            let mut picker = ShipPicker::new();
            picker.move_left(); // wraps to last col
            assert_eq!(picker.col, NUM_PICK_COLS - 1);
        }
    }

    #[test]
    fn picker_navigation_wraps_right() {
        unsafe {
            let mut picker = ShipPicker::new();
            for _ in 0..NUM_PICK_COLS {
                picker.move_right();
            }
            assert_eq!(picker.col, 0); // wrapped back
        }
    }

    #[test]
    fn picker_selected_ship_for_valid_positions() {
        unsafe {
            let mut picker = ShipPicker::new();
            // Position 0 = Androsynth
            assert_eq!(picker.selected_ship(), Some(MeleeShip::Androsynth));
            // Last valid position (24) = ZoqFotPik
            picker.row = 4;
            picker.col = 4;
            assert_eq!(picker.selected_ship(), Some(MeleeShip::ZoqFotPik));
        }
    }

    #[test]
    fn picker_grid_covers_all_25_ships() {
        unsafe {
            let mut seen = std::collections::HashSet::new();
            for r in 0..NUM_PICK_ROWS {
                for c in 0..NUM_PICK_COLS {
                    let picker = ShipPicker { row: r, col: c };
                    if let Some(ship) = picker.selected_ship() {
                        seen.insert(ship as u8);
                    }
                }
            }
            assert_eq!(seen.len(), NUM_MELEE_SHIPS);
        }
    }

    #[test]
    fn picker_confirm_applies_selection() {
        unsafe {
            let result = PickResult::Confirmed(MeleeShip::Chmmr);
            assert_eq!(resolve_pick(&result), Some(MeleeShip::Chmmr));
        }
    }

    #[test]
    fn picker_cancel_leaves_no_selection() {
        unsafe {
            let result = PickResult::Cancelled;
            assert_eq!(resolve_pick(&result), None);
        }
    }
}
