//! The Shofixti group's story gating against a recording fake host.

use super::{
    generate_planets, init_npcs, reinit_npcs, uninit_npcs, ShofixtiSite, METAL_WORLD, NUM_PLANETS,
};
use crate::c_bindings::global_state::{IP_GROUP, PLANET_DESC};
use crate::comm::dispatch::ship::{SHOFIXTI_SHIP, UMGAH_SHIP};
use crate::math::RandomContext;
use crate::planets::generate::default::{Npcs, FLEE, IGNORE_FLAGSHIP, REFORM_GROUP};
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::special::{FleetGroups, Story, SELENIC_WORLD};

#[derive(Debug, PartialEq, Eq)]
enum Call {
    InitIpGroups,
    ClearBattleGroupRef,
    BuildGroups,
    LoadIpGroups,
    SaveIpGroups,
    ReinitNpcBuiltShipQ,
    ReinitIpGroupQ,
    CloneShofixtiNamed(u8),
    PutGroupInfo(u32),
    ReinitBuiltQueue,
}

struct Host {
    group_ref: u32,
    /// What `GetGroupInfo (.., GROUP_INIT_IP)` answers.
    resume_succeeds: bool,
    /// What `PutGroupInfo` returns.
    stored: u32,
    groups: Vec<IP_GROUP>,
    calls: Vec<Call>,
}

impl Host {
    fn new(group_ref: u32, resume_succeeds: bool) -> Self {
        Self {
            group_ref,
            resume_succeeds,
            stored: 77,
            groups: Vec::new(),
            calls: Vec::new(),
        }
    }
}

impl Npcs for Host {
    fn init_ip_groups(&mut self) -> bool {
        self.calls.push(Call::InitIpGroups);
        self.resume_succeeds
    }
    fn clear_battle_group_ref(&mut self) {
        self.calls.push(Call::ClearBattleGroupRef);
        self.group_ref = 0;
    }
    fn build_groups(&mut self) {
        self.calls.push(Call::BuildGroups);
    }
    fn load_ip_groups(&mut self) {
        self.calls.push(Call::LoadIpGroups);
    }
    fn save_ip_groups(&mut self) {
        self.calls.push(Call::SaveIpGroups);
    }
    fn reinit_npc_built_ship_q(&mut self) {
        self.calls.push(Call::ReinitNpcBuiltShipQ);
    }
    fn reinit_ip_group_q(&mut self) {
        self.calls.push(Call::ReinitIpGroupQ);
    }
    fn yehat_civil_war(&self) -> bool {
        false
    }
    fn encounter_race(&self) -> i16 {
        0
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        self.groups.iter_mut().for_each(visit);
    }
}

impl FleetGroups for Host {
    fn battle_group_ref(&self) -> u32 {
        self.group_ref
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        self.group_ref = value;
    }
    fn clone_ship_into_built_queue(&mut self, _ship: u8) {
        panic!("the Shofixti clone carries a name");
    }
    fn add_new_group(&mut self) -> u32 {
        panic!("the Shofixti group is stored with its own reference");
    }
    fn reinit_built_queue(&mut self) {
        self.calls.push(Call::ReinitBuiltQueue);
    }
    fn head_ip_group(&mut self) -> Option<&mut IP_GROUP> {
        self.groups.first_mut()
    }
}

impl ShofixtiSite for Host {
    fn clone_shofixti_named(&mut self, name_index: u8) {
        self.calls.push(Call::CloneShofixtiNamed(name_index));
    }
    fn put_group_info(&mut self, offset: u32) -> u32 {
        self.calls.push(Call::PutGroupInfo(offset));
        self.stored
    }
}

fn group(race_id: u8, task: u8) -> IP_GROUP {
    let mut group: IP_GROUP = unsafe { std::mem::zeroed() };
    group.race_id = race_id;
    group.task = task;
    group.dest_loc = 9;
    group
}

fn story(recruited: u32, kia: u32, bro_kia: u32, maidens: u32) -> FakeStory {
    FakeStory::with(&[
        ("SHOFIXTI_RECRUITED", recruited),
        ("SHOFIXTI_KIA", kia),
        ("SHOFIXTI_BRO_KIA", bro_kia),
        ("MAIDENS_ON_SHIP", maidens),
    ])
}

