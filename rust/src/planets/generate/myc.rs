//! `genmyc.c`: the Mycon home, the Sun Device system and the three egg case
//! systems. Which one a hook runs for is `CurStarDescPtr->Index`; all of them
//! lay out planet 0 as a shattered world. The Mycon home and the Sun Device
//! system share the Mycon conversation (the Sun Device is guarded until the
//! Mycon fleet is beaten there), the Sun Device lies on planet 0 of its system
//! until taken, and each egg case system holds one egg case.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `generateLife`, `pickupEnergy`. Story flags: `SUN_DEVICE`,
//! `SUN_DEVICE_UNGUARDED`, `SUN_DEVICE_ON_SHIP`, `MYCON_FELL_FOR_AMBUSH`,
//! `MYCON_VISITS`, `KNOW_ABOUT_SHATTERED`, `EGG_CASE0_ON_SHIP`,
//! `EGG_CASE1_ON_SHIP`, `EGG_CASE2_ON_SHIP`.

use crate::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::identities::{
    EGG_CASE0_DEFINED, EGG_CASE1_DEFINED, EGG_CASE2_DEFINED, MYCON_DEFINED, SUN_DEVICE_DEFINED,
};
use crate::planets::generate::special::{self, Body, Story, SHATTERED_WORLD};
use crate::planets::generate::surface::ENERGY_SCAN;
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};

/// Every hook looks at planet 0.
const WORLD: Body = Body::Planet(0);
/// `GLOBAL_FLAGS_AND_DATA` during the talk at the Mycon home (`1 << 7`) and
/// at the Sun Device system (`1 << 6`).
const HOME_TALK_FLAGS: u8 = 1 << 7;
const SUN_DEVICE_TALK_FLAGS: u8 = 1 << 6;
/// The Mycon fleet that springs the ambush at the Sun Device system.
const AMBUSH_SHIPS: u8 = 5;
/// `EARTH_RADIUS * 80 / 100`: the shattered world's orbit.
const WORLD_ORBIT_PERCENT: i32 = 80;
/// The most moons planet 0 keeps.
const MAX_MOONS: u8 = 2;

/// The scene `generateOrbital` loads before the default orbital: the mask and
/// report strings of the Sun Device or of an egg case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    SunDevice,
    EggCase,
}

/// What `generateOrbital` does besides the Mycon contact flow
/// ([`HomeContactSite`] with the Mycon ship and conversation).
pub trait MyconSite: HomeContactSite {
    /// `CloneShipFragment (MYCON_SHIP, &GLOBAL (npc_built_ship_q), 0)`.
    fn clone_ambush_ship_into_built_queue(&mut self);
    /// `GetHeadLink (&GLOBAL (npc_built_ship_q)) != 0`.
    fn built_queue_has_ships(&self) -> bool;
    /// `RepairSISBorder ()`.
    fn repair_sis_border(&mut self);
    /// `LoadStdLanderFont`, the mask and the report strings of `scene`.
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        scene: Scene,
    );
}

fn is_egg_case(star_index: u8) -> bool {
    matches!(
        star_index,
        EGG_CASE0_DEFINED | EGG_CASE1_DEFINED | EGG_CASE2_DEFINED
    )
}

/// `GenerateMycon_generatePlanets`: planet 0 is a shattered world with at most
/// two moons.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        let world = &mut planets[0];
        world.data_index = SHATTERED_WORLD;
        if world.num_planets > MAX_MOONS {
            world.num_planets = MAX_MOONS;
        }
        special::reposition(world, WORLD_ORBIT_PERCENT);
    })
}

/// `GenerateMycon_generateOrbital`: at planet 0 the Mycon talk when they can
/// be tracked (at the Sun Device system: until it is unguarded), the Sun
/// Device or egg case scene is loaded while the object is still there, and
/// then the default orbital runs. A talk that was aborted, loaded a game or
/// left Mycon ships alive ends the visit before the default orbital.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    star_index: u8,
    site: &mut (impl MyconSite + Story + super::host::PlanetSite + ?Sized),
) -> bool {
    if prepare_orbital(system, body, star_index, site) {
        return true;
    }

    super::special::default_orbital(system, body, site);
    true
}

/// Contact and scene work, reading the retrieval mask after the conversation.
pub(super) fn prepare_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    star_index: u8,
    site: &mut (impl MyconSite + Story + ?Sized),
) -> bool {
    if body == WORLD {
        if (star_index == MYCON_DEFINED || star_index == SUN_DEVICE_DEFINED)
            && site.start_sphere_tracking()
            && (star_index == MYCON_DEFINED || site.flag(StoryKey::SUN_DEVICE_UNGUARDED) == 0)
            && !mycon_talk(star_index, site)
        {
            return true;
        }

        match star_index {
            SUN_DEVICE_DEFINED => {
                if site.flag(StoryKey::SUN_DEVICE) == 0 {
                    site.load_scene(system, Scene::SunDevice);
                }
            }
            index if is_egg_case(index) => {
                if site.flag(StoryKey::KNOW_ABOUT_SHATTERED) == 0 {
                    site.set_flag(StoryKey::KNOW_ABOUT_SHATTERED, 1);
                }
                let retrieved = system.planet_info().scan_retrieve_mask[ENERGY_SCAN] & 1 != 0;
                if !retrieved {
                    site.load_scene(system, Scene::EggCase);
                }
            }
            _ => {}
        }
    }

    false
}

