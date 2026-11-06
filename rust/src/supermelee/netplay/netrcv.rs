//! Bounded stream framing; effects run between packets without a connection borrow.
use super::{
    abi::*,
    netconnection::{Effect, Event, ReadyAction},
    packet::{DecodeError, Packet},
    session::SessionCore,
};
use std::io;

impl SessionCore {
    pub fn dispatch(&self, player: usize, packet: &Packet, emit: &mut dyn FnMut(Event)) -> i32 {
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
        self.net_input_with(
            timeout.min(crate::callback::ffi::Async_timeBeforeNextMs()),
            emit,
        );
    }
    pub fn net_input_with(&self, timeout: u32, emit: &mut dyn FnMut(Event)) {
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(u64::from(timeout));
        let before = self.connection_progress();
        loop {
            crate::callback::ffi::Async_process();
            for player in 0..2 {
                if self
                    .connection(player)
                    .is_some_and(|conn| !conn.flags.disconnected)
                {
                    self.poll_connection(player, emit);
                }
            }
            self.flush_packet_queues(emit);
            crate::callback::ffi::Callback_process();
            if self.connection_progress() != before || std::time::Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(
                deadline
                    .saturating_duration_since(std::time::Instant::now())
                    .min(std::time::Duration::from_millis(1)),
            );
        }
    }
    fn connection_progress(&self) -> [(u32, usize); 2] {
        std::array::from_fn(|side| {
            self.connection(side)
                .map_or((0, 0), |conn| (conn.state, conn.packets_received))
        })
    }
    fn error(&self, side: usize, error: i32, emit: &mut dyn FnMut(Event)) {
        let event = self
            .connection(side)
            .unwrap_or_else(|| panic!("error connection"))
            .event(Effect::Error(error));
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
        let connecting = conn.state == NetState_NetState_connecting;
        drop(conn);
        if connecting {
            let result = self.transport.borrow_mut().poll(socket);
            match result {
                Ok(false) => return,
                Err(error) => {
                    self.error(side, error.raw_os_error().unwrap_or(libc::EIO), emit);
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
                conn.state = NetState_NetState_init;
                conn.event(Effect::ResetSentTeams)
            };
            emit(event);
            {
                let mut conn = self
                    .connection_mut(side)
                    .unwrap_or_else(|| panic!("connected connection"));
                conn.queue(Packet::Init {
                    protocol: [
                        NETPLAY_PROTOCOL_VERSION_MAJOR as u8,
                        NETPLAY_PROTOCOL_VERSION_MINOR as u8,
                    ],
                    version: [
                        UQM_MAJOR_VERSION as u8,
                        UQM_MINOR_VERSION as u8,
                        UQM_PATCH_VERSION as u8,
                    ],
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
            let mut bytes = [0; NETPLAY_READBUFSIZE as usize];
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
                Err(error) => {
                    self.error(side, error.raw_os_error().unwrap_or(libc::EIO), emit);
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
                        if result == -1 {
                            self.error(side, crate::platform::errno(), emit);
                            return;
                        }
                        processed += length;
                    }
                    Err(DecodeError::Incomplete) => break,
                    Err(_) => {
                        self.error(side, libc::EBADMSG, emit);
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
