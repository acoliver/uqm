//! Local and remote team-change protocol.
use super::*;
pub(super) unsafe fn view_name(state: *mut MELEE_STATE, side: u16) {
    if (*state).melee_started == 0 {
        name(state, side, 4, ptr::null());
    }
}
unsafe fn change_ship(state: *mut MELEE_STATE, side: u16, index: u16, ship: u32) -> bool {
    let catalog_state = &*state;
    let changed = set_ship(
        &mut *catalog_state.melee_setup,
        &catalog_state.catalog,
        side as usize,
        index,
        ship,
    );
    if !changed {
        return false;
    }
    if (*state).melee_started == 0 {
        if ship == 255 {
            let mut r = mem::zeroed();
            GetShipBox(&mut r, side, index / 7, index % 7);
            RepairMeleeFrame(&r);
        } else {
            ship_box(&catalog_state.catalog, side, index as usize, ship, false);
        }
        worth(state, side, 4);
    }
    if (*state).melee_option == 10
        && (*state).side == side
        && index == ((*state).row * 7 + (*state).col)
    {
        (*state).current_ship = ship;
        DrawMeleeShipStrings(state, ship);
    }
    true
}
unsafe fn change_name(state: *mut MELEE_STATE, side: u16, value: *const c_char) -> bool {
    if !MeleeSetup_setTeamName((*state).melee_setup, side as usize, value) {
        return false;
    }
    if (*state).row != 2 || (*state).side != side || (*P_MELEE_STATE).cur_index == u16::MAX {
        view_name(state, side);
    }
    true
}
#[no_mangle]
/// # Safety
/// Call on the game thread with initialized melee UI resources. All pointers
/// must remain live through notification callbacks; side and slot indices must be in range.
pub unsafe extern "C" fn Melee_LocalChange_ship(
    state: *mut MELEE_STATE,
    side: u16,
    index: u16,
    ship: u32,
) -> bool {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        if !change_ship(state, side, index, ship) {
            return false;
        }
        let setup = (*state).melee_setup;
        if MeleeSetup_getSentShip(setup, side as usize, index) == 254 {
            netplay_notify_all_set_ship(state, side, index);
            MeleeSetup_setSentShip(setup, side as usize, index, ship);
        }
        true
    }
}
#[no_mangle]
/// # Safety
/// Call on the game thread with initialized melee UI resources. All pointers
/// must remain live through notification callbacks; side and slot indices must be in range. The name must be NUL-terminated.
pub unsafe extern "C" fn Melee_LocalChange_teamName(
    state: *mut MELEE_STATE,
    side: u16,
    value: *const c_char,
) -> bool {
    if !change_name(state, side, value) {
        return false;
    }
    let setup = (*state).melee_setup;
    if MeleeSetup_getSentTeamName(setup, side as usize).is_null() {
        netplay_notify_all_set_team_name(state, side);
        MeleeSetup_setSentTeamName(setup, side as usize, value);
    }
    true
}
#[no_mangle]
/// # Safety
/// Call on the game thread with initialized melee UI resources. All pointers
/// must remain live through notification callbacks; side and slot indices must be in range. The fleet must contain all 14 ship slots.
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
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub unsafe fn Melee_LocalChange_team(
    state: *mut MELEE_STATE,
    side: usize,
    team: &crate::supermelee::setup::team::MeleeTeam,
) -> bool {
    let fleet = team.ships.map(|ship| ship as u32);
    let mut changed = Melee_LocalChange_fleet(state, side, fleet.as_ptr());
    changed |= Melee_LocalChange_teamName(state, side as u16, team.c_name().as_ptr());
    changed
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub(super) unsafe fn bootstrap_sync_team(state: *mut MELEE_STATE, side: usize) {
    let setup = (*state).melee_setup;
    netplay_notify_all_set_fleet(state, side);
    for index in 0..14 {
        let ship = MeleeSetup_getShip(setup, side, index);
        assert_eq!(MeleeSetup_getSentShip(setup, side, index), 254);
        MeleeSetup_setSentShip(setup, side, index, ship);
    }
    netplay_notify_all_set_team_name(state, side as u16);
    MeleeSetup_setSentTeamName(setup, side, MeleeSetup_getTeamName(setup, side));
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub(super) unsafe fn remote_ship(
    state: *mut MELEE_STATE,
    remote: usize,
    side: u16,
    index: u16,
    ship: u32,
) {
    let setup = (*state).melee_setup;
    let sent = MeleeSetup_getSentShip(setup, side as usize, index);
    if sent == 254 {
        change_ship(state, side, index, ship);
        netplay_notify_all_set_ship(state, side, index);
        return;
    }
    MeleeSetup_setSentShip(setup, side as usize, index, 254);
    if ship != sent && remote as i32 == side as i32 {
        change_ship(state, side, index, ship);
        return;
    }
    if MeleeSetup_getShip(setup, side as usize, index) != sent {
        netplay_notify_all_set_ship(state, side, index);
        MeleeSetup_setSentShip(setup, side as usize, index, ship);
    }
}
/// # Safety
/// The menu and name must be live on the game thread.
pub(super) unsafe fn remote_name(
    state: *mut MELEE_STATE,
    remote: usize,
    side: u16,
    value: *const c_char,
) {
    let setup = (*state).melee_setup;
    let sent = MeleeSetup_getSentTeamName(setup, side as usize);
    if sent.is_null() {
        change_name(state, side, value);
        netplay_notify_all_set_team_name(state, side);
        return;
    }
    if libc::strcmp(value, sent) == 0 && remote as i32 == side as i32 {
        change_name(state, side, value);
        MeleeSetup_setSentTeamName(setup, side as usize, ptr::null());
        return;
    }
    if libc::strcmp(MeleeSetup_getTeamName(setup, side as usize), sent) != 0 {
        netplay_notify_all_set_team_name(state, side);
        MeleeSetup_setSentTeamName(setup, side as usize, value);
    } else {
        MeleeSetup_setSentTeamName(setup, side as usize, ptr::null());
    }
}
pub(super) unsafe fn confirmation_cancelled(state: *mut MELEE_STATE, side: u16) {
    status(game_string(
        NETMELEE_STRING_BASE as i32 + if side == 0 { 16 } else { 17 },
    ));
    if (*state).phase == crate::supermelee::netplay::session::MenuPhase::Confirm {
        (*state).phase = crate::supermelee::netplay::session::MenuPhase::Menu;
    }
}
unsafe fn feedback(
    input: &crate::engine::input::InputSession,
    battle_active: bool,
    value: *const c_char,
    popup: bool,
) {
    if !battle_active && !popup {
        status(value);
    } else {
        do_popup_window(input, value);
    }
}

pub(crate) fn protocol_feedback(
    input: &crate::engine::input::InputSession,
    event: crate::supermelee::netplay::Event,
) {
    use crate::supermelee::netplay::Effect;
    // SAFETY: protocol dispatch runs synchronously in the live menu or its battle.
    unsafe {
        let index = match event.effect {
            Effect::Connected => Some(if event.player == 0 { 8 } else { 9 }),
            Effect::Error(_) => Some(if event.player == 0 { 10 } else { 11 }),
            Effect::Closed => Some(if event.player == 0 { 12 } else { 13 }),
            Effect::Abort(reason) if reason < 4 => Some(25 + reason as i32),
            Effect::Reset { reason, remote } => {
                reset_feedback(input, event.battle_active, reason, remote);
                return;
            }
            Effect::ShipSelected => {
                PlayMenuSound(1);
                return;
            }
            Effect::Abort(_) => None,
            _ => panic!("setup event requires the borrowed melee menu: {event:?}"),
        };
        if let Some(index) = index {
            feedback(
                input,
                event.battle_active,
                game_string(NETMELEE_STRING_BASE as i32 + index),
                matches!(event.effect, Effect::Abort(_)),
            );
        }
        if matches!(event.effect, Effect::Connected) {
            PlayMenuSound(3);
        }
    }
}

unsafe fn reset_feedback(
    input: &crate::engine::input::InputSession,
    battle_active: bool,
    reason: u32,
    remote: bool,
) {
    (*P_MELEE_STATE).netplay.flush_packet_queues();
    if reason == 2 && !remote {
        return;
    }
    if reason < 3 {
        feedback(
            input,
            battle_active,
            game_string(NETMELEE_STRING_BASE as i32 + 29 + reason as i32),
            false,
        );
    }
    crate::engine::World::current()
        .write(|world| world.set_activity(world.activity() | CHECK_ABORT as u16));
}

pub(crate) unsafe fn apply_protocol_event(
    state: *mut MELEE_STATE,
    event: crate::supermelee::netplay::Event,
) {
    use crate::supermelee::netplay::Effect;
    match event.effect {
        Effect::ResetSentTeams => MeleeSetup_resetSentTeams((*state).melee_setup),
        Effect::Bootstrap => {
            bootstrap_sync_team(state, event.player);
            (*state).netplay.flush_packet_queues();
        }
        Effect::ConfirmationCancelled => confirmation_cancelled(state, event.player as u16),
        Effect::Ship { side, index, ship } => remote_ship(state, event.player, side, index, ship),
        Effect::TeamName { side, name } => {
            let mut terminated = name;
            terminated.push(0);
            remote_name(state, event.player, side, terminated.as_ptr().cast());
        }
        Effect::Seed(seed) => {
            crate::math::seed_random(seed);
        }
        _ => {
            let input = (*state).netplay.controller().clone();
            protocol_feedback(&input, event);
        }
    }
}
