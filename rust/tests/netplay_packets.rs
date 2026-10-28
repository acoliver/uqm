//! Production-linked packet and checksum handlers.
extern crate uqm_rust;

use abi::*;
use uqm_rust::engine::battle::net::BattleNet;
use uqm_rust::supermelee::netplay::{abi, NetplaySession};
use uqm_rust::supermelee::netplay::{packet::Packet, packethandlers::dispatch};

extern "C" {
    fn PacketQueue_init(queue: *mut PacketQueue);
    fn PacketQueue_uninit(queue: *mut PacketQueue);
    fn ChecksumBuffer_init(buffer: *mut ChecksumBuffer, delay: usize, interval: usize);
    fn ChecksumBuffer_uninit(buffer: *mut ChecksumBuffer);
    fn ChecksumBuffer_addChecksum(buffer: *mut ChecksumBuffer, frame: u32, checksum: u32) -> bool;
    fn ChecksumBuffer_getChecksum(
        buffer: *mut ChecksumBuffer,
        frame: u32,
        checksum: *mut u32,
    ) -> bool;
}

unsafe fn connection(session: &NetplaySession, state: NetState) -> Box<NetConnection> {
    let mut conn: Box<NetConnection> = Box::new(std::mem::zeroed());
    conn.session = &**session;
    conn.state = state;
    conn.stateFlags.connected = true;
    PacketQueue_init(&raw mut conn.queue);
    conn
}

#[test]
fn checksum_handler_decodes_and_discards_outside_its_frame_window() {
    unsafe {
        let session = NetplaySession::default();
        session.set_battle_input_delay(3);
        session.set_frame(100);
        let mut conn = connection(&session, NetState_NetState_inBattle);
        conn.stateFlags.checksumInterval = 2;
        ChecksumBuffer_init(&raw mut conn.checksumBuffer, 3, 2);
        ChecksumBuffer_addChecksum(&raw mut conn.checksumBuffer, 100, 0xaaaa);
        for (frame, checksum) in [
            (98, 0x12345678),
            (104, 0xabcdef01),
            (106, 0xdead),
            (96, 0xdead),
            (101, 0xdead),
        ] {
            let p = Packet::Checksum { frame, checksum };
            assert_eq!(dispatch(&mut *conn, &p), 0);
        }
        for (frame, expected) in [(98, 0x12345678), (104, 0xabcdef01), (100, 0xaaaa)] {
            let mut result = 0;
            assert!(ChecksumBuffer_getChecksum(
                &raw mut conn.checksumBuffer,
                frame,
                &mut result
            ));
            assert_eq!(result, expected);
        }
        ChecksumBuffer_uninit(&raw mut conn.checksumBuffer);
        PacketQueue_uninit(&raw mut conn.queue);
    }
}

#[test]
fn team_name_requires_room_for_its_terminator() {
    unsafe {
        let session = NetplaySession::default();
        let mut conn = connection(&session, NetState_NetState_inSetup);
        conn.stateFlags.handshake.set_localOk(true);
        let bytes = [0, 6, 0, 5, 0, 0];
        assert_eq!(
            Packet::decode(&bytes),
            Err(uqm_rust::supermelee::netplay::packet::DecodeError::InvalidLength)
        );
        assert!(conn.stateFlags.handshake.localOk());
        assert_eq!(conn.queue.size, 0);
        PacketQueue_uninit(&raw mut conn.queue);
    }
}

// Deliver the production queue bytes through TCP loopback and dispatch with
// the stream receiver's table. Responses remain queued until the next exchange.
unsafe fn exchange(sender: &mut NetConnection, receiver: &mut NetConnection) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut writer = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut reader, _) = listener.accept().unwrap();
    while !sender.queue.first.is_null() {
        let link = sender.queue.first;
        let packet = (*link).packet;
        let wire = (*packet).encode();
        writer.write_all(&wire).unwrap();
        let mut received = vec![0; wire.len()];
        reader.read_exact(&mut received).unwrap();
        let (received, consumed) = Packet::decode(&received).unwrap();
        assert_eq!(consumed, wire.len());
        assert_eq!(dispatch(receiver, &received), 0);
        sender.queue.first = (*link).next;
        sender.queue.size -= 1;
        drop(Box::from_raw(packet));
        libc::free(link.cast());
    }
    sender.queue.end = &raw mut sender.queue.first;
}

