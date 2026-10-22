//! The story gating of the Pkunk ruins, against a recording report and a
//! fake game state.

use super::pickup_energy;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::home_contact::LanderTakeoff;
use crate::planets::generate::special::{Body, Story};

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

const ALSO: [&str; 1] = ["CLEAR_SPINDLE_ON_SHIP"];

#[test]
fn the_first_ruin_gives_the_artifact_and_leaves_the_node() {
    let mut story = FakeStory::default();
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_energy(
        Body::Planet(0),
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 1);
    assert_eq!(story.flag("CLEAR_SPINDLE"), 1);
    for flag in ALSO {
        assert_eq!(story.flag(flag), 1);
    }
}

#[test]
fn later_ruins_only_cycle_the_report() {
    let mut story = FakeStory::with(&[("CLEAR_SPINDLE", 1)]);
    let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
    assert!(!pickup_energy(
        Body::Planet(0),
        &mut story,
        &mut report,
        &mut lander
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(lander.0, 0);
    for flag in ALSO {
        assert_eq!(story.flag(flag), 0);
    }
}

#[test]
fn other_worlds_pick_up_nothing() {
    for body in [Body::Planet(1), Body::Moon { planet: 0, moon: 0 }] {
        let mut story = FakeStory::default();
        let (mut report, mut lander) = (FakeReport::default(), FakeLander::default());
        assert!(!pickup_energy(body, &mut story, &mut report, &mut lander));
        assert!(report.0.is_empty());
        assert_eq!(lander.0, 0);
        assert_eq!(story.flag("CLEAR_SPINDLE"), 0);
    }
}
