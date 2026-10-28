use super::descriptor::*;
use super::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ListenFlags {
    pub family_demand: c_int,
    pub family_prefer: c_int,
    pub backlog: c_int,
}

#[repr(C)]
pub struct ListenError {
    pub state: c_int,
    pub err: c_int,
    pub resolve_error: *const ResolveError,
}

pub type ListenCallback = Option<
    unsafe extern "C" fn(
        *mut ListenState,
        *mut NetDescriptor,
        *mut NetDescriptor,
        *const libc::sockaddr,
        socklen_t,
    ),
>;
pub type ListenErrorCallback = Option<unsafe extern "C" fn(*mut ListenState, *const ListenError)>;

#[repr(C)]
pub struct ListenState {
    ref_count: u32,
    pub(super) state: c_int,
    flags: ListenFlags,
    connect_callback: ListenCallback,
    error_callback: ListenErrorCallback,
    extra: *mut c_void,
    resolve_state: *mut ResolveState,
    pub(super) nds: *mut *mut NetDescriptor,
    pub(super) num_nd: usize,
}

/// # Safety
/// `state` must hold a live reference.
pub unsafe extern "C" fn ListenState_incRef(state: *mut ListenState) {
    assert!((*state).ref_count < u32::MAX);
    (*state).ref_count += 1;
}

/// # Safety
/// `state` must hold a live reference; listening descriptors must be closed before the final decrement.
pub unsafe extern "C" fn ListenState_decRef(state: *mut ListenState) -> bool {
    assert!((*state).ref_count > 0);
    (*state).ref_count -= 1;
    if (*state).ref_count == 0 {
        assert!((*state).nds.is_null());
        drop(Box::from_raw(state));
        true
    } else {
        false
    }
}

/// # Safety
/// `state` must hold a live reference. Cancels resolution, closes listeners and consumes that reference.
pub unsafe extern "C" fn ListenState_close(state: *mut ListenState) {
    if !(*state).resolve_state.is_null() {
        resolve_close((*state).resolve_state);
        (*state).resolve_state = ptr::null_mut();
    }
    if !(*state).nds.is_null() {
        let nds = ptr::slice_from_raw_parts_mut((*state).nds, (*state).num_nd);
        while (*state).num_nd > 0 {
            (*state).num_nd -= 1;
            NetDescriptor_close(*(*state).nds.add((*state).num_nd));
        }
        drop(Box::from_raw(nds));
        (*state).nds = ptr::null_mut();
    }
    (*state).state = 0;
    ListenState_decRef(state);
}

/// # Safety
/// `state` must hold a live reference.
pub unsafe extern "C" fn ListenState_getExtra(state: *mut ListenState) -> *mut c_void {
    (*state).extra
}

unsafe fn report_error(state: *mut ListenState, error: &ListenError) {
    let callback = (*state).error_callback.expect("listen error callback");
    ListenState_incRef(state);
    callback(state, error);
    ListenState_decRef(state);
}

unsafe fn close_preserving_errno(sock: *mut Socket) {
    let err = last_errno();
    Socket_close(sock);
    set_errno(err);
}

unsafe fn listen_single(state: *mut ListenState, info: *mut libc::addrinfo) -> *mut NetDescriptor {
    let sock = socket_open_native((*info).ai_family, (*info).ai_socktype, (*info).ai_protocol);
    if sock.is_null() {
        return ptr::null_mut();
    }
    Socket_setReuseAddr(sock);
    if Socket_setNonBlocking(sock) == -1
        || Socket_bind(sock, (*info).ai_addr, (*info).ai_addrlen as socklen_t) == -1
        || Socket_listen(sock, (*state).flags.backlog) == -1
    {
        close_preserving_errno(sock);
        return ptr::null_mut();
    }
    let nd = NetDescriptor_new(sock, state.cast());
    if nd.is_null() {
        close_preserving_errno(sock);
        return ptr::null_mut();
    }
    NetDescriptor_setReadCallback(nd, Some(accept_callback));
    nd
}

unsafe fn listen_multi(state: *mut ListenState, info: *mut libc::addrinfo) {
    let mut addr_count = 0;
    let mut address = info;
    while !address.is_null() {
        addr_count += 1;
        address = (*address).ai_next;
    }
    let mut descriptors = Vec::with_capacity(addr_count);
    address = info;
    while !address.is_null() {
        let nd = listen_single(state, address);
        if !nd.is_null() {
            descriptors.push(nd);
        }
        address = (*address).ai_next;
    }
    libc::freeaddrinfo(info);
    (*state).num_nd = descriptors.len();
    if descriptors.is_empty() {
        (*state).nds = ptr::null_mut();
        report_error(
            state,
            &ListenError {
                state: 2,
                err: libc::EIO,
                resolve_error: ptr::null(),
            },
        );
    } else {
        (*state).nds = Box::into_raw(descriptors.into_boxed_slice()).cast();
    }
}

