//! Fleet-slot and team-name editing.
use super::*;
unsafe extern "C" fn changed(entry: *mut TEXTENTRY_STATE) -> u32 {
    let state = (*entry).CbParam.cast::<MELEE_STATE>();
    (*state).CurIndex = (*entry).CursorPos as u16;
    name(
        state,
        (*state).side,
        1 | if (*entry).JoystickMode != 0 { 8 } else { 0 },
        (*entry).BaseStr,
    )
}
unsafe extern "C" fn frame(_entry: *mut TEXTENTRY_STATE) -> u32 {
    netInput();
    1
}
unsafe fn advance(state: *mut MELEE_STATE) {
    ((*state).row, (*state).col) = super::super::rules::advance_cursor((*state).row, (*state).col);
}
pub(super) unsafe extern "C" fn edit(state: *mut MELEE_STATE) -> u32 {
    let time = GetTimeCounter();
    GamePaused = 0;
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        return 0;
    }
    SetMenuSounds(15, 16 | 512);
    if (*state).Initialized == 0 {
        current(state);
        (*state).Initialized = 1;
        (*state).InputFunc = Some(edit);
        return 1;
    }
    netInput();
    let row = (*state).row;
    let col = (*state).col;
    if (row < 2 || (*state).currentShip == 255)
        && (key(KEY_MENU_CANCEL) || key(KEY_MENU_RIGHT) && (col == 6 || row == 2))
    {
        highlight(10, false);
        (*state).currentShip = 255;
        (*state).MeleeOption = 4;
        (*state).InputFunc = Some(DoMelee);
        (*state).LastInputTime = GetTimeCounter();
    } else if row < 2 && key(KEY_MENU_SELECT) {
        highlight(10, false);
        (*state).MeleeOption = 11;
        let ok = crate::supermelee::native::build_picker::live::BuildPickShip(state);
        if ok {
            Melee_LocalChange_ship(
                state,
                (*state).side,
                (*state).row * 7 + (*state).col,
                (*state).currentShip,
            );
            advance(state);
        }
        (*state).MeleeOption = 10;
        let mut r = mem::zeroed();
        crate::supermelee::native::build_picker::live::GetBuildPickFrameRect(&mut r);
        RepairMeleeFrame(&r);
        current(state);
        (*state).InputFunc = Some(edit);
    } else if row < 2 && key(KEY_MENU_SPECIAL) {
        highlight(10, false);
    } else if row < 2 && key(KEY_MENU_DELETE) {
        highlight(10, false);
        Melee_LocalChange_ship(state, (*state).side, row * 7 + col, 255);
        advance(state);
        current(state);
    } else {
        let (mut side, mut row, mut col) = ((*state).side, row, col);
        if row == 2 && key(KEY_MENU_SELECT) {
            (*state).CurIndex = 0;
            name(state, side, 1, ptr::null());
            let mut buf = [0i8; 31];
            libc::strncpy(
                buf.as_mut_ptr(),
                MeleeSetup_getTeamName((*state).meleeSetup, side as usize),
                30,
            );
            let mut entry: TEXTENTRY_STATE = mem::zeroed();
            entry.Initialized = 0;
            entry.BaseStr = buf.as_mut_ptr();
            entry.CursorPos = 0;
            entry.MaxSize = 31;
            entry.CbParam = state.cast();
            entry.ChangeCallback = Some(changed);
            entry.FrameCallback = Some(frame);
            DoTextEntry(&mut entry);
            (*state).CurIndex = u16::MAX;
            if entry.Success == 0 || !Melee_LocalChange_teamName(state, side, buf.as_ptr()) {
                view_name(state, side);
            }
            return 1;
        }
        (side, row, col) = super::super::rules::move_cursor(
            (side, row, col),
            key(KEY_MENU_LEFT),
            key(KEY_MENU_RIGHT),
            key(KEY_MENU_UP),
            key(KEY_MENU_DOWN),
        );
        if (side, row, col) != ((*state).side, (*state).row, (*state).col) {
            highlight(10, false);
            (*state).side = side;
            (*state).row = row;
            (*state).col = col;
            current(state);
        }
    }
    flushPacketQueues();
    Melee_flashSelection(state);
    SleepThreadUntil(time.wrapping_add(840 / 30));
    1
}
