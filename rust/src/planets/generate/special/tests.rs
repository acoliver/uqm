//! Story gating and side effects of the small per-system generators, with a
//! fake game state, report and sites.

use crate::engine::StoryKey;
use std::collections::HashMap;

use crate::engine::world::queues::IpGroup as IP_GROUP;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planet_side::runtime::AdapterError;
use crate::planets::generate::default::Npcs;
use crate::planets::generate::special::{
    Body, FleetGroups, Story, MELNORME_SHIP, URQUAN_SHIP, ZOQFOTPIK_SHIP,
};
use crate::planets::generate::{colony, rainbow, syreen, vault, wreck, yehat, zfpscout};

#[derive(Default)]
struct FakeStory(HashMap<StoryKey, u32>);

impl FakeStory {
    fn with(flags: &[(StoryKey, u32)]) -> Self {
        Self(flags.iter().copied().collect())
    }
}

impl Story for FakeStory {
    fn flag(&self, name: StoryKey) -> u32 {
        self.0.get(&name).copied().unwrap_or(0)
    }
    fn set_flag(&mut self, name: StoryKey, value: u32) {
        self.0.insert(name, value);
    }
    fn word(&self, name: StoryKey) -> u32 {
        self.flag(name)
    }
    fn set_word(&mut self, name: StoryKey, value: u32) {
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
    fn load_vault_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _first_report: u16,
    ) {
    }
    fn lander_takeoff(&mut self) {
        self.takeoffs += 1;
    }
}

impl wreck::WreckSite for FakeSite {
    fn load_wreck_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _first_report: u16,
    ) {
    }
    fn lander_takeoff(&mut self) {
        self.takeoffs += 1;
    }
}

const VAULT: Body = Body::Moon { planet: 0, moon: 0 };

