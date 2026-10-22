use super::abi::*;
use super::packetsenders::sendReset;
use std::ptr;

unsafe fn connection_reset(conn: *mut NetConnection, reason: NetplayResetReason, by_remote: bool) {
    if [
        NetState_NetState_preBattle,
        NetState_NetState_interBattle,
        NetState_NetState_selectShip,
        NetState_NetState_inBattle,
        NetState_NetState_endingBattle,
        NetState_NetState_endingBattle2,
    ]
    .contains(&(*conn).state)
    {
        resetFeedback(conn, reason, by_remote);
    }
}

unsafe fn condition_triggered(conn: *mut NetConnection) {
    if let Some(callback) = (*conn).resetCallback {
        if !(*conn).stateFlags.reset.localReset() || !(*conn).stateFlags.reset.remoteReset() {
            return;
        }
        (*conn).stateFlags.reset.set_localReset(false);
        (*conn).stateFlags.reset.set_remoteReset(false);
        let arg = (*conn).resetCallbackArg;
        // The callback may install the next reset exchange.
        NetConnection_setResetCallback(conn, None, ptr::null_mut());
        callback(conn, arg);
    }
}

/// # Safety
/// `conn` and `reset_arg` remain valid through the callback, on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_setResetCallback(
    conn: *mut NetConnection,
    callback: NetConnection_ResetCallback,
    reset_arg: *mut libc::c_void,
) {
    NetConnection_setResetCallback(conn, callback, reset_arg);
    condition_triggered(conn);
}

/// # Safety
/// `conn` is a live connection owned by the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_localReset(conn: *mut NetConnection, reason: NetplayResetReason) {
    assert!(!(*conn).stateFlags.reset.localReset());
    (*conn).stateFlags.reset.set_localReset(true);
    if (*conn).stateFlags.reset.remoteReset() {
        condition_triggered(conn);
    } else {
        sendReset(conn, reason);
        connection_reset(conn, reason, false);
    }
}

/// # Safety
/// `conn` is a live connection owned by the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_remoteReset(conn: *mut NetConnection, reason: NetplayResetReason) {
    assert!(!(*conn).stateFlags.reset.remoteReset());
    (*conn).stateFlags.reset.set_remoteReset(true);
    if !(*conn).stateFlags.reset.localReset() {
        sendReset(conn, reason);
        (*conn).stateFlags.reset.set_localReset(true);
        connection_reset(conn, reason, true);
    }
    condition_triggered(conn);
}

/// # Safety
/// `conn` points to a live connection.
#[no_mangle]
pub unsafe extern "C" fn Netplay_isLocalReset(conn: *const NetConnection) -> bool {
    (*conn).stateFlags.reset.localReset()
}

/// # Safety
/// `conn` points to a live connection.
#[no_mangle]
pub unsafe extern "C" fn Netplay_isRemoteReset(conn: *const NetConnection) -> bool {
    (*conn).stateFlags.reset.remoteReset()
}
