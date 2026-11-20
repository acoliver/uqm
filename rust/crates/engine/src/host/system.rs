//! The operating-system services the engine takes from the host: real time,
//! sleeping, threads and process exit.
//!
//! Replays read the same real time: movie pacing and frame diagnostics are
//! wall-clock measurements, while game time comes from [`super::Clock`].

use std::time::Duration;

/// A thread started through [`System::spawn`].
pub trait Worker: Send {
    /// Wait for the thread to finish; `false` when it panicked.
    fn join(self: Box<Self>) -> bool;
}

pub trait System: Send + Sync {
    /// Real time since the host started.
    fn elapsed(&self) -> Duration;
    fn sleep(&self, duration: Duration);
    /// Sleep for a frame-pacing wait, with as little timer coalescing as
    /// the OS offers. Only the game thread paces.
    fn sleep_precise(&self, duration: Duration);
    fn yield_now(&self);
    /// Milliseconds since the Unix epoch, or `None` when the clock reads earlier.
    fn unix_millis(&self) -> Option<u64>;
    /// Start a named thread running `body`.
    fn spawn(&self, name: &str, body: Box<dyn FnOnce() + Send>) -> Box<dyn Worker>;
    /// Run `handler` when the process exits normally.
    fn at_exit(&self, handler: fn());
    /// End the process with `code`.
    fn exit(&self, code: i32) -> !;
}

fn system() -> &'static dyn System {
    super::platform().system()
}

/// Real time since the host started.
pub fn elapsed() -> Duration {
    system().elapsed()
}

pub fn sleep(duration: Duration) {
    system().sleep(duration);
}

/// Sleep for a frame-pacing wait; see [`System::sleep_precise`].
pub fn sleep_precise(duration: Duration) {
    system().sleep_precise(duration);
}

pub fn yield_now() {
    system().yield_now();
}

/// Milliseconds since the Unix epoch, or `None` when the clock reads earlier.
pub fn unix_millis() -> Option<u64> {
    system().unix_millis()
}

/// Start a named thread running `body`.
pub fn spawn(name: &str, body: impl FnOnce() + Send + 'static) -> Box<dyn Worker> {
    system().spawn(name, Box::new(body))
}

/// Run `handler` when the process exits normally.
pub fn at_exit(handler: fn()) {
    system().at_exit(handler);
}

/// End the process with `code`.
pub fn exit(code: i32) -> ! {
    system().exit(code)
}

/// A point in real time that a polling loop waits for.
#[derive(Clone, Copy, Debug)]
pub struct Deadline(Duration);

impl Deadline {
    pub fn after_millis(millis: u32) -> Self {
        Self(elapsed() + Duration::from_millis(u64::from(millis)))
    }

    pub fn has_passed(self) -> bool {
        elapsed() >= self.0
    }

    /// Sleep until the deadline, but for no longer than one millisecond.
    pub fn nap(self) {
        sleep(
            self.0
                .saturating_sub(elapsed())
                .min(Duration::from_millis(1)),
        );
    }
}

/// A thread's identity, without asking the OS: each thread takes the next
/// number the first time it asks.
pub fn thread_number() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    thread_local! {
        static NUMBER: u64 = NEXT.fetch_add(1, Ordering::Relaxed);
    }
    NUMBER.with(|number| *number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_millisecond_deadline_has_passed_and_a_long_one_has_not() {
        assert!(Deadline::after_millis(0).has_passed());
        assert!(!Deadline::after_millis(60_000).has_passed());
    }

    #[test]
    fn nap_returns_within_a_millisecond_of_a_distant_deadline() {
        let start = elapsed();
        Deadline::after_millis(60_000).nap();
        assert!(elapsed() - start < Duration::from_millis(500));
    }

    #[test]
    fn each_thread_has_its_own_stable_number() {
        let here = thread_number();
        assert_eq!(thread_number(), here);
        let there = spawn("number", move || assert_ne!(thread_number(), here));
        assert!(there.join());
    }
}
