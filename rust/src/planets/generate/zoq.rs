//! `genzoq.c`: the Zoq-Fot-Pik home, a redux world with ruins. With the
//! Zoq-Fot-Pik in distress a Black Ur-Quan ship waits in orbit.
//!
//! Overridden: `initNpcs`, `generatePlanets`, `generateOrbital`,
//! `generateEnergy`, `pickupEnergy`. Story flags: `ZOQFOT_DISTRESS`.

use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default::{self, Npcs};
use crate::planets::generate::home_contact::HomeContactSite;
use crate::planets::generate::special::{self, Body, Orbit, Story, REDUX_WORLD};

const WORLD: Body = Body::Planet(0);

/// `GLOBAL_FLAGS_AND_DATA` during either talk: `1 << 7`.
const TALK_FLAGS: u8 = 1 << 7;

/// What `generateOrbital` asks of the host. The race of the contact calls
/// shared with the other race homes is `ZOQFOTPIK_SHIP`.
pub trait ZoqFotPikSite: HomeContactSite {
    /// `CloneShipFragment (BLACK_URQUAN_SHIP, &GLOBAL (npc_built_ship_q), 0)`.
    fn clone_black_urquan_into_built_queue(&mut self);
    /// `InitCommunication (BLACKURQ_CONVERSATION)`.
    fn talk_black_urquan(&mut self);
}

/// `GenerateZoqFotPik_initNpcs`: nobody is in orbit while the Zoq-Fot-Pik are
/// in distress (value 1).
pub fn init_npcs(story: &impl Story, host: &mut impl Npcs) -> bool {
    if story.flag("ZOQFOT_DISTRESS") != 1 {
        default::init_npcs(host);
    }
    true
}

/// `GenerateZoqFotPik_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        let world = &mut planets[0];
        world.data_index = REDUX_WORLD;
        world.NumPlanets = 1;
        special::reposition(world, 138);
    })
}

/// `GenerateZoqFotPik_generateOrbital`: at planet 0 a tracked sphere of
/// influence starts the visit. In distress the Black Ur-Quan are met first
/// and, if any of them survive, the visit ends there; then the Zoq-Fot-Pik
/// talk. Without tracking the ruins scene is loaded before the default
/// orbital.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &impl Story,
    site: &mut impl ZoqFotPikSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == WORLD {
        if site.start_sphere_tracking() {
            meet(story, site);
            return true;
        }
        site.load_ruins_scene(false);
    }
    orbit.default_orbital(load_planet);
    true
}

fn conversation_ended_the_game(site: &impl ZoqFotPikSite) -> bool {
    site.current_activity() & (CHECK_ABORT | CHECK_LOAD) != 0
}

fn begin_talk(site: &mut impl ZoqFotPikSite) {
    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.set_global_flags_and_data(TALK_FLAGS);
}

fn restore_world(site: &mut impl ZoqFotPikSite) {
    let activity = site.current_activity();
    site.set_current_activity(activity & !START_INTERPLANETARY);
    site.reinit_built_ship_q();
    site.load_ip_groups();
}

fn meet(story: &impl Story, site: &mut impl ZoqFotPikSite) {
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the fleet is cloned into it"
    );

    if story.flag("ZOQFOT_DISTRESS") != 0 {
        site.clone_black_urquan_into_built_queue();
        begin_talk(site);
        site.talk_black_urquan();
        if conversation_ended_the_game(site) {
            return;
        }
        if site.built_ship_count() > 0 {
            restore_world(site);
            return;
        }
    }

    site.clone_fleet_into_built_queue();
    begin_talk(site);
    site.talk();
    if !conversation_ended_the_game(site) {
        restore_world(site);
    }
}

/// `GenerateZoqFotPik_generateEnergy`: the ruins of planet 0.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    if body == WORLD {
        return default::generate_ruins(rng, sys_info, which_node, info);
    }
    0
}

/// `GenerateZoqFotPik_pickupEnergy`: the standard cycled ruins report.
pub fn pickup_energy(body: Body, report: &mut impl DiscoveryReportPort) -> bool {
    if body == WORLD {
        default::lander_report_cycle(report);
    }
    false
}

#[cfg(test)]
mod tests;
