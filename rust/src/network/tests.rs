unsafe fn pair() -> (*mut NetDescriptor, *mut Socket) {
    let mut fds = [0; 2];
    assert_eq!(
        libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()),
        0
    );
    let socket = Box::into_raw(Box::new(Socket { fd: fds[0] }));
    let peer = Box::into_raw(Box::new(Socket { fd: fds[1] }));
    (NetDescriptor_new(socket, ptr::null_mut()), peer)
}

#[test]
#[serial]
fn deferred_close_consumes_exactly_the_owner_reference_and_can_be_cancelled() {
    unsafe {
        init();
        let mut events = Events::default();
        let (nd, peer) = pair();
        NetDescriptor_setExtra(nd, (&mut events as *mut Events).cast());
        NetDescriptor_incRef(nd);
        NetDescriptor_setCloseCallback(nd, Some(closed));
        NetDescriptor_close(nd);
        assert!(events.order.is_empty());
        Callback_process();
        assert_eq!(events.order, ["close"]);
        assert!(NetDescriptor_decRef(nd));
        Socket_close(peer);

        let (nd, peer) = pair();
        NetDescriptor_incRef(nd);
        NetDescriptor_setCloseCallback(nd, Some(closed));
        NetDescriptor_close(nd);
        NetDescriptor_setCloseCallback(nd, None);
        Callback_process();
        assert!(NetDescriptor_decRef(nd));
        Socket_close(peer);
        uninit();
    }
}

unsafe extern "C" fn close_other(nd: *mut NetDescriptor) {
    NetDescriptor_close(NetDescriptor_getExtra(nd).cast());
    NetDescriptor_setWriteCallback(nd, None);
}

#[test]
#[serial]
fn earlier_callback_can_remove_a_later_ready_descriptor() {
    unsafe {
        init();
        let a = pair();
        let b = pair();
        let ((first, first_peer), (second, second_peer)) =
            if (*NetDescriptor_getSocket(a.0)).fd < (*NetDescriptor_getSocket(b.0)).fd {
                (a, b)
            } else {
                (b, a)
            };
        NetDescriptor_setExtra(first, second.cast());
        NetDescriptor_setWriteCallback(first, Some(close_other));
        NetDescriptor_setWriteCallback(second, Some(writable));
        let mut timeout = 0;
        assert_eq!(NetManager_process(&mut timeout), 0);
        NetDescriptor_close(first);
        Socket_close(first_peer);
        Socket_close(second_peer);
        uninit();
    }
}

#[test]
#[serial]
fn detach_preserves_socket_and_out_of_range_fds_are_rejected() {
    unsafe {
        init();
        let (nd, peer) = pair();
        let socket = NetDescriptor_getSocket(nd);
        NetDescriptor_detach(nd);
        let mut timeout = 0;
        assert_eq!(NetManager_process(&mut timeout), 0);
        assert_ne!(libc::fcntl((*socket).fd, libc::F_GETFD), -1);
        Socket_close(socket);
        Socket_close(peer);
        let mut invalid = Socket {
            fd: libc::FD_SETSIZE as c_int,
        };
        assert!(NetDescriptor_new(&mut invalid, ptr::null_mut()).is_null());
        assert_eq!(last_errno(), libc::EMFILE);
        uninit();
    }
}

#[test]
fn family_split_empty_partition_keeps_a_writable_tail() {
    unsafe {
        let mut info: libc::addrinfo = std::mem::zeroed();
        info.ai_family = libc::AF_INET;
        let mut selected = ptr::null_mut();
        let mut selected_end = ptr::null_mut();
        let mut rest = ptr::null_mut();
        let mut rest_end = ptr::null_mut();
        splitAddrInfoOnFamily(
            &mut info,
            libc::AF_INET6,
            &mut selected,
            &mut selected_end,
            &mut rest,
            &mut rest_end,
        );
        assert!(selected.is_null());
        assert_eq!(selected_end, &mut selected as *mut _);
        assert_eq!(rest, &mut info as *mut _);
        *selected_end = rest;
        assert_eq!(selected, &mut info as *mut _);
        assert_eq!(rest_end, &mut info.ai_next as *mut _);
    }
}

#[test]
#[serial]
fn per_address_timeout_reports_all_failed_without_retry() {
    unsafe {
        init();
        let mut events = Events::default();
        let flags = ConnectFlags {
            family_demand: 1,
            family_prefer: 1,
            timeout: 0,
            retry_delay_ms: -1,
        };
        connectHostByName(
            c"127.0.0.1".as_ptr(),
            c"1".as_ptr(),
            0,
            &flags,
            Some(connected),
            Some(connect_error),
            (&mut events as *mut Events).cast(),
        );
        Callback_process();
        while crate::callback::ffi::Alarm_processOne() {}
        assert_eq!(events.error, Some((2, libc::ETIMEDOUT)));
        assert!(events.connected.is_null());
        uninit();
    }
}

