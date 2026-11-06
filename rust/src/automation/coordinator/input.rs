//! Input admission, fixture setup and scheduler commits.

use super::*;

/// Discard the repeat pulse of a menu key that a scripted tap only holds.
///
/// Returns `false` when the native owner has no such key.
pub(super) fn discard_held_repeat(
    held: Option<crate::automation::script::MenuKey>,
    discard: impl FnOnce(i32) -> bool,
) -> bool {
    held.is_none_or(|key| discard(i32::from(crate::automation::input::menu_key_to_index(key))))
}

impl AutomationObserver {
    pub(super) fn inject_menu_key(&self, index: i32, value: i32) -> bool {
        input_ffi::inject_menu_key(&self.input, index, value);
        !self.input.injection_rejected()
    }

    pub(super) fn inject_player_key(&self, index: i32, value: i32) -> bool {
        input_ffi::inject_player_key(&self.input, index, value);
        !self.input.injection_rejected()
    }

    #[must_use]
    pub fn start_scene_replaces_introduction(&self) -> bool {
        self.inner
            .lock()
            .pending_start_scene
            .replaces_introduction()
    }

    pub(super) fn activate_input_runtime() -> Arc<RuntimeModel> {
        let runtime = Arc::new(RuntimeModel::new());
        runtime.activate();
        runtime
    }

    pub(super) fn admit_boundary(&self, inner: &mut CoordInner, kind: CallbackKind) -> bool {
        if let Some(class) = self.runtime.mirror.terminal.load() {
            self.set_terminal(inner, class);
            return true;
        }
        let now = Instant::now();
        let entry = WatchdogEntry {
            kind,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed: now.duration_since(self.started_at),
            ready: inner.ready,
            idle: now.saturating_duration_since(inner.last_progress),
            clock: ClockSample {
                started_at: self.started_at,
                last_observed: inner.last_observed,
                now,
            },
        };
        let result = watchdog_reduce(&entry, &self.watchdog_limits);
        inner.last_observed = now;
        inner.input_seen = result.candidate_input_seen;
        inner.present_seen = result.candidate_present_seen;
        if result.outcome == WatchdogOutcome::WallTimeout && self.native_trace.is_some() {
            #[cfg(feature = "debug-process")]
            let expected = super::super::native_window::active_native_window_config()
                .and_then(|config| config.runtime_contract.expected_input_ticks);
            #[cfg(not(feature = "debug-process"))]
            let expected = None;
            let report = super::super::native_timing::timeout_report(
                entry.elapsed,
                inner.input_seen,
                expected,
            );
            eprintln!("{report}");
            self.write_trace_labeled(inner, RecordKind::SemanticAssertion, report);
        }
        if let Some(class) = watchdog_terminal_class(result.outcome) {
            self.set_terminal(inner, class);
            return true;
        }
        inner.last_progress = now;
        false
    }

    pub(super) fn process_input_at_boundary(
        &self,
        capture: &mut dyn FnMut() -> campaign_state::CampaignView,
        button_wait: bool,
    ) -> bool {
        self.process_input_admission(
            capture,
            button_wait,
            &mut crate::automation::input_admission::Unavailable,
            &mut crate::engine::observe::Unobserved,
        )
    }

    pub(crate) fn process_input_admission(
        &self,
        capture: &mut dyn FnMut() -> campaign_state::CampaignView,
        button_wait: bool,
        setup: &mut crate::automation::input_admission::Target<'_>,
        sink: &mut dyn crate::engine::observe::Observer<Option<campaign_state::CampaignView>>,
    ) -> bool {
        let mut inner = self.inner.lock();
        let event = match crate::automation::input_admission::legacy(setup, || {
            self.prepare_input_boundary(&mut inner, capture, button_wait)
        }) {
            std::ops::ControlFlow::Break(stop) => return stop,
            std::ops::ControlFlow::Continue(event) => event,
        };
        if self.parity_fixture_input(&mut inner, event, setup, sink) {
            return true;
        }
        crate::automation::input_admission::legacy(setup, || {
            self.queue_input_fixture(&mut inner, event)
                || self.commit_input_event(&mut inner, event)
        })
    }

