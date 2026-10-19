//! `genspa.c`: Spathiwa, its moon, the broadcasters and the Evil Ones.

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::calc;
use crate::planets::consts::PLANET_SHIELDED;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{self, Body, Orbit, Story, MIN_MOON_RADIUS, WATER_WORLD};
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::planets::orbits::{self, OrbitRequest, MOON_DELTA};
use crate::planets::surface::{self, BIOLOGICAL_SCAN, MINERAL_SCAN};

const WORLD: Body = Body::Planet(0);
const MOON: Body = Body::Moon { planet: 0, moon: 0 };
// `plandata.h`: PELLUCID_WORLD; `lifeform.h`: NUM_CREATURE_TYPES.
const PELLUCID_WORLD: u8 = 22;
const EVIL_ONE: u16 = 23;
const NUM_EVIL_ONES: u16 = 32;

pub trait SpathiSite: HomeContactSite {
    /// Load font, UMGAH_BCS mask and strings; select entry 1 if unshielded.
    fn load_broadcasters_scene(&mut self, unshielded: bool);
}

pub fn generate_planets(
    rng: &mut RandomContext,
    size: u8,
    color: u8,
    planets: &mut [PLANET_DESC],
    story: &impl Story,
) -> u8 {
    orbits::fill_orbits(
        rng,
        OrbitRequest {
            star_size: size,
            star_color: color,
            num_planets: 1,
            generating_moons: false,
            types_defined: false,
        },
        planets,
    );
    special::reposition(&mut planets[0], 1150);
    planets[0].data_index = WATER_WORLD;
    if story.flag("SPATHI_SHIELDED_SELVES") != 0 {
        planets[0].data_index |= PLANET_SHIELDED;
    }
    planets[0].NumPlanets = 1;
    1
}

pub fn generate_moons(
    rng: &mut RandomContext,
    size: u8,
    color: u8,
    planet: u8,
    count: u8,
    moons: &mut [PLANET_DESC],
) {
    default::generate_moons(rng, size, color, count, moons);
    if planet == 0 {
        moons[0].data_index = PELLUCID_WORLD;
        moons[0].radius = MIN_MOON_RADIUS + MOON_DELTA as i16;
        let angle = (rng.next_random() as u16) & 63;
        special::place(&mut moons[0], angle);
    }
}

pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &impl Story,
    site: &mut impl SpathiSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == MOON {
        if story.flag("SPATHI_SHIELDED_SELVES") == 0 && site.start_sphere_tracking() {
            site.notify_others_all_clear();
            site.save_ip_groups();
            site.reinit_ip_group_q();
            assert_eq!(site.built_ship_count(), 0);
            site.clone_fleet_into_built_queue();
            site.set_global_flags_and_data(1 << 7);
            site.set_current_activity(site.current_activity() | START_INTERPLANETARY);
            site.talk();
            let activity = site.current_activity();
            if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
                site.set_current_activity(activity & !START_INTERPLANETARY);
                site.reinit_built_ship_q();
                site.load_ip_groups();
            }
            return true;
        }
        orbit.analyse();
        orbit.sys_info.PlanetInfo.Weather = 0;
        orbit.sys_info.PlanetInfo.Tectonics = 0;
        orbit.sys_info.PlanetInfo.SurfaceTemperature = 28;
        if story.flag("UMGAH_BROADCASTERS") == 0 {
            site.load_broadcasters_scene(story.flag("SPATHI_SHIELDED_SELVES") == 0);
        }
        load_planet();
        return true;
    }
    if body == WORLD {
        calc::do_planetary_analysis(
            orbit.rng,
            orbit.plan_data,
            orbit.star,
            orbit.site,
            orbit.sys_info,
            orbit.world,
        );
        orbit.sys_info.PlanetInfo.ScanSeed[MINERAL_SCAN] = orbit.rng.seed();
        let mut spare: NODE_INFO = unsafe { std::mem::zeroed() };
        surface::generate_mineral_deposits(
            orbit.rng,
            orbit.sys_info,
            default::GENERATE_ALL,
            &mut spare,
        );
        orbit.sys_info.PlanetInfo.ScanSeed[BIOLOGICAL_SCAN] = orbit.rng.seed();
        orbit.sys_info.PlanetInfo.PlanetRadius = 120;
        orbit.sys_info.PlanetInfo.SurfaceGravity = calc::calc_gravity(&orbit.sys_info.PlanetInfo);
        orbit.sys_info.PlanetInfo.Weather = 0;
        orbit.sys_info.PlanetInfo.Tectonics = 0;
        orbit.sys_info.PlanetInfo.SurfaceTemperature = 31;
        load_planet();
        return true;
    }
    orbit.default_orbital(load_planet)
}

pub fn generate_energy(
    rng: &mut RandomContext,
    info: &SYSTEM_INFO,
    body: Body,
    which: u16,
    node: &mut NODE_INFO,
    story: &impl Story,
) -> u16 {
    if body == MOON && story.flag("UMGAH_BROADCASTERS") == 0 {
        return default::generate_artifact(rng, info, which, node);
    }
    0
}

pub fn generate_life(
    rng: &mut RandomContext,
    info: &SYSTEM_INFO,
    body: Body,
    which: u16,
    node: &mut NODE_INFO,
) -> u16 {
    if body == WORLD {
        return surface::generate_random_nodes(
            rng,
            info,
            BIOLOGICAL_SCAN,
            NUM_EVIL_ONES,
            EVIL_ONE,
            which,
            node,
        );
    }
    0
}

pub fn pickup_energy(
    body: Body,
    which: u16,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if body != MOON {
        return false;
    }
    assert!(story.flag("UMGAH_BROADCASTERS") == 0 && which == 0);
    default::lander_report(report);
    lander.lander_takeoff();
    story.set_flag("UMGAH_BROADCASTERS", 1);
    story.set_flag("UMGAH_BROADCASTERS_ON_SHIP", 1);
    true
}

pub fn pickup_life(body: Body, retrieved: u32, story: &mut impl Story) -> bool {
    if body == WORLD {
        assert!(
            story.flag("SPATHI_CREATURES_ELIMINATED") == 0
                && story.flag("SPATHI_SHIELDED_SELVES") == 0
        );
        story.set_flag("SPATHI_CREATURES_EXAMINED", 1);
        if retrieved.count_ones() + 1 == u32::from(NUM_EVIL_ONES) {
            story.set_flag("SPATHI_CREATURES_ELIMINATED", 1);
        }
    }
    true
}

#[cfg(test)]
mod tests;
