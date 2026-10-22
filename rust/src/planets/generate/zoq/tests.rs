//! The Zoq-Fot-Pik home's story gating and conversation flow against
//! recording fakes.

use std::cell::RefCell;
use std::rc::Rc;

use super::{
    generate_energy, generate_orbital, init_npcs, pickup_energy, ZoqFotPikSite, TALK_FLAGS,
};
use crate::c_bindings::global_state::PlanetFrame;
use crate::c_bindings::global_state::{IP_GROUP, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::generate::default::Npcs;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::home_contact::HomeContactSite;
use crate::planets::generate::special::{Body, Orbit};

const WORLD: Body = Body::Planet(0);
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

#[test]
fn the_world_has_ruins_and_nothing_else_has_nodes() {
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

#[derive(Default)]
struct FakeNpcs {
    calls: Vec<&'static str>,
}

impl Npcs for FakeNpcs {
    fn init_ip_groups(&mut self) -> bool {
        self.calls.push("init_ip_groups");
        true
    }
    fn clear_battle_group_ref(&mut self) {
        self.calls.push("clear_battle_group_ref");
    }
    fn build_groups(&mut self) {
        self.calls.push("build_groups");
    }
    fn load_ip_groups(&mut self) {
        self.calls.push("load_ip_groups");
    }
    fn save_ip_groups(&mut self) {
        self.calls.push("save_ip_groups");
    }
    fn reinit_npc_built_ship_q(&mut self) {
        self.calls.push("reinit_npc_built_ship_q");
    }
    fn reinit_ip_group_q(&mut self) {
        self.calls.push("reinit_ip_group_q");
    }
    fn yehat_civil_war(&self) -> bool {
        false
    }
    fn encounter_race(&self) -> i16 {
        0
    }
    fn for_each_ip_group(&mut self, _visit: &mut dyn FnMut(&mut IP_GROUP)) {}
}

#[test]
fn the_default_groups_are_made_unless_the_zoq_fot_pik_are_in_distress_value_one() {
    for distress in 0..4u32 {
        let story = FakeStory::with(&[("ZOQFOT_DISTRESS", distress)]);
        let mut host = FakeNpcs::default();
        assert!(init_npcs(&story, &mut host));
        let expected: &[&str] = if distress == 1 {
            &[]
        } else {
            &["init_ip_groups"]
        };
        assert_eq!(host.calls, expected, "distress {distress}");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Call {
    StartSphereTracking,
    SaveIpGroups,
    ReinitIpGroupQ,
    CloneFleet,
    CloneBlackUrquan,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    Talk,
    TalkBlackUrquan,
    ReinitBuiltShipQ,
    LoadIpGroups,
    LoadRuinsScene(bool),
    LoadPlanet,
}

const IN_INTERPLANETARY: u16 = 4;

/// A site recording every host call; the activity each conversation leaves
/// and the Black Ur-Quan ships that survive are configured.
struct Site {
    tracking: bool,
    activity: u16,
    activity_after_black_urquan: Option<u16>,
    activity_after_talk: Option<u16>,
    black_urquan_survives: bool,
    built: u16,
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
        panic!("the Zoq-Fot-Pik home does not notify others");
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

impl ZoqFotPikSite for Site {
    fn clone_black_urquan_into_built_queue(&mut self) {
        self.record(Call::CloneBlackUrquan);
        self.built = 1;
    }
    fn talk_black_urquan(&mut self) {
        self.record(Call::TalkBlackUrquan);
        if let Some(after) = self.activity_after_black_urquan {
            self.activity = after;
        }
        if !self.black_urquan_survives {
            self.built = 0;
        }
    }
}

fn visit(body: Body, distress: u32, configure: impl FnOnce(&mut Site)) -> Vec<Call> {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let story = FakeStory::with(&[("ZOQFOT_DISTRESS", distress)]);
    let mut site = Site {
        tracking: false,
        activity: IN_INTERPLANETARY,
        activity_after_black_urquan: None,
        activity_after_talk: None,
        black_urquan_survives: false,
        built: 0,
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
        &story,
        &mut site,
        || { loaded.borrow_mut().push(Call::LoadPlanet) }
    ));
    calls.take()
}

fn groups_put_away() -> Vec<Call> {
    vec![
        Call::StartSphereTracking,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
    ]
}

fn zoq_fot_pik_talk(activity: u16) -> Vec<Call> {
    vec![
        Call::CloneFleet,
        Call::SetActivity(activity | START_INTERPLANETARY),
        Call::SetGlobalFlagsAndData(TALK_FLAGS),
        Call::Talk,
    ]
}

fn black_urquan_talk() -> Vec<Call> {
    vec![
        Call::CloneBlackUrquan,
        Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY),
        Call::SetGlobalFlagsAndData(TALK_FLAGS),
        Call::TalkBlackUrquan,
    ]
}

fn world_restored(activity: u16) -> Vec<Call> {
    vec![
        Call::SetActivity(activity),
        Call::ReinitBuiltShipQ,
        Call::LoadIpGroups,
    ]
}

#[test]
fn without_distress_the_zoq_fot_pik_talk_and_the_world_is_restored() {
    let calls = visit(WORLD, 0, |site| site.tracking = true);
    let mut expected = groups_put_away();
    expected.extend(zoq_fot_pik_talk(IN_INTERPLANETARY));
    expected.extend(world_restored(IN_INTERPLANETARY));
    assert_eq!(calls, expected);
}

#[test]
fn an_aborted_or_loaded_zoq_fot_pik_conversation_leaves_everything_as_it_is() {
    for flag in [CHECK_ABORT, CHECK_LOAD] {
        let calls = visit(WORLD, 0, |site| {
            site.tracking = true;
            site.activity_after_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | flag);
        });
        let mut expected = groups_put_away();
        expected.extend(zoq_fot_pik_talk(IN_INTERPLANETARY));
        assert_eq!(calls, expected, "{flag:#x}");
    }
}

#[test]
fn in_distress_the_black_urquan_are_met_first_and_survivors_end_the_visit() {
    for distress in [1, 2, 3] {
        let calls = visit(WORLD, distress, |site| {
            site.tracking = true;
            site.black_urquan_survives = true;
        });
        let mut expected = groups_put_away();
        expected.extend(black_urquan_talk());
        expected.extend(world_restored(IN_INTERPLANETARY));
        assert_eq!(calls, expected, "distress {distress}");
    }
}

#[test]
fn an_aborted_or_loaded_black_urquan_conversation_leaves_everything_as_it_is() {
    for flag in [CHECK_ABORT, CHECK_LOAD] {
        let calls = visit(WORLD, 1, |site| {
            site.tracking = true;
            site.black_urquan_survives = true;
            site.activity_after_black_urquan =
                Some(IN_INTERPLANETARY | START_INTERPLANETARY | flag);
        });
        let mut expected = groups_put_away();
        expected.extend(black_urquan_talk());
        assert_eq!(calls, expected, "{flag:#x}");
    }
}

#[test]
fn a_beaten_black_urquan_leads_on_to_the_zoq_fot_pik() {
    let calls = visit(WORLD, 1, |site| site.tracking = true);
    let mut expected = groups_put_away();
    expected.extend(black_urquan_talk());
    // The interplanetary flag set for the first talk is still set.
    expected.extend(zoq_fot_pik_talk(IN_INTERPLANETARY | START_INTERPLANETARY));
    expected.extend(world_restored(IN_INTERPLANETARY));
    assert_eq!(calls, expected);
}

#[test]
fn without_sphere_tracking_the_ruins_are_loaded_before_the_default_orbital() {
    let calls = visit(WORLD, 1, |_| {});
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
fn other_worlds_go_straight_to_the_default_orbital() {
    for body in OTHER_BODIES {
        let calls = visit(body, 0, |site| site.tracking = true);
        assert_eq!(calls, [Call::LoadPlanet], "{body:?}");
    }
}
