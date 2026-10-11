//! Incremental view of the child's trace for per-presentation checkpoint checks.
//!
//! The controller must compare every publication against the first
//! `trace_record_count` trace records. Reparsing the whole journal for each
//! presentation is quadratic in run length: in a debug-built controller, a
//! long scenario spent most of its wall clock blocked on acknowledgements.
//!
//! The cursor parses each newline-terminated record once. Every call still
//! reads the whole journal and requires the already-parsed prefix to be
//! byte-identical; if it is not, the cursor reparses from the start, so the
//! result always equals a fresh parse of the current bytes.
//!
//! The same holds for the semantic snapshot and checkpoint verdicts derived from
//! those records: the cursor carries their scans forward over the records that
//! are new, and restarts them whenever the parsed records are discarded or a
//! shorter prefix is requested.

use super::native_checkpoint::{CheckpointScan, NativeCheckpointPlan};
use super::native_window::{NativeWindowProofError, NativeWindowSemanticSnapshot, SemanticScan};
use super::trace::TraceRecord;
#[cfg(test)]
use std::borrow::Cow;

#[derive(Debug, Default)]
pub(crate) struct TraceCursor {
    /// Exact bytes, including terminators, of the records in `records`.
    prefix: Vec<u8>,
    records: Vec<TraceRecord>,
    semantic: SemanticScan,
    checkpoints: CheckpointScan,
    /// Lines deserialized over the cursor's lifetime, for tests.
    parsed_lines: u64,
}

/// What a publication claims about the trace prefix it covers.
pub(crate) struct PrefixClaim<'a> {
    /// Number of records the publication covers.
    pub(crate) count: usize,
    pub(crate) plan: &'a NativeCheckpointPlan,
    /// Semantic snapshot the publication recorded for that prefix.
    pub(crate) semantic: &'a NativeWindowSemanticSnapshot,
    /// Checkpoint markers already acted on; their frames are not reported.
    pub(crate) handled_checkpoints: usize,
}

/// A prefix whose parse, semantic snapshot and checkpoint commitments passed.
pub(crate) struct VerifiedPrefix {
    /// Number of selected obligations committed in the prefix.
    pub(crate) committed: usize,
    /// Presented frame count of each committed marker after the handled ones.
    pub(crate) new_marker_frames: Vec<Option<u64>>,
}

#[derive(Debug)]
pub(crate) enum PrefixError {
    Trace(serde_json::Error),
    /// Fewer records exist than the publication covers.
    Short,
    Semantic(NativeWindowProofError),
    SemanticMismatch,
    Checkpoint(String),
}