unsafe extern "C" fn ready_count(conn: *mut NetConnection, arg: *mut libc::c_void) {
    assert!((*conn).readyCallback.is_none());
    assert!(!(*conn).stateFlags.ready.localReady());
    assert!(!(*conn).stateFlags.ready.remoteReady());
    *arg.cast::<usize>() += 1;
}

unsafe extern "C" fn reset_count(conn: *mut NetConnection, arg: *mut libc::c_void) {
    assert!((*conn).resetCallback.is_none());
    assert!(!(*conn).stateFlags.reset.localReset());
    assert!(!(*conn).stateFlags.reset.remoteReset());
    *arg.cast::<usize>() += 1;
}

#[test]
fn two_peers_confirm_cancel_ready_and_reset_over_loopback() {
    unsafe {
        let session = NetplaySession::default();
        let mut a = connection(&session, NetState_NetState_inSetup);
        let mut b = connection(&session, NetState_NetState_inSetup);
        assert_eq!(Netplay_cancelConfirmation(&mut *a), -1);
        assert_eq!(Netplay_confirm(&mut *a), 0);
        assert_eq!(Netplay_confirm(&mut *a), -1);
        exchange(&mut a, &mut b);
        assert!(b.stateFlags.handshake.remoteOk());
        assert_eq!(Netplay_cancelConfirmation(&mut *a), 0);
        assert_eq!(Netplay_confirm(&mut *a), 0);
        assert_eq!(a.queue.size, 1);
        exchange(&mut a, &mut b);
        assert!(!b.stateFlags.handshake.remoteOk());
        exchange(&mut b, &mut a);
        assert!(!a.stateFlags.handshake.canceling());
        exchange(&mut a, &mut b);
        assert_eq!(Netplay_confirm(&mut *b), 0);
        exchange(&mut b, &mut a);
        assert_eq!(a.state, NetState_NetState_preBattle);
        exchange(&mut a, &mut b);
        assert_eq!(b.state, NetState_NetState_preBattle);

        let mut a_ready = 0usize;
        let mut b_ready = 0usize;
        for remote_first in [false, true] {
            assert!(!Netplay_localReady(
                &mut *a,
                Some(ready_count),
                (&raw mut a_ready).cast(),
                true
            ));
            if remote_first {
                exchange(&mut a, &mut b);
            }
            assert_eq!(
                Netplay_localReady(&mut *b, Some(ready_count), (&raw mut b_ready).cast(), true),
                remote_first
            );
            if !remote_first {
                exchange(&mut a, &mut b);
            }
            exchange(&mut b, &mut a);
        }
        assert_eq!((a_ready, b_ready), (2, 2));

        // inSetup permits reset without invoking battle UI feedback.
        a.state = NetState_NetState_inSetup;
        b.state = NetState_NetState_inSetup;
        let mut a_resets = 0usize;
        let mut b_resets = 0usize;
        for simultaneous in [false, true] {
            Netplay_setResetCallback(&mut *a, Some(reset_count), (&raw mut a_resets).cast());
            Netplay_localReset(&mut *a, NetplayResetReason_ResetReason_manualReset);
            if simultaneous {
                Netplay_localReset(&mut *b, NetplayResetReason_ResetReason_manualReset);
            }
            exchange(&mut a, &mut b);
            assert_eq!(b_resets, usize::from(simultaneous));
            Netplay_setResetCallback(&mut *b, Some(reset_count), (&raw mut b_resets).cast());
            exchange(&mut b, &mut a);
        }
        assert_eq!((a_resets, b_resets), (2, 2));
        PacketQueue_uninit(&raw mut a.queue);
        PacketQueue_uninit(&raw mut b.queue);
    }
}

