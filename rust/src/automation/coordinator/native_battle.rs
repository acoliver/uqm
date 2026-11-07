//! Native-only battle samples use the existing ordered trace reservation.

use super::*;
use crate::automation::trace::NativeBattleEvidence;
use crate::engine::observe::battle::{BattleTick, ReplacementGate};

impl AutomationObserver {
    pub(crate) fn native_battle_diagnostics(&self) -> bool {
        self.native_trace.is_some() && !crate::time::source::is_replay()
    }

    pub(crate) fn trace_battle_tick(&self, tick: &BattleTick) {
        self.trace_native_battle(Some(*tick), None);
    }

    pub(crate) fn trace_replacement_gate(&self, gate: &ReplacementGate) {
        self.trace_native_battle(None, Some(*gate));
    }

    fn trace_native_battle(&self, tick: Option<BattleTick>, gate: Option<ReplacementGate>) {
        if !self.native_battle_diagnostics() {
            return;
        }
        let mut inner = self.inner.lock();
        if inner.terminal_class.is_some() {
            return;
        }
        let now = self.elapsed_ms();
        let phase = match tick.map(|tick| tick.phase) {
            Some(crate::engine::observe::battle::BattlePhase::Input) => 0,
            Some(crate::engine::observe::battle::BattlePhase::Advanced) => 1,
            None => 2,
        };
        let delta_ms = inner.native_battle_previous_ms[phase].map(|previous| now - previous);
        inner.native_battle_previous_ms[phase] = Some(now);
        let (generation, active) = self.ui.ship_picker_observation();
        let reservation = self.runtime.commit.reserve();
        let record = TraceRecord {
            schema: TraceRecord::BATTLE_SCHEMA,
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
                frame: crate::collections::observation::current_frame(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::observe::battle::BattlePhase;

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
            crate::engine::World::new().activity_control(),
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
