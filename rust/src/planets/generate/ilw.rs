//! `genilw.c`: the Ilwrath home system. Planet 0 is a primordial world with
//! ruins; flying into orbit may start the Ilwrath conversation.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: none; the conversation sequence is a host
//! effect.

use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::special::{self, Body, Orbit, PRIMORDIAL_WORLD};
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};

/// `GLOBAL_FLAGS_AND_DATA` value that marks the Ilwrath conversation as a
/// home contact: `1 << 7`.
pub const HOME_CONTACT_FLAGS: u8 = 1 << 7;

/// What entering orbit at the Ilwrath home does to the rest of the game, one
/// method per host call of `GenerateIlwrath_generateOrbital`, in the order the
/// C makes them.
pub trait IlwrathSite {
    /// `StartSphereTracking (ILWRATH_SHIP)`.
    fn start_sphere_tracking(&mut self) -> bool;
    /// `NotifyOthers (ILWRATH_SHIP, IPNL_ALL_CLEAR)`.
    fn notify_others_all_clear(&mut self);
    /// `PutGroupInfo (GROUPS_RANDOM, GROUP_SAVE_IP)`.
    fn save_ip_groups(&mut self);
    /// `ReinitQueue (&GLOBAL (ip_group_q))`.
    fn reinit_ip_group_q(&mut self);
    /// `CountLinks (&GLOBAL (npc_built_ship_q))`.
    fn built_ship_count(&self) -> u16;
    /// `CloneShipFragment (ILWRATH_SHIP, &GLOBAL (npc_built_ship_q),
    /// INFINITE_FLEET)`.
    fn clone_ilwrath_fleet_into_built_queue(&mut self);
    /// `GLOBAL (CurrentActivity)`.
    fn current_activity(&self) -> u16;
    fn set_current_activity(&mut self, activity: u16);
    /// `SET_GAME_STATE (GLOBAL_FLAGS_AND_DATA, value)`.
    fn set_global_flags_and_data(&mut self, value: u8);
    /// `InitCommunication (ILWRATH_CONVERSATION)`.
    fn talk_to_ilwrath(&mut self);
    /// `ReinitQueue (&GLOBAL (npc_built_ship_q))`.
    fn reinit_built_ship_q(&mut self);
    /// `GetGroupInfo (GROUPS_RANDOM, GROUP_LOAD_IP)`.
    fn load_ip_groups(&mut self);
    /// `LoadStdLanderFont`, the ruins mask and the ruins report strings.
    fn load_ruins_scene(&mut self);
}

/// `GenerateIlwrath_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = PRIMORDIAL_WORLD;
        special::reposition(&mut planets[0], 204);
    })
}

/// `GenerateIlwrath_generateOrbital`: while the Ilwrath sphere of influence
/// can still be tracked, entering orbit at the home world is the Ilwrath
/// conversation and nothing else; otherwise the ruins scene is loaded before
/// the default orbital, and the home world gets its fixed weather.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    site: &mut impl IlwrathSite,
    load_planet: impl FnOnce(),
) -> bool {
    if enter_orbit(body, site) {
        return true;
    }
    orbit.default_orbital(load_planet);
    set_home_climate(body, orbit.sys_info);
    true
}

/// The part of `generateOrbital` before the default orbital: true when the
/// Ilwrath conversation took the visit, so the default orbital is skipped.
fn enter_orbit(body: Body, site: &mut impl IlwrathSite) -> bool {
    if body != Body::Planet(0) {
        return false;
    }
    if site.start_sphere_tracking() {
        meet_ilwrath(site);
        return true;
    }
    site.load_ruins_scene();
    false
}

/// The contact sequence: the interplanetary groups are put away, the Ilwrath
/// flagship stands alone in the built-ship queue for the conversation, and
/// unless the conversation aborted or loaded a game the world is put back.
fn meet_ilwrath(site: &mut impl IlwrathSite) {
    site.notify_others_all_clear();
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the Ilwrath flagship is cloned into it"
    );

    site.clone_ilwrath_fleet_into_built_queue();

    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.set_global_flags_and_data(HOME_CONTACT_FLAGS);
    site.talk_to_ilwrath();

    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        site.set_current_activity(activity & !START_INTERPLANETARY);
        site.reinit_built_ship_q();
        site.load_ip_groups();
    }
}

/// The weather and tectonics the home world has after the default orbital.
fn set_home_climate(body: Body, sys_info: &mut SYSTEM_INFO) {
    if body == Body::Planet(0) {
        sys_info.PlanetInfo.Weather = 2;
        sys_info.PlanetInfo.Tectonics = 3;
    }
}

/// `GenerateIlwrath_generateEnergy`.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    if body == Body::Planet(0) {
        default::generate_ruins(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GenerateIlwrath_pickupEnergy`: the ruins give the standard cycled report
/// and are never taken.
pub fn pickup_energy(body: Body, report: &mut impl DiscoveryReportPort) -> bool {
    if body == Body::Planet(0) {
        default::lander_report_cycle(report);
    }
    false
}

#[cfg(test)]
mod tests;
