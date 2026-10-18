//! The Utwig systems' story gating, guard conversations and pickup against
//! recording fakes.

use std::cell::RefCell;
use std::rc::Rc;

use super::{
    generate_energy, generate_orbital, generate_planets, init_npcs, pickup_energy, UtwigSite,
    BOMB_MOON, BOMB_TALK_FLAGS, HOME, HOME_TALK_FLAGS,
};
use crate::battle::gendef::{BOMB_DEFINED, UTWIG_DEFINED};
use crate::c_bindings::global_state::{PlanetFrame, IP_GROUP, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::consts::EARTH_RADIUS;
use crate::planets::generate::default::Npcs;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{Body, Orbit, Story, WATER_WORLD};

const MOON: Body = Body::Moon { planet: 0, moon: 0 };
const IN_INTERPLANETARY: u16 = 4;

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
    StartDruugeTracking,
    CloneDruuge,
    TalkDruuge,
    RepairSisBorder,
    LoadScene(bool),
    LoadPlanet,
    InitIpGroups,
    ClearBattleGroupRef,
    BuildGroups,
    ReinitNpcBuiltShipQ,
}

/// A site recording every host call.
struct Site {
    utwig_tracking: bool,
    druuge_tracking: bool,
    built_ships: u16,
    druuge_survivors: bool,
    stored_groups: bool,
    activity: u16,
    activity_after_utwig_talk: Option<u16>,
    activity_after_druuge_talk: Option<u16>,
    calls: Rc<RefCell<Vec<Call>>>,
}

impl Site {
    fn new(calls: &Rc<RefCell<Vec<Call>>>) -> Self {
        Self {
            utwig_tracking: false,
            druuge_tracking: false,
            built_ships: 0,
            druuge_survivors: false,
            stored_groups: false,
            activity: IN_INTERPLANETARY,
            activity_after_utwig_talk: None,
            activity_after_druuge_talk: None,
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
        self.utwig_tracking
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
        self.built_ships
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
        if let Some(after) = self.activity_after_utwig_talk {
            self.activity = after;
        }
    }
    fn reinit_built_ship_q(&mut self) {
        self.record(Call::ReinitBuiltShipQ);
    }
    fn load_ip_groups(&mut self) {
        self.record(Call::LoadIpGroups);
    }
    fn load_ruins_scene(&mut self, _artifact_taken: bool) {
        panic!("the Utwig load their own scene");
    }
}

impl UtwigSite for Site {
    fn start_druuge_tracking(&mut self) -> bool {
        self.record(Call::StartDruugeTracking);
        self.druuge_tracking
    }
    fn clone_druuge_into_built_queue(&mut self) {
        self.record(Call::CloneDruuge);
    }
    fn talk_druuge(&mut self) {
        self.record(Call::TalkDruuge);
        if let Some(after) = self.activity_after_druuge_talk {
            self.activity = after;
        }
    }
    fn built_queue_has_ships(&self) -> bool {
        self.druuge_survivors
    }
    fn repair_sis_border(&mut self) {
        self.record(Call::RepairSisBorder);
    }
    fn load_scene(&mut self, bomb: bool) {
        self.record(Call::LoadScene(bomb));
    }
}

impl Npcs for Site {
    fn init_ip_groups(&mut self) -> bool {
        self.record(Call::InitIpGroups);
        self.stored_groups
    }
    fn clear_battle_group_ref(&mut self) {
        self.record(Call::ClearBattleGroupRef);
    }
    fn build_groups(&mut self) {
        self.record(Call::BuildGroups);
    }
    fn load_ip_groups(&mut self) {
        self.record(Call::LoadIpGroups);
    }
    fn save_ip_groups(&mut self) {
        self.record(Call::SaveIpGroups);
    }
    fn reinit_npc_built_ship_q(&mut self) {
        self.record(Call::ReinitNpcBuiltShipQ);
    }
    fn reinit_ip_group_q(&mut self) {
        self.record(Call::ReinitIpGroupQ);
    }
    fn yehat_civil_war(&self) -> bool {
        false
    }
    fn encounter_race(&self) -> i16 {
        -1
    }
    fn for_each_ip_group(&mut self, _visit: &mut dyn FnMut(&mut IP_GROUP)) {}
}

fn nodes(star_index: u8, body: Body, story: &FakeStory) -> u16 {
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = [1, 2, 3];
    let mut rng = RandomContext::with_seed(7);
    let mut node: NODE_INFO = unsafe { std::mem::zeroed() };
    generate_energy(&mut rng, &info, star_index, body, !0, &mut node, story)
}

#[test]
fn the_home_has_ruins_and_the_bomb_moon_one_artifact_until_taken() {
    let fresh = FakeStory::default();
    assert_eq!(nodes(UTWIG_DEFINED, HOME, &fresh), 16);
    assert_eq!(nodes(BOMB_DEFINED, BOMB_MOON, &fresh), 1);

    let taken = FakeStory::with(&[("UTWIG_BOMB", 1)]);
    assert_eq!(nodes(BOMB_DEFINED, BOMB_MOON, &taken), 0);
    assert_eq!(nodes(UTWIG_DEFINED, HOME, &taken), 16);
}

#[test]
fn other_worlds_and_systems_have_no_nodes() {
    let fresh = FakeStory::default();
    let others = [
        Body::Planet(1),
        MOON,
        Body::Moon { planet: 5, moon: 0 },
        Body::Moon { planet: 4, moon: 1 },
        Body::Planet(5),
    ];
    for body in others {
        assert_eq!(nodes(UTWIG_DEFINED, body, &fresh), 0, "utwig {body:?}");
        assert_eq!(nodes(BOMB_DEFINED, body, &fresh), 0, "bomb {body:?}");
    }
    assert_eq!(nodes(UTWIG_DEFINED, BOMB_MOON, &fresh), 0);
    assert_eq!(nodes(BOMB_DEFINED, HOME, &fresh), 0);
    assert_eq!(nodes(0, HOME, &fresh), 0);
    assert_eq!(nodes(0, BOMB_MOON, &fresh), 0);
}

#[test]
fn the_home_ruins_only_cycle_the_report() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_energy(
        UTWIG_DEFINED,
        HOME,
        5,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 0);
    assert_eq!(story.flag("UTWIG_BOMB"), 0);
}

