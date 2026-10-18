//! The talking pet system's conversation flow, the Ur-Quan abduction and the
//! ruins against recording fakes.

use std::cell::RefCell;
use std::rc::Rc;

use super::{
    generate_energy, generate_orbital, generate_planets, pickup_energy, zap_to_urquan_encounter,
    TalkingPetSite, UrquanEncounterSite, TALK_FLAGS, URQUAN_LOC, WORLD,
};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{
    CHECK_ABORT, CHECK_LOAD, IN_HYPERSPACE, IN_INTERPLANETARY, START_ENCOUNTER,
    START_INTERPLANETARY,
};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::consts::EARTH_RADIUS;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::home_contact::HomeContactSite;
use crate::planets::generate::special::{Body, Orbit, TELLURIC_WORLD};

const OTHER_BODIES: [Body; 3] = [
    Body::Planet(1),
    Body::Moon { planet: 0, moon: 0 },
    Body::Moon { planet: 1, moon: 0 },
];

#[derive(Default)]
struct FakeReport(Vec<ReportDisposition>);

impl DiscoveryReportPort for FakeReport {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        self.0.push(disposition);
        Ok(true)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Call {
    StartSphereTracking,
    NotifyOthers,
    SaveIpGroups,
    ReinitIpGroupQ,
    CloneUmgah,
    CloneZombie,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    TalkUmgah,
    TalkPet,
    ReinitBuiltShipQ,
    LoadIpGroups,
    LoadRuinsScene(bool),
    BeginUrquanEncounter((i16, i16)),
    FadeToBlack,
    MoveGameClockDays(u16),
    DeltaFuel(i16),
    Random,
    ClearSisMessage,
    DrawHyperCoords((i16, i16)),
    FinishEncounter,
}

struct Site {
    tracking: bool,
    /// Whether the Umgah are still trackable once the conversation is over.
    tracking_after: bool,
    talked: bool,
    activity: u16,
    activity_after_talk: Option<u16>,
    survivors_after_talk: bool,
    built: u16,
    encounter_available: bool,
    star: (i16, i16),
    fuel: u32,
    random: u32,
    calls: Rc<RefCell<Vec<Call>>>,
}

impl Site {
    fn new(calls: &Rc<RefCell<Vec<Call>>>) -> Self {
        Self {
            tracking: false,
            tracking_after: false,
            talked: false,
            activity: IN_INTERPLANETARY,
            activity_after_talk: None,
            survivors_after_talk: false,
            built: 0,
            encounter_available: true,
            star: (5288, 4892),
            fuel: 1000,
            random: 0,
            calls: Rc::clone(calls),
        }
    }

    fn record(&self, call: Call) {
        self.calls.borrow_mut().push(call);
    }

