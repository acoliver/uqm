//! Bounded stream framing; effects run between packets without a connection borrow.
use super::{
    netconnection::{Effect, Event, ReadyAction, Rejected},
    packet::{DecodeError, Packet},
    session::SessionCore,
};
use crate::engine::battle::net::NetState;
use crate::supermelee::netplay::READ_BUFFER_SIZE;
use std::io;

impl SessionCore {
    pub fn dispatch(
        &self,
        player: usize,
        packet: &Packet,
        emit: &mut dyn FnMut(Event),
    ) -> Result<(), Rejected> {
        let (events, result) = {
            let mut conn = self
                .connection_mut(player)
                .unwrap_or_else(|| panic!("dispatch connection"));
            conn.packets_received += 1;
            conn.packet_types_received[packet.kind()] += 1;
            super::packethandlers::handle(self, &mut conn, packet)
        };
        self.apply_events(events, emit);
        result
    }
    pub(super) fn apply_events(&self, events: Vec<Event>, emit: &mut dyn FnMut(Event)) {
        for event in events {
            if event.effect == Effect::EndBattle {
                // The frame-count send precedes installing the next ready exchange.
                let _ = self.flush_player(event.player);
                let events = self
                    .connection_mut(event.player)
                    .unwrap_or_else(|| panic!("end connection"))
                    .local_ready(ReadyAction::EndBattle2, false, self.frame_count.get());
                self.apply_events(events, emit);
            } else if event.effect == Effect::FinishReset {
                // Reset feedback may close the peer while flushing its reply.
                if let Some(mut conn) = self.connection_mut(event.player) {
                    conn.reset_complete();
                }
            } else {
                let seed = matches!(event.effect, Effect::Seed(_));
                let player = event.player;
                emit(event);
                if seed {
                    self.connection_mut(player)
                        .unwrap_or_else(|| panic!("seed connection"))
                        .flags
                        .random_seed = true;
                }
            }
        }
    }
    pub fn net_input_blocking(&self, timeout: u32, emit: &mut dyn FnMut(Event)) {
        self.net_input_with(timeout.min(uqm_engine::host::async_wait_ms()), emit);
    }
    pub fn net_input_with(&self, timeout: u32, emit: &mut dyn FnMut(Event)) {
        let deadline = uqm_engine::host::Deadline::after_millis(timeout);
        let before = self.connection_progress();
        loop {
            uqm_engine::host::process_async();
            for player in 0..2 {
                if self
                    .connection(player)
                    .is_some_and(|conn| !conn.flags.disconnected)
                {
                    self.poll_connection(player, emit);
                }
            }
            self.flush_packet_queues(emit);
            uqm_engine::host::dispatch().process_callbacks();
            if self.connection_progress() != before || deadline.has_passed() {
                return;
            }
            deadline.nap();
        }
    }
    fn connection_progress(&self) -> [(u32, usize); 2] {
        std::array::from_fn(|side| {
            self.connection(side)
                .map_or((0, 0), |conn| (conn.state as u32, conn.packets_received))
        })
    }
    fn error(&self, side: usize, emit: &mut dyn FnMut(Event)) {
        let event = self
            .connection(side)
            .unwrap_or_else(|| panic!("error connection"))
            .event(Effect::Error);
        emit(event);
        self.disconnect(side, emit);
    }
    fn poll_connection(&self, side: usize, emit: &mut dyn FnMut(Event)) {
        let conn = self
            .connection(side)
            .unwrap_or_else(|| panic!("poll connection"));
        let socket = conn
            .socket
            .unwrap_or_else(|| panic!("connection without transport"));
        let connecting = conn.state == NetState::Connecting;
        drop(conn);
        if connecting {
            let result = self.transport.borrow_mut().poll(socket);
            match result {
                Ok(false) => return,
                Err(_) => {
                    self.error(side, emit);
                    return;
                }
                Ok(true) => {}
            }
            let event = {
                let mut conn = self
                    .connection_mut(side)
                    .unwrap_or_else(|| panic!("connected connection"));
                conn.flags.discriminant = conn.options.server;
                conn.flags.connected = true;
                conn.state = NetState::Init;
                conn.event(Effect::ResetSentTeams)
            };
            emit(event);
            {
                let mut conn = self
                    .connection_mut(side)
                    .unwrap_or_else(|| panic!("connected connection"));
                conn.queue(Packet::Init {
                    protocol: super::PROTOCOL_VERSION,
                    version: super::UQM_VERSION,
                });
                conn.local_ready(ReadyAction::EnterSetup, false, self.frame_count.get());
            }
        }
        if self
            .connection(side)
            .unwrap_or_else(|| panic!("receive connection"))
            .flags
            .connected
        {
            self.receive(side, emit);
        }
    }
    fn receive(&self, side: usize, emit: &mut dyn FnMut(Event)) {
        let socket = self
            .connection(side)
            .unwrap_or_else(|| panic!("receive connection"))
            .socket
            .unwrap_or_else(|| panic!("receive without transport"));
        loop {
            let mut bytes = [0; READ_BUFFER_SIZE];
            let used = self
                .connection(side)
                .unwrap_or_else(|| panic!("receive buffer"))
                .read_buffer
                .len();
            let capacity = bytes.len() - used;
            let result = self
                .transport
                .borrow_mut()
                .recv(socket, &mut bytes[..capacity]);
            let count = match result {
                Ok(0) => {
                    self.disconnect(side, emit);
                    return;
                }
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.error(side, emit);
                    return;
                }
            };
            self.connection_mut(side)
                .unwrap_or_else(|| panic!("receive buffer"))
                .read_buffer
                .extend_from_slice(&bytes[..count]);
            let mut processed = 0;
            loop {
                let decoded = {
                    let conn = self
                        .connection(side)
                        .unwrap_or_else(|| panic!("decode buffer"));
                    if processed == conn.read_buffer.len() {
                        break;
                    }
                    Packet::decode(&conn.read_buffer[processed..])
                };
                match decoded {
                    Ok((packet, length)) => {
                        let result = self.dispatch(side, &packet, emit);
                        if self.connection(side).is_none() {
                            return;
                        }
                        if result.is_err() {
                            self.error(side, emit);
                            return;
                        }
                        processed += length;
                    }
                    Err(DecodeError::Incomplete) => break,
                    Err(_) => {
                        self.error(side, emit);
                        return;
                    }
                }
            }
            if processed == 0 {
                return;
            }
            self.connection_mut(side)
                .unwrap_or_else(|| panic!("receive buffer"))
                .read_buffer
                .drain(..processed);
        }
    }
}
