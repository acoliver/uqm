//! The conversation's player font.
//!
//! `hail_alien` loads `font.player` once and the font must outlive every draw
//! command queued with it: `font_DrawText` queues glyph commands that keep
//! pointers into the font until the draw queue is consumed. Renderers borrow
//! the conversation font through [`ConversationFont::current`]; only the
//! conversation teardown releases it, after the queue has been drained.

use std::ffi::c_void;
use std::fmt;
use std::sync::atomic::{AtomicPtr, Ordering};

/// What the teardown needs from the graphics system to free the font.
pub(crate) trait FontRelease {
    /// Consume every committed draw command.
    fn drain_draw_queue(&mut self);
    fn destroy_font(&mut self, font: *mut c_void);
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConversationFontError {
    /// The `font.player` resource did not load.
    LoadFailed,
    /// A conversation font is already installed and was never released.
    AlreadyInstalled,
}

impl fmt::Display for ConversationFontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed => f.write_str("the font.player resource failed to load"),
            Self::AlreadyInstalled => {
                f.write_str("a conversation player font is already installed")
            }
        }
    }
}

impl std::error::Error for ConversationFontError {}

/// Holder of the font loaded for the running conversation.
pub(crate) struct ConversationFont {
    handle: AtomicPtr<c_void>,
}

/// The running conversation's player font.
#[cfg(not(test))]
pub(crate) static CONVERSATION_PLAYER_FONT: ConversationFont = ConversationFont::new();

impl ConversationFont {
    pub(crate) const fn new() -> Self {
        Self {
            handle: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    /// Takes ownership of the freshly loaded `font`.
    pub(crate) fn install(&self, font: *mut c_void) -> Result<(), ConversationFontError> {
        if font.is_null() {
            return Err(ConversationFontError::LoadFailed);
        }
        self.handle
            .compare_exchange(
                std::ptr::null_mut(),
                font,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| ConversationFontError::AlreadyInstalled)
    }

    /// The font renderers draw with while the conversation runs.
    pub(crate) fn current(&self) -> *mut c_void {
        let font = self.handle.load(Ordering::Acquire);
        assert!(
            !font.is_null(),
            "conversation text drawn outside a conversation: no player font installed"
        );
        font
    }

    /// Frees the font once no queued draw command can still read it.
    pub(crate) fn release<R: FontRelease>(&self, releaser: &mut R) {
        let font = self.handle.swap(std::ptr::null_mut(), Ordering::AcqRel);
        assert!(!font.is_null(), "no conversation player font to release");
        releaser.drain_draw_queue();
        releaser.destroy_font(font);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingRelease {
        events: Vec<String>,
    }

    impl FontRelease for RecordingRelease {
        fn drain_draw_queue(&mut self) {
            self.events.push("drain".into());
        }
        fn destroy_font(&mut self, font: *mut c_void) {
            self.events.push(format!("destroy {}", font as usize));
        }
    }

    fn font(n: usize) -> *mut c_void {
        n as *mut c_void
    }

    #[test]
    fn release_drains_queued_draws_before_destroying_the_font() {
        let slot = ConversationFont::new();
        slot.install(font(7)).unwrap();
        let mut releaser = RecordingRelease::default();

        slot.release(&mut releaser);

        assert_eq!(releaser.events, ["drain", "destroy 7"]);
    }

    #[test]
    fn installed_font_is_the_one_renderers_borrow() {
        let slot = ConversationFont::new();
        slot.install(font(7)).unwrap();

        assert_eq!(slot.current(), font(7));
        assert_eq!(slot.current(), font(7));
    }

    #[test]
    fn a_font_that_failed_to_load_is_rejected() {
        let slot = ConversationFont::new();

        assert_eq!(
            slot.install(std::ptr::null_mut()),
            Err(ConversationFontError::LoadFailed)
        );
    }

    #[test]
    fn a_second_conversation_font_is_rejected_until_the_first_is_released() {
        let slot = ConversationFont::new();
        slot.install(font(7)).unwrap();

        assert_eq!(
            slot.install(font(8)),
            Err(ConversationFontError::AlreadyInstalled)
        );

        slot.release(&mut RecordingRelease::default());
        assert_eq!(slot.install(font(8)), Ok(()));
    }

    #[test]
    #[should_panic(expected = "no player font installed")]
    fn borrowing_without_a_conversation_font_fails_fast() {
        ConversationFont::new().current();
    }

    #[test]
    #[should_panic(expected = "no conversation player font to release")]
    fn releasing_without_a_conversation_font_fails_fast() {
        ConversationFont::new().release(&mut RecordingRelease::default());
    }
}
