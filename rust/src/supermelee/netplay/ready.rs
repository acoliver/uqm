use super::abi::*;
use super::packetsenders::sendReady;
use std::ptr;

pub(super) fn meaningful(state: NetState) -> bool {
    [
        NetState_NetState_init,
        NetState_NetState_preBattle,
        NetState_NetState_selectShip,
        NetState_NetState_interBattle,
        NetState_NetState_inBattle,
        NetState_NetState_endingBattle,
        NetState_NetState_endingBattle2,
    ]
    .contains(&state)
}

unsafe fn both_ready(conn: *mut NetConnection) {
    let callback = (*conn).readyCallback.expect("ready callback");
    let arg = (*conn).readyCallbackArg;
    // Clear before invoking: the callback may begin the next ready exchange.
    NetConnection_setReadyCallback(conn, None, ptr::null_mut());
    callback(conn, arg);
}

/// # Safety
/// `conn` and `readyArg` remain valid through the callback, on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_localReady(
    conn: *mut NetConnection,
    callback: NetConnection_ReadyCallback,
    ready_arg: *mut libc::c_void,
    notify_remote: bool,
) -> bool {
    assert!(meaningful((*conn).state));
    assert!(!(*conn).stateFlags.ready.localReady());
    assert!(callback.is_some());
    NetConnection_setReadyCallback(conn, callback, ready_arg);
    if notify_remote {
        sendReady(conn);
    }
    if !(*conn).stateFlags.ready.remoteReady() {
        (*conn).stateFlags.ready.set_localReady(true);
        return false;
    }
    (*conn).stateFlags.ready.set_remoteReady(false);
    both_ready(conn);
    true
}

/// # Safety
/// `conn` is a live connection owned by the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_remoteReady(conn: *mut NetConnection) -> bool {
    assert!(meaningful((*conn).state));
    assert!(!(*conn).stateFlags.ready.remoteReady());
    if !(*conn).stateFlags.ready.localReady() {
        (*conn).stateFlags.ready.set_remoteReady(true);
        return false;
    }
    (*conn).stateFlags.ready.set_localReady(false);
    both_ready(conn);
    true
}

/// # Safety
/// `conn` points to a live connection.
#[no_mangle]
pub unsafe extern "C" fn Netplay_isLocalReady(conn: *const NetConnection) -> bool {
    (*conn).stateFlags.ready.localReady()
}
