pub mod bridge;
pub mod observe;

use std::sync::Arc;

/// Owns the observer for the full application lifecycle, including finalization.
pub struct Application {
    pub game: crate::engine::Game,
    observer: Option<Arc<crate::automation::AutomationObserver>>,
}

impl Application {
    pub fn new(observer: Option<Arc<crate::automation::AutomationObserver>>) -> Self {
        if let Some(observer) = &observer {
            bridge::install_observer(observer);
        }
        Self {
            observer,
            game: crate::engine::Game::new(),
        }
    }

    /// # Safety
    /// The game-thread call must not retain owner references after returning.
    pub unsafe fn legacy<R>(&mut self, call: impl FnOnce() -> R) -> R {
        // SAFETY: the application owns Game and suspends direct access for this call.
        unsafe { bridge::with_game(&mut self.game, call) }
    }

    pub fn observer(&self) -> Option<&Arc<crate::automation::AutomationObserver>> {
        self.observer.as_ref()
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        if let Some(observer) = &self.observer {
            bridge::uninstall_observer(observer);
        }
    }
}
