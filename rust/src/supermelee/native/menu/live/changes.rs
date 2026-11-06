//! Local and remote team-change protocol.
use super::*;
use crate::supermelee::types::MeleeShip as SetupShip;

pub(super) unsafe fn view_name(state: &mut MeleeSession, side: u16) {
    if state.melee_started == 0 {
        name(state, side, 4, None);
    }
}
unsafe fn change_ship(state: &mut MeleeSession, side: u16, index: u16, ship: u32) -> bool {
    let ship_kind = SetupShip::from_u8(
        u8::try_from(ship).unwrap_or_else(|error| panic!("melee ship width: {error}")),
    )
    .unwrap_or_else(|| panic!("validated melee ship"));
    let menu = &mut *state;
    let changed = menu
        .melee_setup
        .set_ship_with_cost(side as usize, index, ship_kind, |ship| {
            crate::ships::master_native::choice_cost(&menu.catalog, ship as u16)
        })
        .unwrap_or_else(|error| panic!("validated melee fleet slot: {error}"));
    if !changed {
        return false;
    }
    if menu.melee_started == 0 {
        if ship == 255 {
            let r = slot_rect(side, index / 7, index % 7);
            repair_frame(state, &r);
        } else {
            ship_box(menu, side, index as usize, ship, false);
        }
        worth(state, side, 4);
    }
    if state.melee_option == 10 && state.side == side && index == (state.row * 7 + state.col) {
        state.current_ship = ship;
        draw_ship_strings(state, ship);
    }
    true
}
unsafe fn change_name(state: &mut MeleeSession, side: u16, value: &std::ffi::CStr) -> bool {
    if !state.melee_setup.teams[side as usize].rename(value) {
        return false;
    }
    if state.row != 2 || state.side != side || state.cur_index == u16::MAX {
        view_name(state, side);
    }
    true
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub unsafe fn local_change_ship(
    state: &mut MeleeSession,
    side: u16,
    index: u16,
    ship: u32,
) -> bool {
    if !change_ship(state, side, index, ship) {
        return false;
    }
    if state.melee_setup.sent[side as usize].ships[index as usize] == SetupShip::MeleeUnset {
        netplay_notify_all_set_ship(state, side, index);
        state.melee_setup.sent[side as usize].ships[index as usize] =
            SetupShip::from_u8(ship as u8).unwrap_or_else(|| panic!("validated melee ship"));
    }
    true
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub unsafe fn local_change_team_name(
    state: &mut MeleeSession,
    side: u16,
    value: &std::ffi::CStr,
) -> bool {
    if !change_name(state, side, value) {
        return false;
    }
    if state.melee_setup.sent[side as usize].name.is_none() {
        netplay_notify_all_set_team_name(state, side);
        state.melee_setup.sent[side as usize].set_name(Some(value));
    }
    true
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub unsafe fn local_change_team(
    state: &mut MeleeSession,
    side: usize,
    team: &crate::supermelee::setup::team::MeleeTeam,
) -> bool {
    let mut changed = false;
    for (index, ship) in team.ships.iter().enumerate() {
        changed |= local_change_ship(state, side as u16, index as u16, *ship as u32);
    }
    changed |= local_change_team_name(state, side as u16, team.c_name());
    changed
}
pub(super) fn bootstrap_sync_team(state: &mut MeleeSession, side: usize) {
    netplay_notify_all_set_fleet(state, side);
    let setup = &mut state.melee_setup;
    for index in 0..14 {
        assert_eq!(setup.sent[side].ships[index], SetupShip::MeleeUnset);
        setup.sent[side].ships[index] = setup.teams[side].ships[index];
    }
    netplay_notify_all_set_team_name(state, side as u16);
    let setup = &mut state.melee_setup;
    setup.sent[side].set_name(Some(setup.teams[side].c_name()));
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub(super) unsafe fn remote_ship(
    state: &mut MeleeSession,
    remote: usize,
    side: u16,
    index: u16,
    ship: u32,
) {
    let sent = state.melee_setup.sent[side as usize].ships[index as usize];
    if sent == SetupShip::MeleeUnset {
        change_ship(state, side, index, ship);
        netplay_notify_all_set_ship(state, side, index);
        return;
    }
    state.melee_setup.sent[side as usize].ships[index as usize] = SetupShip::MeleeUnset;
    if ship != sent as u32 && remote == side as usize {
        change_ship(state, side, index, ship);
        return;
    }
    if state.melee_setup.teams[side as usize].ships[index as usize] != sent {
        netplay_notify_all_set_ship(state, side, index);
        state.melee_setup.sent[side as usize].ships[index as usize] =
            SetupShip::from_u8(ship as u8).unwrap_or_else(|| panic!("validated remote ship"));
    }
}
/// # Safety
/// The menu and its graphics must be live on the game thread.
pub(super) unsafe fn remote_name(
    state: &mut MeleeSession,
    remote: usize,
    side: u16,
    value: &std::ffi::CStr,
) {
    let sent = state.melee_setup.sent[side as usize].name.clone();
    let Some(sent) = sent else {
        change_name(state, side, value);
        netplay_notify_all_set_team_name(state, side);
        return;
    };
    if value == sent.as_c_str() && remote == side as usize {
        change_name(state, side, value);
        state.melee_setup.sent[side as usize].set_name(None);
        return;
    }
    if state.melee_setup.teams[side as usize].c_name() != sent.as_c_str() {
        netplay_notify_all_set_team_name(state, side);
        state.melee_setup.sent[side as usize].set_name(Some(value));
    } else {
        state.melee_setup.sent[side as usize].set_name(None);
    }
}
pub(super) unsafe fn confirmation_cancelled(state: &mut MeleeSession, side: u16) {
    status(
        state,
        game_string(NETMELEE_STRING_BASE as i32 + if side == 0 { 16 } else { 17 }),
    );
    if state.phase == crate::supermelee::session::MenuPhase::Confirm {
        state.phase = crate::supermelee::session::MenuPhase::Menu;
    }
}
pub(super) fn with_protocol_driver<R>(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    action: impl FnOnce(&mut dyn crate::engine::battle::net::ProtocolDriver) -> R,
) -> R {
    let mut notices = |cx: &mut crate::engine::battle::Cx<'_>,
                       io: &mut crate::engine::observe::Io<'_>,
                       notice| {
        crate::supermelee::native::protocol::feedback(cx, io, notice, |cx, message| {
            cx.with_legacy(|| {
                // SAFETY: feedback lends terminated text while the menu owns its drawing resources.
                unsafe { status(state, message.as_ptr()) }
            });
        });
    };
    action(&mut crate::supermelee::native::protocol::Driver::new(
        cx,
        io,
        &mut notices,
    ))
}

pub(crate) fn apply_protocol_event(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    event: crate::supermelee::netplay::Event,
) {
    use crate::supermelee::netplay::Effect;
    match event.effect {
        Effect::ResetSentTeams => state.melee_setup.reset_sent(),
        Effect::Bootstrap => {
            bootstrap_sync_team(state, event.player);
            flush_menu(state, cx, io);
        }
        Effect::ConfirmationCancelled => {
            cx.with_legacy(|| {
                // SAFETY: the live menu retains its graphics through this synchronous draw.
                unsafe { confirmation_cancelled(state, event.player as u16) }
            })
        }
        Effect::Ship { side, index, ship } => {
            cx.with_legacy(|| {
                // SAFETY: validated fleet slots use this menu's retained catalog and frame.
                unsafe { remote_ship(state, event.player, side, index, ship) }
            })
        }
        Effect::TeamName { side, name } => {
            let mut terminated = name;
            terminated.push(0);
            let name = std::ffi::CStr::from_bytes_until_nul(&terminated)
                .unwrap_or_else(|error| panic!("terminated remote name: {error}"));
            cx.with_legacy(|| {
                // SAFETY: the terminated name and menu resources stay borrowed through rendering.
                unsafe { remote_name(state, event.player, side, name) }
            });
        }
        Effect::Seed(seed) => {
            crate::math::seed_random(seed);
        }
        _ => {
            let session = state.netplay.clone();
            with_protocol_driver(state, cx, io, |driver| session.effect(event, driver));
        }
    }
}
