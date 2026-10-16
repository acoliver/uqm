//! Typed errors for the save codec and slot storage.

use std::fmt;

/// Where a read ran out of bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// The leading `UMS\x01` word.
    FileMagic,
    /// A chunk's tag or size word.
    ChunkHeader,
    /// The body of the chunk with this tag.
    Chunk(u32),
}

/// A save file that is not a well-formed UMS version 1 file. All of these
/// describe external data; none of them is a programming error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveFormatError {
    /// The file does not start with the UMS version 1 magic. Pre-UMS
    /// (`starcon2.NN`) saves land here; this crate has no legacy decoder.
    BadMagic { found: u32 },
    /// Summ, GlSt and GmSt must be the first three chunks, in that order.
    UnexpectedTag { expected: u32, found: u32 },
    /// The input ended inside a field.
    Truncated { at: Section },
    /// A chunk size the chunk's layout cannot have, or one that does not
    /// match its contents.
    ChunkSize { tag: u32, size: u32 },
    /// More entries than the format can express.
    CountOverflow { tag: u32, count: u64 },
    /// A chunk every save must contain is missing.
    MissingChunk { tag: u32 },
    /// The reader failed for a reason other than running out of bytes.
    Io { kind: std::io::ErrorKind },
}

/// A save that cannot be encoded because a collection exceeds what the
/// format can express.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    CountOverflow { tag: u32, count: u64 },
}

/// A save name the format cannot hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveNameError {
    TooLong { len: usize },
    ContainsNul,
}

/// Failures of slot storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotError {
    /// The save directory handle is null.
    NoSaveDir,
    /// The slot number is not one of the 50 slots.
    OutOfRange { slot: usize },
    /// The slot has no UMS file and no pre-UMS file.
    Missing { slot: usize },
    /// The slot has only a pre-UMS `starcon2.NN` file, which this crate
    /// cannot read.
    LegacyFormatUnsupported { slot: usize },
    /// The slot file name is a directory.
    IsDirectory { slot: usize },
    /// The slot file could not be opened for writing.
    OpenFailed { slot: usize },
    /// A write came up short or the close failed; the partial file was
    /// removed (or removal itself failed, see `removed`).
    WriteFailed { slot: usize, removed: bool },
    /// The file is not a well-formed UMS save.
    Format { slot: usize, error: SaveFormatError },
    /// The save cannot be encoded.
    Encode { slot: usize, error: EncodeError },
}

impl fmt::Display for SaveFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl fmt::Display for SaveNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl fmt::Display for SlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SaveFormatError {}
impl std::error::Error for EncodeError {}
impl std::error::Error for SaveNameError {}
impl std::error::Error for SlotError {}