unsafe extern "C" fn resolved(resolve: *mut ResolveState, info: *mut libc::addrinfo) {
    let state = resolve_state_get_extra(resolve).cast::<ListenState>();
    resolve_close((*state).resolve_state);
    (*state).resolve_state = ptr::null_mut();
    (*state).state = 2;
    listen_multi(state, info);
}

unsafe extern "C" fn resolve_error(resolve: *mut ResolveState, error: *const ResolveError) {
    let state = resolve_state_get_extra(resolve).cast::<ListenState>();
    assert_ne!((*error).gai_res, 0);
    report_error(
        state,
        &ListenError {
            state: 1,
            err: (*error).err,
            resolve_error: error,
        },
    );
}

unsafe extern "C" fn accept_callback(nd: *mut NetDescriptor) {
    let state = NetDescriptor_getExtra(nd).cast::<ListenState>();
    let sock = NetDescriptor_getSocket(nd);
    let mut address: libc::sockaddr_storage = std::mem::zeroed();
    let mut len = std::mem::size_of_val(&address) as socklen_t;
    let accepted = Socket_accept(
        sock,
        (&mut address as *mut libc::sockaddr_storage).cast(),
        &mut len,
    );
    if accepted.is_null() {
        let err = last_errno();
        if matches!(
            err,
            libc::EWOULDBLOCK
                | libc::ECONNABORTED
                | libc::EMFILE
                | libc::ENFILE
                | libc::ENOBUFS
                | libc::ENOMEM
        ) {
            return;
        }
        #[cfg(any(target_os = "linux", target_vendor = "apple", target_os = "netbsd"))]
        if err == libc::ENOSR {
            return;
        }
        panic!("unexpected accept error: {err}");
    }
    Socket_setReuseAddr(accepted);
    if Socket_setNonBlocking(accepted) == -1 {
        close_preserving_errno(accepted);
        return;
    }
    Socket_setInlineOOB(accepted);
    Socket_setKeepAlive(sock);
    let new_nd = NetDescriptor_new(accepted, ptr::null_mut());
    if new_nd.is_null() {
        close_preserving_errno(accepted);
        return;
    }
    let callback = (*state).connect_callback.expect("listen connect callback");
    ListenState_incRef(state);
    callback(
        state,
        nd,
        new_nd,
        (&address as *const libc::sockaddr_storage).cast(),
        len,
    );
    ListenState_decRef(state);
}

/// # Safety
/// `service` and `flags` must be valid for the call; callbacks and `extra` must outlive
/// the operation. Each connect callback takes ownership of the accepted descriptor.
pub unsafe extern "C" fn listenPort(
    service: *const c_char,
    proto: c_int,
    flags: *const ListenFlags,
    callback: ListenCallback,
    error_callback: ListenErrorCallback,
    extra: *mut c_void,
) -> *mut ListenState {
    assert!((0..=2).contains(&(*flags).family_demand));
    assert!((0..=2).contains(&(*flags).family_prefer));
    assert!((0..=1).contains(&proto));
    let mut hints: libc::addrinfo = std::mem::zeroed();
    hints.ai_family = PROTOCOL_FAMILY_TRANSLATION[(*flags).family_demand as usize];
    hints.ai_protocol = PROTOCOL_TRANSLATION[proto as usize];
    hints.ai_socktype = SOCKET_TYPE_TRANSLATION[proto as usize];
    hints.ai_flags = libc::AI_PASSIVE;
    let state = Box::into_raw(Box::new(ListenState {
        ref_count: 1,
        state: 1,
        flags: *flags,
        connect_callback: callback,
        error_callback,
        extra,
        resolve_state: ptr::null_mut(),
        nds: ptr::null_mut(),
        num_nd: 0,
    }));
    let mut resolve_flags = ResolveFlags { dummy: 0 };
    (*state).resolve_state = getaddrinfo_async(
        ptr::null(),
        service,
        &hints,
        &mut resolve_flags,
        Some(resolved),
        Some(resolve_error),
        state.cast(),
    );
    state
}
