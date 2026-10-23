//! Descriptor-owned, bounded live trace publication before native acknowledgements.

use super::trace::OrderedCommit;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
};

pub(super) struct NativeTraceJournal {
    file: File,
    pending: Vec<u8>,
    remaining: u64,
}

impl NativeTraceJournal {
    pub(super) fn create(root: &Path, limit: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("trace.jsonl"))?;
        Ok(Self {
            file,
            pending: Vec::new(),
            remaining: limit,
        })
    }

    /// Drain ordered records into bounded memory without touching the file.
    pub(super) fn stage(&mut self, commit: &OrderedCommit) -> io::Result<()> {
        commit.publish_all(self)
    }

    /// Publish one batch before a proof boundary is acknowledged. The
    /// controller still syncs the finished trace before hashing it.
    pub(super) fn publish(&mut self, commit: &OrderedCommit) -> io::Result<()> {
        self.stage(commit)?;
        self.file.write_all(&self.pending)?;
        self.file.flush()?;
        self.pending.clear();
        Ok(())
    }
}

impl Write for NativeTraceJournal {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other(
                "native trace exhausted its authority byte reservation",
            ));
        }
        self.pending.extend_from_slice(bytes);
        self.remaining -= bytes.len() as u64;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // OrderedCommit flushes after each record; publication owns file I/O.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_prefix_is_readable_before_finalization_and_preserves_sequence() {
        let root = tempfile::tempdir().unwrap();
        let commit = OrderedCommit::new();
        let mut journal = NativeTraceJournal::create(root.path(), 64).unwrap();
        commit.reserve().commit_record("first\n".into());
        journal.publish(&commit).unwrap();
        assert_eq!(
            std::fs::read(root.path().join("trace.jsonl")).unwrap(),
            b"first\n"
        );
        commit.reserve().commit_record("second\n".into());
        journal.publish(&commit).unwrap();
        assert_eq!(
            std::fs::read(root.path().join("trace.jsonl")).unwrap(),
            b"first\nsecond\n"
        );
    }

    #[test]
    fn staging_is_bounded_and_batches_preserve_the_exact_ordered_bytes() {
        let root = tempfile::tempdir().unwrap();
        let commit = OrderedCommit::new();
        let mut journal = NativeTraceJournal::create(root.path(), 4096).unwrap();
        let mut expected = Vec::new();
        for index in 0..100 {
            let line = format!("record:{index}\n");
            expected.extend_from_slice(line.as_bytes());
            commit.reserve().commit_record(line);
            journal.stage(&commit).unwrap();
            if index % 25 == 0 {
                journal.publish(&commit).unwrap();
                assert_eq!(
                    std::fs::read(root.path().join("trace.jsonl")).unwrap(),
                    expected
                );
            }
        }
        assert!(!journal.pending.is_empty());
        journal.publish(&commit).unwrap();
        assert_eq!(
            std::fs::read(root.path().join("trace.jsonl")).unwrap(),
            expected
        );
        assert!(journal.pending.is_empty());
        assert_eq!(journal.remaining, 4096 - expected.len() as u64);
    }

    #[test]
    fn an_unpublished_batch_cannot_exceed_the_trace_reservation() {
        let root = tempfile::tempdir().unwrap();
        let commit = OrderedCommit::new();
        let mut journal = NativeTraceJournal::create(root.path(), 6).unwrap();
        commit.reserve().commit_record("first\n".into());
        journal.stage(&commit).unwrap();
        assert!(std::fs::read(root.path().join("trace.jsonl"))
            .unwrap()
            .is_empty());
        commit.reserve().commit_record("second\n".into());
        assert!(journal.stage(&commit).is_err());
        assert!(commit.is_sink_failed());
        assert_eq!(journal.pending, b"first\n");
    }

    #[test]
    fn collisions_and_exhaustion_do_not_overwrite_or_publish_partial_records() {
        let root = tempfile::tempdir().unwrap();
        let mut journal = NativeTraceJournal::create(root.path(), 6).unwrap();
        assert!(NativeTraceJournal::create(root.path(), 6).is_err());
        let commit = OrderedCommit::new();
        commit.reserve().commit_record("first\n".into());
        journal.publish(&commit).unwrap();
        commit.reserve().commit_record("second\n".into());
        assert!(journal.publish(&commit).is_err());
        assert!(commit.is_sink_failed());
        assert_eq!(
            std::fs::read(root.path().join("trace.jsonl")).unwrap(),
            b"first\n"
        );
    }
}
