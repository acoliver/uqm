//! C ABI connection lifetime. All access is serialized on the network/game thread.
use super::abi::*;
use super::netrcv::data_ready_callback;
use super::netstate::NetConnectionStateData_release;
use super::packetq::{PacketQueue_init, PacketQueue_uninit};
use std::{ffi::c_void, ptr};

unsafe extern "C" fn release_connect(data: *mut NetConnectionStateData) {
    let data = data.cast::<ConnectStateData>();
    if (*data).isServer {
        ListenState_close((*data).state.listenState);
    } else {
        ConnectState_close((*data).state.connectState);
    }
    libc::free(data.cast());
}

unsafe fn go(conn: *mut NetConnection) -> i32 {
    if (*conn).stateFlags.connected {
        return 0;
    }
    assert_eq!((*conn).state, NetState_NetState_unconnected);
    NetConnection_setState(conn, NetState_NetState_connecting);
    let is_server = (*(*conn).options).isServer;
    let mut data: ConnectStateData = std::mem::zeroed();
    data.releaseFunction = Some(release_connect);
    data.isServer = is_server;
    if is_server {
        let flags = ListenFlags {
            familyDemand: ProtocolFamily_PF_unspec,
            familyPrefer: ProtocolFamily_PF_unspec,
            backlog: NETPLAY_LISTEN_BACKLOG as i32,
        };
        let state = listenPort(
            (*(*conn).options).port,
            Protocol_IPProto_tcp,
            &flags,
            Some(server_connected),
            Some(server_error),
            conn.cast(),
        );
        if state.is_null() {
            return -1;
        }
        data.state.listenState = state;
    } else {
        let flags = ConnectFlags {
            familyDemand: ProtocolFamily_PF_unspec,
            familyPrefer: ProtocolFamily_PF_unspec,
            timeout: NETPLAY_CONNECTTIMEOUT as i32,
            retryDelayMs: NETPLAY_RETRYDELAY as i32,
        };
        let state = connectHostByName(
            (*(*conn).options).host,
            (*(*conn).options).port,
            Protocol_IPProto_tcp,
            &flags,
            Some(client_connected),
            Some(client_error),
            conn.cast(),
        );
        if state.is_null() {
            return -1;
        }
        data.state.connectState = state;
    }
    let allocated = libc::malloc(size_of::<ConnectStateData>()).cast::<ConnectStateData>();
    assert!(!allocated.is_null());
    allocated.write(data);
    (*conn).stateData = allocated.cast();
    0
}

unsafe fn connected(conn: *mut NetConnection, nd: *mut NetDescriptor, discriminant: bool) {
    assert!((*conn).nd.is_null());
    (*conn).nd = nd;
    NetDescriptor_setExtra(nd, conn.cast());
    Socket_setInteractive(NetDescriptor_getSocket(nd));
    (*conn).stateFlags.discriminant = discriminant;
    (*conn).stateFlags.connected = true;
    NetConnection_setState(conn, NetState_NetState_init);
    release_connect((*conn).stateData);
    (*conn).stateData = ptr::null_mut();
    NetDescriptor_setReadCallback(nd, Some(data_ready_callback));
    NetDescriptor_setCloseCallback(nd, Some(descriptor_closed));
    (*conn).connectCallback.expect("connect callback")(conn);
}

unsafe extern "C" fn server_connected(
    state: *mut ListenState,
    _listener: *mut NetDescriptor,
    nd: *mut NetDescriptor,
    _addr: *const sockaddr,
    _len: socklen_t,
) {
    connected(ListenState_getExtra(state).cast(), nd, true);
}
unsafe extern "C" fn client_connected(
    state: *mut ConnectState,
    nd: *mut NetDescriptor,
    _addr: *const sockaddr,
    _len: socklen_t,
) {
    connected(ConnectState_getExtra(state).cast(), nd, false);
}
unsafe fn connect_failed(conn: *mut NetConnection, error: &NetConnectionError) {
    (*conn).stateFlags.connected = false;
    (*conn).stateFlags.disconnected = true;
    NetConnection_setState(conn, NetState_NetState_unconnected);
    libc::free((*conn).stateData.cast());
    (*conn).stateData = ptr::null_mut();
    if let Some(callback) = (*conn).errorCallback {
        callback(conn, error);
    }
    NetConnection_close(conn);
}
unsafe extern "C" fn server_error(state: *mut ListenState, error: *const ListenError) {
    let conn = ListenState_getExtra(state).cast::<NetConnection>();
    (*conn).stateFlags.connected = false;
    (*conn).stateFlags.disconnected = true;
    ListenState_close(state);
    connect_failed(
        conn,
        &NetConnectionError {
            state: NetState_NetState_connecting,
            err: (*error).err,
            extra: NetConnectionError__bindgen_ty_1 { listenError: error },
        },
    );
}
unsafe extern "C" fn client_error(state: *mut ConnectState, error: *const ConnectError) {
    let conn = ConnectState_getExtra(state).cast::<NetConnection>();
    (*conn).stateFlags.connected = false;
    (*conn).stateFlags.disconnected = true;
    ConnectState_close(state);
    connect_failed(
        conn,
        &NetConnectionError {
            state: NetState_NetState_connecting,
            err: (*error).err,
            extra: NetConnectionError__bindgen_ty_1 {
                connectError: error,
            },
        },
    );
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
) -> *mut NetConnection {
    let conn = libc::calloc(1, size_of::<NetConnection>()).cast::<NetConnection>();
    assert!(!conn.is_null());
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
unsafe extern "C" fn descriptor_closed(nd: *mut NetDescriptor) {
    let conn = NetDescriptor_getExtra(nd).cast::<NetConnection>();
    if conn.is_null() {
        return;
    }
    (*conn).nd = ptr::null_mut();
    do_close(conn);
}

/// # Safety
/// Consumes the live connection on the game thread.
#[no_mangle]
pub unsafe extern "C" fn NetConnection_close(conn: *mut NetConnection) {
    if !(*conn).nd.is_null() {
        NetDescriptor_setCloseCallback((*conn).nd, None);
        NetDescriptor_close((*conn).nd);
        (*conn).nd = ptr::null_mut();
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
            std::ffi::CStr::from_ptr(super::netstate::netStateData[state as usize].name)
                .to_string_lossy()
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
