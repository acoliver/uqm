use super::*;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::vux::tests::{record, with_orbit, Site};
use std::rc::Rc;

#[derive(Default)]
struct Guard {
    calls: Vec<String>,
    reference: u32,
    groups: Vec<IP_GROUP>,
    built: u16,
    moved: Option<POINT>,
}
impl GuardSite for Guard {
    fn save_ip_groups(&mut self) {
        self.calls.push("save".into());
    }
    fn load_ip_groups(&mut self) {
        self.calls.push("load".into());
    }
    fn reset_encounter(&mut self) {
        self.calls.push("reset".into());
    }
    fn built_ship_count(&self) -> u16 {
        self.built
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        self.calls.push(format!("ref:{value}"));
        self.reference = value;
    }
    fn battle_group_ref(&self) -> u32 {
        self.reference
    }
    fn clone_ship(&mut self, race: u8) {
        self.calls.push(format!("clone:{race}"));
    }
    fn add_new_group(&mut self) -> u32 {
        self.calls.push("add".into());
        7919
    }
    fn store_group(&mut self, reference: u32, group: u8) {
        self.calls.push(format!("store:{reference}:{group}"));
    }
    fn reinit_built_queue(&mut self) {
        self.calls.push("clear".into());
    }
    fn init_ip_groups(&mut self, reference: u32) {
        self.calls.push(format!("init:{reference}"));
    }
    fn planet_outer_location(&self, planet: u16) -> POINT {
        assert_eq!(planet, 4);
        POINT { x: 100, y: -200 }
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        self.groups.iter_mut().for_each(visit);
    }
    fn ip_location(&self) -> POINT {
        POINT { x: -900, y: 800 }
    }
    fn set_and_transform_ip_location(&mut self, value: POINT) {
        self.calls.push("transform".into());
        self.moved = Some(value);
    }
}
fn groups(count: usize) -> Vec<IP_GROUP> {
    (0..count)
        .map(|i| {
            let mut g: IP_GROUP = unsafe { std::mem::zeroed() };
            g.race_id = i as u8;
            g.group_id = 27;
            g.group_counter = 0xabcd;
            g.sys_loc = 11;
            g.dest_loc = 13;
            g.orbit_pos = 15;
            g.in_system = 17;
            g.loc = POINT { x: -23, y: 29 };
            g
        })
        .collect()
}

#[test]
fn guard_build_order_races_and_full_groups() {
    for frenzy in [0, 1] {
        for reference in [0, u32::MAX] {
            let mut story = FakeStory::with(&[("KOHR_AH_FRENZY", frenzy)]);
            story.set_word("SAMATRA_GRPOFFS0", reference);
            let mut site = Guard {
                groups: groups(17),
                ..Guard::default()
            };
            let mut expected_groups = site.groups.clone();
            assert!(init_npcs(&mut story, &mut site));
            let stored = if reference == 0 { 7919 } else { reference };
            assert_eq!(story.word("SAMATRA_GRPOFFS0"), stored);
            let first = if frenzy == 0 { 18 } else { 21 };
            let second = if frenzy == 0 { 21 } else { 18 };
            let mut expected = vec![format!("ref:{reference}")];
            expected.extend((0..14).map(|_| format!("clone:{first}")));
            if reference == 0 {
                expected.extend(["add".into(), "ref:7919".into()]);
            }
            expected.extend((1..=12).map(|i| format!("store:{stored}:{i}")));
            expected.push("clear".into());
            expected.extend((0..14).map(|_| format!("clone:{second}")));
            expected.extend((13..=16).map(|i| format!("store:{stored}:{i}")));
            expected.extend(["clear".into(), format!("init:{stored}")]);
            assert_eq!(site.calls, expected);
            let angles = [4, 8, 12, 20, 24, 28, 36, 40, 44, 52, 56, 60, 0, 16, 32, 48];
            for (g, angle) in expected_groups.iter_mut().zip(angles) {
                g.task = 7;
                g.sys_loc = 0;
                g.dest_loc = 5;
                g.orbit_pos = (((angle + 2) >> 2) & 15) as u8;
                g.group_counter = 0;
                g.loc = POINT {
                    x: (100 + cosine(angle, 1600)) as i16,
                    y: (-200 + sine(angle, 1600)) as i16,
                };
            }
            assert_eq!(format!("{:?}", site.groups), format!("{expected_groups:?}"));
        }
    }
}

#[test]
fn ghosts_are_saved_without_building_and_empty_queue_is_enforced() {
    let mut site = Guard::default();
    init_npcs(&mut FakeStory::with(&[("URQUAN_MESSED_UP", 1)]), &mut site);
    assert_eq!(site.calls, ["save"]);
    assert!(std::panic::catch_unwind(|| {
        init_npcs(
            &mut FakeStory::default(),
            &mut Guard {
                built: 1,
                ..Guard::default()
            },
        );
    })
    .is_err());
    assert!(std::panic::catch_unwind(|| {
        init_npcs(
            &mut FakeStory::default(),
            &mut Guard {
                groups: groups(15),
                ..Guard::default()
            },
        );
    })
    .is_err());
}

