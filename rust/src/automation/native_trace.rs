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

    pub(super) fn publish(&mut self, commit: &OrderedCommit) -> io::Result<()> {
        commit.publish_all(self)?;
        self.file.sync_all()
    }
}

impl Write for NativeTraceJournal {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::other(
                "native trace exhausted its authority byte reservation",
            ));
        }
        self.file.write_all(bytes)?;
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
}
