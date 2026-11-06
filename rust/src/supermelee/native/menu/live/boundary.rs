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
pub(super) unsafe fn close_player_network_connection(
    state: *mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    player: u16,
) {
    let session = (*state).netplay.clone();
    session.close_player_network_connection(player, &mut |event| {
        changes::apply_protocol_event(state, cx, io, event)
    });
}
pub(super) unsafe fn open_player_network_connection(
    player: u16,
    state: *mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) -> Option<ConnectionView> {
    let session = (*state).netplay.clone();
    if session.open_player_network_connection(player, &mut |event| {
        changes::apply_protocol_event(state, cx, io, event)
    }) {
        connection(&*state, player as usize)
    } else {
        None
    }
}
pub(crate) unsafe fn poll_menu(
    state: *mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    timeout: u32,
) {
    let session = (*state).netplay.clone();
    session.net_input_with(timeout, &mut |event| {
        changes::apply_protocol_event(state, cx, io, event)
    });
}
pub(super) unsafe fn flush_menu(
    state: *mut MeleeSession,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    let session = (*state).netplay.clone();
    session.flush_packet_queues(&mut |event| changes::apply_protocol_event(state, cx, io, event));
}

pub(super) unsafe fn get_line_within_width(
    text: *mut TEXT,
    next: *mut *const c_char,
    width: i16,
    count: u16,
) -> i32 {
    crate::comm::comm_abi::getLineWithinWidth(text.cast(), next, width, count)
}
pub(super) unsafe fn set_context_clip_rect(rect: *mut RECT) -> u32 {
    crate::graphics::context_abi::SetContextClipRect(rect.cast())
}
pub(super) unsafe fn text_rect(text: *mut TEXT, rect: *mut RECT, deltas: *mut u8) -> u32 {
    crate::graphics::font_abi::TextRect(text.cast(), rect.cast(), deltas)
}
pub(super) unsafe fn get_context_font_leading(leading: *mut i16) -> u32 {
    crate::graphics::font_abi::GetContextFontLeading(leading)
}
pub(super) unsafe fn draw_star_con_box(
    rect: *mut RECT,
    border: i16,
    top: Color,
    bottom: Color,
    fill: u32,
    inside: Color,
) {
    let convert = |c: Color| crate::navigation::abi::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    };
    crate::ui::util::live::DrawStarConBox(
        rect.cast(),
        border,
        convert(top),
        convert(bottom),
        fill,
        convert(inside),
    );
}
pub(super) unsafe fn do_text_entry(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    input: &crate::engine::input::InputSession,
    entry: *mut TEXTENTRY_STATE,
) -> u32 {
    crate::ui::getchar::run_text_entry(cx, io, entry.cast(), input)
}

pub(super) unsafe fn get_context_clip_rect(rect: *mut RECT) -> u32 {
    crate::graphics::context_abi::GetContextClipRect(rect.cast())
}
pub(super) unsafe fn set_context_origin(origin: POINT) -> POINT {
    let old = crate::graphics::context_abi::SetContextOrigin(crate::graphics::native_abi::POINT {
        x: origin.x,
        y: origin.y,
    });
    POINT { x: old.x, y: old.y }
}
pub(super) unsafe fn draw_shadowed_box(
    rect: *mut RECT,
    background: Color,
    dark: Color,
    medium: Color,
) {
    let convert = |c: Color| crate::graphics::native_abi::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    };
    crate::graphics::widgets_abi::DrawShadowedBox(
        rect.cast(),
        convert(background),
        convert(dark),
        convert(medium),
    );
}
