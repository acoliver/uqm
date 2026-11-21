//! Native-only battle samples use the existing ordered trace reservation.

use super::*;
use crate::automation::trace::NativeBattleEvidence;
use uqm_engine::observe::battle::{BattleTick, ReplacementGate};

impl AutomationObserver {
    pub(crate) fn native_diagnostics(&self) -> bool {
        self.native_trace.is_some() && !uqm_engine::host::is_replay()
    }

    pub(crate) fn observes_pkunk_rebirth(&self) -> bool {
        self.actions
            .iter()
            .any(|action| matches!(action, Action::WaitForPkunkRebirth(_)))
    }

    pub(crate) fn reset_pkunk_observation(&self) {
        self.inner.lock().pkunk_rebirth.finish();
    }

    pub(crate) fn trace_battle_tick(&self, tick: &BattleTick) {
        if self.observes_pkunk_rebirth() {
            self.inner
                .lock()
                .pkunk_rebirth
                .observe(uqm_engine::collections::observation::current_frame(), tick);
        }
        self.trace_native_battle(Some(*tick), None);
    }

    pub(crate) fn trace_replacement_gate(&self, gate: &ReplacementGate) {
        self.trace_native_battle(None, Some(*gate));
    }

    fn trace_native_battle(&self, tick: Option<BattleTick>, gate: Option<ReplacementGate>) {
        if !self.native_diagnostics() {
            return;
        }
        let mut inner = self.inner.lock();
        if inner.terminal_class.is_some() {
            return;
        }
        let now = self.elapsed_ms();
        let phase = match tick.map(|tick| tick.phase) {
            Some(uqm_engine::observe::battle::BattlePhase::Input) => 0,
            Some(uqm_engine::observe::battle::BattlePhase::Advanced) => 1,
            None => 2,
        };
        let delta_ms = inner.native_battle_previous_ms[phase].map(|previous| now - previous);
        inner.native_battle_previous_ms[phase] = Some(now);
        let (generation, active) = self.ui.ship_picker_observation();
        let reservation = self.runtime.commit.reserve();
        let record = TraceRecord {
            schema: TraceRecord::NATIVE_SCHEMA,
            run: 1,
            sequence: reservation.sequence(),
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: now,
            kind: RecordKind::NativeBattle,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: None,
            activity: None,
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
            native_battle: Some(NativeBattleEvidence {
                frame: uqm_engine::collections::observation::current_frame(),
                tick,
                gate,
                picker_active: active,
                picker_generation: generation,
                delta_ms,
            }),
        };
        reservation.commit_record(
            record
                .to_jsonl()
                .unwrap_or_else(|error| panic!("serialize native battle sample: {error}")),
        );
    }
}

