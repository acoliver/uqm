//! The control flow of `GenerateIlwrath_generateOrbital` and the ruins
//! pickup, checked against a recording site and report.

use super::{enter_orbit, pickup_energy, set_home_climate, IlwrathSite, HOME_CONTACT_FLAGS};
use crate::c_bindings::global_state::SYSTEM_INFO;
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::special::Body;

const IN_INTERPLANETARY: u16 = 4;

#[derive(Debug, PartialEq, Eq)]
enum Call {
    StartSphereTracking,
    NotifyAllClear,
    SaveIpGroups,
    ReinitIpGroupQ,
    CountBuiltShips,
    CloneIlwrathFleet,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    TalkToIlwrath,
    ReinitBuiltShipQ,
    LoadIpGroups,
    LoadRuinsScene,
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

impl IlwrathSite for RecordingSite {
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
    fn clone_ilwrath_fleet_into_built_queue(&mut self) {
        self.record(Call::CloneIlwrathFleet);
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
    fn talk_to_ilwrath(&mut self) {
        self.record(Call::TalkToIlwrath);
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
    fn load_ruins_scene(&mut self) {
        self.record(Call::LoadRuinsScene);
    }
}

/// Everything up to and including the conversation.
fn contact_prefix() -> Vec<Call> {
    vec![
        Call::StartSphereTracking,
        Call::NotifyAllClear,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
        Call::CountBuiltShips,
        Call::CloneIlwrathFleet,
        Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY),
        Call::SetGlobalFlagsAndData(HOME_CONTACT_FLAGS),
        Call::TalkToIlwrath,
    ]
}

#[test]
fn contact_that_ends_normally_puts_the_interplanetary_world_back() {
    let mut site = RecordingSite::new(true);
    assert!(enter_orbit(Body::Planet(0), &mut site));

    let mut expected = contact_prefix();
    expected.extend([
        Call::SetActivity(IN_INTERPLANETARY),
        Call::ReinitBuiltShipQ,
        Call::LoadIpGroups,
    ]);
    assert_eq!(*site.calls.borrow(), expected);
    assert_eq!(site.activity, IN_INTERPLANETARY);
}

#[test]
fn contact_keeps_the_conversation_state_when_the_game_aborts_or_loads() {
    for flag in [CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
        let mut site = RecordingSite::new(true);
        let during = IN_INTERPLANETARY | START_INTERPLANETARY | flag;
        site.activity_after_talk = Some(during);
        assert!(enter_orbit(Body::Planet(0), &mut site));

        assert_eq!(*site.calls.borrow(), contact_prefix(), "flag {flag:#x}");
        assert_eq!(site.activity, during);
    }
}

#[test]
fn contact_clears_only_start_interplanetary_afterwards() {
    let mut site = RecordingSite::new(true);
    let during = IN_INTERPLANETARY | START_INTERPLANETARY | 0x0100;
    site.activity_after_talk = Some(during);
    assert!(enter_orbit(Body::Planet(0), &mut site));
    assert_eq!(site.activity, IN_INTERPLANETARY | 0x0100);
}

#[test]
fn contact_needs_the_built_ship_queue_to_be_empty() {
    let mut site = RecordingSite::new(true);
    site.built_ships = 1;
    let visit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        enter_orbit(Body::Planet(0), &mut site)
    }));
    assert!(visit.is_err());
}

#[test]
fn no_sphere_tracking_loads_the_ruins_and_leaves_the_orbital_to_the_default() {
    let mut site = RecordingSite::new(false);
    assert!(!enter_orbit(Body::Planet(0), &mut site));
    assert_eq!(
        *site.calls.borrow(),
        [Call::StartSphereTracking, Call::LoadRuinsScene]
    );
    assert_eq!(site.activity, IN_INTERPLANETARY);
}

#[test]
fn other_worlds_touch_nothing() {
    for body in [Body::Planet(1), Body::Moon { planet: 0, moon: 0 }] {
        let mut site = RecordingSite::new(true);
        assert!(!enter_orbit(body, &mut site));
        assert!(site.calls.borrow().is_empty());
    }
}

#[test]
fn the_home_world_alone_gets_the_fixed_climate() {
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    set_home_climate(Body::Planet(1), &mut info);
    set_home_climate(Body::Moon { planet: 0, moon: 0 }, &mut info);
    assert_eq!((info.PlanetInfo.Weather, info.PlanetInfo.Tectonics), (0, 0));

    set_home_climate(Body::Planet(0), &mut info);
    assert_eq!((info.PlanetInfo.Weather, info.PlanetInfo.Tectonics), (2, 3));
}

#[derive(Default)]
struct FakeReport(Vec<ReportDisposition>);

impl DiscoveryReportPort for FakeReport {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        self.0.push(disposition);
        Ok(true)
    }
}

#[test]
fn pickup_on_the_home_world_cycles_the_ruins_report_and_takes_nothing() {
    let mut report = FakeReport::default();
    assert!(!pickup_energy(Body::Planet(0), &mut report));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
}

#[test]
fn pickup_elsewhere_reports_nothing() {
    for body in [Body::Planet(1), Body::Moon { planet: 0, moon: 0 }] {
        let mut report = FakeReport::default();
        assert!(!pickup_energy(body, &mut report));
        assert!(report.0.is_empty());
    }
}