#[derive(Default)]
struct Lifecycle {
    events: Vec<&'static str>,
}
unsafe extern "C" fn connected_test(conn: *mut NetConnection) {
    assert_eq!((*conn).state, NetState_NetState_init);
    assert!((*conn).stateData.is_null());
    assert!((*conn).stateFlags.connected);
    (*(*conn).extra.cast::<Lifecycle>()).events.push("connect");
    NetConnection_setState(conn, NetState_NetState_inSetup);
}
unsafe extern "C" fn closed_test(conn: *mut NetConnection) {
    assert!(!(*conn).stateFlags.connected);
    assert!((*conn).stateFlags.disconnected);
    assert_eq!(
        (*conn).stateData.is_null(),
        (*conn).state == NetState_NetState_connecting
    );
    let events = &mut *(*conn).extra.cast::<Lifecycle>();
    events
        .events
        .push(if (*conn).state == NetState_NetState_connecting {
            "cancel"
        } else {
            "close"
        });
}
unsafe extern "C" fn deleted_test(conn: *mut NetConnection) {
    assert_eq!((*conn).state, NetState_NetState_unconnected);
    assert_eq!(
        (*conn).stateData.is_null(),
        (*(*conn).extra.cast::<Lifecycle>()).events == ["cancel"]
    );
    (*(*conn).extra.cast::<Lifecycle>()).events.push("delete");
}
unsafe extern "C" fn failed_test(_conn: *mut NetConnection, _error: *const NetConnectionError) {
    panic!("unexpected connection error");
}
#[repr(C)]
struct LifecycleData {
    release: Option<unsafe extern "C" fn(*mut NetConnectionStateData)>,
    events: *mut Lifecycle,
}
unsafe extern "C" fn released_test(data: *mut NetConnectionStateData) {
    let data = Box::from_raw(data.cast::<LifecycleData>());
    (*data.events).events.push("release");
}
unsafe fn attach_data(conn: *mut NetConnection, events: *mut Lifecycle) {
    (*conn).stateData = Box::into_raw(Box::new(LifecycleData {
        release: Some(released_test),
        events,
    }))
    .cast();
}