impl AutomationObserver {
    pub(super) fn service_battle_wait(
        &self,
        inner: &mut CoordInner,
        world: &campaign_state::CampaignView,
        mut scheduler_event: SchedulerEvent,
    ) -> Option<SchedulerEvent> {
        match self.actions.get(inner.sched_state.step_index)? {
            Action::WaitForBattleFrames(wait) => {
                self.service_battle_frames(inner, wait, scheduler_event)
            }
            Action::WaitForPkunkRebirth(wait) => {
                self.service_pkunk_wait(inner, *wait, scheduler_event)
            }
            Action::WaitForBattleShipLoss(wait) => {
                match inner.battle_loss.observe(
                    mode::require_live(world, &self.ui, self.scene.lock().active_scene()),
                    crate::automation::battle_observer::current_frame(),
                    battle_outcome::live_fleets(world),
                    wait.minimum_ships,
                ) {
                    Ok(Some(proof)) => {
                        scheduler_event = SchedulerEvent::ConditionReached;
                        self.write_trace_labeled(
                            inner,
                            RecordKind::SemanticAssertion,
                            proof.trace_label(),
                        );
                    }
                    Ok(None) => {}
                    Err(error) => {
                        self.write_trace_labeled(
                            inner,
                            RecordKind::SemanticAssertion,
                            error.trace_label(),
                        );
                        self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    }
                }
                Some(scheduler_event)
            }
            _ => None,
        }
    }
    fn service_battle_frames(
        &self,
        inner: &mut CoordInner,
        wait: &crate::automation::script::WaitForBattleFramesStep,
        mut scheduler_event: SchedulerEvent,
    ) -> Option<SchedulerEvent> {
        let actual = crate::automation::battle_observer::current_frame();
        if battle_frame_floor_reached(actual, wait.minimum) {
            inner.verified_battle_frames = inner.verified_battle_frames.max(actual);
            scheduler_event = SchedulerEvent::ConditionReached;
            self.write_trace_labeled(
                inner,
                RecordKind::SemanticAssertion,
                format!(
                    "battle_frames_reached:count={actual}:minimum={}",
                    wait.minimum
                ),
            );
        }
        Some(scheduler_event)
    }
    fn service_pkunk_wait(
        &self,
        inner: &mut CoordInner,
        wait: crate::automation::pkunk_rebirth::WaitStep,
        mut scheduler_event: SchedulerEvent,
    ) -> Option<SchedulerEvent> {
        inner.pkunk_rebirth.begin(wait.side);
        if let Some(proof) = inner.pkunk_rebirth.proof(wait) {
            scheduler_event = SchedulerEvent::ConditionReached;
            self.write_trace_labeled(inner, RecordKind::SemanticAssertion, proof.trace_label());
            if wait.phase == crate::automation::pkunk_rebirth::Phase::Restored {
                inner.pkunk_rebirth.finish();
            }
        }
        Some(scheduler_event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uqm_engine::observe::battle::BattlePhase;

    fn make_observer(root: &std::path::Path, native: bool) -> Arc<AutomationObserver> {
        let bytes = br#"{"version":1,"name":"battle-trace","steps":[{"action":"finish"}],"budgets":{"max_input_ticks":10,"max_presentations":100,"max_wallclock_seconds":60}}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(bytes, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        AutomationObserver::init_with_native_checkpoints(
            script,
            root.into(),
            native.then_some(16384),
            uqm_engine::World::new().activity_control(),
            Default::default(),
            Default::default(),
        )
    }

    fn tick() -> BattleTick {
        BattleTick {
            phase: BattlePhase::Input,
            activity: 0x202,
            keys: [[0; 7]; 2],
            pulsed: [[0; 7]; 2],
            menu: [0; 24],
            menu_pulsed: [0; 24],
            rng: 12345,
            ditty: 1,
            ships: [None; 2],
            counters: [1, 1],
        }
    }

    #[test]
    fn native_samples_retain_keys_picker_rng_gate_and_timing_without_headless_records() {
        let native = tempfile::tempdir().unwrap();
        let observer = make_observer(native.path(), true);
        observer.ui.observe_ship_picker(true);
        let mut sampled = tick();
        sampled.keys[1][4] = 7;
        observer.trace_battle_tick(&sampled);
        observer.trace_replacement_gate(&ReplacementGate {
            ditty_before: 1,
            track_playing: Some(false),
            ditty_after: 0,
            blocked: false,
        });
        observer.trace_battle_tick(&sampled);
        observer.flush_native_trace().unwrap();
        let records: Vec<TraceRecord> = std::fs::read_to_string(native.path().join("trace.jsonl"))
            .unwrap()
            .lines()
            .map(|line| TraceRecord::from_jsonl(line).unwrap())
            .filter(|record| record.kind == RecordKind::NativeBattle)
            .collect();
        assert_eq!(records.len(), 3);
        let evidence = records[0].native_battle.as_ref().unwrap();
        assert_eq!(evidence.tick, Some(sampled));
        assert!(evidence.picker_active);
        assert_eq!(evidence.picker_generation, 1);
        assert_eq!(evidence.delta_ms, None);
        assert_eq!(records[0].schema, 3);
        assert_eq!(
            records[1]
                .native_battle
                .as_ref()
                .unwrap()
                .gate
                .unwrap()
                .track_playing,
            Some(false)
        );
        assert!(records[2]
            .native_battle
            .as_ref()
            .unwrap()
            .delta_ms
            .is_some());

        let plain = tempfile::tempdir().unwrap();
        let observer = make_observer(plain.path(), false);
        let before = observer.runtime.commit.next_sequence();
        observer.trace_battle_tick(&sampled);
        observer.trace_replacement_gate(&ReplacementGate {
            ditty_before: 0,
            track_playing: None,
            ditty_after: 0,
            blocked: false,
        });
        assert_eq!(observer.runtime.commit.next_sequence(), before);
    }
}
