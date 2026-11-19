use super::{abi::*, netconnection::NetConnection, packet::Packet, NetplaySession};
use crate::engine::battle::net::{
    ProtocolDriver, ProtocolNotice, ProtocolNoticeKind, ProtocolPoll,
};
use uqm_engine::{
    host::{Connection, Net},
    network::loopback::LoopbackNet,
};

struct Driver<'a> {
    receive: &'a mut dyn FnMut(ProtocolNotice),
    before_wait: &'a mut dyn FnMut(),
    waits: usize,
}
impl ProtocolDriver for Driver<'_> {
    fn notice(&mut self, notice: ProtocolNotice) {
        (self.receive)(notice);
    }
    fn wait(&mut self, poll: &mut ProtocolPoll<'_>) {
        self.waits += 1;
        (self.before_wait)();
        assert!(!poll(self));
    }
}

fn connected() -> (NetplaySession, LoopbackNet, Connection) {
    let (mut local, mut remote) = LoopbackNet::pair();
    let socket = local.listen("21837").unwrap();
    let peer = remote.connect("localhost", "21837").unwrap();
    let session = NetplaySession::with_net(local);
    let mut conn = NetConnection::new(0, Default::default());
    conn.socket = Some(socket);
    conn.state = NetState_NetState_selectShip;
    conn.flags.connected = true;
    conn.battle_active = true;
    *session.connections[0].borrow_mut() = Some(conn);
    (session, remote, peer)
}

#[test]
fn selection_notice_is_immediate_and_does_not_hold_the_connection() {
    let (session, _, _) = connected();
    let net = session.battle_net();
    let mut available = [[false; 14]; 2];
    available[0][3] = true;
    net.begin_selection(1, available);
    let mut seen = Vec::new();
    let mut receive = |notice: ProtocolNotice| {
        assert_eq!(net.selected(0), Some(3));
        let mut conn = session.connection_mut(0).unwrap();
        assert_eq!(conn.packets_received, 1);
        conn.state = NetState_NetState_interBattle;
        seen.push(notice);
    };
    let mut before_wait = || panic!("selection must not wait");
    let mut driver = Driver {
        receive: &mut receive,
        before_wait: &mut before_wait,
        waits: 0,
    };
    assert_eq!(
        session.dispatch(0, &Packet::SelectShip(3), &mut |event| session
            .effect(event, &mut driver)),
        0
    );
    assert_eq!(
        seen,
        [ProtocolNotice {
            player: 0,
            battle_active: true,
            kind: ProtocolNoticeKind::ShipSelected
        }]
    );
    assert_eq!(
        session.connection(0).unwrap().state,
        NetState_NetState_interBattle
    );
}

#[test]
fn reset_reply_is_flushed_before_the_borrowed_notice_handler() {
    let (session, mut remote, peer) = connected();
    let mut seen = Vec::new();
    let mut receive = |notice: ProtocolNotice| {
        let mut bytes = [0; 8];
        let count = remote.recv(peer, &mut bytes).unwrap();
        assert_eq!(
            Packet::decode(&bytes[..count]).unwrap(),
            (Packet::Reset(1), count)
        );
        assert!(session.connection_mut(0).unwrap().queue.is_empty());
        seen.push(notice.kind);
    };
    let mut before_wait = || panic!("reset must not wait");
    let mut driver = Driver {
        receive: &mut receive,
        before_wait: &mut before_wait,
        waits: 0,
    };
    session.battle_net().reset(1, &mut driver);
    assert_eq!(
        seen,
        [ProtocolNoticeKind::Reset {
            reason: 1,
            remote: false
        }]
    );
}

#[test]
fn ready_exchange_uses_the_supplied_wait_frame() {
    let (session, mut remote, peer) = connected();
    let mut receive = |notice| panic!("ready exchange emitted an unexpected notice: {notice:?}");
    let mut before_wait = || {
        let mut bytes = [0; 8];
        let count = remote.recv(peer, &mut bytes).unwrap();
        assert_eq!(
            Packet::decode(&bytes[..count]).unwrap(),
            (Packet::Ready, count)
        );
        remote.send(peer, &Packet::Ready.encode()).unwrap();
    };
    let mut driver = Driver {
        receive: &mut receive,
        before_wait: &mut before_wait,
        waits: 0,
    };
    assert!(session
        .battle_net()
        .negotiate(true, NetState_NetState_interBattle, &mut driver));
    assert_eq!(driver.waits, 1);
    assert_eq!(
        session.connection(0).unwrap().state,
        NetState_NetState_interBattle
    );
    assert_eq!(session.connection(0).unwrap().ready_generation, 1);
}

#[test]
fn reset_reports_a_failed_reply_before_the_reset_notice() {
    let (session, mut remote, peer) = connected();
    remote.close(peer);
    let mut seen = Vec::new();
    let mut receive = |notice: ProtocolNotice| {
        let state = session.connection_mut(0).map(|conn| conn.state);
        seen.push((notice.kind, state));
    };
    let mut before_wait = || panic!("reset must not wait");
    let mut driver = Driver {
        receive: &mut receive,
        before_wait: &mut before_wait,
        waits: 0,
    };
    session.battle_net().reset(1, &mut driver);
    assert_eq!(
        seen,
        [
            (
                ProtocolNoticeKind::Closed,
                Some(NetState_NetState_selectShip)
            ),
            (
                ProtocolNoticeKind::Reset {
                    reason: 1,
                    remote: false
                },
                None
            ),
        ]
    );
}
