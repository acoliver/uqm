//! Automation's implementation of the engine's ordered observation boundaries.

use super::{coordinator::AutomationObserver, input_ffi, ui_observation as ui};
use crate::engine::observe::{CheckpointView, Injected, InputView, Observer, Presented};
use std::sync::Arc;

pub(crate) struct Handle<'a>(
    pub(crate) Arc<AutomationObserver>,
    pub(crate) &'a mut dyn FnMut() -> super::campaign_state::CampaignView,
);

impl Observer for Handle<'_> {
    fn input_admitted(&mut self, view: &InputView) -> Injected {
        let stop = match *view {
            InputView::BeforeUpdate => input_ffi::service_input(&self.0, self.1, false) != 0,
            InputView::Nested => input_ffi::service_input(&self.0, self.1, true) != 0,
            InputView::AfterUpdate => input_ffi::after_input_update(&self.0) != 0,
            InputView::Active => true,
            InputView::MainMenuReady => self.0.process_main_menu_ready(),
            InputView::MenuTransition { from, to } => self.0.process_menu_transition(from, to),
            InputView::QuitSelected => {
                self.0.observe_quit_selected();
                false
            }
            InputView::ReplacesIntroduction => self.0.start_scene_replaces_introduction(),
            InputView::Seed { domain, fallback } => {
                return Injected {
                    stop: false,
                    seed: Some(self.0.seed_value(domain, fallback)),
                }
            }
            InputView::PendingSpeech => self
                .0
                .service_and_should_skip_pending_communication_speech(self.1),
            InputView::PendingResponse { count } => {
                self.0.pending_communication_response_action(count)
            }
            InputView::ClosingSpeech => self.0.should_skip_communication_closing_speech(),
            InputView::CommunicationCompleted => {
                self.0.service_communication_completion_boundary(self.1)
            }
        };
        Injected { stop, seed: None }
    }

    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        match *view {
            CheckpointView::Simulation(boundary) => self.0.parity_checkpoint(boundary),
            CheckpointView::BattleReset => {}
            CheckpointView::BattleAdvanced => {
                AutomationObserver::native_battle_frame_completed(
                    crate::mainloop::ffi::get_current_activity(),
                );
                self.0.parity_checkpoint("battle");
            }
            CheckpointView::InputSampled => self.0.parity_input(),
            CheckpointView::GameInitialized => {
                crate::app::observe::game_initialized(&self.0);
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
        presented.stop = self.0.process_present(presented.frame.take());
    }
}