/// The groups are stored when nobody has recruited the Shofixti and either
/// Tanaka lives or Katana does and the maidens are aboard.
#[test]
fn a_group_is_made_exactly_when_a_shofixti_remains() {
    for bits in 0..16u32 {
        let (recruited, kia, bro_kia, maidens) =
            (bits & 1, (bits >> 1) & 1, (bits >> 2) & 1, (bits >> 3) & 1);
        let remains = recruited == 0 && (kia == 0 || (bro_kia == 0 && maidens != 0));
        let mut story = story(recruited, kia, bro_kia, maidens);
        let mut host = Host::new(0, true);
        assert!(init_npcs(&mut story, &mut host));
        let cloned = host
            .calls
            .iter()
            .any(|call| matches!(call, Call::CloneShofixtiNamed(_)));
        assert_eq!(cloned, remains, "{bits:04b}");
        assert_eq!(
            story.word("SHOFIXTI_GRPOFFS0"),
            if remains { 77 } else { 0 },
            "{bits:04b}"
        );
    }
}

#[test]
fn the_new_group_is_named_for_the_shofixti_who_lives() {
    for (kia, name) in [(0, 21), (1, 22)] {
        // Katana lives only when the maidens are aboard.
        let mut story = story(0, kia, 0, 1);
        let mut host = Host::new(0, true);
        init_npcs(&mut story, &mut host);
        assert!(
            host.calls.contains(&Call::CloneShofixtiNamed(name)),
            "{kia}"
        );
    }
}

#[test]
fn a_missing_reference_is_stored_as_all_ones_until_the_group_exists() {
    let mut story = story(0, 0, 0, 0);
    let mut host = Host::new(0, true);
    init_npcs(&mut story, &mut host);
    assert_eq!(
        host.calls,
        [
            Call::CloneShofixtiNamed(21),
            Call::PutGroupInfo(!0),
            Call::ReinitBuiltQueue,
            Call::InitIpGroups,
        ],
        "the reference is read without asking for the groups, and the default \
         init then finds them"
    );
    assert_eq!(host.group_ref, 77);
    assert_eq!(story.word("SHOFIXTI_GRPOFFS0"), 77);
}

#[test]
fn a_stored_reference_resumes_its_group_and_a_lost_one_is_put_again() {
    let mut stored = story(0, 0, 0, 0);
    stored.set_word("SHOFIXTI_GRPOFFS0", 5);

    let mut host = Host::new(0, true);
    init_npcs(&mut stored, &mut host);
    assert_eq!(host.group_ref, 5);
    assert_eq!(host.calls, [Call::InitIpGroups, Call::InitIpGroups]);

    let mut host = Host::new(0, false);
    init_npcs(&mut stored, &mut host);
    assert_eq!(
        host.calls,
        [
            Call::InitIpGroups,
            Call::CloneShofixtiNamed(21),
            Call::PutGroupInfo(5),
            Call::ReinitBuiltQueue,
            Call::InitIpGroups,
            Call::ClearBattleGroupRef,
            Call::BuildGroups,
        ]
    );
    assert_eq!(stored.word("SHOFIXTI_GRPOFFS0"), 77);
}

#[test]
fn nothing_is_stored_once_the_shofixti_are_recruited() {
    let mut story = story(1, 0, 0, 0);
    let mut host = Host::new(9, true);
    assert!(init_npcs(&mut story, &mut host));
    assert_eq!(host.group_ref, 9);
    assert_eq!(host.calls, [Call::InitIpGroups]);
}

