//! Typed JSONL trace records and the ordered commit synchronization primitive.
//!
//! Implements REQ-TRACE-001 (serialization primitive) and REQ-IO-001 (ordered
//! reservation/commit). The ordered commit object waits synchronously for
//! `sequence == next_to_publish`, writes exactly that record, advances the
//! cursor, and notifies waiters. It never holds the runtime mutex while
//! waiting or writing.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
//! @requirement REQ-IO-001, REQ-TRACE-001

use crate::automation::error::AutomationError;
use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::Arc;

// ===========================================================================
//  Typed JSONL records (REQ-TRACE-001)
// ===========================================================================

/// The kind of trace record.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-TRACE-001
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    RunStart,
    RunEnd,
    InputTick,
    Presentation,
    Capture,
    MenuTransition,
    SemanticAssertion,
    SeedApplication,
    Terminal,
}

/// Stable automation seed domains. Each application is recorded at the exact
/// initialization boundary that consumes the seed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SeedDomain {
    SuperMeleeMenu,
    SuperMeleeBattle,
    /// The generator seeding that happens when a new game starts.
    ///
    /// Without this the campaign seeds itself from the wall clock, so nothing
    /// outside Super Melee replays: two runs of the same script at the same
    /// commit produce different planet surfaces.
    NewGame,
}

impl SeedDomain {
    pub const SUPER_MELEE_MENU_ID: u32 = 1;
    pub const SUPER_MELEE_BATTLE_ID: u32 = 2;
    pub const NEW_GAME_ID: u32 = 3;

    #[must_use]
    pub fn from_ffi(value: u32) -> Option<Self> {
        match value {
            Self::SUPER_MELEE_MENU_ID => Some(Self::SuperMeleeMenu),
            Self::SUPER_MELEE_BATTLE_ID => Some(Self::SuperMeleeBattle),
            Self::NEW_GAME_ID => Some(Self::NewGame),
            _ => None,
        }
    }
}

/// Typed seed application evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SeedApplication {
    pub domain: SeedDomain,
    pub seed: u32,
}

/// Metadata for the immutable frame read back immediately before presentation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresentationEvidence {
    pub count: u64,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
}

/// Typed activity assertion evidence from the live activity word.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActivityEvidence {
    pub word: u16,
    pub mask: u16,
    pub equals: u16,
    pub passed: bool,
}

/// A typed JSONL trace record. Each record is independently serializable as
/// one JSON object on one line.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-TRACE-001
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceRecord {
    pub schema: u16,
    pub run: u64,
    pub sequence: u64,
    pub input_seen: u64,
    pub present_seen: u64,
    pub elapsed_ms: u64,
    pub kind: RecordKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed_application: Option<SeedApplication>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<PresentationEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<ActivityEvidence>,
}

impl TraceRecord {
    /// Current schema version.
    pub const SCHEMA: u16 = 1;

    /// Serialize this record as one JSON line followed by a newline.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
    /// @requirement REQ-TRACE-001
    pub fn to_jsonl(&self) -> Result<String, AutomationError> {
        serde_json::to_string(self)
            .map(|s| format!("{s}\n"))
            .map_err(|e| AutomationError::InvalidJson {
                path: "<trace>".into(),
                reason: e.to_string(),
            })
    }

    /// Parse one JSONL line into a trace record.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
    /// @requirement REQ-TRACE-001
    pub fn from_jsonl(line: &str) -> Result<Self, AutomationError> {
        serde_json::from_str(line.trim()).map_err(|e| AutomationError::InvalidJson {
            path: "<trace>".into(),
            reason: e.to_string(),
        })
    }
}

// ===========================================================================
//  OrderedCommit synchronization primitive (REQ-IO-001)
// ===========================================================================

/// Internal state of the ordered commit object.
struct OrderedCommitState {
    /// The next sequence number to publish.
    next_to_publish: u64,
    /// The next sequence number to allocate via reserve().
    next_to_reserve: u64,
    /// Submitted records keyed by sequence number.
    pending: BTreeMap<u64, SubmitEntry>,
    /// Whether the sink has experienced a fatal error.
    sink_failed: bool,
}

/// An entry submitted to the ordered commit object.
enum SubmitEntry {
    /// A record waiting to be published.
    Record(String),
    /// A cancelled slot (advance cursor without writing).
    Cancelled,
}

/// RAII reservation guard. On drop, publishes either success (if committed)
/// or cancellation. This ensures a missing sequence can never block later
/// commits.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-001
pub struct Reservation {
    sequence: u64,
    committed: Option<String>,
    commit: Option<Arc<OrderedCommitInner>>,
}

