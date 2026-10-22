//! `genzfpscout.c`: the Zoq-Fot-Pik scout's system. Until the player has met
//! the Zoq-Fot-Pik, a scout group with a fixed reference patrols it.
//!
//! Overridden: `initNpcs`, `reinitNpcs`. Story flags: `MET_ZOQFOT`,
//! `ZOQFOT_GRPOFFS0` (32-bit group reference).

use crate::planets::generate::default;
use crate::planets::generate::special::{self, FleetGroups, Story, ZOQFOTPIK_SHIP};

/// `FLEE`, `IGNORE_FLAGSHIP` and `REFORM_GROUP` (`grpinfo.h`).
use crate::planets::generate::default::{FLEE, IGNORE_FLAGSHIP, REFORM_GROUP};

/// `GenerateZoqFotPikScout_initNpcs`.
pub fn init_npcs(story: &mut impl Story, host: &mut impl FleetGroups) -> bool {
    if story.flag("MET_ZOQFOT") == 0 {
        special::ensure_fleet(story, host, "ZOQFOT_GRPOFFS0", ZOQFOTPIK_SHIP);
    }
    default::init_npcs(host);
    true
}

/// `GenerateZoqFotPikScout_reinitNpcs`: a scout the flagship collided with
/// flees.
pub fn reinit_npcs(host: &mut impl FleetGroups) -> bool {
    default::reinit_npcs(host);
    if host.battle_group_ref() == 0 {
        return true;
    }
    if let Some(group) = host.head_ip_group() {
        if group.race_id == ZOQFOTPIK_SHIP && group.task & REFORM_GROUP != 0 {
            group.task = FLEE | IGNORE_FLAGSHIP | REFORM_GROUP;
            group.dest_loc = 0;
        }
    }
    true
}
