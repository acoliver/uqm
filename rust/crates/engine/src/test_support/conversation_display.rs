use super::campaign_display::CampaignDisplay;
use crate::graphics::context::LoadedContext;
use crate::world::Game;

pub struct ConversationDisplay {
    _display: CampaignDisplay,
    contexts: Vec<LoadedContext>,
}

impl ConversationDisplay {
    pub fn install(game: &mut Game) -> Result<Self, Box<dyn std::error::Error>> {
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
            let context = LoadedContext::create(name);
            *slot = Some(context.context());
            contexts.push(context);
        }
        Ok(Self {
            _display: display,
            contexts,
        })
    }

    pub fn finish(mut self, game: Game) {
        self._display.finish(game);
        self.contexts.clear();
    }
}
