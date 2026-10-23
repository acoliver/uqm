//! Typed access to engine owners shared with the melee menu.
use super::*;
pub(super) use crate::mainloop::c_extern::ClearPlayerInputAll;
use crate::supermelee::netplay as net;
pub(super) use net::{
    cancelConfirmations, closeAllConnections, closeDisconnectedConnections, confirmConnections,
    setupInputDelay, Netplay_NotifyAll_inputDelay,
};

pub(super) unsafe fn connection(player: usize) -> *mut NetConnection {
    ptr::addr_of!(netConnections)
        .cast::<*mut NetConnection>()
        .add(player)
        .read()
}
pub(super) unsafe fn set_connection(player: usize, value: *mut NetConnection) {
    ptr::addr_of_mut!(netConnections)
        .cast::<*mut NetConnection>()
        .add(player)
        .write(value);
}
pub(super) unsafe fn NetConnection_isConnected(conn: *mut NetConnection) -> bool {
    net::NetConnection_isConnected(conn.cast())
}
pub(super) unsafe fn NetConnection_getPlayerNr(conn: *mut NetConnection) -> i32 {
    net::NetConnection_getPlayerNr(conn.cast())
}
pub(super) unsafe fn is_server(conn: *mut NetConnection) -> bool {
    (*net::NetConnection_getPeerOptions(conn.cast())).isServer
}
pub(super) unsafe fn NetMelee_getBattleState(conn: *mut NetConnection) -> *mut std::ffi::c_void {
    net::NetMelee_getBattleState(conn.cast()).cast()
}
pub(super) unsafe fn Netplay_Notify_seedRandom(conn: *mut NetConnection, seed: u32) {
    net::Netplay_Notify_seedRandom(conn.cast(), seed);
}
pub(super) unsafe fn Netplay_NotifyAll_setShip(state: *mut MELEE_STATE, side: u16, index: u16) {
    net::Netplay_NotifyAll_setShip(state.cast(), side as usize, index as usize);
}
pub(super) unsafe fn Netplay_NotifyAll_setTeamName(state: *mut MELEE_STATE, side: u16) {
    net::Netplay_NotifyAll_setTeamName(state.cast(), side as usize);
}
pub(super) unsafe fn Netplay_NotifyAll_setFleet(state: *mut MELEE_STATE, side: usize) {
    net::Netplay_NotifyAll_setFleet(state.cast(), side);
}
pub(super) unsafe fn closePlayerNetworkConnection(player: u16) {
    net::closePlayerNetworkConnection(player);
}
pub(super) unsafe fn openPlayerNetworkConnection(
    player: u16,
    state: *mut std::ffi::c_void,
) -> *mut NetConnection {
    net::openPlayerNetworkConnection(player, state).cast()
}
pub(super) unsafe fn CaptureSound(sound: *mut std::ffi::c_void) -> SOUND {
    crate::game_init::macro_abi::CaptureSound(sound.cast()).cast()
}
pub(super) unsafe fn ReleaseSound(sound: SOUND) -> *mut std::ffi::c_void {
    crate::game_init::macro_abi::ReleaseSound(sound.cast()).cast()
}
pub(super) unsafe fn SetPlayerInputAll() -> i32 {
    i32::from(crate::navigation::abi::SetPlayerInputAll())
}
extern "C" {
    pub(super) fn PlayMenuSound(sound: i32);
}

pub(super) unsafe fn getLineWithinWidth(
    text: *mut TEXT,
    next: *mut *const c_char,
    width: i16,
    count: u16,
) -> i32 {
    crate::comm::comm_abi::getLineWithinWidth(text.cast(), next, width, count)
}
pub(super) unsafe fn SetContextClipRect(rect: *mut RECT) -> u32 {
    crate::graphics::context_abi::SetContextClipRect(rect.cast())
}
pub(super) unsafe fn TextRect(text: *mut TEXT, rect: *mut RECT, deltas: *mut u8) -> u32 {
    crate::graphics::font_abi::TextRect(text.cast(), rect.cast(), deltas)
}
pub(super) unsafe fn GetContextFontLeading(leading: *mut i16) -> u32 {
    crate::graphics::font_abi::GetContextFontLeading(leading)
}
pub(super) unsafe fn DrawStarConBox(
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
pub(super) unsafe fn DoTextEntry(entry: *mut TEXTENTRY_STATE) -> u32 {
    crate::ui::getchar::DoTextEntry(entry.cast())
}

pub(super) unsafe fn GetContextClipRect(rect: *mut RECT) -> u32 {
    crate::graphics::context_abi::GetContextClipRect(rect.cast())
}
pub(super) unsafe fn SetContextOrigin(origin: POINT) -> POINT {
    let old = crate::graphics::context_abi::SetContextOrigin(crate::graphics::native_abi::POINT {
        x: origin.x,
        y: origin.y,
    });
    POINT { x: old.x, y: old.y }
}
pub(super) unsafe fn DrawShadowedBox(
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
