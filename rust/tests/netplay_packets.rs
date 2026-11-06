//! Production-linked protocol transitions and transport ownership.
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{self, Read, Write},
    rc::Rc,
};
use uqm_engine::{
    host::{Connection, Net},
    network::loopback::LoopbackNet,
};
use uqm_rust::engine::battle::net::{BattleNet, ProtocolDriver, ProtocolNotice, ProtocolPoll};
use uqm_rust::supermelee::netplay::{
    abi::*,
    checkbuf::ChecksumBuffer,
    netconnection::{NetConnection, PeerOptions, ReadyAction},
    packet::{DecodeError, Packet},
    Effect, Event, NetplaySession,
};

#[derive(Default)]
struct Driver {
    allow_wait: bool,
    admissions: usize,
}
impl ProtocolDriver for Driver {
    fn notice(&mut self, notice: ProtocolNotice) {
        panic!("unexpected protocol notice: {notice:?}");
    }
    fn wait(&mut self, poll: &mut ProtocolPoll<'_>) {
        assert!(self.allow_wait, "premature protocol wait");
        self.admissions += 1;
        assert!(!poll(self));
    }
}

fn prepared(state: u32) -> NetplaySession {
    let session = NetplaySession::with_net(LoopbackNet::default());
    assert!(session.open_player_network_connection(0, &mut |_| panic!("open failed")));
    {
        let mut conn = session.connection_mut(0).unwrap();
        conn.state = state;
        conn.flags.connected = true;
    }
    session
}
fn dispatch(session: &NetplaySession, packet: Packet) -> i32 {
    session.dispatch(0, &packet, &mut |_| {})
}
fn queued_type(session: &NetplaySession) -> usize {
    session.connection(0).unwrap().queue.front().unwrap().kind()
}
fn exchange(sender: &NetplaySession, receiver: &NetplaySession) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|error| panic!("listen: {error}"));
    let address = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("local address: {error}"));
    let mut writer =
        std::net::TcpStream::connect(address).unwrap_or_else(|error| panic!("connect: {error}"));
    let (mut reader, _) = listener
        .accept()
        .unwrap_or_else(|error| panic!("accept: {error}"));
    let mut wire = Vec::new();
    sender
        .connection_mut(0)
        .unwrap()
        .queue
        .flush(|bytes| {
            wire.extend_from_slice(bytes);
            Ok(bytes.len())
        })
        .unwrap();
    writer
        .write_all(&wire)
        .unwrap_or_else(|error| panic!("write: {error}"));
    let mut received = vec![0; wire.len()];
    reader
        .read_exact(&mut received)
        .unwrap_or_else(|error| panic!("read: {error}"));
    while !received.is_empty() {
        let (packet, count) = Packet::decode(&received).unwrap();
        assert_eq!(dispatch(receiver, packet), 0);
        received.drain(..count);
    }
}
#[test]
fn checksum_handler_decodes_and_discards_outside_its_frame_window() {
    let session = prepared(NetState_NetState_inBattle);
    session.set_battle_input_delay(3);
    session.set_frame(100);
    {
        let mut conn = session.connection_mut(0).unwrap();
        conn.flags.checksum_interval = 2;
        conn.checksums = ChecksumBuffer::new(3, 2);
        conn.checksums.add(100, 0xaaaa);
    }
    for (frame, checksum) in [
        (98, 0x12345678),
        (104, 0xabcdef01),
        (106, 0xdead),
        (96, 0xdead),
        (101, 0xdead),
    ] {
        assert_eq!(dispatch(&session, Packet::Checksum { frame, checksum }), 0);
    }
    for (frame, expected) in [(98, 0x12345678), (104, 0xabcdef01), (100, 0xaaaa)] {
        assert_eq!(
            session.connection(0).unwrap().checksums.get(frame),
            expected
        );
    }
}
#[test]
fn team_name_requires_room_for_its_terminator() {
    let session = prepared(NetState_NetState_inSetup);
    session.connection_mut(0).unwrap().flags.handshake.local = true;
    assert_eq!(
        Packet::decode(&[0, 6, 0, 5, 0, 0]),
        Err(DecodeError::InvalidLength)
    );
    let conn = session.connection(0).unwrap();
    assert!(conn.flags.handshake.local);
    assert!(conn.queue.is_empty());
}
#[test]
fn two_peers_confirm_cancel_ready_and_reset_over_loopback() {
    let a = prepared(NetState_NetState_inSetup);
    let b = prepared(NetState_NetState_inSetup);
    assert!(!a.connection_mut(0).unwrap().cancel_confirmation());
    assert!(a.connection_mut(0).unwrap().confirm());
    assert!(!a.connection_mut(0).unwrap().confirm());
    exchange(&a, &b);
    assert!(b.connection(0).unwrap().flags.handshake.remote);
    assert!(a.connection_mut(0).unwrap().cancel_confirmation());
    assert!(a.connection_mut(0).unwrap().confirm());
    assert_eq!(a.connection(0).unwrap().queue.len(), 1);
    exchange(&a, &b);
    assert!(!b.connection(0).unwrap().flags.handshake.remote);
    exchange(&b, &a);
    assert!(!a.connection(0).unwrap().flags.handshake.canceling);
    exchange(&a, &b);
    assert!(b.connection_mut(0).unwrap().confirm());
    exchange(&b, &a);
    assert_eq!(a.connection(0).unwrap().state, NetState_NetState_preBattle);
    exchange(&a, &b);
    assert_eq!(b.connection(0).unwrap().state, NetState_NetState_preBattle);
    for remote_first in [false, true] {
        a.connection_mut(0)
            .unwrap()
            .local_ready(ReadyAction::Transition(4), true, 0);
        if remote_first {
            exchange(&a, &b);
        }
        let generation = b.connection(0).unwrap().ready_generation;
        b.connection_mut(0)
            .unwrap()
            .local_ready(ReadyAction::Transition(4), true, 0);
        assert_eq!(
            b.connection(0).unwrap().ready_generation != generation,
            remote_first
        );
        if !remote_first {
            exchange(&a, &b);
        }
        exchange(&b, &a);
        for session in [&a, &b] {
            let conn = session.connection(0).unwrap();
            assert!(conn.ready_action.is_none());
            assert!(!conn.flags.ready.local && !conn.flags.ready.remote);
        }
    }
    assert_eq!(a.connection(0).unwrap().ready_generation, 2);
    assert_eq!(b.connection(0).unwrap().ready_generation, 2);
    a.connection_mut(0).unwrap().state = 3;
    b.connection_mut(0).unwrap().state = 3;
    for simultaneous in [false, true] {
        a.connection_mut(0).unwrap().reset_next = Some(3);
        a.connection_mut(0).unwrap().local_reset(2);
        if simultaneous {
            b.connection_mut(0).unwrap().local_reset(2);
        }
        exchange(&a, &b);
        assert_eq!(
            b.connection(0).unwrap().reset_generation,
            usize::from(simultaneous)
        );
        {
            let mut conn = b.connection_mut(0).unwrap();
            conn.reset_next = Some(3);
            conn.reset_complete();
            assert!(conn.reset_next.is_none());
            assert!(!conn.flags.reset.local && !conn.flags.reset.remote);
        }
        exchange(&b, &a);
    }
    assert_eq!(a.connection(0).unwrap().reset_generation, 2);
    assert_eq!(b.connection(0).unwrap().reset_generation, 2);
}
#[test]
fn handshake_order_cancel_and_reconfirmation() {
    let a = prepared(3);
    a.connection_mut(0).unwrap().flags.handshake.local = true;
    assert_eq!(dispatch(&a, Packet::Handshake0), 0);
    assert!(a.connection(0).unwrap().flags.handshake.remote);
    assert_eq!(queued_type(&a), 7);
    assert_eq!(dispatch(&a, Packet::Handshake1), 0);
    assert_eq!(a.connection(0).unwrap().state, 4);
    assert!(!a.connection(0).unwrap().flags.handshake.local);
    assert!(!a.connection(0).unwrap().flags.handshake.remote);
    let a = prepared(3);
    a.connection_mut(0).unwrap().flags.handshake.local = true;
    assert_eq!(dispatch(&a, Packet::Handshake1), 0);
    assert_eq!(queued_type(&a), 7);
    assert_eq!(a.connection(0).unwrap().state, 4);
    for remote in [false, true] {
        let a = prepared(3);
        {
            let mut conn = a.connection_mut(0).unwrap();
            conn.flags.handshake.canceling = true;
            conn.flags.handshake.local = true;
            conn.flags.handshake.remote = remote;
        }
        assert_eq!(dispatch(&a, Packet::HandshakeCancelAck), 0);
        assert!(!a.connection(0).unwrap().flags.handshake.canceling);
        assert_eq!(queued_type(&a), if remote { 7 } else { 6 });
    }
    let a = prepared(3);
    a.connection_mut(0).unwrap().flags.handshake.remote = true;
    assert_eq!(dispatch(&a, Packet::HandshakeCancel), 0);
    assert!(!a.connection(0).unwrap().flags.handshake.remote);
    assert_eq!(queued_type(&a), 9);
}
#[test]
fn reset_gate_and_wrong_state_do_not_change_handshake() {
    let a = prepared(7);
    assert_eq!(dispatch(&a, Packet::Handshake0), -1);
    assert!(!a.connection(0).unwrap().flags.handshake.remote);
    a.connection_mut(0).unwrap().flags.reset.local = true;
    assert_eq!(dispatch(&a, Packet::Handshake0), 0);
    {
        let mut conn = a.connection_mut(0).unwrap();
        conn.flags.reset.local = false;
        conn.flags.reset.remote = true;
    }
    assert_eq!(dispatch(&a, Packet::Handshake0), -1);
    assert!(!a.connection(0).unwrap().flags.handshake.remote);
    assert!(a.connection(0).unwrap().queue.is_empty());
}
#[test]
fn input_delay_decodes_and_rejects_over_limit() {
    let a = prepared(4);
    for delay in [0, 35] {
        assert_eq!(dispatch(&a, Packet::InputDelay(delay)), 0);
        assert_eq!(a.connection(0).unwrap().flags.input_delay, delay as usize);
    }
    assert_eq!(dispatch(&a, Packet::InputDelay(36)), -1);
    assert_eq!(a.connection(0).unwrap().flags.input_delay, 35);
}
#[test]
fn stream_packets_decode_without_native_alignment() {
    for offset in 0..4 {
        let mut storage = [0u8; 16];
        let bytes = &mut storage[offset..offset + 8];
        let a = prepared(4);
        bytes.copy_from_slice(&[0, 8, 0, 11, 0, 0, 0, 17]);
        assert_eq!(dispatch(&a, Packet::decode(bytes).unwrap().0), 0);
        assert_eq!(a.connection(0).unwrap().flags.input_delay, 17);
        bytes.copy_from_slice(&[0, 8, 0, 1, 0x12, 0x34, 0x56, 0x78]);
        assert_eq!(dispatch(&a, Packet::decode(bytes).unwrap().0), 0);
        assert_eq!(
            a.connection(0).unwrap().queue.front(),
            Some(&Packet::Ack(u32::from_ne_bytes([0x12, 0x34, 0x56, 0x78])))
        );
    }
}
#[test]
fn notifications_visit_connected_peers_and_use_relative_sides() {
    let a = prepared(3);
    a.notify_ship(0, 4, 1);
    assert_eq!(
        a.connection(0).unwrap().queue.front(),
        Some(&Packet::Fleet {
            side: 1,
            ships: vec![(4, 1)]
        })
    );
    a.connection_mut(0).unwrap().flags.connected = false;
    a.notify_ship(1, 3, 2);
    assert_eq!(a.connection(0).unwrap().queue.len(), 1);
    a.connection_mut(0).unwrap().flags.connected = true;
    a.connection_mut(0).unwrap().state = 4;
    a.notify_seed(0, 123);
    assert!(a.connection(0).unwrap().flags.random_seed);
    assert_eq!(a.connection(0).unwrap().queue.len(), 2);
}

