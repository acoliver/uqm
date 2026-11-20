//! Slot storage through an isolated Vfs on an owned scratch directory.

use super::*;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uqm_engine::host::{FileStat, FileStream, Files};
use uqm_engine::io::Vfs;

/// The save game the C game wrote (epic 20 load/save evidence).
const C_WRITTEN_SAVE: &[u8] = include_bytes!("../../tests/fixtures/savegame/legacy-save.00");

/// An isolated Vfs rooted in an owned scratch directory.
pub(super) struct SaveDirFixture {
    scratch: TempDir,
    vfs: Vfs,
}

impl SaveDirFixture {
    pub(super) fn new() -> Self {
        let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
        fs::create_dir_all(&temp).unwrap();
        Self {
            scratch: TempDir::new_in(temp).unwrap(),
            vfs: Vfs::default(),
        }
    }

    pub(super) fn slots(&self) -> SaveSlots<'_> {
        SaveSlots::new(self)
    }

    pub(super) fn host(&self, name: &str) -> PathBuf {
        self.scratch.path().join(name)
    }

    fn host_names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.scratch.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Files for SaveDirFixture {
    fn open_read(&self, path: &Path) -> std::io::Result<FileStream> {
        self.vfs.open_read(&self.scratch.path().join(path))
    }
    fn open_write(&self, path: &Path) -> std::io::Result<FileStream> {
        self.vfs.open_write(&self.scratch.path().join(path))
    }
    fn remove(&self, path: &Path) -> std::io::Result<()> {
        self.vfs.remove(&self.scratch.path().join(path))
    }
    fn list(&self, path: &Path) -> std::io::Result<Vec<String>> {
        self.vfs.list(&self.scratch.path().join(path))
    }
    fn stat(&self, path: &Path) -> std::io::Result<FileStat> {
        self.vfs.stat(&self.scratch.path().join(path))
    }
}

fn seed() -> SaveFile {
    decode_save(C_WRITTEN_SAVE).unwrap()
}

