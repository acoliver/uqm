//! Captured thread identity for thread-bound game resources.

use crate::host::thread_number;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreadAffinity(u64);

impl ThreadAffinity {
    pub fn current() -> Self {
        Self(thread_number())
    }

    pub fn is_current(self) -> bool {
        self.0 == thread_number()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_affinity_matches_its_thread() {
        let affinity = ThreadAffinity::current();
        assert!(affinity.is_current());
        assert_eq!(affinity, ThreadAffinity::current());
    }

    #[test]
    fn affinity_rejects_another_thread_without_rebinding() {
        let affinity = ThreadAffinity::current();
        std::thread::spawn(move || {
            assert!(!affinity.is_current());
            assert!(ThreadAffinity::current().is_current());
        })
        .join()
        .unwrap();
        assert!(affinity.is_current());
    }
}
