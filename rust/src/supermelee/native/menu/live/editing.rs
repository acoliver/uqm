//! Fleet-slot and team-name editing.
use super::*;
unsafe fn changed(
    entry: *mut TEXTENTRY_STATE,
    cx: &mut crate::engine::battle::Cx<'_>,
    _input: &crate::engine::input::InputSession,
) -> u32 {
    cx.with_legacy(|| {
        // SAFETY: the function contract requires live, game-thread-owned records
        // and callback arguments for all legacy accesses in this operation.
        unsafe {
            let state = (*entry).CbParam.cast::<MeleeSession>();
            (*state).cur_index = (*entry).CursorPos as u16;
            name(
                &*state,
                (*state).side,
                1 | if (*entry).JoystickMode != 0 { 8 } else { 0 },
                (*entry).BaseStr,
            )
        }
    })
}
unsafe fn frame(
    entry: *mut TEXTENTRY_STATE,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    _input: &crate::engine::input::InputSession,
) -> u32 {
    let state = (*entry).CbParam.cast::<MeleeSession>();
    poll_menu(state, cx, io, 0);
    1
}
unsafe fn advance(state: *mut MeleeSession) {
    ((*state).row, (*state).col) = super::super::rules::advance_cursor((*state).row, (*state).col);
}
unsafe fn edit_name(
    state: *mut MeleeSession,
    side: u16,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    input: &crate::engine::input::InputSession,
) {
    (*state).cur_index = 0;
    cx.with_legacy(|| name(&*state, side, 1, ptr::null()));
    let mut buf =
        super::super::rules::name_editor_buffer((*state).melee_setup.teams[side as usize].c_name());
    let mut entry: TEXTENTRY_STATE = mem::zeroed();
    entry.Initialized = 0;
    entry.BaseStr = buf.as_mut_ptr().cast();
    entry.CursorPos = 0;
    entry.MaxSize = 31;
    entry.CbParam = state.cast();
    entry.ChangeCallback = Some(changed);
    entry.FrameCallback = Some(frame);
    do_text_entry(cx, io, input, &mut entry);
    (*state).cur_index = u16::MAX;
    if entry.Success == 0
        || !cx.with_legacy(|| {
            local_change_team_name(state, side, std::ffi::CStr::from_ptr(buf.as_ptr().cast()))
        })
    {
        cx.with_legacy(|| view_name(state, side));
    }
}
pub(super) unsafe fn edit(
    state: *mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    input: &crate::engine::input::InputSession,
) -> u32 {
    let time = GetTimeCounter();
    input.set_paused(false);
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        return 0;
    }
    SetMenuSounds(input, 15, 16 | 512);
    if (*state).initialized == 0 {
        cx.with_legacy(|| current(&mut *state));
        (*state).initialized = 1;
        (*state).phase = crate::supermelee::session::MenuPhase::Editing;
        return 1;
    }
    poll_menu(state, cx, io, 0);
    let row = (*state).row;
    let col = (*state).col;
    if (row < 2 || (*state).current_ship == 255)
        && (key(input, KEY_MENU_CANCEL) || key(input, KEY_MENU_RIGHT) && (col == 6 || row == 2))
    {
        cx.with_legacy(|| highlight(&*state, 10, false));
        (*state).current_ship = 255;
        (*state).melee_option = 4;
        (*state).phase = crate::supermelee::session::MenuPhase::Menu;
        (*state).last_input_time = GetTimeCounter();
    } else if row < 2 && key(input, KEY_MENU_SELECT) {
        cx.with_legacy(|| highlight(&*state, 10, false));
        (*state).melee_option = 11;
        let ok =
            crate::supermelee::native::build_picker::live::build_pick_ship(cx, io, input, state);
        if ok {
            cx.with_legacy(|| {
                local_change_ship(
                    state,
                    (*state).side,
                    (*state).row * 7 + (*state).col,
                    (*state).current_ship,
                )
            });
            advance(state);
        }
        (*state).melee_option = 10;
        let r = (*state).build_picker().rect();
        cx.with_legacy(|| repair_frame(&*state, &r));
        cx.with_legacy(|| current(&mut *state));
        (*state).phase = crate::supermelee::session::MenuPhase::Editing;
    } else if row < 2 && key(input, KEY_MENU_SPECIAL) {
        cx.with_legacy(|| highlight(&*state, 10, false));
    } else if row < 2 && key(input, KEY_MENU_DELETE) {
        cx.with_legacy(|| highlight(&*state, 10, false));
        cx.with_legacy(|| local_change_ship(state, (*state).side, row * 7 + col, 255));
        advance(state);
        cx.with_legacy(|| current(&mut *state));
    } else {
        let (mut side, mut row, mut col) = ((*state).side, row, col);
        if row == 2 && key(input, KEY_MENU_SELECT) {
            edit_name(state, side, cx, io, input);
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
            cx.with_legacy(|| highlight(&*state, 10, false));
            (*state).side = side;
            (*state).row = row;
            (*state).col = col;
            cx.with_legacy(|| current(&mut *state));
        }
    }
    flush_menu(state, cx, io);
    cx.with_legacy(|| {
        flash_selection(&mut *state);
        SleepThreadUntil(time.wrapping_add(840 / 30));
    });
    1
}
