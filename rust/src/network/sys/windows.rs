#![allow(non_camel_case_types, non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use windows_sys::Win32::Networking::WinSock as w;

pub use libc::{
    ECONNABORTED, EINPROGRESS, EINTR, EIO, EMFILE, ENFILE, ENOBUFS, ENOMEM, ETIMEDOUT, EWOULDBLOCK,
};
pub use w::{
    ADDRINFOA as addrinfo, FD_SET as fd_set, INVALID_SOCKET, SOCKADDR as sockaddr,
    SOCKADDR_STORAGE as sockaddr_storage, SOCKET as SocketHandle, TIMEVAL as timeval,
};
pub type socklen_t = c_int;
pub const PF_UNSPEC: c_int = w::AF_UNSPEC as c_int;
pub const PF_INET: c_int = w::AF_INET as c_int;
pub const PF_INET6: c_int = w::AF_INET6 as c_int;
pub const IPPROTO_TCP: c_int = w::IPPROTO_TCP;
pub const IPPROTO_UDP: c_int = w::IPPROTO_UDP;
pub const IPPROTO_IP: c_int = w::IPPROTO_IP;
pub const SOCK_STREAM: c_int = w::SOCK_STREAM;
pub const SOCK_DGRAM: c_int = w::SOCK_DGRAM;
pub const SOL_SOCKET: c_int = w::SOL_SOCKET;
pub const SO_REUSEADDR: c_int = w::SO_REUSEADDR;
pub const SO_OOBINLINE: c_int = w::SO_OOBINLINE;
pub const SO_KEEPALIVE: c_int = w::SO_KEEPALIVE;
pub const SO_ERROR: c_int = w::SO_ERROR;
pub const TCP_NODELAY: c_int = w::TCP_NODELAY;
pub const IP_TOS: c_int = w::IP_TOS;
pub const AI_PASSIVE: c_int = w::AI_PASSIVE as c_int;
pub const FD_SETSIZE: usize = w::FD_SETSIZE as usize;

fn errno_from_winsock(error: i32) -> i32 {
    match error {
        0 => 0,
        w::WSAEINTR => libc::EINTR,
        w::WSAEBADF => libc::EBADF,
        w::WSAEACCES => libc::EACCES,
        w::WSAEFAULT => libc::EFAULT,
        w::WSAEINVAL => libc::EINVAL,
        w::WSAEMFILE => libc::EMFILE,
        w::WSAEWOULDBLOCK => libc::EWOULDBLOCK,
        w::WSAEINPROGRESS => libc::EINPROGRESS,
        w::WSAEALREADY => libc::EALREADY,
        w::WSAENOTSOCK => libc::ENOTSOCK,
        w::WSAEDESTADDRREQ => libc::EDESTADDRREQ,
        w::WSAEMSGSIZE => libc::EMSGSIZE,
        w::WSAEPROTOTYPE => libc::EPROTOTYPE,
        w::WSAENOPROTOOPT => libc::ENOPROTOOPT,
        w::WSAEPROTONOSUPPORT => libc::EPROTONOSUPPORT,
        w::WSAEOPNOTSUPP => libc::EOPNOTSUPP,
        w::WSAEAFNOSUPPORT => libc::EAFNOSUPPORT,
        w::WSAEADDRINUSE => libc::EADDRINUSE,
        w::WSAEADDRNOTAVAIL => libc::EADDRNOTAVAIL,
        w::WSAENETDOWN => libc::ENETDOWN,
        w::WSAENETUNREACH => libc::ENETUNREACH,
        w::WSAENETRESET => libc::ENETRESET,
        w::WSAECONNABORTED => libc::ECONNABORTED,
        w::WSAECONNRESET => libc::ECONNRESET,
        w::WSAENOBUFS => libc::ENOBUFS,
        w::WSAEISCONN => libc::EISCONN,
        w::WSAENOTCONN => libc::ENOTCONN,
        w::WSAETIMEDOUT => libc::ETIMEDOUT,
        w::WSAECONNREFUSED => libc::ECONNREFUSED,
        w::WSAEHOSTUNREACH => libc::EHOSTUNREACH,
        _ => libc::EIO,
    }
}

pub(super) fn status(result: i32) -> i32 {
    if result == w::SOCKET_ERROR {
        // SAFETY: reads this thread's most recent Winsock error.
        crate::platform::set_errno(errno_from_winsock(unsafe { w::WSAGetLastError() }));
    }
    result
}

pub unsafe fn initialize() {
    let mut data = std::mem::zeroed();
    let result = w::WSAStartup(0x0202, &mut data);
    assert_eq!(result, 0, "WSAStartup failed: {result}");
}

pub unsafe fn uninitialize() {
    assert_eq!(w::WSACleanup(), 0, "WSACleanup failed");
}

