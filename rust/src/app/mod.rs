pub mod bridge;
mod lifecycle;
mod runtime;
#[cfg(not(test))]
pub use runtime::run;

use std::sync::Arc;

/// Owns the observer for the full application lifecycle, including finalization.
pub struct Application {
    pub game: crate::engine::Game,
    observer: Option<Arc<crate::automation::AutomationObserver>>,
}

impl Application {
    pub fn new(
        mut game: crate::engine::Game,
        observer: Option<Arc<crate::automation::AutomationObserver>>,
    ) -> Self {
        game.world.set_catalog(std::borrow::Cow::Borrowed(
            &crate::planets::tables::STARMAP_ARRAY,
        ));
        Self { observer, game }
    }

    #[cfg(not(test))]
    pub fn install_input(&mut self, input: crate::engine::input::InputControls) {
        let runtime = crate::mainloop::input_abi::NativeInputRuntime::new(
            self.observer().cloned(),
            input,
            self.game.kernel.configuration.clone(),
        );
        self.game
            .kernel
            .install_input_runtime(std::rc::Rc::new(runtime));
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

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn application_retains_the_catalog_without_publishing_game() {
        let app = Application::new(crate::engine::Game::new(), None);
        assert!(std::ptr::eq(
            app.game.world.catalog(),
            &crate::planets::tables::STARMAP_ARRAY,
        ));
        assert!(app.game.world.selected_star().is_none());
        assert!(app.game.world.star_pointer().is_null());
    }
}
