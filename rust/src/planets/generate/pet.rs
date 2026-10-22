//! `genpet.c`: the Umgah system, home of the talking pet. Planet 0 is a
//! telluric world of ruins. Entering orbit while the Umgah can be met (or the
//! zombie blobbies run the system, or the pet has not been found) plays the
//! Umgah or talking pet conversation; a hypnotised player is then taken to an
//! Ur-Quan ship in hyperspace.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: `UMGAH_ZOMBIE_BLOBBIES`, `TALKING_PET`,
//! `PLAYER_HYPNOTIZED`.

use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{
    CHECK_ABORT, CHECK_LOAD, IN_HYPERSPACE, START_ENCOUNTER, START_INTERPLANETARY,
};
use crate::math::{square_root, RandomContext};
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::HomeContactSite;
use crate::planets::generate::special::{self, Body, Orbit, Story, TELLURIC_WORLD};
use crate::starbase::sis_modules::FUEL_TANK_SCALE;

const WORLD: Body = Body::Planet(0);

/// `GLOBAL_FLAGS_AND_DATA` during either conversation: `1 << 7`.
const TALK_FLAGS: u8 = 1 << 7;
/// The zombie fleet: ten Umgah ships of fleet size 0.
const ZOMBIE_SHIPS: u32 = 10;

/// Where the Ur-Quan ship that takes the hypnotised player waits
/// (`loc_pt`, universe coordinates).
const URQUAN_LOC: (i16, i16) = (5288, 4892);
/// `LOST_DAYS`: the days the hypnosis takes.
const LOST_DAYS: u16 = 15;

/// What `generateOrbital` asks of the host. The shared contact calls are for
/// the Umgah (`UMGAH_SHIP`); their conversation is `talk`.
pub trait TalkingPetSite: HomeContactSite {
    /// `CloneShipFragment (UMGAH_SHIP, &GLOBAL (npc_built_ship_q), 0)`.
    fn clone_zombie_into_built_queue(&mut self);
    /// `GetHeadLink (&GLOBAL (npc_built_ship_q)) != 0`.
    fn built_ship_survives(&self) -> bool;
    /// `InitCommunication (TALKING_PET_CONVERSATION)`.
    fn talk_pet(&mut self);
}

/// What `ZapToUrquanEncounter` does to the rest of the game.
pub trait UrquanEncounterSite {
    /// `AllocEncounter ()`, or failing that the head encounter (removed and
    /// cleared); then the one-shot Ur-Quan encounter described by the
    /// `URQUAN_SHIP` fleet template is built at the universe point `loc` and
    /// the SIS logical position is set to match. False when there is no
    /// encounter to use. The encounter stays locked until `finish_encounter`.
    fn begin_urquan_encounter(&mut self, loc: (i16, i16)) -> bool;
    /// `SleepThreadUntil (FadeScreen (FadeAllToBlack, ONE_SECOND * 2))`.
    fn fade_to_black(&mut self);
    /// `MoveGameClockDays (days)`.
    fn move_game_clock_days(&mut self, days: u16);
    /// `CurStarDescPtr->star_pt`.
    fn star_point(&self) -> (i16, i16);
    /// `GLOBAL_SIS (FuelOnBoard)`.
    fn fuel_on_board(&self) -> u32;
    /// `DeltaSISGauges (0, delta, 0)`.
    fn delta_fuel(&mut self, delta: i16);
    /// `TFB_Random ()`.
    fn random(&mut self) -> u32;
    /// `DrawSISMessage (NULL)`.
    fn clear_sis_message(&mut self);
    /// `DrawHyperCoords (loc)`.
    fn draw_hyper_coords(&mut self, loc: (i16, i16));
    /// `UnlockEncounter (hEncounter)`.
    fn finish_encounter(&mut self);
}

/// `GenerateTalkingPet_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        let world = &mut planets[0];
        world.data_index = TELLURIC_WORLD;
        special::reposition(world, 204);
    })
}

