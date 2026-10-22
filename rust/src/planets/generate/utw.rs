//! `genutw.c`: the two Utwig systems. Which one a hook runs for is
//! `CurStarDescPtr->Index`: the Utwig home (a water world with ruins) or the
//! bomb system, where the Utwig bomb lies on moon 1 of planet 5 until the
//! player takes it, guarded by the Utwig (until the player has the Ultron) and
//! by the Druuge (until the bomb is unprotected).
//!
//! Overridden: `initNpcs`, `generatePlanets`, `generateOrbital`,
//! `generateEnergy`, `pickupEnergy`. Story flags: `UTWIG_BOMB`,
//! `UTWIG_HAVE_ULTRON`, `BOMB_UNPROTECTED`, `UTWIG_BOMB_ON_SHIP`,
//! `DRUUGE_MANNER`, `DRUUGE_VISITS`, `DRUUGE_HOME_VISITS`.

use crate::battle::gendef::{BOMB_DEFINED, UTWIG_DEFINED};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default::{self, Npcs};
use crate::planets::generate::home_contact::{self, HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{self, Body, Orbit, Story, WATER_WORLD};
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};

/// The Utwig home world.
const HOME: Body = Body::Planet(0);
/// The moon of the bomb system that holds the bomb: `matchWorld (.., 5, 1)`.
const BOMB_MOON: Body = Body::Moon { planet: 5, moon: 1 };

/// `GLOBAL_FLAGS_AND_DATA` during the talk at the Utwig home (`1 << 7`) and
/// during the talks at the bomb moon (`1 << 6`).
const HOME_TALK_FLAGS: u8 = 1 << 7;
const BOMB_TALK_FLAGS: u8 = 1 << 6;
/// The Druuge guard the bomb with five ships.
const DRUUGE_GUARD_SHIPS: u8 = 5;
/// `EARTH_RADIUS * 174 / 100`: the Utwig home's orbit.
const HOME_ORBIT_PERCENT: i32 = 174;

/// What `generateOrbital` does besides the Utwig contact flow
/// ([`HomeContactSite`] with the Utwig ship and conversation).
pub trait UtwigSite: HomeContactSite {
    /// `StartSphereTracking (DRUUGE_SHIP)`.
    fn start_druuge_tracking(&mut self) -> bool;
    /// `CloneShipFragment (DRUUGE_SHIP, &GLOBAL (npc_built_ship_q), 0)`.
    fn clone_druuge_into_built_queue(&mut self);
    /// `InitCommunication (DRUUGE_CONVERSATION)`.
    fn talk_druuge(&mut self);
    /// `GetHeadLink (&GLOBAL (npc_built_ship_q)) != 0`.
    fn built_queue_has_ships(&self) -> bool;
    /// `RepairSISBorder ()`.
    fn repair_sis_border(&mut self);
    /// `LoadStdLanderFont`, the mask and the report strings of the bomb
    /// (`bomb`) or the Utwig ruins.
    fn load_scene(&mut self, bomb: bool);
}

/// The bomb moon is the object of the visit while the bomb is still there.
fn at_bomb(star_index: u8, body: Body, story: &impl Story) -> bool {
    star_index == BOMB_DEFINED && body == BOMB_MOON && story.flag("UTWIG_BOMB") == 0
}

/// `GenerateUtwig_initNpcs`: with the bomb still in place no ships roam the
/// bomb system.
pub fn init_npcs<H: Npcs + HomeContactSite>(
    star_index: u8,
    story: &impl Story,
    host: &mut H,
) -> bool {
    if star_index == BOMB_DEFINED && story.flag("UTWIG_BOMB") == 0 {
        Npcs::reinit_ip_group_q(host);
        assert_eq!(
            HomeContactSite::built_ship_count(host),
            0,
            "the built-ship queue must be empty before the bomb system is left empty"
        );
    } else {
        default::init_npcs(host);
    }
    true
}

/// `GenerateUtwig_generatePlanets` for the system at `star_index`: only the
/// Utwig home reshapes planet 0.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
    star_index: u8,
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        if star_index == UTWIG_DEFINED {
            let world = &mut planets[0];
            world.data_index = WATER_WORLD;
            world.NumPlanets = 1;
            special::reposition(world, HOME_ORBIT_PERCENT);
        }
    })
}

