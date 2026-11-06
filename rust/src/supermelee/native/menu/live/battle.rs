//! Battle launch and network confirmation.
use super::*;
pub(super) fn ready(state: &MeleeSession, controls: [u8; 2]) -> isize {
    let mut done = 0;
    for player in [0usize, 1] {
        let conn = connection(state, player);
        if conn.is_none() && controls[player] & NETWORK_CONTROL == 0 {
            done += 1;
            continue;
        }
        if conn.is_none() || !net_connection_is_connected(conn) {
            return -1;
        }
        if conn.unwrap_or_else(|| panic!("connection")).state > 3 {
            done += 1;
        }
    }
    done
}
unsafe fn prepare_confirmation(
    input: &crate::engine::input::InputSession,
    state: &mut MeleeSession,
    controls: [u8; 2],
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) -> Option<u32> {
    let session = state.netplay.clone();
    input.set_paused(false);
    if key(input, KEY_MENU_CANCEL) {
        state.phase = crate::supermelee::session::MenuPhase::Menu;
        state.netplay.cancel_confirmations();
        status_game(cx, state, (NETMELEE_STRING_BASE as i32 + 4) as usize);
        return Some(1);
    }
    if key(input, KEY_MENU_LEFT) || key(input, KEY_MENU_UP) || key(input, KEY_MENU_DOWN) {
        state.phase = crate::supermelee::session::MenuPhase::Menu;
        state.netplay.cancel_confirmations();
        status(cx, state, c"");
        return Some(do_melee(state, cx, io, input));
    }
    session.close_disconnected_connections(&mut |event| {
        changes::apply_protocol_event(state, cx, io, event)
    });
    poll_menu(state, cx, io, 0);
    cx.with_legacy(|| SleepThread(840 / 120));
    let mut count = ready(state, controls);
    session.replay_wait(|| {
        if count != 1 {
            return false;
        }
        poll_menu(state, cx, io, 0);
        count = ready(state, controls);
        count == 1
    });
    if count == -1 {
        state.netplay.cancel_confirmations();
        flush_menu(state, cx, io);
        state.phase = crate::supermelee::session::MenuPhase::Menu;
        return Some(1);
    }
    if count != 2 {
        return Some(1);
    }
    state.netplay.queue_all(
        crate::supermelee::netplay::packet::Packet::InputDelay(session.options.input_delay as u32),
        false,
    );
    for player in [0usize, 1] {
        if controls[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(state, player);
        assert!(!conn.is_none());
        if !net_connection_is_connected(conn) {
            continue;
        }
        if conn.unwrap_or_else(|| panic!("connection")).discriminant {
            netplay_notify_seed_random(state, conn, SeedRandomNumbers());
        }
    }
    flush_menu(state, cx, io);
    None
}
pub(super) unsafe fn confirm(
    input: &crate::engine::input::InputSession,
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) -> u32 {
    let controls = cx.kernel.player_control;
    if let Some(result) = prepare_confirmation(input, state, controls, cx, io) {
        return result;
    }
    let session = state.netplay.clone();
    if !changes::with_protocol_driver(state, cx, io, |driver| {
        session.negotiate_ready_connections(true, 5, driver)
    }) {
        return 0;
    }
    if !state.netplay.setup_input_delay(session.options.input_delay) {
        return 0;
    }
    state.phase = crate::supermelee::session::MenuPhase::Menu;
    start(state, cx, io);
    u32::from(cx.world.activity() & CHECK_ABORT as u16 == 0)
}
pub(super) unsafe fn start(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    cx.with_legacy(|| {
        FadeMusic(0, 840 / 2);
        SleepThreadUntil(FadeScreen(FADE_ALL_TO_BLACK, 840 / 2).wrapping_add(840 / 60));
        FlushColorXForms();
        stop_music();
        FadeMusic(NORMAL_VOLUME as u8, 0);
    });
    drop(state.h_music.take());
    cx.kernel.install_player_inputs();
    {
        assert!(
            cx.b.selection_frames.is_none(),
            "battle picker already admitted"
        );
        cx.b.selection_frames = Some(
            state
                .fleet_picker
                .as_ref()
                .unwrap_or_else(|| panic!("fleet picker not loaded"))
                .frames(),
        );
        crate::supermelee::native::battle_picker::live::fill_pick_melee_frame(
            cx,
            &state.melee_setup,
        );
        cx.with_legacy(|| WaitForSoundEnd(u16::MAX));
        let selector = (cx.random() as u16 % NUMBER_OF_PLANET_TYPES as u16) as u8;
        load_gravity_well(cx.kernel, selector);
        assert!(cx.b.net.is_none(), "melee battle already owns a session");
        cx.b.net = Some(state.netplay.battle_net());
        crate::battle::battle_native::battle(cx, io, None);
        cx.b.net = None;
        cx.b.selection_frames = None;
        free_gravity_well(cx.kernel);
        cx.kernel.clear_player_inputs();
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            return;
        }
        cx.with_legacy(|| {
            SleepThreadUntil(FadeScreen(FADE_ALL_TO_BLACK, 840 / 2).wrapping_add(840 / 60));
            FlushColorXForms();
        });
    }
    cx.world.set_activity(SUPER_MELEE as u16);
    state.initialized = 0;
}
pub(super) unsafe fn start_button(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
) {
    let controls = cx.kernel.player_control;
    if !state.can_start_battle() {
        crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
        return;
    }
    if controls[0] & NETWORK_CONTROL != 0 && controls[1] & NETWORK_CONTROL != 0 {
        status_game(cx, state, (NETMELEE_STRING_BASE as i32 + 32) as usize);
        return;
    }
    if (controls[0] & NETWORK_CONTROL != 0 && controls[1] & COMPUTER_CONTROL != 0)
        || (controls[1] & NETWORK_CONTROL != 0 && controls[0] & COMPUTER_CONTROL != 0)
    {
        status_game(cx, state, (NETMELEE_STRING_BASE as i32 + 33) as usize);
        return;
    }
    let mut ok = true;
    for player in [0usize, 1] {
        if controls[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let index = match connection(state, player).filter(|conn| conn.connected) {
            Some(conn) if conn.state != 3 => Some(if player == 0 { 14 } else { 15 }),
            Some(_) => None,
            None => Some(if player == 0 { 5 } else { 6 }),
        };
        if let Some(index) = index {
            ok = false;
            status_game(cx, state, (NETMELEE_STRING_BASE as i32 + index) as usize);
        }
    }
    if !ok {
        crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
        return;
    }
    if ready(state, controls) != 2 {
        status_game(cx, state, (NETMELEE_STRING_BASE as i32 + 7) as usize);
    }
    state.netplay.confirm_connections();
    state.phase = crate::supermelee::session::MenuPhase::Confirm;
}
pub(super) unsafe fn connecting(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    input: &crate::engine::input::InputSession,
) -> u32 {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        let time = GetTimeCounter();
        let side = usize::from(state.melee_option == 0);
        input.set_paused(false);
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            return 0;
        }
        SetMenuSounds(input, 0, 0);
        if state.initialized == 0 {
            if !connection(state, side).is_none() {
                close_player_network_connection(state, cx, io, side as u16);
            }
            state.initialized = 1;
            let conn = open_player_network_connection(side as u16, state, cx, io);
            state.phase = crate::supermelee::session::MenuPhase::Connecting;
            connection_box(state, cx.kernel, is_server(conn));
        }
        poll_menu(state, cx, io, 0);
        if key(input, KEY_MENU_CANCEL) {
            if !connection(state, side).is_none() {
                close_player_network_connection(state, cx, io, side as u16);
                status_player(cx, state, side as isize);
            }
            redraw(cx, state);
            state.phase = crate::supermelee::session::MenuPhase::Menu;
            state.last_input_time = GetTimeCounter();
            flush_menu(state, cx, io);
            return 1;
        }
        if let Some(conn) = connection(state, side) {
            if conn.state == 2 || conn.state == 3 {
                cx.kernel.player_control[side] = NETWORK_CONTROL | 16;
                controls(cx, state, side, true);
                redraw(cx, state);
                status_player(cx, state, side as isize);
                state.phase = crate::supermelee::session::MenuPhase::Menu;
                state.last_input_time = GetTimeCounter();
                highlight(cx, state, state.melee_option, false);
                state.melee_option = 4;
            }
        }
        flush_menu(state, cx, io);
        cx.with_legacy(|| SleepThreadUntil(time.wrapping_add(840 / 30)));
        1
    }
}
pub(super) unsafe fn disconnects(state: &mut MeleeSession, cx: &mut crate::engine::battle::Cx<'_>) {
    for player in [0usize, 1] {
        if cx.kernel.player_control[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(state, player);
        if conn.is_none() || !net_connection_is_connected(conn) {
            cx.kernel.player_control[player] = HUMAN_CONTROL | 16;
            controls(cx, state, player, false);
            log::info!("Player {player} has disconnected; shifting controls");
        }
    }
}
pub(super) unsafe fn next_control(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    side: usize,
) {
    let human = HUMAN_CONTROL | 16;
    let computer = COMPUTER_CONTROL;
    let value = cx.kernel.player_control[side];
    cx.kernel.player_control[side] = if value == human
        || value == computer | 16
        || value == computer | 32
        || value == computer | 64
    {
        super::super::rules::next_control(value)
    } else if value == NETWORK_CONTROL | 16 {
        if !connection(state, side).is_none() {
            close_player_network_connection(state, cx, io, side as u16);
        }
        status_player(cx, state, -1);
        human
    } else {
        log::error!("Bad control type ({value}) in nextControlType");
        human
    };
    controls(cx, state, side, true);
}
