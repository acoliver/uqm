//! Observation ports specialized to the owned campaign view.

use super::World;
pub use uqm_engine::observe::{
    CheckpointView, DeviceObservation, Injected, InputView, Observer, Presented, Unobserved,
};
pub type Io<'a> = uqm_engine::observe::Io<'a, Option<CampaignView>>;

/// Values copied for one observer call; this view never outlives its input boundary.
#[derive(Clone)]
pub struct CampaignView {
    pub(crate) activity: u16,
    pub(crate) flagship: crate::engine::world::SisState,
    pub(crate) velocity: (i32, i32),
    pub(crate) ship_position: (i16, i16),
    pub(crate) ship_facing: i32,
    pub(crate) autopilot: (i16, i16),
    pub(crate) combat_flags: u8,
    pub(crate) date: (u8, u8, u16),
    pub(crate) story: Option<crate::engine::world::story::GameState>,
    pub(crate) current_star: Option<crate::engine::StarId>,
    pub(crate) contact_race: Option<u8>,
    pub(crate) escorts: Vec<(u8, u16)>,
    pub(crate) groups: Vec<(u8, u8, (i16, i16))>,
}

impl CampaignView {
    pub(crate) fn for_input(view: InputView, capture: impl FnOnce() -> Self) -> Option<Self> {
        matches!(
            view,
            InputView::BeforeUpdate
                | InputView::Nested
                | InputView::PendingSpeech
                | InputView::CommunicationCompleted
        )
        .then(capture)
    }

    pub fn from_world(world: &World) -> Self {
        let game = &world.data.game;
        Self {
            activity: world.activity(),
            flagship: world.data.flagship,
            velocity: game.velocity.get_current_components(),
            ship_position: (game.ship_stamp.origin.x, game.ship_stamp.origin.y),
            ship_facing: 0,
            autopilot: (game.autopilot.x, game.autopilot.y),
            combat_flags: game.glob_flags,
            date: (
                game.game_clock.day_index,
                game.game_clock.month_index,
                game.game_clock.year_index,
            ),
            story: world.story.clone(),
            current_star: world.current_star,
            contact_race: None,
            escorts: Vec::new(),
            groups: Vec::new(),
        }
    }

    pub(crate) fn story_value(&self, key: crate::engine::StoryKey) -> Option<u8> {
        self.story
            .as_ref()
            .map(|story| story.get_state(key.start(), key.end()))
    }

    pub(crate) fn named_state_value(&self, name: &str) -> Option<u8> {
        self.story_value(crate::engine::StoryKey::from_name(name)?)
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::from_world(&World::new())
    }
}
