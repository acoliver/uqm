//! Production-linked packet and checksum handlers.
extern crate uqm_rust;

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    unnecessary_transmutes,
    clippy::all,
    clashing_extern_declarations,
    reason = "generated engine ABI shared with the production netplay bridge"
)]
mod abi {
    include!(concat!(env!("OUT_DIR"), "/netplay_abi.rs"));
}
use abi::*;

#[link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")]
extern "C" {
    fn PacketQueue_init(queue: *mut PacketQueue);
    fn PacketQueue_uninit(queue: *mut PacketQueue);
    fn Packet_Checksum_create(frame: u32, checksum: u32) -> *mut Packet_Checksum;
    fn Packet_delete(packet: *mut Packet);
    fn PacketHandler_Checksum(conn: *mut NetConnection, packet: *const Packet_Checksum) -> i32;
    fn PacketHandler_TeamName(conn: *mut NetConnection, packet: *const Packet_TeamName) -> i32;
    fn ChecksumBuffer_init(buffer: *mut ChecksumBuffer, delay: usize, interval: usize);
    fn ChecksumBuffer_uninit(buffer: *mut ChecksumBuffer);
    fn ChecksumBuffer_addChecksum(buffer: *mut ChecksumBuffer, frame: u32, checksum: u32) -> bool;
    fn ChecksumBuffer_getChecksum(
        buffer: *mut ChecksumBuffer,
        frame: u32,
        checksum: *mut u32,
    ) -> bool;
}

unsafe fn connection(state: NetState) -> Box<NetConnection> {
    let mut conn: Box<NetConnection> = Box::new(std::mem::zeroed());
    conn.state = state;
    conn.stateFlags.connected = true;
    PacketQueue_init(&raw mut conn.queue);
    conn
}

#[test]
fn checksum_handler_decodes_and_discards_outside_its_frame_window() {
    extern "C" {
        fn setBattleInputDelay(delay: usize);
    }
    unsafe {
        let previous_delay = getBattleInputDelay();
        let previous_frame = battleFrameCount;
        setBattleInputDelay(3);
        battleFrameCount = 100;
        let mut conn = connection(NetState_NetState_inBattle);
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
            let p = Packet_Checksum_create(frame, checksum);
            assert_eq!(PacketHandler_Checksum(&mut *conn, p), 0);
            Packet_delete(p.cast());
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
        setBattleInputDelay(previous_delay);
        battleFrameCount = previous_frame;
    }
}

#[test]
fn team_name_requires_room_for_its_terminator() {
    unsafe {
        let mut conn = connection(NetState_NetState_inSetup);
        conn.stateFlags.handshake.set_localOk(true);
        let bytes = [0, 6, 0, 5, 0, 0];
        assert_eq!(
            PacketHandler_TeamName(&mut *conn, bytes.as_ptr().cast()),
            -1
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
        let length = u16::from_be((*packet).header.len) as usize;
        writer
            .write_all(std::slice::from_raw_parts(packet.cast::<u8>(), length))
            .unwrap();
        let mut received = vec![0; length];
        reader.read_exact(&mut received).unwrap();
        let kind = u16::from_be((*packet).header.type_) as usize;
        let handler = (*std::ptr::addr_of!(packetTypeData)
            .cast::<PacketTypeData>()
            .add(kind))
        .handler
        .unwrap();
        assert_eq!(handler(receiver, received.as_ptr().cast()), 0);
        sender.queue.first = (*link).next;
        sender.queue.size -= 1;
        Packet_delete(packet);
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
        let mut a = connection(NetState_NetState_inSetup);
        let mut b = connection(NetState_NetState_inSetup);
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
