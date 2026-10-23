//! Campaign persistence for modern UMS and legacy compressed saves.
//!
//! The menu ABI captures and restores live runtime sections; the codecs own
//! the wire layouts, and slots owns resource-file storage.

pub mod abi;
pub mod decode;
pub mod encode;
pub mod error;
pub mod format;
pub mod legacy;
#[cfg(not(test))]
mod live;
pub mod lzh;
#[cfg(not(test))]
mod problem;
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

#[cfg(test)]
mod legacy_tests;
