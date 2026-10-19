use super::*;
use crate::planet_side::report_adapter::ReportDisposition;
use crate::planet_side::runtime::AdapterError;
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::generate::home_contact::tests::FakeStory;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) type Calls = Rc<RefCell<Vec<String>>>;
pub(crate) fn record(calls: &Calls, value: impl Into<String>) {
    calls.borrow_mut().push(value.into());
}

pub(crate) struct Site {
    pub calls: Calls,
    pub tracking: bool,
    pub built: u16,
    pub activity: u16,
    pub after: u16,
    pub dead: bool,
    pub kill: bool,
    pub survivors: bool,
    pub yehat: bool,
    pub escort: bool,
}
impl Default for Site {
    fn default() -> Self {
        Self {
            calls: Rc::default(),
            tracking: false,
            built: 0,
            activity: 4 | 0x100,
            after: 4 | 0x100 | START_INTERPLANETARY,
            dead: false,
            kill: false,
            survivors: false,
            yehat: false,
            escort: false,
        }
    }
}
impl HomeContactSite for Site {
    fn start_sphere_tracking(&mut self) -> bool {
        record(&self.calls, "track");
        self.tracking
    }
    fn notify_others_all_clear(&mut self) {
        record(&self.calls, "notify");
    }
    fn save_ip_groups(&mut self) {
        record(&self.calls, "save");
    }
    fn reinit_ip_group_q(&mut self) {
        record(&self.calls, "clear-ip");
    }
    fn built_ship_count(&self) -> u16 {
        record(&self.calls, "count");
        self.built
    }
    fn clone_fleet_into_built_queue(&mut self) {
        record(&self.calls, "fleet");
    }
    fn current_activity(&self) -> u16 {
        self.activity
    }
    fn set_current_activity(&mut self, value: u16) {
        record(&self.calls, format!("activity:{value}"));
        self.activity = value;
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        record(&self.calls, format!("flags:{value}"));
    }
    fn talk(&mut self) {
        record(&self.calls, "talk");
        self.activity = self.after;
        self.dead |= self.kill;
    }
    fn reinit_built_ship_q(&mut self) {
        record(&self.calls, "clear-built");
    }
    fn load_ip_groups(&mut self) {
        record(&self.calls, "load-ip");
    }
    fn load_ruins_scene(&mut self, _: bool) {
        panic!("special scene required");
    }
}
impl VuxSite for Site {
    fn zex_is_dead(&self) -> bool {
        self.dead
    }
    fn repair_sis_border(&mut self) {
        record(&self.calls, "repair");
    }
    fn load_scene(&mut self, scene: Scene) {
        record(&self.calls, format!("scene:{scene:?}"));
    }
}
impl crate::planets::generate::spa::SpathiSite for Site {
    fn load_broadcasters_scene(&mut self, unshielded: bool) {
        record(&self.calls, format!("broadcasters:{unshielded}"));
    }
}
impl crate::planets::generate::sam::SamatraSite for Site {
    fn clone_guard_fleet(&mut self, black: bool) {
        record(&self.calls, format!("guard:{black}"));
    }
    fn clone_remnant(&mut self, black: bool) {
        record(&self.calls, format!("remnant:{black}"));
    }
    fn built_queue_has_ships(&self) -> bool {
        record(&self.calls, "survivors");
        self.survivors
    }
    fn reset_encounter(&mut self) {
        record(&self.calls, "reset-encounter");
    }
    fn track_yehat(&mut self) -> bool {
        record(&self.calls, "track-yehat");
        self.yehat
    }
    fn rebel_escort_feasible(&mut self) -> bool {
        record(&self.calls, "escort");
        self.escort
    }
    fn talk_rebels(&mut self) {
        record(&self.calls, "rebels");
    }
}

pub(crate) struct Pickup(pub Calls);
impl DiscoveryReportPort for Pickup {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        record(&self.0, format!("report:{disposition:?}"));
        Ok(true)
    }
}
impl LanderTakeoff for Pickup {
    fn lander_takeoff(&mut self) {
        record(&self.0, "takeoff");
    }
}

