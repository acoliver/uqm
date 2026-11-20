//! Input-free planet rendering, using caller-retained native graphics.

pub mod c_api;
pub mod oval;
pub mod sphere_geometry;
pub mod topo_gen;
pub mod topo_scale;
pub mod topography;

pub(crate) fn draw_star_background(
    context: &uqm_engine::graphics::context::ContextLease,
    background: &uqm_engine::graphics::drawable::ContextFrame,
) {
    context.draw_stamp(
        background,
        uqm_engine::graphics::geometry::Point { x: 0, y: 0 },
    );
}

#[cfg(test)]
mod tests {
    #[test]
    #[serial_test::serial]
    fn background_draw_keeps_the_supplied_frame_alive_after_its_owner_drops() {
        use uqm_engine::graphics::{
            context::ContextLease, drawable::ContextFrame, drawable::LoadedDrawable,
        };
        let mut game = uqm_engine::Game::new();
        let display =
            uqm_engine::test_support::campaign_display::CampaignDisplay::install(&mut game)
                .unwrap();
        let context = ContextLease::require(game.kernel.off_screen_context);
        context.set_frame(ContextFrame::retain(game.kernel.screen));
        let owner = LoadedDrawable::create(2, 320, 240, 1).unwrap();
        let background = owner.lease().context_frame(0);
        drop(owner);
        super::draw_star_background(&context, &background);
        assert_eq!(background.rect().extent.width, 320);
        assert_eq!(background.rect().extent.height, 240);
        drop(background);
        drop(context);
        display.finish(game);
    }
}
