//! Local and remote team-change protocol.
use super::*;
use crate::supermelee::types::MeleeShip as SetupShip;

pub(super) unsafe fn view_name(state: *mut MELEE_STATE, side: u16) {
    if (*state).melee_started == 0 {
        name(state, side, 4, ptr::null());
    }
}
unsafe fn change_ship(state: *mut MELEE_STATE, side: u16, index: u16, ship: u32) -> bool {
    let ship_kind = SetupShip::from_u8(u8::try_from(ship).expect("melee ship width"))
        .expect("validated melee ship");
    let menu = &mut *state;
    let changed = menu
        .melee_setup
        .set_ship_with_cost(side as usize, index, ship_kind, |ship| {
            crate::ships::master_native::choice_cost(&menu.catalog, ship as u16)
        })
        .expect("validated melee fleet slot");
    if !changed {
        return false;
    }
    if menu.melee_started == 0 {
        if ship == 255 {
            let mut r = mem::zeroed();
            GetShipBox(&mut r, side, index / 7, index % 7);
            RepairMeleeFrame(&r);
        } else {
            ship_box(&menu.catalog, side, index as usize, ship, false);
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
unsafe fn change_name(state: *mut MELEE_STATE, side: u16, value: &std::ffi::CStr) -> bool {
    if !(*state).melee_setup.teams[side as usize].rename(value) {
        return false;
    }
    if (*state).row != 2 || (*state).side != side || (*state).cur_index == u16::MAX {
        view_name(state, side);
    }
    true
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub unsafe fn Melee_LocalChange_ship(
    state: *mut MELEE_STATE,
    side: u16,
    index: u16,
    ship: u32,
) -> bool {
    if !change_ship(state, side, index, ship) {
        return false;
    }
    if (*state).melee_setup.sent[side as usize].ships[index as usize] == SetupShip::MeleeUnset {
        netplay_notify_all_set_ship(&*state, side, index);
        (*state).melee_setup.sent[side as usize].ships[index as usize] =
            SetupShip::from_u8(ship as u8).expect("validated melee ship");
    }
    true
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub unsafe fn Melee_LocalChange_teamName(
    state: *mut MELEE_STATE,
    side: u16,
    value: &std::ffi::CStr,
) -> bool {
    if !change_name(state, side, value) {
        return false;
    }
    if (*state).melee_setup.sent[side as usize].name.is_none() {
        netplay_notify_all_set_team_name(&*state, side);
        (*state).melee_setup.sent[side as usize].set_name(Some(value));
    }
    true
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub unsafe fn Melee_LocalChange_team(
    state: *mut MELEE_STATE,
    side: usize,
    team: &crate::supermelee::setup::team::MeleeTeam,
) -> bool {
    let mut changed = false;
    for (index, ship) in team.ships.iter().enumerate() {
        changed |= Melee_LocalChange_ship(state, side as u16, index as u16, *ship as u32);
    }
    changed |= Melee_LocalChange_teamName(state, side as u16, team.c_name());
    changed
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub(super) unsafe fn bootstrap_sync_team(state: *mut MELEE_STATE, side: usize) {
    netplay_notify_all_set_fleet(&*state, side);
    let setup = &mut (*state).melee_setup;
    for index in 0..14 {
        assert_eq!(setup.sent[side].ships[index], SetupShip::MeleeUnset);
        setup.sent[side].ships[index] = setup.teams[side].ships[index];
    }
    netplay_notify_all_set_team_name(&*state, side as u16);
    let setup = &mut (*state).melee_setup;
    setup.sent[side].set_name(Some(setup.teams[side].c_name()));
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
    let sent = (*state).melee_setup.sent[side as usize].ships[index as usize];
    if sent == SetupShip::MeleeUnset {
        change_ship(state, side, index, ship);
        netplay_notify_all_set_ship(&*state, side, index);
        return;
    }
    (*state).melee_setup.sent[side as usize].ships[index as usize] = SetupShip::MeleeUnset;
    if ship != sent as u32 && remote == side as usize {
        change_ship(state, side, index, ship);
        return;
    }
    if (*state).melee_setup.teams[side as usize].ships[index as usize] != sent {
        netplay_notify_all_set_ship(&*state, side, index);
        (*state).melee_setup.sent[side as usize].ships[index as usize] =
            SetupShip::from_u8(ship as u8).expect("validated remote ship");
    }
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub(super) unsafe fn remote_name(
    state: *mut MELEE_STATE,
    remote: usize,
    side: u16,
    value: &std::ffi::CStr,
) {
    let sent = (*state).melee_setup.sent[side as usize].name.clone();
    let Some(sent) = sent else {
        change_name(state, side, value);
        netplay_notify_all_set_team_name(&*state, side);
        return;
    };
    if value == sent.as_c_str() && remote == side as usize {
        change_name(state, side, value);
        (*state).melee_setup.sent[side as usize].set_name(None);
        return;
    }
    if (*state).melee_setup.teams[side as usize].c_name() != sent.as_c_str() {
        netplay_notify_all_set_team_name(&*state, side);
        (*state).melee_setup.sent[side as usize].set_name(Some(value));
    } else {
        (*state).melee_setup.sent[side as usize].set_name(None);
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
        Effect::ResetSentTeams => (*state).melee_setup.reset_sent(),
        Effect::Bootstrap => {
            bootstrap_sync_team(state, event.player);
            (*state).netplay.flush_packet_queues();
        }
        Effect::ConfirmationCancelled => confirmation_cancelled(state, event.player as u16),
        Effect::Ship { side, index, ship } => remote_ship(state, event.player, side, index, ship),
        Effect::TeamName { side, name } => {
            let mut terminated = name;
            terminated.push(0);
            remote_name(
                state,
                event.player,
                side,
                std::ffi::CStr::from_bytes_until_nul(&terminated).expect("terminated remote name"),
            );
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
