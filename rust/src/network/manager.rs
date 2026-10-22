use super::descriptor::{invoke, NetDescriptor};
use super::{last_errno, set_errno};
use libc::{c_int, fd_set};
use parking_lot::Mutex;
use std::sync::LazyLock;

struct Manager {
    descriptors: [usize; libc::FD_SETSIZE],
    max_nd: usize,
    read_set: fd_set,
    write_set: fd_set,
    exception_set: fd_set,
}

impl Manager {
    fn new() -> Self {
        Self {
            descriptors: [0; libc::FD_SETSIZE],
            max_nd: 0,
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
    if fd as usize >= libc::FD_SETSIZE {
        set_errno(libc::EMFILE);
        return -1;
    }
    let mut manager = MANAGER.lock();
    assert_eq!(manager.descriptors[fd as usize], 0);
    manager.descriptors[fd as usize] = nd as usize;
    manager.max_nd = manager.max_nd.max(fd as usize + 1);
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
    assert_eq!(manager.descriptors[fd as usize], nd as usize);
    libc::FD_CLR(fd, &mut manager.read_set);
    libc::FD_CLR(fd, &mut manager.write_set);
    libc::FD_CLR(fd, &mut manager.exception_set);
    manager.descriptors[fd as usize] = 0;
    if fd as usize + 1 == manager.max_nd {
        while manager.max_nd > 0 && manager.descriptors[manager.max_nd - 1] == 0 {
            manager.max_nd -= 1;
        }
    }
}

macro_rules! activation {
    ($name:ident, $field:ident, $op:ident) => {
        /// # Safety
        /// `nd` must be registered with an open socket.
        #[no_mangle]
        pub unsafe extern "C" fn $name(nd: *mut NetDescriptor) {
            let fd = (*(*nd).socket).fd;
            libc::$op(fd, &mut MANAGER.lock().$field);
        }
    };
}
activation!(NetManager_activateReadCallback, read_set, FD_SET);
activation!(NetManager_deactivateReadCallback, read_set, FD_CLR);
activation!(NetManager_activateWriteCallback, write_set, FD_SET);
activation!(NetManager_deactivateWriteCallback, write_set, FD_CLR);
activation!(NetManager_activateExceptionCallback, exception_set, FD_SET);
activation!(
    NetManager_deactivateExceptionCallback,
    exception_set,
    FD_CLR
);

/// # Safety
/// `timeout_ms` must be writable. Calls must be serialized on the network event thread;
/// callbacks may reenter this function, as in the C implementation.
#[no_mangle]
pub unsafe extern "C" fn NetManager_process(timeout_ms: *mut u32) -> c_int {
    let mut timeout = libc::timeval {
        tv_sec: (*timeout_ms / 1000).into(),
        tv_usec: ((*timeout_ms % 1000) * 1000) as _,
    };
    let (max_nd, mut reads, mut writes, mut exceptions) = {
        let manager = MANAGER.lock();
        (
            manager.max_nd,
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
    let mut i = 0;
    while i < MANAGER.lock().max_nd && result != 0 {
        // Look up each entry after earlier callbacks, which can remove or replace descriptors.
        let nd = MANAGER.lock().descriptors[i] as *mut NetDescriptor;
        if !nd.is_null() {
            let fd = i as c_int;
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
        i += 1;
    }
    *timeout_ms = timeout.tv_sec as u32 * 1000 + timeout.tv_usec as u32 / 1000;
    0
}
