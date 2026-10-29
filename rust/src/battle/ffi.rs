/// Game-thread access to the owned battle counters.
#[cfg(not(test))]
pub(crate) mod live_counters {
    /// # Safety
    /// Game-thread legacy boundary with the battle context published.
    pub(crate) unsafe fn battle_counters() -> [u8; 2] {
        // SAFETY: the caller guarantees the active game-thread publication.
        unsafe { crate::app::bridge::battle(|battle| battle.counters) }
    }
}
