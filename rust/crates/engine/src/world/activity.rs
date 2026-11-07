//! One live activity word with explicitly supplied observation and flag signaling.

use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc,
};

pub struct ActivityState {
    value: Arc<AtomicU16>,
}

impl ActivityState {
    pub fn new(value: u16) -> Self {
        Self {
            value: Arc::new(AtomicU16::new(value)),
        }
    }

    pub fn activity(&self) -> u16 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn set_activity(&mut self, value: u16) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn control(&self) -> ActivityControl {
        ActivityControl {
            value: self.value.clone(),
        }
    }
}

/// Reads the owner's current word and raises flags without retaining a World borrow.
#[derive(Clone)]
pub struct ActivityControl {
    value: Arc<AtomicU16>,
}

impl ActivityControl {
    pub fn activity(&self) -> u16 {
        self.value.load(Ordering::Relaxed)
    }

    /// Return the activity before raising flags, for terminal-outcome recording.
    pub fn raise_flags(&self, flags: u16) -> u16 {
        self.value.fetch_or(flags, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::ActivityState;

    #[test]
    fn controls_read_fresh_owner_values_without_a_publication() {
        let mut state = ActivityState::new(3);
        let control = state.control();
        let other_control = control.clone();
        state.set_activity(0x1004);
        assert_eq!(control.activity(), 0x1004);
        assert_eq!(other_control.activity(), 0x1004);
        assert_eq!(state.activity(), 0x1004);
    }

    #[test]
    fn raising_flags_returns_the_preimage_and_preserves_the_activity() {
        let state = ActivityState::new(0x0403);
        let control = state.control();
        assert_eq!(control.raise_flags(0x4000), 0x0403);
        assert_eq!(state.activity(), 0x4403);
        assert_eq!(control.raise_flags(0x4000), 0x4403);
        assert_eq!(state.activity(), 0x4403);
    }

    #[test]
    fn reset_uses_the_same_word_and_does_not_affect_another_owner() {
        let mut first = ActivityState::new(0xffff);
        let second = ActivityState::new(7);
        let control = first.control();
        first.set_activity(0);
        assert_eq!(control.activity(), 0);
        assert_eq!(second.activity(), 7);
        control.raise_flags(0x4000);
        assert_eq!(first.activity(), 0x4000);
        assert_eq!(second.activity(), 7);
    }

    #[test]
    fn flag_signal_keeps_the_observer_send_and_sync_without_sharing_a_world() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<super::ActivityControl>();
        let state = ActivityState::new(3);
        let control = state.control();
        std::thread::scope(|scope| {
            scope.spawn(move || assert_eq!(control.raise_flags(0x4000), 3));
        });
        assert_eq!(state.activity(), 0x4003);
    }
}
