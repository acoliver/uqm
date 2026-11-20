//! The reasons a session restarts or ends, as sent on the wire.

/// Why a peer asked for a synchronized restart, as sent on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ResetReason {
    SyncLoss = 1,
    ManualReset = 2,
}

/// Wire value of the abort sent when the peer's protocol or version is unusable.
pub const ABORT_VERSION_MISMATCH: u32 = 1;
