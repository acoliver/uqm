//! `genmel.c`: the nine Melnorme trading posts. Each keeps its own fixed
//! group; everything but `initNpcs` is the default.
//!
//! Overridden: `initNpcs`. Story flags: `MELNORME0_GRPOFFS0` to
//! `MELNORME8_GRPOFFS0` (32-bit group references), picked by the star's index.

use crate::planets::generate::special::{self, FleetGroups, Story, MELNORME_SHIP};

/// `MELNORME0_DEFINED` (`gendef.h`): the star indexes of the nine posts are
/// consecutive from here.
pub const MELNORME0_DEFINED: u8 = 7;

const REF_VARS: [&str; 9] = [
    "MELNORME0_GRPOFFS0",
    "MELNORME1_GRPOFFS0",
    "MELNORME2_GRPOFFS0",
    "MELNORME3_GRPOFFS0",
    "MELNORME4_GRPOFFS0",
    "MELNORME5_GRPOFFS0",
    "MELNORME6_GRPOFFS0",
    "MELNORME7_GRPOFFS0",
    "MELNORME8_GRPOFFS0",
];

/// `SelectMelnormeRefVar`: the group reference of the post at `star_index`.
pub fn ref_var(star_index: u8) -> Option<&'static str> {
    let post = usize::from(star_index.checked_sub(MELNORME0_DEFINED)?);
    REF_VARS.get(post).copied()
}

/// `GenerateMelnorme_initNpcs`. A star that is no Melnorme post has no
/// reference to keep, as in C (which logs a warning and reads 0).
pub fn init_npcs(star_index: u8, story: &mut impl Story, host: &mut impl FleetGroups) -> bool {
    match ref_var(star_index) {
        Some(var) => special::ensure_fleet(story, host, var, MELNORME_SHIP),
        None => {
            host.set_battle_group_ref(0);
            host.clone_ship_into_built_queue(MELNORME_SHIP);
            let group = host.add_new_group();
            host.set_battle_group_ref(group);
            host.reinit_built_queue();
        }
    }
    special::default_init(host);
    true
}
