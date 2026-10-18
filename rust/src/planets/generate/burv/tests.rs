//! The story gating of the Burvixese ruins and broadcasters, and the home
//! system's orbital, against recording fakes.

use super::{generate_energy, generate_orbital, pickup_energy, BurvScenes};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::home_contact::LanderTakeoff;
use crate::planets::generate::special::{Body, Orbit, Story};

const HOME: Body = Body::Planet(0);
const MOON: Body = Body::Moon { planet: 0, moon: 0 };

#[derive(Default)]
struct FakeReport(Vec<ReportDisposition>);

impl DiscoveryReportPort for FakeReport {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        self.0.push(disposition);
        Ok(true)
    }
}

#[derive(Default)]
struct FakeLander(u32);

impl LanderTakeoff for FakeLander {
    fn lander_takeoff(&mut self) {
        self.0 += 1;
    }
}

#[derive(Default)]
struct FakeScenes {
    ruins: u32,
    broadcasters: u32,
}

impl BurvScenes for FakeScenes {
    fn load_ruins_scene(&mut self) {
        self.ruins += 1;
    }
    fn load_broadcasters_scene(&mut self) {
        self.broadcasters += 1;
    }
}

fn node_count(body: Body, story: &FakeStory) -> u16 {
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = [1, 2, 3];
    let mut rng = RandomContext::with_seed(7);
    let mut node: NODE_INFO = unsafe { std::mem::zeroed() };
    generate_energy(&mut rng, &info, body, !0, &mut node, story)
}

#[test]
fn the_home_world_has_ruins_and_the_moon_one_artifact_until_taken() {
    let fresh = FakeStory::default();
    assert_eq!(node_count(HOME, &fresh), 16);
    assert_eq!(node_count(MOON, &fresh), 1);
}

#[test]
fn taken_broadcasters_leave_no_node() {
    let taken = FakeStory::with(&[("BURVIXESE_BROADCASTERS", 1)]);
    assert_eq!(node_count(MOON, &taken), 0);
    assert_eq!(node_count(HOME, &taken), 16);
}

#[test]
fn other_worlds_have_no_nodes() {
    let fresh = FakeStory::default();
    for body in [
        Body::Planet(1),
        Body::Moon { planet: 0, moon: 1 },
        Body::Moon { planet: 1, moon: 0 },
    ] {
        assert_eq!(node_count(body, &fresh), 0, "{body:?}");
    }
}

#[test]
fn ruins_only_cycle_the_report() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_energy(
        HOME,
        3,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 0);
    assert_eq!(story.flag("BURVIXESE_BROADCASTERS"), 0);
}

#[test]
fn the_broadcasters_are_taken_once_with_a_single_report() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(pickup_energy(MOON, 0, &mut story, &mut report, &mut lander));
    assert_eq!(report.0, [ReportDisposition::Once]);
    assert_eq!(lander.0, 1);
    assert_eq!(story.flag("BURVIXESE_BROADCASTERS"), 1);
    assert_eq!(story.flag("BURV_BROADCASTERS_ON_SHIP"), 1);
}

#[test]
fn the_broadcasters_cannot_be_taken_twice() {
    let mut story = FakeStory::with(&[("BURVIXESE_BROADCASTERS", 1)]);
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    let again = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pickup_energy(MOON, 0, &mut story, &mut report, &mut lander)
    }));
    assert!(again.is_err());
    assert!(report.0.is_empty());
}

#[test]
fn the_broadcasters_come_from_node_zero_only() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    let wrong = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pickup_energy(MOON, 1, &mut story, &mut report, &mut lander)
    }));
    assert!(wrong.is_err());
}

#[test]
fn other_worlds_pick_up_nothing() {
    for body in [
        Body::Planet(1),
        Body::Moon { planet: 0, moon: 1 },
        Body::Moon { planet: 1, moon: 0 },
    ] {
        let mut story = FakeStory::default();
        let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
        assert!(!pickup_energy(
            body,
            0,
            &mut story,
            &mut report,
            &mut lander
        ));
        assert!(report.0.is_empty());
        assert_eq!(lander.0, 0);
        assert_eq!(story.flag("BURVIXESE_BROADCASTERS"), 0);
    }
}

fn orbit_scenes(body: Body, story: &FakeStory) -> (u32, u32, (u8, u8), u32) {
    use crate::planets::calc::{AnalysisSite, StarDescription};
    let mut rng = RandomContext::with_seed(11);
    let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    sys_info.PlanetInfo.Weather = 9;
    sys_info.PlanetInfo.Tectonics = 9;
    let mut world: crate::c_bindings::global_state::PLANET_DESC = unsafe { std::mem::zeroed() };
    world.radius = 400;
    let plan_data: Vec<PlanetFrame> = vec![unsafe { std::mem::zeroed() }; 128];
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
    let mut scenes = FakeScenes::default();
    let mut loaded = 0;
    assert!(generate_orbital(
        &mut orbit,
        body,
        story,
        &mut scenes,
        || {
            loaded += 1;
        }
    ));
    let weather = (
        sys_info.PlanetInfo.Weather as u8,
        sys_info.PlanetInfo.Tectonics as u8,
    );
    (scenes.ruins, scenes.broadcasters, weather, loaded)
}

#[test]
fn orbital_scene_follows_the_world_and_the_broadcasters() {
    let fresh = FakeStory::default();
    let taken = FakeStory::with(&[("BURVIXESE_BROADCASTERS", 1)]);

    let (ruins, broadcasters, weather, loaded) = orbit_scenes(HOME, &fresh);
    assert_eq!((ruins, broadcasters, weather, loaded), (1, 0, (0, 0), 1));

    let (ruins, broadcasters, _, loaded) = orbit_scenes(MOON, &fresh);
    assert_eq!((ruins, broadcasters, loaded), (0, 1, 1));

    let (ruins, broadcasters, _, loaded) = orbit_scenes(MOON, &taken);
    assert_eq!((ruins, broadcasters, loaded), (0, 0, 1));

    let (ruins, broadcasters, _, loaded) = orbit_scenes(Body::Planet(1), &fresh);
    assert_eq!((ruins, broadcasters, loaded), (0, 0, 1));
}
