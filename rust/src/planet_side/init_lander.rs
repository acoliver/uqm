//! Lander status drawing from supplied campaign or save-preview values.

use super::resources::{LanderGraphic, PlanetSideAssets};
use crate::{
    engine::{StoryKey, World},
    graphics::{
        context_abi::{ContextFrame, ContextLease},
        native_abi::{Color, EXTENT, POINT, RECT},
    },
};

const MAX_SCROUNGED: u16 = 50;
const RADAR_WIDTH: i16 = 56;
const RADAR_HEIGHT: i16 = 53;
const BLACK_COLOR: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

#[derive(Clone, Copy)]
pub struct LanderStatus {
    pub landers: u8,
    pub shield_flags: u8,
    pub capacity_shift: u8,
    pub free_space: u16,
    pub override_flags: u8,
}

impl LanderStatus {
    pub fn campaign(world: &World) -> Self {
        Self {
            landers: world.data.flagship.num_landers,
            shield_flags: world.story_state(StoryKey::LANDER_SHIELDS) as u8,
            capacity_shift: world.story_state(StoryKey::IMPROVED_LANDER_CARGO) as u8,
            free_space: free_space(&world.data.flagship),
            override_flags: 0,
        }
    }

    pub fn preview(ship: &crate::engine::world::SisState, flags: u8) -> Self {
        Self {
            landers: ship.num_landers,
            shield_flags: flags & 15,
            capacity_shift: u8::from(flags & (1 << 5) != 0),
            free_space: free_space(ship),
            override_flags: flags | 0x80,
        }
    }
}

fn free_space(ship: &crate::engine::world::SisState) -> u16 {
    let capacity = crate::ui::sis::capacity::storage_capacity(ship);
    capacity.wrapping_sub(ship.total_element_mass)
}

pub fn draw_lander(context: &ContextLease, assets: &PlanetSideAssets, status: LanderStatus) {
    context.draw_batch(|| {
        context.fill_rectangle(
            BLACK_COLOR,
            RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: RADAR_WIDTH,
                    height: RADAR_HEIGHT,
                },
            },
        );
        if status.landers != 0 || status.override_flags != 0 {
            draw_lander_status(context, &assets.graphic(LanderGraphic::Lander), status);
        }
    });
}

fn draw_lander_status(context: &ContextLease, lander: &ContextFrame, status: LanderStatus) {
    let draw = |index| context.draw_stamp(&lander.wrapped_frame(index), POINT { x: 0, y: 0 });
    draw(32);
    let flags = status.override_flags;
    if flags != 0 {
        draw(33);
        if flags & (1 << 4) != 0 {
            draw(57);
        } else {
            draw(35);
            draw(36);
        }
        if flags & (1 << 5) != 0 {
            draw(59);
        }
        draw(if flags & (1 << 6) != 0 { 58 } else { 34 });
    }
    let max_scrounged = MAX_SCROUNGED
        .checked_shl(u32::from(status.capacity_shift))
        .unwrap_or(MAX_SCROUNGED);
    if status.free_space < max_scrounged {
        let height = max_scrounged
            .saturating_sub(status.free_space >> status.capacity_shift)
            .saturating_add(1);
        context.fill_rectangle(
            BLACK_COLOR,
            RECT {
                corner: POINT { x: 1, y: 0 },
                extent: EXTENT {
                    width: 4,
                    height: height as i16,
                },
            },
        );
    }
    for (bit, frame) in [(1, 37), (0, 38), (2, 39), (3, 40)] {
        if status.shield_flags & (1 << bit) != 0 {
            draw(frame);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_and_preview_status_use_the_supplied_owners() {
        let mut world = World::new();
        world.story = Some(crate::engine::world::story::StoryState::new());
        world.data.flagship.num_landers = 2;
        world.set_story_state(StoryKey::LANDER_SHIELDS, 5);
        world.set_story_state(StoryKey::IMPROVED_LANDER_CARGO, 1);
        let live = LanderStatus::campaign(&world);
        assert_eq!(
            (
                live.landers,
                live.shield_flags,
                live.capacity_shift,
                live.override_flags
            ),
            (2, 5, 1, 0)
        );
        let mut saved = world.data.flagship;
        saved.num_landers = 0;
        saved.module_slots[0] = crate::starbase::sis_modules::ModuleType::StorageBay as u8;
        saved.total_element_mass = 475;
        let preview = LanderStatus::preview(&saved, 0x52);
        assert_eq!(
            (
                preview.landers,
                preview.shield_flags,
                preview.capacity_shift,
                preview.free_space,
                preview.override_flags
            ),
            (0, 2, 0, 25, 0xd2)
        );
        assert_eq!(world.data.flagship.num_landers, 2);
        assert_eq!(
            (BLACK_COLOR.r, BLACK_COLOR.g, BLACK_COLOR.b, BLACK_COLOR.a),
            (0, 0, 0, 255)
        );
        assert_eq!((RADAR_WIDTH, RADAR_HEIGHT, MAX_SCROUNGED), (56, 53, 50));
    }
}