#[test]
fn a_recruited_shofixti_the_flagship_collided_with_flees() {
    let collided = FLEE | IGNORE_FLAGSHIP | REFORM_GROUP;
    for (race, task, recruited, group_ref, flees) in [
        (SHOFIXTI_SHIP, REFORM_GROUP, 1, 3, true),
        (SHOFIXTI_SHIP, REFORM_GROUP, 0, 3, false),
        (SHOFIXTI_SHIP, 0, 1, 3, false),
        (UMGAH_SHIP, REFORM_GROUP, 1, 3, false),
        (SHOFIXTI_SHIP, REFORM_GROUP, 1, 0, false),
    ] {
        let story = story(recruited, 0, 0, 0);
        let mut host = Host::new(group_ref, true);
        host.groups = vec![group(race, task)];
        assert!(reinit_npcs(&story, &mut host));
        assert_eq!(host.calls, [Call::LoadIpGroups]);
        let head = &host.groups[0];
        assert_eq!(head.task, if flees { collided } else { task });
        assert_eq!(head.dest_loc, if flees { 0 } else { 9 });
    }

    // Recruited and referenced, but nobody in the system.
    let story = story(1, 0, 0, 0);
    let mut host = Host::new(3, true);
    assert!(reinit_npcs(&story, &mut host));
}

#[test]
fn init_checks_the_old_shofixti_after_the_default_groups() {
    let mut story = story(1, 0, 0, 0);
    let mut host = Host::new(3, true);
    host.groups = vec![group(SHOFIXTI_SHIP, REFORM_GROUP)];
    init_npcs(&mut story, &mut host);
    assert_eq!(host.groups[0].task, FLEE | IGNORE_FLAGSHIP | REFORM_GROUP);
}

#[test]
fn a_missing_group_means_a_shofixti_died() {
    let saved_and_emptied = [
        Call::SaveIpGroups,
        Call::ReinitNpcBuiltShipQ,
        Call::ReinitIpGroupQ,
    ];
    // (kia, bro_kia, maidens, group_ref, recruited, group present)
    // -> (kia, bro_kia, visits) afterwards, visits starting at 5.
    for ((kia, bro_kia, maidens, group_ref, recruited, present), expect) in [
        // Tanaka dies: killed, and the visit count restarts.
        ((0, 0, 0, 3, 0, false), (1, 0, 0)),
        // Katana dies, with the maidens aboard.
        ((1, 0, 1, 3, 0, false), (1, 1, 5)),
        // Katana's death without the maidens aboard is not recorded.
        ((1, 0, 0, 3, 0, false), (1, 0, 5)),
        // A group is still there, there is no reference, or already recruited.
        ((0, 0, 0, 3, 0, true), (0, 0, 5)),
        ((0, 0, 0, 0, 0, false), (0, 0, 5)),
        ((0, 0, 0, 3, 1, false), (0, 0, 5)),
    ] {
        let mut story = story(recruited, kia, bro_kia, maidens);
        story.set_flag("SHOFIXTI_VISITS", 5);
        let mut host = Host::new(group_ref, true);
        if present {
            host.groups = vec![group(SHOFIXTI_SHIP, 0)];
        }
        assert!(uninit_npcs(&mut story, &mut host));
        assert_eq!(host.calls, saved_and_emptied);
        assert_eq!(
            (
                story.flag("SHOFIXTI_KIA"),
                story.flag("SHOFIXTI_BRO_KIA"),
                story.flag("SHOFIXTI_VISITS")
            ),
            expect,
            "{kia}{bro_kia}{maidens}{group_ref}{recruited}{present}"
        );
    }
}

#[test]
fn three_selenic_and_three_metal_worlds_with_no_moons() {
    let mut rng = RandomContext::with_seed(3);
    let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
    for planet in &mut planets {
        planet.NumPlanets = 4;
    }
    let drawn = generate_planets(&mut rng, 0, 0, &mut planets);
    assert_eq!(drawn, NUM_PLANETS);
    // `FillOrbits` orders the worlds by orbit, so only the counts are fixed.
    let count = |kind| {
        planets
            .iter()
            .take(6)
            .filter(|p| p.data_index == kind)
            .count()
    };
    assert_eq!((count(SELENIC_WORLD), count(METAL_WORLD)), (3, 3));
    assert!(planets.iter().take(6).all(|p| p.NumPlanets == 0));
    assert!(planets.iter().skip(6).all(|p| p.NumPlanets == 4));
}
