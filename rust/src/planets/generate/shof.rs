//! `genshof.c`: the Shofixti system. While the Shofixti have not been
//! recruited, a single Shofixti scout is stored as the system's group: Tanaka
//! until he dies, then his brother Katana.
//!
//! Overridden: `initNpcs`, `reinitNpcs`, `uninitNpcs`, `generatePlanets`.
//! Story flags: `SHOFIXTI_RECRUITED`, `SHOFIXTI_KIA`, `SHOFIXTI_BRO_KIA`,
//! `MAIDENS_ON_SHIP`, `SHOFIXTI_VISITS`, `SHOFIXTI_GRPOFFS0` (32-bit group
//! reference).

use crate::c_bindings::global_state::PLANET_DESC;
use crate::comm::dispatch::ship::SHOFIXTI_SHIP;
use crate::math::RandomContext;
use crate::planets::generate::default::{self, FLEE, IGNORE_FLAGSHIP, REFORM_GROUP};
use crate::planets::generate::special::{FleetGroups, Story, SELENIC_WORLD};
use crate::planets::orbits::{self, OrbitRequest};
use crate::ships::queue::{NAME_OFFSET, NUM_CAPTAINS_NAMES};

/// `METAL_WORLD` (`plandata.h`).
const METAL_WORLD: u8 = 6;
/// `NUM_PLANETS`: the system always has six worlds.
const NUM_PLANETS: u8 = 6;

/// What the Shofixti group needs of the host beyond the shared fleet calls.
pub trait ShofixtiSite: FleetGroups {
    /// `CloneShipFragment (SHOFIXTI_SHIP, &GLOBAL (npc_built_ship_q), 1)` and,
    /// when a fragment was made, `captains_name_index = name_index`.
    fn clone_shofixti_named(&mut self, name_index: u8);
    /// `PutGroupInfo (offset, 1)`.
    fn put_group_info(&mut self, offset: u32) -> u32;
}

/// `GenerateShofixti_initNpcs`: while a Shofixti is still to be found (not
/// recruited, and either alive or the brother is alive and the maidens are
/// aboard) the stored group is resumed or, failing that, made anew.
pub fn init_npcs(story: &mut impl Story, host: &mut impl ShofixtiSite) -> bool {
    let a_shofixti_remains = story.flag("SHOFIXTI_RECRUITED") == 0
        && (story.flag("SHOFIXTI_KIA") == 0
            || (story.flag("SHOFIXTI_BRO_KIA") == 0 && story.flag("MAIDENS_ON_SHIP") != 0));
    if a_shofixti_remains {
        host.set_battle_group_ref(story.word("SHOFIXTI_GRPOFFS0"));
        if host.battle_group_ref() == 0 || !host.init_ip_groups() {
            if host.battle_group_ref() == 0 {
                host.set_battle_group_ref(!0);
            }
            // Tanaka, or Katana (+1) once Tanaka is dead.
            let name = NAME_OFFSET + NUM_CAPTAINS_NAMES + (story.flag("SHOFIXTI_KIA") & 1) as u8;
            host.clone_shofixti_named(name);
            let stored = host.put_group_info(host.battle_group_ref());
            host.set_battle_group_ref(stored);
            host.reinit_built_queue();
            story.set_word("SHOFIXTI_GRPOFFS0", stored);
        }
    }

    // This was originally a fallthrough to REINIT_NPCS.
    default::init_npcs(host);
    check_old_shofixti(story, host);
    true
}

/// `GenerateShofixti_reinitNpcs`.
pub fn reinit_npcs(story: &impl Story, host: &mut impl FleetGroups) -> bool {
    default::reinit_npcs(host);
    check_old_shofixti(story, host);
    true
}

/// `GenerateShofixti_uninitNpcs`: a Shofixti group that is gone from the
/// system when the player leaves was killed; the brother is the second one.
pub fn uninit_npcs(story: &mut impl Story, host: &mut impl FleetGroups) -> bool {
    if host.battle_group_ref() != 0
        && story.flag("SHOFIXTI_RECRUITED") == 0
        && host.head_ip_group().is_none()
    {
        if story.flag("SHOFIXTI_KIA") == 0 {
            story.set_flag("SHOFIXTI_KIA", 1);
            story.set_flag("SHOFIXTI_VISITS", 0);
        } else if story.flag("MAIDENS_ON_SHIP") != 0 {
            story.set_flag("SHOFIXTI_BRO_KIA", 1);
        }
    }

    default::uninit_npcs(host);
    true
}

/// `GenerateShofixti_generatePlanets`: three selenic and three metal worlds.
pub fn generate_planets(
    rng: &mut RandomContext,
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    for (index, planet) in planets
        .iter_mut()
        .take(usize::from(NUM_PLANETS))
        .enumerate()
    {
        planet.NumPlanets = 0;
        planet.data_index = if index < usize::from(NUM_PLANETS >> 1) {
            SELENIC_WORLD
        } else {
            METAL_WORLD
        };
    }
    let request = OrbitRequest {
        star_size,
        star_color,
        num_planets: NUM_PLANETS,
        generating_moons: false,
        types_defined: true,
    };
    let drawn = orbits::fill_orbits(rng, request, planets);
    assert!(drawn.is_none(), "FillOrbits draws no count for a fixed one");
    NUM_PLANETS
}

/// `check_old_shofixti`: a Shofixti the flagship collided with flees once he
/// has been recruited.
fn check_old_shofixti(story: &impl Story, host: &mut impl FleetGroups) {
    if host.battle_group_ref() == 0 {
        return;
    }
    let recruited = story.flag("SHOFIXTI_RECRUITED") != 0;
    if let Some(group) = host.head_ip_group() {
        if group.race_id == SHOFIXTI_SHIP && group.task & REFORM_GROUP != 0 && recruited {
            group.task = FLEE | IGNORE_FLAGSHIP | REFORM_GROUP;
            group.dest_loc = 0;
        }
    }
}

#[cfg(test)]
mod tests;
