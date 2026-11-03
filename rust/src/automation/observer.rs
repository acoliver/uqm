//! Automation's implementation of the engine's ordered observation boundaries.

use super::{coordinator::AutomationObserver, input_ffi, ui_observation as ui};
use crate::engine::observe::{CheckpointView, Injected, InputView, Observer, Presented};
use std::sync::Arc;

pub(crate) struct Handle(pub(crate) Arc<AutomationObserver>);

impl Observer for Handle {
    fn input_admitted(&mut self, view: &InputView) -> Injected {
        let stop = match *view {
            InputView::BeforeUpdate => input_ffi::service_input(false) != 0,
            InputView::Nested => input_ffi::service_input(true) != 0,
            InputView::AfterUpdate => input_ffi::after_input_update() != 0,
            InputView::Active => true,
            InputView::MainMenuReady => AutomationObserver::process_main_menu_ready(),
            InputView::MenuTransition { from, to } => {
                AutomationObserver::process_menu_transition(from, to)
            }
            InputView::QuitSelected => {
                AutomationObserver::observe_quit_selected();
                false
            }
            InputView::ReplacesIntroduction => {
                AutomationObserver::start_scene_replaces_introduction()
            }
            InputView::Seed { domain, fallback } => {
                return Injected {
                    stop: false,
                    seed: Some(super::coordinator::seed_value(domain, fallback)),
                }
            }
            InputView::PendingSpeech => {
                AutomationObserver::service_and_should_skip_pending_communication_speech()
            }
            InputView::PendingResponse { count } => {
                AutomationObserver::pending_communication_response_action(count)
            }
            InputView::ClosingSpeech => {
                AutomationObserver::should_skip_communication_closing_speech()
            }
            InputView::CommunicationCompleted => {
                AutomationObserver::service_communication_completion_boundary()
            }
        };
        Injected { stop, seed: None }
    }

    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        match *view {
            CheckpointView::Simulation(boundary) => AutomationObserver::parity_checkpoint(boundary),
            CheckpointView::BattleReset => {}
            CheckpointView::BattleAdvanced => {
                AutomationObserver::native_battle_frame_completed(
                    crate::mainloop::ffi::get_current_activity(),
                );
                AutomationObserver::parity_checkpoint("battle");
            }
            CheckpointView::InputSampled => AutomationObserver::parity_input(),
            CheckpointView::GameInitialized => {
                // SAFETY: game initialization publishes the lifecycle context for this synchronous hook.
                unsafe { crate::app::bridge::enter(AutomationObserver::activate_start_scene) }
            }
            CheckpointView::FlagshipDestroyed => self
                .0
                .flagship_destroyed
                .store(true, std::sync::atomic::Ordering::SeqCst),
            CheckpointView::ShipEscaped => ui::record_ship_escape(),
            CheckpointView::EncounterDispatch(id) => {
                super::scenario::observe_encounter_dispatch(id)
            }
            CheckpointView::DialogueDispatch(id) => super::scenario::observe_dialogue_dispatch(id),
            CheckpointView::EncounterMenu(active) => ui::observe_encounter_menu(active),
            CheckpointView::CommunicationResponses(count) => {
                ui::observe_communication_responses(count)
            }
            CheckpointView::ResponsesReady(ready) => ui::set_communication_responses_ready(ready),
            CheckpointView::CommunicationStarted => ui::begin_communication(),
            CheckpointView::CommunicationEnded => ui::complete_communication(),
            CheckpointView::CommunicationReplay(active) => ui::observe_communication_replay(active),
            CheckpointView::GameOptions(active) => {
                ui::rust_automation_observe_game_options(i32::from(active))
            }
            CheckpointView::PlanetMenu(phase) => ui::rust_automation_observe_planet_menu(phase),
            CheckpointView::StarbaseMenu(active) => ui::observe_starbase_menu(active),
            CheckpointView::Won => super::ending::won(),
            CheckpointView::PresentationStarted => super::ending::presentation_started(),
            CheckpointView::PresentationCancelled => super::ending::presentation_cancelled(),
            CheckpointView::PresentationCompleted => super::ending::presentation_completed(),
            CheckpointView::CreditsStarted => super::ending::credits_started(),
            CheckpointView::CreditsReady => super::ending::credits_controls_ready(),
            CheckpointView::CreditsCompleted => super::ending::credits_completed(),
        }
    }

    fn presented(&mut self, presented: &mut Presented) {
        presented.stop = AutomationObserver::process_present(presented.frame.take());
    }
}