#[test]
#[serial]
fn exhausted_addresses_schedule_retry_and_close_cancels_it() {
    unsafe {
        init();
        let mut events = Events::default();
        let flags = ConnectFlags {
            family_demand: 1,
            family_prefer: 0,
            timeout: 0,
            retry_delay_ms: 0,
        };
        let state = connectHostByName(
            c"127.0.0.1".as_ptr(),
            c"1".as_ptr(),
            0,
            &flags,
            Some(connected),
            Some(connect_error),
            (&mut events as *mut Events).cast(),
        );
        Callback_process();
        assert!(crate::callback::ffi::Alarm_processOne());
        assert_eq!(events.error, None);
        assert!(crate::callback::ffi::Alarm_processOne());
        assert_eq!(events.error, None);
        ConnectState_close(state);
        while crate::callback::ffi::Alarm_processOne() {}
        assert!(events.order.is_empty());
        uninit();
    }
}

use super::connect::*;
use super::descriptor::*;
use super::listen::*;
use super::manager::*;
use super::*;
use crate::callback::ffi::{
    Alarm_init, Alarm_uninit, Callback_init, Callback_process, Callback_uninit,
};
use serial_test::serial;

#[derive(Default)]
struct Events {
    connected: *mut NetDescriptor,
    accepted: *mut NetDescriptor,
    order: Vec<&'static str>,
    error: Option<(c_int, c_int)>,
}

unsafe extern "C" fn connected(
    state: *mut ConnectState,
    nd: *mut NetDescriptor,
    addr: *const libc::sockaddr,
    len: socklen_t,
) {
    assert!(!addr.is_null());
    assert!(len as usize >= std::mem::size_of::<libc::sockaddr_in>());
    let events = &mut *ConnectState_getExtra(state).cast::<Events>();
    events.connected = nd;
    events.order.push("connected");
    ConnectState_close(state);
}

unsafe extern "C" fn accepted(
    state: *mut ListenState,
    _listener: *mut NetDescriptor,
    nd: *mut NetDescriptor,
    addr: *const libc::sockaddr,
    _len: socklen_t,
) {
    assert_eq!((*addr).sa_family as c_int, libc::AF_INET);
    let events = &mut *ListenState_getExtra(state).cast::<Events>();
    events.accepted = nd;
    events.order.push("accepted");
    ListenState_close(state);
}

unsafe extern "C" fn connect_error(state: *mut ConnectState, error: *const ConnectError) {
    let events = &mut *ConnectState_getExtra(state).cast::<Events>();
    events.error = Some(((*error).state, (*error).err));
    ConnectState_close(state);
}

unsafe extern "C" fn listen_error(state: *mut ListenState, error: *const ListenError) {
    let events = &mut *ListenState_getExtra(state).cast::<Events>();
    events.error = Some(((*error).state, (*error).err));
    ListenState_close(state);
}

unsafe extern "C" fn readable(nd: *mut NetDescriptor) {
    let events = &mut *NetDescriptor_getExtra(nd).cast::<Events>();
    let mut byte = 0u8;
    assert_eq!(
        Socket_recv(
            NetDescriptor_getSocket(nd),
            (&mut byte as *mut u8).cast(),
            1,
            0
        ),
        1
    );
    assert_eq!(byte, 42);
    events.order.push("read");
    NetDescriptor_close(nd);
}

unsafe extern "C" fn writable(nd: *mut NetDescriptor) {
    let events = &mut *NetDescriptor_getExtra(nd).cast::<Events>();
    events.order.push("write");
    NetDescriptor_setWriteCallback(nd, None);
}

unsafe extern "C" fn closed(nd: *mut NetDescriptor) {
    assert!(NetDescriptor_getSocket(nd).is_null());
    let events = &mut *NetDescriptor_getExtra(nd).cast::<Events>();
    events.order.push("close");
}

fn init() {
    Callback_init();
    Callback_uninit();
    Alarm_init();
    NetManager_init();
}

fn uninit() {
    Callback_process();
    NetManager_uninit();
    Alarm_uninit();
    Callback_uninit();
}

