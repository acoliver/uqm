//! The Androsynth ruins' report story and climate against fakes.

use super::{
    generate_energy, generate_orbital, generate_planets, pickup_energy, AndrosynthSite, WORLD,
};
use crate::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::calc::{AnalysisSite, StarDescription};
use crate::planets::generate::consts::EARTH_RADIUS;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::special::{Body, Orbit, Story, TELLURIC_WORLD};
use crate::planets::generate::surface::ENERGY_SCAN;

#[derive(Default)]
struct FakeReport(Vec<ReportDisposition>);

impl DiscoveryReportPort for FakeReport {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        self.0.push(disposition);
        Ok(true)
    }
}

#[derive(Default)]
struct Site {
    stories: Vec<u32>,
}

impl AndrosynthSite for Site {
    fn load_ruins_story(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        visits: u32,
    ) {
        self.stories.push(visits);
    }
}

const OTHER_BODIES: [Body; 3] = [
    Body::Planet(0),
    Body::Moon { planet: 1, moon: 0 },
    Body::Moon { planet: 0, moon: 0 },
];

fn orbital(body: Body, energy_mask: u32) -> (Site, SystemInfo) {
    let mut site = Site::default();
    let mut rng = RandomContext::with_seed(11);
    let mut sys_info: SystemInfo = unsafe { std::mem::zeroed() };
    sys_info.planet_info.scan_retrieve_mask[ENERGY_SCAN] = energy_mask;
    let mut world: PlanetDesc = unsafe { std::mem::zeroed() };
    world.radius = 400;
    let plan_data: Vec<PlanetFrame> = vec![unsafe { std::mem::zeroed() }; 128];
    let mut loaded = 0;
    {
        let mut orbit = Orbit {
            rng: &mut rng,
            plan_data: &plan_data,
            star: StarDescription {
                size: 0,
                star_type: 0,
            },
            site: AnalysisSite {
                parent_radius: None,
                is_orbit_entry: true,
            },
            sys_info: &mut sys_info,
            world: &world,
        };
        assert!(crate::planets::generate::testing::with_orbit(
            &mut orbit,
            body,
            |system| generate_orbital(
                system,
                body,
                &mut crate::planets::generate::testing::Effects {
                    story: (),
                    site: &mut site,
                    report: (),
                    lander: (),
                    load: Some(|| loaded += 1)
                }
            )
        ));
    }
    assert_eq!(
        loaded, 1,
        "the planet is loaded once by the default orbital"
    );
    (site, sys_info)
}

#[test]
fn the_story_resumes_after_the_ruins_already_visited() {
    // Only the upper 16 bits count; the lower ones are the nodes on the map.
    for (mask, visits) in [
        (0u32, 0),
        (0x0000_ffff, 0),
        (0x0001_0000, 1),
        (0x8000_0000, 1),
        (0x0005_0003, 2),
        (0xffff_ffff, 16),
    ] {
        let (site, _) = orbital(WORLD, mask);
        assert_eq!(site.stories, [visits], "{mask:08x}");
    }
}

#[test]
fn the_world_has_a_fixed_climate_and_other_bodies_are_left_alone() {
    let (_, info) = orbital(WORLD, 0);
    let planet = &info.planet_info;
    assert_eq!(planet.atmo_density, 72);
    assert_eq!(
        (planet.temperature, planet.weather, planet.tectonics),
        (28, 1, 1)
    );
    for body in OTHER_BODIES {
        let (site, info) = orbital(body, 0);
        assert!(site.stories.is_empty(), "{body:?}");
        let planet = &info.planet_info;
        assert_ne!(
            (planet.temperature, planet.weather, planet.tectonics),
            (28, 1, 1),
            "{body:?}"
        );
    }
}

#[test]
fn planet_one_is_the_telluric_world() {
    let mut rng = RandomContext::with_seed(5);
    let plan_data: Vec<PlanetFrame> = vec![unsafe { std::mem::zeroed() }; 128];
    let mut planets: Vec<PlanetDesc> = vec![unsafe { std::mem::zeroed() }; 16];
    // A frame table of zeros has the C layout drawn from the same RNG.
    let drawn = generate_planets(&mut rng, &plan_data, 0, 2, &mut planets);
    assert!(drawn >= 2, "the layout always has a planet 1 to place");
    assert_eq!(planets[1].data_index, TELLURIC_WORLD);
    assert_eq!(
        i32::from(planets[1].radius),
        i32::from(EARTH_RADIUS) * 204 / 100
    );
}