/// `GenerateTalkingPet_generateOrbital`: at planet 0, the conversation takes
/// the visit while the zombie blobbies run the system, the pet has not been
/// found, or the Umgah sphere of influence can be tracked. Otherwise the
/// ruins scene is loaded before the default orbital, which is then given calm
/// weather.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &mut impl Story,
    site: &mut (impl TalkingPetSite + UrquanEncounterSite),
    load_planet: impl FnOnce(),
) -> bool {
    if body == WORLD
        && (story.flag("UMGAH_ZOMBIE_BLOBBIES") != 0
            || story.flag("TALKING_PET") == 0
            || site.start_sphere_tracking())
    {
        meet(story, site);
        return true;
    }

    if body == WORLD {
        site.load_ruins_scene(false);
    }

    orbit.default_orbital(load_planet);

    if body == WORLD {
        orbit.sys_info.PlanetInfo.Weather = 0;
    }
    true
}

fn meet(story: &impl Story, site: &mut (impl TalkingPetSite + UrquanEncounterSite)) {
    site.notify_others_all_clear();
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the fleet is cloned into it"
    );

    if site.start_sphere_tracking() {
        let activity = HomeContactSite::current_activity(site);
        HomeContactSite::set_current_activity(site, activity | START_INTERPLANETARY);
        site.set_global_flags_and_data(TALK_FLAGS);
        if story.flag("UMGAH_ZOMBIE_BLOBBIES") == 0 {
            site.clone_fleet_into_built_queue();
            site.talk();
        } else {
            for _ in 0..ZOMBIE_SHIPS {
                site.clone_zombie_into_built_queue();
            }
            site.talk_pet();
        }
    }

    let activity = HomeContactSite::current_activity(site);
    if activity & (CHECK_ABORT | CHECK_LOAD) != 0 {
        return;
    }

    let umgah_survivors = site.built_ship_survives();
    HomeContactSite::set_current_activity(site, activity & !START_INTERPLANETARY);

    if story.flag("PLAYER_HYPNOTIZED") != 0 {
        zap_to_urquan_encounter(site);
    } else if (story.flag("UMGAH_ZOMBIE_BLOBBIES") != 0 && !umgah_survivors)
        || !site.start_sphere_tracking()
    {
        // The zombie fleet was defeated, or the Kohr-Ah have destroyed the
        // Umgah and the talking pet survived.
        site.talk_pet();
    }

    site.reinit_built_ship_q();
    site.load_ip_groups();
}

/// `ZapToUrquanEncounter`: the hypnotised player wakes up in hyperspace at
/// an Ur-Quan ship, fifteen days later and short of fuel.
fn zap_to_urquan_encounter(site: &mut (impl HomeContactSite + UrquanEncounterSite)) {
    if !site.begin_urquan_encounter(URQUAN_LOC) {
        return;
    }

    site.fade_to_black();
    site.move_game_clock_days(LOST_DAYS);

    HomeContactSite::set_current_activity(site, IN_HYPERSPACE | START_ENCOUNTER);

    let star = site.star_point();
    let dx = star.0.wrapping_sub(URQUAN_LOC.0);
    let dy = star.1.wrapping_sub(URQUAN_LOC.1);
    let distance =
        square_root((i32::from(dx) * i32::from(dx) + i32::from(dy) * i32::from(dy)) as u32);
    let cost = (distance as i16).wrapping_add((FUEL_TANK_SCALE >> 1) as i16);

    site.delta_fuel(cost.wrapping_neg());
    let scale = FUEL_TANK_SCALE as i32;
    if site.fuel_on_board() < 5 * FUEL_TANK_SCALE {
        let refill = ((5 + (site.random() as u16 % 5)) as i32) * scale;
        site.delta_fuel(refill.wrapping_sub(i32::from(site.fuel_on_board() as i16)) as i16);
    }
    site.clear_sis_message();
    site.draw_hyper_coords(URQUAN_LOC);

    site.finish_encounter();
}

/// `GenerateTalkingPet_generateEnergy`: the ruins of planet 0.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    if body == WORLD {
        default::generate_ruins(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GenerateTalkingPet_pickupEnergy`: the standard cycled ruins report.
pub fn pickup_energy(body: Body, report: &mut impl DiscoveryReportPort) -> bool {
    if body == WORLD {
        default::lander_report_cycle(report);
    }
    false
}

#[cfg(test)]
mod tests;