#[test]
fn the_bomb_is_taken_once_and_wins_the_druuge_over() {
    let mut story = FakeStory::with(&[
        ("DRUUGE_MANNER", 0),
        ("DRUUGE_VISITS", 3),
        ("DRUUGE_HOME_VISITS", 4),
    ]);
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(pickup_energy(
        BOMB_DEFINED,
        BOMB_MOON,
        0,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Once]);
    assert_eq!(lander.0, 1);
    for (flag, value) in [
        ("UTWIG_BOMB", 1),
        ("UTWIG_BOMB_ON_SHIP", 1),
        ("DRUUGE_MANNER", 1),
        ("DRUUGE_VISITS", 0),
        ("DRUUGE_HOME_VISITS", 0),
    ] {
        assert_eq!(story.flag(flag), value, "{flag}");
    }
}

#[test]
fn the_bomb_cannot_be_taken_twice_or_from_another_node() {
    for (taken, node) in [(1, 0), (0, 1)] {
        let mut story = FakeStory::with(&[("UTWIG_BOMB", taken)]);
        let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pickup_energy(
                BOMB_DEFINED,
                BOMB_MOON,
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
    for (star_index, body) in [
        (UTWIG_DEFINED, Body::Planet(1)),
        (UTWIG_DEFINED, BOMB_MOON),
        (BOMB_DEFINED, HOME),
        (BOMB_DEFINED, Body::Moon { planet: 5, moon: 0 }),
        (0, HOME),
    ] {
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
        assert_eq!(story.flag("UTWIG_BOMB"), 0);
    }
}

#[test]
fn the_bomb_system_stays_empty_while_the_bomb_is_there() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut site = Site::new(&calls);
    assert!(init_npcs(BOMB_DEFINED, &FakeStory::default(), &mut site));
    assert_eq!(calls.take(), [Call::ReinitIpGroupQ]);
}

#[test]
fn every_other_case_starts_the_default_groups() {
    let taken = FakeStory::with(&[("UTWIG_BOMB", 1)]);
    let fresh = FakeStory::default();
    for (star_index, story, stored) in [
        (BOMB_DEFINED, &taken, false),
        (UTWIG_DEFINED, &fresh, false),
        (0, &fresh, false),
        (BOMB_DEFINED, &taken, true),
    ] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut site = Site::new(&calls);
        site.stored_groups = stored;
        assert!(init_npcs(star_index, story, &mut site));
        let expected = if stored {
            vec![Call::InitIpGroups]
        } else {
            vec![
                Call::InitIpGroups,
                Call::ClearBattleGroupRef,
                Call::BuildGroups,
            ]
        };
        assert_eq!(calls.take(), expected, "{star_index} {stored}");
    }
}

