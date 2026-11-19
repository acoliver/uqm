//! Owned protocol state for one remote player.
use super::{abi::*, checkbuf::ChecksumBuffer, packet::Packet, packetq::PacketQueue};

#[derive(Debug, Clone)]
pub struct PeerOptions {
    pub server: bool,
    pub host: String,
    pub port: String,
}
impl Default for PeerOptions {
    fn default() -> Self {
        Self {
            server: true,
            host: "localhost".into(),
            port: "21837".into(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Options {
    pub peer: [PeerOptions; 2],
    pub input_delay: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            peer: std::array::from_fn(|_| PeerOptions::default()),
            input_delay: 2,
        }
    }
}
#[derive(Debug, Default)]
pub struct Exchange {
    pub local: bool,
    pub remote: bool,
}
#[derive(Debug, Default)]
pub struct Handshake {
    pub local: bool,
    pub remote: bool,
    pub canceling: bool,
}
#[derive(Debug, Default)]
pub struct Flags {
    pub connected: bool,
    pub disconnected: bool,
    pub discriminant: bool,
    pub handshake: Handshake,
    pub ready: Exchange,
    pub reset: Exchange,
    pub random_seed: bool,
    pub input_delay: usize,
    pub checksum_interval: usize,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReadyAction {
    EnterSetup,
    Transition(NetState),
    EndBattle,
    EndBattle2,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    ResetSentTeams,
    ShipSelected,
    EndBattle,
    FinishReset,
    Connected,
    Closed,
    Error(i32),
    Bootstrap,
    ConfirmationCancelled,
    Ship { side: u16, index: u16, ship: u32 },
    TeamName { side: u16, name: Vec<u8> },
    Seed(u32),
    Abort(u32),
    Reset { reason: u32, remote: bool },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub player: usize,
    pub battle_active: bool,
    pub effect: Effect,
}
#[derive(Debug)]
pub struct NetConnection {
    pub socket: Option<uqm_engine::host::Connection>,
    pub player: usize,
    pub state: NetState,
    pub flags: Flags,
    pub ready_action: Option<ReadyAction>,
    pub reset_next: Option<NetState>,
    pub ready_generation: usize,
    pub reset_generation: usize,
    pub options: PeerOptions,
    pub queue: PacketQueue,
    pub packets_received: usize,
    pub packet_types_received: [usize; 18],
    pub checksums: ChecksumBuffer,
    pub read_buffer: Vec<u8>,
    pub battle_active: bool,
    pub end_frame_count: u32,
}
impl NetConnection {
    pub fn new(player: usize, options: PeerOptions) -> Self {
        Self {
            socket: None,
            player,
            state: NetState_NetState_unconnected,
            flags: Flags {
                checksum_interval: NETPLAY_CHECKSUM_INTERVAL as usize,
                ..Flags::default()
            },
            ready_action: None,
            reset_next: None,
            ready_generation: 0,
            reset_generation: 0,
            options,
            queue: PacketQueue::default(),
            packets_received: 0,
            packet_types_received: [0; 18],
            checksums: ChecksumBuffer::default(),
            read_buffer: Vec::with_capacity(NETPLAY_READBUFSIZE as usize),
            battle_active: false,
            end_frame_count: 0,
        }
    }
    pub fn queue(&mut self, packet: Packet) {
        assert!(self.flags.connected);
        self.queue.push(packet);
    }
    pub fn event(&self, effect: Effect) -> Event {
        Event {
            player: self.player,
            battle_active: self.battle_active,
            effect,
        }
    }
    pub fn local_ready(&mut self, action: ReadyAction, notify: bool, frame: u32) -> Vec<Event> {
        assert!(super::ready::meaningful(self.state));
        assert!(!self.flags.ready.local);
        self.ready_action = Some(action);
        if notify {
            self.queue(Packet::Ready);
        }
        if !self.flags.ready.remote {
            self.flags.ready.local = true;
            return Vec::new();
        }
        self.flags.ready.remote = false;
        self.both_ready(frame)
    }
    pub fn remote_ready(&mut self, frame: u32) -> Vec<Event> {
        assert!(super::ready::meaningful(self.state));
        assert!(!self.flags.ready.remote);
        if !self.flags.ready.local {
            self.flags.ready.remote = true;
            return Vec::new();
        }
        self.flags.ready.local = false;
        self.both_ready(frame)
    }
    fn both_ready(&mut self, frame: u32) -> Vec<Event> {
        let action = self
            .ready_action
            .take()
            .unwrap_or_else(|| panic!("ready action"));
        self.ready_generation += 1;
        match action {
            ReadyAction::EnterSetup => {
                self.state = NetState_NetState_inSetup;
                vec![self.event(Effect::Connected), self.event(Effect::Bootstrap)]
            }
            ReadyAction::Transition(next) => {
                self.state = next;
                Vec::new()
            }
            ReadyAction::EndBattle => {
                self.state = NetState_NetState_endingBattle;
                let target = frame.wrapping_add(1);
                self.end_frame_count = self.end_frame_count.max(target);
                self.queue(Packet::FrameCount(target));
                vec![self.event(Effect::EndBattle)]
            }
            ReadyAction::EndBattle2 => {
                self.state = NetState_NetState_endingBattle2;
                Vec::new()
            }
        }
    }
    pub fn confirm(&mut self) -> bool {
        assert_eq!(self.state, NetState_NetState_inSetup);
        if self.flags.handshake.local {
            return false;
        }
        self.flags.handshake.local = true;
        if !self.flags.handshake.canceling {
            self.queue(if self.flags.handshake.remote {
                Packet::Handshake1
            } else {
                Packet::Handshake0
            });
        }
        true
    }
    pub fn cancel_confirmation(&mut self) -> bool {
        assert_eq!(self.state, NetState_NetState_inSetup);
        if !self.flags.handshake.local {
            return false;
        }
        self.flags.handshake.local = false;
        if !self.flags.handshake.canceling {
            self.flags.handshake.canceling = true;
            self.queue(Packet::HandshakeCancel);
        }
        true
    }
    pub fn reset_complete(&mut self) {
        if self.flags.reset.local && self.flags.reset.remote {
            if let Some(next) = self.reset_next.take() {
                self.flags.reset = Exchange::default();
                self.reset_generation += 1;
                if next != NetState::MAX {
                    self.state = next;
                }
            }
        }
    }
    pub fn local_reset(&mut self, reason: u32) -> Vec<Event> {
        assert!(!self.flags.reset.local);
        self.flags.reset.local = true;
        if self.flags.reset.remote {
            self.reset_complete();
            Vec::new()
        } else {
            self.queue(Packet::Reset(reason as u16));
            self.reset_feedback(reason, false)
        }
    }
    pub fn remote_reset(&mut self, reason: u32) -> Vec<Event> {
        assert!(!self.flags.reset.remote);
        self.flags.reset.remote = true;
        if !self.flags.reset.local {
            self.queue(Packet::Reset(reason as u16));
            self.flags.reset.local = true;
            let mut effects = self.reset_feedback(reason, true);
            if effects.is_empty() {
                self.reset_complete();
            } else {
                effects.push(self.event(Effect::FinishReset));
            }
            effects
        } else {
            self.reset_complete();
            Vec::new()
        }
    }
    fn reset_feedback(&self, reason: u32, remote: bool) -> Vec<Event> {
        if (NetState_NetState_preBattle..=NetState_NetState_endingBattle2).contains(&self.state) {
            vec![self.event(Effect::Reset { reason, remote })]
        } else {
            Vec::new()
        }
    }
}
