use super::lifecycle_tests::{group, pair, QueueHost, Trace};
use super::orbital::{Orbital, Queues, Scene};
use super::*;
use crate::c_bindings::global_state::{IP_GROUP, PLANET_INFO};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::generate::default::Npcs;
use crate::planets::generate::special::Orbit;
use crate::planets::tables::PLANET_ARRAY;

struct Surface(Trace);

impl Drop for Surface {
    fn drop(&mut self) {
        self.0.borrow_mut().push("free-earth".into());
    }
}

struct Site {
    queues: QueueHost,
    info: SYSTEM_INFO,
    analyzed: Option<SYSTEM_INFO>,
    world: PLANET_DESC,
    rng: RandomContext,
    body: Body,
    activity: u16,
    encounter: u8,
    flags: u8,
    scene: Option<Scene>,
    surface: Option<Surface>,
}

impl Site {
    fn new(queues: QueueHost, body: Body) -> Self {
        let mut planets = unsafe { std::mem::zeroed() };
        generate_planets(&mut RandomContext::with_seed(1), &mut planets);
        let world = match body {
            Body::Planet(p) => planets[usize::from(p)],
            Body::Moon { planet, moon } => {
                let mut moons = unsafe { std::mem::zeroed() };
                generate_moons(
                    &mut RandomContext::with_seed(99),
                    0,
                    5,
                    planet,
                    4,
                    &mut moons,
                );
                moons[usize::from(moon)]
            }
        };
        let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        info.PlanetInfo.ScanSeed = [11, 13, 17];
        info.PlanetInfo.ScanRetrieveMask = [19, 23, 29];
        Self {
            queues,
            info,
            analyzed: None,
            world,
            rng: RandomContext::with_seed(101),
            body,
            activity: 77,
            encounter: 31,
            flags: 29,
            scene: None,
            surface: None,
        }
    }

    fn leave(&mut self) {
        self.surface.take();
        if self.scene.take().is_some() {
            self.queues.call("free-scene");
        }
    }
}

impl Npcs for Site {
    fn init_ip_groups(&mut self) -> bool {
        self.queues.init_ip_groups()
    }
    fn clear_battle_group_ref(&mut self) {
        self.queues.clear_battle_group_ref();
    }
    fn build_groups(&mut self) {
        self.queues.build_groups();
    }
    fn load_ip_groups(&mut self) {
        self.queues.load_ip_groups();
    }
    fn save_ip_groups(&mut self) {
        self.queues.save_ip_groups();
    }
    fn reinit_npc_built_ship_q(&mut self) {
        self.queues.reinit_npc_built_ship_q();
    }
    fn reinit_ip_group_q(&mut self) {
        self.queues.reinit_ip_group_q();
    }
    fn yehat_civil_war(&self) -> bool {
        self.queues.yehat_civil_war()
    }
    fn encounter_race(&self) -> i16 {
        self.queues.encounter_race()
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        self.queues.for_each_ip_group(visit);
    }
}

impl Queues for Site {
    fn built_ship_count(&self) -> u16 {
        self.queues.built_ship_count()
    }
}

impl Orbital for Site {
    type EarthSurface = Surface;
    fn set_encounter_group(&mut self, value: u8) {
        self.queues.call(format!("encounter:{value}"));
        self.encounter = value;
    }
    fn current_activity(&self) -> u16 {
        self.queues.call("activity");
        self.activity
    }
    fn set_current_activity(&mut self, value: u16) {
        self.queues.call(format!("activity:{value}"));
        self.activity = value;
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.queues.call(format!("flags:{value}"));
        self.flags = value;
    }
    fn with_orbit(&mut self, run: impl FnOnce(&mut Orbit<'_>)) {
        self.queues.call("analyse");
        let parent_radius = match self.body {
            Body::Planet(_) => None,
            Body::Moon { planet, .. } => {
                Some((i32::from(EARTH_RADIUS) * PLANETS[usize::from(planet)].1 / 100) as i16)
            }
        };
        run(&mut Orbit {
            rng: &mut self.rng,
            plan_data: &PLANET_ARRAY.0,
            star: StarDescription {
                size: 0,
                star_type: (5 << 5) | 5,
            },
            site: AnalysisSite {
                parent_radius,
                is_orbit_entry: true,
            },
            sys_info: &mut self.info,
            world: &self.world,
        });
        self.analyzed = Some(self.info);
    }
    fn planet_info(&mut self) -> &mut PLANET_INFO {
        self.queues.call("overrides");
        &mut self.info.PlanetInfo
    }
    fn load_scene(&mut self, scene: Scene) {
        assert_eq!(
            format!("{:?}", self.info),
            format!("{:?}", self.analyzed.unwrap())
        );
        assert!(self.scene.replace(scene).is_none());
        self.queues.call("font");
        self.queues.call(scene.graphics().to_str().unwrap());
        self.queues.call(scene.strings().to_str().unwrap());
    }
    fn load_earth_surface(&mut self) -> Surface {
        self.queues.call("graphics.earthmask");
        Surface(self.queues.trace.clone())
    }
    fn load_planet(&mut self, surface: Option<Surface>) {
        assert_eq!(surface.is_some(), self.body == Body::Planet(2));
        assert!(self.surface.is_none());
        self.surface = surface;
        self.queues.call("load-planet");
    }
}

#[test]
fn starbase_saves_then_clears_ip_and_enters_encounter_without_analysis_or_resources() {
    for activity in [0, 77, CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
        let (story, mut queue) = pair();
        queue.reference = 33;
        queue.groups.push(group(23, 8));
        let mut host = Site::new(queue, STARBASE);
        host.activity = activity;
        let before = format!("{:?}", host.info);
        assert!(generate_orbital(STARBASE, &story, &mut host));
        assert_eq!(format!("{:?}", host.info), before);
        assert_eq!(host.rng.seed(), 101);
        assert!(host.analyzed.is_none());
        assert!(host.queues.groups.is_empty());
        assert_eq!(host.queues.reference, 33);
        assert_eq!(host.encounter, 0);
        assert_eq!(host.activity, activity | START_ENCOUNTER);
        assert_eq!(host.flags, 255);
        assert_eq!(
            *story.trace.borrow(),
            [
                "save".into(),
                "reset-ip".into(),
                "count-built".into(),
                "encounter:0".into(),
                "activity".into(),
                format!("activity:{}", activity | START_ENCOUNTER),
                "flags:255".into()
            ]
        );
    }
}

#[test]
fn starbase_built_queue_assertion_precedes_encounter_writes() {
    let (story, mut queue) = pair();
    queue.built = 1;
    let mut host = Site::new(queue, STARBASE);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        generate_orbital(STARBASE, &story, &mut host)
    }));
    assert!(result.is_err());
    assert_eq!(*story.trace.borrow(), ["save", "reset-ip", "count-built"]);
    assert_eq!((host.activity, host.encounter, host.flags), (77, 31, 29));
    assert_eq!(host.queues.built, 1);
}

