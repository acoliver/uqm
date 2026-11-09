//! Campaign-owned rendering resources borrowed for a synchronous conversation.

use crate::engine::Kernel;
use crate::graphics::native_abi::{CONTEXT, FONT, FRAME, STRING};

#[derive(Default)]
pub(crate) struct Presentation {
    pub space_context: CONTEXT,
    pub off_screen_context: CONTEXT,
    pub radar_context: CONTEXT,
    pub screen: FRAME,
    pub activity_frame: FRAME,
    pub tiny_font: FONT,
    pub game_strings: STRING,
    pub using_speech: u32,
}

impl From<&Kernel> for Presentation {
    fn from(kernel: &Kernel) -> Self {
        Self {
            space_context: kernel.space_context,
            off_screen_context: kernel.off_screen_context,
            radar_context: kernel.radar_context,
            screen: kernel.screen,
            activity_frame: kernel.activity_frame,
            tiny_font: kernel.tiny_font,
            game_strings: kernel.game_strings,
            using_speech: kernel.using_speech,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_borrows_the_supplied_kernels_resources() {
        let mut kernel = Kernel::new();
        kernel.using_speech = 1;
        let presentation = Presentation::from(&kernel);
        assert_eq!(presentation.space_context, kernel.space_context);
        assert_eq!(presentation.off_screen_context, kernel.off_screen_context);
        assert_eq!(presentation.radar_context, kernel.radar_context);
        assert_eq!(presentation.screen, kernel.screen);
        assert_eq!(presentation.activity_frame, kernel.activity_frame);
        assert_eq!(presentation.tiny_font, kernel.tiny_font);
        assert_eq!(presentation.game_strings, kernel.game_strings);
        assert_eq!(presentation.using_speech, 1);
    }
}