    fn prepare_input_boundary(
        &self,
        inner: &mut CoordInner,
        capture: &mut dyn FnMut() -> campaign_state::CampaignView,
        button_wait: bool,
    ) -> std::ops::ControlFlow<bool, SchedulerEvent> {
        if inner.terminal_class.is_some() {
            return std::ops::ControlFlow::Break(true);
        }
        if self.reject_unobserved_player_input(inner) {
            return std::ops::ControlFlow::Break(true);
        }

        let released_semantic_select = inner.release_semantic_select;
        if released_semantic_select {
            let index = i32::from(crate::automation::script::MenuKey::Select.index());
            if !self.inject_menu_key(index, 0) {
                self.reject_input(inner, "menu", index, 0);
                return std::ops::ControlFlow::Break(true);
            }
            inner.release_semantic_select = false;
        }
        let orbit_transition_pending = inner.orbit.transition_pending;
        inner.orbit.transition_pending = false;

        if self.admit_boundary(inner, CallbackKind::Input) {
            return std::ops::ControlFlow::Break(true);
        }
        if self.save_input(inner, button_wait, capture) {
            return std::ops::ControlFlow::Break(true);
        }

        if self.reserve_nested_button_poll(inner, button_wait) {
            return std::ops::ControlFlow::Break(false);
        }

        // Keep the next action behind the committed-presentation barrier.
        if self.native_checkpoint_pending(inner) {
            return std::ops::ControlFlow::Break(false);
        }
        if inner.pending_main_menu_ready {
            inner.pending_main_menu_ready = false;
            return std::ops::ControlFlow::Break(self.apply_main_menu_ready(inner));
        }
        let world = &capture();
        if self.verify_runtime_assertions(world, inner)
            || self.verify_activity_assertion(world, inner)
        {
            return std::ops::ControlFlow::Break(true);
        }

        // Step 2: Feed to scheduler. Runtime waits are satisfied only by
        // observed production state, never by injected input.
        let mut scheduler_event = self.service_semantic_waits(
            world,
            inner,
            released_semantic_select,
            orbit_transition_pending,
        );
        if inner.terminal_class.is_some() {
            return std::ops::ControlFlow::Break(true);
        }

        scheduler_event = self.service_navigation(world, inner, scheduler_event);

        if inner.terminal_class.is_some() {
            return std::ops::ControlFlow::Break(true);
        }
        std::ops::ControlFlow::Continue(scheduler_event)
    }

    fn reserve_nested_button_poll(&self, inner: &CoordInner, button_wait: bool) -> bool {
        // Autoscan polls release keys and spend run budgets, but cannot select a menu item.
        button_wait
            && matches!(
                self.actions.get(inner.sched_state.step_index),
                Some(Action::SelectPlanetMenu(_))
            )
    }

    fn queue_input_fixture(&self, inner: &mut CoordInner, scheduler_event: SchedulerEvent) -> bool {
        if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(Action::SetupMeleeMutualLossFixture(_))
        ) && inner.sched_state.phase == ActionPhase::WaitingForInput
            && scheduler_event == SchedulerEvent::AdmittedInput
        {
            match crate::automation::melee_fixture::trigger(&|| self.parity_checkpoint("fixture")) {
                Ok(()) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    "melee_mutual_loss_fixture_triggered".to_string(),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        format!("melee_mutual_loss_fixture_failed:{error}"),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        // The setup_planet_side_collision_fixture action is explicit and
        // one-shot: only the current action queues, only once.  A duplicate
        // while a request is still pending is a fail-fast semantic
        // mismatch instead of an idempotent success.
        if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(
                Action::SetupPlanetSideCollisionFixture(_)
                    | Action::SetupPlanetSideMineralGallery(_)
            )
        ) && inner.sched_state.phase == ActionPhase::WaitingForInput
            && scheduler_event == SchedulerEvent::AdmittedInput
        {
            let gallery = matches!(
                self.actions.get(inner.sched_state.step_index),
                Some(Action::SetupPlanetSideMineralGallery(_))
            );
            let request = if gallery {
                crate::planet_side::automation_fixture::coordinator_queues_mineral_gallery_request()
            } else {
                crate::planet_side::automation_fixture::coordinator_queues_fixture_request()
            };
            match request {
                Ok(()) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        if gallery {
                            "planet_side_mineral_gallery_queued"
                        } else {
                            "planet_side_collision_fixture_queued"
                        }
                        .to_string(),
                    );
                }
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        format!(
                            "planet_side_collision_fixture_duplicate:{}",
                            error.operation
                        ),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        false
    }

