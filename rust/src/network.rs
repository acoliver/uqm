#[cfg(test)]
#[path = "network/tests.rs"]
mod async_tests;
pub mod connect;
pub mod descriptor;
pub mod listen;
pub mod manager;

unsafe fn set_errno(err: libc::c_int) {
    #[cfg(target_os = "linux")]
    {
        *libc::__errno_location() = err;
    }
    #[cfg(any(
        target_vendor = "apple",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        *libc::__error() = err;
    }
}

use libc::{c_char, c_int, c_void, size_t, socklen_t, ssize_t};
use std::ptr;

#[repr(C)]
pub struct Socket {
    fd: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResolveFlags {
    dummy: c_int,
}

#[repr(C)]
pub struct ResolveError {
    gai_res: c_int,
    err: c_int,
}

pub type ResolveCallback = Option<unsafe extern "C" fn(*mut ResolveState, *mut libc::addrinfo)>;
pub type ResolveErrorCallback =
    Option<unsafe extern "C" fn(*mut ResolveState, *const ResolveError)>;
type CallbackFunction = Option<unsafe extern "C" fn(*mut c_void)>;

#[repr(C)]
pub struct ResolveState {
    ref_count: u32,
    state: c_int,
    flags: ResolveFlags,
    callback: ResolveCallback,
    error_callback: ResolveErrorCallback,
    extra: *mut c_void,
    callback_id: *mut c_void,
    error: ResolveError,
    result: *mut libc::addrinfo,
}

extern "C" {
    fn Callback_add(callback: CallbackFunction, arg: *mut c_void) -> *mut c_void;
    fn Callback_remove(id: *mut c_void) -> bool;
}

const CALLBACK_ID_INVALID: *mut c_void = ptr::null_mut();
const RESOLVE_CLOSED: c_int = 0;
const RESOLVE_RESOLVING: c_int = 1;

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
unsafe fn last_errno() -> c_int {
    *libc::__error()
}
#[no_mangle]
pub static protocolFamilyTranslation: [c_int; 3] = [libc::PF_UNSPEC, libc::PF_INET, libc::PF_INET6];

#[no_mangle]
pub static protocolTranslation: [c_int; 2] = [libc::IPPROTO_TCP, libc::IPPROTO_UDP];

#[no_mangle]
pub static socketTypeTranslation: [c_int; 2] = [libc::SOCK_STREAM, libc::SOCK_DGRAM];

#[cfg(target_os = "linux")]
unsafe fn last_errno() -> c_int {
    *libc::__errno_location()
}

#[no_mangle]
pub extern "C" fn Network_init() {}

#[no_mangle]
pub extern "C" fn Network_uninit() {}

unsafe fn socket_ref(sock: *mut Socket) -> &'static mut Socket {
    &mut *sock
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_open(
    domain: c_int,
    socket_type: c_int,
    protocol: c_int,
) -> *mut Socket {
    let domains = [libc::PF_UNSPEC, libc::PF_INET, libc::PF_INET6];
    let types = [libc::SOCK_STREAM, libc::SOCK_DGRAM];
    let protocols = [libc::IPPROTO_TCP, libc::IPPROTO_UDP];
    let fd = libc::socket(
        domains[domain as usize],
        types[socket_type as usize],
        protocols[protocol as usize],
    );
    if fd == -1 {
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(Socket { fd }))
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_openNative(
    domain: c_int,
    socket_type: c_int,
    protocol: c_int,
) -> *mut Socket {
    let fd = libc::socket(domain, socket_type, protocol);
    if fd == -1 {
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(Socket { fd }))
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_close(sock: *mut Socket) -> c_int {
    loop {
        let result = libc::close(socket_ref(sock).fd);
        if result == 0 {
            drop(Box::from_raw(sock));
            return 0;
        }
        if last_errno() != libc::EINTR {
            return -1;
        }
    }
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_connect(
    sock: *mut Socket,
    addr: *const libc::sockaddr,
    addr_len: socklen_t,
) -> c_int {
    loop {
        let result = libc::connect(socket_ref(sock).fd, addr, addr_len);
        if result != -1 || last_errno() != libc::EINTR {
            return result;
        }
    }
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_bind(
    sock: *mut Socket,
    addr: *const libc::sockaddr,
    addr_len: socklen_t,
) -> c_int {
    libc::bind(socket_ref(sock).fd, addr, addr_len)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_listen(sock: *mut Socket, backlog: c_int) -> c_int {
    libc::listen(socket_ref(sock).fd, backlog)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_accept(
    sock: *mut Socket,
    addr: *mut libc::sockaddr,
    addr_len: *mut socklen_t,
) -> *mut Socket {
    loop {
        let mut length = *addr_len;
        let fd = libc::accept(socket_ref(sock).fd, addr, &mut length);
        if fd >= 0 {
            *addr_len = length;
            return Box::into_raw(Box::new(Socket { fd }));
        }
        if last_errno() != libc::EINTR {
            return ptr::null_mut();
        }
    }
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_send(
    sock: *mut Socket,
    buf: *const c_void,
    len: size_t,
    flags: c_int,
) -> ssize_t {
    libc::send(socket_ref(sock).fd, buf, len, flags)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_sendto(
    sock: *mut Socket,
    buf: *const c_void,
    len: size_t,
    flags: c_int,
    addr: *const libc::sockaddr,
    addr_len: socklen_t,
) -> ssize_t {
    libc::sendto(socket_ref(sock).fd, buf, len, flags, addr, addr_len)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_recv(
    sock: *mut Socket,
    buf: *mut c_void,
    len: size_t,
    flags: c_int,
) -> ssize_t {
    libc::recv(socket_ref(sock).fd, buf, len, flags)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_recvfrom(
    sock: *mut Socket,
    buf: *mut c_void,
    len: size_t,
    flags: c_int,
    from: *mut libc::sockaddr,
    from_len: *mut socklen_t,
) -> ssize_t {
    libc::recvfrom(socket_ref(sock).fd, buf, len, flags, from, from_len)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_setNonBlocking(sock: *mut Socket) -> c_int {
    let fd = socket_ref(sock).fd;
    let flags = libc::fcntl(fd, libc::F_GETFL);
    if flags == -1 {
        return -1;
    }
    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK)
}

unsafe fn set_socket_option(sock: *mut Socket, level: c_int, option: c_int, value: c_int) -> c_int {
    libc::setsockopt(
        socket_ref(sock).fd,
        level,
        option,
        &value as *const _ as *const c_void,
        std::mem::size_of_val(&value) as socklen_t,
    )
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_setReuseAddr(sock: *mut Socket) -> c_int {
    set_socket_option(sock, libc::SOL_SOCKET, libc::SO_REUSEADDR, 1)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_setNodelay(sock: *mut Socket) -> c_int {
    set_socket_option(sock, libc::IPPROTO_TCP, libc::TCP_NODELAY, 1)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_setTOS(sock: *mut Socket, tos: c_int) -> c_int {
    set_socket_option(sock, libc::IPPROTO_IP, libc::IP_TOS, tos)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_setInteractive(sock: *mut Socket) -> c_int {
    if Socket_setNodelay(sock) == -1 {
        return -1;
    }
    Socket_setTOS(sock, 0x10)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_setInlineOOB(sock: *mut Socket) -> c_int {
    set_socket_option(sock, libc::SOL_SOCKET, libc::SO_OOBINLINE, 1)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_setKeepAlive(sock: *mut Socket) -> c_int {
    set_socket_option(sock, libc::IPPROTO_TCP, libc::SO_KEEPALIVE, 1)
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Socket_getError(sock: *mut Socket, error: *mut c_int) -> c_int {
    let mut len = std::mem::size_of::<c_int>() as socklen_t;
    libc::getsockopt(
        socket_ref(sock).fd,
        libc::SOL_SOCKET,
        libc::SO_ERROR,
        error as *mut c_void,
        &mut len,
    )
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn ResolveState_incRef(state: *mut ResolveState) {
    (*state).ref_count += 1;
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn ResolveState_decRef(state: *mut ResolveState) -> bool {
    (*state).ref_count -= 1;
    if (*state).ref_count == 0 {
        debug_assert_eq!((*state).callback_id, CALLBACK_ID_INVALID);
        drop(Box::from_raw(state));
        true
    } else {
        false
    }
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn ResolveState_setExtra(state: *mut ResolveState, extra: *mut c_void) {
    (*state).extra = extra;
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn ResolveState_getExtra(state: *mut ResolveState) -> *mut c_void {
    (*state).extra
}

unsafe extern "C" fn resolve_callback(arg: *mut c_void) {
    let state = arg.cast::<ResolveState>();
    (*state).callback_id = CALLBACK_ID_INVALID;
    if (*state).error.gai_res == 0 {
        ResolveState_incRef(state);
        if let Some(callback) = (*state).callback {
            callback(state, (*state).result);
        }
        let released = ResolveState_decRef(state);
        debug_assert!(released);
    } else {
        ResolveState_incRef(state);
        if let Some(callback) = (*state).error_callback {
            callback(state, &(*state).error);
        }
        let released = ResolveState_decRef(state);
        debug_assert!(released);
    }
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn getaddrinfoAsync(
    node: *const c_char,
    service: *const c_char,
    hints: *const libc::addrinfo,
    flags: *mut ResolveFlags,
    callback: ResolveCallback,
    error_callback: ResolveErrorCallback,
    extra: *mut c_void,
) -> *mut ResolveState {
    let mut state = Box::new(ResolveState {
        ref_count: 1,
        state: RESOLVE_RESOLVING,
        flags: *flags,
        callback,
        error_callback,
        extra,
        callback_id: CALLBACK_ID_INVALID,
        error: ResolveError { gai_res: 0, err: 0 },
        result: ptr::null_mut(),
    });
    state.error.gai_res = libc::getaddrinfo(node, service, hints, &mut state.result);
    state.error.err = last_errno();
    let raw = Box::into_raw(state);
    (*raw).callback_id = Callback_add(Some(resolve_callback), raw.cast());
    raw
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn Resolve_close(state: *mut ResolveState) {
    if (*state).callback_id != CALLBACK_ID_INVALID {
        Callback_remove((*state).callback_id);
        (*state).callback_id = CALLBACK_ID_INVALID;
    }
    (*state).state = RESOLVE_CLOSED;
    ResolveState_decRef(state);
}

/// # Safety
/// Pointer arguments must satisfy the corresponding C API contract and ownership rules.
#[no_mangle]
pub unsafe extern "C" fn splitAddrInfoOnFamily(
    info: *mut libc::addrinfo,
    family: c_int,
    selected: *mut *mut libc::addrinfo,
    selected_end: *mut *mut *mut libc::addrinfo,
    rest: *mut *mut libc::addrinfo,
    rest_end: *mut *mut *mut libc::addrinfo,
) {
    let mut selected_next = selected;
    let mut rest_next = rest;
    let mut current = info;
    while !current.is_null() {
        let next = (*current).ai_next;
        if (*current).ai_family == family {
            *selected_next = current;
            selected_next = &mut (*current).ai_next;
        } else {
            *rest_next = current;
            rest_next = &mut (*current).ai_next;
        }
        current = next;
    }
    *selected_next = ptr::null_mut();
    *rest_next = ptr::null_mut();
    *selected_end = selected_next;
    *rest_end = rest_next;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_open_bind_listen_and_connect_on_loopback() {
        unsafe {
            let listener = Socket_open(1, 0, 0);
            assert!(!listener.is_null());
            assert_eq!(Socket_setReuseAddr(listener), 0);
            let mut address: libc::sockaddr_in = std::mem::zeroed();
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "dragonfly"
            ))]
            {
                address.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
            }
            address.sin_family = libc::AF_INET as _;
            address.sin_addr.s_addr = u32::from_ne_bytes([127, 0, 0, 1]);
            address.sin_port = 0;
            assert_eq!(
                Socket_bind(
                    listener,
                    &address as *const _ as *const _,
                    std::mem::size_of_val(&address) as socklen_t
                ),
                0
            );
            assert_eq!(Socket_listen(listener, 1), 0);
            let mut bound: libc::sockaddr_in = std::mem::zeroed();
            let mut bound_len = std::mem::size_of_val(&bound) as socklen_t;
            assert_eq!(
                libc::getsockname(
                    socket_ref(listener).fd,
                    &mut bound as *mut _ as *mut _,
                    &mut bound_len
                ),
                0
            );
            let client = Socket_open(1, 0, 0);
            assert_eq!(
                Socket_connect(client, &bound as *const _ as *const _, bound_len),
                0
            );
            let mut peer: libc::sockaddr_in = std::mem::zeroed();
            let mut peer_len = std::mem::size_of_val(&peer) as socklen_t;
            let accepted = Socket_accept(listener, &mut peer as *mut _ as *mut _, &mut peer_len);
            assert!(!accepted.is_null());
            assert_eq!(Socket_close(accepted), 0);
            assert_eq!(Socket_close(client), 0);
            assert_eq!(Socket_close(listener), 0);
        }
    }
}
