//! Session-owned connection orchestration and protocol waits.
use super::{
    abi::*,
    netconnection::{Effect, Event, NetConnection},
    packet::Packet,
};
use crate::engine::battle::net::ProtocolDriver;

impl super::session::SessionCore {
    pub fn get_num_net_connections(&self) -> usize {
        self.connections
            .iter()
            .filter(|conn| conn.borrow().is_some())
            .count()
    }
    pub fn each(&self, mut action: impl FnMut(&mut NetConnection) -> bool) -> bool {
        for player in 0..2 {
            if let Some(mut conn) = self.connection_mut(player) {
                if conn.flags.connected && !action(&mut conn) {
                    return false;
                }
            }
        }
        true
    }
    pub fn open_player_network_connection(&self, player: u16, emit: &mut dyn FnMut(Event)) -> bool {
        let player = usize::from(player);
        assert!(self.connections[player].borrow().is_none());
        let mut conn = NetConnection::new(player, self.options.peer[player].clone());
        conn.state = NetState_NetState_connecting;
        let result = if conn.options.server {
            self.transport.borrow_mut().listen(&conn.options.port)
        } else {
            self.transport
                .borrow_mut()
                .connect(&conn.options.host, &conn.options.port)
        };
        match result {
            Ok(socket) => conn.socket = Some(socket),
            Err(error) => {
                let error = error.raw_os_error().unwrap_or(libc::EIO);
                super::set_errno(error);
                emit(conn.event(Effect::Error(error)));
                return false;
            }
        }
        *self.connections[player].borrow_mut() = Some(conn);
        true
    }
    pub fn close_player_with(&self, player: usize, emit: &mut dyn FnMut(Event)) {
        self.disconnect(player, emit);
        self.connections[player]
            .borrow_mut()
            .take()
            .unwrap_or_else(|| panic!("close absent connection"));
    }
    pub fn close_player_network_connection(&self, player: u16, emit: &mut dyn FnMut(Event)) {
        self.close_player_with(usize::from(player), emit);
    }
    pub fn close_all_connections(&self, emit: &mut dyn FnMut(Event)) {
        for player in 0..2 {
            if self.connection(player).is_some() {
                self.close_player_network_connection(player as u16, emit);
            }
        }
    }
    pub fn close_disconnected_connections(&self, emit: &mut dyn FnMut(Event)) {
        for player in 0..2 {
            if self
                .connection(player)
                .is_some_and(|conn| !conn.flags.connected)
            {
                self.close_player_network_connection(player as u16, emit);
            }
        }
    }
    pub(super) fn disconnect(&self, player: usize, emit: &mut dyn FnMut(Event)) {
        let event = {
            let mut conn = self
                .connection_mut(player)
                .unwrap_or_else(|| panic!("disconnect absent connection"));
            if let Some(socket) = conn.socket.take() {
                self.transport.borrow_mut().close(socket);
            }
            if conn.flags.disconnected {
                return;
            }
            conn.flags.connected = false;
            conn.flags.disconnected = true;
            conn.event(Effect::Closed)
        };
        emit(event);
        self.connection_mut(player)
            .unwrap_or_else(|| panic!("disconnected connection"))
            .state = NetState_NetState_unconnected;
    }
    pub(super) fn flush_player(&self, player: usize) -> std::io::Result<()> {
        let Some(mut conn) = self.connection_mut(player) else {
            return Ok(());
        };
        if !conn.flags.connected {
            return Ok(());
        }
        let socket = conn
            .socket
            .unwrap_or_else(|| panic!("send without transport"));
        conn.queue
            .flush(|bytes| self.transport.borrow_mut().send(socket, bytes))
    }
    pub fn flush_packet_queues(&self, emit: &mut dyn FnMut(Event)) {
        for player in 0..2 {
            if let Err(error) = self.flush_player(player) {
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    self.close_player_network_connection(player as u16, emit);
                }
            }
        }
    }
    pub fn confirm_connections(&self) {
        self.each(|conn| {
            conn.confirm();
            true
        });
    }
    pub fn cancel_confirmations(&self) {
        self.each(|conn| {
            conn.cancel_confirmation();
            true
        });
    }
    pub fn all_connected(&self) -> bool {
        (0..2).all(|player| {
            self.connection(player)
                .is_none_or(|conn| conn.flags.connected)
        })
    }
    pub fn init_battle_state_data_connections(&self) {
        for player in 0..2 {
            if let Some(mut conn) = self.connection_mut(player) {
                conn.end_frame_count = 0;
            }
        }
    }
    pub(super) fn set_battle_active_connections(&self, active: bool) {
        for player in 0..2 {
            if let Some(mut conn) = self.connection_mut(player) {
                conn.battle_active = active;
            }
        }
    }
    pub fn setup_input_delay(&self, local_delay: usize) -> bool {
        let mut network = false;
        let mut delay = 0;
        self.each(|conn| {
            network = true;
            delay = delay.max(conn.flags.input_delay);
            true
        });
        self.set_battle_input_delay(if network {
            delay.max(local_delay)
        } else {
            delay
        });
        true
    }
    pub fn set_state_connections(&self, state: NetState) -> bool {
        self.each(|conn| {
            conn.state = state;
            true
        })
    }
    pub fn reset_connections(&self, reason: u32, emit: &mut dyn FnMut(Event)) -> bool {
        for player in 0..2 {
            let events = self
                .connection_mut(player)
                .filter(|conn| conn.flags.connected)
                .map(|mut conn| conn.local_reset(reason))
                .unwrap_or_default();
            self.apply_events(events, emit);
        }
        true
    }
    pub fn network_battle_input(
        &self,
        player: usize,
        activity: &crate::engine::world::activity::ActivityControl,
        driver: &mut dyn ProtocolDriver,
    ) -> u8 {
        use crate::mainloop::activity_flags::CHECK_ABORT;
        loop {
            if let Some(input) = self.pop_input(player) {
                return input;
            }
            self.with_notices(driver, |emit| self.net_input_with(0, emit));
            if let Some(input) = self.pop_input(player) {
                return input;
            }
            if !self
                .connection(player)
                .is_some_and(|conn| conn.flags.connected)
            {
                activity.raise_flags(CHECK_ABORT);
                return 0;
            }
            if activity.activity() & CHECK_ABORT != 0 {
                return 0;
            }
            self.with_notices(driver, |emit| self.net_input_blocking(500, emit));
            if !self
                .connection(player)
                .is_some_and(|conn| conn.flags.connected)
            {
                activity.raise_flags(CHECK_ABORT);
                return 0;
            }
        }
    }
    pub fn queue_all(&self, packet: Packet, setup_only: bool) {
        self.each(|conn| {
            if !setup_only || conn.state == NetState_NetState_inSetup {
                conn.queue(packet.clone());
            }
            true
        });
    }
}
