use crate::graphics::{
    context_abi::{CreateContextAux, DestroyContext},
    native_abi::CONTEXT,
};
use crate::{campaign_display_test::CampaignDisplay, engine::Game};

pub(super) struct ConversationDisplay {
    _display: CampaignDisplay,
    contexts: Vec<CONTEXT>,
}

impl ConversationDisplay {
    pub(super) fn install(game: &mut Game) -> Result<Self, Box<dyn std::error::Error>> {
        let display = CampaignDisplay::install(game)?;
        let mut contexts = Vec::new();
        for (name, slot) in [
            (
                c"conversation space fixture",
                &mut game.kernel.space_context,
            ),
            (
                c"conversation screen fixture",
                &mut game.kernel.screen_context,
            ),
        ] {
            // SAFETY: the serialized fixture owns admission and releases each context.
            let context = unsafe {
                #[cfg(feature = "debug-process")]
                {
                    CreateContextAux(name.as_ptr())
                }
                #[cfg(not(feature = "debug-process"))]
                {
                    let _ = name;
                    CreateContextAux()
                }
            };
            *slot = context;
            contexts.push(context);
        }
        Ok(Self {
            _display: display,
            contexts,
        })
    }

    pub(super) fn finish(self, game: Game) {
        self._display.finish(game);
        drop(self);
    }
}

impl Drop for ConversationDisplay {
    fn drop(&mut self) {
        // SAFETY: these contexts belong only to this serialized fixture.
        unsafe {
            for context in self.contexts.drain(..) {
                DestroyContext(context);
            }
        }
    }
}
