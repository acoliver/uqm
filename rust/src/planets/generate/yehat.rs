//! `genyeh.c`: the Yehat home system. Planet 0 is a water world with the
//! ruins of the Yehat capital; flying into orbit may start the Yehat
//! conversation.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Everything else is the default. Story flags: none; the
//! conversation sequence is a host effect.

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::special::{self, Body, Orbit, WATER_WORLD};

/// `START_INTERPLANETARY`, `CHECK_ABORT` and `CHECK_LOAD` of
/// `GLOBAL (CurrentActivity)`.
pub use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};

/// `GLOBAL_FLAGS_AND_DATA` value that marks the Yehat conversation as a
/// capital contact: `1 << 7`.
pub const CAPITAL_CONTACT_FLAGS: u8 = 1 << 7;

/// What entering orbit at the Yehat capital does to the rest of the game,
/// one method per host call of `GenerateYehat_generateOrbital`, in the order
/// the C makes them.
pub trait YehatSite {
    /// `StartSphereTracking (YEHAT_SHIP)`.
    fn start_sphere_tracking(&mut self) -> bool;
    /// `NotifyOthers (YEHAT_SHIP, IPNL_ALL_CLEAR)`.
    fn notify_others_all_clear(&mut self);
    /// `PutGroupInfo (GROUPS_RANDOM, GROUP_SAVE_IP)`.
    fn save_ip_groups(&mut self);
    /// `ReinitQueue (&GLOBAL (ip_group_q))`.
    fn reinit_ip_group_q(&mut self);
    /// `CountLinks (&GLOBAL (npc_built_ship_q))`.
    fn built_ship_count(&self) -> u16;
    /// `CloneShipFragment (YEHAT_SHIP, &GLOBAL (npc_built_ship_q),
    /// INFINITE_FLEET)`.
    fn clone_yehat_fleet_into_built_queue(&mut self);
    /// `GLOBAL (CurrentActivity)`.
    fn current_activity(&self) -> u16;
    fn set_current_activity(&mut self, activity: u16);
    /// `SET_GAME_STATE (GLOBAL_FLAGS_AND_DATA, value)`.
    fn set_global_flags_and_data(&mut self, value: u8);
    /// `InitCommunication (YEHAT_CONVERSATION)`.
    fn talk_to_yehat(&mut self);
    /// `ReinitQueue (&GLOBAL (npc_built_ship_q))`.
    fn reinit_built_ship_q(&mut self);
    /// `GetGroupInfo (GROUPS_RANDOM, GROUP_LOAD_IP)`.
    fn load_ip_groups(&mut self);
    /// `LoadStdLanderFont`, the ruins mask and the ruins report strings.
    fn load_ruins_scene(&mut self);
}

/// `GenerateYehat_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[crate::c_bindings::global_state::PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = WATER_WORLD;
        planets[0].NumPlanets = 1;
        special::reposition(&mut planets[0], 106);
    })
}

/// `GenerateYehat_generateOrbital`: while the Yehat sphere of influence can
/// still be tracked, entering orbit at the capital is the Yehat conversation
/// and nothing else; otherwise the ruins scene is loaded before the default
/// orbital.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    site: &mut impl YehatSite,
    load_planet: impl FnOnce(),
) -> bool {
    if enter_orbit(body, site) {
        return true;
    }
    orbit.default_orbital(load_planet)
}

/// The part of `generateOrbital` before the default orbital: true when the
/// Yehat conversation took the visit, so the default orbital is skipped.
fn enter_orbit(body: Body, site: &mut impl YehatSite) -> bool {
    if body != Body::Planet(0) {
        return false;
    }
    if site.start_sphere_tracking() {
        meet_yehat(site);
        return true;
    }
    site.load_ruins_scene();
    false
}

/// The contact sequence: the interplanetary groups are put away, the Yehat
/// flagship stands alone in the built-ship queue for the conversation, and
/// unless the conversation aborted or loaded a game the world is put back.
fn meet_yehat(site: &mut impl YehatSite) {
    site.notify_others_all_clear();
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the Yehat flagship is cloned into it"
    );

    site.clone_yehat_fleet_into_built_queue();

    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.set_global_flags_and_data(CAPITAL_CONTACT_FLAGS);
    site.talk_to_yehat();

    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        site.set_current_activity(activity & !START_INTERPLANETARY);
        site.reinit_built_ship_q();
        site.load_ip_groups();
    }
}

/// `GenerateYehat_generateEnergy`.
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

/// `GenerateYehat_pickupEnergy`: the ruins give the standard cycled report and
/// are never taken.
pub fn pickup_energy(body: Body, report: &mut impl DiscoveryReportPort) -> bool {
    if body == Body::Planet(0) {
        default::lander_report_cycle(report);
    }
    false
}

#[cfg(test)]
mod tests;
