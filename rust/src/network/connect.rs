use super::descriptor::*;
use super::*;
use crate::callback::ffi::{AlarmId, Alarm_addRelativeMs, Alarm_remove};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConnectFlags {
    pub family_demand: c_int,
    pub family_prefer: c_int,
    pub timeout: c_int,
    pub retry_delay_ms: c_int,
}

#[repr(C)]
pub struct ConnectError {
    pub state: c_int,
    pub err: c_int,
    pub resolve_error: *const ResolveError,
}

pub type ConnectCallback = Option<
    unsafe extern "C" fn(*mut ConnectState, *mut NetDescriptor, *const libc::sockaddr, socklen_t),
>;
pub type ConnectErrorCallback =
    Option<unsafe extern "C" fn(*mut ConnectState, *const ConnectError)>;

#[repr(C)]
pub struct ConnectState {
    ref_count: u32,
    pub(super) state: c_int,
    flags: ConnectFlags,
    connect_callback: ConnectCallback,
    error_callback: ConnectErrorCallback,
    extra: *mut c_void,
    info: *mut libc::addrinfo,
    info_ptr: *mut libc::addrinfo,
    resolve_state: *mut ResolveState,
    pub(super) nd: *mut NetDescriptor,
    alarm: Option<AlarmId>,
}

/// # Safety
/// `state` must hold a live reference.
#[no_mangle]
pub unsafe extern "C" fn ConnectState_incRef(state: *mut ConnectState) {
    assert!((*state).ref_count < u32::MAX);
    (*state).ref_count += 1;
}

/// # Safety
/// `state` must hold a live reference; all resources must be released before the final decrement.
#[no_mangle]
pub unsafe extern "C" fn ConnectState_decRef(state: *mut ConnectState) -> bool {
    assert!((*state).ref_count > 0);
    (*state).ref_count -= 1;
    if (*state).ref_count == 0 {
        assert!((*state).nd.is_null() && (*state).alarm.is_none());
        assert!((*state).info.is_null() && (*state).info_ptr.is_null());
        drop(Box::from_raw(state));
        true
    } else {
        false
    }
}

/// # Safety
/// `state` must hold a live reference. Cancels pending work and consumes that reference.
#[no_mangle]
pub unsafe extern "C" fn ConnectState_close(state: *mut ConnectState) {
    if !(*state).resolve_state.is_null() {
        resolve_close((*state).resolve_state);
        (*state).resolve_state = ptr::null_mut();
    }
    if (*state).alarm.is_some() {
        Alarm_remove((*state).alarm);
        (*state).alarm = None;
    }
    if !(*state).nd.is_null() {
        NetDescriptor_close((*state).nd);
        (*state).nd = ptr::null_mut();
    }
    if !(*state).info.is_null() {
        libc::freeaddrinfo((*state).info);
        (*state).info = ptr::null_mut();
        (*state).info_ptr = ptr::null_mut();
    }
    (*state).state = 0;
    ConnectState_decRef(state);
}

/// # Safety
/// `state` must hold a live reference.
#[no_mangle]
pub unsafe extern "C" fn ConnectState_getExtra(state: *mut ConnectState) -> *mut c_void {
    (*state).extra
}

unsafe fn report_error(state: *mut ConnectState, error: &ConnectError) {
    let callback = (*state).error_callback.expect("connect error callback");
    ConnectState_incRef(state);
    callback(state, error);
    ConnectState_decRef(state);
}

unsafe extern "C" fn connect_callback(nd: *mut NetDescriptor) {
    let state = NetDescriptor_getExtra(nd).cast::<ConnectState>();
    if (*state).alarm.is_some() {
        Alarm_remove((*state).alarm);
        (*state).alarm = None;
    }
    if (*state).state == 0 {
        ConnectState_decRef(state);
        return;
    }
    let mut err = 0;
    assert_ne!(Socket_getError(NetDescriptor_getSocket(nd), &mut err), -1);
    if err != 0 {
        NetDescriptor_close(nd);
        (*state).nd = ptr::null_mut();
        (*state).info_ptr = (*(*state).info_ptr).ai_next;
        connect_next(state);
        return;
    }
    (*state).nd = ptr::null_mut();
    NetDescriptor_setWriteCallback(nd, None);
    let callback = (*state).connect_callback.expect("connect success callback");
    ConnectState_incRef(state);
    ConnectState_incRef(state);
    callback(
        state,
        nd,
        (*(*state).info_ptr).ai_addr,
        (*(*state).info_ptr).ai_addrlen as socklen_t,
    );
    ConnectState_decRef(state);
    let released = ConnectState_decRef(state);
    assert!(released, "connect callback must close its state");
}

fn timeout_callback(arg: usize) {
    unsafe {
        let state = arg as *mut ConnectState;
        (*state).alarm = None;
        NetDescriptor_close((*state).nd);
        (*state).nd = ptr::null_mut();
        (*state).info_ptr = (*(*state).info_ptr).ai_next;
        connect_next(state);
    }
}

fn retry_callback(arg: usize) {
    unsafe {
        let state = arg as *mut ConnectState;
        (*state).alarm = None;
        (*state).info_ptr = (*state).info;
        connect_next(state);
    }
}

