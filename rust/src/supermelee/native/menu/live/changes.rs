//! Local and remote team-change protocol.
use super::*;
pub(super) unsafe fn view_name(state: *mut MELEE_STATE, side: u16) {
    if (*state).meleeStarted == 0 {
        name(state, side, 4, ptr::null());
    }
}
unsafe fn change_ship(state: *mut MELEE_STATE, side: u16, index: u16, ship: u32) -> bool {
    if !MeleeSetup_setShip((*state).meleeSetup, side as usize, index, ship) {
        return false;
    }
    if (*state).meleeStarted == 0 {
        if ship == 255 {
            let mut r = mem::zeroed();
            GetShipBox(&mut r, side, index / 7, index % 7);
            RepairMeleeFrame(&r);
        } else {
            ship_box(side, index as usize, ship, false);
        }
        worth(state, side, 4);
    }
    if (*state).MeleeOption == 10
        && (*state).side == side
        && index == ((*state).row * 7 + (*state).col)
    {
        (*state).currentShip = ship;
        DrawMeleeShipStrings(state, ship);
    }
    true
}
unsafe fn change_name(state: *mut MELEE_STATE, side: u16, value: *const c_char) -> bool {
    if !MeleeSetup_setTeamName((*state).meleeSetup, side as usize, value) {
        return false;
    }
    if (*state).row != 2 || (*state).side != side || (*pMeleeState).CurIndex == u16::MAX {
        view_name(state, side);
    }
    true
}
#[no_mangle]
pub unsafe extern "C" fn Melee_LocalChange_ship(
    state: *mut MELEE_STATE,
    side: u16,
    index: u16,
    ship: u32,
) -> bool {
    if !change_ship(state, side, index, ship) {
        return false;
    }
    let setup = (*state).meleeSetup;
    if MeleeSetup_getSentShip(setup, side as usize, index) == 254 {
        Netplay_NotifyAll_setShip(state, side, index);
        MeleeSetup_setSentShip(setup, side as usize, index, ship);
    }
    true
}
#[no_mangle]
pub unsafe extern "C" fn Melee_LocalChange_teamName(
    state: *mut MELEE_STATE,
    side: u16,
    value: *const c_char,
) -> bool {
    if !change_name(state, side, value) {
        return false;
    }
    let setup = (*state).meleeSetup;
    if MeleeSetup_getSentTeamName(setup, side as usize).is_null() {
        Netplay_NotifyAll_setTeamName(state, side);
        MeleeSetup_setSentTeamName(setup, side as usize, value);
    }
    true
}
#[no_mangle]
pub unsafe extern "C" fn Melee_LocalChange_fleet(
    state: *mut MELEE_STATE,
    side: usize,
    fleet: *const MeleeShip,
) -> bool {
    let mut changed = false;
    for index in 0..14 {
        changed |= Melee_LocalChange_ship(state, side as u16, index, *fleet.add(index as usize));
    }
    changed
}
#[no_mangle]
pub unsafe extern "C" fn Melee_LocalChange_team(
    state: *mut MELEE_STATE,
    side: usize,
    team: *const MeleeTeam,
) -> bool {
    let fleet = MeleeTeam_getFleet(team);
    let value = MeleeTeam_getTeamName(team);
    let mut changed = Melee_LocalChange_fleet(state, side, fleet);
    changed |= Melee_LocalChange_teamName(state, side as u16, value);
    changed
}
#[no_mangle]
pub unsafe extern "C" fn Melee_bootstrapSyncTeam(state: *mut MELEE_STATE, side: usize) {
    let setup = (*state).meleeSetup;
    Netplay_NotifyAll_setFleet(state, side);
    for index in 0..14 {
        let ship = MeleeSetup_getShip(setup, side, index);
        assert_eq!(MeleeSetup_getSentShip(setup, side, index), 254);
        MeleeSetup_setSentShip(setup, side, index, ship);
    }
    Netplay_NotifyAll_setTeamName(state, side as u16);
    MeleeSetup_setSentTeamName(setup, side, MeleeSetup_getTeamName(setup, side));
}
#[no_mangle]
pub unsafe extern "C" fn Melee_RemoteChange_ship(
    state: *mut MELEE_STATE,
    conn: *mut NetConnection,
    side: u16,
    index: u16,
    ship: u32,
) {
    let setup = (*state).meleeSetup;
    let sent = MeleeSetup_getSentShip(setup, side as usize, index);
    if sent == 254 {
        change_ship(state, side, index, ship);
        Netplay_NotifyAll_setShip(state, side, index);
        return;
    }
    MeleeSetup_setSentShip(setup, side as usize, index, 254);
    if ship != sent && NetConnection_getPlayerNr(conn) == side as i32 {
        change_ship(state, side, index, ship);
        return;
    }
    if MeleeSetup_getShip(setup, side as usize, index) != sent {
        Netplay_NotifyAll_setShip(state, side, index);
        MeleeSetup_setSentShip(setup, side as usize, index, ship);
    }
}
#[no_mangle]
pub unsafe extern "C" fn Melee_RemoteChange_teamName(
    state: *mut MELEE_STATE,
    conn: *mut NetConnection,
    side: u16,
    value: *const c_char,
) {
    let setup = (*state).meleeSetup;
    let sent = MeleeSetup_getSentTeamName(setup, side as usize);
    if sent.is_null() {
        change_name(state, side, value);
        Netplay_NotifyAll_setTeamName(state, side);
        return;
    }
    if libc::strcmp(value, sent) == 0 && NetConnection_getPlayerNr(conn) == side as i32 {
        change_name(state, side, value);
        MeleeSetup_setSentTeamName(setup, side as usize, ptr::null());
        return;
    }
    if libc::strcmp(MeleeSetup_getTeamName(setup, side as usize), sent) != 0 {
        Netplay_NotifyAll_setTeamName(state, side);
        MeleeSetup_setSentTeamName(setup, side as usize, value);
    } else {
        MeleeSetup_setSentTeamName(setup, side as usize, ptr::null());
    }
}
#[no_mangle]
pub unsafe extern "C" fn updateRandomSeed(_state: *mut MELEE_STATE, _side: u16, seed: u32) {
    TFB_SeedRandom(seed);
}
#[no_mangle]
pub unsafe extern "C" fn confirmationCancelled(state: *mut MELEE_STATE, side: u16) {
    status(game_string(
        NETMELEE_STRING_BASE as i32 + if side == 0 { 16 } else { 17 },
    ));
    if (*state).InputFunc.map(|f| f as usize) == Some(confirm as *const () as usize) {
        (*state).InputFunc = Some(DoMelee);
    }
}
unsafe fn feedback(conn: *mut NetConnection, value: *const c_char, popup: bool) {
    if NetMelee_getBattleState(conn).is_null() && !popup {
        status(value);
    } else {
        DoPopupWindow(value);
    }
}
#[no_mangle]
pub unsafe extern "C" fn connectedFeedback(conn: *mut NetConnection) {
    feedback(
        conn,
        game_string(
            NETMELEE_STRING_BASE as i32
                + if NetConnection_getPlayerNr(conn) == 0 {
                    8
                } else {
                    9
                },
        ),
        false,
    );
    PlayMenuSound(3);
}
#[no_mangle]
pub unsafe extern "C" fn errorFeedback(conn: *mut NetConnection) {
    feedback(
        conn,
        game_string(
            NETMELEE_STRING_BASE as i32
                + if NetConnection_getPlayerNr(conn) == 0 {
                    10
                } else {
                    11
                },
        ),
        false,
    );
}
#[no_mangle]
pub unsafe extern "C" fn closeFeedback(conn: *mut NetConnection) {
    feedback(
        conn,
        game_string(
            NETMELEE_STRING_BASE as i32
                + if NetConnection_getPlayerNr(conn) == 0 {
                    12
                } else {
                    13
                },
        ),
        false,
    );
}
#[no_mangle]
pub unsafe extern "C" fn abortFeedback(conn: *mut NetConnection, reason: u32) {
    if reason < 4 {
        feedback(
            conn,
            game_string(NETMELEE_STRING_BASE as i32 + 25 + reason as i32),
            true,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn resetFeedback(conn: *mut NetConnection, reason: u32, remote: bool) {
    flushPacketQueues();
    if reason == 2 && !remote {
        return;
    }
    if reason < 3 {
        feedback(
            conn,
            game_string(NETMELEE_STRING_BASE as i32 + 29 + reason as i32),
            false,
        );
    }
    GlobData.Game_state.CurrentActivity |= CHECK_ABORT as u16;
}
