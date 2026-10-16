//! Save slot storage: `uqmsave.NN` files in the save directory.
//!
//! Files are opened, read, written and removed through the same resource
//! file calls the C game used (`res_OpenResFile`, `ReadResFile`,
//! `WriteResFile`, `DeleteResFile`) against the uio save directory, so a
//! virtual save directory works exactly as it does for the rest of the game.

use std::ffi::{c_void, CString};
use std::io;

use super::decode::{decode_save_from, decode_summary_from};
use super::encode::encode_save;
use super::error::SlotError;
use super::types::{SaveFile, Summary, MAX_SAVED_GAMES};
use crate::resource::ffi_bridge::{
    res_CloseResFile, res_OpenResFile, DeleteResFile, LengthResFile, ReadResFile, WriteResFile,
    STREAM_SENTINEL,
};

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
pub struct SaveSlots {
    dir: *mut c_void,
}

impl SaveSlots {
    /// # Safety
    ///
    /// `dir` must be a live uio directory handle (the game's `saveDir`) that
    /// stays open for as long as this value is used.
    pub unsafe fn new(dir: *mut c_void) -> Result<Self, SlotError> {
        if dir.is_null() {
            return Err(SlotError::NoSaveDir);
        }
        Ok(SaveSlots { dir })
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

    /// Write a save the way `SaveGame` does: the whole file at once, and the
    /// slot file removed again if any write or the close fails. A failure to
    /// open leaves whatever was there untouched.
    pub fn write(&self, slot: usize, save: &SaveFile) -> Result<(), SlotError> {
        let name = Self::file_name(slot)?;
        let bytes = encode_save(save).map_err(|error| SlotError::Encode { slot, error })?;

        let mut stream = self
            .open(&name, c"wb")
            .ok_or(SlotError::OpenFailed { slot })?;
        if stream.is_directory() {
            return Err(SlotError::IsDirectory { slot });
        }
        let written = stream.write_all(&bytes);
        let closed = stream.close();
        self.settle_write(slot, written, closed)
    }

    /// The outcome rule of `SaveGame`: a failed write leaves no slot file
    /// behind. Only the slot's own file is ever removed.
    pub(super) fn settle_write(
        &self,
        slot: usize,
        written: bool,
        closed: bool,
    ) -> Result<(), SlotError> {
        if written && closed {
            return Ok(());
        }
        let removed = self.remove(&Self::file_name(slot)?);
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

    fn open_for_read(&self, slot: usize) -> Result<(Stream, u64), SlotError> {
        let name = Self::file_name(slot)?;
        let Some(stream) = self.open(&name, c"rb") else {
            // The C loader would try `starcon2.NN` here. There is no legacy
            // decoder in this crate, so say so instead of reporting a slot
            // that holds a game as empty.
            let legacy = Self::legacy_file_name(slot);
            return Err(match self.open(&legacy, c"rb") {
                Some(_) => SlotError::LegacyFormatUnsupported { slot },
                None => SlotError::Missing { slot },
            });
        };
        if stream.is_directory() {
            return Err(SlotError::IsDirectory { slot });
        }
        let len = stream.len();
        Ok((stream, len))
    }

    fn open(&self, name: &str, mode: &std::ffi::CStr) -> Option<Stream> {
        let name = CString::new(name).expect("slot file names contain no NUL");
        // SAFETY: `dir` is live per the contract of `new`; both strings are
        // NUL-terminated and outlive the call.
        let fp = unsafe { res_OpenResFile(self.dir, name.as_ptr(), mode.as_ptr()) };
        if fp.is_null() {
            None
        } else {
            Some(Stream { fp, closed: false })
        }
    }

    fn remove(&self, name: &str) -> bool {
        let name = CString::new(name).expect("slot file names contain no NUL");
        // SAFETY: as in `open`.
        unsafe { DeleteResFile(self.dir, name.as_ptr()) == 1 }
    }
}

fn check_slot(slot: usize) -> Result<(), SlotError> {
    if slot < MAX_SAVED_GAMES {
        Ok(())
    } else {
        Err(SlotError::OutOfRange { slot })
    }
}

/// An open resource file, closed when dropped.
struct Stream {
    fp: *mut c_void,
    closed: bool,
}

impl Stream {
    fn is_directory(&self) -> bool {
        self.fp == STREAM_SENTINEL
    }

    fn len(&self) -> u64 {
        // SAFETY: `fp` is an open, non-directory stream.
        unsafe { LengthResFile(self.fp) as u64 }
    }

    fn write_all(&mut self, bytes: &[u8]) -> bool {
        // SAFETY: `fp` is an open stream and `bytes` is a valid slice.
        let n = unsafe { WriteResFile(bytes.as_ptr().cast(), 1, bytes.len(), self.fp) };
        n == bytes.len()
    }

    fn close(&mut self) -> bool {
        self.closed = true;
        if self.is_directory() {
            return true;
        }
        // SAFETY: `fp` is an open stream and is closed once.
        unsafe { res_CloseResFile(self.fp) == 1 }
    }
}

impl io::Read for &mut Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        // SAFETY: `fp` is an open, non-directory stream and `buf` is valid.
        Ok(unsafe { ReadResFile(buf.as_mut_ptr().cast(), 1, buf.len(), self.fp) })
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if !self.closed {
            self.close();
        }
    }
}