pub unsafe fn socket(domain: i32, kind: i32, protocol: i32) -> SocketHandle {
    let result = w::socket(domain, kind, protocol);
    if result == INVALID_SOCKET {
        status(-1);
    }
    result
}

pub unsafe fn close(socket: SocketHandle) -> i32 {
    status(w::closesocket(socket))
}
pub unsafe fn connect(socket: SocketHandle, address: *const sockaddr, len: socklen_t) -> i32 {
    let result = status(w::connect(socket, address, len));
    if result == -1 && crate::platform::errno() == libc::EWOULDBLOCK {
        crate::platform::set_errno(libc::EINPROGRESS);
    }
    result
}
pub unsafe fn bind(socket: SocketHandle, address: *const sockaddr, len: socklen_t) -> i32 {
    status(w::bind(socket, address, len))
}
pub unsafe fn listen(socket: SocketHandle, backlog: i32) -> i32 {
    status(w::listen(socket, backlog))
}
pub unsafe fn accept(
    socket: SocketHandle,
    address: *mut sockaddr,
    len: *mut socklen_t,
) -> SocketHandle {
    let result = w::accept(socket, address, len);
    if result == INVALID_SOCKET {
        status(-1);
    }
    result
}
pub unsafe fn send(socket: SocketHandle, buf: *const c_void, len: usize, flags: i32) -> isize {
    status(w::send(
        socket,
        buf.cast(),
        len.min(i32::MAX as usize) as i32,
        flags,
    )) as isize
}
pub unsafe fn recv(socket: SocketHandle, buf: *mut c_void, len: usize, flags: i32) -> isize {
    status(w::recv(
        socket,
        buf.cast(),
        len.min(i32::MAX as usize) as i32,
        flags,
    )) as isize
}
pub unsafe fn nonblocking(socket: SocketHandle) -> i32 {
    let mut enabled = 1;
    status(w::ioctlsocket(socket, w::FIONBIO, &mut enabled))
}
pub unsafe fn setsockopt(
    socket: SocketHandle,
    level: i32,
    option: i32,
    value: *const c_void,
    len: socklen_t,
) -> i32 {
    status(w::setsockopt(socket, level, option, value.cast(), len))
}
pub unsafe fn getsockopt(
    socket: SocketHandle,
    level: i32,
    option: i32,
    value: *mut c_void,
    len: *mut socklen_t,
) -> i32 {
    let result = status(w::getsockopt(socket, level, option, value.cast(), len));
    if result == 0 && option == SO_ERROR {
        let error = value.cast::<i32>();
        *error = errno_from_winsock(*error);
    }
    result
}
pub unsafe fn getaddrinfo(
    node: *const c_char,
    service: *const c_char,
    hints: *const addrinfo,
    result: *mut *mut addrinfo,
) -> i32 {
    w::getaddrinfo(node.cast(), service.cast(), hints, result)
}
pub unsafe fn freeaddrinfo(info: *mut addrinfo) {
    w::freeaddrinfo(info)
}

pub unsafe fn FD_SET(socket: SocketHandle, set: *mut fd_set) {
    let set = &mut *set;
    if !FD_ISSET(socket, set) {
        assert!((set.fd_count as usize) < FD_SETSIZE);
        set.fd_array[set.fd_count as usize] = socket;
        set.fd_count += 1;
    }
}
pub unsafe fn FD_CLR(socket: SocketHandle, set: *mut fd_set) {
    let set = &mut *set;
    if let Some(index) = set.fd_array[..set.fd_count as usize]
        .iter()
        .position(|&fd| fd == socket)
    {
        set.fd_count -= 1;
        set.fd_array[index] = set.fd_array[set.fd_count as usize];
    }
}
pub unsafe fn FD_ISSET(socket: SocketHandle, set: *const fd_set) -> bool {
    let set = &*set;
    set.fd_array[..set.fd_count as usize].contains(&socket)
}
pub unsafe fn select(
    _: i32,
    reads: *mut fd_set,
    writes: *mut fd_set,
    exceptions: *mut fd_set,
    timeout: *mut timeval,
) -> i32 {
    let duration = std::time::Duration::from_secs((*timeout).tv_sec as u64)
        + std::time::Duration::from_micros((*timeout).tv_usec as u64);
    let started = std::time::Instant::now();
    let result = if (*reads).fd_count + (*writes).fd_count + (*exceptions).fd_count == 0 {
        // Winsock rejects empty sets instead of sleeping as POSIX select does.
        std::thread::sleep(duration);
        0
    } else {
        status(w::select(0, reads, writes, exceptions, timeout))
    };
    let remaining = duration.saturating_sub(started.elapsed());
    (*timeout).tv_sec = remaining.as_secs() as i32;
    (*timeout).tv_usec = remaining.subsec_micros() as i32;
    result
}
