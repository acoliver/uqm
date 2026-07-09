//! Heap + Callback + Alarm + Async — ported from C `libs/heap`, `libs/callback`.
//!
//! This module replaces four C files (heap.c, callback.c, alarm.c, async.c)
//! with idiomatic Rust. The C headers remain for type declarations; the
//! function symbols are resolved from here via `#[no_mangle] extern "C"`.
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
