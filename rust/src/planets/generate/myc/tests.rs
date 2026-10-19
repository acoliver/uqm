//! The Mycon systems' story gating, guard conversations and pickups against
//! recording fakes.

use std::cell::RefCell;
use std::rc::Rc;

use super::{
    generate_energy, generate_life, generate_orbital, generate_planets, pickup_energy, MyconSite,
    Scene, AMBUSH_SHIPS, HOME_TALK_FLAGS, SUN_DEVICE_TALK_FLAGS, WORLD,
};
use crate::battle::gendef::{
    EGG_CASE0_DEFINED, EGG_CASE1_DEFINED, EGG_CASE2_DEFINED, MYCON_DEFINED, SUN_DEVICE_DEFINED,
};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::consts::EARTH_RADIUS;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{Body, Orbit, Story, SHATTERED_WORLD};
use crate::planets::surface::ENERGY_SCAN;

const MOON: Body = Body::Moon { planet: 0, moon: 0 };
const IN_INTERPLANETARY: u16 = 4;
const EGG_CASES: [u8; 3] = [EGG_CASE0_DEFINED, EGG_CASE1_DEFINED, EGG_CASE2_DEFINED];

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
    CloneAmbushShip,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    Talk,
    ReinitBuiltShipQ,
    LoadIpGroups,
    RepairSisBorder,
    LoadScene(Scene),
    LoadPlanet,
    Report(ReportDisposition),
    Takeoff,
    SetFlag(&'static str, u32),
}

/// A site recording every host call.
struct Site {
    tracking: bool,
    built_ships: u16,
    survivors: bool,
    activity: u16,
    activity_after_talk: Option<u16>,
    calls: Rc<RefCell<Vec<Call>>>,
}

