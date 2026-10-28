//! Typed access to engine owners shared with the melee menu.
use super::*;
pub(super) use crate::mainloop::c_extern::ClearPlayerInputAll;
use crate::supermelee::netplay as net;
pub(super) unsafe fn connection(player: usize) -> *mut NetConnection {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe { (*P_MELEE_STATE).netplay.connection(player) }
}
pub(super) unsafe fn net_connection_is_connected(conn: *mut NetConnection) -> bool {
    net::NetConnection_isConnected(conn.cast())
}
pub(super) unsafe fn net_connection_get_player_nr(conn: *mut NetConnection) -> i32 {
    net::NetConnection_getPlayerNr(conn.cast())
}
pub(super) unsafe fn is_server(conn: *mut NetConnection) -> bool {
    (*net::NetConnection_getPeerOptions(conn.cast())).isServer
}
pub(super) unsafe fn net_melee_get_battle_state(conn: *mut NetConnection) -> *mut std::ffi::c_void {
    net::NetMelee_getBattleState(conn.cast()).cast()
}
pub(super) unsafe fn netplay_notify_seed_random(conn: *mut NetConnection, seed: u32) {
    net::Netplay_Notify_seedRandom(conn.cast(), seed);
}
pub(super) unsafe fn netplay_notify_all_set_ship(state: *mut MELEE_STATE, side: u16, index: u16) {
    (*state)
        .netplay
        .netplay_notify_all_set_ship(state.cast(), side as usize, index as usize);
}
pub(super) unsafe fn netplay_notify_all_set_team_name(state: *mut MELEE_STATE, side: u16) {
    (*state)
        .netplay
        .netplay_notify_all_set_team_name(state.cast(), side as usize);
}
pub(super) unsafe fn netplay_notify_all_set_fleet(state: *mut MELEE_STATE, side: usize) {
    (*state)
        .netplay
        .netplay_notify_all_set_fleet(state.cast(), side);
}
pub(super) unsafe fn close_player_network_connection(player: u16) {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        (*P_MELEE_STATE)
            .netplay
            .close_player_network_connection(player);
    }
}
pub(super) unsafe fn open_player_network_connection(
    player: u16,
    state: *mut std::ffi::c_void,
) -> *mut NetConnection {
    (*state.cast::<MELEE_STATE>())
        .netplay
        .open_player_network_connection(player, state)
        .cast()
}
pub(super) unsafe fn capture_sound(sound: *mut std::ffi::c_void) -> SOUND {
    crate::game_init::macro_abi::CaptureSound(sound.cast()).cast()
}
pub(super) unsafe fn release_sound(sound: SOUND) -> *mut std::ffi::c_void {
    crate::game_init::macro_abi::ReleaseSound(sound.cast()).cast()
}
pub(super) unsafe fn set_player_input_all() -> i32 {
    i32::from(crate::navigation::abi::SetPlayerInputAll())
}

pub(super) use crate::battle::sounds::engine::PlayMenuSound;

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
pub(super) unsafe fn do_text_entry(entry: *mut TEXTENTRY_STATE) -> u32 {
    crate::ui::getchar::DoTextEntry(entry.cast())
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
