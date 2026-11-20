//! Bounded UTF-8 editing with borrowed synchronous callbacks.
mod editor;

pub use editor::{edit_text, Alphabet, Editor, TextEntryCallbacks, TextEntryView};