impl Site {
    fn new(calls: &Rc<RefCell<Vec<Call>>>) -> Self {
        Self {
            tracking: false,
            built_ships: 0,
            survivors: false,
            activity: IN_INTERPLANETARY,
            activity_after_talk: None,
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
        if let Some(after) = self.activity_after_talk {
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
        panic!("the Mycon systems load their own scenes");
    }
}

impl MyconSite for Site {
    fn clone_ambush_ship_into_built_queue(&mut self) {
        self.record(Call::CloneAmbushShip);
    }
    fn built_queue_has_ships(&self) -> bool {
        self.survivors
    }
    fn repair_sis_border(&mut self) {
        self.record(Call::RepairSisBorder);
    }
    fn load_scene(&mut self, scene: Scene) {
        self.record(Call::LoadScene(scene));
    }
}

fn nodes(star_index: u8, body: Body, story: &FakeStory, strings_loaded: bool) -> u16 {
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = [1, 2, 3];
    if strings_loaded {
        info.PlanetInfo.DiscoveryString = std::ptr::NonNull::dangling().as_ptr();
    }
    let mut rng = RandomContext::with_seed(7);
    let mut node: NODE_INFO = unsafe { std::mem::zeroed() };
    generate_energy(&mut rng, &info, star_index, body, !0, &mut node, story)
}

#[test]
fn the_sun_device_is_one_artifact_until_taken() {
    assert_eq!(
        nodes(SUN_DEVICE_DEFINED, WORLD, &FakeStory::default(), false),
        1
    );
    let taken = FakeStory::with(&[("SUN_DEVICE", 1)]);
    assert_eq!(nodes(SUN_DEVICE_DEFINED, WORLD, &taken, false), 0);
}

#[test]
fn an_egg_case_is_one_artifact_while_its_strings_are_loaded() {
    for star_index in EGG_CASES {
        let story = FakeStory::default();
        assert_eq!(nodes(star_index, WORLD, &story, true), 1, "{star_index}");
        assert_eq!(nodes(star_index, WORLD, &story, false), 0, "{star_index}");
    }
}

#[test]
fn other_worlds_and_systems_have_no_nodes() {
    let story = FakeStory::default();
    let others = [Body::Planet(1), MOON, Body::Moon { planet: 1, moon: 0 }];
    for star_index in [SUN_DEVICE_DEFINED, MYCON_DEFINED, EGG_CASE0_DEFINED, 0] {
        for body in others {
            assert_eq!(
                nodes(star_index, body, &story, true),
                0,
                "{star_index} {body:?}"
            );
        }
    }
    assert_eq!(nodes(MYCON_DEFINED, WORLD, &story, true), 0);
    assert_eq!(nodes(0, WORLD, &story, true), 0);
}

#[test]
fn there_is_no_life_in_mycon_systems() {
    assert_eq!(generate_life(), 0);
}

#[test]
fn the_sun_device_is_taken_once_from_node_zero() {
    let mut story = FakeStory::with(&[("MYCON_VISITS", 4)]);
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(pickup_energy(
        SUN_DEVICE_DEFINED,
        WORLD,
        0,
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Once]);
    assert_eq!(lander.0, 1);
    for (flag, value) in [
        ("SUN_DEVICE", 1),
        ("SUN_DEVICE_ON_SHIP", 1),
        ("MYCON_VISITS", 0),
    ] {
        assert_eq!(story.flag(flag), value, "{flag}");
    }
}

#[test]
fn the_sun_device_cannot_be_taken_twice_or_from_another_node() {
    for (taken, node) in [(1, 0), (0, 1)] {
        let mut story = FakeStory::with(&[("SUN_DEVICE", taken)]);
        let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pickup_energy(
                SUN_DEVICE_DEFINED,
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
fn each_egg_case_system_marks_its_own_egg_case() {
    let flags = [
        "EGG_CASE0_ON_SHIP",
        "EGG_CASE1_ON_SHIP",
        "EGG_CASE2_ON_SHIP",
    ];
    for (taken, star_index) in EGG_CASES.into_iter().enumerate() {
        let mut story = FakeStory::default();
        let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
        assert!(pickup_energy(
            star_index,
            WORLD,
            0,
            &mut story,
            &mut report,
            &mut lander
        ));
        assert_eq!(report.0, [ReportDisposition::Once]);
        assert_eq!(lander.0, 1);
        for (index, flag) in flags.iter().enumerate() {
            assert_eq!(
                story.flag(flag),
                u32::from(index == taken),
                "{star_index} {flag}"
            );
        }
        assert_eq!(story.flag("SUN_DEVICE"), 0);
    }
}

#[test]
fn an_egg_case_cannot_be_taken_from_another_node() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pickup_energy(
            EGG_CASE1_DEFINED,
            WORLD,
            3,
            &mut story,
            &mut report,
            &mut lander,
        )
    }));
    assert!(attempt.is_err());
    assert_eq!(lander.0, 0);
}

#[test]
fn other_worlds_pick_up_nothing() {
    for (star_index, body) in [
        (SUN_DEVICE_DEFINED, Body::Planet(1)),
        (SUN_DEVICE_DEFINED, MOON),
        (EGG_CASE2_DEFINED, Body::Planet(1)),
        (EGG_CASE0_DEFINED, MOON),
        (MYCON_DEFINED, WORLD),
        (0, WORLD),
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
    }
}

/// What the Mycon talk looks like to the host, with `flags` in
/// `GLOBAL_FLAGS_AND_DATA`, with the ambush fleet cloned when `ambush`.
fn mycon_contact(flags: u8, ambush: bool) -> Vec<Call> {
    let mut calls = vec![
        Call::StartSphereTracking,
        Call::NotifyAllClear,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
    ];
    if ambush {
        calls.extend((0..AMBUSH_SHIPS).map(|_| Call::CloneAmbushShip));
    } else {
        calls.push(Call::CloneFleet);
    }
    calls.extend([
        Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY),
        Call::SetGlobalFlagsAndData(flags),
        Call::Talk,
    ]);
    calls
}

fn world_restored() -> Vec<Call> {
    vec![
        Call::SetActivity(IN_INTERPLANETARY),
        Call::ReinitBuiltShipQ,
        Call::LoadIpGroups,
    ]
}