#[test]
#[serial]
fn async_loopback_connect_accept_read_write_and_deferred_close() {
    unsafe {
        init();
        let mut events = Events::default();
        let extra = (&mut events as *mut Events).cast();
        let listen_flags = ListenFlags {
            family_demand: 1,
            family_prefer: 0,
            backlog: 4,
        };
        let listener = listenPort(
            c"0".as_ptr(),
            0,
            &listen_flags,
            Some(accepted),
            Some(listen_error),
            extra,
        );
        assert_eq!((*listener).state, 1);
        Callback_process();
        assert_eq!((*listener).state, 2);
        assert_eq!((*listener).num_nd, 1);
        let listener_nd = *(*listener).nds;
        let mut address: libc::sockaddr_in = std::mem::zeroed();
        let mut len = std::mem::size_of_val(&address) as socklen_t;
        assert_eq!(
            libc::getsockname(
                (*NetDescriptor_getSocket(listener_nd)).fd,
                (&mut address as *mut libc::sockaddr_in).cast(),
                &mut len
            ),
            0
        );
        let service = std::ffi::CString::new(u16::from_be(address.sin_port).to_string()).unwrap();
        let flags = ConnectFlags {
            family_demand: 1,
            family_prefer: 0,
            timeout: 1000,
            retry_delay_ms: -1,
        };
        let state = connectHostByName(
            c"127.0.0.1".as_ptr(),
            service.as_ptr(),
            0,
            &flags,
            Some(connected),
            Some(connect_error),
            extra,
        );
        assert_eq!((*state).state, 1);
        assert!(events.connected.is_null());
        Callback_process();
        let listener_fd = (*NetDescriptor_getSocket(listener_nd)).fd;
        let client_fd = (*NetDescriptor_getSocket((*state).nd)).fd;
        for (fd, interest) in [(listener_fd, libc::POLLIN), (client_fd, libc::POLLOUT)] {
            let mut ready = libc::pollfd {
                fd,
                events: interest,
                revents: 0,
            };
            assert_eq!(libc::poll(&mut ready, 1, 1000), 1);
        }
        let mut timeout = 100;
        assert_eq!(NetManager_process(&mut timeout), 0);
        assert_eq!(events.error, None);
        assert!(!events.connected.is_null() && !events.accepted.is_null());
        let mut expected = if listener_fd < client_fd {
            vec!["accepted", "connected"]
        } else {
            vec!["connected", "accepted"]
        };
        assert_eq!(events.order, expected);
        NetDescriptor_setExtra(events.accepted, extra);
        NetDescriptor_setWriteCallback(events.accepted, Some(writable));
        NetDescriptor_setReadCallback(events.accepted, Some(readable));
        NetDescriptor_setCloseCallback(events.accepted, Some(closed));
        let byte = 42u8;
        assert_eq!(
            Socket_send(
                NetDescriptor_getSocket(events.connected),
                (&byte as *const u8).cast(),
                1,
                0
            ),
            1
        );
        let mut ready = libc::pollfd {
            fd: (*NetDescriptor_getSocket(events.accepted)).fd,
            events: libc::POLLIN,
            revents: 0,
        };
        assert_eq!(libc::poll(&mut ready, 1, 1000), 1);
        let mut timeout = 100;
        assert_eq!(NetManager_process(&mut timeout), 0);
        expected.extend(["write", "read"]);
        assert_eq!(events.order, expected);
        Callback_process();
        expected.push("close");
        assert_eq!(events.order, expected);
        NetDescriptor_close(events.connected);
        uninit();
    }
}

#[test]
#[serial]
fn cancelling_before_resolution_removes_queued_callbacks() {
    unsafe {
        init();
        let mut events = Events::default();
        let extra = (&mut events as *mut Events).cast();
        let flags = ConnectFlags {
            family_demand: 1,
            family_prefer: 0,
            timeout: 10,
            retry_delay_ms: -1,
        };
        let state = connectHostByName(
            c"127.0.0.1".as_ptr(),
            c"1".as_ptr(),
            0,
            &flags,
            Some(connected),
            Some(connect_error),
            extra,
        );
        ConnectState_close(state);
        let flags = ListenFlags {
            family_demand: 1,
            family_prefer: 0,
            backlog: 1,
        };
        let state = listenPort(
            c"0".as_ptr(),
            0,
            &flags,
            Some(accepted),
            Some(listen_error),
            extra,
        );
        ListenState_close(state);
        Callback_process();
        assert!(events.order.is_empty());
        assert_eq!(events.error, None);
        uninit();
    }
}

#[test]
#[serial]
fn resolver_failures_report_resolving_state() {
    unsafe {
        init();
        let mut events = Events::default();
        let extra = (&mut events as *mut Events).cast();
        let flags = ConnectFlags {
            family_demand: 1,
            family_prefer: 0,
            timeout: 10,
            retry_delay_ms: -1,
        };
        connectHostByName(
            c"127.0.0.1".as_ptr(),
            c"not-a-valid-service".as_ptr(),
            0,
            &flags,
            Some(connected),
            Some(connect_error),
            extra,
        );
        Callback_process();
        assert_eq!(events.error.map(|e| e.0), Some(1));
        events.error = None;
        let flags = ListenFlags {
            family_demand: 1,
            family_prefer: 0,
            backlog: 1,
        };
        listenPort(
            c"not-a-valid-service".as_ptr(),
            0,
            &flags,
            Some(accepted),
            Some(listen_error),
            extra,
        );
        Callback_process();
        assert_eq!(events.error.map(|e| e.0), Some(1));
        uninit();
    }
}