/// `GenerateUtwig_generateOrbital`: at the Utwig home, or at the bomb while it
/// is there, the Utwig talk when they can be tracked (at the bomb: until the
/// player has the Ultron), then at the bomb the Druuge guard it until it is
/// unprotected; the ruins or bomb scene is loaded before the default orbital.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    star_index: u8,
    story: &mut impl Story,
    site: &mut impl UtwigSite,
    load_planet: impl FnOnce(),
) -> bool {
    if prepare_orbital(body, star_index, story, site) {
        return true;
    }

    orbit.default_orbital(load_planet);

    if star_index == UTWIG_DEFINED && body == HOME {
        let info = &mut orbit.sys_info.PlanetInfo;
        info.Weather = 1;
        info.Tectonics = 1;
    }
    true
}

/// Contact and scene work, with no system or RNG borrow across host calls.
pub(super) fn prepare_orbital(
    body: Body,
    star_index: u8,
    story: &mut impl Story,
    site: &mut impl UtwigSite,
) -> bool {
    let at_home = star_index == UTWIG_DEFINED && body == HOME;
    let bomb = at_bomb(star_index, body, story);
    if at_home || bomb {
        if (star_index == UTWIG_DEFINED || story.flag("UTWIG_HAVE_ULTRON") == 0)
            && site.start_sphere_tracking()
        {
            let flags = if star_index == UTWIG_DEFINED {
                HOME_TALK_FLAGS
            } else {
                BOMB_TALK_FLAGS
            };
            home_contact::meet(site, flags);
            return true;
        }

        if star_index == BOMB_DEFINED
            && story.flag("BOMB_UNPROTECTED") == 0
            && site.start_druuge_tracking()
        {
            if !druuge_guard_talk(site) {
                return true;
            }
            site.repair_sis_border();
            story.set_flag("BOMB_UNPROTECTED", 1);
        }

        site.load_scene(star_index == BOMB_DEFINED);
    }

    false
}

/// The Druuge confront the player at the bomb: false when the talk aborted or
/// loaded a game, or when Druuge ships survived it.
fn druuge_guard_talk(site: &mut impl UtwigSite) -> bool {
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the Druuge are cloned into it"
    );
    for _ in 0..DRUUGE_GUARD_SHIPS {
        site.clone_druuge_into_built_queue();
    }
    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.set_global_flags_and_data(BOMB_TALK_FLAGS);
    site.talk_druuge();

    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) != 0 {
        return false;
    }
    let survivors = site.built_queue_has_ships();
    site.set_current_activity(activity & !START_INTERPLANETARY);
    site.reinit_built_ship_q();
    site.load_ip_groups();
    !survivors
}

/// `GenerateUtwig_generateEnergy`: ruins at the Utwig home and the bomb at the
/// bomb moon until it is taken.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    star_index: u8,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
    story: &impl Story,
) -> u16 {
    if star_index == UTWIG_DEFINED && body == HOME {
        return default::generate_ruins(rng, sys_info, which_node, info);
    }
    if star_index == BOMB_DEFINED && body == BOMB_MOON {
        if story.flag("UTWIG_BOMB") != 0 {
            return 0;
        }
        return default::generate_artifact(rng, sys_info, which_node, info);
    }
    0
}

/// `GenerateUtwig_pickupEnergy`: the home ruins give the standard cycled
/// report; the bomb is taken once, from node 0, and makes the Druuge
/// interested.
pub fn pickup_energy(
    star_index: u8,
    body: Body,
    which_node: u16,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if star_index == UTWIG_DEFINED && body == HOME {
        default::lander_report_cycle(report);
        return false;
    }
    if star_index == BOMB_DEFINED && body == BOMB_MOON {
        assert!(
            story.flag("UTWIG_BOMB") == 0 && which_node == 0,
            "the bomb is picked up once, from node 0"
        );
        default::lander_report(report);
        lander.lander_takeoff();
        story.set_flag("UTWIG_BOMB", 1);
        story.set_flag("UTWIG_BOMB_ON_SHIP", 1);
        story.set_flag("DRUUGE_MANNER", 1);
        story.set_flag("DRUUGE_VISITS", 0);
        story.set_flag("DRUUGE_HOME_VISITS", 0);
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
