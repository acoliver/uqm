//! Locking policy shared by the engine and the app.
use std::sync::{Mutex, MutexGuard};

/// Lock `mutex`, stopping the process when a holder panicked.
///
/// A poisoned mutex means another thread died halfway through an update, so
/// the protected state can no longer be trusted. Continuing would hide that
/// failure, and every caller would otherwise repeat the same unwrap.
#[track_caller]
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(_) => panic!("mutex poisoned by a panic on another thread"),
    }
}
