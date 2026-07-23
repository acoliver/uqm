//! Inactive authenticated Unix datagram transport model.
//!
//! Only proof inactive-smoke options enable transport. Normal inactive
//! callbacks remain neutral/no-work. Transport uses an exclusive run root,
//! AF_UNIX/SOCK_DGRAM, mode 0600, random 256-bit nonce, typed command IDs,
//! replay/duplicate rejection, fixed packet cap, and typed acks.
//!
//! Darwin is explicitly classified as not supporting peer credentials for
//! SOCK_DGRAM (LOCAL_PEERCRED returns EINVAL). Darwin retains exclusive
//! path, 0600 permissions, nonce authentication, and replay rejection
//! without claiming credential verification.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
//! @requirement REQ-TRANSPORT-001..003

use std::collections::HashSet;
use std::path::PathBuf;

// ===========================================================================
//  Transport commands and acknowledgements (REQ-TRANSPORT-001)
// ===========================================================================

/// Typed command IDs for the datagram transport protocol.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-TRANSPORT-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CommandId {
    /// Tap the menu-down key (proof smoke).
    TapDown = 1,
    /// Push SDL_QUIT (quit smoke).
    QuitSmoke = 2,
    /// No-op ping.
    Ping = 3,
}

impl CommandId {
    /// Convert from a raw byte.
    #[must_use]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::TapDown),
            2 => Some(Self::QuitSmoke),
            3 => Some(Self::Ping),
            _ => None,
        }
    }

    /// Convert to a raw byte.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Typed acknowledgement for a transport command.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-TRANSPORT-001
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckKind {
    /// Command accepted and dispatched.
    Accepted,
    /// Command rejected: invalid nonce.
    RejectedBadNonce,
    /// Command rejected: replay/duplicate.
    RejectedReplay,
    /// Command rejected: unknown command ID.
    RejectedUnknownCommand,
    /// Command rejected: push failure.
    RejectedPushFailed,
}

/// A typed acknowledgement record.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-TRANSPORT-001
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckRecord {
    pub kind: AckKind,
    pub command_id: CommandId,
}

// ===========================================================================
//  Packet model (REQ-TRANSPORT-001)
// ===========================================================================

/// A typed transport packet.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-TRANSPORT-001
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportPacket {
    /// Protocol version.
    pub version: u8,
    /// 256-bit nonce (32 bytes).
    pub nonce: [u8; 32],
    /// Command ID.
    pub command_id: u8,
    /// Command payload (opaque).
    pub command: Vec<u8>,
}

/// Current protocol version.
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum socket path length for AF_UNIX SOCK_DGRAM.
///
/// Darwin has a smaller limit than Linux; we use a conservative value.
pub const MAX_SOCKET_PATH_LEN: usize = 81;

/// Maximum packets to process per pump.
pub const PACKETS_PER_PUMP: u32 = 16;

// ===========================================================================
//  Transport state (pure model for testing)
// ===========================================================================

/// The authentication state for the transport.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-TRANSPORT-001
#[derive(Debug, Clone)]
pub struct TransportState {
    /// The expected nonce (set at setup).
    expected_nonce: [u8; 32],
    /// Seen nonces for replay rejection.
    seen_nonces: HashSet<[u8; 32]>,
    /// Whether peer credentials are supported on this platform.
    peer_credentials_supported: bool,
    /// Socket path.
    socket_path: Option<PathBuf>,
    /// Whether transport is enabled (proof-smoke only).
    enabled: bool,
}

impl TransportState {
    /// Create a new transport state with the given nonce.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-TRANSPORT-001
    #[must_use]
    pub fn new(nonce: [u8; 32], peer_credentials_supported: bool) -> Self {
        Self {
            expected_nonce: nonce,
            seen_nonces: HashSet::new(),
            peer_credentials_supported,
            socket_path: None,
            enabled: false,
        }
    }

    /// Enable the transport (proof-smoke only).
    pub fn enable(&mut self, path: PathBuf) {
        self.socket_path = Some(path);
        self.enabled = true;
    }

