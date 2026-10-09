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

use super::trace::TraceRecord;
use std::borrow::Cow;

#[derive(Debug, Default)]
pub(crate) struct TraceCursor {
    /// Exact bytes, including terminators, of the records in `records`.
    prefix: Vec<u8>,
    records: Vec<TraceRecord>,
    /// Lines deserialized over the cursor's lifetime, for tests.
    parsed_lines: u64,
}

impl TraceCursor {
    /// The first `count` nonempty lines of `bytes`, parsed as trace records.
    ///
    /// Equivalent to splitting `bytes` on `\n`, skipping empty lines, taking
    /// `count` and deserializing each, including when fewer lines exist.
    pub(crate) fn records(
        &mut self,
        bytes: &[u8],
        count: usize,
    ) -> Result<Cow<'_, [TraceRecord]>, serde_json::Error> {
        if !bytes.starts_with(&self.prefix) {
            self.prefix.clear();
            self.records.clear();
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
                // A record still being written is used once, never cached.
                tail = Some(record);
                break;
            }
            self.prefix.extend_from_slice(&rest[..consumed]);
            self.records.push(record);
            offset += consumed;
        }
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

    #[cfg(test)]
    pub(crate) const fn parsed_lines(&self) -> u64 {
        self.parsed_lines
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
}
