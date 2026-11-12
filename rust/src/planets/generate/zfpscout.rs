//! `genzfpscout.c`: the Zoq-Fot-Pik scout's system. Until the player has met
//! the Zoq-Fot-Pik, a scout group with a fixed reference patrols it.
//!
//! Overridden: `initNpcs`, `reinitNpcs`. Story flags: `MET_ZOQFOT`,
//! `ZOQFOT_GRPOFFS0` (32-bit group reference).

use crate::engine::StoryKey;
use crate::planets::generate::default;
use crate::planets::generate::special::{self, FleetGroups, Story, ZOQFOTPIK_SHIP};

use crate::engine::world::queues::IpGroup as IP_GROUP;

/// `GenerateZoqFotPikScout_initNpcs`.
pub fn init_npcs(host: &mut (impl FleetGroups + Story + ?Sized)) -> bool {
    if host.flag(StoryKey::MET_ZOQFOT) == 0 {
        special::ensure_fleet(host, StoryKey::ZOQFOT_GRPOFFS0, ZOQFOTPIK_SHIP);
    }
    default::init_npcs(host);
    true
}

/// `GenerateZoqFotPikScout_reinitNpcs`: a scout the flagship collided with
/// flees.
pub fn reinit_npcs(host: &mut (impl FleetGroups + ?Sized)) -> bool {
    default::reinit_npcs(host);
    if host.battle_group_ref() == 0 {
        return true;
    }
    host.head_ip_group(&mut |group| {
        if group.race_id == ZOQFOTPIK_SHIP && group.task & IP_GROUP::REFORM_GROUP != 0 {
            group.task = IP_GROUP::FLEE | IP_GROUP::IGNORE_FLAGSHIP | IP_GROUP::REFORM_GROUP;
            group.dest_loc = 0;
        }
    });
    true
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn init_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        init_npcs(host)
    }
    fn reinit_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        reinit_npcs(host)
    }
}