/// Run `generate_orbital` on `body`; the host calls, the story afterwards and
/// the orbital's result.
fn visit(
    star_index: u8,
    body: Body,
    flags: &[(&'static str, u32)],
    energy_retrieved: bool,
    configure: impl FnOnce(&mut Site),
) -> (Vec<Call>, FakeStory) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut story = FakeStory::with(flags);
    let mut site = Site::new(&calls);
    configure(&mut site);

    let mut rng = RandomContext::with_seed(11);
    let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    if energy_retrieved {
        sys_info.PlanetInfo.ScanRetrieveMask[ENERGY_SCAN] = 1;
    }
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

#[test]
fn the_mycon_talk_at_their_home_and_win_or_lose_ends_as_the_fleet_does() {
    // Fleet beaten: the visit goes on to the default orbital.
    let (calls, story) = visit(MYCON_DEFINED, WORLD, &[], false, |site| {
        site.tracking = true
    });
    let mut expected = mycon_contact(HOME_TALK_FLAGS, false);
    expected.extend(world_restored());
    expected.extend([Call::RepairSisBorder, Call::LoadPlanet]);
    assert_eq!(calls, expected);
    assert_eq!(story.flag("SUN_DEVICE_UNGUARDED"), 1);

    // Mycon ships survive: the visit ends without the default orbital.
    let (calls, story) = visit(MYCON_DEFINED, WORLD, &[], false, |site| {
        site.tracking = true;
        site.survivors = true;
    });
    let mut expected = mycon_contact(HOME_TALK_FLAGS, false);
    expected.extend(world_restored());
    assert_eq!(calls, expected);
    assert_eq!(story.flag("SUN_DEVICE_UNGUARDED"), 0);
}

#[test]
fn the_mycon_home_ambush_story_flag_does_not_matter() {
    let (calls, _) = visit(
        MYCON_DEFINED,
        WORLD,
        &[("MYCON_FELL_FOR_AMBUSH", 1)],
        false,
        |site| site.tracking = true,
    );
    assert!(calls.contains(&Call::CloneFleet));
    assert!(!calls.contains(&Call::CloneAmbushShip));
}

#[test]
fn an_untracked_mycon_home_goes_to_the_default_orbital() {
    let (calls, _) = visit(MYCON_DEFINED, WORLD, &[], false, |_| {});
    assert_eq!(calls, [Call::StartSphereTracking, Call::LoadPlanet]);
}

#[test]
fn the_sun_device_system_is_guarded_until_unguarded() {
    let (calls, story) = visit(SUN_DEVICE_DEFINED, WORLD, &[], false, |site| {
        site.tracking = true;
    });
    let mut expected = mycon_contact(SUN_DEVICE_TALK_FLAGS, false);
    expected.extend(world_restored());
    expected.extend([
        Call::RepairSisBorder,
        Call::LoadScene(Scene::SunDevice),
        Call::LoadPlanet,
    ]);
    assert_eq!(calls, expected);
    assert_eq!(story.flag("SUN_DEVICE_UNGUARDED"), 1);

    let (calls, _) = visit(
        SUN_DEVICE_DEFINED,
        WORLD,
        &[("SUN_DEVICE_UNGUARDED", 1)],
        false,
        |site| site.tracking = true,
    );
    assert_eq!(
        calls,
        [
            Call::StartSphereTracking,
            Call::LoadScene(Scene::SunDevice),
            Call::LoadPlanet
        ]
    );
}

#[test]
fn a_mycon_fleet_that_fell_for_the_ambush_is_five_finite_ships() {
    let (calls, _) = visit(
        SUN_DEVICE_DEFINED,
        WORLD,
        &[("MYCON_FELL_FOR_AMBUSH", 1)],
        false,
        |site| {
            site.tracking = true;
            site.survivors = true;
        },
    );
    let mut expected = mycon_contact(SUN_DEVICE_TALK_FLAGS, true);
    expected.extend(world_restored());
    assert_eq!(calls, expected);
}

#[test]
fn an_aborted_or_loaded_talk_leaves_everything_as_it_is() {
    for (star_index, flags) in [
        (MYCON_DEFINED, HOME_TALK_FLAGS),
        (SUN_DEVICE_DEFINED, SUN_DEVICE_TALK_FLAGS),
    ] {
        for flag in [CHECK_ABORT, CHECK_LOAD] {
            let (calls, story) = visit(star_index, WORLD, &[], false, |site| {
                site.tracking = true;
                site.survivors = true;
                site.activity_after_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | flag);
            });
            assert_eq!(calls, mycon_contact(flags, false), "{star_index} {flag:#x}");
            assert_eq!(story.flag("SUN_DEVICE_UNGUARDED"), 0);
        }
    }
}

#[test]
fn a_talk_with_ships_already_queued_is_a_broken_invariant() {
    let attempt = std::panic::catch_unwind(|| {
        visit(MYCON_DEFINED, WORLD, &[], false, |site| {
            site.tracking = true;
            site.built_ships = 1;
        })
    });
    assert!(attempt.is_err());
}

#[test]
fn a_taken_sun_device_loads_no_scene() {
    let (calls, _) = visit(
        SUN_DEVICE_DEFINED,
        WORLD,
        &[("SUN_DEVICE", 1), ("SUN_DEVICE_UNGUARDED", 1)],
        false,
        |site| site.tracking = true,
    );
    assert_eq!(calls, [Call::StartSphereTracking, Call::LoadPlanet]);
}

#[test]
fn an_egg_case_system_teaches_about_the_shattered_worlds_and_loads_the_scene() {
    for star_index in EGG_CASES {
        for known in [0, 1, 2] {
            let (calls, story) = visit(
                star_index,
                WORLD,
                &[("KNOW_ABOUT_SHATTERED", known)],
                false,
                |site| site.tracking = true,
            );
            assert_eq!(
                calls,
                [Call::LoadScene(Scene::EggCase), Call::LoadPlanet],
                "{star_index}"
            );
            assert_eq!(story.flag("KNOW_ABOUT_SHATTERED"), known.max(1));
        }
    }
}

#[test]
fn a_taken_egg_case_loads_no_scene_but_the_player_still_learns() {
    let (calls, story) = visit(EGG_CASE0_DEFINED, WORLD, &[], true, |_| {});
    assert_eq!(calls, [Call::LoadPlanet]);
    assert_eq!(story.flag("KNOW_ABOUT_SHATTERED"), 1);
}

#[test]
fn other_worlds_go_straight_to_the_default_orbital() {
    for star_index in [MYCON_DEFINED, SUN_DEVICE_DEFINED, EGG_CASE1_DEFINED, 0] {
        for body in [Body::Planet(1), MOON] {
            let (calls, story) = visit(star_index, body, &[], false, |site| site.tracking = true);
            assert_eq!(calls, [Call::LoadPlanet], "{star_index} {body:?}");
            assert_eq!(story.flag("KNOW_ABOUT_SHATTERED"), 0);
        }
    }
    let (calls, _) = visit(0, WORLD, &[], false, |site| site.tracking = true);
    assert_eq!(calls, [Call::LoadPlanet]);
}

#[test]
fn planet_zero_is_a_shattered_world_with_at_most_two_moons() {
    let mut rng = RandomContext::with_seed(5);
    let plan_data: Vec<PlanetFrame> = vec![unsafe { std::mem::zeroed() }; 128];
    let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
    generate_planets(&mut rng, &plan_data, 0, 2, &mut planets);
    assert_eq!(planets[0].data_index, SHATTERED_WORLD);
    assert_eq!(
        i32::from(planets[0].radius),
        i32::from(EARTH_RADIUS) * 80 / 100
    );
    assert!(planets[0].NumPlanets <= 2);
}

#[test]
fn all_egg_scenes_use_the_energy_retrieval_bit_and_preserve_prior_knowledge() {
    for star in EGG_CASES {
        for known in [0, 1, 2] {
            for retrieved in [false, true] {
                let (calls, story) = visit(
                    star,
                    WORLD,
                    &[("KNOW_ABOUT_SHATTERED", known)],
                    retrieved,
                    |_| {},
                );
                let expected = if retrieved {
                    vec![Call::LoadPlanet]
                } else {
                    vec![Call::LoadScene(Scene::EggCase), Call::LoadPlanet]
                };
                assert_eq!(calls, expected);
                assert_eq!(story.flag("KNOW_ABOUT_SHATTERED"), known.max(1));
            }
        }
    }
}

#[test]
fn taking_the_sun_device_does_not_skip_a_still_guarded_conversation() {
    let (calls, story) = visit(
        SUN_DEVICE_DEFINED,
        WORLD,
        &[("SUN_DEVICE", 1)],
        false,
        |site| site.tracking = true,
    );
    let mut expected = mycon_contact(SUN_DEVICE_TALK_FLAGS, false);
    expected.extend(world_restored());
    expected.extend([Call::RepairSisBorder, Call::LoadPlanet]);
    assert_eq!(calls, expected);
    assert_eq!(story.flag("SUN_DEVICE"), 1);
    assert_eq!(story.flag("SUN_DEVICE_UNGUARDED"), 1);
}

#[test]
fn guard_story_and_conversation_outcomes_have_exact_host_effects() {
    for star in [MYCON_DEFINED, SUN_DEVICE_DEFINED] {
        for tracking in [false, true] {
            for unguarded in [0, 1] {
                for ambush in [0, 1] {
                    for survivors in [false, true] {
                        for status in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
                            let flags = [
                                ("SUN_DEVICE_UNGUARDED", unguarded),
                                ("MYCON_FELL_FOR_AMBUSH", ambush),
                            ];
                            let (calls, story) = visit(star, WORLD, &flags, false, |site| {
                                site.tracking = tracking;
                                site.survivors = survivors;
                                site.activity_after_talk =
                                    Some(IN_INTERPLANETARY | START_INTERPLANETARY | status);
                            });
                            let talks = tracking && (star == MYCON_DEFINED || unguarded == 0);
                            let mut expected = vec![Call::StartSphereTracking];
                            let mut resulting_unguarded = unguarded;
                            let mut continues = true;
                            if talks {
                                expected = mycon_contact(
                                    if star == MYCON_DEFINED {
                                        HOME_TALK_FLAGS
                                    } else {
                                        SUN_DEVICE_TALK_FLAGS
                                    },
                                    star == SUN_DEVICE_DEFINED && ambush != 0,
                                );
                                if status != 0 {
                                    continues = false;
                                } else {
                                    expected.extend(world_restored());
                                    if survivors {
                                        continues = false;
                                    } else {
                                        resulting_unguarded = 1;
                                        expected.push(Call::RepairSisBorder);
                                    }
                                }
                            }
                            if continues {
                                if star == SUN_DEVICE_DEFINED {
                                    expected.push(Call::LoadScene(Scene::SunDevice));
                                }
                                expected.push(Call::LoadPlanet);
                            }
                            assert_eq!(calls, expected, "star={star} tracking={tracking} flags={flags:?} survivors={survivors} status={status:#x}");
                            assert_eq!(story.flag("SUN_DEVICE_UNGUARDED"), resulting_unguarded);
                            assert_eq!(story.flag("MYCON_FELL_FOR_AMBUSH"), ambush);
                        }
                    }
                }
            }
        }
    }
}

