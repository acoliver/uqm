//! UQM save byte layouts and codecs, independent of the running game.
#![forbid(unsafe_code)]

pub mod decode;
pub mod encode;
pub mod error;
pub mod format;
pub mod types;

pub use decode::{decode_save, decode_save_from, decode_summary_from};
pub use encode::encode_save;
pub use error::{EncodeError, SaveFormatError, SaveNameError, Section, SlotError};
pub use types::*;
