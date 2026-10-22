use super::*;
use crate::planets::generate::home_contact::tests::FakeStory;
use crate::planets::generate::vux::tests::{record, with_orbit, Calls, Pickup, Site};
use std::rc::Rc;

#[test]
fn moon_contact_shield_broadcaster_and_abort_matrix() {
    for body in [
        WORLD,
        MOON,
        Body::Planet(1),
        Body::Moon { planet: 1, moon: 0 },
    ] {
        for shielded in [0, 1] {
            for taken in [0, 1] {
                for tracking in [false, true] {
                    for status in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
                        let story = FakeStory::with(&[
                            ("SPATHI_SHIELDED_SELVES", shielded),
                            ("UMGAH_BROADCASTERS", taken),
                        ]);
                        let mut site = Site {
                            tracking,
                            after: 4 | 0x100 | START_INTERPLANETARY | status,
                            ..Site::default()
                        };
                        let calls = Rc::clone(&site.calls);
                        let (info, rng) = with_orbit(|orbit| {
                            assert!(generate_orbital(orbit, body, &story, &mut site, || record(
                                &calls, "planet"
                            )));
                        });
                        let mut expected: Vec<String> = Vec::new();
                        let talk = body == MOON && shielded == 0 && tracking;
                        if body == MOON && shielded == 0 {
                            expected.push("track".into());
                        }
                        if talk {
                            expected.extend(
                                [
                                    "notify",
                                    "save",
                                    "clear-ip",
                                    "count",
                                    "fleet",
                                    "flags:128",
                                    "activity:2308",
                                    "talk",
                                ]
                                .map(String::from),
                            );
                            if status == 0 {
                                expected.extend(
                                    ["activity:260", "clear-built", "load-ip"].map(String::from),
                                );
                            }
                            assert_eq!(rng, 104729);
                            assert_eq!(info.PlanetInfo.ScanSeed, [101, 202, 303]);
                        } else {
                            if body == MOON && taken == 0 {
                                expected.push(format!("broadcasters:{}", shielded == 0));
                            }
                            expected.push("planet".into());
                            if body == WORLD || body == MOON {
                                assert_eq!(
                                    (info.PlanetInfo.Weather, info.PlanetInfo.Tectonics),
                                    (0, 0)
                                );
                                assert_eq!(
                                    info.PlanetInfo.SurfaceTemperature,
                                    if body == WORLD { 31 } else { 28 }
                                );
                            }
                            if body == WORLD {
                                assert_eq!(info.PlanetInfo.PlanetRadius, 120);
                                assert_eq!(
                                    info.PlanetInfo.SurfaceGravity,
                                    calc::calc_gravity(&info.PlanetInfo)
                                );
                                assert_eq!(info.PlanetInfo.ScanSeed[1], 202);
                            }
                        }
                        assert_eq!(*calls.borrow(), expected);
                    }
                }
            }
        }
    }
}

#[test]
fn world_scan_order_preserves_energy_and_matches_direct_analysis() {
    let (expected, seed) = with_orbit(|orbit| {
        calc::do_planetary_analysis(
            orbit.rng,
            orbit.plan_data,
            orbit.star,
            orbit.site,
            orbit.sys_info,
            orbit.world,
        );
        orbit.sys_info.PlanetInfo.ScanSeed[MINERAL_SCAN] = orbit.rng.seed();
        let mut node = unsafe { std::mem::zeroed() };
        surface::generate_mineral_deposits(orbit.rng, orbit.sys_info, !0, &mut node);
        orbit.sys_info.PlanetInfo.ScanSeed[BIOLOGICAL_SCAN] = orbit.rng.seed();
        orbit.sys_info.PlanetInfo.PlanetRadius = 120;
        orbit.sys_info.PlanetInfo.SurfaceGravity = calc::calc_gravity(&orbit.sys_info.PlanetInfo);
        orbit.sys_info.PlanetInfo.SurfaceTemperature = 31;
        orbit.sys_info.PlanetInfo.Weather = 0;
        orbit.sys_info.PlanetInfo.Tectonics = 0;
    });
    let (actual, actual_seed) = with_orbit(|orbit| {
        generate_orbital(
            orbit,
            WORLD,
            &FakeStory::default(),
            &mut Site::default(),
            || {},
        );
    });
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    assert_eq!(actual_seed, seed);
}

