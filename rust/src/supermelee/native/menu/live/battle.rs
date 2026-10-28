//! Battle launch and network confirmation.
use super::*;
pub(super) unsafe fn ready() -> isize {
    let mut done = 0;
    for player in [0usize, 1] {
        if PlayerControl[player] & NETWORK_CONTROL == 0 {
            done += 1;
            continue;
        }
        let conn = connection(player);
        if conn.is_null() || !NetConnection_isConnected(conn) {
            return -1;
        }
        if NetConnection_getState(conn) > 3 {
            done += 1;
        }
    }
    done
}
pub(super) unsafe extern "C" fn confirm(state: *mut MELEE_STATE) -> u32 {
    GamePaused = 0;
    if key(KEY_MENU_CANCEL) {
        (*state).InputFunc = Some(DoMelee);
        (*state).netplay.cancel_confirmations();
        status(game_string(NETMELEE_STRING_BASE as i32 + 4));
        return 1;
    }
    if key(KEY_MENU_LEFT) || key(KEY_MENU_UP) || key(KEY_MENU_DOWN) {
        (*state).InputFunc = Some(DoMelee);
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
        (*state).InputFunc = Some(DoMelee);
        return 1;
    }
    if count != 2 {
        return 1;
    }
    (*state)
        .netplay
        .netplay_notify_all_input_delay((&(*state).netplay).options.inputDelay);
    for player in [0usize, 1] {
        if PlayerControl[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(player);
        assert!(!conn.is_null());
        if !NetConnection_isConnected(conn) {
            continue;
        }
        if NetConnection_getDiscriminant(conn) {
            Netplay_Notify_seedRandom(conn, SeedRandomNumbers());
        }
    }
    (*state).netplay.flush_packet_queues();
    if !(*state).netplay.negotiate_ready_connections(true, 5) {
        return 0;
    }
    if !(*state)
        .netplay
        .setup_input_delay((&(*state).netplay).options.inputDelay)
    {
        return 0;
    }
    (*state).InputFunc = Some(DoMelee);
    start(state);
    u32::from(GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0)
}
pub(super) unsafe fn start(state: *mut MELEE_STATE) {
    FadeMusic(0, 840 / 2);
    SleepThreadUntil(FadeScreen(FADE_ALL_TO_BLACK, 840 / 2).wrapping_add(840 / 60));
    FlushColorXForms();
    StopMusic();
    FadeMusic(NORMAL_VOLUME as u8, 0);
    if !(*state).hMusic.is_null() {
        DestroyMusic((*state).hMusic);
        (*state).hMusic = ptr::null_mut();
    }
    if SetPlayerInputAll() != 0 {
        crate::supermelee::native::battle_picker::live::FillPickMeleeFrame((*state).meleeSetup);
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
    (*state).Initialized = 0;
}
pub(super) unsafe fn start_button(state: *mut MELEE_STATE) {
    if MeleeSetup_getFleetValue((*state).meleeSetup, 0) == 0
        || MeleeSetup_getFleetValue((*state).meleeSetup, 1) == 0
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
        let index = if conn.is_null() || !NetConnection_isConnected(conn) {
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
    (*state).InputFunc = Some(confirm);
}
pub(super) unsafe extern "C" fn connecting(state: *mut MELEE_STATE) -> u32 {
    let time = GetTimeCounter();
    let side = usize::from((*state).MeleeOption == 0);
    GamePaused = 0;
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        return 0;
    }
    SetMenuSounds(0, 0);
    if (*state).Initialized == 0 {
        if !connection(side).is_null() {
            closePlayerNetworkConnection(side as u16);
        }
        (*state).Initialized = 1;
        let conn = openPlayerNetworkConnection(side as u16, state.cast());
        (*state).InputFunc = Some(connecting);
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
        DrawShadowedBox(&mut r, rgb(20, 20, 20), rgb(8, 8, 8), rgb(16, 16, 16));
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
            closePlayerNetworkConnection(side as u16);
            status_player(side as isize);
        }
        redraw();
        (*state).InputFunc = Some(DoMelee);
        (*state).LastInputTime = GetTimeCounter();
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
            (*state).InputFunc = Some(DoMelee);
            (*state).LastInputTime = GetTimeCounter();
            highlight((*state).MeleeOption, false);
            (*state).MeleeOption = 4;
        }
    }
    (*state).netplay.flush_packet_queues();
    SleepThreadUntil(time.wrapping_add(840 / 30));
    1
}
pub(super) unsafe fn disconnects() {
    for player in [0usize, 1] {
        if PlayerControl[player] & NETWORK_CONTROL == 0 {
            continue;
        }
        let conn = connection(player);
        if conn.is_null() || !NetConnection_isConnected(conn) {
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
            closePlayerNetworkConnection(side as u16);
        }
        status_player(-1);
        human
    } else {
        log::error!("Bad control type ({value}) in nextControlType");
        human
    };
    controls(side, true);
}
