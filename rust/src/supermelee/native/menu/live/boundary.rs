//! Typed access to engine owners shared with the melee menu.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct ConnectionView {
    pub player: usize,
    pub state: u32,
    pub connected: bool,
    pub server: bool,
    pub discriminant: bool,
}
pub(super) fn connection(state: &MeleeSession, player: usize) -> Option<ConnectionView> {
    state.netplay.connection(player).map(|conn| ConnectionView {
        player,
        state: conn.state,
        connected: conn.flags.connected,
        server: conn.options.server,
        discriminant: conn.flags.discriminant,
    })
}
pub(super) fn net_connection_is_connected(conn: Option<ConnectionView>) -> bool {
    conn.is_some_and(|conn| conn.connected)
}
pub(super) fn is_server(conn: Option<ConnectionView>) -> bool {
    conn.is_some_and(|conn| conn.server)
}
pub(super) fn netplay_notify_seed_random(
    state: &MeleeSession,
    conn: Option<ConnectionView>,
    seed: u32,
) {
    state.netplay.notify_seed(
        conn.unwrap_or_else(|| panic!("seed connection")).player,
        seed,
    );
}
pub(super) fn netplay_notify_all_set_ship(state: &MeleeSession, side: u16, index: u16) {
    let ship = state.melee_setup.teams[side as usize].ships[index as usize] as u32;
    state
        .netplay
        .notify_ship(side as usize, index as usize, ship);
}
pub(super) fn netplay_notify_all_set_team_name(state: &MeleeSession, side: u16) {
    let name = state.melee_setup.teams[side as usize].c_name();
    state.netplay.notify_name(side as usize, name.to_bytes());
}
pub(super) fn netplay_notify_all_set_fleet(state: &MeleeSession, side: usize) {
    let ships = state.melee_setup.teams[side].ships.map(|ship| ship as u32);
    state.netplay.notify_fleet(side, &ships);
}
pub(super) fn close_player_network_connection(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    player: u16,
) {
    let session = state.netplay.clone();
    session.close_player_network_connection(player, &mut |event| {
        changes::apply_protocol_event(state, cx, io, event)
    });
}
pub(super) fn open_player_network_connection(
    player: u16,
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) -> Option<ConnectionView> {
    let session = state.netplay.clone();
    if session.open_player_network_connection(player, &mut |event| {
        changes::apply_protocol_event(state, cx, io, event)
    }) {
        connection(state, player as usize)
    } else {
        None
    }
}
pub(crate) fn poll_menu(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    timeout: u32,
) {
    let session = state.netplay.clone();
    session.net_input_with(timeout, &mut |event| {
        changes::apply_protocol_event(state, cx, io, event)
    });
}
pub(super) fn flush_menu(
    state: &mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    let session = state.netplay.clone();
    session.flush_packet_queues(&mut |event| changes::apply_protocol_event(state, cx, io, event));
}

pub(super) unsafe fn set_context_clip_rect(rect: *mut RECT) -> u32 {
    crate::graphics::context_abi::SetContextClipRect(rect.cast())
}
pub(super) unsafe fn get_context_clip_rect(rect: *mut RECT) -> u32 {
    crate::graphics::context_abi::GetContextClipRect(rect.cast())
}