#[test]
#[serial]
fn scratch_directories_are_unique_temporary_and_removed_on_drop() {
    let scratch_path = || {
        let fx = SaveDirFixture::new();
        let path = fx.scratch.path().to_path_buf();
        assert!(
            path.starts_with(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp")),
            "{path:?}"
        );
        fs::write(fx.host("scratch.txt"), b"owned by the test").unwrap();
        drop(fx);
        assert!(!path.exists(), "fixture must remove its scratch directory");
        path
    };
    assert_ne!(scratch_path(), scratch_path());
}

#[test]
fn slot_file_names_are_uqmsave_with_two_digits() {
    assert_eq!(SaveSlots::file_name(0).unwrap(), "uqmsave.00");
    assert_eq!(SaveSlots::file_name(7).unwrap(), "uqmsave.07");
    assert_eq!(SaveSlots::file_name(49).unwrap(), "uqmsave.49");
    assert_eq!(
        SaveSlots::file_name(50),
        Err(SlotError::OutOfRange { slot: 50 })
    );
    assert_eq!(MAX_SAVED_GAMES, 50);
}

#[test]
#[serial]
fn write_puts_exactly_the_encoded_bytes_in_the_named_file() {
    let fx = SaveDirFixture::new();
    fx.slots().write(7, &seed()).unwrap();

    // The file on the host is byte-identical to what the C game wrote.
    assert_eq!(fs::read(fx.host("uqmsave.07")).unwrap(), C_WRITTEN_SAVE);
    assert_eq!(fx.host_names(), vec!["uqmsave.07".to_string()]);
}

#[test]
#[serial]
fn write_replaces_a_longer_existing_file() {
    let fx = SaveDirFixture::new();
    fs::write(fx.host("uqmsave.02"), vec![0xEE; C_WRITTEN_SAVE.len() * 2]).unwrap();
    fx.slots().write(2, &seed()).unwrap();
    assert_eq!(fs::read(fx.host("uqmsave.02")).unwrap(), C_WRITTEN_SAVE);
}

#[test]
#[serial]
fn read_decodes_a_file_the_game_wrote() {
    let fx = SaveDirFixture::new();
    fs::write(fx.host("uqmsave.03"), C_WRITTEN_SAVE).unwrap();
    let slots = fx.slots();
    assert_eq!(slots.read(3).unwrap(), seed());
    assert_eq!(slots.read_summary(3).unwrap(), seed().summary);
}

#[test]
#[serial]
fn written_slot_reads_back_equal() {
    let fx = SaveDirFixture::new();
    let slots = fx.slots();
    let mut save = seed();
    save.summary.save_name = SaveName::new("Caf\u{e9}".as_bytes()).unwrap();
    slots.write(49, &save).unwrap();
    assert_eq!(slots.read(49).unwrap(), save);
}

#[test]
#[serial]
fn summary_read_needs_only_the_first_chunk() {
    let fx = SaveDirFixture::new();
    // Magic and Summ chunk only: 4 + 8 + 160 bytes.
    fs::write(fx.host("uqmsave.04"), &C_WRITTEN_SAVE[..172]).unwrap();
    let slots = fx.slots();
    assert_eq!(slots.read_summary(4).unwrap(), seed().summary);
    assert_eq!(
        slots.read(4),
        Err(SlotError::Format {
            slot: 4,
            error: SaveFormatError::Truncated {
                at: Section::ChunkHeader
            }
        })
    );
}

#[test]
#[serial]
fn missing_slot_is_missing() {
    let fx = SaveDirFixture::new();
    let slots = fx.slots();
    assert_eq!(slots.read(1), Err(SlotError::Missing { slot: 1 }));
    assert_eq!(
        slots.read_summary(50),
        Err(SlotError::OutOfRange { slot: 50 })
    );
    assert_eq!(
        slots.write(50, &seed()),
        Err(SlotError::OutOfRange { slot: 50 })
    );
    assert!(fx.host_names().is_empty());
}

#[test]
#[serial]
fn corrupt_ums_slot_is_an_error_not_a_legacy_fallback() {
    let fx = SaveDirFixture::new();
    let mut bad = C_WRITTEN_SAVE.to_vec();
    bad[0] = b'X';
    fs::write(fx.host("uqmsave.05"), &bad).unwrap();
    // A legacy file for the same slot must not be consulted.
    fs::write(fx.host("starcon2.05"), b"legacy").unwrap();
    assert_eq!(
        fx.slots().read(5),
        Err(SlotError::Format {
            slot: 5,
            error: SaveFormatError::BadMagic {
                found: u32::from_le_bytes(*b"XMS\x01")
            }
        })
    );
}

#[test]
#[serial]
fn slot_with_only_a_pre_ums_file_says_so() {
    let fx = SaveDirFixture::new();
    fs::write(fx.host("starcon2.08"), b"legacy").unwrap();
    let slots = fx.slots();
    assert_eq!(
        slots.read(8),
        Err(SlotError::LegacyFormatUnsupported { slot: 8 })
    );
    assert_eq!(
        slots.read_summary(8),
        Err(SlotError::LegacyFormatUnsupported { slot: 8 })
    );
}

#[test]
#[serial]
fn summaries_cover_all_fifty_slots() {
    let fx = SaveDirFixture::new();
    fs::write(fx.host("uqmsave.00"), C_WRITTEN_SAVE).unwrap();
    fs::write(fx.host("uqmsave.49"), C_WRITTEN_SAVE).unwrap();
    fs::write(fx.host("uqmsave.06"), &C_WRITTEN_SAVE[..100]).unwrap();
    fs::write(fx.host("uqmsave.07"), b"").unwrap();
    fs::write(fx.host("starcon2.08"), b"legacy").unwrap();
    fs::create_dir(fx.host("uqmsave.09")).unwrap();
    // Not slots: wrong name, and slot numbers past the last slot.
    fs::write(fx.host("uqmsave.50"), C_WRITTEN_SAVE).unwrap();
    fs::write(fx.host("uqmsave.1"), C_WRITTEN_SAVE).unwrap();

    let states = fx.slots().read_summaries();
    assert_eq!(states.len(), 50);
    let summary = seed().summary;
    assert_eq!(states[0], SlotState::Occupied(summary.clone()));
    assert_eq!(states[49], SlotState::Occupied(summary));
    assert_eq!(
        states[6],
        SlotState::Unreadable(SlotError::Format {
            slot: 6,
            error: SaveFormatError::Truncated {
                at: Section::Chunk(SUMMARY_TAG_FOR_TESTS)
            }
        })
    );
    assert_eq!(
        states[7],
        SlotState::Unreadable(SlotError::Format {
            slot: 7,
            error: SaveFormatError::Truncated {
                at: Section::FileMagic
            }
        })
    );
    assert_eq!(
        states[8],
        SlotState::Unreadable(SlotError::LegacyFormatUnsupported { slot: 8 })
    );
    assert_eq!(
        states[9],
        SlotState::Unreadable(SlotError::IsDirectory { slot: 9 })
    );
    for (slot, state) in states.iter().enumerate() {
        if ![0, 6, 7, 8, 9, 49].contains(&slot) {
            assert_eq!(state, &SlotState::Empty, "slot {slot}");
        }
    }
}

const SUMMARY_TAG_FOR_TESTS: u32 = u32::from_le_bytes(*b"Summ");

#[test]
#[serial]
fn write_to_a_directory_named_like_a_slot_is_refused_and_leaves_it() {
    let fx = SaveDirFixture::new();
    fs::create_dir(fx.host("uqmsave.11")).unwrap();
    assert_eq!(
        fx.slots().write(11, &seed()),
        Err(SlotError::IsDirectory { slot: 11 })
    );
    assert!(fx.host("uqmsave.11").is_dir());
}

#[test]
#[serial]
fn failure_to_open_leaves_the_existing_file_alone() {
    let fx = SaveDirFixture::new();
    let path = fx.host("uqmsave.12");
    fs::write(&path, b"keep me").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();

    let outcome = fx.slots().write(12, &seed());

    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(outcome, Err(SlotError::OpenFailed { slot: 12 }));
    assert_eq!(fs::read(&path).unwrap(), b"keep me");
}

#[test]
#[serial]
fn failed_write_removes_only_that_slots_file() {
    let fx = SaveDirFixture::new();
    fs::write(fx.host("uqmsave.12"), b"half written").unwrap();
    fs::write(fx.host("uqmsave.13"), b"neighbour").unwrap();
    fs::write(fx.host("starcon2.12"), b"legacy").unwrap();
    fs::write(fx.host("notes.txt"), b"unrelated").unwrap();
    let slots = fx.slots();

    assert_eq!(
        slots.settle_write(12, false, true),
        Err(SlotError::WriteFailed {
            slot: 12,
            removed: true
        })
    );
    assert_eq!(
        fx.host_names(),
        vec!["notes.txt", "starcon2.12", "uqmsave.13"]
    );

    // A failed close is a failed write too.
    fs::write(fx.host("uqmsave.12"), b"half written").unwrap();
    assert_eq!(
        slots.settle_write(12, true, false),
        Err(SlotError::WriteFailed {
            slot: 12,
            removed: true
        })
    );
    assert!(!fx.host("uqmsave.12").exists());

    // Nothing left to remove is reported, not hidden.
    assert_eq!(
        slots.settle_write(12, false, false),
        Err(SlotError::WriteFailed {
            slot: 12,
            removed: false
        })
    );
}

#[test]
#[serial]
fn successful_write_removes_nothing() {
    let fx = SaveDirFixture::new();
    fs::write(fx.host("uqmsave.12"), b"whole").unwrap();
    assert_eq!(fx.slots().settle_write(12, true, true), Ok(()));
    assert_eq!(fs::read(fx.host("uqmsave.12")).unwrap(), b"whole");
}

struct FailingFile {
    inner: FileStream,
    fail_flush: bool,
}
impl std::io::Read for FailingFile {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.inner, bytes)
    }
}
impl std::io::Write for FailingFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if !self.fail_flush {
            if self.inner.position() != 0 {
                return Err(std::io::ErrorKind::StorageFull.into());
            }
            return std::io::Write::write(&mut self.inner, &bytes[..bytes.len().min(2)]);
        }
        std::io::Write::write(&mut self.inner, bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.fail_flush {
            Err(std::io::ErrorKind::StorageFull.into())
        } else {
            std::io::Write::flush(&mut self.inner)
        }
    }
}
impl std::io::Seek for FailingFile {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.inner, position)
    }
}
impl uqm_engine::host::File for FailingFile {
    fn len(&self) -> std::io::Result<u64> {
        self.inner.len()
    }
}
struct FailingFiles<'a> {
    directory: &'a SaveDirFixture,
    fail_flush: bool,
}
impl Files for FailingFiles<'_> {
    fn open_read(&self, path: &Path) -> std::io::Result<FileStream> {
        self.directory.open_read(path)
    }
    fn open_write(&self, path: &Path) -> std::io::Result<FileStream> {
        Ok(FileStream::new(FailingFile {
            inner: self.directory.open_write(path)?,
            fail_flush: self.fail_flush,
        }))
    }
    fn remove(&self, path: &Path) -> std::io::Result<()> {
        self.directory.remove(path)
    }
    fn list(&self, path: &Path) -> std::io::Result<Vec<String>> {
        self.directory.list(path)
    }
    fn stat(&self, path: &Path) -> std::io::Result<FileStat> {
        self.directory.stat(path)
    }
}

#[test]
fn partial_write_and_finish_errors_remove_only_the_failed_slot() {
    let fx = SaveDirFixture::new();
    fs::write(fx.host("uqmsave.18"), b"neighbour").unwrap();
    fs::write(fx.host("starcon2.17"), b"legacy").unwrap();
    for fail_flush in [false, true] {
        let files = FailingFiles {
            directory: &fx,
            fail_flush,
        };
        let slots = SaveSlots::new(&files);
        assert_eq!(
            slots.write(17, &seed()),
            Err(SlotError::WriteFailed {
                slot: 17,
                removed: true
            })
        );
        assert!(!fx.host("uqmsave.17").exists());
        assert_eq!(fs::read(fx.host("uqmsave.18")).unwrap(), b"neighbour");
        assert_eq!(fs::read(fx.host("starcon2.17")).unwrap(), b"legacy");
    }
}
