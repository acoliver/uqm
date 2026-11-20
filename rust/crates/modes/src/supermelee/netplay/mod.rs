//! Super Melee wire protocol and owned deterministic session runtime.
mod battle;
pub mod checkbuf;
pub mod checksum;
pub(crate) mod crc;
pub mod netconnection;
pub(crate) mod netmelee;
mod netrcv;
mod netwait;
mod notify;
pub mod packet;
mod packethandlers;
pub mod packetq;
mod ready;
pub(crate) mod session;
mod state;
pub use netconnection::{Effect, Event};
pub use session::NetplaySession;
pub use state::{ResetReason, ABORT_VERSION_MISMATCH};

/// Frames between checksum exchanges while a battle runs.
pub const CHECKSUM_INTERVAL: usize = 1;
/// Bytes read from a peer socket per pass.
pub const READ_BUFFER_SIZE: usize = 2048;
/// Wire protocol version that both peers must share.
pub const PROTOCOL_VERSION: [u8; 2] = [0, 4];
/// Oldest game version a peer may run.
pub const MIN_UQM_VERSION: [u8; 3] = [0, 6, 9];
/// Game version announced to peers.
pub const UQM_VERSION: [u8; 3] = [0, 8, 0];
#[cfg(test)]
mod driver_tests;
pub(crate) mod netinput;