impl Reservation {
    /// Commit a record payload for this reservation.
    pub fn commit_record(mut self, jsonl: String) {
        self.committed = Some(jsonl);
        // Drop will publish.
    }

    /// Explicitly cancel this reservation.
    pub fn cancel(mut self) {
        self.committed = None;
        // Drop will publish cancellation.
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if let Some(inner) = self.commit.take() {
            let entry = match self.committed.take() {
                Some(jsonl) => SubmitEntry::Record(jsonl),
                None => SubmitEntry::Cancelled,
            };
            let (sequence, entry_data) = (self.sequence, entry);
            inner.submit(sequence, entry_data);
        }
    }
}

/// Inner shared state behind the ordered commit object.
struct OrderedCommitInner {
    state: Mutex<OrderedCommitState>,
    cv: Condvar,
}

impl OrderedCommitInner {
    fn submit(&self, sequence: u64, entry: SubmitEntry) {
        let mut state = self.state.lock();
        state.pending.insert(sequence, entry);
        self.cv.notify_all();
    }

    fn run_publisher(&self, sink: &mut dyn Write) -> Result<(), std::io::Error> {
        loop {
            let to_publish = {
                let state = self.state.lock();
                if state.sink_failed {
                    return Ok(());
                }
                state.next_to_publish
            };

            let entry = {
                let mut state = self.state.lock();
                loop {
                    if state.sink_failed {
                        return Ok(());
                    }
                    let next = state.next_to_publish;
                    if state.pending.contains_key(&next) {
                        break state.pending.remove(&next).unwrap();
                    }
                    if state.pending.is_empty() {
                        return Ok(());
                    }
                    self.cv.wait(&mut state);
                }
            };

            match entry {
                SubmitEntry::Record(jsonl) => {
                    if let Err(e) = sink.write_all(jsonl.as_bytes()) {
                        let mut state = self.state.lock();
                        state.sink_failed = true;
                        state.pending.insert(to_publish, SubmitEntry::Record(jsonl));
                        self.cv.notify_all();
                        return Err(e);
                    }
                    if let Err(e) = sink.flush() {
                        let mut state = self.state.lock();
                        state.sink_failed = true;
                        state.pending.insert(to_publish, SubmitEntry::Record(jsonl));
                        self.cv.notify_all();
                        return Err(e);
                    }
                }
                SubmitEntry::Cancelled => {
                    // Advance cursor without writing.
                }
            }

            {
                let mut state = self.state.lock();
                state.next_to_publish = to_publish + 1;
                self.cv.notify_all();
            }
        }
    }

    fn is_sink_failed(&self) -> bool {
        self.state.lock().sink_failed
    }

    fn next_sequence(&self) -> u64 {
        let mut state = self.state.lock();
        let seq = state.next_to_reserve;
        state.next_to_reserve = seq.checked_add(1).unwrap_or(seq);
        seq
    }
}

/// The ordered commit object. Manages sequence reservation, publication, and
/// RAII cancellation.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-001
pub struct OrderedCommit {
    inner: Arc<OrderedCommitInner>,
}

impl OrderedCommit {
    /// Create a new ordered commit object starting at sequence 0.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OrderedCommitInner {
                state: Mutex::new(OrderedCommitState {
                    next_to_publish: 0,
                    next_to_reserve: 0,
                    pending: BTreeMap::new(),
                    sink_failed: false,
                }),
                cv: Condvar::new(),
            }),
        }
    }

    /// Reserve the next sequence number. Returns an RAII guard that publishes
    /// on commit or drop.
    #[must_use]
    pub fn reserve(&self) -> Reservation {
        let seq = self.inner.next_sequence();
        Reservation {
            sequence: seq,
            committed: None,
            commit: Some(Arc::clone(&self.inner)),
        }
    }

    /// Reserve a specific sequence number (for checked-add callers).
    #[must_use]
    pub fn reserve_sequence(&self, sequence: u64) -> Reservation {
        Reservation {
            sequence,
            committed: None,
            commit: Some(Arc::clone(&self.inner)),
        }
    }

    /// Publish all pending records in order to the given sink. Blocks until
    /// all currently-submitted records are written.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
    /// @requirement REQ-IO-001
    pub fn publish_all(&self, sink: &mut dyn Write) -> Result<(), std::io::Error> {
        self.inner.run_publisher(sink)
    }

    /// Whether the sink has experienced a fatal error.
    #[must_use]
    pub fn is_sink_failed(&self) -> bool {
        self.inner.is_sink_failed()
    }

    /// The next sequence number that would be reserved.
    #[must_use]
    pub fn next_sequence(&self) -> u64 {
        self.inner.next_sequence()
    }
}