#[test]
fn a_bomb_system_with_ships_queued_is_a_broken_invariant() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut site = Site::new(&calls);
    site.built_ships = 1;
    let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        init_npcs(BOMB_DEFINED, &FakeStory::default(), &mut site)
    }));
    assert!(attempt.is_err());
}

/// What the Utwig talk looks like to the host, with `flags` in
/// `GLOBAL_FLAGS_AND_DATA`.
fn utwig_contact(flags: u8) -> Vec<Call> {
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

/// What the Druuge guard talk looks like to the host.
fn druuge_contact() -> Vec<Call> {
    let mut calls = vec![
        Call::StartDruugeTracking,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
    ];
    calls.extend((0..5).map(|_| Call::CloneDruuge));
    calls.extend([
        Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY),
        Call::SetGlobalFlagsAndData(BOMB_TALK_FLAGS),
        Call::TalkDruuge,
    ]);
    calls
}

/// Run `generate_orbital` on `body`; the host calls, the story afterwards and
/// the climate the orbital left.
fn visit(
    star_index: u8,
    body: Body,
    flags: &[(&'static str, u32)],
    configure: impl FnOnce(&mut Site),
) -> (Vec<Call>, FakeStory, (u16, u16)) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut story = FakeStory::with(flags);
    let mut site = Site::new(&calls);
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
    let climate = (
        orbit.sys_info.PlanetInfo.Weather,
        orbit.sys_info.PlanetInfo.Tectonics,
    );
    (calls.take(), story, climate)
}

#[test]
fn the_utwig_talk_at_their_home_ends_the_visit() {
    let (calls, _, _) = visit(UTWIG_DEFINED, HOME, &[], |site| site.utwig_tracking = true);
    let mut expected = utwig_contact(HOME_TALK_FLAGS);
    expected.extend(world_restored());
    assert_eq!(calls, expected);
}

#[test]
fn the_utwig_talk_at_the_bomb_until_the_player_has_the_ultron() {
    let (calls, _, _) = visit(BOMB_DEFINED, BOMB_MOON, &[], |site| {
        site.utwig_tracking = true;
    });
    let mut expected = utwig_contact(BOMB_TALK_FLAGS);
    expected.extend(world_restored());
    assert_eq!(calls, expected);

    // With the Ultron the Utwig are not even tracked; the Druuge are next.
    let (calls, _, _) = visit(
        BOMB_DEFINED,
        BOMB_MOON,
        &[("UTWIG_HAVE_ULTRON", 1)],
        |site| {
            site.utwig_tracking = true;
        },
    );
    assert_eq!(
        calls,
        [
            Call::StartDruugeTracking,
            Call::LoadScene(true),
            Call::LoadPlanet
        ]
    );
}

#[test]
fn an_aborted_or_loaded_utwig_talk_leaves_everything_as_it_is() {
    for (star_index, body, flags) in [
        (UTWIG_DEFINED, HOME, HOME_TALK_FLAGS),
        (BOMB_DEFINED, BOMB_MOON, BOMB_TALK_FLAGS),
    ] {
        for flag in [CHECK_ABORT, CHECK_LOAD] {
            let (calls, story, _) = visit(star_index, body, &[], |site| {
                site.utwig_tracking = true;
                site.activity_after_utwig_talk =
                    Some(IN_INTERPLANETARY | START_INTERPLANETARY | flag);
            });
            assert_eq!(calls, utwig_contact(flags), "{star_index} {flag:#x}");
            assert_eq!(story.flag("BOMB_UNPROTECTED"), 0);
        }
    }
}

#[test]
fn the_druuge_guard_the_bomb_and_a_defeated_guard_unprotects_it() {
    let (calls, story, _) = visit(BOMB_DEFINED, BOMB_MOON, &[], |site| {
        site.druuge_tracking = true;
    });
    let mut expected = vec![Call::StartSphereTracking];
    expected.extend(druuge_contact());
    expected.extend(world_restored());
    expected.extend([
        Call::RepairSisBorder,
        Call::LoadScene(true),
        Call::LoadPlanet,
    ]);
    assert_eq!(calls, expected);
    assert_eq!(story.flag("BOMB_UNPROTECTED"), 1);
}

#[test]
fn surviving_druuge_end_the_visit_with_the_bomb_still_guarded() {
    let (calls, story, _) = visit(BOMB_DEFINED, BOMB_MOON, &[], |site| {
        site.druuge_tracking = true;
        site.druuge_survivors = true;
    });
    let mut expected = vec![Call::StartSphereTracking];
    expected.extend(druuge_contact());
    expected.extend(world_restored());
    assert_eq!(calls, expected);
    assert_eq!(story.flag("BOMB_UNPROTECTED"), 0);
}

#[test]
fn an_aborted_or_loaded_druuge_talk_leaves_everything_as_it_is() {
    for flag in [CHECK_ABORT, CHECK_LOAD] {
        let (calls, story, _) = visit(BOMB_DEFINED, BOMB_MOON, &[], |site| {
            site.druuge_tracking = true;
            site.druuge_survivors = true;
            site.activity_after_druuge_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | flag);
        });
        let mut expected = vec![Call::StartSphereTracking];
        expected.extend(druuge_contact());
        assert_eq!(calls, expected, "{flag:#x}");
        assert_eq!(story.flag("BOMB_UNPROTECTED"), 0);
    }
}

