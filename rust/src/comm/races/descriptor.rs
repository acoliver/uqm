//! Immutable definitions used by conversation content.

pub(crate) use crate::graphics::native_abi::{point as Point, Color};
use std::ffi::CStr;

pub const MAX_ANIMATIONS: usize = 20;

#[derive(Debug, Clone, Copy, Default)]
pub struct AnimationDescriptor {
    pub start_index: u16,
    pub num_frames: u8,
    pub anim_flags: u8,
    pub base_frame_rate: u16,
    pub random_frame_rate: u16,
    pub base_restart_rate: u16,
    pub random_restart_rate: u16,
    pub block_mask: u32,
}

#[derive(Clone, Default)]
pub struct CommDescriptor {
    pub alien_frame_res: Option<&'static CStr>,
    pub alien_font_res: Option<&'static CStr>,
    pub alien_colormap_res: Option<&'static CStr>,
    pub alien_song_res: Option<&'static CStr>,
    pub alien_alt_song_res: Option<&'static CStr>,
    pub conversation_phrases_res: Option<&'static CStr>,
    pub alien_text_fcolor: Color,
    pub alien_text_bcolor: Color,
    pub alien_text_baseline: Point,
    pub alien_text_width: u16,
    pub alien_text_align: u32,
    pub alien_text_valign: u32,
    pub alien_song_flags: u32,
    pub num_animations: u16,
    pub alien_ambient_array: [AnimationDescriptor; MAX_ANIMATIONS],
    pub alien_transition_desc: AnimationDescriptor,
    pub alien_talk_desc: AnimationDescriptor,
    pub alien_number_speech: Option<&'static NumberSpeech>,
}

#[derive(Debug, Clone, Copy)]
pub struct DigitName {
    pub divider: i32,
    pub max_remainder: i32,
    pub phrase: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct SpeechDigit {
    pub divider: i32,
    pub subtrahend: i32,
    pub strings: Option<&'static [u16; 10]>,
    pub names: &'static [DigitName],
    pub common: u16,
}

impl SpeechDigit {
    pub const fn new(
        divider: i32,
        subtrahend: i32,
        strings: Option<&'static [u16; 10]>,
        common: u16,
    ) -> Self {
        Self {
            divider,
            subtrahend,
            strings,
            names: &[],
            common,
        }
    }
}

#[derive(Debug)]
pub struct NumberSpeech {
    pub digits: &'static [SpeechDigit],
}

#[cfg(test)]
mod tests {
    #[test]
    fn digit_constructor_preserves_rules_and_optional_strings() {
        const STRINGS: [u16; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let digit = super::SpeechDigit::new(100, 1, Some(&STRINGS), 11);
        assert_eq!(
            (digit.divider, digit.subtrahend, digit.common),
            (100, 1, 11)
        );
        assert_eq!(digit.strings, Some(&STRINGS));
        assert!(digit.names.is_empty());

        let digit = super::SpeechDigit::new(10, 0, None, 0);
        assert_eq!((digit.divider, digit.subtrahend, digit.common), (10, 0, 0));
        assert_eq!(digit.strings, None);
        assert!(digit.names.is_empty());
    }

    #[test]
    fn content_factory_is_shareable_and_retains_dormant_animation_slots() {
        fn assert_send_sync<T: Send + Sync>(_: &T) {}
        let mut first: super::CommDescriptor = super::super::orz::descriptor::descriptor();
        let second = super::super::orz::descriptor::descriptor();
        assert_send_sync(&first);
        assert_eq!(first.alien_frame_res, Some(c"comm.orz.graphics"));
        assert_eq!(first.conversation_phrases_res, Some(c"comm.orz.dialogue"));
        assert_eq!(first.num_animations, 12);
        assert_eq!(first.alien_ambient_array[12].start_index, 118);
        assert_eq!(first.alien_ambient_array[12].block_mask, 1024);
        first.alien_ambient_array[12].block_mask = 0;
        assert_eq!(first.alien_ambient_array[12].block_mask, 0);
        assert_eq!(second.alien_ambient_array[12].block_mask, 1024);
        assert_eq!(
            (
                second.alien_talk_desc.start_index,
                second.alien_talk_desc.num_frames
            ),
            (1, 3)
        );
    }
}
