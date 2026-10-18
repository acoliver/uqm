//! The control flow shared by the ruined homeworlds, checked against a
//! recording site, report and game state.

use std::collections::HashMap;

use super::{enter_orbit, pickup_artifact, HomeContactSite, LanderTakeoff, HOME_CONTACT_FLAGS};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::special::Story;

const IN_INTERPLANETARY: u16 = 4;

#[derive(Debug, PartialEq, Eq)]
enum Call {
    StartSphereTracking,
    NotifyAllClear,
    SaveIpGroups,
    ReinitIpGroupQ,
    CountBuiltShips,
    CloneFleet,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    Talk,
    ReinitBuiltShipQ,
    LoadIpGroups,
    LoadRuinsScene(bool),
}

/// A site whose conversation leaves `activity_after_talk` in
/// `CurrentActivity`, as an abort or a loaded game would.
struct RecordingSite {
    tracking: bool,
    built_ships: u16,
    activity: u16,
    activity_after_talk: Option<u16>,
    calls: std::cell::RefCell<Vec<Call>>,
}

impl RecordingSite {
    fn new(tracking: bool) -> Self {
        Self {
            tracking,
            built_ships: 0,
            activity: IN_INTERPLANETARY,
            activity_after_talk: None,
            calls: std::cell::RefCell::default(),
        }
    }

    fn record(&self, call: Call) {
        self.calls.borrow_mut().push(call);
    }
}

impl HomeContactSite for RecordingSite {
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
        self.record(Call::CountBuiltShips);
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
    fn load_ruins_scene(&mut self, artifact_taken: bool) {
        self.record(Call::LoadRuinsScene(artifact_taken));
    }
}

fn contact_prefix() -> Vec<Call> {
    vec![
        Call::StartSphereTracking,
        Call::NotifyAllClear,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
        Call::CountBuiltShips,
        Call::CloneFleet,
        Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY),
        Call::SetGlobalFlagsAndData(HOME_CONTACT_FLAGS),
        Call::Talk,
    ]
}

#[test]
fn contact_runs_the_conversation_and_puts_the_world_back() {
    for taken in [false, true] {
        let mut site = RecordingSite::new(true);
        assert!(enter_orbit(&mut site, taken));

        let mut expected = contact_prefix();
        expected.extend([
            Call::SetActivity(IN_INTERPLANETARY),
            Call::ReinitBuiltShipQ,
            Call::LoadIpGroups,
        ]);
        assert_eq!(*site.calls.borrow(), expected);
        assert_eq!(site.activity, IN_INTERPLANETARY);
    }
}

#[test]
fn contact_keeps_the_conversation_state_when_the_game_aborts_or_loads() {
    for flag in [CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
        let mut site = RecordingSite::new(true);
        let during = IN_INTERPLANETARY | START_INTERPLANETARY | flag;
        site.activity_after_talk = Some(during);
        assert!(enter_orbit(&mut site, false));

        assert_eq!(*site.calls.borrow(), contact_prefix(), "flag {flag:#x}");
        assert_eq!(site.activity, during);
    }
}

#[test]
fn contact_clears_only_start_interplanetary_afterwards() {
    let mut site = RecordingSite::new(true);
    site.activity_after_talk = Some(IN_INTERPLANETARY | START_INTERPLANETARY | 0x0100);
    assert!(enter_orbit(&mut site, false));
    assert_eq!(site.activity, IN_INTERPLANETARY | 0x0100);
}

#[test]
fn contact_needs_the_built_ship_queue_to_be_empty() {
    let mut site = RecordingSite::new(true);
    site.built_ships = 1;
    let visit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        enter_orbit(&mut site, false)
    }));
    assert!(visit.is_err());
}

#[test]
fn no_sphere_tracking_loads_the_ruins_and_leaves_the_orbital_to_the_default() {
    for taken in [false, true] {
        let mut site = RecordingSite::new(false);
        assert!(!enter_orbit(&mut site, taken));
        assert_eq!(
            *site.calls.borrow(),
            [Call::StartSphereTracking, Call::LoadRuinsScene(taken)]
        );
        assert_eq!(site.activity, IN_INTERPLANETARY);
    }
}

#[derive(Default)]
pub(crate) struct FakeStory(HashMap<&'static str, u32>);

impl FakeStory {
    pub(crate) fn with(flags: &[(&'static str, u32)]) -> Self {
        Self(flags.iter().copied().collect())
    }
}

impl Story for FakeStory {
    fn flag(&self, name: &'static str) -> u32 {
        self.0.get(name).copied().unwrap_or(0)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        self.0.insert(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        self.flag(name)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        self.set_flag(name, value);
    }
}

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

#[test]
fn the_first_pickup_takes_the_artifact_and_launches_the_lander() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_artifact(
        &mut story,
        &mut report,
        &mut lander,
        "A",
        &["B", "C"]
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 1);
    assert_eq!(
        (story.flag("A"), story.flag("B"), story.flag("C")),
        (1, 1, 1)
    );
}

#[test]
fn later_pickups_only_cycle_the_report() {
    let mut story = FakeStory::with(&[("A", 1)]);
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_artifact(
        &mut story,
        &mut report,
        &mut lander,
        "A",
        &["B"]
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 0);
    assert_eq!(story.flag("B"), 0);
}