unsafe fn try_next(state: *mut ConnectState) -> *mut Socket {
    assert!((*state).nd.is_null());
    let info = (*state).info_ptr;
    let sock = socket_open_native((*info).ai_family, (*info).ai_socktype, (*info).ai_protocol);
    if sock.is_null() {
        return sock;
    }
    if Socket_setNonBlocking(sock) == -1 {
        return ptr::null_mut();
    }
    Socket_setReuseAddr(sock);
    Socket_setInlineOOB(sock);
    Socket_setKeepAlive(sock);
    if Socket_connect(sock, (*info).ai_addr, (*info).ai_addrlen as socklen_t) == 0
        || last_errno() == libc::EINPROGRESS
    {
        return sock;
    }
    let err = last_errno();
    Socket_close(sock);
    set_errno(err);
    ptr::null_mut()
}

unsafe fn connect_next(state: *mut ConnectState) {
    while !(*state).info_ptr.is_null() {
        let sock = try_next(state);
        if !sock.is_null() {
            (*state).nd = NetDescriptor_new(sock, state.cast());
            if (*state).nd.is_null() {
                let err = last_errno();
                Socket_close(sock);
                libc::freeaddrinfo((*state).info);
                (*state).info = ptr::null_mut();
                (*state).info_ptr = ptr::null_mut();
                (*state).state = 0;
                report_error(
                    state,
                    &ConnectError {
                        state: 2,
                        err,
                        resolve_error: ptr::null(),
                    },
                );
                return;
            }
            NetDescriptor_setWriteCallback((*state).nd, Some(connect_callback));
            assert!((*state).alarm.is_none());
            (*state).alarm = Alarm_addRelativeMs(
                (*state).flags.timeout as u32,
                timeout_callback,
                state as usize,
            );
            return;
        }
        (*state).info_ptr = (*(*state).info_ptr).ai_next;
    }
    if (*state).flags.retry_delay_ms == -1 {
        libc::freeaddrinfo((*state).info);
        (*state).info = ptr::null_mut();
        (*state).info_ptr = ptr::null_mut();
        (*state).state = 0;
        report_error(
            state,
            &ConnectError {
                state: 2,
                err: libc::ETIMEDOUT,
                resolve_error: ptr::null(),
            },
        );
    } else {
        assert!((*state).alarm.is_none());
        (*state).alarm = Alarm_addRelativeMs(
            (*state).flags.retry_delay_ms as u32,
            retry_callback,
            state as usize,
        );
    }
}

unsafe extern "C" fn resolved(resolve: *mut ResolveState, mut info: *mut libc::addrinfo) {
    let state = resolve_state_get_extra(resolve).cast::<ConnectState>();
    (*state).state = 2;
    resolve_close(resolve);
    (*state).resolve_state = ptr::null_mut();
    if (*state).flags.family_prefer != libc::PF_UNSPEC {
        let mut preferred = ptr::null_mut();
        let mut preferred_end = ptr::null_mut();
        let mut rest = ptr::null_mut();
        let mut rest_end = ptr::null_mut();
        split_addr_info_on_family(
            info,
            (*state).flags.family_prefer,
            &mut preferred,
            &mut preferred_end,
            &mut rest,
            &mut rest_end,
        );
        *preferred_end = rest;
        info = preferred;
    }
    (*state).info = info;
    (*state).info_ptr = info;
    connect_next(state);
}

unsafe extern "C" fn resolve_error(resolve: *mut ResolveState, error: *const ResolveError) {
    let state = resolve_state_get_extra(resolve).cast::<ConnectState>();
    assert_ne!((*error).gai_res, 0);
    resolve_close(resolve);
    (*state).resolve_state = ptr::null_mut();
    report_error(
        state,
        &ConnectError {
            state: 1,
            err: (*error).err,
            resolve_error: error,
        },
    );
}

/// # Safety
/// Strings and flags must be valid for the call. Callbacks and `extra` must outlive the
/// operation. The success callback takes ownership of the descriptor and must close the state.
#[no_mangle]
pub unsafe extern "C" fn connectHostByName(
    host: *const c_char,
    service: *const c_char,
    proto: c_int,
    flags: *const ConnectFlags,
    callback: ConnectCallback,
    error_callback: ConnectErrorCallback,
    extra: *mut c_void,
) -> *mut ConnectState {
    assert!((0..=2).contains(&(*flags).family_demand));
    assert!((0..=2).contains(&(*flags).family_prefer));
    assert!((0..=1).contains(&proto));
    let mut hints: libc::addrinfo = std::mem::zeroed();
    hints.ai_family = PROTOCOL_FAMILY_TRANSLATION[(*flags).family_demand as usize];
    hints.ai_protocol = PROTOCOL_TRANSLATION[proto as usize];
    hints.ai_socktype = SOCKET_TYPE_TRANSLATION[proto as usize];
    let state = Box::into_raw(Box::new(ConnectState {
        ref_count: 1,
        state: 1,
        flags: *flags,
        connect_callback: callback,
        error_callback,
        extra,
        info: ptr::null_mut(),
        info_ptr: ptr::null_mut(),
        resolve_state: ptr::null_mut(),
        nd: ptr::null_mut(),
        alarm: None,
    }));
    let mut resolve_flags = ResolveFlags { dummy: 0 };
    (*state).resolve_state = getaddrinfo_async(
        host,
        service,
        &hints,
        &mut resolve_flags,
        Some(resolved),
        Some(resolve_error),
        state.cast(),
    );
    state
}
