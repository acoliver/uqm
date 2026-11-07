//! Ordered trace serialization, separate from scheduler and game coordination.

use super::*;

impl AutomationObserver {
    /// Write a trace record through the ordered commit.
    pub(super) fn write_trace(&self, inner: &mut CoordInner, kind: RecordKind) {
        self.write_action_phase(inner);
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();

        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
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

        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
    }

    pub(super) fn write_terminal_trace(&self, inner: &mut CoordInner) {
        self.write_action_phase(inner);
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();
        let terminal_reason = inner
            .terminal_class
            .and_then(|class| serde_json::to_value(class).ok())
            .and_then(|value| value.as_str().map(str::to_owned));
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
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
        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
    }

    /// Write a trace record with a semantic/evidence label.
    pub(super) fn write_trace_labeled(
        &self,
        inner: &mut CoordInner,
        kind: RecordKind,
        label: String,
    ) {
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();

        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
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

        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
    }

    /// Write a readiness trace record carrying what became ready and the
    /// observation that proved it.
    pub(super) fn write_readiness_trace(
        &self,
        inner: &mut CoordInner,
        subject: &str,
        observation: &str,
    ) {
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();

        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
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

        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
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
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
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
        match record.to_jsonl() {
            Ok(jsonl) => {
                reservation.commit_record(jsonl);
                false
            }
            Err(error) => {
                reservation.cancel();
                self.capture_failure(inner, format!("cannot serialize presentation: {error}"));
                true
            }
        }
    }
}
