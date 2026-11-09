//! Playback callbacks mutate only the conversation that queued them.

use crate::comm::Comm;
pub type PhraseCallback = Option<fn(&mut Comm)>;