    fn after_talk(&mut self) {
        self.talked = true;
        if let Some(after) = self.activity_after_talk {
            self.activity = after;
        }
        self.built = u16::from(self.survivors_after_talk);
    }
}

impl HomeContactSite for Site {
    fn start_sphere_tracking(&mut self) -> bool {
        self.record(Call::StartSphereTracking);
        if self.talked {
            self.tracking_after
        } else {
            self.tracking
        }
    }
    fn notify_others_all_clear(&mut self) {
        self.record(Call::NotifyOthers);
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
        self.record(Call::CloneUmgah);
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
        self.record(Call::TalkUmgah);
        self.after_talk();
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

impl TalkingPetSite for Site {
    fn clone_zombie_into_built_queue(&mut self) {
        self.record(Call::CloneZombie);
    }
    fn built_ship_survives(&self) -> bool {
        self.built > 0
    }
    fn talk_pet(&mut self) {
        self.record(Call::TalkPet);
        if !self.talked {
            self.after_talk();
        }
    }
}

impl UrquanEncounterSite for Site {
    fn begin_urquan_encounter(&mut self, loc: (i16, i16)) -> bool {
        self.record(Call::BeginUrquanEncounter(loc));
        self.encounter_available
    }
    fn fade_to_black(&mut self) {
        self.record(Call::FadeToBlack);
    }
    fn move_game_clock_days(&mut self, days: u16) {
        self.record(Call::MoveGameClockDays(days));
    }
    fn star_point(&self) -> (i16, i16) {
        self.star
    }
    fn fuel_on_board(&self) -> u32 {
        self.fuel
    }
    fn delta_fuel(&mut self, delta: i16) {
        self.record(Call::DeltaFuel(delta));
        self.fuel = (i64::from(self.fuel) + i64::from(delta)) as u32;
    }
    fn random(&mut self) -> u32 {
        self.record(Call::Random);
        self.random
    }
    fn clear_sis_message(&mut self) {
        self.record(Call::ClearSisMessage);
    }
    fn draw_hyper_coords(&mut self, loc: (i16, i16)) {
        self.record(Call::DrawHyperCoords(loc));
    }
    fn finish_encounter(&mut self) {
        self.record(Call::FinishEncounter);
    }
}

/// Fly into `body` and return the calls the host saw, how often the planet
/// was loaded, and the weather left in the orbital information.
fn visit(
    body: Body,
    flags: &[(&'static str, u32)],
    configure: impl FnOnce(&mut Site),
) -> (Vec<Call>, u32, u16) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut story = FakeStory::with(flags);
    let mut site = Site::new(&calls);
    configure(&mut site);

    let mut rng = RandomContext::with_seed(11);
    let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    sys_info.PlanetInfo.Weather = 9;
    let mut world: PLANET_DESC = unsafe { std::mem::zeroed() };
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
        assert!(generate_orbital(
            &mut orbit,
            body,
            &mut story,
            &mut site,
            || loaded += 1
        ));
    }
    let seen = calls.borrow().clone();
    (seen, loaded, sys_info.PlanetInfo.Weather)
}

use Call::*;

const ENTER: [Call; 4] = [
    NotifyOthers,
    SaveIpGroups,
    ReinitIpGroupQ,
    StartSphereTracking,
];

fn talk_start(talk_activity: u16) -> [Call; 2] {
    [
        SetActivity(talk_activity | START_INTERPLANETARY),
        SetGlobalFlagsAndData(TALK_FLAGS),
    ]
}

fn concat(parts: &[&[Call]]) -> Vec<Call> {
    parts.iter().flat_map(|part| part.iter().cloned()).collect()
}

#[test]
fn the_ruins_scene_is_loaded_when_nothing_calls_for_a_conversation() {
    // The pet is found, the zombies are gone, the Umgah cannot be tracked.
    let (calls, loaded, weather) = visit(WORLD, &[("TALKING_PET", 1)], |site| {
        site.tracking = false;
    });
    assert_eq!(
        calls,
        [StartSphereTracking, LoadRuinsScene(false)],
        "the tracking test is the last of the three conditions"
    );
    assert_eq!((loaded, weather), (1, 0), "calm weather on the ruins");
}

#[test]
fn other_bodies_only_get_the_default_orbital() {
    for body in OTHER_BODIES {
        let (calls, loaded, weather) = visit(body, &[], |_| {});
        assert!(calls.is_empty(), "{body:?}");
        assert_eq!(loaded, 1);
        assert_ne!(weather, 0, "{body:?}");
    }
}

#[test]
fn each_condition_alone_starts_the_visit_in_the_conversation_flow() {
    for (flags, tracking) in [
        (
            &[("UMGAH_ZOMBIE_BLOBBIES", 1), ("TALKING_PET", 1)][..],
            false,
        ),
        (&[][..], false),
        (&[("TALKING_PET", 1)][..], true),
    ] {
        let (calls, loaded, _) = visit(WORLD, flags, |site| site.tracking = tracking);
        let enter = calls
            .iter()
            .position(|call| *call == NotifyOthers)
            .expect("the conversation flow starts");
        assert_eq!(calls[enter..enter + 3], ENTER[..3], "{flags:?}");
        assert_eq!(loaded, 0, "{flags:?}");
        assert!(!calls.contains(&LoadRuinsScene(false)));
    }
}

#[test]
fn the_umgah_are_met_while_their_sphere_can_be_tracked() {
    let (calls, loaded, _) = visit(WORLD, &[("TALKING_PET", 1)], |site| {
        site.tracking = true;
        site.tracking_after = true;
        site.survivors_after_talk = true;
    });
    assert_eq!(loaded, 0);
    assert_eq!(
        calls,
        concat(&[
            &[StartSphereTracking],
            &ENTER[..3],
            &[StartSphereTracking],
            &talk_start(IN_INTERPLANETARY),
            &[CloneUmgah, TalkUmgah],
            &[
                SetActivity(IN_INTERPLANETARY),
                // Not hypnotised, no zombies, Umgah still trackable.
                StartSphereTracking,
                ReinitBuiltShipQ,
                LoadIpGroups
            ],
        ])
    );
}

#[test]
fn the_talking_pet_speaks_when_the_kohr_ah_have_destroyed_the_umgah() {
    let (calls, _, _) = visit(WORLD, &[], |site| {
        site.tracking = false;
    });
    assert_eq!(
        calls,
        concat(&[
            &ENTER[..3],
            &[StartSphereTracking],
            &[SetActivity(IN_INTERPLANETARY)],
            &[StartSphereTracking, TalkPet, ReinitBuiltShipQ, LoadIpGroups],
        ])
    );
}

#[test]
fn the_zombie_fleet_is_ten_ships_and_the_pet_talks_after_it_is_beaten() {
    for (survivors, pet_talks_after) in [(false, true), (true, false)] {
        let (calls, _, _) = visit(
            WORLD,
            &[("UMGAH_ZOMBIE_BLOBBIES", 1), ("TALKING_PET", 1)],
            |site| {
                site.tracking = true;
                site.tracking_after = true;
                site.survivors_after_talk = survivors;
            },
        );
        let mut expected = concat(&[
            &ENTER[..3],
            &[StartSphereTracking],
            &talk_start(IN_INTERPLANETARY),
            &[CloneZombie; 10],
            &[TalkPet],
            &[SetActivity(IN_INTERPLANETARY)],
        ]);
        if pet_talks_after {
            expected.push(TalkPet);
        } else {
            expected.push(StartSphereTracking);
        }
        expected.extend([ReinitBuiltShipQ, LoadIpGroups]);
        // With survivors the chain goes on to the tracking test, which
        // succeeds; without them it stops at the pet.
        assert_eq!(calls, expected, "survivors {survivors}");
    }
}

#[test]
fn an_aborted_or_loaded_conversation_leaves_the_world_alone() {
    for abort in [CHECK_ABORT, CHECK_LOAD] {
        let (calls, loaded, _) = visit(WORLD, &[("TALKING_PET", 1)], |site| {
            site.tracking = true;
            site.activity_after_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | abort);
        });
        assert_eq!(loaded, 0);
        assert_eq!(
            calls.last(),
            Some(&TalkUmgah),
            "no queue is put back after {abort:#x}"
        );
    }
}

#[test]
fn a_hypnotised_player_is_taken_to_the_urquan_after_the_talk() {
    let (calls, _, _) = visit(
        WORLD,
        &[("TALKING_PET", 1), ("PLAYER_HYPNOTIZED", 1)],
        |site| {
            site.tracking = true;
            site.tracking_after = true;
        },
    );
    assert!(calls.contains(&BeginUrquanEncounter(URQUAN_LOC)));
    assert_eq!(
        calls[calls.len() - 3..],
        [FinishEncounter, ReinitBuiltShipQ, LoadIpGroups]
    );
    assert!(!calls.contains(&TalkPet));
}

fn zap(configure: impl FnOnce(&mut Site)) -> (Vec<Call>, Site) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut site = Site::new(&calls);
    configure(&mut site);
    zap_to_urquan_encounter(&mut site);
    let seen = calls.borrow().clone();
    (seen, site)
}

#[test]
fn the_abduction_charges_the_distance_plus_half_a_tank_unit() {
    // 3 and 4 units from the Ur-Quan: distance 5 plus 50.
    let (calls, site) = zap(|site| {
        site.star = (URQUAN_LOC.0 + 3, URQUAN_LOC.1 - 4);
        site.fuel = 100_000;
    });
    assert_eq!(
        calls,
        [
            BeginUrquanEncounter(URQUAN_LOC),
            FadeToBlack,
            MoveGameClockDays(15),
            SetActivity(IN_HYPERSPACE | START_ENCOUNTER),
            DeltaFuel(-55),
            ClearSisMessage,
            DrawHyperCoords(URQUAN_LOC),
            FinishEncounter,
        ]
    );
    assert_eq!(site.activity, IN_HYPERSPACE | START_ENCOUNTER);
}

#[test]
fn the_abduction_refills_the_tank_to_between_five_and_nine_units() {
    for (random, units) in [(0u32, 5i64), (4, 9), (5, 5), (12, 7)] {
        let (calls, site) = zap(|site| {
            site.fuel = 120;
            site.random = random;
        });
        // Distance 0: the charge is 50, leaving 70, below five units.
        assert_eq!(calls[4], DeltaFuel(-50));
        assert_eq!(calls[5], Random);
        assert_eq!(site.fuel, (units * 100) as u32, "random {random}");
    }
}

#[test]
fn the_abduction_leaves_a_full_enough_tank_alone() {
    let (calls, site) = zap(|site| site.fuel = 550);
    assert!(!calls.contains(&Random));
    assert_eq!(site.fuel, 500);
    // Exactly five units after the charge is not short.
    let (calls, _) = zap(|site| site.fuel = 550);
    assert_eq!(
        calls.iter().filter(|c| matches!(c, DeltaFuel(_))).count(),
        1
    );
}

#[test]
fn nothing_happens_when_there_is_no_encounter_to_use() {
    let (calls, site) = zap(|site| site.encounter_available = false);
    assert_eq!(calls, [BeginUrquanEncounter(URQUAN_LOC)]);
    assert_eq!(site.activity, IN_INTERPLANETARY);
}

#[test]
fn the_world_is_telluric_and_only_it_has_ruins() {
    let mut rng = RandomContext::with_seed(5);
    let plan_data: Vec<PlanetFrame> = vec![unsafe { std::mem::zeroed() }; 128];
    let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
    generate_planets(&mut rng, &plan_data, 0, 2, &mut planets);
    assert_eq!(planets[0].data_index, TELLURIC_WORLD);
    assert_eq!(
        i32::from(planets[0].radius),
        i32::from(EARTH_RADIUS) * 204 / 100
    );

    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = [1, 2, 3];
    let count = |body| {
        let mut rng = RandomContext::with_seed(7);
        let mut node: NODE_INFO = unsafe { std::mem::zeroed() };
        generate_energy(&mut rng, &info, body, !0, &mut node)
    };
    assert_eq!(count(WORLD), 16);
    for body in OTHER_BODIES {
        assert_eq!(count(body), 0, "{body:?}");
    }
}

#[test]
fn the_ruins_cycle_the_report_and_other_worlds_report_nothing() {
    let mut report = FakeReport::default();
    assert!(!pickup_energy(WORLD, &mut report));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    for body in OTHER_BODIES {
        let mut report = FakeReport::default();
        assert!(!pickup_energy(body, &mut report));
        assert!(report.0.is_empty());
    }
}
