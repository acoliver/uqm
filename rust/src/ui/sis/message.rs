//! Campaign message labels borrowed from their world, catalog and text owners.
use crate::mainloop::types::ActivityKind;
use crate::starbase::ids;
use crate::{
    engine::{Kernel, World},
    navigation::globals::in_hyper_space,
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
            .game_text(
                ids::NAVIGATION_STRING_BASE as usize + usize::from(in_hyper_space(world) == 0),
            )
            .to_owned(),
        _ => c"".to_owned(),
    }
}
