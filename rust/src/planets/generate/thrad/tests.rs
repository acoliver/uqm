//! The Thraddash systems' story gating, defence and conversation flow against
//! recording fakes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::{
    generate_energy, generate_orbital, pickup_energy, thraddash_defend, ThraddashSite,
    HELIX_TALK_FLAGS, THRADD_TALK_FLAGS,
};
use crate::battle::gendef::{AQUA_HELIX_DEFINED, THRADD_DEFINED};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{Body, Orbit, Story};

const WORLD: Body = Body::Planet(0);
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

fn nodes(star_index: u8, body: Body, story: &FakeStory) -> u16 {
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = [1, 2, 3];
    let mut rng = RandomContext::with_seed(7);
    let mut node: NODE_INFO = unsafe { std::mem::zeroed() };
    generate_energy(&mut rng, &info, star_index, body, !0, &mut node, story)
}

#[test]
fn the_home_has_ruins_and_aqua_helix_one_artifact_until_taken() {
    let fresh = FakeStory::default();
    assert_eq!(nodes(THRADD_DEFINED, WORLD, &fresh), 16);
    assert_eq!(nodes(AQUA_HELIX_DEFINED, WORLD, &fresh), 1);

    let taken = FakeStory::with(&[("AQUA_HELIX", 1)]);
    assert_eq!(nodes(AQUA_HELIX_DEFINED, WORLD, &taken), 0);
    assert_eq!(nodes(THRADD_DEFINED, WORLD, &taken), 16);
}

#[test]
fn other_worlds_and_systems_have_no_nodes() {
    let fresh = FakeStory::default();
    for star_index in [AQUA_HELIX_DEFINED, THRADD_DEFINED] {
        for body in [Body::Planet(1), MOON, Body::Moon { planet: 1, moon: 0 }] {
            assert_eq!(nodes(star_index, body, &fresh), 0, "{star_index} {body:?}");
        }
    }
    assert_eq!(nodes(0, WORLD, &fresh), 0);
}

#[test]
fn the_home_ruins_only_cycle_the_report() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_energy(
        THRADD_DEFINED,
        WORLD,
        5,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 0);
    assert_eq!(story.flag("AQUA_HELIX"), 0);
}

#[test]
fn the_helix_is_taken_once_and_unprotects_the_system() {
    let mut story = FakeStory::with(&[("HELIX_VISITS", 5)]);
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(pickup_energy(
        AQUA_HELIX_DEFINED,
        WORLD,
        0,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Once]);
    assert_eq!(lander.0, 1);
    for (flag, value) in [
        ("HELIX_VISITS", 0),
        ("AQUA_HELIX", 1),
        ("AQUA_HELIX_ON_SHIP", 1),
        ("HELIX_UNPROTECTED", 1),
    ] {
        assert_eq!(story.flag(flag), value, "{flag}");
    }
}

#[test]
fn the_helix_cannot_be_taken_twice_or_from_another_node() {
    for (taken, node) in [(1, 0), (0, 1)] {
        let mut story = FakeStory::with(&[("AQUA_HELIX", taken)]);
        let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pickup_energy(
                AQUA_HELIX_DEFINED,
                WORLD,
                node,
                &mut story,
                &mut report,
                &mut lander,
            )
        }));
        assert!(attempt.is_err());
        assert!(report.0.is_empty());
        assert_eq!(lander.0, 0);
    }
}

#[test]
fn other_worlds_pick_up_nothing() {
    for star_index in [AQUA_HELIX_DEFINED, THRADD_DEFINED] {
        for body in [Body::Planet(1), MOON] {
            let mut story = FakeStory::default();
            let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
            assert!(!pickup_energy(
                star_index,
                body,
                0,
                &mut story,
                &mut report,
                &mut lander
            ));
            assert!(report.0.is_empty());
            assert_eq!(lander.0, 0);
        }
    }
}

