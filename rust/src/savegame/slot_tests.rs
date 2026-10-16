//! Slot storage against a real uio repository mounted on a scratch directory
//! that the test owns and removes.

use std::ffi::{c_void, CString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::ptr;

use serial_test::serial;
use tempfile::TempDir;

use super::*;
use crate::io::uio_bridge::{
    uio_Repository, uio_closeDir, uio_closeRepository, uio_init, uio_mountDir, uio_openDir,
    uio_openRepository, uio_unInit,
};

const UIO_FSTYPE_STDIO: i32 = 1;
const UIO_MOUNT_TOP: i32 = 1 << 2;

/// The save game the C game wrote (epic 20 load/save evidence).
const C_WRITTEN_SAVE: &[u8] = include_bytes!(
    "../../../docs/evidence/epic-20/exact-production-79e68744e/load-save/seed-uqmsave.00"
);

/// A scratch directory mounted at the root of a fresh uio repository, used
/// as the save directory.
struct SaveDirFixture {
    scratch: TempDir,
    repository: *mut uio_Repository,
    dir: *mut c_void,
}

impl SaveDirFixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/save-slot-test-scratch");
        fs::create_dir_all(&root).unwrap();
        let scratch = TempDir::new_in(&root).unwrap();
        unsafe {
            uio_init();
            let repository = uio_openRepository(0);
            let root = CString::new("/").unwrap();
            let host = CString::new(scratch.path().to_str().unwrap()).unwrap();
            let mount = uio_mountDir(
                repository,
                root.as_ptr(),
                UIO_FSTYPE_STDIO,
                ptr::null_mut(),
                ptr::null(),
                host.as_ptr(),
                ptr::null_mut(),
                UIO_MOUNT_TOP,
                ptr::null_mut(),
            );
            assert!(!mount.is_null());
            let dir = uio_openDir(repository, root.as_ptr(), 0);
            assert!(!dir.is_null());
            SaveDirFixture {
                scratch,
                repository,
                dir: dir.cast(),
            }
        }
    }

    fn slots(&self) -> SaveSlots {
        unsafe { SaveSlots::new(self.dir).unwrap() }
    }

    fn host(&self, name: &str) -> PathBuf {
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

impl Drop for SaveDirFixture {
    fn drop(&mut self) {
        unsafe {
            uio_closeDir(self.dir.cast());
            uio_closeRepository(self.repository);
            uio_unInit();
        }
    }
}

fn seed() -> SaveFile {
    decode_save(C_WRITTEN_SAVE).unwrap()
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
fn a_null_save_directory_is_refused() {
    assert!(matches!(
        unsafe { SaveSlots::new(ptr::null_mut()) },
        Err(SlotError::NoSaveDir)
    ));
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
