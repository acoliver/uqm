//! Input admission, fixture setup and scheduler commits.

use super::*;

impl Coordinator {
    pub(super) fn process_input_at_boundary(&self, button_wait: bool) -> bool {
        let mut inner = self.inner.lock();

        if inner.terminal_class.is_some() {
            return true;
        }
        if self.reject_unobserved_player_input(&mut inner) {
            return true;
        }

        let released_semantic_select = inner.release_semantic_select;
        if released_semantic_select {
            let index = i32::from(crate::automation::script::MenuKey::Select.index());
            if !inject_menu_key(index, 0) {
                self.reject_input(&mut inner, "menu", index, 0);
                return true;
            }
            inner.release_semantic_select = false;
        }
        let orbit_transition_pending = inner.orbit_transition_pending;
        inner.orbit_transition_pending = false;

        let now = Instant::now();
        let elapsed = now.duration_since(self.started_at);

        // Step 1: Watchdog check.
        let entry = WatchdogEntry {
            kind: CallbackKind::Input,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed,
            ready: inner.ready,
            idle: now.saturating_duration_since(inner.last_progress),
            clock: ClockSample {
                started_at: self.started_at,
                last_observed: inner.last_observed,
                now,
            },
        };

        let wd_result = watchdog_reduce(&entry, &self.watchdog_limits);
        inner.last_observed = now;

        // Update counters from watchdog result (post-increment values).
        inner.input_seen = wd_result.candidate_input_seen;
        inner.present_seen = wd_result.candidate_present_seen;

        if let Some(class) = watchdog_terminal_class(wd_result.outcome) {
            self.set_terminal(&mut inner, class);
            return true;
        }
        // Admitted: this callback is progress for the idle watchdog.
        inner.last_progress = now;

        // A synchronous scan's button polls cannot deliver menu selections.
        // Release the previous select and check run budgets above, but reserve
        // the menu action's admission budget for its actual DoInput consumer.
        if button_wait
            && matches!(
                self.actions.get(inner.sched_state.step_index),
                Some(Action::SelectPlanetMenu(_))
            )
        {
            return false;
        }

        // Keep the next action behind the committed-presentation barrier.
        if self.native_checkpoint_pending(&inner) {
            return false;
        }
        if inner.pending_main_menu_ready {
            inner.pending_main_menu_ready = false;
            return self.apply_main_menu_ready(&mut inner);
        }
        if self.verify_runtime_assertions(&mut inner) || self.verify_activity_assertion(&mut inner)
        {
            return true;
        }

        // Step 2: Feed to scheduler. Runtime waits are satisfied only by
        // observed production state, never by injected input.
        let mut scheduler_event = self.service_semantic_waits(
            &mut inner,
            released_semantic_select,
            orbit_transition_pending,
        );
        if inner.terminal_class.is_some() {
            return true;
        }

        scheduler_event = self.service_navigation(&mut inner, scheduler_event);

        if inner.terminal_class.is_some() {
            return true;
        }
        if self.queue_input_fixture(&mut inner, scheduler_event) {
            return true;
        }
        self.commit_input_event(&mut inner, scheduler_event)
    }

    fn queue_input_fixture(&self, inner: &mut CoordInner, scheduler_event: SchedulerEvent) -> bool {
        if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(Action::SetupMeleeMutualLossFixture(_))
        ) && inner.sched_state.phase == ActionPhase::WaitingForInput
            && scheduler_event == SchedulerEvent::AdmittedInput
        {
            match crate::automation::melee_fixture::trigger() {
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
        Coordinator::init(script, temp.path().into());
        let coord = Coordinator::get().ok_or("coordinator not initialized")?;
        if case == "watchdog" {
            let mut stopped = false;
            for _ in 0..11 {
                if coord.process_input_at_boundary(true) {
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
                assert!(!coord.process_input_inner());
                assert_eq!(coord.inner.lock().sched_state.step_index, 1);
            }
            assert!(coord.process_input_inner());
            assert_eq!(
                coord.inner.lock().terminal_class,
                Some(TerminalClass::SemanticMismatch)
            );
        }
        assert!(Coordinator::finalize(0)? != 0);
        Ok(())
    }
}
