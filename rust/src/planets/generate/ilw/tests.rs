//! The home climate and the ruins pickup of the Ilwrath home, checked against
//! a recording report. The contact flow is `home_contact`'s and tested there.

use super::{pickup_energy, set_home_climate};
use crate::c_bindings::global_state::SYSTEM_INFO;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::special::Body;

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
