//! Automation's implementation of the engine's ordered observation boundaries.

use super::{coordinator::AutomationObserver, input_admission, input_ffi};
use crate::engine::observe::{CheckpointView, Injected, InputView, Observer, Presented};
use std::sync::Arc;

pub(crate) struct Handle(pub(crate) Arc<AutomationObserver>);

impl Observer<Option<super::campaign_state::CampaignView>> for Handle {
    fn input_admitted(
        &mut self,
        view: &InputView,
        world: &Option<super::campaign_state::CampaignView>,
    ) -> Injected {
        let mut capture = || {
            world
                .as_ref()
                .unwrap_or_else(|| panic!("input admission without a campaign view"))
                .clone()
        };
        let stop = match *view {
            InputView::BeforeUpdate | InputView::Nested => {
                let coordinator = self.0.clone();
                input_ffi::service_input(
                    &coordinator,
                    &mut capture,
                    *view == InputView::Nested,
                    &mut input_admission::Unavailable,
                    self,
                ) != 0
            }
            InputView::AfterUpdate(activity) => {
                input_ffi::after_input_update(&self.0, activity) != 0
            }
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
                .service_and_should_skip_pending_communication_speech(&mut capture),
            InputView::PendingResponse { count } => {
                self.0.pending_communication_response_action(count)
            }
            InputView::ClosingSpeech => self.0.should_skip_communication_closing_speech(),
            InputView::CommunicationCompleted => self
                .0
                .service_communication_completion_boundary(&mut capture),
        };
        Injected { stop, seed: None }
    }

    fn input_admitted_with_setup(
        &mut self,
        view: &InputView,
        world: &Option<super::campaign_state::CampaignView>,
        setup: &mut input_admission::Target<'_>,
    ) -> Injected {
        if matches!(view, InputView::BeforeUpdate | InputView::Nested) {
            let coordinator = self.0.clone();
            let mut capture = || {
                world
                    .as_ref()
                    .unwrap_or_else(|| panic!("input admission without a campaign view"))
                    .clone()
            };
            let stop = input_ffi::service_input(
                &coordinator,
                &mut capture,
                *view == InputView::Nested,
                setup,
                self,
            ) != 0;
            Injected { stop, seed: None }
        } else {
            input_admission::legacy(setup, || self.input_admitted(view, world))
        }
    }

    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        match *view {
            CheckpointView::Simulation(boundary) => self.0.parity_checkpoint(boundary),
            CheckpointView::BattleReset => {}
            CheckpointView::BattleAdvanced => {
                self.0
                    .native_battle_frame_completed(crate::mainloop::types::ActivityValue(
                        self.0.activity.activity(),
                    ));
                self.0.parity_checkpoint("battle");
            }
            CheckpointView::InputSampled(activity) => self.0.parity_input(activity),
            CheckpointView::FlagshipDestroyed => self
                .0
                .flagship_destroyed
                .store(true, std::sync::atomic::Ordering::SeqCst),
            view => self.0.observe_ui_checkpoint(view),
        }
    }

    fn presented(&mut self, presented: &mut Presented) {
        presented.stop = self.0.process_present(presented.frame.take());
    }
}
