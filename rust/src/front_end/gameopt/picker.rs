//! Save-picker movement and exit decisions, independent of native I/O.

pub(super) const MAX_SAVED_GAMES: usize = 50;
pub(super) const SAVES_PER_PAGE: u16 = 5;

pub(super) struct PickerState {
    pub summary: [crate::engine::save::summary_record::SummaryDesc; MAX_SAVED_GAMES],
    pub saving: bool,
    pub success: bool,
    pub slot: u8,
}

impl PickerState {
    pub fn new(
        summary: [crate::engine::save::summary_record::SummaryDesc; MAX_SAVED_GAMES],
        saving: bool,
        slot: u8,
    ) -> Self {
        assert!(
            usize::from(slot) < MAX_SAVED_GAMES,
            "invalid remembered save slot"
        );
        Self {
            summary,
            saving,
            success: false,
            slot,
        }
    }

    pub fn action(&self, abort: bool, cancel: bool, select: bool) -> PickerAction {
        picker_action(
            abort,
            cancel,
            select,
            self.saving,
            self.summary[usize::from(self.slot)].year_index != 0,
        )
    }

    pub fn navigate(&mut self, left: bool, right: bool, up: bool, down: bool) -> bool {
        let next = picker_slot(self.slot, left, right, up, down);
        let changed = next != self.slot;
        self.slot = next;
        changed
    }
}

pub(super) fn picker_slot(current: u8, left: bool, right: bool, up: bool, down: bool) -> u8 {
    let mut next = current;
    if left {
        if next == 0 {
            next = (MAX_SAVED_GAMES - 1) as u8;
        } else if i32::from(next) - i32::from(SAVES_PER_PAGE) > 0 {
            next -= SAVES_PER_PAGE as u8;
        } else {
            next = 0;
        }
    } else if right {
        if next == (MAX_SAVED_GAMES - 1) as u8 {
            next = 0;
        } else if i32::from(next) + i32::from(SAVES_PER_PAGE) < (MAX_SAVED_GAMES - 1) as i32 {
            next += SAVES_PER_PAGE as u8;
        } else {
            next = (MAX_SAVED_GAMES - 1) as u8;
        }
    } else if up {
        if next == 0 {
            next = (MAX_SAVED_GAMES - 1) as u8;
        } else {
            next -= 1;
        }
    } else if down {
        if next == (MAX_SAVED_GAMES - 1) as u8 {
            next = 0;
        } else {
            next += 1;
        }
    }
    next
}

/// The original DoPickGame exit priority, independent of presentation and I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerAction {
    Continue,
    Cancel,
    Abort,
    Select,
    EmptySlot,
}

pub fn picker_action(
    abort: bool,
    cancel: bool,
    select: bool,
    saving: bool,
    occupied: bool,
) -> PickerAction {
    if abort {
        PickerAction::Abort
    } else if cancel {
        PickerAction::Cancel
    } else if select {
        if saving || occupied {
            PickerAction::Select
        } else {
            PickerAction::EmptySlot
        }
    } else {
        PickerAction::Continue
    }
}