#[derive(Default)]
struct Script {
    reads: VecDeque<Vec<u8>>,
    recv_calls: usize,
    closes: usize,
    sent: Vec<u8>,
}
struct ScriptNet(Rc<RefCell<Script>>);
impl Net for ScriptNet {
    fn listen(&mut self, _: &str) -> io::Result<Connection> {
        Ok(Connection(1))
    }
    fn connect(&mut self, _: &str, _: &str) -> io::Result<Connection> {
        Ok(Connection(1))
    }
    fn local_port(&self, _: Connection) -> io::Result<u16> {
        Ok(1234)
    }
    fn poll(&mut self, _: Connection) -> io::Result<bool> {
        Ok(true)
    }
    fn send(&mut self, _: Connection, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().sent.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn recv(&mut self, _: Connection, bytes: &mut [u8]) -> io::Result<usize> {
        let mut script = self.0.borrow_mut();
        script.recv_calls += 1;
        let Some(next) = script.reads.pop_front() else {
            return Err(io::ErrorKind::WouldBlock.into());
        };
        assert!(next.len() <= bytes.len());
        bytes[..next.len()].copy_from_slice(&next);
        Ok(next.len())
    }
    fn close(&mut self, _: Connection) {
        self.0.borrow_mut().closes += 1;
    }
}
fn scripted(state: u32, reads: Vec<Vec<u8>>) -> (NetplaySession, Rc<RefCell<Script>>) {
    let script = Rc::new(RefCell::new(Script {
        reads: reads.into(),
        ..Script::default()
    }));
    let session = NetplaySession::with_net(ScriptNet(script.clone()));
    assert!(session.open_player_network_connection(0, &mut |_| panic!("open failed")));
    {
        let mut conn = session.connection_mut(0).unwrap();
        conn.state = state;
        conn.flags.connected = true;
    }
    (session, script)
}
fn init_callbacks() {
    uqm_rust::callback::ffi::Callback_init();
    uqm_rust::callback::ffi::Alarm_init();
}
#[test]
#[serial_test::serial]
fn battle_input_treats_a_removed_peer_as_a_disconnect() {
    init_callbacks();
    let (session, script) = scripted(7, vec![]);
    session.init_battle_input_buffers();
    session.close_player_with(0, &mut |_| {});
    let world = uqm_rust::engine::World::new();
    assert_eq!(
        session.network_battle_input(0, &world.activity_control(), &mut Driver::default()),
        0
    );
    assert_ne!(
        world.activity() & uqm_rust::mainloop::activity_flags::CHECK_ABORT,
        0
    );
    assert_eq!(script.borrow().closes, 1);
}
#[test]
#[serial_test::serial]
fn feedback_can_close_the_connection_before_the_next_packet_or_protocol_error() {
    init_callbacks();
    for invalid_suffix in [false, true] {
        let mut ships = vec![(0, 1)];
        if invalid_suffix {
            ships.push((1, 254));
        }
        let (session, script) = scripted(
            3,
            vec![[
                Packet::Fleet { side: 0, ships }.encode(),
                Packet::TeamName {
                    side: 0,
                    name: b"next packet".to_vec(),
                }
                .encode(),
            ]
            .concat()],
        );
        let mut feedback = 0;
        session.net_input_with(0, &mut |event| {
            assert!(matches!(event.effect, Effect::Ship { index: 0, .. }));
            feedback += 1;
            session.close_player_with(0, &mut |closed| {
                assert_eq!(closed.effect, Effect::Closed);
            });
        });
        assert_eq!(feedback, 1);
        assert!(session.connection(0).is_none());
        assert_eq!(script.borrow().closes, 1);
        assert_eq!(script.borrow().recv_calls, 1);
    }
}
#[test]
#[serial_test::serial]
fn incomplete_first_packet_returns_before_another_read_and_ready_precedes_next_packet() {
    init_callbacks();
    let ready = Packet::Ready.encode();
    let (session, script) = scripted(
        6,
        vec![
            ready[..3].to_vec(),
            [ready[3..].to_vec(), Packet::BattleInput(17).encode()].concat(),
        ],
    );
    session.init_battle_input_buffers();
    session
        .connection_mut(0)
        .unwrap()
        .local_ready(ReadyAction::Transition(7), false, 0);
    session.net_input_with(0, &mut |_| {});
    assert_eq!(script.borrow().recv_calls, 1);
    assert_eq!(session.connection(0).unwrap().read_buffer.len(), 3);
    assert_eq!(session.connection(0).unwrap().state, 6);
    session.net_input_with(0, &mut |_| {});
    assert_eq!(session.connection(0).unwrap().state, 7);
    assert_eq!(session.connection(0).unwrap().packets_received, 2);
    assert_eq!(session.pop_input(0), Some(17));
    assert!(session.connection(0).unwrap().read_buffer.is_empty());
}
#[test]
#[serial_test::serial]
fn setup_effects_are_synchronous_between_packets_and_borrows_end_before_feedback() {
    init_callbacks();
    let (session, _) = scripted(
        3,
        vec![[
            Packet::Fleet {
                side: 0,
                ships: vec![(0, 1)],
            }
            .encode(),
            Packet::Fleet {
                side: 0,
                ships: vec![(1, 2)],
            }
            .encode(),
        ]
        .concat()],
    );
    session.connection_mut(0).unwrap().flags.handshake.local = true;
    let mut observed = Vec::new();
    session.net_input_with(0, &mut |event| {
        let conn = session.connection(0).unwrap();
        assert!(!conn.flags.handshake.local);
        observed.push((conn.packets_received, event.effect));
        drop(conn);
        // This response must be queued before dispatching the next packet.
        session.notify_ship(0, 2, 3);
    });
    assert_eq!(
        observed,
        vec![
            (1, Effect::ConfirmationCancelled),
            (
                1,
                Effect::Ship {
                    side: 0,
                    index: 0,
                    ship: 1
                }
            ),
            (
                2,
                Effect::Ship {
                    side: 0,
                    index: 1,
                    ship: 2
                }
            )
        ]
    );
}
#[test]
fn valid_fleet_prefix_is_applied_even_when_a_later_slot_is_invalid() {
    let session = prepared(3);
    let mut observed = Vec::new();
    assert_eq!(
        session.dispatch(
            0,
            &Packet::Fleet {
                side: 0,
                ships: vec![(2, 1), (14, 3)]
            },
            &mut |event| observed.push(event.effect)
        ),
        -1
    );
    assert_eq!(
        observed,
        vec![Effect::Ship {
            side: 0,
            index: 2,
            ship: 1
        }]
    );
}
#[test]
fn owned_selection_rejects_missing_dead_duplicate_and_out_of_range_choices() {
    let session = prepared(6);
    let mut available = [[false; 14]; 2];
    available[0][3] = true;
    session.begin_selection(1, available);
    assert_eq!(dispatch(&session, Packet::SelectShip(14)), -1);
    assert_eq!(dispatch(&session, Packet::SelectShip(2)), -1);
    assert_eq!(dispatch(&session, Packet::SelectShip(3)), 0);
    assert_eq!(session.selected(0), Some(3));
    assert_eq!(dispatch(&session, Packet::SelectShip(3)), -1);
    session.end_selection();
    assert_eq!(dispatch(&session, Packet::SelectShip(3)), -1);
    session.begin_selection(1, available);
    assert_eq!(dispatch(&session, Packet::SelectShip(u16::MAX)), 0);
    assert_eq!(session.selected(0), Some(u16::MAX));
    session.end_selection();
}
#[test]
#[serial_test::serial]
fn connection_lifecycle_retains_old_state_during_close_and_releases_socket_once() {
    init_callbacks();
    let (session, script) = scripted(7, vec![Vec::new()]);
    let mut events = Vec::new();
    session.net_input_with(0, &mut |event| {
        if event.effect == Effect::Closed {
            let conn = session.connection(0).unwrap();
            assert_eq!(conn.state, 7);
            assert!(!conn.flags.connected && conn.flags.disconnected);
        }
        events.push(event.effect);
    });
    assert_eq!(events, vec![Effect::Closed]);
    assert_eq!(script.borrow().closes, 1);
    assert_eq!(session.connection(0).unwrap().state, 0);
    session.close_player_with(0, &mut |_| panic!("duplicate close"));
    assert!(session.connection(0).is_none());
    drop(session);
    assert_eq!(script.borrow().closes, 1);
    let (session, script) = scripted(1, vec![]);
    session.close_player_with(0, &mut |event| {
        assert_eq!(event.effect, Effect::Closed);
        assert_eq!(session.connection(0).unwrap().state, 1);
    });
    assert!(session.connection(0).is_none());
    assert_eq!(script.borrow().closes, 1);
    let (session, script) = scripted(7, vec![]);
    drop(session);
    assert_eq!(script.borrow().closes, 1);
}
#[test]
#[serial_test::serial]
fn production_tcp_connections_confirm_and_disconnect() {
    init_callbacks();
    let mut session = NetplaySession::default();
    session.options_mut().peer[0].port = "0".into();
    assert!(session.open_player_network_connection(0, &mut |_| panic!("listener error")));
    let socket = session.connection(0).unwrap().socket.unwrap();
    let port = session.local_port(socket).unwrap();
    session.options_mut().peer[1] = PeerOptions {
        server: false,
        host: "127.0.0.1".into(),
        port: port.to_string(),
    };
    assert!(session.open_player_network_connection(1, &mut |_| panic!("client error")));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut events = Vec::new();
    while (0..2).any(|side| session.connection(side).unwrap().state != 3) {
        assert!(std::time::Instant::now() < deadline);
        session.net_input_with(20, &mut |event| {
            if event.effect == Effect::ResetSentTeams {
                let conn = session.connection(event.player).unwrap();
                assert_eq!(conn.state, 2);
                assert!(conn.queue.is_empty());
                assert!(conn.ready_action.is_none());
            }
            events.push(event);
        });
    }
    assert!(session.connection(0).unwrap().flags.discriminant);
    assert!(!session.connection(1).unwrap().flags.discriminant);
    for side in 0..2 {
        assert_eq!(
            events
                .iter()
                .filter(|event| event.player == side)
                .map(|event| &event.effect)
                .collect::<Vec<_>>(),
            [
                &Effect::ResetSentTeams,
                &Effect::Connected,
                &Effect::Bootstrap
            ]
        );
    }
    session.confirm_connections();
    while (0..2).any(|side| session.connection(side).unwrap().state != 4) {
        assert!(std::time::Instant::now() < deadline);
        session.net_input_with(20, &mut |_| {});
    }
    session.close_player_with(0, &mut |event| assert_eq!(event.effect, Effect::Closed));
    loop {
        assert!(std::time::Instant::now() < deadline);
        session.net_input_with(20, &mut |event| assert_eq!(event.effect, Effect::Closed));
        if session.connection(1).unwrap().flags.disconnected {
            break;
        }
    }
    session.close_player_with(1, &mut |_| panic!("duplicate close"));
}
#[test]
fn immediate_open_failure_does_not_retain_a_connection() {
    let mut session = NetplaySession::default();
    session.options_mut().peer[0].port = "invalid service with spaces".into();
    let mut events = Vec::<Event>::new();
    uqm_rust::platform::set_errno(0);
    assert!(!session.open_player_network_connection(0, &mut |event| {
        let Effect::Error(error) = event.effect else {
            panic!("unexpected open feedback {event:?}");
        };
        assert_eq!(uqm_rust::platform::errno(), error);
        events.push(event);
    }));
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].effect, Effect::Error(_)));
    assert!(session.connection(0).is_none());
}
#[test]
fn ready_and_reset_registration_is_cleared_before_a_following_exchange() {
    let mut conn = NetConnection::new(0, PeerOptions::default());
    conn.state = 4;
    conn.flags.connected = true;
    for remote_first in [false, true] {
        if remote_first {
            conn.remote_ready(0);
        }
        conn.local_ready(ReadyAction::Transition(4), false, 0);
        if !remote_first {
            conn.remote_ready(0);
        }
        assert!(conn.ready_action.is_none());
        assert!(!conn.flags.ready.local && !conn.flags.ready.remote);
        conn.reset_next = Some(4);
        conn.local_reset(2);
        conn.remote_reset(2);
        assert!(conn.reset_next.is_none());
        assert!(!conn.flags.reset.local && !conn.flags.reset.remote);
    }
    assert_eq!(conn.ready_generation, 2);
    assert_eq!(conn.reset_generation, 2);
}

