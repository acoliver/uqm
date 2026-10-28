use super::descriptor::{invoke, NetDescriptor};
use super::sys as libc;
use super::sys::{fd_set, SocketHandle};
use super::{last_errno, set_errno};
use ::libc::c_int;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::LazyLock;

struct Manager {
    descriptors: BTreeMap<SocketHandle, usize>,
    read_set: fd_set,
    write_set: fd_set,
    exception_set: fd_set,
}

impl Manager {
    fn new() -> Self {
        Self {
            descriptors: BTreeMap::new(),
            // fd_set consists of integer bit masks; all-zero is the empty set.
            read_set: unsafe { std::mem::zeroed() },
            write_set: unsafe { std::mem::zeroed() },
            exception_set: unsafe { std::mem::zeroed() },
        }
    }
}

static MANAGER: LazyLock<Mutex<Manager>> = LazyLock::new(|| Mutex::new(Manager::new()));

#[no_mangle]
pub extern "C" fn NetManager_init() {
    *MANAGER.lock() = Manager::new();
}

#[no_mangle]
pub extern "C" fn NetManager_uninit() {}

/// # Safety
/// `nd` must have an open socket not already registered with the manager.
#[no_mangle]
pub unsafe extern "C" fn NetManager_addDesc(nd: *mut NetDescriptor) -> c_int {
    let fd = (*(*nd).socket).fd;
    #[cfg(unix)]
    if fd as usize >= libc::FD_SETSIZE {
        set_errno(libc::EMFILE);
        return -1;
    }
    let mut manager = MANAGER.lock();
    #[cfg(windows)]
    if manager.descriptors.len() >= libc::FD_SETSIZE {
        set_errno(libc::EMFILE);
        return -1;
    }
    assert!(manager.descriptors.insert(fd, nd as usize).is_none());
    if (*nd).read_callback.is_some() {
        libc::FD_SET(fd, &mut manager.read_set);
    }
    if (*nd).write_callback.is_some() {
        libc::FD_SET(fd, &mut manager.write_set);
    }
    if (*nd).exception_callback.is_some() {
        libc::FD_SET(fd, &mut manager.exception_set);
    }
    0
}

/// # Safety
/// `nd` must be registered with an open socket.
#[no_mangle]
pub unsafe extern "C" fn NetManager_removeDesc(nd: *mut NetDescriptor) {
    let fd = (*(*nd).socket).fd;
    let mut manager = MANAGER.lock();
    assert_eq!(manager.descriptors.remove(&fd), Some(nd as usize));
    libc::FD_CLR(fd, &mut manager.read_set);
    libc::FD_CLR(fd, &mut manager.write_set);
    libc::FD_CLR(fd, &mut manager.exception_set);
}

/// # Safety
/// `nd` must be registered with an open socket.
#[no_mangle]
pub unsafe extern "C" fn NetManager_activateReadCallback(nd: *mut NetDescriptor) {
    let fd = (*(*nd).socket).fd;
    libc::FD_SET(fd, &mut MANAGER.lock().read_set);
}

/// # Safety
/// `nd` must be registered with an open socket.
#[no_mangle]
pub unsafe extern "C" fn NetManager_deactivateReadCallback(nd: *mut NetDescriptor) {
    let fd = (*(*nd).socket).fd;
    libc::FD_CLR(fd, &mut MANAGER.lock().read_set);
}

/// # Safety
/// `nd` must be registered with an open socket.
#[no_mangle]
pub unsafe extern "C" fn NetManager_activateWriteCallback(nd: *mut NetDescriptor) {
    let fd = (*(*nd).socket).fd;
    libc::FD_SET(fd, &mut MANAGER.lock().write_set);
}

/// # Safety
/// `nd` must be registered with an open socket.
#[no_mangle]
pub unsafe extern "C" fn NetManager_deactivateWriteCallback(nd: *mut NetDescriptor) {
    let fd = (*(*nd).socket).fd;
    libc::FD_CLR(fd, &mut MANAGER.lock().write_set);
}

/// # Safety
/// `nd` must be registered with an open socket.
#[no_mangle]
pub unsafe extern "C" fn NetManager_activateExceptionCallback(nd: *mut NetDescriptor) {
    let fd = (*(*nd).socket).fd;
    libc::FD_SET(fd, &mut MANAGER.lock().exception_set);
}

/// # Safety
/// `nd` must be registered with an open socket.
#[no_mangle]
pub unsafe extern "C" fn NetManager_deactivateExceptionCallback(nd: *mut NetDescriptor) {
    let fd = (*(*nd).socket).fd;
    libc::FD_CLR(fd, &mut MANAGER.lock().exception_set);
}

/// # Safety
/// `timeout_ms` must be writable. Calls must be serialized on the network event thread;
/// callbacks may reenter this function, as in the C implementation.
#[no_mangle]
pub unsafe extern "C" fn NetManager_process(timeout_ms: *mut u32) -> c_int {
    let mut timeout = libc::timeval {
        tv_sec: (*timeout_ms / 1000) as _,
        tv_usec: ((*timeout_ms % 1000) * 1000) as _,
    };
    let (max_nd, mut reads, mut writes, mut exceptions) = {
        let manager = MANAGER.lock();
        (
            manager
                .descriptors
                .last_key_value()
                .map_or(0, |(&fd, _)| fd as usize + 1),
            manager.read_set,
            manager.write_set,
            manager.exception_set,
        )
    };
    let mut result;
    loop {
        result = libc::select(
            max_nd as c_int,
            &mut reads,
            &mut writes,
            &mut exceptions,
            &mut timeout,
        );
        if result != -1 || last_errno() != libc::EINTR {
            break;
        }
    }
    if result == -1 {
        *timeout_ms = timeout.tv_sec as u32 * 1000 + timeout.tv_usec as u32 / 1000;
        return -1;
    }
    let mut previous = None;
    while result != 0 {
        use std::ops::Bound::{Excluded, Unbounded};
        // Re-read after each callback: a callback may remove or replace later entries.
        let next = MANAGER
            .lock()
            .descriptors
            .range((previous.map_or(Unbounded, Excluded), Unbounded))
            .next()
            .map(|(&fd, &nd)| (fd, nd));
        let Some((fd, nd)) = next else {
            break;
        };
        previous = Some(fd);
        let nd = nd as *mut NetDescriptor;
        {
            let mut bit_set = false;
            let mut closed = false;
            if libc::FD_ISSET(fd, &exceptions) {
                bit_set = true;
                closed = invoke(nd, (*nd).exception_callback);
            }
            if !closed && libc::FD_ISSET(fd, &writes) {
                bit_set = true;
                closed = invoke(nd, (*nd).write_callback);
            }
            if !closed && libc::FD_ISSET(fd, &reads) {
                bit_set = true;
                invoke(nd, (*nd).read_callback);
            }
            if bit_set {
                result -= 1;
            }
        }
    }
    *timeout_ms = timeout.tv_sec as u32 * 1000 + timeout.tv_usec as u32 / 1000;
    0
}
