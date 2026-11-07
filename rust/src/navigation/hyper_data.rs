//! Hyperspace graphics resources and entry/exit lifecycle.
use super::{
    abi::{CHECK_LOAD, POINT},
    color,
};
use crate::engine::{
    battle::{hyperspace as resources, Cx},
    StoryKey,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryDisplay {
    RepairBorder,
    DrawFrame,
    DrawGauges,
    ClearMessage,
    Palette(bool),
    ClearRadar,
}

pub use crate::engine::battle::hyperspace::free_hyper_data;

pub fn load_hyperspace(cx: &mut Cx<'_>, mut display: impl FnMut(&mut Cx<'_>, EntryDisplay)) {
    cx.world.hyper_distance = crate::engine::world::FuelDistance {
        dx: 0,
        dy: 0,
        extra: 0,
        ticks: 1,
    };
    cx.world.data.game.ship_stamp.origin = POINT { x: -9999, y: -9999 };
    resources::load(cx.kernel);
    resources::free_hyperspace(cx.kernel);
    if cx.kernel.last_activity & CHECK_LOAD as u16 == 0 {
        display(cx, EntryDisplay::RepairBorder);
    } else if cx.kernel.last_activity as u8 == 0 {
        display(cx, EntryDisplay::DrawFrame);
    } else {
        display(cx, EntryDisplay::DrawGauges);
        display(cx, EntryDisplay::RepairBorder);
    }
    if cx.world.data.game.autopilot.x == -1 || cx.world.data.game.autopilot.y == -1 {
        display(cx, EntryDisplay::ClearMessage);
    }
    let radar = resources::radar_context(cx.kernel);
    radar.select();
    radar.set_background(color(0, 14, 0));
    let space = resources::space_context(cx.kernel);
    space.select();
    let quasispace = cx.world.story_state(StoryKey::ARILOU_SPACE_SIDE) > 1;
    space.set_background(if quasispace {
        color(0, 26, 0)
    } else {
        color(7, 0, 0)
    });
    display(cx, EntryDisplay::Palette(quasispace));
    if quasispace {
        cx.world.set_story_state(StoryKey::USED_BROADCASTER, 0);
        cx.world.set_story_state(StoryKey::BROADCASTER_RESPONSE, 0);
    }
    display(cx, EntryDisplay::ClearRadar);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_resource_lifecycle_needs_no_published_game() {
        let mut game = crate::engine::Game::new();
        free_hyper_data(&mut game.kernel);
        resources::free_hyperspace(&mut game.kernel);
        assert!(game.kernel.hyperstars.iter().all(|frame| frame.is_null()));
        assert!(game.kernel.stars_in_space.is_null());
        assert!(game.kernel.cmaps.iter().all(|map| map.is_null()));
    }
}
