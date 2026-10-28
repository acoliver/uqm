//! Battle launch and network confirmation.
use super::*;
pub(super) unsafe fn ready() -> isize {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        let mut done = 0;
        for player in [0usize, 1] {
            let conn = connection(player);
            if conn.is_null() && PlayerControl[player] & NETWORK_CONTROL == 0 {
                done += 1;
                continue;
            }
            if conn.is_null() || !net_connection_is_connected(conn) {
                return -1;
            }
            if NetConnection_getState(conn) > 3 {
                done += 1;
            }
        }
        done
    }
}
pub(super) unsafe extern "C" fn confirm(state: *mut MELEE_STATE) -> u32 {
    let session = &(*state).netplay;
    GamePaused = 0;
    if key(KEY_MENU_CANCEL) {
        (*state).input_func = Some(DoMelee);
        (*state).netplay.cancel_confirmations();
        status(game_string(NETMELEE_STRING_BASE as i32 + 4));
        return 1;
    }
    if key(KEY_MENU_LEFT) || key(KEY_MENU_UP) || key(KEY_MENU_DOWN) {
        (*state).input_func = Some(DoMelee);
        (*state).netplay.cancel_confirmations();
        status(c"".as_ptr());
        return DoMelee(state);
    }
    (*state).netplay.close_disconnected_connections();
    (*state).netplay.net_input();
    SleepThread(840 / 120);
    let count = ready();
    if count == -1 {
        (*state).netplay.cancel_confirmations();
        (*state).netplay.flush_packet_queues();
        (*state).input_func = Some(DoMelee);
        return 1;
    }
    if count != 2 {
        return 1;
    }
    (*state)
        .netplay
        .netplay_notify_all_input_delay(session.options.inputDelay);
    for player in [0usize, 1] {
        if PlayerControl[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(player);
        assert!(!conn.is_null());
        if !net_connection_is_connected(conn) {
            continue;
        }
        if NetConnection_getDiscriminant(conn) {
            netplay_notify_seed_random(conn, SeedRandomNumbers());
        }
    }
    (*state).netplay.flush_packet_queues();
    if !(*state).netplay.negotiate_ready_connections(true, 5) {
        return 0;
    }
    if !(*state)
        .netplay
        .setup_input_delay(session.options.inputDelay)
    {
        return 0;
    }
    (*state).input_func = Some(DoMelee);
    start(state);
    u32::from(GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0)
}
pub(super) unsafe fn start(state: *mut MELEE_STATE) {
    FadeMusic(0, 840 / 2);
    SleepThreadUntil(FadeScreen(FADE_ALL_TO_BLACK, 840 / 2).wrapping_add(840 / 60));
    FlushColorXForms();
    StopMusic();
    FadeMusic(NORMAL_VOLUME as u8, 0);
    if !(*state).h_music.is_null() {
        DestroyMusic((*state).h_music);
        (*state).h_music = ptr::null_mut();
    }
    if set_player_input_all() != 0 {
        crate::supermelee::native::battle_picker::live::FillPickMeleeFrame((*state).melee_setup);
        WaitForSoundEnd(u16::MAX);
        load_gravity_well((TFB_Random() as u16 % NUMBER_OF_PLANET_TYPES as u16) as u8);
        let battle = crate::app::bridge::battle(|b| b as *mut crate::engine::battle::Battle);
        assert!(
            (*battle).net.is_none(),
            "melee battle already owns a session"
        );
        (*battle).net = Some((*state).netplay.battle_net());
        Battle(None);
        (*battle).net = None;
        free_gravity_well();
        ClearPlayerInputAll();
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return;
        }
        SleepThreadUntil(FadeScreen(FADE_ALL_TO_BLACK, 840 / 2).wrapping_add(840 / 60));
        FlushColorXForms();
    }
    GlobData.Game_state.CurrentActivity = SUPER_MELEE as u16;
    (*state).initialized = 0;
}
pub(super) unsafe fn start_button(state: *mut MELEE_STATE) {
    if MeleeSetup_getFleetValue((*state).melee_setup, 0) == 0
        || MeleeSetup_getFleetValue((*state).melee_setup, 1) == 0
    {
        PlayMenuSound(2);
        return;
    }
    if PlayerControl[0] & NETWORK_CONTROL != 0 && PlayerControl[1] & NETWORK_CONTROL != 0 {
        status(game_string(NETMELEE_STRING_BASE as i32 + 32));
        return;
    }
    if (PlayerControl[0] & NETWORK_CONTROL != 0 && PlayerControl[1] & COMPUTER_CONTROL != 0)
        || (PlayerControl[1] & NETWORK_CONTROL != 0 && PlayerControl[0] & COMPUTER_CONTROL != 0)
    {
        status(game_string(NETMELEE_STRING_BASE as i32 + 33));
        return;
    }
    let mut ok = true;
    for player in [0usize, 1] {
        if PlayerControl[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(player);
        let index = if conn.is_null() || !net_connection_is_connected(conn) {
            Some(if player == 0 { 5 } else { 6 })
        } else if NetConnection_getState(conn) != 3 {
            Some(if player == 0 { 14 } else { 15 })
        } else {
            None
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
    (*state).input_func = Some(confirm);
}
pub(super) unsafe extern "C" fn connecting(state: *mut MELEE_STATE) -> u32 {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        let time = GetTimeCounter();
        let side = usize::from((*state).melee_option == 0);
        GamePaused = 0;
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return 0;
        }
        SetMenuSounds(0, 0);
        if (*state).initialized == 0 {
            if !connection(side).is_null() {
                close_player_network_connection(side as u16);
            }
            (*state).initialized = 1;
            let conn = open_player_network_connection(side as u16, state.cast());
            (*state).input_func = Some(connecting);
            let font = SetContextFont(StarConFont);
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
        (*state).netplay.net_input();
        if key(KEY_MENU_CANCEL) {
            if !connection(side).is_null() {
                close_player_network_connection(side as u16);
                status_player(side as isize);
            }
            redraw();
            (*state).input_func = Some(DoMelee);
            (*state).last_input_time = GetTimeCounter();
            (*state).netplay.flush_packet_queues();
            return 1;
        }
        let conn = connection(side);
        if !conn.is_null() {
            let phase = NetConnection_getState(conn);
            if phase == 2 || phase == 3 {
                PlayerControl[side] = NETWORK_CONTROL | 16;
                controls(side, true);
                redraw();
                status_player(side as isize);
                (*state).input_func = Some(DoMelee);
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
        if PlayerControl[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(player);
        if conn.is_null() || !net_connection_is_connected(conn) {
            PlayerControl[player] = HUMAN_CONTROL | 16;
            controls(player, false);
            log::info!("Player {player} has disconnected; shifting controls");
        }
    }
}
pub(super) unsafe fn next_control(side: usize) {
    let human = HUMAN_CONTROL | 16;
    let computer = COMPUTER_CONTROL;
    let value = PlayerControl[side];
    PlayerControl[side] = if value == human
        || value == computer | 16
        || value == computer | 32
        || value == computer | 64
    {
        super::super::rules::next_control(value)
    } else if value == NETWORK_CONTROL | 16 {
        if !connection(side).is_null() {
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