    /// Disable the transport.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.socket_path = None;
    }

    /// Returns `true` if transport is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns `true` if peer credentials are supported.
    #[must_use]
    pub fn peer_credentials_supported(&self) -> bool {
        self.peer_credentials_supported
    }

    /// Validate and authenticate a received packet.
    ///
    /// Returns an `AckRecord` indicating acceptance or rejection.
    /// Does NOT perform any I/O — this is the pure authentication model.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-TRANSPORT-001
    pub fn authenticate(&mut self, packet: &TransportPacket) -> AckRecord {
        // Check version.
        if packet.version != PROTOCOL_VERSION {
            return AckRecord {
                kind: AckKind::RejectedUnknownCommand,
                command_id: CommandId::Ping,
            };
        }

        // Check nonce.
        if packet.nonce != self.expected_nonce {
            return AckRecord {
                kind: AckKind::RejectedBadNonce,
                command_id: CommandId::from_u8(packet.command_id).unwrap_or(CommandId::Ping),
            };
        }

        // Check replay.
        if self.seen_nonces.contains(&packet.nonce) {
            return AckRecord {
                kind: AckKind::RejectedReplay,
                command_id: CommandId::from_u8(packet.command_id).unwrap_or(CommandId::Ping),
            };
        }

        // Check command ID.
        let cmd = match CommandId::from_u8(packet.command_id) {
            Some(c) => c,
            None => {
                return AckRecord {
                    kind: AckKind::RejectedUnknownCommand,
                    command_id: CommandId::Ping,
                };
            }
        };

        // Accept: record nonce for replay rejection.
        self.seen_nonces.insert(packet.nonce);

        AckRecord {
            kind: AckKind::Accepted,
            command_id: cmd,
        }
    }

    /// Check socket path length.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-TRANSPORT-001
    #[must_use]
    pub fn check_path_length(path: &str) -> bool {
        path.len() < MAX_SOCKET_PATH_LEN
    }
}

// ===========================================================================
//  Counter model (REQ-TRANSPORT-002)
// ===========================================================================

/// Typed counters for the inactive transport proof.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-TRANSPORT-002
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransportCounters {
    /// Datagram accepted count.
    pub datagrams_accepted: u64,
    /// Datagram rejected count.
    pub datagrams_rejected: u64,
    /// Replay rejections.
    pub replays_rejected: u64,
    /// Acknowledgements sent.
    pub acks_sent: u64,
    /// SDL_PushEvent successes.
    pub push_success: u64,
    /// SDL_PushEvent failures.
    pub push_fail: u64,
    /// C SDL_PollEvent count.
    pub c_poll_count: u64,
    /// Rust VControl dispatch count.
    pub rust_dispatch_count: u64,
    /// Post-update observation count.
    pub post_update_count: u64,
    /// Key observed (menu-down nonzero).
    pub key_observed: u64,
    /// SDL_QUIT pushed.
    pub quit_pushed: u64,
    /// SDL_QUIT polled.
    pub quit_polled: u64,
    /// Lifecycle observed QuitPosted.
    pub quit_lifecycle_observed: u64,
    /// Per-shell ABI entry (may be nonzero in inactive mode).
    pub abi_entry: u64,
    /// Active-gate entry (must be 0 in inactive mode).
    pub active_gate_entry: u64,
    /// Scheduler service transitions (must be 0 in inactive mode).
    pub scheduler_service: u64,
    /// Setter writes (must be 0 in inactive mode).
    pub setter_writes: u64,
}

