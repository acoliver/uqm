//! `genchmmr.c`: the Chmmr home system. Planet 1 is the sapphire world, shielded
//! until the Chmmr are unleashed, with a starbase moon.
//!
//! Overridden: `generatePlanets`, `generateMoons`, `generateOrbital`. Story
//! flags: `CHMMR_UNLEASHED`, `CHMMR_BOMB_STATE`, `SUN_DEVICE_ON_SHIP`,
//! `ILWRATH_DECEIVED`.

use crate::battle::battle_types::normalize_angle;
use crate::c_bindings::global_state::{PlanetFrame, PLANET_DESC};
use crate::math::RandomContext;
use crate::planets::consts::PLANET_SHIELDED;
use crate::planets::generate::default;
use crate::planets::generate::special::{
    self, Body, Orbit, Story, HIERARCHY_STARBASE, MIN_MOON_RADIUS, SAPPHIRE_WORLD,
};
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};

/// `END_INTERPLANETARY` is `START_INTERPLANETARY` (`planets.h`).
const END_INTERPLANETARY: u16 = START_INTERPLANETARY;

/// `GLOBAL_FLAGS_AND_DATA` value for the Chmmr conversation: `1 << 7`.
pub const CHMMR_CONTACT_FLAGS: u8 = 1 << 7;
/// `GLOBAL_FLAGS_AND_DATA` value for the Ilwrath ambush: `1 << 6`.
pub const ILWRATH_AMBUSH_FLAGS: u8 = 1 << 6;
/// `CHMMR_BOMB_STATE` once the bomb has been delivered.
const BOMB_DELIVERED: u32 = 2;

/// What visiting the Chmmr home does to the rest of the game, one method per
/// host call of `GenerateChmmr_generateOrbital`, in the order the C makes
/// them.
pub trait ChmmrSite {
    /// `InitCommunication (CHMMR_CONVERSATION)`.
    fn talk_to_chmmr(&mut self);
    /// `StartSphereTracking (ILWRATH_SHIP)`.
    fn start_ilwrath_sphere_tracking(&mut self) -> bool;
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
    /// The starbase visit: lander font, `CHMMR_BASE_STRTAB` report, and the
    /// release of both.
    fn visit_starbase(&mut self);
}

/// `GenerateChmmr_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
    story: &impl Story,
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[1].data_index = SAPPHIRE_WORLD;
        if story.flag("CHMMR_UNLEASHED") == 0 {
            planets[1].data_index |= PLANET_SHIELDED;
        }
        planets[1].NumPlanets = 1;
    })
}

/// `GenerateChmmr_generateMoons`: `moons` is the whole moon table and
/// `planet_moons` the moon count of the planet being generated.
pub fn generate_moons(
    rng: &mut RandomContext,
    star_size: u8,
    star_color: u8,
    planet_moons: u8,
    moons: &mut [PLANET_DESC],
    body: Body,
) -> bool {
    default::generate_moons(
        rng,
        star_size,
        star_color,
        planet_moons,
        special::moon_slice(moons, planet_moons),
    );
    if body == Body::Planet(1) {
        let base = &mut moons[0];
        base.data_index = HIERARCHY_STARBASE;
        base.radius = MIN_MOON_RADIUS;
        let angle = normalize_angle(rng.next_random() as u16);
        special::place(base, angle);
    }
    true
}

/// `GenerateChmmr_generateOrbital`: the home world, once freed, is the Chmmr
/// conversation; while shielded it may instead be the Ilwrath ambush, and the
/// starbase moon is a report. Anything else is the default orbital.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &impl Story,
    site: &mut impl ChmmrSite,
    load_planet: impl FnOnce(),
) -> bool {
    if enter_orbit(body, story, site) {
        return true;
    }
    orbit.default_orbital(load_planet)
}

/// The part of `generateOrbital` before the default orbital: true when a
/// conversation or the starbase report took the visit.
fn enter_orbit(body: Body, story: &impl Story, site: &mut impl ChmmrSite) -> bool {
    if body == Body::Planet(1) {
        if story.flag("CHMMR_UNLEASHED") != 0 {
            site.set_global_flags_and_data(CHMMR_CONTACT_FLAGS);
            site.talk_to_chmmr();
            if story.flag("CHMMR_BOMB_STATE") == BOMB_DELIVERED {
                let activity = site.current_activity();
                site.set_current_activity(activity | END_INTERPLANETARY);
            }
            return true;
        }
        if story.flag("SUN_DEVICE_ON_SHIP") != 0
            && story.flag("ILWRATH_DECEIVED") == 0
            && site.start_ilwrath_sphere_tracking()
        {
            ambush_by_ilwrath(site);
            return true;
        }
        false
    } else if body == (Body::Moon { planet: 1, moon: 0 }) {
        site.visit_starbase();
        true
    } else {
        false
    }
}

/// The Ilwrath ambush: the interplanetary groups are put away, the Ilwrath
/// flagship stands alone in the built-ship queue for the conversation, and
/// unless the conversation aborted or loaded a game the world is put back.
fn ambush_by_ilwrath(site: &mut impl ChmmrSite) {
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the Ilwrath flagship is cloned into it"
    );

    site.clone_ilwrath_fleet_into_built_queue();

    site.set_global_flags_and_data(ILWRATH_AMBUSH_FLAGS);
    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.talk_to_ilwrath();

    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        site.set_current_activity(activity & !START_INTERPLANETARY);
        site.reinit_built_ship_q();
        site.load_ip_groups();
    }
}

#[cfg(test)]
mod tests;
