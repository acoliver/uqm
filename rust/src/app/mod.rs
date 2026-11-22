mod lifecycle;
mod runtime;
pub use runtime::run;

use std::sync::Arc;

/// Owns the observer for the full application lifecycle, including finalization.
pub struct Application {
    pub game: uqm_engine::Game,
    observer: Option<Arc<crate::automation::AutomationObserver>>,
}

impl Application {
    pub fn new(
        mut game: uqm_engine::Game,
        observer: Option<Arc<crate::automation::AutomationObserver>>,
    ) -> Self {
        game.world.set_catalog(std::borrow::Cow::Borrowed(
            &uqm_modes::planets::tables::STARMAP_ARRAY,
        ));
        Self { observer, game }
    }

    pub fn install_input(&mut self, input: uqm_engine::input::InputControls) {
        let runtime = crate::mainloop::input_abi::NativeInputRuntime::new(
            self.observer().cloned(),
            input,
            self.game.kernel.configuration.clone(),
        );
        uqm_modes::battle::services::install(&mut self.game.kernel);
        self.game.kernel.install_options_menu(|cx, io| {
            uqm_modes::front_end::gameopt::game_options(cx, io, None) != 0
        });
        self.game
            .kernel
            .install_ship_lifecycle(uqm_engine::battle::services::ShipLifecycle {
                init: crate::game_init::space_resources::init_battle_ships,
                uninit: crate::game_init::space_resources::uninit_ships,
            });
        self.game
            .kernel
            .install_save_slots(crate::savegame::live::SLOTS);
        self.game
            .kernel
            .install_input_runtime(std::rc::Rc::new(runtime));
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
        let app = Application::new(uqm_engine::Game::new(), None);
        assert!(std::ptr::eq(
            app.game.world.catalog(),
            &uqm_modes::planets::tables::STARMAP_ARRAY,
        ));
        assert!(app.game.world.selected_star().is_none());
    }
}