#[test]
fn a_druuge_guard_with_ships_already_queued_is_a_broken_invariant() {
    let attempt = std::panic::catch_unwind(|| {
        visit(BOMB_DEFINED, BOMB_MOON, &[], |site| {
            site.druuge_tracking = true;
            site.built_ships = 2;
        })
    });
    assert!(attempt.is_err());
}

#[test]
fn an_unprotected_bomb_has_no_guard_and_loads_the_bomb_scene() {
    let (calls, _, _) = visit(
        BOMB_DEFINED,
        BOMB_MOON,
        &[("BOMB_UNPROTECTED", 1)],
        |site| {
            site.druuge_tracking = true;
        },
    );
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadScene(true),
            Call::LoadPlanet
        ]
    );
}

#[test]
fn without_a_guard_in_reach_the_scene_is_loaded_before_the_default_orbital() {
    let (calls, _, climate) = visit(UTWIG_DEFINED, HOME, &[], |_| {});
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadScene(false),
            Call::LoadPlanet
        ]
    );
    assert_eq!(climate, (1, 1), "the Utwig home has a fixed climate");

    let (calls, _, climate) = visit(BOMB_DEFINED, BOMB_MOON, &[], |_| {});
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::StartDruugeTracking,
            Call::LoadScene(true),
            Call::LoadPlanet
        ]
    );
    assert_ne!(climate, (1, 1));
}

#[test]
fn a_taken_bomb_is_just_another_moon() {
    let (calls, _, _) = visit(BOMB_DEFINED, BOMB_MOON, &[("UTWIG_BOMB", 1)], |site| {
        site.utwig_tracking = true;
        site.druuge_tracking = true;
    });
    assert_eq!(calls, [Call::LoadPlanet]);
}

#[test]
fn other_worlds_go_straight_to_the_default_orbital() {
    for (star_index, body) in [
        (UTWIG_DEFINED, Body::Planet(1)),
        (UTWIG_DEFINED, MOON),
        (UTWIG_DEFINED, BOMB_MOON),
        (BOMB_DEFINED, HOME),
        (BOMB_DEFINED, Body::Moon { planet: 5, moon: 0 }),
        (0, HOME),
    ] {
        let (calls, _, _) = visit(star_index, body, &[], |site| {
            site.utwig_tracking = true;
            site.druuge_tracking = true;
        });
        assert_eq!(calls, [Call::LoadPlanet], "{star_index} {body:?}");
    }
}

fn layout(star_index: u8) -> Vec<PLANET_DESC> {
    let mut rng = RandomContext::with_seed(5);
    let plan_data: Vec<PlanetFrame> = vec![unsafe { std::mem::zeroed() }; 128];
    let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
    generate_planets(&mut rng, &plan_data, 0, 2, &mut planets, star_index);
    planets
}

#[test]
fn the_utwig_home_is_a_water_world_and_the_bomb_system_is_untouched() {
    let home = layout(UTWIG_DEFINED);
    assert_eq!(home[0].data_index, WATER_WORLD);
    assert_eq!(home[0].NumPlanets, 1);
    assert_eq!(
        i32::from(home[0].radius),
        i32::from(EARTH_RADIUS) * 174 / 100
    );

    let bomb = layout(BOMB_DEFINED);
    let other = layout(0);
    for (a, b) in bomb.iter().zip(&other) {
        assert_eq!(
            (a.data_index, a.radius, a.rand_seed, a.NumPlanets),
            (b.data_index, b.radius, b.rand_seed, b.NumPlanets)
        );
        assert_eq!((a.location.x, a.location.y), (b.location.x, b.location.y));
    }
    assert_ne!(bomb[0].data_index, WATER_WORLD);
}
