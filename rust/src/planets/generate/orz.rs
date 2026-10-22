//! `genorz.c`: the two Orz systems. Which one a hook runs for is
//! `CurStarDescPtr->Index`: the Orz home (a water world with ruins, defended
//! by the Orz fleet) or the Taalo system, where the Taalo protector device
//! lies on moon 2 of planet 1, behind an Orz squadron until they are beaten.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: `TAALO_PROTECTOR`, `TAALO_PROTECTOR_ON_SHIP`,
//! `TAALO_UNPROTECTED`.

use crate::battle::gendef::{ORZ_DEFINED, TAALO_PROTECTOR_DEFINED};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{self, Body, Orbit, Story, WATER_WORLD};

const WORLD: Body = Body::Planet(0);
/// `matchWorld (solarSys, world, 1, 2)`: the moon the device lies on.
const DEVICE_MOON: Body = Body::Moon { planet: 1, moon: 2 };

/// `GLOBAL_FLAGS_AND_DATA` during the talk at the Orz home (`1 << 7`) and at
/// Taalo (`1 << 6`).
const ORZ_TALK_FLAGS: u8 = 1 << 7;
const TAALO_TALK_FLAGS: u8 = 1 << 6;

/// The Orz squadron that guards Taalo: fourteen ships of fleet size 0.
const TAALO_ESCORT_SHIPS: u32 = 14;

/// What `generateOrbital` asks of the host besides the contact calls shared
/// with the other race homes (here the race is `ORZ_SHIP`).
pub trait OrzSite: HomeContactSite {
    /// `CloneShipFragment (ORZ_SHIP, &GLOBAL (npc_built_ship_q), 0)`.
    fn clone_escort_into_built_queue(&mut self);
    /// `RepairSISBorder ()`.
    fn repair_sis_border(&mut self);
    /// `LoadStdLanderFont`, the Taalo device mask and report strings.
    fn load_taalo_device_scene(&mut self);
}

/// `GenerateOrz_generatePlanets` for the system at `star_index`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
    star_index: u8,
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        if star_index == ORZ_DEFINED {
            let world = &mut planets[0];
            world.data_index = WATER_WORLD;
            world.NumPlanets = 0;
            special::reposition(world, 156);
        }
    })
}

/// `GenerateOrz_generateOrbital`: arriving at the Orz home, or at the device
/// while it lies untaken, meets the Orz when they still guard it; unless the
/// conversation was aborted, loaded over, or left Orz survivors the visit
/// goes on and Taalo is unprotected for good.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    star_index: u8,
    story: &mut impl Story,
    site: &mut impl OrzSite,
    load_planet: impl FnOnce(),
) -> bool {
    let at_home = star_index == ORZ_DEFINED && body == WORLD;
    let at_device = star_index == TAALO_PROTECTOR_DEFINED
        && body == DEVICE_MOON
        && story.flag("TAALO_PROTECTOR") == 0;
    if at_home || at_device {
        let guarded = star_index == ORZ_DEFINED || story.flag("TAALO_UNPROTECTED") == 0;
        if guarded && site.start_sphere_tracking() && !meet_orz(star_index, story, site) {
            return true;
        }

        story.set_flag("TAALO_UNPROTECTED", 1);
        if star_index == TAALO_PROTECTOR_DEFINED {
            site.load_taalo_device_scene();
        } else {
            site.load_ruins_scene(false);
        }
    }
    orbit.default_orbital(load_planet);
    true
}

/// The conversation with the Orz: false when the visit is over, either
/// because the conversation aborted or loaded a game or because Orz ships
/// survived it.
fn meet_orz(star_index: u8, story: &impl Story, site: &mut impl OrzSite) -> bool {
    site.notify_others_all_clear();
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the Orz are cloned into it"
    );

    if star_index == ORZ_DEFINED {
        site.clone_fleet_into_built_queue();
        site.set_global_flags_and_data(ORZ_TALK_FLAGS);
    } else {
        for _ in 0..TAALO_ESCORT_SHIPS {
            site.clone_escort_into_built_queue();
        }
        site.set_global_flags_and_data(TAALO_TALK_FLAGS);
    }
    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.talk();

    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) != 0 {
        return false;
    }

    let survivors = site.built_ship_count() > 0
        && (star_index == ORZ_DEFINED || story.flag("TAALO_UNPROTECTED") == 0);
    site.set_current_activity(activity & !START_INTERPLANETARY);
    site.reinit_built_ship_q();
    site.load_ip_groups();
    if survivors {
        return false;
    }
    site.repair_sis_border();
    true
}

/// `GenerateOrz_generateEnergy`: the device while it lies untaken, ruins at
/// the Orz home.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    star_index: u8,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
    story: &impl Story,
) -> u16 {
    if star_index == TAALO_PROTECTOR_DEFINED && body == DEVICE_MOON {
        if story.flag("TAALO_PROTECTOR") != 0 {
            return 0;
        }
        return default::generate_artifact(rng, sys_info, which_node, info);
    }
    if star_index == ORZ_DEFINED && body == WORLD {
        return default::generate_ruins(rng, sys_info, which_node, info);
    }
    0
}

/// `GenerateOrz_pickupEnergy`: the device is taken once, from node 0; the
/// home ruins give the standard cycled report.
pub fn pickup_energy(
    star_index: u8,
    body: Body,
    which_node: u16,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if star_index == TAALO_PROTECTOR_DEFINED && body == DEVICE_MOON {
        assert!(
            story.flag("TAALO_PROTECTOR") == 0 && which_node == 0,
            "the device is picked up once, from node 0"
        );
        default::lander_report(report);
        lander.lander_takeoff();
        story.set_flag("TAALO_PROTECTOR", 1);
        story.set_flag("TAALO_PROTECTOR_ON_SHIP", 1);
        return true;
    }
    if star_index == ORZ_DEFINED && body == WORLD {
        default::lander_report_cycle(report);
        return false;
    }
    false
}

#[cfg(test)]
mod tests;