#[test]
#[serial_test::serial]
fn battle_end_flushes_frame_count_before_the_second_ready_and_uses_the_borrowed_wait() {
    init_callbacks();
    let (session, script) = scripted(7, vec![]);
    session.set_frame(10);
    assert!(!session.end_ready(0, &mut Driver::default()));
    assert!(
        script.borrow().sent.is_empty(),
        "local ready stays queued until the frame flush"
    );
    assert_eq!(
        session.connection(0).unwrap().queue.front(),
        Some(&Packet::Ready)
    );
    assert_eq!(dispatch(&session, Packet::Ready), 0);
    assert_eq!(
        script.borrow().sent,
        [Packet::Ready.encode(), Packet::FrameCount(11).encode()].concat()
    );
    {
        let conn = session.connection(0).unwrap();
        assert_eq!(conn.state, 8);
        assert_eq!(conn.end_frame_count, 11);
        assert_eq!(conn.ready_action, Some(ReadyAction::EndBattle2));
        assert!(conn.flags.ready.local);
    }
    assert!(!session.end_ready(0, &mut Driver::default()));
    assert_eq!(dispatch(&session, Packet::FrameCount(11)), 0);
    assert_eq!(session.connection(0).unwrap().state, 9);
    session.set_frame(11);
    script.borrow_mut().reads.push_back(Packet::Ready.encode());
    let mut driver = Driver {
        allow_wait: true,
        ..Driver::default()
    };
    assert!(session.end_ready(0, &mut driver));
    assert_eq!(driver.admissions, 1);
    assert_eq!(session.connection(0).unwrap().state, 5);
    assert!(session.connection(0).unwrap().ready_action.is_none());
}

