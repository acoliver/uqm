//! Campaign-owned rendering resources borrowed for a synchronous conversation.

use crate::engine::Kernel;
use crate::graphics::{context::ContextId, drawable::FrameId, font::FontId};

#[derive(Default)]
pub(crate) struct Presentation {
    pub space_context: Option<ContextId>,
    pub off_screen_context: Option<ContextId>,
    pub radar_context: Option<ContextId>,
    pub screen: FrameId,
    pub activity_frame: FrameId,
    pub tiny_font: Option<FontId>,
    pub using_speech: u32,
    pub scope_buffer:
        std::rc::Rc<std::cell::OnceCell<Option<crate::graphics::drawable::LoadedDrawable>>>,
}

impl Presentation {
    pub(crate) fn scope_buffer_frame(
        &self,
        width: i16,
        height: i16,
    ) -> crate::graphics::drawable::ContextFrame {
        self.scope_buffer
            .get_or_init(|| crate::graphics::drawable::LoadedDrawable::create(6, width, height, 1))
            .as_ref()
            .unwrap_or_else(|| panic!("oscilloscope buffer allocation failed"))
            .lease()
            .context_frame(0)
    }
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
            using_speech: kernel.using_speech,
            scope_buffer: std::rc::Rc::clone(&kernel.comm_scope_buffer),
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
        assert_eq!(presentation.using_speech, 1);
    }
}