#[test]
#[serial_test::serial]
fn connection_lifecycle_keeps_callback_order_over_loopback() {
    unsafe {
        Callback_init();
        Alarm_init();
        NetManager_init();
        let session = NetplaySession::default();
        let reserved = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port =
            std::ffi::CString::new(reserved.local_addr().unwrap().port().to_string()).unwrap();
        drop(reserved);
        let server_options = NetplayPeerOptions {
            isServer: true,
            host: c"127.0.0.1".as_ptr(),
            port: port.as_ptr(),
        };
        let client_options = NetplayPeerOptions {
            isServer: false,
            ..server_options
        };
        let mut a_events = Lifecycle::default();
        let mut b_events = Lifecycle::default();
        let a = NetConnection_open(
            0,
            &server_options,
            Some(connected_test),
            Some(closed_test),
            Some(failed_test),
            Some(deleted_test),
            (&raw mut a_events).cast(),
            &*session,
        );
        assert_eq!((*a).state, NetState_NetState_connecting);
        Callback_process();
        let b = NetConnection_open(
            1,
            &client_options,
            Some(connected_test),
            Some(closed_test),
            Some(failed_test),
            Some(deleted_test),
            (&raw mut b_events).cast(),
            &*session,
        );
        session.add_net_connection(a, 0);
        session.add_net_connection(b, 1);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while a_events.events.is_empty() || b_events.events.is_empty() {
            assert!(std::time::Instant::now() < deadline);
            let mut timeout = 20;
            session.net_input_blocking(timeout);
        }
        assert!((*a).stateFlags.discriminant);
        assert!(!(*b).stateFlags.discriminant);
        assert_eq!((*a).state, NetState_NetState_inSetup);
        assert_eq!((*b).state, NetState_NetState_inSetup);
        // Exercise protocols through the production queue, socket sender,
        // stream framing and descriptor read callback on both connections.
        assert_eq!(Netplay_confirm(a), 0);
        assert_eq!(Netplay_confirm(b), 0);
        loop {
            if (*a).state == NetState_NetState_preBattle
                && (*b).state == NetState_NetState_preBattle
            {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            assert_eq!(flushPacketQueue(a), 0);
            assert_eq!(flushPacketQueue(b), 0);
            let mut timeout = 20;
            session.net_input_blocking(timeout);
        }
        let mut ready_a = 0usize;
        let mut ready_b = 0usize;
        assert!(!Netplay_localReady(
            a,
            Some(ready_count),
            (&raw mut ready_a).cast(),
            true
        ));
        assert!(!Netplay_localReady(
            b,
            Some(ready_count),
            (&raw mut ready_b).cast(),
            true
        ));
        loop {
            if ready_a != 0 && ready_b != 0 {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            assert_eq!(flushPacketQueue(a), 0);
            assert_eq!(flushPacketQueue(b), 0);
            let mut timeout = 20;
            session.net_input_blocking(timeout);
        }
        NetConnection_setState(a, NetState_NetState_inSetup);
        NetConnection_setState(b, NetState_NetState_inSetup);
        let mut reset_a = 0usize;
        let mut reset_b = 0usize;
        Netplay_setResetCallback(a, Some(reset_count), (&raw mut reset_a).cast());
        Netplay_setResetCallback(b, Some(reset_count), (&raw mut reset_b).cast());
        Netplay_localReset(a, NetplayResetReason_ResetReason_manualReset);
        loop {
            if reset_a != 0 && reset_b != 0 {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            assert_eq!(flushPacketQueue(a), 0);
            assert_eq!(flushPacketQueue(b), 0);
            let mut timeout = 20;
            session.net_input_blocking(timeout);
        }
        assert!((*a).statistics.packetsReceived > 0);
        assert!((*b).statistics.packetsReceived > 0);
        attach_data(a, &raw mut a_events);
        attach_data(b, &raw mut b_events);
        session.remove_net_connection(0);
        NetConnection_close(a);
        assert_eq!(a_events.events, ["connect", "close", "delete", "release"]);
        while b_events.events.len() == 1 {
            assert!(std::time::Instant::now() < deadline);
            let mut timeout = 20;
            session.net_input_blocking(timeout);
        }
        assert_eq!(b_events.events, ["connect", "close"]);
        assert_eq!((*b).state, NetState_NetState_unconnected);
        session.remove_net_connection(1);
        NetConnection_close(b);
        assert_eq!(b_events.events, ["connect", "close", "delete", "release"]);

        // Closing a pending listener releases its connection state after callbacks.
        let mut cancelled = Lifecycle::default();
        let pending = NetConnection_open(
            0,
            &server_options,
            Some(connected_test),
            Some(closed_test),
            Some(failed_test),
            Some(deleted_test),
            (&raw mut cancelled).cast(),
            &*session,
        );
        NetConnection_close(pending);
        assert_eq!(cancelled.events, ["cancel", "delete"]);
        Callback_process();
        NetManager_uninit();
        Alarm_uninit();
        Callback_uninit();
    }
}

#[test]
#[serial_test::serial]
fn notifications_visit_only_connected_peers() {
    unsafe {
        let session = NetplaySession::default();
        let mut a = connection(&session, NetState_NetState_inBattle);
        let mut b = connection(&session, NetState_NetState_inBattle);
        session.add_net_connection(&mut *a, 0);
        session.add_net_connection(&mut *b, 1);
        b.stateFlags.connected = false;
        session.netplay_notify_all_battle_input(0x23);
        assert_eq!(a.queue.size, 1);
        assert_eq!(b.queue.size, 0);
        assert_eq!(&*(*a.queue.first).packet, &Packet::BattleInput(0x23));
        b.stateFlags.connected = true;
        session.netplay_notify_all_checksum(37, 0x12345678);
        assert_eq!(a.queue.size, 2);
        assert_eq!(b.queue.size, 1);
        assert_eq!(
            &*(*b.queue.first).packet,
            &Packet::Checksum {
                frame: 37,
                checksum: 0x12345678
            }
        );
        session.remove_net_connection(0);
        session.remove_net_connection(1);
        PacketQueue_uninit(&raw mut a.queue);
        PacketQueue_uninit(&raw mut b.queue);
    }
}
