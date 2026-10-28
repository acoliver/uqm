//! Game-thread callback and timed-alarm dispatch with Rust-owned closures.
//!
//! ## Architecture
//!
//! - **Callback**: FIFO queue with mutex. `Callback_process` snapshots the
//!   current queue and processes only those entries (callbacks added during
//!   processing are deferred to the next call), matching C semantics.
//! - **Alarm**: Min-heap of timed callbacks. Uses `BinaryHeap` with reversed
//!   ordering. Removal is lazy (cancelled flag) since alarms are few.
//! - **Async**: Combines callback + alarm processing.
//! - **Heap**: Not ported — Rust's `BinaryHeap` replaces it entirely.

pub mod ffi;
