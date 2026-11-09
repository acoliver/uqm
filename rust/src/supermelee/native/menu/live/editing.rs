//! Fleet-slot and team-name editing.
use super::*;
struct NameCallbacks<'a>(&'a mut MeleeSession);

impl crate::ui::getchar::TextEntryCallbacks for NameCallbacks<'_> {
    fn changed(
        &mut self,
        entry: crate::ui::getchar::TextEntryView<'_>,
        cx: &mut crate::engine::battle::Cx<'_>,
        _input: &crate::engine::input::InputSession,
    ) -> bool {
        self.0.cur_index =
            u16::try_from(entry.cursor).unwrap_or_else(|error| panic!("team name cursor: {error}"));
        // SAFETY: the menu owns its retained drawing resources and entry text is borrowed.
        {
            name(
                cx,
                self.0,
                self.0.side,
                1 | if entry.joystick { 8 } else { 0 },
                Some(entry.text),
            ) != 0
        }
    }

    fn frame(
        &mut self,
        _entry: crate::ui::getchar::TextEntryView<'_>,
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        _input: &crate::engine::input::InputSession,
    ) -> bool {
        poll_menu(self.0, cx, io, 0);
        true
    }
}
fn advance(state: &mut MeleeSession) {
    (state.row, state.col) = super::super::rules::advance_cursor(state.row, state.col);
}
fn edit_name(
    state: &mut MeleeSession,
    side: u16,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    input: &crate::engine::input::InputSession,
) {
    state.cur_index = 0;
    name(cx, state, side, 1, None);
    let mut buf =
        super::super::rules::name_editor_buffer(state.melee_setup.teams[side as usize].c_name());
    let success = crate::ui::getchar::edit_text(
        cx,
        io,
        input,
        buf.len(),
        &mut buf,
        0,
        &mut NameCallbacks(state),
    );
    state.cur_index = u16::MAX;
    if !success
        || !{
            let name = std::ffi::CStr::from_bytes_until_nul(&buf)
                .unwrap_or_else(|error| panic!("team name terminator: {error}"));
            local_change_team_name(cx, state, side, name)
        }
    {
        view_name(cx, state, side);
    }
}
pub(super) fn edit(
    state: &mut MeleeSession,
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
    if state.initialized == 0 {
        current(cx, state);
        state.initialized = 1;
        state.phase = crate::supermelee::session::MenuPhase::Editing;
        return 1;
    }
    poll_menu(state, cx, io, 0);
    let row = state.row;
    let col = state.col;
    if (row < 2 || state.current_ship == 255)
        && (key(input, KEY_MENU_CANCEL) || key(input, KEY_MENU_RIGHT) && (col == 6 || row == 2))
    {
        highlight(cx, state, 10, false);
        state.current_ship = 255;
        state.melee_option = 4;
        state.phase = crate::supermelee::session::MenuPhase::Menu;
        state.last_input_time = GetTimeCounter();
    } else if row < 2 && key(input, KEY_MENU_SELECT) {
        highlight(cx, state, 10, false);
        state.melee_option = 11;
        let ok =
            crate::supermelee::native::build_picker::live::build_pick_ship(cx, io, input, state);
        if ok {
            local_change_ship(
                cx,
                state,
                state.side,
                state.row * 7 + state.col,
                state.current_ship,
            );
            advance(state);
        }
        state.melee_option = 10;
        let r = state.build_picker().rect();
        repair_frame(cx, state, &r);
        current(cx, state);
        state.phase = crate::supermelee::session::MenuPhase::Editing;
    } else if row < 2 && key(input, KEY_MENU_SPECIAL) {
        highlight(cx, state, 10, false);
    } else if row < 2 && key(input, KEY_MENU_DELETE) {
        highlight(cx, state, 10, false);
        local_change_ship(cx, state, state.side, row * 7 + col, 255);
        advance(state);
        current(cx, state);
    } else {
        let (mut side, mut row, mut col) = (state.side, row, col);
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
        if (side, row, col) != (state.side, state.row, state.col) {
            highlight(cx, state, 10, false);
            state.side = side;
            state.row = row;
            state.col = col;
            current(cx, state);
        }
    }
    flush_menu(state, cx, io);
    flash_selection(cx, state);
    cx.kernel
        .input_runtime()
        .sleep_until(cx, time.wrapping_add(840 / 30));
    1
}
