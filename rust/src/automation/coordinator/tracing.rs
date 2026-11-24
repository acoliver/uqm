//! Ordered trace serialization, separate from scheduler and game coordination.

use super::*;

impl AutomationObserver {
    /// Write a trace record through the ordered commit.
    pub(super) fn write_trace(&self, inner: &mut CoordInner, kind: RecordKind) {
        self.write_action_phase(inner);

        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: 0,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.elapsed_ms(),
            kind,
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
            native_battle: None,
        };

        self.append_trace(inner, record);
    }

    pub(super) fn write_terminal_trace(&self, inner: &mut CoordInner) {
        self.write_action_phase(inner);
        let terminal_reason = inner
            .terminal_class
            .and_then(|class| serde_json::to_value(class).ok())
            .and_then(|value| value.as_str().map(str::to_owned));
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: 0,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.elapsed_ms(),
            kind: RecordKind::RunEnd,
            label: None,
            from: None,
            to: None,
            terminal_reason,
            seed_application: None,
            presentation: None,
            activity: None,
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
            native_battle: None,
        };
        self.append_trace(inner, record);
    }

    /// Write a trace record with a semantic/evidence label.
    pub(super) fn write_trace_labeled(
        &self,
        inner: &mut CoordInner,
        kind: RecordKind,
        label: String,
    ) {
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: 0,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.elapsed_ms(),
            kind,
            label: Some(label),
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
            native_battle: None,
        };

        self.append_trace(inner, record);
    }

    /// Write a readiness trace record carrying what became ready and the
    /// observation that proved it.
    pub(super) fn write_readiness_trace(
        &self,
        inner: &mut CoordInner,
        subject: &str,
        observation: &str,
    ) {
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: 0,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.elapsed_ms(),
            kind: RecordKind::Readiness,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: None,
            activity: None,
            readiness: Some(ReadinessEvidence {
                subject: subject.to_string(),
                observation: observation.to_string(),
            }),
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
            native_battle: None,
        };

        self.append_trace(inner, record);
    }

    pub(super) fn write_presentation_trace(
        &self,
        inner: &mut CoordInner,
        frame: &PresentedFrame,
    ) -> bool {
        if let Err(error) = self.parity_present(frame) {
            self.capture_failure(inner, format!("parity presentation: {error}"));
            return true;
        }
        self.write_action_phase(inner);
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: 0,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.elapsed_ms(),
            kind: RecordKind::Presentation,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: Some(crate::automation::capture::presentation_evidence(frame)),
            activity: None,
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
            native_battle: None,
        };
        self.append_trace(inner, record)
    }

    /// Every producer uses the same append failure and first-terminal policy.
    pub(super) fn append_trace(&self, inner: &mut CoordInner, record: TraceRecord) -> bool {
        match self.runtime.commit.append(record) {
            Ok(()) => false,
            Err(error) => {
                self.capture_failure(inner, error);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_trace_failure_cannot_finalize_success_even_after_success_won_first() {
        for first_terminal in [None, Some(TerminalClass::Success)] {
            let output = tempfile::tempdir().unwrap();
            let script = crate::automation::validate_script(
                crate::automation::parse_script(
                    br#"{"version":1,"name":"trace-failure","steps":[{"action":"finish"}],"budgets":{"max_input_ticks":10,"max_presentations":10,"max_wallclock_seconds":60}}"#,
                    "test.json",
                ).unwrap(), "test.json",
            ).unwrap();
            let observer = AutomationObserver::new(
                script,
                output.path().into(),
                uqm_engine::World::new().activity_control(),
                Default::default(),
                Default::default(),
            );
            if let Some(class) = first_terminal {
                observer.set_terminal(&mut observer.inner.lock(), class);
            }
            let mut sink = [0_u8; 0].as_mut_slice();
            let error = observer.runtime.commit.publish_all(&mut sink).unwrap_err();
            // Seed observations are a real coordinator producer, not a model API.
            observer.seed_value(SeedDomain::NEW_GAME_ID, 0);
            let expected = first_terminal.unwrap_or(TerminalClass::TraceFailure);
            assert_eq!(observer.inner.lock().terminal_class, Some(expected));
            assert_eq!(observer.runtime.mirror.terminal.load(), Some(expected));
            let final_error = observer.finalize(0).unwrap_err();
            assert!(final_error.contains(&error.to_string()), "{final_error}");
            assert!(!observer.runtime.mirror.is_active());
            assert!(!observer.runtime.can_write());
            assert!(!output.path().join("teardown.json").exists());
            assert!(!output.path().join("trace.jsonl").exists());
            assert!(observer.finalize(0).unwrap_err().contains("more than once"));
        }
    }
}
