//! Session-owned connection orchestration and protocol waits.
use super::{
    abi::*,
    netconnection::{Effect, Event, NetConnection, ReadyAction},
    packet::Packet,
};
use crate::engine::battle::net::ProtocolWait;
const NETWORK_POLL_DELAY: u32 = crate::time::counter_abi::ONE_SECOND / 24;

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
    pub fn close_player_network_connection(&self, player: u16) {
        self.close_player_with(usize::from(player), &mut super::battle::effect);
    }
    pub fn close_all_connections(&self) {
        for player in 0..2 {
            if self.connection(player).is_some() {
                self.close_player_network_connection(player as u16);
            }
        }
    }
    pub fn close_disconnected_connections(&self) {
        for player in 0..2 {
            if self
                .connection(player)
                .is_some_and(|conn| !conn.flags.connected)
            {
                self.close_player_network_connection(player as u16);
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
    pub fn flush_packet_queues(&self) {
        for player in 0..2 {
            if let Err(error) = self.flush_player(player) {
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    self.close_player_network_connection(player as u16);
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
    pub fn reset_connections(&self, reason: u32) -> bool {
        for player in 0..2 {
            let events = self
                .connection_mut(player)
                .filter(|conn| conn.flags.connected)
                .map(|mut conn| conn.local_reset(reason))
                .unwrap_or_default();
            self.apply_events(events, &mut super::battle::effect);
        }
        true
    }
    pub fn negotiate_ready_connections(
        &self,
        notify: bool,
        next: u32,
        wait: &mut ProtocolWait<'_>,
    ) -> bool {
        self.wait_connections(|side| {
            self.negotiate_ready(side, notify, next, wait);
        })
    }
    pub fn wait_reset_connections(&self, next: u32, wait: &mut ProtocolWait<'_>) -> bool {
        self.wait_connections(|side| {
            let generation = {
                let mut conn = self
                    .connection_mut(side)
                    .unwrap_or_else(|| panic!("reset connection"));
                let generation = conn.reset_generation;
                conn.reset_next = Some(next);
                conn.reset_complete();
                generation
            };
            if self
                .connection(side)
                .unwrap_or_else(|| panic!("reset connection"))
                .reset_generation
                == generation
            {
                let effects = {
                    let mut conn = self
                        .connection_mut(side)
                        .unwrap_or_else(|| panic!("reset connection"));
                    if !conn.flags.reset.local {
                        conn.local_reset(2)
                    } else {
                        Vec::new()
                    }
                };
                self.apply_events(effects, &mut super::battle::effect);
                self.flush_packet_queues();
                self.wait_exchange(side, generation, true, wait);
            }
        })
    }
    fn wait_connections(&self, mut wait: impl FnMut(usize)) -> bool {
        let mut disconnected = 0;
        for side in 0..2 {
            let Some(conn) = self.connection(side) else {
                continue;
            };
            let connected = conn.flags.connected;
            drop(conn);
            if !connected {
                disconnected += 1;
                continue;
            }
            wait(side);
        }
        disconnected == 0
    }
    fn wait_exchange(
        &self,
        side: usize,
        generation: usize,
        reset: bool,
        wait: &mut ProtocolWait<'_>,
    ) {
        let mut poll = || {
            self.net_input_blocking(NETWORK_POLL_DELAY);
            self.connection(side).is_some_and(|conn| {
                conn.flags.connected
                    && if reset {
                        conn.reset_generation == generation
                    } else {
                        conn.ready_generation == generation
                    }
            })
        };
        if !self.replay_wait(&mut poll) {
            wait(&mut poll);
        }
    }
    fn negotiate_ready(
        &self,
        side: usize,
        notify: bool,
        next: u32,
        wait: &mut ProtocolWait<'_>,
    ) -> bool {
        let (generation, effects) = {
            let mut conn = self
                .connection_mut(side)
                .unwrap_or_else(|| panic!("ready connection"));
            let generation = conn.ready_generation;
            let effects = conn.local_ready(
                ReadyAction::Transition(next),
                notify,
                self.frame_count.get(),
            );
            (generation, effects)
        };
        self.apply_events(effects, &mut super::battle::effect);
        self.flush_packet_queues();
        if self
            .connection(side)
            .unwrap_or_else(|| panic!("ready connection"))
            .ready_generation
            == generation
        {
            self.wait_exchange(side, generation, false, wait);
        }
        self.connection(side)
            .is_some_and(|conn| conn.flags.connected)
    }
    pub(super) fn ready_for_battle_end_player(
        &self,
        side: usize,
        wait: &mut ProtocolWait<'_>,
    ) -> bool {
        let state = self
            .connection(side)
            .unwrap_or_else(|| panic!("end connection"))
            .state;
        if state == NetState_NetState_interBattle || state == NetState_NetState_inSetup {
            return true;
        }
        if state == NetState_NetState_inBattle {
            let effects = {
                let mut conn = self
                    .connection_mut(side)
                    .unwrap_or_else(|| panic!("end connection"));
                if conn.flags.ready.local {
                    return false;
                }
                conn.local_ready(ReadyAction::EndBattle, true, self.frame_count.get())
            };
            self.apply_events(effects, &mut super::battle::effect);
            if self
                .connection(side)
                .unwrap_or_else(|| panic!("end connection"))
                .state
                == NetState_NetState_inBattle
            {
                return false;
            }
        }
        let conn = self
            .connection(side)
            .unwrap_or_else(|| panic!("end connection"));
        assert!(
            conn.state == NetState_NetState_endingBattle
                || conn.state == NetState_NetState_endingBattle2
        );
        if self.frame_count.get() < conn.end_frame_count {
            return false;
        }
        let generation = conn.ready_generation;
        let state = conn.state;
        drop(conn);
        if state == NetState_NetState_endingBattle {
            self.wait_exchange(side, generation, false, wait);
            let conn = self
                .connection(side)
                .unwrap_or_else(|| panic!("end connection"));
            assert_eq!(conn.state, NetState_NetState_endingBattle2);
            if self.frame_count.get() < conn.end_frame_count {
                return false;
            }
        }
        self.negotiate_ready(side, true, NetState_NetState_interBattle, wait);
        true
    }
    pub fn network_battle_input(
        &self,
        player: usize,
        activity: &crate::engine::world::activity::ActivityControl,
    ) -> u8 {
        use crate::mainloop::activity_flags::CHECK_ABORT;
        loop {
            if let Some(input) = self.pop_input(player) {
                return input;
            }
            self.net_input();
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
            self.net_input_blocking(500);
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
