//! Campaign message labels borrowed from their world, catalog and text owners.
use super::{IN_HYPERSPACE, IN_INTERPLANETARY, IN_LAST_BATTLE, NAVIGATION_STRING_BASE};
use crate::starbase::ids::STAR_NUMBER_BASE;
use crate::{
    engine::{Kernel, World},
    navigation::globals::in_hyper_space,
    planets::starmap::naming::cluster_name,
};
use std::ffi::CString;

pub fn default_message_text(kernel: &Kernel, world: &World) -> CString {
    match u32::from(world.activity() as u8) {
        IN_LAST_BATTLE | IN_INTERPLANETARY => {
            let star = world
                .selected_star()
                .unwrap_or_else(|| panic!("campaign message requires a selected star"));
            cluster_name(kernel, star, STAR_NUMBER_BASE as usize)
        }
        IN_HYPERSPACE => kernel
            .game_text(NAVIGATION_STRING_BASE as usize + usize::from(in_hyper_space(world) == 0))
            .to_owned(),
        _ => c"".to_owned(),
    }
}
