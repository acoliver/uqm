//! Battle launch and network confirmation.
use super::*;
use crate::engine::Kernel as KernelOwner;
pub(super) unsafe fn ready() -> isize {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        let mut done = 0;
        for player in [0usize, 1] {
            let conn = connection(player);
            if conn.is_none()
                && KernelOwner::current().read(|owner| owner.player_control[player])
                    & NETWORK_CONTROL
                    == 0
            {
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
}
unsafe fn prepare_confirmation(
    input: &crate::engine::input::InputSession,
    state: *mut MELEE_STATE,
    controls: [u8; 2],
) -> Option<u32> {
    let session = &(*state).netplay;
    input.set_paused(false);
    if key(input, KEY_MENU_CANCEL) {
        (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
        (*state).netplay.cancel_confirmations();
        status(game_string(NETMELEE_STRING_BASE as i32 + 4));
        return Some(1);
    }
    if key(input, KEY_MENU_LEFT) || key(input, KEY_MENU_UP) || key(input, KEY_MENU_DOWN) {
        (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
        (*state).netplay.cancel_confirmations();
        status(c"".as_ptr());
        return Some(do_melee(state, input));
    }
    (*state).netplay.close_disconnected_connections();
    poll_menu(state, 0);
    SleepThread(840 / 120);
    let mut count = ready();
    session.replay_wait(|| {
        if count != 1 {
            return false;
        }
        poll_menu(state, 0);
        count = ready();
        count == 1
    });
    if count == -1 {
        (*state).netplay.cancel_confirmations();
        (*state).netplay.flush_packet_queues();
        (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
        return Some(1);
    }
    if count != 2 {
        return Some(1);
    }
    (*state).netplay.queue_all(
        crate::supermelee::netplay::packet::Packet::InputDelay(session.options.input_delay as u32),
        false,
    );
    for player in [0usize, 1] {
        if controls[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(player);
        assert!(!conn.is_none());
        if !net_connection_is_connected(conn) {
            continue;
        }
        if conn.unwrap_or_else(|| panic!("connection")).discriminant {
            netplay_notify_seed_random(conn, SeedRandomNumbers());
        }
    }
    (*state).netplay.flush_packet_queues();
    None
}
pub(super) unsafe fn confirm(
    input: &crate::engine::input::InputSession,
    state: *mut MELEE_STATE,
    cx: &mut crate::engine::battle::Cx<'_>,
) -> u32 {
    let controls = cx.kernel.player_control;
    if let Some(result) = cx.with_legacy(|| prepare_confirmation(input, state, controls)) {
        return result;
    }
    let session = (*state).netplay.clone();
    if !session.negotiate_ready_connections(true, 5, &mut |poll| cx.protocol_wait(poll)) {
        return 0;
    }
    if !(*state)
        .netplay
        .setup_input_delay(session.options.input_delay)
    {
        return 0;
    }
    (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
    cx.with_legacy(|| start(state));
    u32::from(cx.world.activity() & CHECK_ABORT as u16 == 0)
}
pub(super) unsafe fn start(state: *mut MELEE_STATE) {
    FadeMusic(0, 840 / 2);
    SleepThreadUntil(FadeScreen(FADE_ALL_TO_BLACK, 840 / 2).wrapping_add(840 / 60));
    FlushColorXForms();
    stop_music();
    FadeMusic(NORMAL_VOLUME as u8, 0);
    if !(*state).h_music.is_null() {
        DestroyMusic((*state).h_music);
        (*state).h_music = ptr::null_mut();
    }
    if set_player_input_all() != 0 {
        crate::supermelee::native::battle_picker::live::FillPickMeleeFrame((*state).melee_setup);
        WaitForSoundEnd(u16::MAX);
        KernelOwner::current().write(|kernel| {
            load_gravity_well(
                kernel,
                (TFB_Random() as u16 % NUMBER_OF_PLANET_TYPES as u16) as u8,
            )
        });
        let battle = crate::app::bridge::battle(|b| b as *mut crate::engine::battle::Battle);
        assert!(
            (*battle).net.is_none(),
            "melee battle already owns a session"
        );
        (*battle).net = Some((*state).netplay.battle_net());
        crate::app::bridge::enter(|cx| {
            let mut observer = cx.kernel.input_runtime().observer();
            let mut io = crate::engine::observe::Io {
                observer: observer.as_mut(),
            };
            crate::battle::battle_native::battle(cx, &mut io, None)
        });
        (*battle).net = None;
        KernelOwner::current().write(free_gravity_well);
        KernelOwner::current().write(ClearPlayerInputAll);
        if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16 != 0
        {
            return;
        }
        SleepThreadUntil(FadeScreen(FADE_ALL_TO_BLACK, 840 / 2).wrapping_add(840 / 60));
        FlushColorXForms();
    }
    crate::engine::World::current().write(|world| world.set_activity(SUPER_MELEE as u16));
    (*state).initialized = 0;
}
pub(super) unsafe fn start_button(state: *mut MELEE_STATE) {
    if MeleeSetup_getFleetValue((*state).melee_setup, 0) == 0
        || MeleeSetup_getFleetValue((*state).melee_setup, 1) == 0
    {
        PlayMenuSound(2);
        return;
    }
    if KernelOwner::current().read(|owner| owner.player_control[0]) & NETWORK_CONTROL != 0
        && KernelOwner::current().read(|owner| owner.player_control[1]) & NETWORK_CONTROL != 0
    {
        status(game_string(NETMELEE_STRING_BASE as i32 + 32));
        return;
    }
    if (KernelOwner::current().read(|owner| owner.player_control[0]) & NETWORK_CONTROL != 0
        && KernelOwner::current().read(|owner| owner.player_control[1]) & COMPUTER_CONTROL != 0)
        || (KernelOwner::current().read(|owner| owner.player_control[1]) & NETWORK_CONTROL != 0
            && KernelOwner::current().read(|owner| owner.player_control[0]) & COMPUTER_CONTROL != 0)
    {
        status(game_string(NETMELEE_STRING_BASE as i32 + 33));
        return;
    }
    let mut ok = true;
    for player in [0usize, 1] {
        if KernelOwner::current().read(|owner| owner.player_control[player]) & NETWORK_CONTROL == 0
        {
            continue;
        }
        let index = match connection(player).filter(|conn| conn.connected) {
            Some(conn) if conn.state != 3 => Some(if player == 0 { 14 } else { 15 }),
            Some(_) => None,
            None => Some(if player == 0 { 5 } else { 6 }),
        };
        if let Some(index) = index {
            ok = false;
            status(game_string(NETMELEE_STRING_BASE as i32 + index));
        }
    }
    if !ok {
        PlayMenuSound(2);
        return;
    }
    if ready() != 2 {
        status(game_string(NETMELEE_STRING_BASE as i32 + 7));
    }
    (*state).netplay.confirm_connections();
    (*state).phase = crate::supermelee::netplay::session::MenuPhase::Confirm;
}
pub(super) unsafe fn connecting(
    state: *mut MELEE_STATE,
    input: &crate::engine::input::InputSession,
) -> u32 {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        let time = GetTimeCounter();
        let side = usize::from((*state).melee_option == 0);
        input.set_paused(false);
        if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16 != 0
        {
            return 0;
        }
        SetMenuSounds(input, 0, 0);
        if (*state).initialized == 0 {
            if !connection(side).is_none() {
                close_player_network_connection(side as u16);
            }
            (*state).initialized = 1;
            let conn = open_player_network_connection(side as u16, state);
            (*state).phase = crate::supermelee::netplay::session::MenuPhase::Connecting;
            let font = SetContextFont(KernelOwner::current().read(|owner| owner.star_con_font));
            let color = SetContextForeGroundColor(rgb(0, 0, 0));
            BatchGraphics();
            let mut r = RECT {
                corner: POINT {
                    x: (ScreenWidth as i16 - 200) >> 1,
                    y: (ScreenHeight as i16 - 30) >> 1,
                },
                extent: EXTENT {
                    width: 200,
                    height: 30,
                },
            };
            draw_shadowed_box(&mut r, rgb(20, 20, 20), rgb(8, 8, 8), rgb(16, 16, 16));
            let mut t = text(
                ScreenWidth as i16 >> 1,
                r.corner.y + 10,
                TEXT_ALIGN_ALIGN_CENTER,
                game_string(NETMELEE_STRING_BASE as i32 + if is_server(conn) { 1 } else { 2 }),
            );
            font_DrawText(&mut t);
            t.pStr = game_string(NETMELEE_STRING_BASE as i32 + 18);
            t.baseline.y += 16;
            font_DrawText(&mut t);
            SetContextFont(font);
            SetContextForeGroundColor(color);
            UnbatchGraphics();
        }
        poll_menu(state, 0);
        if key(input, KEY_MENU_CANCEL) {
            if !connection(side).is_none() {
                close_player_network_connection(side as u16);
                status_player(side as isize);
            }
            redraw();
            (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
            (*state).last_input_time = GetTimeCounter();
            (*state).netplay.flush_packet_queues();
            return 1;
        }
        if let Some(conn) = connection(side) {
            if conn.state == 2 || conn.state == 3 {
                (**KernelOwner::current()).player_control[side] = NETWORK_CONTROL | 16;
                controls(side, true);
                redraw();
                status_player(side as isize);
                (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
                (*state).last_input_time = GetTimeCounter();
                highlight((*state).melee_option, false);
                (*state).melee_option = 4;
            }
        }
        (*state).netplay.flush_packet_queues();
        SleepThreadUntil(time.wrapping_add(840 / 30));
        1
    }
}
pub(super) unsafe fn disconnects() {
    for player in [0usize, 1] {
        if KernelOwner::current().read(|owner| owner.player_control[player]) & NETWORK_CONTROL == 0
        {
            continue;
        }
        let conn = connection(player);
        if conn.is_none() || !net_connection_is_connected(conn) {
            (**KernelOwner::current()).player_control[player] = HUMAN_CONTROL | 16;
            controls(player, false);
            log::info!("Player {player} has disconnected; shifting controls");
        }
    }
}
pub(super) unsafe fn next_control(side: usize) {
    let human = HUMAN_CONTROL | 16;
    let computer = COMPUTER_CONTROL;
    let value = KernelOwner::current().read(|owner| owner.player_control[side]);
    (**KernelOwner::current()).player_control[side] = if value == human
        || value == computer | 16
        || value == computer | 32
        || value == computer | 64
    {
        super::super::rules::next_control(value)
    } else if value == NETWORK_CONTROL | 16 {
        if !connection(side).is_none() {
            close_player_network_connection(side as u16);
        }
        status_player(-1);
        human
    } else {
        log::error!("Bad control type ({value}) in nextControlType");
        human
    };
    controls(side, true);
}
