//! C ABI connection lifetime. All access is serialized on the network/game thread.
use super::abi::*;

use super::netstate::NetConnectionStateData_release;
use super::packetq::{PacketQueue_init, PacketQueue_uninit};
use std::{ffi::c_void, ptr};

unsafe fn go(conn: *mut NetConnection) -> i32 {
    assert_eq!((*conn).state, NetState_NetState_unconnected);
    NetConnection_setState(conn, NetState_NetState_connecting);
    let options = &*(*conn).options;
    let host = std::ffi::CStr::from_ptr(options.host).to_string_lossy();
    let port = std::ffi::CStr::from_ptr(options.port).to_string_lossy();
    let result = {
        let mut net = (*(*conn).session).transport.borrow_mut();
        if options.isServer {
            net.listen(&port)
        } else {
            net.connect(&host, &port)
        }
    };
    match result {
        Ok(socket) => {
            (*conn).socket = Some(socket);
            0
        }
        Err(error) => {
            super::set_errno(error.raw_os_error().unwrap_or(libc::EIO));
            -1
        }
    }
}

/// # Safety
/// The connection and its owning session are live on the game thread.
pub(super) unsafe fn poll(conn: *mut NetConnection) {
    let socket = (*conn).socket.expect("connection without transport");
    if (*conn).state == NetState_NetState_connecting {
        let ready = (*(*conn).session).transport.borrow_mut().poll(socket);
        match ready {
            Ok(false) => return,
            Ok(true) => {
                (*conn).stateFlags.discriminant = (*(*conn).options).isServer;
                (*conn).stateFlags.connected = true;
                NetConnection_setState(conn, NetState_NetState_init);
                (*conn).connectCallback.expect("connect callback")(conn);
            }
            Err(error) => {
                NetConnection_doErrorCallback(conn, error.raw_os_error().unwrap_or(libc::EIO));
                disconnected(conn);
                return;
            }
        }
    }
    if (*conn).stateFlags.connected {
        super::netrcv::receive(conn);
    }
}

/// # Safety
/// The connection is live; its session is on the game thread.
pub(super) unsafe fn disconnected(conn: *mut NetConnection) {
    if let Some(socket) = (*conn).socket.take() {
        (*(*conn).session).transport.borrow_mut().close(socket);
    }
    do_close(conn);
}

/// # Safety
/// Options outlive the connection; callbacks follow the connection ownership contract.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_open(
    player: i32,
    options: *const NetplayPeerOptions,
    connect_callback: NetConnection_ConnectCallback,
    close_callback: NetConnection_CloseCallback,
    error_callback: NetConnection_ErrorCallback,
    delete_callback: NetConnection_DeleteCallback,
    extra: *mut c_void,
    session: *const super::session::SessionCore,
) -> *mut NetConnection {
    let conn = libc::calloc(1, size_of::<NetConnection>()).cast::<NetConnection>();
    assert!(!conn.is_null());
    (*conn).session = session;
    (*conn).player = player;
    (*conn).state = NetState_NetState_unconnected;
    (*conn).options = options;
    (*conn).extra = extra;
    PacketQueue_init(&raw mut (*conn).queue);
    (*conn).connectCallback = connect_callback;
    (*conn).closeCallback = close_callback;
    (*conn).errorCallback = error_callback;
    (*conn).deleteCallback = delete_callback;
    (*conn).readBuf = libc::malloc(NETPLAY_READBUFSIZE as usize).cast();
    assert!(!(*conn).readBuf.is_null());
    (*conn).readEnd = (*conn).readBuf;
    (*conn).stateFlags.checksumInterval = NETPLAY_CHECKSUM_INTERVAL as usize;
    go(conn);
    conn
}