#[test]
fn orbital_effects_preserve_c_order_and_surface_ownership_on_every_body() {
    for planet in 0..9 {
        for body in std::iter::once(Body::Planet(planet))
            .chain((0..4).map(|moon| Body::Moon { planet, moon }))
        {
            if body == STARBASE {
                continue;
            }
            for found in [0, 1] {
                for destroyed in [0, 1] {
                    let (mut story, queue) = pair();
                    story.flags.insert("FOUND_PLUTO_SPATHI", found);
                    story.flags.insert("MOONBASE_DESTROYED", destroyed);
                    let before_flags = story.flags.clone();
                    let mut host = Site::new(queue, body);
                    assert!(generate_orbital(body, &story, &mut host));
                    let mut expected = vec!["analyse"];
                    let scene = if body == PLUTO {
                        expected.push("read:FOUND_PLUTO_SPATHI");
                        (found == 0).then_some(Scene::Pluto)
                    } else if matches!(body, Body::Moon { planet: 2, .. }) {
                        expected.push("read:MOONBASE_DESTROYED");
                        (destroyed == 0).then_some(Scene::Moonbase)
                    } else {
                        None
                    };
                    if let Some(scene) = scene {
                        expected.extend([
                            "font",
                            scene.graphics().to_str().unwrap(),
                            scene.strings().to_str().unwrap(),
                        ]);
                    }
                    expected.push("overrides");
                    if body == Body::Planet(2) {
                        expected.push("graphics.earthmask");
                    }
                    expected.push("load-planet");
                    assert_eq!(
                        *story.trace.borrow(),
                        expected,
                        "{body:?} {found} {destroyed}"
                    );
                    assert_eq!(story.flags, before_flags);
                    assert!(story.words.is_empty());
                    assert_eq!(host.info.PlanetInfo.ScanRetrieveMask, [19, 23, 29]);
                    // No owned object was dropped during orbital loading.
                    assert_eq!(host.scene, scene);
                    assert_eq!(host.surface.is_some(), body == Body::Planet(2));
                    host.leave();
                    if body == Body::Planet(2) {
                        expected.push("free-earth");
                    }
                    if scene.is_some() {
                        expected.push("free-scene");
                    }
                    assert_eq!(*story.trace.borrow(), expected);
                    host.leave();
                    assert_eq!(
                        *story.trace.borrow(),
                        expected,
                        "cleanup must consume each resource once"
                    );
                }
            }
        }
    }
}

#[test]
fn lunar_bio_seed_follows_minerals_and_other_scans_keep_incoming_seeds() {
    for body in [
        Body::Planet(2),
        LUNA,
        Body::Moon { planet: 4, moon: 0 },
        PLUTO,
    ] {
        let (mut story, queue) = pair();
        story.flags.insert("FOUND_PLUTO_SPATHI", 1);
        story.flags.insert("MOONBASE_DESTROYED", 1);
        let mut host = Site::new(queue, body);
        assert!(generate_orbital(body, &story, &mut host));
        assert_eq!(
            host.info.PlanetInfo.ScanSeed[1], 13,
            "energy seed is not written"
        );
        if body == LUNA {
            assert_eq!(host.info.PlanetInfo.ScanSeed[2], host.rng.seed());
            assert_eq!(host.info.PlanetInfo.SurfaceTemperature, -18);
            assert_eq!(host.info.PlanetInfo.SurfaceGravity, 15);
        } else {
            assert_eq!(host.info.PlanetInfo.ScanSeed[2], 17);
        }
        if body == Body::Planet(2) {
            let info = host.info.PlanetInfo;
            assert_eq!(
                (
                    info.AtmoDensity,
                    info.PlanetDensity,
                    info.PlanetRadius,
                    info.AxialTilt,
                    info.Weather,
                    info.Tectonics,
                    info.RotationPeriod,
                    info.SurfaceTemperature,
                    info.SurfaceGravity
                ),
                (50, 100, 100, 23, 1, 1, 240, 22, 100)
            );
        }
    }
}
