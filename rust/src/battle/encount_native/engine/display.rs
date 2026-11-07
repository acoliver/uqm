//! Encounter location text over retained display resources.
#[cfg(test)]
use crate::navigation::globals::in_hqspace;
#[cfg(not(test))]
use crate::ships::native::abi::*;

pub(super) fn draw_title(cx: &mut crate::engine::battle::Cx<'_>) {
    let width = cx.screen_dimensions().0;
    if in_hqspace(cx.world) != 0 {
        crate::ui::sis::draw_hyper_coords(cx.kernel, width, cx.world.data.game.ship_stamp.origin);
    } else if cx.world.data.game.ip_planet == 0 {
        let star = cx
            .world
            .selected_star()
            .unwrap_or_else(|| panic!("encounter title requires a selected star"));
        crate::ui::sis::draw_hyper_coords(cx.kernel, width, star.star_pt);
    } else {
        let name = std::ffi::CStr::from_bytes_until_nul(&cx.world.data.flagship.planet_name)
            .unwrap_or_else(|error| panic!("planet name is not terminated: {error}"));
        crate::ui::sis::draw_title(cx.kernel, width, name);
    }
}

#[cfg(test)]
mod tests {
    use crate::campaign_display_test as campaign_display;
    use crate::engine::world::{CStarDesc, CATALOG_LEN};
    use crate::ui;
    use crate::{
        engine::{
            battle::{Battle, Cx},
            world::StarId,
            Game,
        },
        graphics::{dcqueue_abi::TFB_FlushGraphics, drawable_abi::GetFramePixel, native_abi::*},
        ships::native::abi::{IN_HYPERSPACE, IN_INTERPLANETARY},
    };

    fn title_pixels(game: &Game) -> Vec<(u8, u8, u8)> {
        // SAFETY: each serialized fixture retains its screen and drains its drawing queue before reading.
        unsafe {
            TFB_FlushGraphics();
            (2..9)
                .flat_map(|y| {
                    (193..248).map(move |x| {
                        let pixel = GetFramePixel(game.kernel.screen, POINT { x, y });
                        (pixel.r, pixel.g, pixel.b)
                    })
                })
                .collect()
        }
    }

    #[test]
    #[serial_test::serial]
    fn ui_encounter_title_uses_ship_star_or_planet_from_the_supplied_world() {
        let mut game = Game::new();
        let display = campaign_display::CampaignDisplay::install(&mut game)
            .unwrap_or_else(|error| panic!("campaign panel fixture failed: {error}"));
        let context =
            crate::graphics::context_abi::ContextLease::require(game.kernel.off_screen_context);
        let before = title_pixels(&game);
        super::super::draw_encounter_fleet(&game.world, &context, (320, 240));
        assert_eq!(title_pixels(&game), before);
        drop(context);
        let mut battle = Battle::new();
        let ship = POINT { x: 1111, y: 1111 };
        let star = POINT { x: 2222, y: 2222 };
        let catalog = [CStarDesc {
            star_pt: star,
            star_type: 0,
            index: 0,
            prefix: 0,
            postfix: 0,
        }; CATALOG_LEN];
        game.world.set_activity(IN_HYPERSPACE as u16);
        game.world.data.game.ship_stamp.origin = ship;
        game.world.data.game.ip_planet = 1;
        game.world.data.flagship.planet_name[..2].copy_from_slice(b"A\0");
        super::draw_title(&mut Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            || 0,
        ));
        let ship_pixels = title_pixels(&game);
        ui::sis::draw_hyper_coords(&game.kernel, 320, ship);
        assert_eq!(title_pixels(&game), ship_pixels);
        ui::sis::draw_hyper_coords(&game.kernel, 320, star);
        assert_ne!(title_pixels(&game), ship_pixels);

        game.world.set_activity(IN_INTERPLANETARY as u16);
        game.world.data.game.ip_planet = 0;
        game.world.set_catalog(std::borrow::Cow::Owned(catalog));
        game.world.current_star = Some(StarId(0));
        super::draw_title(&mut Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            || 0,
        ));
        let star_pixels = title_pixels(&game);
        ui::sis::draw_hyper_coords(&game.kernel, 320, star);
        assert_eq!(title_pixels(&game), star_pixels);
        assert_ne!(star_pixels, ship_pixels);

        game.world.data.game.ip_planet = 1;
        game.world.current_star = None;
        super::draw_title(&mut Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            || 0,
        ));
        let planet_pixels = title_pixels(&game);
        ui::sis::draw_title(&game.kernel, 320, c"A");
        assert_eq!(title_pixels(&game), planet_pixels);
        assert_ne!(planet_pixels, star_pixels);
        display.finish(game);
    }
}