unsafe fn do_close(conn: *mut NetConnection) {
    (*conn).stateFlags.connected = false;
    (*conn).stateFlags.disconnected = true;
    // The callback observes the old state and its state data.
    if let Some(callback) = (*conn).closeCallback {
        callback(conn);
    }
    NetConnection_setState(conn, NetState_NetState_unconnected);
}
/// # Safety
/// Consumes the live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_close(conn: *mut NetConnection) {
    if let Some(socket) = (*conn).socket.take() {
        (*(*conn).session).transport.borrow_mut().close(socket);
    }
    if !(*conn).stateFlags.disconnected {
        do_close(conn);
    }
    if let Some(callback) = (*conn).deleteCallback {
        callback(conn);
    }
    if !(*conn).stateData.is_null() {
        NetConnectionStateData_release((*conn).stateData);
        (*conn).stateData = ptr::null_mut();
    }
    libc::free((*conn).readBuf.cast());
    PacketQueue_uninit(&raw mut (*conn).queue);
    libc::free(conn.cast());
}

/// # Safety
/// `conn` has an error callback and is live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_doErrorCallback(conn: *mut NetConnection, err: i32) {
    let error = NetConnectionError {
        state: (*conn).state,
        err,
        extra: std::mem::zeroed(),
    };
    (*conn).errorCallback.expect("error callback")(conn, &error);
}

/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getStateData(
    conn: *const NetConnection,
) -> *mut NetConnectionStateData {
    (*conn).stateData
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_setStateData(
    conn: *mut NetConnection,
    value: *mut NetConnectionStateData,
) {
    (*conn).stateData = value;
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getExtra(conn: *const NetConnection) -> *mut c_void {
    (*conn).extra
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_setExtra(conn: *mut NetConnection, value: *mut c_void) {
    (*conn).extra = value;
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getState(conn: *const NetConnection) -> NetState {
    (*conn).state
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getDiscriminant(conn: *const NetConnection) -> bool {
    (*conn).stateFlags.discriminant
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_isConnected(conn: *const NetConnection) -> bool {
    (*conn).stateFlags.connected
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getPeerOptions(
    conn: *const NetConnection,
) -> *const NetplayPeerOptions {
    (*conn).options
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getPlayerNr(conn: *const NetConnection) -> i32 {
    (*conn).player
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getInputDelay(conn: *const NetConnection) -> usize {
    (*conn).stateFlags.inputDelay
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getChecksumInterval(conn: *const NetConnection) -> usize {
    (*conn).stateFlags.checksumInterval
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getReadyCallback(
    conn: *const NetConnection,
) -> NetConnection_ReadyCallback {
    (*conn).readyCallback
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getReadyCallbackArg(
    conn: *const NetConnection,
) -> *mut c_void {
    (*conn).readyCallbackArg
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getResetCallback(
    conn: *const NetConnection,
) -> NetConnection_ResetCallback {
    (*conn).resetCallback
}
/// # Safety
/// `conn` points to a live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getResetCallbackArg(
    conn: *const NetConnection,
) -> *mut c_void {
    (*conn).resetCallbackArg
}

/// # Safety
/// `conn` points to a live connection and state is a valid NetState.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_setState(conn: *mut NetConnection, state: NetState) {
    if cfg!(debug_assertions) && state == (*conn).state {
        log::warn!(
            "NETPLAY: [{}] Connection state set to {} while already in that state.",
            (*conn).player,
            super::netstate::state_name(state)
        );
    }
    (*conn).state = state;
}

/// # Safety
/// The callback argument remains valid until invoked or cleared on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_setReadyCallback(
    conn: *mut NetConnection,
    callback: NetConnection_ReadyCallback,
    arg: *mut c_void,
) {
    (*conn).readyCallback = callback;
    (*conn).readyCallbackArg = arg;
}
/// # Safety
/// The callback argument remains valid until invoked or cleared on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_setResetCallback(
    conn: *mut NetConnection,
    callback: NetConnection_ResetCallback,
    arg: *mut c_void,
) {
    (*conn).resetCallback = callback;
    (*conn).resetCallbackArg = arg;
}

/// # Safety
/// `conn` is a live exclusively owned connection.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_getChecksumBuffer(
    conn: *mut NetConnection,
) -> *mut ChecksumBuffer {
    &raw mut (*conn).checksumBuffer
}
