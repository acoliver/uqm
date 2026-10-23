use super::manager::*;
use super::{Callback_add, Socket, Socket_close};
use libc::c_void;
use std::ptr;

pub type NdCallback = Option<unsafe extern "C" fn(*mut NetDescriptor)>;

#[repr(C)]
pub struct NetDescriptor {
    pub(super) closed: bool,
    ref_count: u32,
    pub(super) read_callback: NdCallback,
    pub(super) write_callback: NdCallback,
    pub(super) exception_callback: NdCallback,
    close_callback: NdCallback,
    pub(super) socket: *mut Socket,
    smd: *mut c_void,
    extra: *mut c_void,
}

/// # Safety
/// `socket` must be an open, unregistered socket. Ownership transfers on success.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_new(
    socket: *mut Socket,
    extra: *mut c_void,
) -> *mut NetDescriptor {
    let nd = Box::into_raw(Box::new(NetDescriptor {
        closed: false,
        ref_count: 1,
        read_callback: None,
        write_callback: None,
        exception_callback: None,
        close_callback: None,
        socket,
        smd: ptr::null_mut(),
        extra,
    }));
    if NetManager_addDesc(nd) == -1 {
        drop(Box::from_raw(nd));
        return ptr::null_mut();
    }
    nd
}

/// # Safety
/// `nd` must hold a live reference.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_incRef(nd: *mut NetDescriptor) {
    assert!((*nd).ref_count < u32::MAX);
    (*nd).ref_count += 1;
}

/// # Safety
/// `nd` must hold a live reference. The final reference requires a closed or detached socket.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_decRef(nd: *mut NetDescriptor) -> bool {
    assert!((*nd).ref_count > 0);
    (*nd).ref_count -= 1;
    if (*nd).ref_count == 0 {
        assert!((*nd).socket.is_null() && (*nd).smd.is_null());
        drop(Box::from_raw(nd));
        true
    } else {
        false
    }
}

unsafe fn close_callback(arg: *mut c_void) {
    let nd = arg.cast::<NetDescriptor>();
    if let Some(callback) = (*nd).close_callback {
        callback(nd);
    }
    NetDescriptor_decRef(nd);
}

/// # Safety
/// `nd` must be open. Consumes the owner's reference, after any deferred close callback.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_close(nd: *mut NetDescriptor) {
    assert!(!(*nd).closed && !(*nd).socket.is_null());
    NetManager_removeDesc(nd);
    Socket_close((*nd).socket);
    (*nd).socket = ptr::null_mut();
    (*nd).closed = true;
    if (*nd).close_callback.is_some() {
        // The existing owner reference moves to the deferred callback.
        Callback_add(
            |arg| unsafe { close_callback(arg as *mut c_void) },
            nd as usize,
        );
    } else {
        NetDescriptor_decRef(nd);
    }
}

/// # Safety
/// `nd` must be open. Consumes its reference without closing the socket.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_detach(nd: *mut NetDescriptor) {
    NetManager_removeDesc(nd);
    (*nd).socket = ptr::null_mut();
    (*nd).closed = true;
    NetDescriptor_decRef(nd);
}

/// # Safety
/// `nd` must hold a live reference.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_getSocket(nd: *mut NetDescriptor) -> *mut Socket {
    (*nd).socket
}

/// # Safety
/// `nd` must hold a live reference.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_setExtra(nd: *mut NetDescriptor, extra: *mut c_void) {
    (*nd).extra = extra;
}

/// # Safety
/// `nd` must hold a live reference.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_getExtra(nd: *const NetDescriptor) -> *mut c_void {
    (*nd).extra
}

macro_rules! callback_setter {
    ($name:ident, $field:ident, $activate:ident, $deactivate:ident) => {
        /// # Safety
        /// `nd` must hold a live reference. The callback must obey the descriptor ownership contract.
        #[no_mangle]
        pub unsafe extern "C" fn $name(nd: *mut NetDescriptor, callback: NdCallback) {
            (*nd).$field = callback;
            if !(*nd).closed {
                if callback.is_some() {
                    $activate(nd);
                } else {
                    $deactivate(nd);
                }
            }
        }
    };
}
callback_setter!(
    NetDescriptor_setReadCallback,
    read_callback,
    NetManager_activateReadCallback,
    NetManager_deactivateReadCallback
);
callback_setter!(
    NetDescriptor_setWriteCallback,
    write_callback,
    NetManager_activateWriteCallback,
    NetManager_deactivateWriteCallback
);
callback_setter!(
    NetDescriptor_setExceptionCallback,
    exception_callback,
    NetManager_activateExceptionCallback,
    NetManager_deactivateExceptionCallback
);

/// # Safety
/// `nd` must hold a live reference. The callback must not consume the deferred callback's reference.
#[no_mangle]
pub unsafe extern "C" fn NetDescriptor_setCloseCallback(
    nd: *mut NetDescriptor,
    callback: NdCallback,
) {
    (*nd).close_callback = callback;
}

pub(super) unsafe fn invoke(nd: *mut NetDescriptor, callback: NdCallback) -> bool {
    let callback = callback.expect("ready descriptor must have a callback");
    NetDescriptor_incRef(nd);
    callback(nd);
    let closed = (*nd).closed;
    NetDescriptor_decRef(nd);
    closed
}
