//! The Orz systems' story gating and conversation flow against recording
//! fakes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{
    generate_energy, generate_orbital, pickup_energy, OrzSite, ORZ_TALK_FLAGS, TAALO_TALK_FLAGS,
};
use crate::battle::gendef::{ORZ_DEFINED, TAALO_PROTECTOR_DEFINED};
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
const DEVICE: Body = Body::Moon { planet: 1, moon: 2 };
const OTHER_BODIES: [Body; 5] = [
    Body::Planet(1),
    Body::Moon { planet: 0, moon: 0 },
    Body::Moon { planet: 0, moon: 2 },
    Body::Moon { planet: 1, moon: 0 },
    Body::Moon { planet: 2, moon: 2 },
];

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
fn the_home_has_ruins_and_the_device_one_artifact_until_taken() {
    let fresh = FakeStory::default();
    assert_eq!(nodes(ORZ_DEFINED, WORLD, &fresh), 16);
    assert_eq!(nodes(TAALO_PROTECTOR_DEFINED, DEVICE, &fresh), 1);

    let taken = FakeStory::with(&[("TAALO_PROTECTOR", 1)]);
    assert_eq!(nodes(TAALO_PROTECTOR_DEFINED, DEVICE, &taken), 0);
    assert_eq!(nodes(ORZ_DEFINED, WORLD, &taken), 16);
}

#[test]
fn other_worlds_and_systems_have_no_nodes() {
    let fresh = FakeStory::default();
    for body in OTHER_BODIES {
        assert_eq!(nodes(ORZ_DEFINED, body, &fresh), 0, "orz {body:?}");
        assert_eq!(
            nodes(TAALO_PROTECTOR_DEFINED, body, &fresh),
            0,
            "taalo {body:?}"
        );
    }
    assert_eq!(nodes(ORZ_DEFINED, DEVICE, &fresh), 0);
    assert_eq!(nodes(TAALO_PROTECTOR_DEFINED, WORLD, &fresh), 0);
    assert_eq!(nodes(0, WORLD, &fresh), 0);
    assert_eq!(nodes(0, DEVICE, &fresh), 0);
}

#[test]
fn the_home_ruins_only_cycle_the_report() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_energy(
        ORZ_DEFINED,
        WORLD,
        5,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 0);
    assert_eq!(story.flag("TAALO_PROTECTOR"), 0);
}

#[test]
fn the_device_is_taken_once() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(pickup_energy(
        TAALO_PROTECTOR_DEFINED,
        DEVICE,
        0,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Once]);
    assert_eq!(lander.0, 1);
    assert_eq!(story.flag("TAALO_PROTECTOR"), 1);
    assert_eq!(story.flag("TAALO_PROTECTOR_ON_SHIP"), 1);
}

