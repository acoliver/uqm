//! Campaign message labels borrowed from their world, catalog and text owners.
use crate::engine::world::activity::ActivityKind;
use crate::starbase::ids;
use crate::{
    engine::{Kernel, World},
    planets::starmap::naming::cluster_name,
};
use std::ffi::CString;

pub fn default_message_text(kernel: &Kernel, world: &World) -> CString {
    match ActivityKind::from_raw(world.activity() as u8) {
        ActivityKind::InLastBattle | ActivityKind::InInterplanetary => {
            let star = world
                .selected_star()
                .unwrap_or_else(|| panic!("campaign message requires a selected star"));
            cluster_name(kernel, star, ids::STAR_NUMBER_BASE as usize)
        }
        ActivityKind::InHyperspace => kernel
            .game_text(ids::NAVIGATION_STRING_BASE as usize + usize::from(!world.in_hyper_space()))
            .to_owned(),
        _ => c"".to_owned(),
    }
}
