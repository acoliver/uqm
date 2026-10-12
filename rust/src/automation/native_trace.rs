//! Descriptor-owned, bounded live trace publication before native acknowledgements.

use super::trace::OrderedCommit;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
};

pub(super) struct NativeTraceJournal {
    file: File,
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
            remaining: limit,
        })
    }

    /// Make every committed record visible to the observer before the
    /// presentation is acknowledged.
    ///
    /// The observer reads the file on the same host, so completed writes are
    /// already visible to it; a flush to stable storage per presentation only
    /// protects against power loss and made each frame wait on the disk. The
    /// controller syncs the finished trace once before it hashes it.
    pub(super) fn publish(&mut self, commit: &OrderedCommit) -> io::Result<()> {
        commit.publish_all(self)?;
        self.flush()
    }
}

impl Write for NativeTraceJournal {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other(
                "native trace exhausted its authority byte reservation",
            ));
        }
        #[cfg(feature = "debug-process")]
        let written = super::syscall_latency::timed_write_all(
            super::syscall_latency::Target::TraceJournal,
            super::syscall_latency::Syscall::WriteAll,
            &mut self.file,
            bytes,
        );
        #[cfg(not(feature = "debug-process"))]
        let written = self.file.write_all(bytes);
        written?;
        self.remaining -= bytes.len() as u64;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
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

    #[cfg(feature = "debug-process")]
    #[test]
    fn journal_writes_are_counted_once_per_record_with_their_actual_bytes() {
        use crate::automation::syscall_latency::{counts_of, drain_for_tests};

        drain_for_tests();
        let root = tempfile::tempdir().unwrap();
        let commit = OrderedCommit::new();
        let mut journal = NativeTraceJournal::create(root.path(), 13).unwrap();
        commit.reserve().commit_record("first\n".into());
        commit.reserve().commit_record("second\n".into());
        journal.publish(&commit).unwrap();
        let report = drain_for_tests().unwrap();
        assert_eq!(counts_of(&report, "trace_journal", "write_all"), (2, 0, 13));
        assert_eq!(report.operations.len(), 1);

        // A record beyond the reservation is refused before any write.
        commit.reserve().commit_record("third\n".into());
        assert!(journal.publish(&commit).is_err());
        assert_eq!(drain_for_tests(), None);
    }
}
