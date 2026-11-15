//! Owned page snapshots copied while the track list is locked.

use super::{SoundChunk, TRACK_STATE};

/// Identity of a cached subtitle page, independent of its displayed bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubtitleIdentity(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleSnapshot {
    pub identity: SubtitleIdentity,
    pub text: std::ffi::CString,
}

impl SoundChunk {
    fn subtitle_snapshot(&mut self) -> Option<SubtitleSnapshot> {
        self.ensure_text_cstr();
        self.text_cstr.as_ref().map(|text| SubtitleSnapshot {
            identity: SubtitleIdentity(text.as_ptr() as usize),
            text: text.clone(),
        })
    }
}

/// Copy the current page without lending a pointer beyond the track lock.
pub fn subtitle_snapshot() -> Option<SubtitleSnapshot> {
    let state = TRACK_STATE.lock();
    let current = state.cur_sub_chunk?;
    // SAFETY: the current chunk belongs to the list protected by the held track lock.
    unsafe { &mut *current.as_ptr() }.subtitle_snapshot()
}

/// Copy history in the native traversal order: the head, then tagged successor pages.
pub fn subtitle_history() -> Vec<SubtitleSnapshot> {
    let mut state = TRACK_STATE.lock();
    let mut next = state.chunks_head.as_deref_mut();
    let mut first = true;
    let mut history = Vec::new();
    while let Some(chunk) = next {
        if first || chunk.tag_me {
            if let Some(subtitle) = chunk.subtitle_snapshot() {
                history.push(subtitle);
            }
        }
        first = false;
        next = chunk.next.as_deref_mut();
    }
    history
}