#[test]
fn last_creature_uses_retrieved_population_not_node_index() {
    for count in 0..=32 {
        let mask = if count == 32 {
            u32::MAX
        } else {
            (1u32 << count) - 1
        };
        for body in [WORLD, MOON, Body::Planet(1)] {
            let mut story = FakeStory::default();
            assert!(pickup_life(body, mask, &mut story));
            assert_eq!(
                story.flag("SPATHI_CREATURES_EXAMINED"),
                u32::from(body == WORLD)
            );
            assert_eq!(
                story.flag("SPATHI_CREATURES_ELIMINATED"),
                u32::from(body == WORLD && count == 31)
            );
        }
    }
    for flag in ["SPATHI_CREATURES_ELIMINATED", "SPATHI_SHIELDED_SELVES"] {
        assert!(std::panic::catch_unwind(|| {
            pickup_life(WORLD, 0, &mut FakeStory::with(&[(flag, 1)]));
        })
        .is_err());
    }
}

#[test]
fn broadcaster_pickup_and_invariants() {
    for body in [WORLD, MOON, Body::Planet(1)] {
        let calls = Calls::default();
        let mut story = FakeStory::default();
        let result = pickup_energy(
            body,
            0,
            &mut story,
            &mut Pickup(Rc::clone(&calls)),
            &mut Pickup(Rc::clone(&calls)),
        );
        assert_eq!(result, body == MOON);
        assert_eq!(
            *calls.borrow(),
            if result {
                vec!["report:Once", "takeoff"]
            } else {
                vec![]
            }
        );
        assert_eq!(story.flag("UMGAH_BROADCASTERS"), u32::from(result));
        assert_eq!(story.flag("UMGAH_BROADCASTERS_ON_SHIP"), u32::from(result));
    }
    for (taken, which) in [(1, 0), (0, 1), (0, u16::MAX)] {
        let calls = Calls::default();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pickup_energy(
                MOON,
                which,
                &mut FakeStory::with(&[("UMGAH_BROADCASTERS", taken)]),
                &mut Pickup(Rc::clone(&calls)),
                &mut Pickup(Rc::clone(&calls)),
            )
        }))
        .is_err());
        assert!(calls.borrow().is_empty());
    }
}

#[test]
fn moon_queue_must_be_empty() {
    assert!(std::panic::catch_unwind(|| {
        with_orbit(|orbit| {
            generate_orbital(
                orbit,
                MOON,
                &FakeStory::default(),
                &mut Site {
                    tracking: true,
                    built: 1,
                    ..Site::default()
                },
                || panic!("must not load"),
            );
        });
    })
    .is_err());
}

#[test]
fn broadcaster_and_last_creature_flag_write_order() {
    use crate::planets::generate::vux::tests::OrderedStory;
    let calls = Calls::default();
    let mut story = OrderedStory {
        inner: FakeStory::default(),
        calls: Rc::clone(&calls),
    };
    pickup_energy(
        MOON,
        0,
        &mut story,
        &mut Pickup(Rc::clone(&calls)),
        &mut Pickup(Rc::clone(&calls)),
    );
    assert_eq!(
        *calls.borrow(),
        [
            "report:Once",
            "takeoff",
            "set:UMGAH_BROADCASTERS:1",
            "set:UMGAH_BROADCASTERS_ON_SHIP:1"
        ]
    );
    calls.borrow_mut().clear();
    pickup_life(WORLD, 0x7fffffff, &mut story);
    assert_eq!(
        *calls.borrow(),
        [
            "set:SPATHI_CREATURES_EXAMINED:1",
            "set:SPATHI_CREATURES_ELIMINATED:1"
        ]
    );
}
