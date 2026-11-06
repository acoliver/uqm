//! Fleet-slot and team-name editing.
use super::*;
unsafe fn changed(entry: *mut TEXTENTRY_STATE, _input: &crate::engine::input::InputSession) -> u32 {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        let state = (*entry).CbParam.cast::<MELEE_STATE>();
        (*state).cur_index = (*entry).CursorPos as u16;
        name(
            state,
            (*state).side,
            1 | if (*entry).JoystickMode != 0 { 8 } else { 0 },
            (*entry).BaseStr,
        )
    }
}
unsafe fn frame(entry: *mut TEXTENTRY_STATE, _input: &crate::engine::input::InputSession) -> u32 {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        let state = (*entry).CbParam.cast::<MELEE_STATE>();
        poll_menu(state, 0);
        1
    }
}
unsafe fn advance(state: *mut MELEE_STATE) {
    ((*state).row, (*state).col) = super::super::rules::advance_cursor((*state).row, (*state).col);
}
unsafe fn edit_name(state: *mut MELEE_STATE, side: u16, input: &crate::engine::input::InputSession) {
    (*state).cur_index = 0;
    name(state, side, 1, ptr::null());
    let mut buf = [0 as std::ffi::c_char; 31];
    libc::strncpy(
        buf.as_mut_ptr(),
        (*state).melee_setup.teams[side as usize].c_name().as_ptr(),
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
    do_text_entry(input, &mut entry);
    (*state).cur_index = u16::MAX;
    if entry.Success == 0
        || !local_change_team_name(state, side, std::ffi::CStr::from_ptr(buf.as_ptr()))
    {
        view_name(state, side);
    }
}
pub(super) unsafe fn edit(
    state: *mut MELEE_STATE,
    input: &crate::engine::input::InputSession,
) -> u32 {
    let time = GetTimeCounter();
    input.set_paused(false);
    if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16 != 0 {
        return 0;
    }
    SetMenuSounds(input, 15, 16 | 512);
    if (*state).initialized == 0 {
        current(state);
        (*state).initialized = 1;
        (*state).phase = crate::supermelee::netplay::session::MenuPhase::Editing;
        return 1;
    }
    poll_menu(state, 0);
    let row = (*state).row;
    let col = (*state).col;
    if (row < 2 || (*state).current_ship == 255)
        && (key(input, KEY_MENU_CANCEL) || key(input, KEY_MENU_RIGHT) && (col == 6 || row == 2))
    {
        highlight(10, false);
        (*state).current_ship = 255;
        (*state).melee_option = 4;
        (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
        (*state).last_input_time = GetTimeCounter();
    } else if row < 2 && key(input, KEY_MENU_SELECT) {
        highlight(10, false);
        (*state).melee_option = 11;
        let ok = crate::supermelee::native::build_picker::live::build_pick_ship(input, state);
        if ok {
            local_change_ship(
                state,
                (*state).side,
                (*state).row * 7 + (*state).col,
                (*state).current_ship,
            );
            advance(state);
        }
        (*state).melee_option = 10;
        let mut r = mem::zeroed();
        crate::supermelee::native::build_picker::live::get_build_pick_frame_rect(&mut r);
        RepairMeleeFrame(&r);
        current(state);
        (*state).phase = crate::supermelee::netplay::session::MenuPhase::Editing;
    } else if row < 2 && key(input, KEY_MENU_SPECIAL) {
        highlight(10, false);
    } else if row < 2 && key(input, KEY_MENU_DELETE) {
        highlight(10, false);
        local_change_ship(state, (*state).side, row * 7 + col, 255);
        advance(state);
        current(state);
    } else {
        let (mut side, mut row, mut col) = ((*state).side, row, col);
        if row == 2 && key(input, KEY_MENU_SELECT) {
            edit_name(state, side, input);
            return 1;
        }
        (side, row, col) = super::super::rules::move_cursor(
            (side, row, col),
            key(input, KEY_MENU_LEFT),
            key(input, KEY_MENU_RIGHT),
            key(input, KEY_MENU_UP),
            key(input, KEY_MENU_DOWN),
        );
        if (side, row, col) != ((*state).side, (*state).row, (*state).col) {
            highlight(10, false);
            (*state).side = side;
            (*state).row = row;
            (*state).col = col;
            current(state);
        }
    }
    (*state).netplay.flush_packet_queues();
    Melee_flashSelection(state);
    SleepThreadUntil(time.wrapping_add(840 / 30));
    1
}