struct OrderedStory {
    inner: FakeStory,
    calls: Rc<RefCell<Vec<Call>>>,
}

impl Story for OrderedStory {
    fn flag(&self, name: &'static str) -> u32 {
        self.inner.flag(name)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        self.calls.borrow_mut().push(Call::SetFlag(name, value));
        self.inner.set_flag(name, value);
    }
    fn word(&self, _name: &'static str) -> u32 {
        panic!("Mycon hooks do not read group references");
    }
    fn set_word(&mut self, _name: &'static str, _value: u32) {
        panic!("Mycon hooks do not write group references");
    }
}

struct OrderedReport(Rc<RefCell<Vec<Call>>>);

impl DiscoveryReportPort for OrderedReport {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        self.0.borrow_mut().push(Call::Report(disposition));
        Ok(true)
    }
}

struct OrderedLander(Rc<RefCell<Vec<Call>>>);

impl LanderTakeoff for OrderedLander {
    fn lander_takeoff(&mut self) {
        self.0.borrow_mut().push(Call::Takeoff);
    }
}

#[test]
fn pickups_report_then_launch_then_write_only_their_story_flags() {
    for (star, flag) in [
        (SUN_DEVICE_DEFINED, "SUN_DEVICE"),
        (EGG_CASE0_DEFINED, "EGG_CASE0_ON_SHIP"),
        (EGG_CASE1_DEFINED, "EGG_CASE1_ON_SHIP"),
        (EGG_CASE2_DEFINED, "EGG_CASE2_ON_SHIP"),
    ] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut story = OrderedStory {
            inner: FakeStory::with(&[("MYCON_VISITS", 7)]),
            calls: Rc::clone(&calls),
        };
        assert!(pickup_energy(
            star,
            WORLD,
            0,
            &mut story,
            &mut OrderedReport(Rc::clone(&calls)),
            &mut OrderedLander(Rc::clone(&calls))
        ));
        let mut expected = vec![
            Call::Report(ReportDisposition::Once),
            Call::Takeoff,
            Call::SetFlag(flag, 1),
        ];
        if star == SUN_DEVICE_DEFINED {
            expected.extend([
                Call::SetFlag("SUN_DEVICE_ON_SHIP", 1),
                Call::SetFlag("MYCON_VISITS", 0),
            ]);
        }
        assert_eq!(*calls.borrow(), expected, "{star}");
    }
}

#[test]
fn winning_clears_only_the_activity_bit_then_restores_groups_before_unguarding() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut site = Site::new(&calls);
    site.activity_after_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | 0x0100);
    let mut story = OrderedStory {
        inner: FakeStory::default(),
        calls: Rc::clone(&calls),
    };
    assert!(super::mycon_talk(MYCON_DEFINED, &mut story, &mut site));
    let mut expected = mycon_contact(HOME_TALK_FLAGS, false);
    expected.remove(0); // The caller already performed sphere tracking.
    expected.extend([
        Call::SetActivity(IN_INTERPLANETARY | 0x0100),
        Call::ReinitBuiltShipQ,
        Call::LoadIpGroups,
        Call::SetFlag("SUN_DEVICE_UNGUARDED", 1),
        Call::RepairSisBorder,
    ]);
    assert_eq!(*calls.borrow(), expected);
    assert_eq!(site.activity, IN_INTERPLANETARY | 0x0100);
}