#[test]
fn only_planet_one_has_ruins() {
    let mut info: SystemInfo = unsafe { std::mem::zeroed() };
    info.planet_info.scan_seed = [1, 2, 3];
    let count = |body| {
        let mut rng = RandomContext::with_seed(7);
        let mut node: NodeInfo = unsafe { std::mem::zeroed() };
        generate_energy(&mut rng, &info, body, !0, &mut node)
    };
    assert_eq!(count(WORLD), 16);
    for body in OTHER_BODIES {
        assert_eq!(count(body), 0, "{body:?}");
    }
}

#[test]
fn a_new_ruin_is_marked_and_reported_and_a_visited_one_is_not() {
    let mut info = unsafe { std::mem::zeroed::<SystemInfo>() }.planet_info;
    let mut story = FakeStory::default();
    let mut report = FakeReport::default();

    assert!(!crate::planets::generate::testing::with_info(
        &mut info,
        |system| pickup_energy(
            system,
            WORLD,
            3,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut (),
                report: &mut report,
                lander: (),
                load: Some(|| {})
            }
        )
    ));
    assert_eq!(info.scan_retrieve_mask[ENERGY_SCAN], 1 << 19);
    assert_eq!(story.flag(StoryKey::PLANETARY_CHANGE), 1);
    assert_eq!(report.0, [ReportDisposition::Cycle]);

    // The same ruin again changes nothing.
    story.set_flag(StoryKey::PLANETARY_CHANGE, 0);
    assert!(!crate::planets::generate::testing::with_info(
        &mut info,
        |system| pickup_energy(
            system,
            WORLD,
            3,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut (),
                report: &mut report,
                lander: (),
                load: Some(|| {})
            }
        )
    ));
    assert_eq!(info.scan_retrieve_mask[ENERGY_SCAN], 1 << 19);
    assert_eq!(story.flag(StoryKey::PLANETARY_CHANGE), 0);
    assert_eq!(report.0.len(), 1);

    // The map bit of the node is not the visited bit.
    info.scan_retrieve_mask[ENERGY_SCAN] = 1 << 3;
    assert!(!crate::planets::generate::testing::with_info(
        &mut info,
        |system| pickup_energy(
            system,
            WORLD,
            3,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut (),
                report: &mut report,
                lander: (),
                load: Some(|| {})
            }
        )
    ));
    assert_eq!(info.scan_retrieve_mask[ENERGY_SCAN], (1 << 3) | (1 << 19));
    assert_eq!(report.0.len(), 2);
}

#[test]
fn other_bodies_report_nothing() {
    for body in OTHER_BODIES {
        let mut info = unsafe { std::mem::zeroed::<SystemInfo>() }.planet_info;
        let mut story = FakeStory::default();
        let mut report = FakeReport::default();
        assert!(!crate::planets::generate::testing::with_info(
            &mut info,
            |system| pickup_energy(
                system,
                body,
                0,
                &mut crate::planets::generate::testing::Effects {
                    story: &mut story,
                    site: &mut (),
                    report: &mut report,
                    lander: (),
                    load: Some(|| {})
                }
            )
        ));
        assert_eq!(info.scan_retrieve_mask[ENERGY_SCAN], 0);
        assert!(report.0.is_empty());
        assert_eq!(story.flag(StoryKey::PLANETARY_CHANGE), 0);
    }
}

#[test]
#[should_panic(expected = "has no visited bit")]
fn a_ruin_past_the_sixteenth_is_a_broken_invariant() {
    let mut info = unsafe { std::mem::zeroed::<SystemInfo>() }.planet_info;
    let mut story = FakeStory::default();
    let mut report = FakeReport::default();
    crate::planets::generate::testing::with_info(&mut info, |system| {
        pickup_energy(
            system,
            WORLD,
            16,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut (),
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    });
}
