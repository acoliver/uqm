//! Encounter location text over retained display resources.
use crate::ships::native::abi::*;

pub(super) fn draw_title(
    cx: &mut crate::engine::battle::Cx<'_>,
    catalog: Option<&[crate::engine::world::CStarDesc; crate::engine::world::CATALOG_LEN]>,
) {
    let width = cx.screen_dimensions().0;
    if in_hqspace(cx.world) != 0 {
        crate::ui::sis::draw_hyper_coords(cx.kernel, width, cx.world.data.game.ship_stamp.origin);
    } else if cx.world.data.game.ip_planet == 0 {
        let star = cx
            .world
            .selected_star(catalog)
            .unwrap_or_else(|| panic!("encounter title requires a selected star"));
        crate::ui::sis::draw_hyper_coords(cx.kernel, width, star.star_pt);
    } else {
        let name = std::ffi::CStr::from_bytes_until_nul(&cx.world.data.flagship.planet_name)
            .unwrap_or_else(|error| panic!("planet name is not terminated: {error}"));
        crate::ui::sis::draw_title(cx.kernel, width, name);
    }
}
