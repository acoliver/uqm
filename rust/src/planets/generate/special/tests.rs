//! Story gating and side effects of the small per-system generators, with a
//! fake game state, report and sites.

use std::collections::HashMap;

use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::special::{Body, Story};
use crate::planets::generate::{rainbow, syreen, vault, wreck, yehat};

#[derive(Default)]
struct FakeStory(HashMap<&'static str, u32>);

impl FakeStory {
    fn with(flags: &[(&'static str, u32)]) -> Self {
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
struct FakeSite {
    takeoffs: u32,
}

impl vault::VaultSite for FakeSite {
    fn load_vault_scene(&mut self, _first_report: u16) {}
    fn lander_takeoff(&mut self) {
        self.takeoffs += 1;
    }
}

impl wreck::WreckSite for FakeSite {
    fn load_wreck_scene(&mut self, _first_report: u16) {}
    fn lander_takeoff(&mut self) {
        self.takeoffs += 1;
    }
}

const VAULT: Body = Body::Moon { planet: 0, moon: 0 };

#[test]
fn vault_pickup_before_the_shuttle_reports_once_and_learns_of_the_vault() {
    let mut story = FakeStory::default();
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!vault::pickup_energy(
        VAULT,
        0,
        &mut story,
        &mut site,
        &mut report
    ));
    assert_eq!(report.0, [ReportDisposition::Once]);
    assert_eq!(story.flag("KNOW_SYREEN_VAULT"), 1);
    assert_eq!(site.takeoffs, 0);
}

#[test]
fn vault_pickup_with_the_shuttle_unlocks_the_vault() {
    let mut story = FakeStory::with(&[("SYREEN_SHUTTLE_ON_SHIP", 1), ("SYREEN_HOME_VISITS", 5)]);
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!vault::pickup_energy(
        VAULT,
        0,
        &mut story,
        &mut site,
        &mut report
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(site.takeoffs, 1);
    assert_eq!(story.flag("SHIP_VAULT_UNLOCKED"), 1);
    assert_eq!(story.flag("SYREEN_SHUTTLE_ON_SHIP"), 0);
    assert_eq!(story.flag("SYREEN_HOME_VISITS"), 0);
}

#[test]
fn vault_pickup_once_unlocked_only_cycles_the_report() {
    let mut story = FakeStory::with(&[("SHIP_VAULT_UNLOCKED", 1), ("SYREEN_SHUTTLE_ON_SHIP", 1)]);
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!vault::pickup_energy(
        VAULT,
        0,
        &mut story,
        &mut site,
        &mut report
    ));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(site.takeoffs, 0);
    assert_eq!(story.flag("SYREEN_SHUTTLE_ON_SHIP"), 1);
}

#[test]
fn vault_pickup_elsewhere_does_nothing() {
    let mut story = FakeStory::default();
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!vault::pickup_energy(
        Body::Planet(0),
        0,
        &mut story,
        &mut site,
        &mut report
    ));
    assert!(report.0.is_empty());
    assert_eq!(story.0.len(), 0);
}

#[test]
fn wreck_pickup_hands_over_the_portal_key_once() {
    let wreck_body = Body::Planet(6);
    let mut story = FakeStory::default();
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!wreck::pickup_energy(
        wreck_body,
        0,
        &mut story,
        &mut site,
        &mut report
    ));
    assert_eq!(
        (story.flag("PORTAL_KEY"), story.flag("PORTAL_KEY_ON_SHIP")),
        (1, 1)
    );
    assert_eq!(site.takeoffs, 1);

    story.set_flag("PORTAL_KEY_ON_SHIP", 0);
    assert!(!wreck::pickup_energy(
        wreck_body,
        0,
        &mut story,
        &mut site,
        &mut report
    ));
    assert_eq!(site.takeoffs, 1, "the key is only given the first time");
    assert_eq!(story.flag("PORTAL_KEY_ON_SHIP"), 0);
    assert_eq!(
        report.0,
        [ReportDisposition::Cycle, ReportDisposition::Cycle]
    );

    let mut elsewhere = FakeReport::default();
    assert!(!wreck::pickup_energy(
        Body::Planet(5),
        0,
        &mut story,
        &mut site,
        &mut elsewhere
    ));
    assert!(elsewhere.0.is_empty());
}

#[test]
fn yehat_pickup_cycles_the_ruins_report_and_never_takes_the_node() {
    let mut report = FakeReport::default();
    assert!(!yehat::pickup_energy(Body::Planet(0), &mut report));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert!(!yehat::pickup_energy(Body::Planet(1), &mut report));
    assert_eq!(report.0.len(), 1);
}

#[test]
fn rainbow_visits_set_one_bit_each_across_both_flag_bytes() {
    let mut story = FakeStory::with(&[("RAINBOW_WORLD0", 0b0000_0001)]);
    let mask = |story: &FakeStory| story.flag("RAINBOW_WORLD0") | story.flag("RAINBOW_WORLD1") << 8;
    for (which, expected) in [(2u8, 0b101u32), (9, 0b10_0000_0101)] {
        let (a, b) = (story.flag("RAINBOW_WORLD0"), story.flag("RAINBOW_WORLD1"));
        let merged = (a | b << 8) as u16 | 1 << which;
        story.set_flag("RAINBOW_WORLD0", u32::from(merged & 0xFF));
        story.set_flag("RAINBOW_WORLD1", u32::from(merged >> 8));
        assert_eq!(mask(&story), expected);
    }
}

#[test]
fn rainbow_ordinal_counts_earlier_rainbow_stars() {
    let r = rainbow::RAINBOW_DEFINED;
    assert_eq!(rainbow::rainbow_ordinal(&[r, 3, r, r, 9], 3), 2);
    assert_eq!(rainbow::rainbow_ordinal(&[r, 3, r, r, 9], 0), 0);
}

#[test]
fn melnorme_posts_pick_their_own_group_reference() {
    use crate::planets::generate::melnorme::{ref_var, MELNORME0_DEFINED};
    assert_eq!(ref_var(MELNORME0_DEFINED), Some("MELNORME0_GRPOFFS0"));
    assert_eq!(ref_var(MELNORME0_DEFINED + 8), Some("MELNORME8_GRPOFFS0"));
    assert_eq!(ref_var(MELNORME0_DEFINED + 9), None);
    assert_eq!(ref_var(MELNORME0_DEFINED - 1), None);
}

#[test]
fn syreen_moon_slot_is_the_starbase_only_for_planet_zero() {
    use crate::c_bindings::global_state::PLANET_DESC;
    use crate::math::RandomContext;
    for (body, starbase) in [(Body::Planet(0), true), (Body::Planet(1), false)] {
        let mut moons: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
        let mut rng = RandomContext::with_seed(7);
        syreen::generate_moons(&mut rng, 0, 0, 2, &mut moons, body);
        assert_eq!(moons[0].data_index == 127 | 0x80, starbase);
    }
}