#[test]
fn the_device_cannot_be_taken_twice_or_from_another_node() {
    for (taken, node) in [(1, 0), (0, 1)] {
        let mut story = FakeStory::with(&[("TAALO_PROTECTOR", taken)]);
        let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pickup_energy(
                TAALO_PROTECTOR_DEFINED,
                DEVICE,
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
fn other_worlds_and_systems_pick_up_nothing() {
    let cases = OTHER_BODIES
        .into_iter()
        .flat_map(|body| [(ORZ_DEFINED, body), (TAALO_PROTECTOR_DEFINED, body)])
        .chain([
            (ORZ_DEFINED, DEVICE),
            (TAALO_PROTECTOR_DEFINED, WORLD),
            (0, WORLD),
            (0, DEVICE),
        ]);
    for (star_index, body) in cases {
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

#[derive(Debug, PartialEq, Eq)]
enum Call {
    StartSphereTracking,
    NotifyAllClear,
    SaveIpGroups,
    ReinitIpGroupQ,
    CloneFleet,
    CloneEscort,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    Talk,
    ReinitBuiltShipQ,
    LoadIpGroups,
    RepairSisBorder,
    LoadTaaloScene,
    LoadRuinsScene(bool),
    LoadPlanet,
}

const IN_INTERPLANETARY: u16 = 4;

/// Game state shared between the story handed to the hook and the site, so a
/// conversation can change it.
#[derive(Clone, Default)]
struct SharedStory(Rc<RefCell<HashMap<&'static str, u32>>>);

impl SharedStory {
    fn with(flags: &[(&'static str, u32)]) -> Self {
        Self(Rc::new(RefCell::new(flags.iter().copied().collect())))
    }
}

impl Story for SharedStory {
    fn flag(&self, name: &'static str) -> u32 {
        self.0.borrow().get(name).copied().unwrap_or(0)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        self.0.borrow_mut().insert(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        panic!("unexpected read of {name}");
    }
    fn set_word(&mut self, name: &'static str, _value: u32) {
        panic!("unexpected write of {name}");
    }
}

/// A site recording every host call. What the conversation does is
/// configured: the activity it leaves, the ships that survive it and whether
/// it unprotects Taalo.
struct Site {
    tracking: bool,
    activity: u16,
    activity_after_talk: Option<u16>,
    survivors_after_talk: u16,
    built: u16,
    unprotect_on_talk: bool,
    story: SharedStory,
    calls: Rc<RefCell<Vec<Call>>>,
}

impl Site {
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
        self.built
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
        self.built = self.survivors_after_talk;
        if self.unprotect_on_talk {
            self.story.set_flag("TAALO_UNPROTECTED", 1);
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

impl OrzSite for Site {
    fn clone_escort_into_built_queue(&mut self) {
        self.record(Call::CloneEscort);
    }
    fn repair_sis_border(&mut self) {
        self.record(Call::RepairSisBorder);
    }
    fn load_taalo_device_scene(&mut self) {
        self.record(Call::LoadTaaloScene);
    }
}

/// Run `generate_orbital` on `body`; the host calls it made and the story
/// afterwards.
fn visit(
    star_index: u8,
    body: Body,
    flags: &[(&'static str, u32)],
    configure: impl FnOnce(&mut Site),
) -> (Vec<Call>, SharedStory) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut story = SharedStory::with(flags);
    let mut site = Site {
        tracking: false,
        activity: IN_INTERPLANETARY,
        activity_after_talk: None,
        survivors_after_talk: 0,
        built: 0,
        unprotect_on_talk: false,
        story: story.clone(),
        calls: Rc::clone(&calls),
    };
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
        &mut story,
        &mut site,
        || loaded.borrow_mut().push(Call::LoadPlanet)
    ));
    (calls.take(), story)
}

fn contact(star_index: u8) -> Vec<Call> {
    let mut calls = vec![
        Call::NotifyAllClear,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
    ];
    if star_index == ORZ_DEFINED {
        calls.push(Call::CloneFleet);
        calls.push(Call::SetGlobalFlagsAndData(ORZ_TALK_FLAGS));
    } else {
        calls.extend((0..14).map(|_| Call::CloneEscort));
        calls.push(Call::SetGlobalFlagsAndData(TAALO_TALK_FLAGS));
    }
    calls.push(Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY));
    calls.push(Call::Talk);
    calls
}

fn world_restored() -> Vec<Call> {
    vec![
        Call::SetActivity(IN_INTERPLANETARY),
        Call::ReinitBuiltShipQ,
        Call::LoadIpGroups,
    ]
}

fn track(calls: Vec<Call>) -> Vec<Call> {
    let mut all = vec![Call::StartSphereTracking];
    all.extend(calls);
    all
}

#[test]
fn the_orz_home_talks_with_a_fleet_and_after_a_defeat_repairs_the_border_and_lands() {
    let (calls, story) = visit(ORZ_DEFINED, WORLD, &[], |site| site.tracking = true);
    let mut expected = track(contact(ORZ_DEFINED));
    expected.extend(world_restored());
    expected.extend([
        Call::RepairSisBorder,
        Call::LoadRuinsScene(false),
        Call::LoadPlanet,
    ]);
    assert_eq!(calls, expected);
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 1);
}

#[test]
fn surviving_orz_at_their_home_end_the_visit() {
    let (calls, story) = visit(ORZ_DEFINED, WORLD, &[], |site| {
        site.tracking = true;
        site.survivors_after_talk = 3;
        site.unprotect_on_talk = true;
    });
    let mut expected = track(contact(ORZ_DEFINED));
    expected.extend(world_restored());
    assert_eq!(calls, expected);
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 1);
}

#[test]
fn an_aborted_or_loaded_conversation_leaves_everything_as_it_is() {
    for star_index in [ORZ_DEFINED, TAALO_PROTECTOR_DEFINED] {
        let body = if star_index == ORZ_DEFINED {
            WORLD
        } else {
            DEVICE
        };
        for flag in [CHECK_ABORT, CHECK_LOAD] {
            let (calls, _) = visit(star_index, body, &[], |site| {
                site.tracking = true;
                site.survivors_after_talk = 1;
                site.activity_after_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | flag);
            });
            assert_eq!(calls, track(contact(star_index)), "{star_index} {flag:#x}");
        }
    }
}

#[test]
fn defended_taalo_fights_the_squadron_and_a_victory_unprotects_it() {
    let (calls, story) = visit(TAALO_PROTECTOR_DEFINED, DEVICE, &[], |site| {
        site.tracking = true;
        site.unprotect_on_talk = true;
        site.survivors_after_talk = 2;
    });
    // The squadron's survivors do not matter once the conversation unprotected
    // Taalo.
    let mut expected = track(contact(TAALO_PROTECTOR_DEFINED));
    expected.extend(world_restored());
    expected.extend([
        Call::RepairSisBorder,
        Call::LoadTaaloScene,
        Call::LoadPlanet,
    ]);
    assert_eq!(calls, expected);
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 1);
}

#[test]
fn a_surviving_taalo_squadron_ends_the_visit_and_leaves_taalo_protected() {
    let (calls, story) = visit(TAALO_PROTECTOR_DEFINED, DEVICE, &[], |site| {
        site.tracking = true;
        site.survivors_after_talk = 2;
    });
    let mut expected = track(contact(TAALO_PROTECTOR_DEFINED));
    expected.extend(world_restored());
    assert_eq!(calls, expected);
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 0);
}

#[test]
fn an_unprotected_taalo_is_not_talked_over() {
    let (calls, story) = visit(
        TAALO_PROTECTOR_DEFINED,
        DEVICE,
        &[("TAALO_UNPROTECTED", 1)],
        |site| site.tracking = true,
    );
    assert_eq!(calls, [Call::LoadTaaloScene, Call::LoadPlanet]);
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 1);
}

#[test]
fn without_sphere_tracking_the_scene_is_loaded_before_the_default_orbital() {
    let (calls, story) = visit(ORZ_DEFINED, WORLD, &[], |_| {});
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadRuinsScene(false),
            Call::LoadPlanet
        ]
    );
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 1);

    let (calls, story) = visit(TAALO_PROTECTOR_DEFINED, DEVICE, &[], |_| {});
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadTaaloScene,
            Call::LoadPlanet
        ]
    );
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 1);
}

#[test]
fn a_taken_device_loads_no_scene_and_changes_no_state() {
    let (calls, story) = visit(
        TAALO_PROTECTOR_DEFINED,
        DEVICE,
        &[("TAALO_PROTECTOR", 1)],
        |site| site.tracking = true,
    );
    assert_eq!(calls, [Call::LoadPlanet]);
    assert_eq!(story.flag("TAALO_UNPROTECTED"), 0);
}

#[test]
fn other_worlds_and_systems_go_straight_to_the_default_orbital() {
    let cases = OTHER_BODIES
        .into_iter()
        .flat_map(|body| [(ORZ_DEFINED, body), (TAALO_PROTECTOR_DEFINED, body)])
        .chain([(ORZ_DEFINED, DEVICE), (TAALO_PROTECTOR_DEFINED, WORLD)]);
    for (star_index, body) in cases {
        let (calls, story) = visit(star_index, body, &[], |site| site.tracking = true);
        assert_eq!(calls, [Call::LoadPlanet], "{star_index} {body:?}");
        assert_eq!(story.flag("TAALO_UNPROTECTED"), 0);
    }
}