#[test]
fn remote_reset_feedback_precedes_the_registered_transition() {
    let session = prepared(NetState_NetState_inBattle);
    session.connection_mut(0).unwrap().reset_next = Some(NetState_NetState_inSetup);
    let mut observed = false;
    assert_eq!(
        session.dispatch(0, &Packet::Reset(1), &mut |event| {
            assert!(matches!(
                event.effect,
                Effect::Reset {
                    reason: 1,
                    remote: true
                }
            ));
            let conn = session.connection(0).unwrap();
            assert_eq!(conn.state, NetState_NetState_inBattle);
            assert!(conn.flags.reset.local && conn.flags.reset.remote);
            assert_eq!(conn.reset_next, Some(NetState_NetState_inSetup));
            observed = true;
        }),
        0
    );
    assert!(observed);
    let conn = session.connection(0).unwrap();
    assert_eq!(conn.state, NetState_NetState_inSetup);
    assert!(!conn.flags.reset.local && !conn.flags.reset.remote);
    assert_eq!(conn.reset_generation, 1);
    assert_eq!(conn.reset_next, None);
}

#[test]
fn seed_agreement_is_marked_after_synchronous_seed_feedback() {
    let session = prepared(4);
    assert_eq!(
        session.dispatch(0, &Packet::SeedRandom(1729), &mut |event| {
            assert_eq!(event.effect, Effect::Seed(1729));
            assert!(!session.connection(0).unwrap().flags.random_seed);
        }),
        0
    );
    assert!(session.connection(0).unwrap().flags.random_seed);
}