pub(crate) fn with_orbit(run: impl FnOnce(&mut Orbit<'_>)) -> (SYSTEM_INFO, u32) {
    let mut rng = RandomContext::with_seed(104729);
    // The returned SYSTEM_INFO retains PlanDataPtr after this helper returns.
    static TABLE: crate::planets::tables::SharedTable<[PlanetFrame; 59]> =
        crate::planets::tables::SharedTable([unsafe { std::mem::zeroed() }; 59]);
    let mut world: PLANET_DESC = unsafe { std::mem::zeroed() };
    world.rand_seed = 7919;
    world.radius = 512;
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = [101, 202, 303];
    run(&mut Orbit {
        rng: &mut rng,
        plan_data: &TABLE.0,
        star: StarDescription {
            size: 0,
            star_type: 0,
        },
        site: AnalysisSite {
            parent_radius: None,
            is_orbit_entry: true,
        },
        sys_info: &mut info,
        world: &world,
    });
    (info, rng.seed())
}

#[test]
fn contact_scene_and_abort_matrix() {
    for star in [VUX_DEFINED, MAIDENS_DEFINED, VUX_BEAST_DEFINED, 0] {
        for body in [WORLD, Body::Planet(1), Body::Moon { planet: 0, moon: 0 }] {
            for dead in [false, true] {
                for kill in [false, true] {
                    for tracking in [false, true] {
                        for taken in [0, 1] {
                            for status in [0, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
                                let mut site = Site {
                                    dead,
                                    kill,
                                    tracking,
                                    after: 4 | 0x100 | START_INTERPLANETARY | status,
                                    ..Site::default()
                                };
                                let story = FakeStory::with(&[
                                    ("SHOFIXTI_MAIDENS", taken),
                                    ("VUX_BEAST", taken),
                                ]);
                                let calls = Rc::clone(&site.calls);
                                let (info, _) = with_orbit(|orbit| {
                                    assert!(generate_orbital(
                                        orbit,
                                        body,
                                        star,
                                        &story,
                                        &mut site,
                                        || record(&calls, "planet")
                                    ));
                                });
                                let mut expected: Vec<String> = Vec::new();
                                let eligible = body == WORLD
                                    && (star == VUX_DEFINED || (star == MAIDENS_DEFINED && !dead));
                                if eligible {
                                    expected.push("track".into());
                                }
                                let talk = eligible && tracking;
                                let mut lands = true;
                                if talk {
                                    expected.extend(
                                        ["notify", "save", "clear-ip", "count", "fleet"]
                                            .map(String::from),
                                    );
                                    expected.push(format!(
                                        "flags:{}",
                                        if star == VUX_DEFINED { 128 } else { 64 }
                                    ));
                                    expected.extend(["activity:2308", "talk"].map(String::from));
                                    if status != 0 {
                                        lands = false;
                                    } else {
                                        expected.extend(
                                            ["activity:260", "clear-built", "load-ip"]
                                                .map(String::from),
                                        );
                                        if star == VUX_DEFINED || !(dead || kill) {
                                            lands = false;
                                        } else {
                                            expected.push("repair".into());
                                        }
                                    }
                                }
                                if lands {
                                    if body == WORLD {
                                        if star == MAIDENS_DEFINED && taken == 0 {
                                            expected.push("scene:Maidens".into());
                                        } else if star == VUX_BEAST_DEFINED && taken == 0 {
                                            expected.push("scene:Beast".into());
                                        } else if star != MAIDENS_DEFINED
                                            && star != VUX_BEAST_DEFINED
                                        {
                                            expected.push("scene:Ruins".into());
                                        }
                                    }
                                    expected.push("planet".into());
                                }
                                assert_eq!(
                                    *calls.borrow(),
                                    expected,
                                    "{star} {body:?} {dead} {kill} {tracking} {taken} {status}"
                                );
                                if lands && body == WORLD {
                                    assert_eq!(
                                        (info.PlanetInfo.Weather, info.PlanetInfo.Tectonics),
                                        (2, 0)
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn pickup_effects_and_nonartifact_nodes() {
    for star in [VUX_DEFINED, MAIDENS_DEFINED, VUX_BEAST_DEFINED, 0] {
        for body in [WORLD, Body::Planet(1), Body::Moon { planet: 0, moon: 0 }] {
            for which in [0, 1, 31, 32, u16::MAX] {
                for life in [false, true] {
                    let calls = Calls::default();
                    let mut story = FakeStory::default();
                    let result = if life {
                        pickup_life(
                            star,
                            body,
                            which,
                            &mut story,
                            &mut Pickup(Rc::clone(&calls)),
                            &mut Pickup(Rc::clone(&calls)),
                        )
                    } else if star == MAIDENS_DEFINED && body == WORLD && which != 0 {
                        continue;
                    } else {
                        pickup_energy(
                            star,
                            body,
                            which,
                            &mut story,
                            &mut Pickup(Rc::clone(&calls)),
                            &mut Pickup(Rc::clone(&calls)),
                        )
                    };
                    let artifact = body == WORLD
                        && which == 0
                        && ((life && star == VUX_BEAST_DEFINED)
                            || (!life && star == MAIDENS_DEFINED));
                    let ruins = !life && star == VUX_DEFINED && body == WORLD;
                    assert_eq!(result, life || artifact);
                    assert_eq!(
                        *calls.borrow(),
                        if artifact {
                            vec!["report:Once", "takeoff"]
                        } else if ruins {
                            vec!["report:Cycle"]
                        } else {
                            vec![]
                        }
                    );
                    assert_eq!(story.flag("SHOFIXTI_MAIDENS"), u32::from(artifact && !life));
                    assert_eq!(story.flag("MAIDENS_ON_SHIP"), u32::from(artifact && !life));
                    assert_eq!(story.flag("VUX_BEAST"), u32::from(artifact && life));
                    assert_eq!(story.flag("VUX_BEAST_ON_SHIP"), u32::from(artifact && life));
                }
            }
        }
    }
}

#[test]
fn pickup_and_queue_invariants_fail_before_side_effects() {
    for life in [false, true] {
        for (taken, which) in [(1, 0), (0, 1)] {
            if life && which != 0 {
                continue;
            }
            let calls = Calls::default();
            let mut story = FakeStory::with(&[(
                if life {
                    "VUX_BEAST"
                } else {
                    "SHOFIXTI_MAIDENS"
                },
                taken,
            )]);
            let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if life {
                    pickup_life(
                        VUX_BEAST_DEFINED,
                        WORLD,
                        which,
                        &mut story,
                        &mut Pickup(Rc::clone(&calls)),
                        &mut Pickup(Rc::clone(&calls)),
                    )
                } else {
                    pickup_energy(
                        MAIDENS_DEFINED,
                        WORLD,
                        which,
                        &mut story,
                        &mut Pickup(Rc::clone(&calls)),
                        &mut Pickup(Rc::clone(&calls)),
                    )
                }
            }));
            assert!(failed.is_err());
            assert!(calls.borrow().is_empty());
        }
    }
    let failed = std::panic::catch_unwind(|| {
        let mut site = Site {
            built: 1,
            tracking: true,
            ..Site::default()
        };
        with_orbit(|orbit| {
            generate_orbital(
                orbit,
                WORLD,
                VUX_DEFINED,
                &FakeStory::default(),
                &mut site,
                || panic!("must not load"),
            );
        });
    });
    assert!(failed.is_err());
}

#[test]
fn beast_insertion_and_maidens_second_layout_preserve_whole_descriptors() {
    let table = vec![unsafe { std::mem::zeroed() }; 59];
    for seed in [1, 7919, 190056, u32::MAX] {
        let mut base: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
        for (i, desc) in base.iter_mut().enumerate() {
            desc.NextIndex = i as u16 + 37;
            desc.image.origin.x = -53;
        }
        let initial = base.clone();
        let mut rng = RandomContext::with_seed(seed);
        let count = default::generate_planets(&mut rng, &table, 0, 0, &mut base);
        let mut actual = initial.clone();
        let mut actual_rng = RandomContext::with_seed(seed);
        let drawn = generate_planets(
            &mut actual_rng,
            &table,
            0,
            0,
            VUX_BEAST_DEFINED,
            &mut actual,
        );
        assert_eq!(drawn, count + 1);
        assert_eq!(
            format!("{:?}", &actual[1..=usize::from(count)]),
            format!("{:?}", &base[..usize::from(count)])
        );
        assert_eq!(rng.seed(), actual_rng.seed());
        let mut maidens = initial.clone();
        let mut expected = initial;
        let mut expected_rng = RandomContext::with_seed(seed);
        default::generate_planets(&mut expected_rng, &table, 0, 0, &mut expected);
        let count = default::generate_planets(&mut expected_rng, &table, 0, 0, &mut expected);
        expected[0].data_index = REDUX_WORLD;
        special::reposition(&mut expected[0], 212);
        actual_rng.reseed(seed);
        assert_eq!(
            generate_planets(&mut actual_rng, &table, 0, 0, MAIDENS_DEFINED, &mut maidens),
            count
        );
        assert_eq!(actual_rng.seed(), expected_rng.seed());
        assert_eq!(format!("{maidens:?}"), format!("{expected:?}"));
    }
}

pub(crate) struct OrderedStory {
    pub inner: FakeStory,
    pub calls: Calls,
}
impl Story for OrderedStory {
    fn flag(&self, name: &'static str) -> u32 {
        self.inner.flag(name)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        record(&self.calls, format!("set:{name}:{value}"));
        self.inner.set_flag(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        self.inner.word(name)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        record(&self.calls, format!("word:{name}:{value}"));
        self.inner.set_word(name, value);
    }
}

#[test]
fn pickup_reports_takeoff_and_flag_writes_have_c_order() {
    for life in [false, true] {
        let calls = Calls::default();
        let mut story = OrderedStory {
            inner: FakeStory::default(),
            calls: Rc::clone(&calls),
        };
        if life {
            pickup_life(
                VUX_BEAST_DEFINED,
                WORLD,
                0,
                &mut story,
                &mut Pickup(Rc::clone(&calls)),
                &mut Pickup(Rc::clone(&calls)),
            );
        } else {
            pickup_energy(
                MAIDENS_DEFINED,
                WORLD,
                0,
                &mut story,
                &mut Pickup(Rc::clone(&calls)),
                &mut Pickup(Rc::clone(&calls)),
            );
        }
        let flags = if life {
            ["VUX_BEAST", "VUX_BEAST_ON_SHIP"]
        } else {
            ["SHOFIXTI_MAIDENS", "MAIDENS_ON_SHIP"]
        };
        assert_eq!(
            *calls.borrow(),
            vec![
                "report:Once".into(),
                "takeoff".into(),
                format!("set:{}:1", flags[0]),
                format!("set:{}:1", flags[1])
            ]
        );
    }
}