    fn commit_input_event(&self, inner: &mut CoordInner, scheduler_event: SchedulerEvent) -> bool {
        let config = SchedulerConfig {
            actions: &self.actions,
            transitions: &self.transitions,
        };
        let transition = scheduler_reduce(&inner.sched_state, &config, scheduler_event);
        let previous_state = inner.sched_state;
        inner.sched_state = transition.new_state;

        // Step 3: Apply effects.
        if !self.apply_effects(inner, &transition.effects, None) {
            inner.sched_state =
                scheduler_state_after_effects(previous_state, transition.new_state, false);
            return true;
        }

        // Step 4: Write trace.
        self.write_trace(inner, RecordKind::InputTick);

        // Step 4b: Replay menu transitions that arrived before the scheduler
        // was ready for them.
        self.replay_pending_transitions(inner);

        // Step 5: Check terminal.
        if inner.sched_state.is_terminal() {
            if terminal_waits_for_player_input_observation(
                inner.sched_state,
                inner.pending_player_input.is_some(),
            ) {
                return false;
            }
            let class = map_scheduler_terminal(inner.sched_state.terminal);
            eprintln!(
                "[automation] scheduler terminal: {:?} -> class={:?}",
                inner.sched_state.terminal, class
            );
            if class == TerminalClass::SemanticMismatch {
                self.trace_unsatisfied_planet_side_wait(inner);
            }
            self.set_terminal(inner, class);
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_and_terminal_admission_do_not_read_campaign_owners() {
        let source = serde_json::to_vec(&serde_json::json!({
            "version": 1, "name": "lazy-owner",
            "steps": [
                {"action": "select_planet_menu", "phase": "dispatch", "max_ticks": 2},
                {"action": "finish"}
            ],
            "budgets": {"max_input_ticks": 10, "max_presentations": 100, "max_wallclock_seconds": 60}
        })).unwrap();
        let script = crate::automation::validate_script(
            crate::automation::parse_script(&source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let observer = AutomationObserver::new(
            script,
            temp.path().into(),
            crate::engine::World::new().activity_control(),
            crate::engine::input::InputControls::default(),
            Default::default(),
        );
        let mut capture = || panic!("admission must not acquire a campaign owner");
        assert!(!observer.process_input_at_boundary(&mut capture, true));
        assert_eq!(observer.inner.lock().input_seen, 1);
        assert_eq!(observer.inner.lock().sched_state, SchedulerState::initial());
        observer
            .runtime
            .mirror
            .terminal
            .try_set(TerminalClass::SemanticMismatch);
        assert_eq!(
            input_ffi::service_input(
                &observer,
                &mut capture,
                false,
                &mut crate::automation::input_admission::Unavailable,
                &mut crate::engine::observe::Unobserved
            ),
            1
        );
        assert_eq!(input_ffi::after_input_update(&observer, 0), 1);
        assert!(observer.process_input_at_boundary(&mut capture, false));
        assert_eq!(observer.inner.lock().input_seen, 1);
    }

    #[test]
    fn setup_effects_complete_inside_the_admitted_scheduler_transaction() {
        use crate::engine::observe::{CampaignView, InputSetup, InputView, Observer, PeerSetup};
        struct Setup<'a> {
            coordinator: &'a AutomationObserver,
            effects: Vec<&'static str>,
            ready: bool,
        }
        impl InputSetup<Option<CampaignView>> for Setup<'_> {
            fn configure_fleet(&mut self, ships: [[u8; 7]; 2], cyborg: bool) -> Result<(), String> {
                assert_eq!(ships, [[0; 7]; 2]);
                assert!(cyborg);
                assert!(self.coordinator.inner.try_lock().is_none());
                self.effects.push("configure");
                Ok(())
            }
            fn connect_peer(
                &mut self,
                player: u16,
                server: bool,
                port: u16,
                observer: &mut dyn Observer<Option<CampaignView>>,
            ) -> Result<Option<u16>, String> {
                assert_eq!((player, server, port), (1, true, 0));
                assert!(self.coordinator.inner.try_lock().is_none());
                assert!(observer.input_admitted(&InputView::Active, &None).stop);
                self.effects.push("connect");
                Ok(Some(2020))
            }
            fn peer_setup(&self, _: u16) -> Option<PeerSetup> {
                Some(PeerSetup {
                    ready: self.ready,
                    disconnected: false,
                })
            }
            fn poll_peer(&mut self, observer: &mut dyn Observer<Option<CampaignView>>) {
                assert!(self.coordinator.inner.try_lock().is_none());
                assert!(observer.input_admitted(&InputView::Active, &None).stop);
                self.effects.push("poll");
                self.ready = true;
            }
            fn with_legacy(&mut self, operation: &mut dyn FnMut()) {
                operation();
            }
        }
        let source = serde_json::to_vec(&serde_json::json!({
            "version": 1, "name": "borrowed-setup", "steps": [
                {"action":"configure_parity_melee","ships":[[0,0,0,0,0,0,0],[0,0,0,0,0,0,0]],"cyborg":true},
                {"action":"connect_netplay","remote_player":1,"server":true,"port":0},
                {"action":"finish"}
            ],
            "budgets":{"max_input_ticks":10,"max_presentations":100,"max_wallclock_seconds":60}
        })).unwrap();
        let script = crate::automation::validate_script(
            crate::automation::parse_script(&source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let coordinator = AutomationObserver::new(
            script,
            directory.path().into(),
            crate::engine::World::new().activity_control(),
            crate::engine::input::InputControls::default(),
            Default::default(),
        );
        let mut handle = crate::automation::observer::Handle(coordinator.clone());
        let mut setup = Setup {
            coordinator: &coordinator,
            effects: vec![],
            ready: false,
        };
        let world = Some(CampaignView::empty());
        assert!(
            !handle
                .input_admitted_with_setup(&InputView::BeforeUpdate, &world, &mut setup)
                .stop
        );
        assert_eq!(coordinator.inner.lock().sched_state.step_index, 1);
        assert!(
            !handle
                .input_admitted_with_setup(&InputView::BeforeUpdate, &world, &mut setup)
                .stop
        );
        assert_eq!(coordinator.inner.lock().sched_state.step_index, 2);
        assert_eq!(coordinator.inner.lock().input_seen, 2);
        assert_eq!(setup.effects, ["configure", "connect", "poll"]);
    }

    #[test]
    fn fixture_failures_and_button_wait_watchdog_are_terminal(
    ) -> Result<(), Box<dyn std::error::Error>> {
        const CHILD: &str = "UQM_INPUT_BOUNDARY_FAILURE";
        let Some(case) = std::env::var_os(CHILD) else {
            for case in ["melee", "collision", "gallery", "watchdog"] {
                let status = std::process::Command::new(std::env::current_exe()?)
                    .args(["--exact", "automation::coordinator::input::tests::fixture_failures_and_button_wait_watchdog_are_terminal"])
                    .env(CHILD, case)
                    .status()?;
                assert!(status.success(), "{case}");
            }
            return Ok(());
        };
        let case = case.to_string_lossy();
        let steps = match case.as_ref() {
            "melee" => serde_json::json!([
                {"action": "setup_melee_mutual_loss_fixture"}, {"action": "finish"}
            ]),
            "collision" | "gallery" => {
                let action = if case == "collision" {
                    "setup_planet_side_collision_fixture"
                } else {
                    "setup_planet_side_mineral_gallery"
                };
                serde_json::json!([{"action": action}, {"action": action}, {"action": "finish"}])
            }
            "watchdog" => serde_json::json!([
                {"action": "select_planet_menu", "phase": "dispatch", "max_ticks": 2},
                {"action": "finish"}
            ]),
            _ => unreachable!(),
        };
        let source = serde_json::to_vec(&serde_json::json!({
            "version": 1, "name": "input-failure", "steps": steps,
            "budgets": {"max_input_ticks": 10, "max_presentations": 100, "max_wallclock_seconds": 60}
        }))?;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(&source, "test.json")?,
            "test.json",
        )?;
        let temp = tempfile::tempdir()?;
        let _app = {
            let game = crate::engine::Game::new();
            let observer = Some(AutomationObserver::new(
                script,
                temp.path().into(),
                game.world.activity_control(),
                crate::engine::input::InputControls::default(),
                Default::default(),
            ));
            crate::app::Application::new(game, observer)
        };
        let coord = _app
            .observer()
            .cloned()
            .ok_or("coordinator not initialized")?;
        if case == "watchdog" {
            let mut stopped = false;
            for _ in 0..11 {
                if coord.process_input_at_boundary(&mut campaign_state::empty_view, true) {
                    stopped = true;
                    break;
                }
            }
            assert!(
                stopped,
                "nested waits must not bypass the global input budget"
            );
            assert_eq!(coord.inner.lock().sched_state, SchedulerState::initial());
        } else {
            if case != "melee" {
                assert!(!coord.process_input_inner(&mut campaign_state::empty_view));
                assert_eq!(coord.inner.lock().sched_state.step_index, 1);
            }
            assert!(coord.process_input_inner(&mut campaign_state::empty_view));
            assert_eq!(
                coord.inner.lock().terminal_class,
                Some(TerminalClass::SemanticMismatch)
            );
        }
        assert!(_app.observer().unwrap().finalize(0)? != 0);
        Ok(())
    }
}