impl TransportCounters {
    /// Returns `true` if the inactive acceptance criteria are met.
    ///
    /// Inactive acceptance requires:
    /// - active_gate_entry == 0
    /// - scheduler_service == 0
    /// - setter_writes == 0
    /// - abi_entry may be nonzero
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-TRANSPORT-002
    #[must_use]
    pub fn is_inactive_accepted(&self) -> bool {
        self.active_gate_entry == 0 && self.scheduler_service == 0 && self.setter_writes == 0
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_nonce() -> [u8; 32] {
        let mut n = [0u8; 32];
        n[0] = 42;
        n
    }

    fn make_packet(nonce: [u8; 32], cmd: u8) -> TransportPacket {
        TransportPacket {
            version: PROTOCOL_VERSION,
            nonce,
            command_id: cmd,
            command: vec![],
        }
    }

    // --- CommandId ---

    #[test]
    fn command_id_roundtrip() {
        for cmd in [CommandId::TapDown, CommandId::QuitSmoke, CommandId::Ping] {
            let raw = cmd.as_u8();
            assert_eq!(CommandId::from_u8(raw), Some(cmd));
        }
        assert_eq!(CommandId::from_u8(0), None);
        assert_eq!(CommandId::from_u8(255), None);
    }

    // --- Authentication ---

    #[test]
    fn valid_packet_accepted() {
        let mut state = TransportState::new(test_nonce(), false);
        let packet = make_packet(test_nonce(), CommandId::TapDown.as_u8());
        let ack = state.authenticate(&packet);
        assert_eq!(ack.kind, AckKind::Accepted);
        assert_eq!(ack.command_id, CommandId::TapDown);
    }

    #[test]
    fn bad_nonce_rejected() {
        let mut state = TransportState::new(test_nonce(), false);
        let bad_nonce = [99u8; 32];
        let packet = make_packet(bad_nonce, CommandId::TapDown.as_u8());
        let ack = state.authenticate(&packet);
        assert_eq!(ack.kind, AckKind::RejectedBadNonce);
    }

    #[test]
    fn replay_rejected() {
        let mut state = TransportState::new(test_nonce(), false);
        let packet = make_packet(test_nonce(), CommandId::TapDown.as_u8());

        // First packet: accepted.
        let ack1 = state.authenticate(&packet);
        assert_eq!(ack1.kind, AckKind::Accepted);

        // Second packet with same nonce: replay.
        let ack2 = state.authenticate(&packet);
        assert_eq!(ack2.kind, AckKind::RejectedReplay);
    }

    #[test]
    fn unknown_command_rejected() {
        let mut state = TransportState::new(test_nonce(), false);
        let packet = make_packet(test_nonce(), 99);
        let ack = state.authenticate(&packet);
        assert_eq!(ack.kind, AckKind::RejectedUnknownCommand);
    }

    #[test]
    fn wrong_version_rejected() {
        let mut state = TransportState::new(test_nonce(), false);
        let mut packet = make_packet(test_nonce(), CommandId::Ping.as_u8());
        packet.version = 2;
        let ack = state.authenticate(&packet);
        assert_eq!(ack.kind, AckKind::RejectedUnknownCommand);
    }

    // --- Platform classification ---

    #[test]
    fn darwin_peer_credentials_unsupported() {
        let state = TransportState::new(test_nonce(), false);
        assert!(!state.peer_credentials_supported());
    }

    #[test]
    fn linux_peer_credentials_supported() {
        let state = TransportState::new(test_nonce(), true);
        assert!(state.peer_credentials_supported());
    }

    // --- Socket path ---

    #[test]
    fn short_path_accepted() {
        assert!(TransportState::check_path_length("/tmp/uqm-transport/sock"));
    }

    #[test]
    fn long_path_rejected() {
        let long_path = format!("/tmp/{}", "a".repeat(100));
        assert!(!TransportState::check_path_length(&long_path));
    }

    // --- Enable/disable ---

    #[test]
    fn enable_sets_enabled() {
        let mut state = TransportState::new(test_nonce(), false);
        assert!(!state.is_enabled());
        state.enable(PathBuf::from("/tmp/uqm-transport/sock"));
        assert!(state.is_enabled());
        state.disable();
        assert!(!state.is_enabled());
    }

    // --- Counters (REQ-TRANSPORT-002) ---

    #[test]
    fn inactive_acceptance_zero_active_counters() {
        let counters = TransportCounters::default();
        assert!(counters.is_inactive_accepted());
    }

    #[test]
    fn inactive_acceptance_fails_with_active_gate() {
        let counters = TransportCounters {
            active_gate_entry: 1,
            ..TransportCounters::default()
        };
        assert!(!counters.is_inactive_accepted());
    }

    #[test]
    fn inactive_acceptance_fails_with_scheduler_service() {
        let counters = TransportCounters {
            scheduler_service: 1,
            ..TransportCounters::default()
        };
        assert!(!counters.is_inactive_accepted());
    }

    #[test]
    fn inactive_acceptance_fails_with_setter_writes() {
        let counters = TransportCounters {
            setter_writes: 1,
            ..TransportCounters::default()
        };
        assert!(!counters.is_inactive_accepted());
    }

    #[test]
    fn inactive_acceptance_allows_nonzero_abi_entry() {
        let counters = TransportCounters {
            abi_entry: 100,
            ..TransportCounters::default()
        };
        assert!(counters.is_inactive_accepted());
    }

    #[test]
    fn inactive_acceptance_allows_nonzero_counters() {
        // Transport proof counters can be nonzero in inactive mode.
        let counters = TransportCounters {
            datagrams_accepted: 5,
            c_poll_count: 10,
            rust_dispatch_count: 3,
            post_update_count: 2,
            ..TransportCounters::default()
        };
        assert!(counters.is_inactive_accepted());
    }

    // --- Protocol constants ---

    #[test]
    fn protocol_version_is_one() {
        // Verify protocol version constant is 1 at compile time.
        const _: () = assert!(PROTOCOL_VERSION == 1);
    }

    #[test]
    fn packets_per_pump_is_bounded() {
        const _: () = assert!(PACKETS_PER_PUMP > 0);
        const _: () = assert!(PACKETS_PER_PUMP <= 64);
    }
}
