//! Setup notifications borrow values, never menu or team storage.
use super::{abi::*, netconnection::NetConnection, packet::Packet, session::SessionCore};

fn side(conn: &NetConnection, player: usize) -> u8 {
    u8::from(player == conn.player)
}
impl SessionCore {
    fn notify_setup(&self, mut packet: impl FnMut(&NetConnection) -> Packet) {
        self.each(|conn| {
            if conn.state == NetState_NetState_inSetup {
                assert!(!conn.flags.handshake.local);
                conn.queue(packet(conn));
            }
            true
        });
    }
    pub fn notify_ship(&self, player: usize, index: usize, ship: u32) {
        self.notify_setup(|conn| Packet::Fleet {
            side: side(conn, player),
            ships: vec![(index as u8, ship as u8)],
        });
    }
    pub fn notify_name(&self, player: usize, name: &[u8]) {
        self.notify_setup(|conn| Packet::TeamName {
            side: side(conn, player),
            name: name.to_vec(),
        });
    }
    pub fn notify_fleet(&self, player: usize, ships: &[u32]) {
        self.notify_setup(|conn| Packet::Fleet {
            side: side(conn, player),
            ships: ships
                .iter()
                .enumerate()
                .map(|(slot, ship)| (slot as u8, *ship as u8))
                .collect(),
        });
    }
    pub fn notify_seed(&self, player: usize, seed: u32) {
        let mut conn = self
            .connection_mut(player)
            .unwrap_or_else(|| panic!("seed connection"));
        assert_eq!(conn.state, NetState_NetState_preBattle);
        conn.queue(Packet::SeedRandom(seed));
        conn.flags.random_seed = true;
    }
}