#[test]
fn the_thraddash_defend_their_home_and_helix_until_unprotected_or_on_mission() {
    for mission in 0..8u32 {
        let story = FakeStory::with(&[("THRADD_MISSION", mission)]);
        assert!(thraddash_defend(THRADD_DEFINED, &story));
        let under_way = (1..=3).contains(&mission);
        assert_eq!(
            thraddash_defend(AQUA_HELIX_DEFINED, &story),
            !under_way,
            "mission {mission}"
        );

        let unprotected = FakeStory::with(&[("THRADD_MISSION", mission), ("HELIX_UNPROTECTED", 1)]);
        assert!(!thraddash_defend(AQUA_HELIX_DEFINED, &unprotected));
        assert!(thraddash_defend(THRADD_DEFINED, &unprotected));
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Call {
    StartSphereTracking,
    NotifyAllClear,
    SaveIpGroups,
    ReinitIpGroupQ,
    CloneFleet,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    Talk,
    ReinitBuiltShipQ,
    LoadIpGroups,
    RepairSisBorder,
    LoadAquaScene,
    LoadRuinsScene(bool),
    LoadPlanet,
}

const IN_INTERPLANETARY: u16 = 4;

/// A site recording every host call; `unprotect_on_talk` makes the
/// conversation end the Thraddash defence of Aqua Helix.
struct Site {
    tracking: bool,
    activity: u16,
    activity_after_talk: Option<u16>,
    unprotected: Rc<Cell<bool>>,
    unprotect_on_talk: bool,
    calls: Rc<RefCell<Vec<Call>>>,
}

impl Site {
    fn new(tracking: bool, calls: &Rc<RefCell<Vec<Call>>>, unprotected: &Rc<Cell<bool>>) -> Self {
        Self {
            tracking,
            activity: IN_INTERPLANETARY,
            activity_after_talk: None,
            unprotected: Rc::clone(unprotected),
            unprotect_on_talk: false,
            calls: Rc::clone(calls),
        }
    }

    fn record(&self, call: Call) {
        self.calls.borrow_mut().push(call);
    }
}

impl HomeContactSite for Site {
    fn start_sphere_tracking(&mut self) -> bool {
        self.record(Call::StartSphereTracking);
        self.tracking
    }
    fn notify_others_all_clear(&mut self) {
        self.record(Call::NotifyAllClear);
    }
    fn save_ip_groups(&mut self) {
        self.record(Call::SaveIpGroups);
    }
    fn reinit_ip_group_q(&mut self) {
        self.record(Call::ReinitIpGroupQ);
    }
    fn built_ship_count(&self) -> u16 {
        0
    }
    fn clone_fleet_into_built_queue(&mut self) {
        self.record(Call::CloneFleet);
    }
    fn current_activity(&self) -> u16 {
        self.activity
    }
    fn set_current_activity(&mut self, activity: u16) {
        self.record(Call::SetActivity(activity));
        self.activity = activity;
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.record(Call::SetGlobalFlagsAndData(value));
    }
    fn talk(&mut self) {
        self.record(Call::Talk);
        if let Some(after) = self.activity_after_talk {
            self.activity = after;
        }
        if self.unprotect_on_talk {
            self.unprotected.set(true);
        }
    }
    fn reinit_built_ship_q(&mut self) {
        self.record(Call::ReinitBuiltShipQ);
    }
    fn load_ip_groups(&mut self) {
        self.record(Call::LoadIpGroups);
    }
    fn load_ruins_scene(&mut self, artifact_taken: bool) {
        self.record(Call::LoadRuinsScene(artifact_taken));
    }
}

impl ThraddashSite for Site {
    fn repair_sis_border(&mut self) {
        self.record(Call::RepairSisBorder);
    }
    fn load_aqua_scene(&mut self) {
        self.record(Call::LoadAquaScene);
    }
}

/// A story whose `HELIX_UNPROTECTED` follows a cell the site can change.
struct SharedStory {
    unprotected: Rc<Cell<bool>>,
    others: HashMap<&'static str, u32>,
}

impl Story for SharedStory {
    fn flag(&self, name: &'static str) -> u32 {
        if name == "HELIX_UNPROTECTED" {
            return u32::from(self.unprotected.get());
        }
        self.others.get(name).copied().unwrap_or(0)
    }
    fn set_flag(&mut self, name: &'static str, _value: u32) {
        panic!("unexpected write of {name}");
    }
    fn word(&self, name: &'static str) -> u32 {
        panic!("unexpected read of {name}");
    }
    fn set_word(&mut self, name: &'static str, _value: u32) {
        panic!("unexpected write of {name}");
    }
}

/// Run `generate_orbital` on `body` and return what the host was asked to do.
fn visit(
    star_index: u8,
    body: Body,
    others: &[(&'static str, u32)],
    configure: impl FnOnce(&mut Site),
) -> Vec<Call> {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let unprotected = Rc::new(Cell::new(false));
    let story = SharedStory {
        unprotected: Rc::clone(&unprotected),
        others: others.iter().copied().collect(),
    };
    let mut site = Site::new(false, &calls, &unprotected);
    configure(&mut site);

    let mut rng = RandomContext::with_seed(11);
    let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    let mut world: PLANET_DESC = unsafe { std::mem::zeroed() };
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
    let loaded = Rc::clone(&calls);
    assert!(generate_orbital(
        &mut orbit,
        body,
        star_index,
        &story,
        &mut site,
        || loaded.borrow_mut().push(Call::LoadPlanet)
    ));
    calls.take()
}

fn contact(flags: u8) -> Vec<Call> {
    vec![
        Call::StartSphereTracking,
        Call::NotifyAllClear,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
        Call::CloneFleet,
        Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY),
        Call::SetGlobalFlagsAndData(flags),
        Call::Talk,
    ]
}

fn world_restored() -> Vec<Call> {
    vec![
        Call::SetActivity(IN_INTERPLANETARY),
        Call::ReinitBuiltShipQ,
        Call::LoadIpGroups,
    ]
}

#[test]
fn the_defended_home_talks_and_then_ends_the_visit() {
    let calls = visit(THRADD_DEFINED, WORLD, &[], |site| site.tracking = true);
    let mut expected = contact(THRADD_TALK_FLAGS);
    expected.extend(world_restored());
    assert_eq!(calls, expected);
}

#[test]
fn defended_aqua_helix_talks_with_its_own_flags_and_ends_the_visit() {
    let calls = visit(AQUA_HELIX_DEFINED, WORLD, &[], |site| site.tracking = true);
    let mut expected = contact(HELIX_TALK_FLAGS);
    expected.extend(world_restored());
    assert_eq!(calls, expected);
}

#[test]
fn an_aborted_or_loaded_conversation_leaves_everything_as_it_is() {
    for star_index in [AQUA_HELIX_DEFINED, THRADD_DEFINED] {
        for flag in [CHECK_ABORT, CHECK_LOAD] {
            let calls = visit(star_index, WORLD, &[], |site| {
                site.tracking = true;
                site.activity_after_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | flag);
            });
            let flags = if star_index == THRADD_DEFINED {
                THRADD_TALK_FLAGS
            } else {
                HELIX_TALK_FLAGS
            };
            assert_eq!(calls, contact(flags), "{star_index} {flag:#x}");
        }
    }
}

#[test]
fn a_conversation_that_unprotects_the_helix_repairs_the_border_and_lands() {
    let calls = visit(AQUA_HELIX_DEFINED, WORLD, &[], |site| {
        site.tracking = true;
        site.unprotect_on_talk = true;
    });
    let mut expected = contact(HELIX_TALK_FLAGS);
    expected.extend(world_restored());
    expected.extend([Call::RepairSisBorder, Call::LoadAquaScene, Call::LoadPlanet]);
    assert_eq!(calls, expected);
}

#[test]
fn an_undefended_helix_is_not_talked_over() {
    let on_mission = [("THRADD_MISSION", 2)];
    let calls = visit(AQUA_HELIX_DEFINED, WORLD, &on_mission, |site| {
        site.tracking = true;
    });
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadAquaScene,
            Call::LoadPlanet
        ]
    );
}

#[test]
fn without_sphere_tracking_the_scene_is_loaded_before_the_default_orbital() {
    let calls = visit(AQUA_HELIX_DEFINED, WORLD, &[], |_| {});
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadAquaScene,
            Call::LoadPlanet
        ]
    );
    let calls = visit(THRADD_DEFINED, WORLD, &[], |_| {});
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadRuinsScene(false),
            Call::LoadPlanet
        ]
    );
}

#[test]
fn a_taken_helix_loads_no_scene() {
    let calls = visit(AQUA_HELIX_DEFINED, WORLD, &[("AQUA_HELIX", 1)], |_| {});
    assert_eq!(calls, [Call::StartSphereTracking, Call::LoadPlanet]);
}

#[test]
fn other_worlds_go_straight_to_the_default_orbital() {
    for star_index in [AQUA_HELIX_DEFINED, THRADD_DEFINED] {
        for body in [Body::Planet(1), MOON] {
            let calls = visit(star_index, body, &[], |site| site.tracking = true);
            assert_eq!(calls, [Call::LoadPlanet], "{star_index} {body:?}");
        }
    }
}
