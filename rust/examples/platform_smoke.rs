//! Exercise the native socket backend without SDL, user configuration or saves.

use std::ffi::{c_void, CString};
use std::io::Read;
use uqm_rust::callback::ffi::{
    Alarm_init, Alarm_uninit, Callback_init, Callback_process, Callback_uninit,
};
use uqm_rust::network::connect::*;
use uqm_rust::network::descriptor::*;
use uqm_rust::network::manager::*;
use uqm_rust::network::{
    self, sockaddr, socklen_t, Socket_close, Socket_open, Socket_recv, Socket_send,
    Socket_setNonBlocking,
};

#[derive(Default)]
struct Connection {
    descriptor: *mut NetDescriptor,
    error: Option<i32>,
}

unsafe extern "C" fn connected(
    state: *mut ConnectState,
    descriptor: *mut NetDescriptor,
    _: *const sockaddr,
    _: socklen_t,
) {
    (*ConnectState_getExtra(state).cast::<Connection>()).descriptor = descriptor;
    ConnectState_close(state);
}

unsafe extern "C" fn failed(state: *mut ConnectState, error: *const ConnectError) {
    (*ConnectState_getExtra(state).cast::<Connection>()).error = Some((*error).err);
    ConnectState_close(state);
}

fn main() {
    network::network_init();
    NetManager_init();
    Callback_init();
    Alarm_init();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let service = CString::new(listener.local_addr().unwrap().port().to_string()).unwrap();
    let mut connection = Connection::default();
    unsafe {
        let flags = ConnectFlags {
            family_demand: 1,
            family_prefer: 1,
            timeout: 5000,
            retry_delay_ms: -1,
        };
        connectHostByName(
            c"127.0.0.1".as_ptr(),
            service.as_ptr(),
            0,
            &flags,
            Some(connected),
            Some(failed),
            (&mut connection as *mut Connection).cast::<c_void>(),
        );
        Callback_process();
        let mut timeout = 5000;
        assert_eq!(NetManager_process(&mut timeout), 0);
        assert_eq!(connection.error, None);
        assert!(!connection.descriptor.is_null());
        let socket = NetDescriptor_getSocket(connection.descriptor);
        assert_eq!(Socket_setNonBlocking(socket), 0);
        let mut byte = 0u8;
        assert_eq!(Socket_recv(socket, (&mut byte as *mut u8).cast(), 1, 0), -1);
        assert_eq!(uqm_rust::platform::errno(), libc::EWOULDBLOCK);
        assert_eq!(Socket_send(socket, b"UQM".as_ptr().cast(), 3, 0), 3);
        let (mut peer, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut received = [0; 3];
        peer.read_exact(&mut received).unwrap();
        assert_eq!(&received, b"UQM");
        NetDescriptor_close(connection.descriptor);

        // Exercise the socket open/close API separately from the async connector.
        let invalid = Socket_open(1, 0, 0);
        assert!(!invalid.is_null());
        assert_eq!(Socket_close(invalid), 0);
        let mut empty_timeout = 0;
        assert_eq!(NetManager_process(&mut empty_timeout), 0);
    }
    Alarm_uninit();
    Callback_uninit();
    NetManager_uninit();
    drop(listener);
    network::network_uninit();
    println!(
        "Platform smoke passed: DNS, nonblocking connect/recv, send, select and socket cleanup"
    );
}