/// The Mycon confront the player: false when the visit is over, because the
/// talk aborted or loaded a game or because Mycon ships survived it. When the
/// player wins, the Sun Device is unguarded and the border repaired.
fn mycon_talk(star_index: u8, site: &mut (impl MyconSite + Story + ?Sized)) -> bool {
    site.notify_others_all_clear();
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(
        site.built_ship_count(),
        0,
        "the built-ship queue must be empty before the Mycon are cloned into it"
    );

    if star_index == MYCON_DEFINED || site.flag(StoryKey::MYCON_FELL_FOR_AMBUSH) == 0 {
        site.clone_fleet_into_built_queue();
    } else {
        for _ in 0..AMBUSH_SHIPS {
            site.clone_ambush_ship_into_built_queue();
        }
    }

    let activity = site.current_activity();
    site.set_current_activity(activity | START_INTERPLANETARY);
    site.set_global_flags_and_data(if star_index == MYCON_DEFINED {
        HOME_TALK_FLAGS
    } else {
        SUN_DEVICE_TALK_FLAGS
    });
    site.talk();

    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) != 0 {
        return false;
    }
    let survivors = site.built_queue_has_ships();
    site.set_current_activity(activity & !START_INTERPLANETARY);
    site.reinit_built_ship_q();
    site.load_ip_groups();
    if survivors {
        return false;
    }
    site.set_flag(StoryKey::SUN_DEVICE_UNGUARDED, 1);
    site.repair_sis_border();
    true
}

#[derive(Clone, Copy)]
pub struct EnergyContext {
    pub star_index: u8,
    pub report_loaded: bool,
}

/// `GenerateMycon_generateEnergy`: the Sun Device on planet 0 of its system
/// until taken, and the egg case on planet 0 of an egg case system while the
/// orbital left its report strings loaded (they are not once it is taken).
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    context: EnergyContext,
    body: Body,
    which_node: u16,
    info: &mut NodeInfo,
    story: &(impl Story + ?Sized),
) -> u16 {
    let EnergyContext {
        star_index,
        report_loaded,
    } = context;
    if star_index == SUN_DEVICE_DEFINED && body == WORLD {
        if story.flag(StoryKey::SUN_DEVICE) != 0 {
            return 0;
        }
        return default::generate_artifact(rng, sys_info, which_node, info);
    }
    if is_egg_case(star_index) && body == WORLD {
        if !report_loaded {
            return 0;
        }
        return default::generate_artifact(rng, sys_info, which_node, info);
    }
    0
}

/// `GenerateMycon_generateLife`: there is no life in Mycon systems.
pub fn generate_life() -> u16 {
    0
}

/// `GenerateMycon_pickupEnergy`: the Sun Device or the egg case is taken from
/// node 0, which launches the lander and records it on the ship.
pub fn pickup_energy(
    system: &mut SolarSys<'_>,
    star_index: u8,
    body: Body,
    which_node: u16,
    lander: &mut (impl LanderTakeoff + Story + super::host::ReportSite + ?Sized),
) -> bool {
    if star_index == SUN_DEVICE_DEFINED && body == WORLD {
        assert!(
            lander.flag(StoryKey::SUN_DEVICE) == 0 && which_node == 0,
            "the Sun Device is picked up once, from node 0"
        );
        lander.report(
            system,
            crate::engine::world::generation::ReportDisposition::Once,
        );
        lander.lander_takeoff();
        lander.set_flag(StoryKey::SUN_DEVICE, 1);
        lander.set_flag(StoryKey::SUN_DEVICE_ON_SHIP, 1);
        lander.set_flag(StoryKey::MYCON_VISITS, 0);
        return true;
    }
    if is_egg_case(star_index) && body == WORLD {
        assert_eq!(which_node, 0, "an egg case is picked up from node 0");
        lander.report(
            system,
            crate::engine::world::generation::ReportDisposition::Once,
        );
        lander.lander_takeoff();
        lander.set_flag(egg_case_flag(star_index), 1);
        return true;
    }
    false
}

fn egg_case_flag(star_index: u8) -> StoryKey {
    match star_index {
        EGG_CASE0_DEFINED => StoryKey::EGG_CASE0_ON_SHIP,
        EGG_CASE1_DEFINED => StoryKey::EGG_CASE1_ON_SHIP,
        EGG_CASE2_DEFINED => StoryKey::EGG_CASE2_ON_SHIP,
        other => unreachable!("star {other} has no egg case"),
    }
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn generate_planets(&self, system: &mut SolarSys<'_>, _host: &mut dyn SiteHost) -> bool {
        system.layout(|rng, table, size, color, planets| {
            generate_planets(rng, table, size, color, planets)
        });
        true
    }
    fn generate_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        host: &dyn SiteHost,
    ) -> u16 {
        let context = EnergyContext {
            star_index: system.star_index(),
            report_loaded: system.report.current().is_some(),
        };
        system.scan(body, |rng, sys, _frame| {
            generate_energy(rng, sys, context, body, node, info, host)
        })
    }
    fn generate_life(
        &self,
        _system: &mut SolarSys<'_>,
        _body: Body,
        _node: u16,
        _info: &mut NodeInfo,
        _host: &dyn SiteHost,
    ) -> u16 {
        generate_life()
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, system.star_index(), host)
    }
    fn pickup_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        host: &mut dyn SiteHost,
    ) -> bool {
        pickup_energy(system, system.star_index(), body, node, host)
    }
}

#[cfg(test)]
mod tests;
