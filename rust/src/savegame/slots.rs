//! Save slot storage through the Files port, with failed writes removed.

use std::io::Write;
use std::path::Path;

use super::decode::{decode_save_from, decode_summary_from};
use super::encode::encode_save;
use super::error::SlotError;
use super::types::{SaveFile, Summary, MAX_SAVED_GAMES};
use crate::engine::host::{FileStream, Files};

/// What one slot holds, as the slot list needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotState {
    /// No save file of either generation exists.
    Empty,
    Occupied(Summary),
    /// A file exists but cannot be used; the error says why.
    Unreadable(SlotError),
}

/// The 50 save slots of one save directory.
pub struct SaveSlots<'a> {
    files: &'a dyn Files,
}

impl<'a> SaveSlots<'a> {
    pub fn new(files: &'a dyn Files) -> Self {
        Self { files }
    }

    /// `uqmsave.%02u` (save.c:765).
    pub fn file_name(slot: usize) -> Result<String, SlotError> {
        check_slot(slot)?;
        Ok(format!("uqmsave.{slot:02}"))
    }

    /// The pre-UMS name `LoadLegacyGame` looks for (load_legacy.c:624).
    fn legacy_file_name(slot: usize) -> String {
        format!("starcon2.{slot:02}")
    }

    /// Write the whole file, removing it if writing or finishing fails.
    /// A failure to open leaves whatever was there untouched.
    pub fn write(&self, slot: usize, save: &SaveFile) -> Result<(), SlotError> {
        let name = Self::file_name(slot)?;
        let bytes = encode_save(save).map_err(|error| SlotError::Encode { slot, error })?;
        let path = Path::new(&name);
        if self.is_directory(path) {
            return Err(SlotError::IsDirectory { slot });
        }
        let mut stream = self
            .files
            .open_write(path)
            .map_err(|_| SlotError::OpenFailed { slot })?;
        let written = stream.write_all(&bytes).is_ok();
        let closed = stream.finish().is_ok();
        self.settle_write(slot, written, closed)
    }

    /// A failed write leaves no slot file behind. Only that slot is removed.
    pub(super) fn settle_write(
        &self,
        slot: usize,
        written: bool,
        closed: bool,
    ) -> Result<(), SlotError> {
        if written && closed {
            return Ok(());
        }
        let removed = self
            .files
            .remove(Path::new(&Self::file_name(slot)?))
            .is_ok();
        Err(SlotError::WriteFailed { slot, removed })
    }

    /// Read and decode a whole save.
    pub fn read(&self, slot: usize) -> Result<SaveFile, SlotError> {
        let (mut stream, len) = self.open_for_read(slot)?;
        decode_save_from(&mut stream, len).map_err(|error| SlotError::Format { slot, error })
    }

    /// Read only the summary, without touching the rest of the file.
    pub fn read_summary(&self, slot: usize) -> Result<Summary, SlotError> {
        let (mut stream, len) = self.open_for_read(slot)?;
        decode_summary_from(&mut stream, len).map_err(|error| SlotError::Format { slot, error })
    }

    /// The summary of every slot, in slot order.
    pub fn read_summaries(&self) -> Vec<SlotState> {
        (0..MAX_SAVED_GAMES)
            .map(|slot| match self.read_summary(slot) {
                Ok(summary) => SlotState::Occupied(summary),
                Err(SlotError::Missing { .. }) => SlotState::Empty,
                Err(error) => SlotState::Unreadable(error),
            })
            .collect()
    }

    fn open_for_read(&self, slot: usize) -> Result<(FileStream, u64), SlotError> {
        let name = Self::file_name(slot)?;
        let path = Path::new(&name);
        if self.is_directory(path) {
            return Err(SlotError::IsDirectory { slot });
        }
        let Ok(stream) = self.files.open_read(path) else {
            let legacy = Self::legacy_file_name(slot);
            return Err(if self.files.open_read(Path::new(&legacy)).is_ok() {
                SlotError::LegacyFormatUnsupported { slot }
            } else {
                SlotError::Missing { slot }
            });
        };
        let len = stream.len().map_err(|_| SlotError::OpenFailed { slot })?;
        Ok((stream, len))
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.files.stat(path).is_ok_and(|stat| stat.is_dir)
    }

    pub(super) fn open_legacy(&self, slot: u16) -> std::io::Result<FileStream> {
        let name = Self::legacy_file_name(usize::from(slot));
        let path = Path::new(&name);
        if self.is_directory(path) {
            return Err(std::io::ErrorKind::IsADirectory.into());
        }
        self.files.open_read(path)
    }
}

fn check_slot(slot: usize) -> Result<(), SlotError> {
    if slot < MAX_SAVED_GAMES {
        Ok(())
    } else {
        Err(SlotError::OutOfRange { slot })
    }
}