impl TraceCursor {
    /// The first `count` nonempty lines of `bytes`, parsed as trace records.
    /// The reference the verdict methods are tested against.
    ///
    /// Equivalent to splitting `bytes` on `\n`, skipping empty lines, taking
    /// `count` and deserializing each, including when fewer lines exist.
    #[cfg(test)]
    pub(crate) fn records(
        &mut self,
        bytes: &[u8],
        count: usize,
    ) -> Result<Cow<'_, [TraceRecord]>, serde_json::Error> {
        let tail = self.parse_through(bytes, count)?;
        let available = self.records.len().min(count);
        Ok(match tail {
            Some(record) if available < count => {
                let mut owned = self.records[..available].to_vec();
                owned.push(record);
                Cow::Owned(owned)
            }
            _ => Cow::Borrowed(&self.records[..available]),
        })
    }

    /// Check the first `claim.count` records against what a publication claims,
    /// scanning only the records that arrived since the previous call.
    ///
    /// Equivalent to parsing the prefix, taking its semantic snapshot, comparing
    /// that with the claim, and then asking the plan which obligations the
    /// prefix commits, in that order of failure.
    pub(crate) fn verify_prefix(
        &mut self,
        bytes: &[u8],
        claim: &PrefixClaim<'_>,
    ) -> Result<VerifiedPrefix, PrefixError> {
        let tail = self
            .parse_through(bytes, claim.count)
            .map_err(PrefixError::Trace)?;
        let stable = self.records.len().min(claim.count);
        // A record still being written is judged once and never kept.
        let tailed = tail.is_some();
        if let Some(record) = tail {
            self.records.push(record);
        }
        let verdict = self.verify_loaded(claim, stable);
        if tailed {
            self.records.pop();
        }
        verdict
    }

    fn verify_loaded(
        &mut self,
        claim: &PrefixClaim<'_>,
        stable: usize,
    ) -> Result<VerifiedPrefix, PrefixError> {
        if self.records.len().min(claim.count) != claim.count {
            return Err(PrefixError::Short);
        }
        let prefix = &self.records[..claim.count];
        let semantic = if stable == claim.count {
            self.semantic.finish(prefix)
        } else {
            self.semantic
                .advance(&prefix[..stable])
                .map_err(PrefixError::Semantic)?;
            self.semantic.clone().finish(prefix)
        }
        .map_err(PrefixError::Semantic)?;
        if semantic != *claim.semantic {
            return Err(PrefixError::SemanticMismatch);
        }
        let tail_scan;
        let checkpoints = if stable == claim.count {
            self.checkpoints
                .advance(claim.plan, prefix)
                .map_err(PrefixError::Checkpoint)?;
            &self.checkpoints
        } else {
            self.checkpoints
                .advance(claim.plan, &prefix[..stable])
                .map_err(PrefixError::Checkpoint)?;
            let mut scratch = self.checkpoints.clone();
            scratch
                .advance(claim.plan, prefix)
                .map_err(PrefixError::Checkpoint)?;
            tail_scan = scratch;
            &tail_scan
        };
        let committed = checkpoints.committed();
        let new_marker_frames = checkpoints
            .marker_positions()
            .iter()
            .skip(claim.handled_checkpoints)
            .map(|position| {
                prefix[*position]
                    .presentation
                    .as_ref()
                    .map(|frame| frame.count)
            })
            .collect();
        Ok(VerifiedPrefix {
            committed,
            new_marker_frames,
        })
    }

    /// Parse and cache newline-terminated records until `count` are cached,
    /// returning the unterminated final record if one ends the journal first.
    fn parse_through(
        &mut self,
        bytes: &[u8],
        count: usize,
    ) -> Result<Option<TraceRecord>, serde_json::Error> {
        if !bytes.starts_with(&self.prefix) {
            self.prefix.clear();
            self.records.clear();
            self.semantic.restart();
            self.checkpoints.restart();
        }
        let mut offset = self.prefix.len();
        let mut tail = None;
        while self.records.len() < count && offset < bytes.len() {
            let rest = &bytes[offset..];
            let (line, consumed, terminated) = match rest.iter().position(|byte| *byte == b'\n') {
                Some(end) => (&rest[..end], end + 1, true),
                None => (rest, rest.len(), false),
            };
            if line.is_empty() {
                // Blank lines carry no record; remember them so the prefix
                // still matches byte for byte on the next call.
                self.prefix.extend_from_slice(&rest[..consumed]);
                offset += consumed;
                continue;
            }
            self.parsed_lines += 1;
            let record: TraceRecord = serde_json::from_slice(line)?;
            if !terminated {
                tail = Some(record);
                break;
            }
            self.prefix.extend_from_slice(&rest[..consumed]);
            self.records.push(record);
            offset += consumed;
        }
        Ok(tail)
    }

    #[cfg(test)]
    pub(crate) const fn parsed_lines(&self) -> u64 {
        self.parsed_lines
    }

    /// Records examined by the semantic and checkpoint scans, for tests.
    #[cfg(test)]
    pub(crate) const fn scan_visits(&self) -> (u64, u64) {
        (self.semantic.visits(), self.checkpoints.visits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::trace::RecordKind;

    fn record(sequence: u64) -> TraceRecord {
        TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence,
            input_seen: sequence,
            present_seen: 0,
            elapsed_ms: sequence,
            kind: RecordKind::InputTick,
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
        }
    }

    fn journal(range: std::ops::Range<u64>) -> Vec<u8> {
        range
            .flat_map(|sequence| {
                let mut line = serde_json::to_vec(&record(sequence)).unwrap();
                line.push(b'\n');
                line
            })
            .collect()
    }

    /// What the controller did before the cursor existed.
    fn fresh(bytes: &[u8], count: usize) -> Vec<TraceRecord> {
        bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .take(count)
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect()
    }

    #[test]
    fn a_growing_journal_parses_each_record_once() {
        let mut cursor = TraceCursor::default();
        for end in 1..=200 {
            let bytes = journal(0..end);
            let records = cursor.records(&bytes, end as usize).unwrap().into_owned();
            assert_eq!(records, fresh(&bytes, end as usize));
        }
        assert_eq!(cursor.parsed_lines(), 200);
    }

    #[test]
    fn a_rewritten_prefix_is_reparsed_rather_than_trusted() {
        let mut cursor = TraceCursor::default();
        let original = journal(0..10);
        cursor.records(&original, 10).unwrap();
        let mut rewritten = journal(0..10);
        let mut changed = record(3);
        changed.elapsed_ms = 999;
        let changed_line = serde_json::to_vec(&changed).unwrap();
        let start = rewritten
            .split(|byte| *byte == b'\n')
            .take(3)
            .map(|line| line.len() + 1)
            .sum::<usize>();
        let old_len = serde_json::to_vec(&record(3)).unwrap().len();
        rewritten.splice(start..start + old_len, changed_line);
        let records = cursor.records(&rewritten, 10).unwrap().into_owned();
        assert_eq!(records, fresh(&rewritten, 10));
        assert_eq!(records[3].elapsed_ms, 999);
    }

    #[test]
    fn counts_behind_ahead_and_unterminated_tails_match_a_fresh_parse() {
        let mut cursor = TraceCursor::default();
        let mut bytes = journal(0..5);
        bytes.extend_from_slice(b"\n\n");
        bytes.extend_from_slice(&journal(5..8));
        assert_eq!(
            cursor.records(&bytes, 3).unwrap().into_owned(),
            fresh(&bytes, 3)
        );
        assert_eq!(
            cursor.records(&bytes, 8).unwrap().into_owned(),
            fresh(&bytes, 8)
        );
        assert_eq!(
            cursor.records(&bytes, 2).unwrap().into_owned(),
            fresh(&bytes, 2)
        );
        assert_eq!(
            cursor.records(&bytes, 20).unwrap().into_owned(),
            fresh(&bytes, 20)
        );

        let mut unterminated = bytes.clone();
        unterminated.extend_from_slice(&serde_json::to_vec(&record(8)).unwrap());
        assert_eq!(
            cursor.records(&unterminated, 9).unwrap().into_owned(),
            fresh(&unterminated, 9)
        );
        // Once the record is terminated it joins the cache like any other.
        unterminated.push(b'\n');
        assert_eq!(
            cursor.records(&unterminated, 9).unwrap().into_owned(),
            fresh(&unterminated, 9)
        );
    }

    #[test]
    fn a_malformed_record_inside_the_requested_count_is_an_error() {
        let mut cursor = TraceCursor::default();
        let mut bytes = journal(0..2);
        bytes.extend_from_slice(b"{not json}\n");
        assert!(cursor.records(&bytes, 3).is_err());
        assert_eq!(cursor.records(&bytes, 2).unwrap().len(), 2);
    }

    /// Incremental verdicts against the validators they replaced.
    mod incremental {
        use super::*;
        use crate::automation::native_checkpoint::{CheckpointScan, NativeCheckpointPlan};
        use crate::automation::native_trace_oracle as oracle;
        use crate::automation::native_window::{NativeWindowSemanticSnapshot, SemanticScan};
        use crate::automation::script::Action;
        use crate::automation::trace::{CheckpointEvidence, PresentationEvidence};
        use proptest::prelude::*;

        fn plan_with_rounds(rounds: usize) -> NativeCheckpointPlan {
            let mut steps = Vec::new();
            for round in 0..rounds {
                steps.push(serde_json::json!({
                    "action":"wait_for_dispatch","encounter":2,"dialogue":2,"max_ticks":1
                }));
                steps.push(serde_json::json!({"action":"capture","label":format!("shot{round}")}));
                steps.push(serde_json::json!({"action":"assert_battle_frames","minimum":4}));
            }
            steps.push(serde_json::json!({"action":"finish"}));
            let source = serde_json::to_vec(&serde_json::json!({
                "version": 1, "name": "incremental-trace", "budgets": {
                    "max_input_ticks": 100_000, "max_presentations": 100_000,
                    "max_wallclock_seconds": 60
                }, "steps": steps
            }))
            .unwrap();
            let doc = crate::automation::script::parse_script(&source, "t.json").unwrap();
            let script = crate::automation::script::validate_script(doc, "t.json").unwrap();
            NativeCheckpointPlan::derive(&source, &script).unwrap()
        }

        /// A trace written the way the child writes it, with the snapshot the
        /// child would publish after each complete unit of records.
        struct Journal {
            plan: NativeCheckpointPlan,
            records: Vec<TraceRecord>,
            boundaries: Vec<(usize, NativeWindowSemanticSnapshot)>,
            accepted: u64,
            verified: u64,
            presented: u64,
            dispatches: u64,
            next_obligation: usize,
        }

        impl Journal {
            fn new(plan: NativeCheckpointPlan) -> Self {
                Self {
                    plan,
                    records: Vec::new(),
                    boundaries: Vec::new(),
                    accepted: 0,
                    verified: 0,
                    presented: 0,
                    dispatches: 0,
                    next_obligation: 0,
                }
            }

            fn push(&mut self, kind: RecordKind, label: Option<String>) -> &mut TraceRecord {
                let mut next = record(self.records.len() as u64);
                next.kind = kind;
                next.label = label;
                self.records.push(next);
                self.records.last_mut().unwrap()
            }

            fn frame(&self) -> PresentationEvidence {
                PresentationEvidence {
                    count: self.presented,
                    generation: 9,
                    width: 320,
                    height: 240,
                }
            }

            fn mark(&mut self) {
                self.boundaries.push((
                    self.records.len(),
                    NativeWindowSemanticSnapshot {
                        trace_record_count: self.records.len() as u64,
                        accepted_player_inputs: self.accepted,
                        verified_battle_frames: self.verified,
                    },
                ));
            }

            fn input_pair(&mut self, unit: u64) {
                let key = [
                    "Thrust", "Down", "Left", "Right", "Weapon", "Special", "Escape",
                ][unit as usize % 7];
                let value = unit % 2;
                self.push(
                    RecordKind::SemanticAssertion,
                    Some(format!(
                        "player_input_observed:key={key}:intended={value}:current={value}:pulsed={value}"
                    )),
                );
                self.push(
                    RecordKind::SemanticAssertion,
                    Some(format!("player_input:key={key}:value={value}")),
                );
                self.accepted += value;
            }

            fn commit_next(&mut self) {
                let Some(obligation) = self.plan.checkpoints.get(self.next_obligation) else {
                    return;
                };
                let id = obligation.id.clone();
                let action = obligation.action.clone();
                self.next_obligation += 1;
                self.presented += 1;
                let frame = self.frame();
                match action {
                    Action::Capture(step) => {
                        self.push(RecordKind::Presentation, None).presentation =
                            Some(frame.clone());
                        let evidence = self.push(
                            RecordKind::Capture,
                            Some(format!("{}_gen{}", step.label, frame.generation)),
                        );
                        evidence.presentation = Some(frame.clone());
                    }
                    Action::WaitForDispatch(_) => {
                        self.dispatches += 1;
                        let label = format!(
                            "dispatch_observed:generation={}:encounter=2:dialogue=2",
                            self.dispatches
                        );
                        self.push(RecordKind::SemanticAssertion, Some(label));
                        self.push(RecordKind::Presentation, None).presentation =
                            Some(frame.clone());
                    }
                    _ => {
                        self.verified = self.verified.max(4 + self.presented);
                        let label = format!("battle_frames_verified:count={}", 4 + self.presented);
                        self.push(RecordKind::SemanticAssertion, Some(label));
                        self.push(RecordKind::Presentation, None).presentation =
                            Some(frame.clone());
                    }
                }
                let marker = self.push(RecordKind::Checkpoint, None);
                marker.presentation = Some(frame);
                marker.checkpoint = Some(CheckpointEvidence { id });
            }

            fn grow(&mut self, units: u64) {
                for unit in 0..units {
                    self.push(RecordKind::InputTick, None);
                    self.input_pair(unit);
                    if unit % 4 == 3 {
                        self.commit_next();
                    }
                    self.mark();
                }
            }
        }

        fn bytes_of(records: &[TraceRecord]) -> Vec<u8> {
            records
                .iter()
                .flat_map(|record| {
                    let mut line = serde_json::to_vec(record).unwrap();
                    line.push(b'\n');
                    line
                })
                .collect()
        }

        fn fresh_parse(bytes: &[u8], count: usize) -> Result<Vec<TraceRecord>, serde_json::Error> {
            bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .take(count)
                .map(serde_json::from_slice)
                .collect()
        }

        type Verdict = Result<(usize, Vec<Option<u64>>), String>;

        /// What the controller computed before it carried state between calls.
        fn oracle_verdict(
            bytes: &[u8],
            plan: &NativeCheckpointPlan,
            claim: &PrefixClaim<'_>,
        ) -> Verdict {
            let records = fresh_parse(bytes, claim.count).map_err(|e| format!("trace {e}"))?;
            if records.len() != claim.count {
                return Err("short".into());
            }
            let semantic =
                oracle::semantic_snapshot(&records).map_err(|e| format!("semantic {e:?}"))?;
            if semantic != *claim.semantic {
                return Err("mismatch".into());
            }
            let committed = oracle::committed(plan, &records)
                .map_err(|e| format!("checkpoint {e}"))?
                .len();
            let frames = records
                .iter()
                .filter(|record| record.kind == RecordKind::Checkpoint)
                .skip(claim.handled_checkpoints)
                .map(|record| record.presentation.as_ref().map(|frame| frame.count))
                .collect();
            Ok((committed, frames))
        }

        fn cursor_verdict(
            cursor: &mut TraceCursor,
            bytes: &[u8],
            claim: &PrefixClaim<'_>,
        ) -> Verdict {
            cursor
                .verify_prefix(bytes, claim)
                .map(|verified| (verified.committed, verified.new_marker_frames))
                .map_err(|error| match error {
                    PrefixError::Trace(error) => format!("trace {error}"),
                    PrefixError::Short => "short".into(),
                    PrefixError::Semantic(error) => format!("semantic {error:?}"),
                    PrefixError::SemanticMismatch => "mismatch".into(),
                    PrefixError::Checkpoint(message) => format!("checkpoint {message}"),
                })
        }

        /// The snapshot the publication would carry, or an arbitrary one when the
        /// prefix has no valid snapshot, so the error path is still reached.
        fn claimed_snapshot(
            bytes: &[u8],
            count: usize,
            wrong: bool,
        ) -> NativeWindowSemanticSnapshot {
            let honest = fresh_parse(bytes, count)
                .ok()
                .and_then(|records| oracle::semantic_snapshot(&records).ok())
                .unwrap_or(NativeWindowSemanticSnapshot {
                    trace_record_count: count as u64,
                    accepted_player_inputs: 0,
                    verified_battle_frames: 0,
                });
            NativeWindowSemanticSnapshot {
                accepted_player_inputs: honest.accepted_player_inputs + u64::from(wrong),
                ..honest
            }
        }

        fn assert_same_verdict(
            cursor: &mut TraceCursor,
            bytes: &[u8],
            plan: &NativeCheckpointPlan,
            count: usize,
            handled: usize,
            wrong: bool,
        ) {
            let semantic = claimed_snapshot(bytes, count, wrong);
            let claim = PrefixClaim {
                count,
                plan,
                semantic: &semantic,
                handled_checkpoints: handled,
            };
            assert_eq!(
                cursor_verdict(cursor, bytes, &claim),
                oracle_verdict(bytes, plan, &claim),
                "count {count} handled {handled} wrong {wrong}"
            );
        }

        #[test]
        fn every_prefix_of_a_valid_trace_matches_the_old_validators() {
            let mut journal = Journal::new(plan_with_rounds(5));
            journal.grow(40);
            assert_eq!(journal.next_obligation, 10);
            let bytes = bytes_of(&journal.records);
            let plan = journal.plan.clone();
            let mut shared = TraceCursor::default();
            for count in 1..=journal.records.len() + 2 {
                for handled in [0, 3, 99] {
                    assert_same_verdict(&mut shared, &bytes, &plan, count, handled, false);
                    assert_same_verdict(&mut shared, &bytes, &plan, count, handled, true);
                    assert_same_verdict(
                        &mut TraceCursor::default(),
                        &bytes,
                        &plan,
                        count,
                        handled,
                        false,
                    );
                }
            }
        }

        #[test]
        fn counts_may_regress_repeat_and_run_ahead() {
            let mut journal = Journal::new(plan_with_rounds(5));
            journal.grow(40);
            let bytes = bytes_of(&journal.records);
            let plan = journal.plan.clone();
            let mut cursor = TraceCursor::default();
            for count in [30, 10, 30, 60, 45, 60, 5, 60, 61, 61, 200, 60, 1, 150] {
                assert_same_verdict(&mut cursor, &bytes, &plan, count, 2, false);
            }
        }

        #[test]
        fn an_unterminated_final_record_is_judged_without_being_kept() {
            let mut journal = Journal::new(plan_with_rounds(5));
            journal.grow(40);
            let plan = journal.plan.clone();
            let full = bytes_of(&journal.records);
            let mut cursor = TraceCursor::default();
            for end in 1..=journal.records.len() {
                // Every record, in turn, is the one still being written.
                let partial = bytes_of(&journal.records[..end]);
                let partial = &partial[..partial.len() - 1];
                for count in [end.saturating_sub(1).max(1), end] {
                    assert_same_verdict(&mut cursor, partial, &plan, count, 0, false);
                }
                assert_same_verdict(&mut cursor, &full, &plan, end, 0, false);
            }
        }

        #[test]
        fn a_rewritten_prefix_of_the_same_length_is_rescanned() {
            let mut journal = Journal::new(plan_with_rounds(5));
            journal.grow(40);
            let plan = journal.plan.clone();
            let count = journal.records.len();
            let original = bytes_of(&journal.records);
            let mut cursor = TraceCursor::default();
            assert_same_verdict(&mut cursor, &original, &plan, count, 0, false);
            let parsed = cursor.parsed_lines();
            let text = String::from_utf8(original.clone()).unwrap();
            let rewrites = [
                // Breaks an observed/accepted pair.
                (text.find("value=1").unwrap(), "value=1", "value=0"),
                // Misidentifies a checkpoint marker.
                (text.find("native:").unwrap() + 80, "0", "1"),
                // Harmless: only elapsed time changes.
                (
                    text.find("\"elapsed_ms\":3").unwrap(),
                    "\"elapsed_ms\":3",
                    "\"elapsed_ms\":4",
                ),
            ];
            for (at, from, to) in rewrites {
                let mut rewritten = text.clone().into_bytes();
                if rewritten[at..].starts_with(from.as_bytes()) {
                    rewritten[at..at + to.len()].copy_from_slice(to.as_bytes());
                } else {
                    let flip = at
                        + rewritten[at..]
                            .iter()
                            .position(|b| b.is_ascii_digit())
                            .unwrap();
                    rewritten[flip] = if rewritten[flip] == b'1' { b'2' } else { b'1' };
                }
                assert_ne!(rewritten, original);
                assert_eq!(rewritten.len(), original.len());
                assert_same_verdict(&mut cursor, &rewritten, &plan, count, 0, false);
                assert_same_verdict(&mut cursor, &original, &plan, count, 0, false);
            }
            assert!(cursor.parsed_lines() > parsed, "rewrites were not reparsed");
        }

        #[test]
        fn a_dispatch_generation_may_not_be_consumed_twice() {
            let mut journal = Journal::new(plan_with_rounds(5));
            journal.grow(40);
            let plan = journal.plan.clone();
            let mut records = journal.records.clone();
            let second = records
                .iter()
                .enumerate()
                .filter(|(_, record)| {
                    record
                        .label
                        .as_deref()
                        .is_some_and(|label| label.starts_with("dispatch_observed:"))
                })
                .nth(1)
                .unwrap()
                .0;
            records[second].label =
                Some("dispatch_observed:generation=1:encounter=2:dialogue=2".into());
            let bytes = bytes_of(&records);
            let mut cursor = TraceCursor::default();
            let mut rejected = false;
            for count in 1..=records.len() {
                assert_same_verdict(&mut cursor, &bytes, &plan, count, 0, false);
                let semantic = claimed_snapshot(&bytes, count, false);
                let claim = PrefixClaim {
                    count,
                    plan: &plan,
                    semantic: &semantic,
                    handled_checkpoints: 0,
                };
                rejected |= cursor.verify_prefix(&bytes, &claim).is_err();
            }
            assert!(rejected, "the stale generation was accepted");
        }

        #[test]
        fn a_final_record_that_changes_while_unterminated_is_judged_afresh() {
            let mut journal = Journal::new(plan_with_rounds(5));
            journal.grow(40);
            let plan = journal.plan.clone();
            let markers: Vec<usize> = (0..journal.records.len())
                .filter(|index| journal.records[*index].kind == RecordKind::Checkpoint)
                .collect();
            for end in markers.into_iter().map(|index| index + 1) {
                let mut damaged = journal.records[..end].to_vec();
                damaged[end - 1].checkpoint.as_mut().unwrap().id.push('x');
                let unterminated = |records: &[TraceRecord]| {
                    let mut bytes = bytes_of(records);
                    bytes.pop();
                    bytes
                };
                let (good, bad) = (
                    unterminated(&journal.records[..end]),
                    unterminated(&damaged),
                );
                let mut cursor = TraceCursor::default();
                for bytes in [&good, &bad, &good, &bad, &bad, &good] {
                    assert_same_verdict(&mut cursor, bytes, &plan, end, 0, false);
                }
            }
        }

        #[derive(Clone, Debug)]
        enum Damage {
            Sequence(usize),
            Run(usize),
            Remove(usize),
            SwapNext(usize),
            Duplicate(usize),
            DropCheckpointId(usize),
            StrayInput(usize),
            MalformedObserved(usize),
            WrongCheckpointId(usize),
            PresentationCount(usize),
            UnrelatedLabel(usize),
            StaleDispatch(usize),
        }

        fn damage() -> impl Strategy<Value = Damage> {
            (0..12_u8, 0..400_usize).prop_map(|(kind, at)| match kind {
                0 => Damage::Sequence(at),
                1 => Damage::Run(at),
                2 => Damage::Remove(at),
                3 => Damage::SwapNext(at),
                4 => Damage::Duplicate(at),
                5 => Damage::DropCheckpointId(at),
                6 => Damage::StrayInput(at),
                7 => Damage::MalformedObserved(at),
                8 => Damage::WrongCheckpointId(at),
                9 => Damage::PresentationCount(at),
                10 => Damage::UnrelatedLabel(at),
                _ => Damage::StaleDispatch(at),
            })
        }

        fn renumber(records: &mut [TraceRecord]) {
            for (index, record) in records.iter_mut().enumerate() {
                record.sequence = index as u64;
            }
        }

        fn apply(records: &mut Vec<TraceRecord>, damage: &Damage) {
            let len = records.len();
            let (Damage::Sequence(at)
            | Damage::Run(at)
            | Damage::Remove(at)
            | Damage::SwapNext(at)
            | Damage::Duplicate(at)
            | Damage::DropCheckpointId(at)
            | Damage::StrayInput(at)
            | Damage::MalformedObserved(at)
            | Damage::WrongCheckpointId(at)
            | Damage::PresentationCount(at)
            | Damage::UnrelatedLabel(at)
            | Damage::StaleDispatch(at)) = damage;
            let at = at % len;
            match damage {
                Damage::Sequence(_) => records[at].sequence += 3,
                Damage::Run(_) => records[at].run += 1,
                Damage::Remove(_) => {
                    records.remove(at);
                    renumber(records);
                }
                Damage::SwapNext(_) if at + 1 < len => {
                    records.swap(at, at + 1);
                    renumber(records);
                }
                Damage::SwapNext(_) => {}
                Damage::Duplicate(_) => {
                    let copy = records[at].clone();
                    records.insert(at, copy);
                    renumber(records);
                }
                Damage::DropCheckpointId(_) => records[at].checkpoint = None,
                Damage::StrayInput(_) => {
                    records[at].label = Some("player_input:key=Thrust:value=1".into());
                }
                Damage::MalformedObserved(_) => {
                    records[at].label = Some("player_input_observed:key=Nope".into());
                }
                Damage::WrongCheckpointId(_) => {
                    if let Some(marker) = records[at].checkpoint.as_mut() {
                        marker.id.push('x');
                    }
                }
                Damage::PresentationCount(_) => {
                    if let Some(frame) = records[at].presentation.as_mut() {
                        frame.count += 2;
                    }
                }
                Damage::UnrelatedLabel(_) => records[at].label = Some("unrelated".into()),
                Damage::StaleDispatch(_) => {
                    let dispatches: Vec<usize> = (0..len)
                        .filter(|index| {
                            records[*index]
                                .label
                                .as_deref()
                                .is_some_and(|label| label.starts_with("dispatch_observed:"))
                        })
                        .collect();
                    let chosen = dispatches[at % dispatches.len()];
                    records[chosen].label =
                        Some("dispatch_observed:generation=1:encounter=2:dialogue=2".into());
                }
            }
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(96))]

            #[test]
            fn damaged_traces_walked_in_any_order_match_the_old_validators(
                damages in proptest::collection::vec(damage(), 0..3),
                walk in proptest::collection::vec(1..160_usize, 1..30),
                ending in 0..4_u8,
                blanks in proptest::collection::vec(0..160_usize, 0..3),
            ) {
                let mut journal = Journal::new(plan_with_rounds(5));
                journal.grow(40);
                let plan = journal.plan.clone();
                let mut records = journal.records;
                for item in &damages {
                    apply(&mut records, item);
                }
                let mut bytes = Vec::new();
                for (index, record) in records.iter().enumerate() {
                    bytes.extend_from_slice(&serde_json::to_vec(record).unwrap());
                    bytes.push(b'\n');
                    if blanks.contains(&index) {
                        bytes.push(b'\n');
                    }
                }
                match ending {
                    1 => { bytes.pop(); }
                    2 => bytes.truncate(bytes.len() - 6),
                    _ => {}
                }
                let mut cursor = TraceCursor::default();
                for (step, count) in walk.into_iter().enumerate() {
                    assert_same_verdict(&mut cursor, &bytes, &plan, count, step % 4, step % 7 == 6);
                }
            }
        }

        #[test]
        fn scans_visit_each_record_once_however_often_the_prefix_is_checked() {
            let mut journal = Journal::new(plan_with_rounds(400));
            journal.grow(3_200);
            let total = journal.records.len();
            assert!(total > 10_000, "{total} records");
            assert_eq!(journal.next_obligation, 800);
            let plan = journal.plan.clone();
            let mut semantic = SemanticScan::default();
            let mut checkpoints = CheckpointScan::default();
            for (count, snapshot) in &journal.boundaries {
                let prefix = &journal.records[..*count];
                for _ in 0..2 {
                    assert_eq!(semantic.finish(prefix).unwrap(), *snapshot);
                    checkpoints.advance(&plan, prefix).unwrap();
                }
            }
            assert_eq!(checkpoints.committed(), 800);
            let total = total as u64;
            // Replaying history at every boundary would examine about 20 million.
            assert!(
                semantic.visits() >= total / 2 && semantic.visits() <= total,
                "semantic visits {}",
                semantic.visits()
            );
            assert!(
                checkpoints.visits() >= total && checkpoints.visits() <= 2 * total,
                "checkpoint visits {}",
                checkpoints.visits()
            );
        }

        #[test]
        fn the_cursor_checks_a_growing_journal_in_work_proportional_to_its_growth() {
            let mut journal = Journal::new(plan_with_rounds(100));
            journal.grow(800);
            let total = journal.records.len();
            let plan = journal.plan.clone();
            let bytes = bytes_of(&journal.records);
            let mut ends = Vec::new();
            let mut offset = 0;
            for line in bytes.split_inclusive(|byte| *byte == b'\n') {
                offset += line.len();
                ends.push(offset);
            }
            let mut cursor = TraceCursor::default();
            let mut handled = 0;
            for (count, snapshot) in &journal.boundaries {
                let claim = PrefixClaim {
                    count: *count,
                    plan: &plan,
                    semantic: snapshot,
                    handled_checkpoints: handled,
                };
                for _ in 0..2 {
                    let verified = cursor
                        .verify_prefix(&bytes[..ends[count - 1]], &claim)
                        .unwrap();
                    assert!(verified.new_marker_frames.len() <= 1);
                    handled = handled.max(verified.committed);
                }
            }
            assert_eq!(handled, 200);
            assert_eq!(cursor.parsed_lines(), total as u64);
            let (semantic, checkpoints) = cursor.scan_visits();
            assert!(semantic <= total as u64, "semantic visits {semantic}");
            assert!(
                checkpoints <= 2 * total as u64,
                "checkpoint visits {checkpoints}"
            );
        }

        #[test]
        fn a_record_still_being_written_costs_work_for_that_record_only() {
            let mut journal = Journal::new(plan_with_rounds(100));
            journal.grow(800);
            let total = journal.records.len() as u64;
            let plan = journal.plan.clone();
            let bytes = bytes_of(&journal.records);
            let mut ends = Vec::new();
            let mut offset = 0;
            for line in bytes.split_inclusive(|byte| *byte == b'\n') {
                offset += line.len();
                ends.push(offset);
            }
            let mut cursor = TraceCursor::default();
            let mut handled = 0;
            for (count, snapshot) in &journal.boundaries {
                let claim = PrefixClaim {
                    count: *count,
                    plan: &plan,
                    semantic: snapshot,
                    handled_checkpoints: handled,
                };
                let end = ends[count - 1];
                // The final record is seen unterminated, repeatedly, before it is complete.
                for visible in [end - 1, end - 1, end, end] {
                    let verified = cursor.verify_prefix(&bytes[..visible], &claim).unwrap();
                    handled = handled.max(verified.committed);
                }
            }
            assert_eq!(handled, 200);
            let (semantic, checkpoints) = cursor.scan_visits();
            assert!(semantic <= 3 * total, "semantic visits {semantic}");
            assert!(checkpoints <= 3 * total, "checkpoint visits {checkpoints}");
        }

        /// Reads a retained run: `UQM_ARCHIVED_TRACE` is its trace journal and
        /// `UQM_ARCHIVED_SCRIPT` the script that produced it. The old validators
        /// judge predeclared prefixes; the incremental cursor judges every
        /// prefix against a tally of the records themselves.
        #[test]
        #[ignore = "needs a retained run: UQM_ARCHIVED_TRACE and UQM_ARCHIVED_SCRIPT"]
        fn every_prefix_of_a_retained_run_matches_the_records_it_covers() {
            let trace = std::fs::read(std::env::var("UQM_ARCHIVED_TRACE").unwrap()).unwrap();
            let source = std::fs::read(std::env::var("UQM_ARCHIVED_SCRIPT").unwrap()).unwrap();
            let doc = crate::automation::script::parse_script(&source, "retained.json").unwrap();
            let script = crate::automation::script::validate_script(doc, "retained.json").unwrap();
            let plan = NativeCheckpointPlan::derive(&source, &script).unwrap();
            let records = fresh_parse(&trace, usize::MAX).unwrap();
            let mut ends = Vec::new();
            let mut offset = 0;
            for line in trace.split_inclusive(|byte| *byte == b'\n') {
                offset += line.len();
                ends.push(offset);
            }
            let total = records.len();
            let is_observed = |record: &TraceRecord| {
                record
                    .label
                    .as_deref()
                    .is_some_and(|label| label.starts_with("player_input_observed:"))
            };
            let samples: Vec<usize> = (1..=total)
                .filter(|count| {
                    count % 1_000 == 1
                        || *count == total
                        || records[count - 1].kind == RecordKind::Checkpoint
                            && records[count - 1]
                                .checkpoint
                                .as_ref()
                                .is_some_and(|marker| {
                                    marker.id.ends_with(":0") || marker.id.ends_with(":7")
                                })
                })
                .collect();
            let mut cursor = TraceCursor::default();
            let (mut accepted, mut verified, mut markers) = (0_u64, 0_u64, 0_usize);
            for count in 1..=total {
                let record = &records[count - 1];
                let label = record.label.as_deref().unwrap_or_default();
                if record.kind == RecordKind::Checkpoint {
                    markers += 1;
                }
                if let Some(value) = label.strip_prefix("battle_frames_verified:count=") {
                    verified = verified.max(value.parse().unwrap());
                }
                if label.starts_with("battle_frames_reached:") {
                    let value = label.split(':').nth(1).unwrap();
                    verified = verified.max(value.strip_prefix("count=").unwrap().parse().unwrap());
                }
                if is_observed(record) {
                    // Its accepted input is the next record; the prefix ends too early.
                    accepted += label
                        .rsplit("pulsed=")
                        .next()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap();
                }
                let semantic = NativeWindowSemanticSnapshot {
                    trace_record_count: count as u64,
                    accepted_player_inputs: accepted,
                    verified_battle_frames: verified,
                };
                let bytes = &trace[..ends[count - 1]];
                let claim = PrefixClaim {
                    count,
                    plan: &plan,
                    semantic: &semantic,
                    handled_checkpoints: 0,
                };
                let verdict = cursor.verify_prefix(bytes, &claim);
                if is_observed(record) {
                    assert!(
                        matches!(verdict, Err(PrefixError::Semantic(_))),
                        "prefix {count} ends before an accepted input"
                    );
                } else {
                    let verdict = verdict.unwrap_or_else(|error| panic!("{count}: {error:?}"));
                    assert_eq!(verdict.committed, markers, "prefix {count}");
                    assert_eq!(verdict.new_marker_frames.len(), markers, "prefix {count}");
                }
                if samples.contains(&count) {
                    assert_same_verdict(&mut TraceCursor::default(), bytes, &plan, count, 0, false);
                    assert_same_verdict(&mut cursor, bytes, &plan, count, markers / 2, false);
                }
            }
            let (semantic, checkpoints) = cursor.scan_visits();
            eprintln!(
                "records {total} markers {markers} semantic visits {semantic} \
                 checkpoint visits {checkpoints} samples {}",
                samples.len()
            );
            assert!(semantic <= 2 * total as u64 && checkpoints <= 3 * total as u64);
        }

        #[test]
        fn the_scans_agree_with_the_oracle_on_every_boundary_snapshot() {
            let mut journal = Journal::new(plan_with_rounds(20));
            journal.grow(160);
            for (count, snapshot) in &journal.boundaries {
                let prefix = &journal.records[..*count];
                assert_eq!(oracle::semantic_snapshot(prefix).unwrap(), *snapshot);
                assert_eq!(
                    crate::automation::native_window::native_window_trace_semantic_snapshot(prefix)
                        .unwrap(),
                    *snapshot
                );
            }
        }
    }
}
