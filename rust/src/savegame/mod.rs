//! UQM Save version 1 ("UMS\x01") persistence.
//!
//! This module owns the on-disk format and the slot files of the Starbase
//! Save/Load menu. [`runtime`] captures and restores the part of a save that
//! lives in `GlobData`, the game-state singleton and the clock's event queue;
//! the other sections need the live queues and state files and are not ported.

pub mod decode;
pub mod encode;
pub mod error;
pub mod format;
pub mod runtime;
pub mod slots;
pub mod types;

#[cfg(test)]
mod format_tests;
#[cfg(test)]
mod slot_tests;

pub use decode::{decode_save, decode_save_from, decode_summary_from};
pub use encode::encode_save;
pub use error::{EncodeError, SaveFormatError, SaveNameError, Section, SlotError};
pub use slots::{SaveSlots, SlotState};
pub use types::*;