#[test]
fn vault_pickup_before_the_shuttle_reports_once_and_learns_of_the_vault() {
    let mut story = FakeStory::default();
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        vault::pickup_energy(
            system,
            VAULT,
            0,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut site,
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert_eq!(report.0, [ReportDisposition::Once]);
    assert_eq!(story.flag(StoryKey::KNOW_SYREEN_VAULT), 1);
    assert_eq!(site.takeoffs, 0);
}

#[test]
fn vault_pickup_with_the_shuttle_unlocks_the_vault() {
    let mut story = FakeStory::with(&[
        (StoryKey::SYREEN_SHUTTLE_ON_SHIP, 1),
        (StoryKey::SYREEN_HOME_VISITS, 5),
    ]);
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        vault::pickup_energy(
            system,
            VAULT,
            0,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut site,
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(site.takeoffs, 1);
    assert_eq!(story.flag(StoryKey::SHIP_VAULT_UNLOCKED), 1);
    assert_eq!(story.flag(StoryKey::SYREEN_SHUTTLE_ON_SHIP), 0);
    assert_eq!(story.flag(StoryKey::SYREEN_HOME_VISITS), 0);
}

#[test]
fn vault_pickup_once_unlocked_only_cycles_the_report() {
    let mut story = FakeStory::with(&[
        (StoryKey::SHIP_VAULT_UNLOCKED, 1),
        (StoryKey::SYREEN_SHUTTLE_ON_SHIP, 1),
    ]);
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        vault::pickup_energy(
            system,
            VAULT,
            0,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut site,
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert_eq!(site.takeoffs, 0);
    assert_eq!(story.flag(StoryKey::SYREEN_SHUTTLE_ON_SHIP), 1);
}

#[test]
fn vault_pickup_elsewhere_does_nothing() {
    let mut story = FakeStory::default();
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        vault::pickup_energy(
            system,
            Body::Planet(0),
            0,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut site,
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert!(report.0.is_empty());
    assert_eq!(story.0.len(), 0);
}

#[test]
fn wreck_pickup_hands_over_the_portal_key_once() {
    let wreck_body = Body::Planet(6);
    let mut story = FakeStory::default();
    let (mut site, mut report) = (FakeSite::default(), FakeReport::default());
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        wreck::pickup_energy(
            system,
            wreck_body,
            0,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut site,
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert_eq!(
        (
            story.flag(StoryKey::PORTAL_KEY),
            story.flag(StoryKey::PORTAL_KEY_ON_SHIP)
        ),
        (1, 1)
    );
    assert_eq!(site.takeoffs, 1);

    story.set_flag(StoryKey::PORTAL_KEY_ON_SHIP, 0);
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        wreck::pickup_energy(
            system,
            wreck_body,
            0,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut site,
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert_eq!(site.takeoffs, 1, "the key is only given the first time");
    assert_eq!(story.flag(StoryKey::PORTAL_KEY_ON_SHIP), 0);
    assert_eq!(
        report.0,
        [ReportDisposition::Cycle, ReportDisposition::Cycle]
    );

    let mut elsewhere = FakeReport::default();
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        wreck::pickup_energy(
            system,
            Body::Planet(5),
            0,
            &mut crate::planets::generate::testing::Effects {
                story: &mut story,
                site: &mut site,
                report: &mut elsewhere,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert!(elsewhere.0.is_empty());
}

#[test]
fn yehat_pickup_cycles_the_ruins_report_and_never_takes_the_node() {
    let mut report = FakeReport::default();
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        yehat::pickup_energy(
            system,
            Body::Planet(0),
            &mut crate::planets::generate::testing::Effects {
                story: (),
                site: &mut (),
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert_eq!(report.0, [ReportDisposition::Cycle]);
    assert!(!crate::planets::generate::testing::with_empty(|system| {
        yehat::pickup_energy(
            system,
            Body::Planet(1),
            &mut crate::planets::generate::testing::Effects {
                story: (),
                site: &mut (),
                report: &mut report,
                lander: (),
                load: Some(|| {}),
            },
        )
    }));
    assert_eq!(report.0.len(), 1);
}

#[test]
fn rainbow_visits_set_one_bit_each_across_both_flag_bytes() {
    let mut story = FakeStory::with(&[(StoryKey::RAINBOW_WORLD0, 0b0000_0001)]);
    let mask = |story: &FakeStory| {
        story.flag(StoryKey::RAINBOW_WORLD0) | story.flag(StoryKey::RAINBOW_WORLD1) << 8
    };
    for (which, expected) in [(2u8, 0b101u32), (9, 0b10_0000_0101)] {
        let (a, b) = (
            story.flag(StoryKey::RAINBOW_WORLD0),
            story.flag(StoryKey::RAINBOW_WORLD1),
        );
        let merged = (a | b << 8) as u16 | 1 << which;
        story.set_flag(StoryKey::RAINBOW_WORLD0, u32::from(merged & 0xFF));
        story.set_flag(StoryKey::RAINBOW_WORLD1, u32::from(merged >> 8));
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
    assert_eq!(
        ref_var(MELNORME0_DEFINED),
        Some(StoryKey::MELNORME0_GRPOFFS0)
    );
    assert_eq!(
        ref_var(MELNORME0_DEFINED + 8),
        Some(StoryKey::MELNORME8_GRPOFFS0)
    );
    assert_eq!(ref_var(MELNORME0_DEFINED + 9), None);
    assert_eq!(ref_var(MELNORME0_DEFINED - 1), None);
}

#[test]
fn syreen_moon_slot_is_the_starbase_only_for_planet_zero() {
    use crate::engine::world::planet::PlanetDesc;
    use crate::math::RandomContext;
    for (body, starbase) in [(Body::Planet(0), true), (Body::Planet(1), false)] {
        let mut moons: Vec<PlanetDesc> = vec![unsafe { std::mem::zeroed() }; 16];
        let mut rng = RandomContext::with_seed(7);
        syreen::generate_moons(&mut rng, 0, 0, 2, &mut moons, body);
        assert_eq!(moons[0].data_index == 127 | 0x80, starbase);
    }
}

/// A byte-addressed store of the game state bits, as `getGameState` sees it.
#[test]
fn story_words_are_stored_least_significant_byte_first() {
    let key = StoryKey::new(40, 47);
    let mut state = crate::state::game_state::StoryState::new();
    state.set_word(key, 0x1234_ABCD);
    assert_eq!(
        state.as_bytes().iter().filter(|&&byte| byte != 0).count(),
        4
    );
    assert_eq!(state.get_state(40, 47), 0xCD);
    assert_eq!(state.get_state(48, 55), 0xAB);
    assert_eq!(state.get_state(56, 63), 0x34);
    assert_eq!(state.get_state(64, 71), 0x12);
    assert_eq!(state.word(key), 0x1234_ABCD);
    // Bytes the word does not cover are not read.
    state.set_state(72, 79, 0xFF);
    assert_eq!(state.word(key), 0x1234_ABCD);
}

/// The ship groups, queues and group references one system keeps.
#[derive(Default)]
struct FakeFleet {
    group_ref: u32,
    next_group: u32,
    resume_succeeds: bool,
    yehat_civil_war: bool,
    built_clones: Vec<u8>,
    built_reinits: u32,
    groups: Vec<IP_GROUP>,
    built_groups: u32,
    loads: u32,
    cleared: u32,
}

impl Npcs for FakeFleet {
    fn init_ip_groups(&mut self) -> bool {
        self.resume_succeeds
    }
    fn clear_battle_group_ref(&mut self) {
        self.cleared += 1;
        self.group_ref = 0;
    }
    fn build_groups(&mut self) {
        self.built_groups += 1;
    }
    fn load_ip_groups(&mut self) {
        self.loads += 1;
    }
    fn save_ip_groups(&mut self) {}
    fn reinit_npc_built_ship_q(&mut self) {}
    fn reinit_ip_group_q(&mut self) {}
    fn yehat_civil_war(&self) -> bool {
        self.yehat_civil_war
    }
    fn encounter_race(&self) -> i16 {
        0
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        self.groups.iter_mut().for_each(visit);
    }
}

impl FleetGroups for FakeFleet {
    fn battle_group_ref(&self) -> u32 {
        self.group_ref
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        self.group_ref = value;
    }
    fn clone_ship_into_built_queue(&mut self, ship: u8) {
        self.built_clones.push(ship);
    }
    fn add_new_group(&mut self) -> u32 {
        self.next_group += 1;
        self.next_group
    }
    fn reinit_built_queue(&mut self) {
        self.built_reinits += 1;
    }
    fn head_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) -> bool {
        if let Some(group) = self.groups.first_mut() {
            visit(group);
            true
        } else {
            false
        }
    }
}

fn group(race_id: u8, task: u8) -> IP_GROUP {
    let mut group: IP_GROUP = unsafe { std::mem::zeroed() };
    group.race_id = race_id;
    group.task = task;
    group.sys_loc = 9;
    group.dest_loc = 9;
    group.loc.x = 5;
    group.loc.y = 5;
    group.group_counter = 5;
    group
}

fn fleet(group_ref: u32) -> FakeFleet {
    FakeFleet {
        group_ref,
        next_group: 40,
        resume_succeeds: true,
        groups: vec![group(URQUAN_SHIP, 0)],
        ..FakeFleet::default()
    }
}

#[test]
fn colony_builds_its_group_once_and_parks_it_in_orbit() {
    let mut story = FakeStory::default();
    let mut host = fleet(0);
    assert!(colony::init_npcs(&mut (&mut story, &mut host)));
    assert_eq!(host.built_clones, [URQUAN_SHIP]);
    assert_eq!(host.built_reinits, 1);
    assert_eq!(story.word(StoryKey::COLONY_GRPOFFS0), 41);
    assert_eq!(host.group_ref, 41);
    let parked = &host.groups[0];
    assert_eq!(
        (parked.task, parked.sys_loc, parked.dest_loc),
        (IP_GROUP::IN_ORBIT, 1, 1)
    );
    assert_eq!(
        (parked.loc.x, parked.loc.y, parked.group_counter),
        (0, 0, 0)
    );

    // A stored reference is reused, and the group is parked again.
    let mut story = FakeStory::with(&[(StoryKey::COLONY_GRPOFFS0, 7)]);
    let mut host = fleet(0);
    host.groups[0].task = 3;
    assert!(colony::init_npcs(&mut (&mut story, &mut host)));
    assert!(host.built_clones.is_empty());
    assert_eq!(host.group_ref, 7);
    assert_eq!(host.groups[0].task, IP_GROUP::IN_ORBIT);
}

#[test]
fn colony_without_a_group_in_the_queue_parks_nothing() {
    let mut story = FakeStory::with(&[(StoryKey::COLONY_GRPOFFS0, 7)]);
    let mut host = fleet(0);
    host.groups.clear();
    assert!(colony::init_npcs(&mut (&mut story, &mut host)));
    // The default init found no saved groups to resume and dropped the ref.
    let mut host = fleet(0);
    host.resume_succeeds = false;
    assert!(colony::init_npcs(&mut (&mut story, &mut host)));
    assert_eq!((host.cleared, host.built_groups, host.group_ref), (1, 1, 0));
    assert_eq!(host.groups[0].task, 0);
}

#[test]
fn melnorme_known_post_reuses_or_stores_its_reference() {
    use crate::planets::generate::melnorme::{self, MELNORME0_DEFINED};
    let mut story = FakeStory::with(&[(StoryKey::MELNORME2_GRPOFFS0, 12)]);
    let mut host = fleet(0);
    assert!(melnorme::init_npcs(
        MELNORME0_DEFINED + 2,
        &mut (&mut story, &mut host)
    ));
    assert!(host.built_clones.is_empty());
    assert_eq!(host.group_ref, 12);

    let mut host = fleet(0);
    assert!(melnorme::init_npcs(
        MELNORME0_DEFINED + 3,
        &mut (&mut story, &mut host)
    ));
    assert_eq!(host.built_clones, [MELNORME_SHIP]);
    assert_eq!(story.word(StoryKey::MELNORME3_GRPOFFS0), 41);
    assert_eq!(host.group_ref, 41);
}

#[test]
fn melnorme_unknown_star_builds_a_group_it_cannot_remember() {
    use crate::planets::generate::melnorme;
    let mut story = FakeStory::default();
    let mut host = fleet(99);
    assert!(melnorme::init_npcs(
        MELNORME0_DEFINED_PAST_END,
        &mut (&mut story, &mut host)
    ));
    assert_eq!(host.built_clones, [MELNORME_SHIP]);
    assert_eq!((host.group_ref, host.built_reinits), (41, 1));
    assert!(
        story.0.is_empty(),
        "no reference is stored for an unknown star"
    );
}

const MELNORME0_DEFINED_PAST_END: u8 = crate::planets::generate::melnorme::MELNORME0_DEFINED + 9;

#[test]
fn zoqfotpik_scout_builds_its_group_only_before_the_meeting() {
    let mut story = FakeStory::default();
    let mut host = fleet(0);
    assert!(zfpscout::init_npcs(&mut (&mut story, &mut host)));
    assert_eq!(host.built_clones, [ZOQFOTPIK_SHIP]);
    assert_eq!(story.word(StoryKey::ZOQFOT_GRPOFFS0), 41);

    let mut story = FakeStory::with(&[(StoryKey::ZOQFOT_GRPOFFS0, 8)]);
    let mut host = fleet(0);
    assert!(zfpscout::init_npcs(&mut (&mut story, &mut host)));
    assert!(host.built_clones.is_empty());
    assert_eq!(host.group_ref, 8);

    // After the meeting the reference is not read at all.
    let mut story = FakeStory::with(&[(StoryKey::MET_ZOQFOT, 1), (StoryKey::ZOQFOT_GRPOFFS0, 8)]);
    let mut host = fleet(55);
    assert!(zfpscout::init_npcs(&mut (&mut story, &mut host)));
    assert!(host.built_clones.is_empty());
    assert_eq!(host.group_ref, 55);
}

#[test]
fn zoqfotpik_scout_flees_only_after_colliding_with_the_flagship() {
    let collided = IP_GROUP::FLEE | IP_GROUP::IGNORE_FLAGSHIP | IP_GROUP::REFORM_GROUP;
    for (race, task, group_ref, flees) in [
        (ZOQFOTPIK_SHIP, IP_GROUP::REFORM_GROUP, 3, true),
        (ZOQFOTPIK_SHIP, 0, 3, false),
        (URQUAN_SHIP, IP_GROUP::REFORM_GROUP, 3, false),
        (ZOQFOTPIK_SHIP, IP_GROUP::REFORM_GROUP, 0, false),
    ] {
        let mut host = fleet(group_ref);
        host.groups = vec![group(race, task)];
        assert!(zfpscout::reinit_npcs(&mut host));
        assert_eq!(host.loads, 1);
        let head = &host.groups[0];
        assert_eq!(head.task, if flees { collided } else { task });
        assert_eq!(head.dest_loc, if flees { 0 } else { 9 });
    }
    let mut host = fleet(3);
    host.groups.clear();
    assert!(zfpscout::reinit_npcs(&mut host));
}

#[test]
fn owned_story_implements_generator_flags_and_group_words_without_a_live_owner() {
    let mut first = crate::engine::world::story::StoryState::new();
    let second = crate::engine::world::story::StoryState::new();
    first.set_flag(StoryKey::TAALO_UNPROTECTED, 1);
    first.set_word(StoryKey::SAMATRA_GRPOFFS0, 0x89abcdef);
    assert_eq!(first.flag(StoryKey::TAALO_UNPROTECTED), 1);
    assert_eq!(second.flag(StoryKey::TAALO_UNPROTECTED), 0);
    assert_eq!(first.word(StoryKey::SAMATRA_GRPOFFS0), 0x89abcdef);
    assert_eq!(second.word(StoryKey::SAMATRA_GRPOFFS0), 0);
    let start = crate::engine::StoryKey::SAMATRA_GRPOFFS0.start();
    for (byte, expected) in [0xef, 0xcd, 0xab, 0x89].into_iter().enumerate() {
        assert_eq!(
            first.get_state(start + 8 * byte, start + 8 * byte + 7),
            expected
        );
    }
}
