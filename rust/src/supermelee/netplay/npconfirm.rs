use super::abi::*;
use super::packetsenders::{sendHandshake0, sendHandshake1, sendHandshakeCancel};

/// # Safety
/// `conn` is a live connection owned by the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_confirm(conn: *mut NetConnection) -> i32 {
    assert_eq!((*conn).state, NetState_NetState_inSetup);
    if (*conn).stateFlags.handshake.localOk() {
        super::set_errno(libc::EINVAL);
        return -1;
    }
    (*conn).stateFlags.handshake.set_localOk(true);
    if !(*conn).stateFlags.handshake.canceling() {
        if (*conn).stateFlags.handshake.remoteOk() {
            sendHandshake1(conn);
        } else {
            sendHandshake0(conn);
        }
    }
    0
}

/// # Safety
/// `conn` is a live connection owned by the game thread.
#[no_mangle]
pub unsafe extern "C" fn Netplay_cancelConfirmation(conn: *mut NetConnection) -> i32 {
    assert_eq!((*conn).state, NetState_NetState_inSetup);
    if !(*conn).stateFlags.handshake.localOk() {
        super::set_errno(libc::EINVAL);
        return -1;
    }
    (*conn).stateFlags.handshake.set_localOk(false);
    if !(*conn).stateFlags.handshake.canceling() {
        (*conn).stateFlags.handshake.set_canceling(true);
        sendHandshakeCancel(conn);
    }
    0
}
