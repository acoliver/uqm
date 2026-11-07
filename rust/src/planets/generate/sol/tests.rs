use super::*;
use crate::engine::world::planet::NodeInfo;
use crate::engine::StoryKey;
use std::collections::BTreeMap;

#[test]
fn special_pickups_reject_retrieved_nodes_and_nonzero_indices_before_effects() {
    for (body, flag) in [
        (PLUTO, StoryKey::FOUND_PLUTO_SPATHI),
        (LUNA, StoryKey::MOONBASE_DESTROYED),
    ] {
        for (taken, which) in [(1, 0), (0, 1), (1, 1)] {
            let mut host = Host {
                survive: true,
                ..Host::default()
            };
            host.flags.insert(flag, taken);
            let before = host.flags.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::planets::generate::testing::with_empty(|system| {
                    pickup_energy(system, body, which, &mut host)
                })
            }));
            assert!(result.is_err());
            assert_eq!(host.flags, before);
            assert!(host.calls.is_empty());
            assert!(host.words.is_empty());
        }
    }
}

#[derive(Default)]
struct Host {
    flags: BTreeMap<StoryKey, u32>,
    words: BTreeMap<StoryKey, u32>,
    survive: bool,
    calls: Vec<String>,
}

impl Story for Host {
    fn flag(&self, key: StoryKey) -> u32 {
        self.flags.get(&key).copied().unwrap_or(0)
    }
    fn set_flag(&mut self, key: StoryKey, value: u32) {
        self.calls.push(format!(
            "flag:{}:{value}",
            crate::planets::generate::testing::story_label(key)
        ));
        self.flags.insert(key, value);
    }
    fn word(&self, key: StoryKey) -> u32 {
        self.words.get(&key).copied().unwrap_or(0)
    }
    fn set_word(&mut self, key: StoryKey, value: u32) {
        self.calls.push(format!(
            "word:{}:{value}",
            crate::planets::generate::testing::story_label(key)
        ));
        self.words.insert(key, value);
    }
}

impl Pickup for Host {
    fn kill_lander_crew(&mut self, count: u16, interval_ticks: u32) -> bool {
        self.calls.push(format!("crew:{count}:{interval_ticks}"));
        self.survive
    }
    fn lander_report(&mut self, _: &mut SolarSys<'_>) {
        self.calls.push("report".into());
    }
    fn lander_takeoff(&mut self) {
        self.calls.push("takeoff".into());
    }
}

impl PlanetNames for Host {
    fn set_planet_name(&mut self, string_index: u16) {
        self.calls.push(format!("name:{string_index}"));
    }
    fn set_battle_planet(&mut self, frame: u8) {
        self.calls.push(format!("battle:{frame}"));
    }
}

#[test]
fn sol_names_use_parent_planet_strings_and_frames_in_order() {
    for (planet, &(frame, _, _)) in PLANETS.iter().enumerate() {
        let mut host = Host::default();
        assert!(generate_name(&mut host, planet as u16, frame));
        assert_eq!(
            host.calls,
            [
                format!("name:{}", PLANET_NUMBER_BASE as usize + planet),
                format!("battle:{frame}")
            ]
        );
        assert!(host.flags.is_empty());
        assert!(host.words.is_empty());
    }
}

#[test]
fn pluto_pickup_stops_on_crew_failure_and_retains_the_node_on_success() {
    for survive in [false, true] {
        let mut host = Host {
            survive,
            ..Host::default()
        };
        host.flags.insert(StoryKey::SHOFIXTI_VISITS, 93);
        assert!(!crate::planets::generate::testing::with_empty(|system| {
            pickup_energy(system, PLUTO, 0, &mut host)
        }));
        let expected = if survive {
            vec![
                "crew:8:42",
                "flag:FOUND_PLUTO_SPATHI:1",
                "report",
                "takeoff",
            ]
        } else {
            vec!["crew:8:42"]
        };
        assert_eq!(host.calls, expected);
        assert_eq!(host.flag(StoryKey::FOUND_PLUTO_SPATHI), u32::from(survive));
        assert_eq!(host.flag(StoryKey::MOONBASE_DESTROYED), 0);
        assert_eq!(host.flag(StoryKey::MOONBASE_ON_SHIP), 0);
        assert_eq!(host.flag(StoryKey::SHOFIXTI_VISITS), 93);
        assert!(host.words.is_empty());
        let mut node: NodeInfo = unsafe { std::mem::zeroed() };
        assert_eq!(
            generate_energy(PLUTO, Some(&mut node), &host),
            u16::from(!survive)
        );
    }
}

#[test]
fn luna_pickup_reports_then_takes_off_before_both_story_writes() {
    let mut host = Host::default();
    host.flags.insert(StoryKey::SHOFIXTI_VISITS, 93);
    assert!(crate::planets::generate::testing::with_empty(|system| {
        pickup_energy(system, LUNA, 0, &mut host)
    }));
    assert_eq!(
        host.calls,
        [
            "report",
            "takeoff",
            "flag:MOONBASE_DESTROYED:1",
            "flag:MOONBASE_ON_SHIP:1"
        ]
    );
    assert_eq!(host.flag(StoryKey::MOONBASE_DESTROYED), 1);
    assert_eq!(host.flag(StoryKey::MOONBASE_ON_SHIP), 1);
    assert_eq!(host.flag(StoryKey::FOUND_PLUTO_SPATHI), 0);
    assert_eq!(host.flag(StoryKey::SHOFIXTI_VISITS), 93);
    assert!(host.words.is_empty());
    assert_eq!(generate_energy(LUNA, None, &host), 0);
}

#[test]
fn other_energy_pickups_have_no_effect_even_for_nonzero_node_indices() {
    for planet in 0..9 {
        for body in [
            Body::Planet(planet),
            Body::Moon { planet, moon: 0 },
            Body::Moon { planet, moon: 1 },
        ] {
            if body == PLUTO || body == LUNA {
                continue;
            }
            for which in [0, 1, u16::MAX] {
                let mut host = Host::default();
                assert!(!crate::planets::generate::testing::with_empty(|system| {
                    pickup_energy(system, body, which, &mut host)
                }));
                assert!(host.calls.is_empty());
                assert!(host.flags.is_empty());
                assert!(host.words.is_empty());
            }
        }
    }
}