impl Default for OrderedCommit {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- JSONL records ---

    #[test]
    fn record_roundtrip() {
        let rec = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: 5,
            input_seen: 10,
            present_seen: 3,
            elapsed_ms: 1234,
            kind: RecordKind::InputTick,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: None,
            activity: None,
        };
        let jsonl = rec.to_jsonl().unwrap();
        assert!(jsonl.ends_with('\n'));
        let parsed = TraceRecord::from_jsonl(&jsonl).unwrap();
        assert_eq!(parsed, rec);
    }

    #[test]
    fn record_with_semantic_transition() {
        let rec = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: 0,
            input_seen: 0,
            present_seen: 0,
            elapsed_ms: 0,
            kind: RecordKind::SemanticAssertion,
            label: None,
            from: Some("NewGame".into()),
            to: Some("LoadGame".into()),
            terminal_reason: None,
            seed_application: None,
            presentation: None,
            activity: None,
        };
        let jsonl = rec.to_jsonl().unwrap();
        assert!(jsonl.contains("NewGame"));
        assert!(jsonl.contains("LoadGame"));
        assert!(jsonl.contains("semantic_assertion"));
    }

    #[test]
    fn typed_runtime_evidence_roundtrips() {
        let records = [
            TraceRecord {
                schema: TraceRecord::SCHEMA,
                run: 1,
                sequence: 0,
                input_seen: 0,
                present_seen: 0,
                elapsed_ms: 0,
                kind: RecordKind::SeedApplication,
                label: None,
                from: None,
                to: None,
                terminal_reason: None,
                seed_application: Some(SeedApplication {
                    domain: SeedDomain::SuperMeleeBattle,
                    seed: 0x55AA_2317,
                }),
                presentation: None,
                activity: None,
            },
            TraceRecord {
                schema: TraceRecord::SCHEMA,
                run: 1,
                sequence: 1,
                input_seen: 1,
                present_seen: 2,
                elapsed_ms: 3,
                kind: RecordKind::Presentation,
                label: None,
                from: None,
                to: None,
                terminal_reason: None,
                seed_application: None,
                presentation: Some(PresentationEvidence {
                    count: 2,
                    generation: 4,
                    width: 320,
                    height: 240,
                }),
                activity: None,
            },
            TraceRecord {
                schema: TraceRecord::SCHEMA,
                run: 1,
                sequence: 2,
                input_seen: 2,
                present_seen: 2,
                elapsed_ms: 4,
                kind: RecordKind::SemanticAssertion,
                label: Some("activity_assertion_passed".into()),
                from: None,
                to: None,
                terminal_reason: None,
                seed_application: None,
                presentation: None,
                activity: Some(ActivityEvidence {
                    word: 0x0102,
                    mask: 0x00ff,
                    equals: 2,
                    passed: true,
                }),
            },
        ];

        for record in records {
            let parsed = TraceRecord::from_jsonl(&record.to_jsonl().unwrap()).unwrap();
            assert_eq!(parsed, record);
        }
    }

    #[test]
    fn seed_domains_are_strict_and_repeated_applications_remain_distinct() {
        assert_eq!(SeedDomain::from_ffi(1), Some(SeedDomain::SuperMeleeMenu));
        assert_eq!(SeedDomain::from_ffi(2), Some(SeedDomain::SuperMeleeBattle));
        assert_eq!(SeedDomain::from_ffi(0), None);
        assert_eq!(SeedDomain::from_ffi(3), None);

        let first = SeedApplication {
            domain: SeedDomain::SuperMeleeBattle,
            seed: 0x55AA_2317,
        };
        let second = first.clone();
        assert_eq!(first, second);
    }

    #[test]
    fn each_line_independently_parses() {
        let recs: Vec<_> = (0..5)
            .map(|i| TraceRecord {
                schema: TraceRecord::SCHEMA,
                run: 1,
                sequence: i,
                input_seen: i,
                present_seen: 0,
                elapsed_ms: i * 100,
                kind: RecordKind::InputTick,
                label: None,
                from: None,
                to: None,
                terminal_reason: None,
                seed_application: None,
                presentation: None,
                activity: None,
            })
            .collect();
        let lines: Vec<_> = recs.iter().map(|r| r.to_jsonl().unwrap()).collect();
        let jsonl = lines.concat();
        for (i, line) in jsonl.lines().enumerate() {
            let parsed = TraceRecord::from_jsonl(line).unwrap();
            assert_eq!(parsed.sequence, i as u64);
        }
    }

    #[test]
    fn record_rejects_missing_newline_independent_parse() {
        // Each line is independent; missing fields cause parse failure.
        let bad = r#"{"schema":1}"#;
        assert!(TraceRecord::from_jsonl(bad).is_err());
    }

    // --- OrderedCommit: sequential publication ---

    #[test]
    fn ordered_commit_sequential() {
        let oc = OrderedCommit::new();
        let r0 = oc.reserve();
        let r1 = oc.reserve();
        let r2 = oc.reserve();

        r0.commit_record("rec0\n".into());
        r1.commit_record("rec1\n".into());
        r2.commit_record("rec2\n".into());

        let mut sink = Vec::new();
        oc.publish_all(&mut sink).unwrap();
        let output = String::from_utf8(sink).unwrap();
        assert_eq!(output, "rec0\nrec1\nrec2\n");
    }

    // --- OrderedCommit: out-of-order completion publishes in order ---

    #[test]
    fn ordered_commit_out_of_order() {
        let oc = OrderedCommit::new();
        let r0 = oc.reserve();
        let r1 = oc.reserve();
        let r2 = oc.reserve();

        // Drop in reverse order.
        r2.commit_record("rec2\n".into());
        r1.commit_record("rec1\n".into());
        r0.commit_record("rec0\n".into());

        let mut sink = Vec::new();
        oc.publish_all(&mut sink).unwrap();
        let output = String::from_utf8(sink).unwrap();
        assert_eq!(output, "rec0\nrec1\nrec2\n");
    }

    // --- OrderedCommit: dropped reservation cancels (no gap) ---

    #[test]
    fn dropped_reservation_cancels_no_gap() {
        let oc = OrderedCommit::new();
        let r0 = oc.reserve();
        let _r1 = oc.reserve(); // will be dropped (cancelled)
        let r2 = oc.reserve();

        r0.commit_record("rec0\n".into());
        drop(_r1); // cancelled
        r2.commit_record("rec2\n".into());

        let mut sink = Vec::new();
        oc.publish_all(&mut sink).unwrap();
        let output = String::from_utf8(sink).unwrap();
        // rec0 published, seq1 cancelled (no gap), rec2 published
        assert_eq!(output, "rec0\nrec2\n");
    }

    // --- OrderedCommit: first sink failure rejects later success ---

    #[test]
    fn first_sink_failure_rejects_later_success() {
        // Use a writer that always fails.
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("sink fail"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let oc = OrderedCommit::new();
        let r0 = oc.reserve();
        let r1 = oc.reserve();

        r0.commit_record("rec0\n".into());
        r1.commit_record("rec1\n".into());

        let mut sink = FailingWriter;
        let _ = oc.publish_all(&mut sink);
        assert!(oc.is_sink_failed());
    }

    // --- OrderedCommit: cancelled slots advance cursor ---

    #[test]
    fn cancelled_slots_advance_cursor() {
        let oc = OrderedCommit::new();
        let r0 = oc.reserve();
        let r1 = oc.reserve();
        let r2 = oc.reserve();

        r0.commit_record("rec0\n".into());
        r1.cancel();
        r2.commit_record("rec2\n".into());

        let mut sink = Vec::new();
        oc.publish_all(&mut sink).unwrap();
        let output = String::from_utf8(sink).unwrap();
        assert_eq!(output, "rec0\nrec2\n");
    }

    // --- OrderedCommit: no runtime mutex needed ---

    #[test]
    fn publish_all_does_not_require_runtime_mutex() {
        // This is a structural test: publish_all takes &mut dyn Write, not a
        // runtime mutex. The ordered commit uses its own internal Mutex +
        // Condvar.
        let oc = OrderedCommit::new();
        let r0 = oc.reserve();
        r0.commit_record("rec0\n".into());
        let mut sink = Vec::new();
        oc.publish_all(&mut sink).unwrap();
    }

    // --- OrderedCommit: explicit cancel does not leave gap ---

    #[test]
    fn explicit_cancel_no_gap() {
        let oc = OrderedCommit::new();
        let r0 = oc.reserve();
        let r1 = oc.reserve();
        let r2 = oc.reserve();

        r0.commit_record("rec0\n".into());
        r1.cancel();
        r2.commit_record("rec2\n".into());

        let mut sink = Vec::new();
        oc.publish_all(&mut sink).unwrap();
        let output = String::from_utf8(sink).unwrap();
        assert_eq!(output, "rec0\nrec2\n");
    }
}
