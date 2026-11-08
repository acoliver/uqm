//! Native-only graphics observations immediately before renderer readback.

use super::*;
use crate::graphics::presentation_diagnostics::PresentationSample;

pub(crate) const MAIN_MENU_COMMITTED_OBSERVATION: &str = "committed_non_uniform_menu_frame";

impl AutomationObserver {
    pub(super) fn native_checkpoint_pending(&self, inner: &CoordInner) -> bool {
        self.native_checkpoint_plan.as_ref().is_some_and(|plan| {
            plan.checkpoints_between(inner.checkpoint_step..inner.sched_state.step_index)
                .next()
                .is_some()
        })
    }

    pub(super) fn commit_native_checkpoints(
        &self,
        inner: &mut CoordInner,
        frame: &PresentedFrame,
    ) -> bool {
        let Some(plan) = &self.native_checkpoint_plan else {
            return false;
        };
        for checkpoint in
            plan.checkpoints_between(inner.checkpoint_step..inner.sched_state.step_index)
        {
            if matches!(checkpoint.action, Action::NavigateToHyperspace(_))
                && inner.departure.native_frame_pending
            {
                return false;
            }
            if matches!(checkpoint.action, Action::WaitForMainMenuReady(_)) {
                if !frame
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[..3] != frame.rgba[..3])
                {
                    return false;
                }
                self.write_committed_menu_ready(inner, frame);
            }
            let id = checkpoint.id.clone();
            let reservation = self.runtime.commit.reserve();
            let mut record = crate::automation::capture::capture_trace_record(
                reservation.sequence(),
                self.elapsed_ms(),
                CaptureGeneration(frame.generation),
                &id,
            );
            record.run = 1;
            record.kind = RecordKind::Checkpoint;
            record.input_seen = inner.input_seen;
            record.present_seen = inner.present_seen;
            record.presentation = Some(crate::automation::capture::presentation_evidence(frame));
            record.checkpoint = Some(crate::automation::trace::CheckpointEvidence { id });
            match record.to_jsonl() {
                Ok(line) => {
                    reservation.commit_record(line);
                    inner.native_publication_pending = true;
                }
                Err(error) => {
                    reservation.cancel();
                    self.capture_failure(inner, error);
                    return true;
                }
            }
        }
        inner.checkpoint_step = inner.sched_state.step_index;
        false
    }

    fn write_committed_menu_ready(&self, inner: &CoordInner, frame: &PresentedFrame) {
        let reservation = self.runtime.commit.reserve();
        let mut record = crate::automation::capture::capture_trace_record(
            reservation.sequence(),
            self.elapsed_ms(),
            CaptureGeneration(frame.generation),
            "",
        );
        record.kind = RecordKind::Readiness;
        record.run = 1;
        record.input_seen = inner.input_seen;
        record.present_seen = inner.present_seen;
        record.label = None;
        record.presentation = Some(crate::automation::capture::presentation_evidence(frame));
        record.readiness = Some(ReadinessEvidence {
            subject: MAIN_MENU_READINESS_SUBJECT.into(),
            observation: MAIN_MENU_COMMITTED_OBSERVATION.into(),
        });
        reservation.commit_record(
            record
                .to_jsonl()
                .expect("serialize committed menu readiness"),
        );
    }

    pub(crate) fn trace_native_graphics(&self, count: u64, samples: &[PresentationSample]) {
        if !self.native_diagnostics() {
            return;
        }
        let inner = self.inner.lock();
        if inner.terminal_class.is_some() {
            return;
        }
        let reservation = self.runtime.commit.reserve();
        let mut record = crate::automation::capture::capture_trace_record(
            reservation.sequence(),
            self.elapsed_ms(),
            CaptureGeneration(self.runtime.mirror.capture_generation()),
            "",
        );
        record.schema = TraceRecord::NATIVE_SCHEMA;
        record.kind = RecordKind::NativeGraphics;
        record.run = 1;
        record.input_seen = inner.input_seen;
        record.present_seen = inner.present_seen;
        record.presentation = None;
        record.label =
            Some(serde_json::json!({"readback_count": count, "samples": samples}).to_string());
        reservation.commit_record(
            record
                .to_jsonl()
                .expect("serialize native graphics samples"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphics_samples_are_native_only_and_keep_the_next_readback_identity() {
        for native in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let script = crate::automation::validate_script(
                crate::automation::parse_script(
                    br#"{"version":1,"name":"graphics-probe","steps":[{"action":"finish"}],"budgets":{"max_input_ticks":10,"max_presentations":100,"max_wallclock_seconds":60}}"#,
                    "test.json",
                )
                .unwrap(),
                "test.json",
            )
            .unwrap();
            let observer = AutomationObserver::init_with_native_checkpoints(
                script,
                root.path().into(),
                native.then_some(16384),
                crate::engine::World::new().activity_control(),
                Default::default(),
                Default::default(),
            );
            let samples = [PresentationSample::Preprocess {
                force_redraw: 1,
                transition_amount: 255,
                fade_amount: 0,
            }];
            let before = observer.runtime.commit.next_sequence();
            observer.trace_native_graphics(10, &samples);
            if !native {
                assert_eq!(observer.runtime.commit.next_sequence(), before);
                continue;
            }
            observer.flush_native_trace().unwrap();
            let bytes = std::fs::read(root.path().join("trace.jsonl")).unwrap();
            let records: Vec<_> = String::from_utf8(bytes)
                .unwrap()
                .lines()
                .map(|line| TraceRecord::from_jsonl(line).unwrap())
                .filter(|record| record.kind == RecordKind::NativeGraphics)
                .collect();
            assert_eq!(records.len(), usize::from(native));
            if native {
                assert!(records[0].has_valid_schema());
                let value: serde_json::Value =
                    serde_json::from_str(records[0].label.as_ref().unwrap()).unwrap();
                assert_eq!(value["readback_count"], 10);
                assert_eq!(value["samples"][0]["fade_amount"], 0);
            }
        }
    }
}

#[cfg(test)]
#[path = "native_menu_tests.rs"]
mod menu_tests;