#[test]
fn reinit_preserves_full_groups_and_moves_only_engaged_guard() {
    for messed in [0, 1] {
        for count in [0, 1, 16] {
            for task in 0..=15 {
                let mut site = Guard {
                    groups: groups(count),
                    ..Guard::default()
                };
                for g in &mut site.groups {
                    g.task = task;
                }
                let mut expected = site.groups.clone();
                for g in &mut expected {
                    if messed != 0 {
                        g.task = (task & 8) | 6;
                        g.dest_loc = 0;
                    } else if task & 8 != 0 {
                        g.task = task & !8;
                        g.group_counter = 0;
                    }
                }
                reinit_npcs(&FakeStory::with(&[("URQUAN_MESSED_UP", messed)]), &mut site);
                assert_eq!(format!("{:?}", site.groups), format!("{expected:?}"));
                let engaged = messed == 0 && count != 0 && task & 8 != 0;
                assert_eq!(
                    site.calls,
                    if engaged {
                        vec!["load", "reset", "transform"]
                    } else {
                        vec!["load", "reset"]
                    }
                );
                if let Some(location) = site.moved {
                    let angle = arctan(-1000, 1000);
                    assert_eq!(
                        (location.x, location.y),
                        (
                            (100 + cosine(angle, 3000)) as i16,
                            (-200 + sine(angle, 3000)) as i16
                        )
                    );
                }
            }
        }
    }
}

#[test]
fn final_contact_all_story_survivor_abort_and_rebel_paths() {
    for messed in [0, 1] {
        for frenzy in [0, 1] {
            for survivors in [false, true] {
                for civil in [0, 1] {
                    for tracking in [false, true] {
                        for feasible in [false, true] {
                            for status in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
                                let mut story = FakeStory::with(&[
                                    ("URQUAN_MESSED_UP", messed),
                                    ("KOHR_AH_FRENZY", frenzy),
                                    ("YEHAT_CIVIL_WAR", civil),
                                ]);
                                let mut site = Site {
                                    survivors,
                                    yehat: tracking,
                                    escort: feasible,
                                    after: 4 | 0x100 | START_INTERPLANETARY | status,
                                    ..Site::default()
                                };
                                let (info, seed) = with_orbit(|orbit| {
                                    assert!(generate_orbital(
                                        orbit,
                                        WORLD,
                                        &mut story,
                                        &mut site,
                                        || panic!("final battle is not a landing")
                                    ));
                                });
                                assert_eq!(seed, 104729);
                                assert_eq!(info.PlanetInfo.ScanSeed, [101, 202, 303]);
                                let mut expected: Vec<String> =
                                    ["save", "clear-ip", "count"].map(String::from).to_vec();
                                if messed == 0 {
                                    expected.push(format!("guard:{}", frenzy != 0));
                                } else {
                                    for _ in 0..3 {
                                        expected.extend(
                                            ["remnant:false", "remnant:true"].map(String::from),
                                        );
                                    }
                                }
                                expected.extend(
                                    ["activity:2308", "flags:128", "talk"].map(String::from),
                                );
                                if status == 0 {
                                    expected.extend(
                                        ["survivors", "activity:260", "clear-built", "load-ip"]
                                            .map(String::from),
                                    );
                                    if !survivors {
                                        expected.extend(
                                            ["reset-encounter", "activity:1025"].map(String::from),
                                        );
                                        if civil != 0 {
                                            expected.push("track-yehat".into());
                                            if tracking {
                                                expected.push("escort".into());
                                                if feasible {
                                                    expected.push("rebels".into());
                                                }
                                            }
                                        }
                                    }
                                }
                                assert_eq!(*site.calls.borrow(), expected);
                                assert_eq!(
                                    story.flag("URQUAN_PROTECTING_SAMATRA"),
                                    u32::from(status != 0 || !survivors)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn nonfinal_world_falls_through_and_final_queue_must_be_empty() {
    for body in [
        Body::Planet(4),
        Body::Planet(0),
        Body::Moon { planet: 4, moon: 1 },
        Body::Moon { planet: 0, moon: 0 },
    ] {
        let mut site = Site::default();
        let calls = Rc::clone(&site.calls);
        with_orbit(|orbit| {
            generate_orbital(orbit, body, &mut FakeStory::default(), &mut site, || {
                record(&calls, "planet")
            });
        });
        assert_eq!(*calls.borrow(), ["planet"]);
    }
    assert!(std::panic::catch_unwind(|| {
        with_orbit(|orbit| {
            generate_orbital(
                orbit,
                WORLD,
                &mut FakeStory::default(),
                &mut Site {
                    built: 1,
                    ..Site::default()
                },
                || {},
            );
        });
    })
    .is_err());
}

#[test]
fn protecting_flag_precedes_talk_and_survivors_clear_it_after_restoration() {
    use crate::planets::generate::vux::tests::OrderedStory;
    let mut site = Site {
        survivors: true,
        ..Site::default()
    };
    let mut story = OrderedStory {
        inner: FakeStory::default(),
        calls: Rc::clone(&site.calls),
    };
    with_orbit(|orbit| {
        generate_orbital(orbit, WORLD, &mut story, &mut site, || {});
    });
    assert_eq!(
        *site.calls.borrow(),
        [
            "save",
            "clear-ip",
            "count",
            "guard:false",
            "activity:2308",
            "flags:128",
            "set:URQUAN_PROTECTING_SAMATRA:1",
            "talk",
            "survivors",
            "activity:260",
            "clear-built",
            "load-ip",
            "set:URQUAN_PROTECTING_SAMATRA:0"
        ]
    );
}
