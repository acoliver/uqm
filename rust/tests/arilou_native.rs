//! Exercise the Arilou live descriptor and registered callbacks against the linked engine.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "support/special_hooks.rs"]
mod special_hooks;
use battle::FrameSet;
use battle::{Battle, Ship};
use uqm_engine::battle::ai_constants::{
    ENEMY_SHIP_INDEX, ENEMY_WEAPON_INDEX, IGNORE_SIMILAR, IMMEDIATE_WEAPON, LEFT,
    MOVEMENT_STATE_ENTICE, MOVEMENT_STATE_PURSUE, RIGHT, SEEKING_SPECIAL, SEEKING_WEAPON, SPECIAL,
    THRUST, WEAPON,
};
use uqm_engine::battle::flags::{
    APPEARING, CHANGING, CREW_OBJECT, FINITE_LIFE, NONSOLID, NORMAL_LIFE, PLAYER_SHIP,
    SHIP_AT_MAX_SPEED,
};
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::battle::LINE_PRIMITIVE;
use uqm_engine::battle::STAMP_PRIMITIVE;
use uqm_engine::collections::observation;
use uqm_engine::graphics::drawable::FrameId;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::random;
use uqm_engine::math::seed_random;
use uqm_engine::species::species_id::SPECIES_ID_ARILOU_ID;

fn index_of(frame: FrameId) -> u16 {
    frame.index()
}

struct Fleet<'a> {
    fixture: Ship<'a>,
}

impl<'a> Fleet<'a> {
    fn new(battle: &'a Battle, player: i16) -> Self {
        Self {
            fixture: Ship::bare(battle, SPECIES_ID_ARILOU_ID, player),
        }
    }

    fn spawn_ship(&mut self, player: i16, x: i16, y: i16) -> ElementId {
        let ship = self.fixture.id;
        battle::get_mut(ship).player_nr = player;
        battle::get_mut(ship).state_flags = PLAYER_SHIP as u16;
        battle::get_mut(ship).life = NORMAL_LIFE as u16;
        battle::get_mut(ship).current.location = Point { x, y };
        battle::get_mut(ship).next.location = Point { x, y };
        battle::prim_mut(ship).kind = STAMP_PRIMITIVE;
        self.fixture.queue();
        ship
    }
}

type Rgba = (u8, u8, u8, u8);

const KEYS: u16 = (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;

/// Draws `count` values from the engine generator after seeding it.
fn draws(seed: u32, count: usize) -> Vec<u32> {
    seed_random(seed);
    (0..count).map(|_| random()).collect()
}

/// First seed whose opening draws satisfy `accept`.
fn seed_where(count: usize, accept: impl Fn(&[u32]) -> bool) -> u32 {
    (1..1_000_000)
        .find(|&seed| accept(&draws(seed, count)))
        .expect("a suitable seed exists")
}

/// Seeds the generator, runs `action`, and reports how many draws it took.
fn draws_consumed(seed: u32, action: impl FnOnce()) -> usize {
    let sequence = draws(seed, 16);
    seed_random(seed);
    action();
    let next = random();
    sequence
        .iter()
        .position(|&value| value == next)
        .expect("generator advanced by fewer than sixteen draws")
}

fn velocity_of(element: ElementId) -> (i16, i16) {
    let (velocity_x, velocity_y) = (battle::get(element).velocity).get_current_components();
    let (dx, dy) = (velocity_x as i16, velocity_y as i16);
    (dx, dy)
}

#[test]
fn skiff_descriptor_matches_the_c_template() {
    battle_context::run(|_| {
        {
            let template = battle::descriptor(SPECIES_ID_ARILOU_ID);
            let desc = &template;
            assert!(desc.race.is_some());
            assert_eq!(desc.ship_info.ship_flags, IMMEDIATE_WEAPON as u16);
            assert_eq!(desc.ship_info.ship_cost, 16);
            assert_eq!((desc.ship_info.crew_level, desc.ship_info.max_crew), (6, 6));
            assert_eq!(
                (desc.ship_info.energy_level, desc.ship_info.max_energy),
                (20, 20)
            );
            assert_eq!(
                desc.ship_info.race_strings_rsc.unwrap(),
                c"ship.arilou.text"
            );
            assert_eq!(desc.ship_info.icons_rsc.unwrap(), c"ship.arilou.icons");
            assert_eq!(
                desc.ship_info.melee_icon_rsc.unwrap(),
                c"ship.arilou.meleeicons"
            );
            // 250 / SPHERE_RADIUS_INCREMENT * 2
            assert_eq!(desc.fleet.strength, 44);
            assert_eq!(
                (desc.fleet.known_loc.x, desc.fleet.known_loc.y),
                (438, 6372)
            );
            let c = &desc.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (40, 40, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (2, 3, 6)
            );
            assert_eq!((c.turn_wait, c.thrust_wait, c.weapon_wait), (0, 0, 1));
            assert_eq!((c.special_wait, c.ship_mass), (2, 1));

            let data = &desc.ship_data;
            let expected_ship = [
                c"ship.arilou.graphics.skiff.large",
                c"ship.arilou.graphics.skiff.medium",
                c"ship.arilou.graphics.skiff.small",
            ];
            let expected_special = [
                c"ship.arilou.graphics.warp.large",
                c"ship.arilou.graphics.warp.medium",
                c"ship.arilou.graphics.warp.small",
            ];
            for i in 0..3 {
                assert_eq!(data.ship_rsc[i].unwrap(), expected_ship[i]);
                assert_eq!(data.special_rsc[i].unwrap(), expected_special[i]);
                assert!(data.weapon_rsc[i].is_none());
                assert!(data.ship.view(i).is_absent());
                assert!(data.weapon.view(i).is_absent());
                assert!(data.special.view(i).is_absent());
            }
            assert_eq!(
                data.captain_control.captain_rsc.unwrap(),
                c"ship.arilou.graphics.captain"
            );
            assert_eq!(data.victory_ditty_rsc.unwrap(), c"ship.arilou.ditty");
            assert_eq!(data.ship_sounds_rsc.unwrap(), c"ship.arilou.sounds");

            // LASER_RANGE >> 1, with LASER_RANGE = DISPLAY_TO_WORLD (109).
            assert_eq!(desc.cyborg_control.maneuverability_index, 0);
            assert_eq!(desc.cyborg_control.weapon_range, 218);
            assert_ne!(desc.callbacks()[1], 0);
            assert_ne!(desc.callbacks()[3], 0);
            assert_ne!(desc.callbacks()[4], 0);
            assert_eq!(desc.callbacks()[0], 0);
            assert_eq!(desc.callbacks()[2], 0);
        }
    })
}

fn fire(fleet: &mut Fleet, ship: ElementId) -> ElementId {
    let id = ship;
    crate::battle_context::with(|cx| {
        let origin = cx.weapon_origin(id);
        let out = cx.spawn_weapons(fleet.fixture.ship_id, origin);
        assert_eq!(out.len(), 1);
        out[0].unwrap_or_else(|| panic!("allocated laser"))
    })
}

#[test]
fn laser_snaps_to_the_enemy_within_the_turn_and_is_yellow() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut other = Fleet::new(&_battle, 1);
            // Straight along +x: arctan gives angle 16, facing 4, four facings
            // from the ship's facing 0.
            other.spawn_ship(1, 2000, 1000);
            fleet.fixture.ship_mut().ship_facing = 0;

            let laser = fire(&mut fleet, ship);

            // Angle 16 points along +x. The beam starts nine display pixels out
            // and its end point lies LASER_RANGE (109 display pixels) away.
            assert_eq!(
                (
                    battle::get(laser).current.location.x,
                    battle::get(laser).current.location.y
                ),
                (1000 + 9 * 4, 1000)
            );
            assert_eq!(velocity_of(laser), ((109 - 9) * 4 * 32, 0));
            assert_eq!(
                battle::get(laser).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!(battle::get(laser).life, 1);
            assert_eq!(battle::get(laser).hits, 1);
            assert_eq!(battle::get(laser).mass, 1);
            assert_eq!(battle::get(laser).thrust, 1);
            assert_eq!(battle::get(laser).player_nr, 0);
            assert!(battle::get(laser).collision_func.is_some());
            assert!(battle::get(ship).target.is_none());

            // BUILD_COLOR (MAKE_RGB15 (0x1F, 0x1F, 0x0A), 0x0E) on a line.
            let prim = battle::prim(laser);
            assert_eq!(prim.kind, LINE_PRIMITIVE);
            let color: Rgba = (prim.color.r, prim.color.g, prim.color.b, prim.color.a);
            assert_eq!(color, (255, 255, 82, 255));
        }
    })
}

#[test]
fn laser_without_a_target_follows_the_ship_facing() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            fleet.fixture.ship_mut().ship_facing = 8;

            let laser = fire(&mut fleet, ship);

            // Facing 8 is angle 32, straight down the screen.
            assert_eq!(
                (
                    battle::get(laser).current.location.x,
                    battle::get(laser).current.location.y
                ),
                (1000, 1000 + 9 * 4)
            );
            assert_eq!(velocity_of(laser), (0, (109 - 9) * 4 * 32));
        }
    })
}

#[test]
fn laser_aims_exactly_at_a_target_directly_behind() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut other = Fleet::new(&_battle, 1);
            // Eight facings away: TrackShip turns a random way by one step, but
            // Arilou adds the whole delta, so the beam points straight at it.
            other.spawn_ship(1, 1000, 2000);
            fleet.fixture.ship_mut().ship_facing = 0;

            let seed = seed_where(1, |d| d[0] & 1 == 0);
            seed_random(seed);
            let laser = fire(&mut fleet, ship);
            assert_eq!(velocity_of(laser), (0, (109 - 9) * 4 * 32));

            let seed = seed_where(1, |d| d[0] & 1 == 1);
            seed_random(seed);
            let laser = fire(&mut fleet, ship);
            assert_eq!(velocity_of(laser), (0, (109 - 9) * 4 * 32));
        }
    })
}

/// Runs the Arilou intelligence with no concerns for `ship_intelligence` to
/// act on, so only the Skiff's own decisions show in the input state.
fn think(fleet: &mut Fleet, ship: ElementId, concerns: &mut [Concern; 4], input: u8) -> u8 {
    fleet.fixture.ship_mut().ship_input_state = input;
    let mut subject = crate::battle_context::with(|cx| cx.weapon_origin(ship));
    fleet
        .fixture
        .intelligence_subject(&mut subject, concerns, 0);
    fleet.fixture.ship_mut().ship_input_state
}

fn empty_concerns() -> [Concern; 4] {
    [Concern {
        object: None,
        facing: 0,
        which_turn: 0,
        movement: MOVEMENT_STATE_PURSUE,
    }; 4]
}

fn threat_concerns(threat: ElementId, which_turn: u16) -> [Concern; 4] {
    let mut concerns = empty_concerns();
    concerns[ENEMY_WEAPON_INDEX as usize].object = Some(threat);
    concerns[ENEMY_WEAPON_INDEX as usize].which_turn = which_turn;
    concerns
}

#[test]
fn intelligence_always_thrusts_and_entices_the_enemy() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut concerns = empty_concerns();

            let input = think(&mut fleet, ship, &mut concerns, 0);
            assert_eq!(input, THRUST as u8);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].movement,
                MOVEMENT_STATE_ENTICE
            );
            // A stale SPECIAL is dropped when the teleporter is ready.
            let input = think(&mut fleet, ship, &mut concerns, SPECIAL as u8);
            assert_eq!(input, THRUST as u8);
            // A running teleporter counter keeps whatever SPECIAL was asked for.
            fleet.fixture.ship_mut().special_counter = 1;
            let input = think(&mut fleet, ship, &mut concerns, SPECIAL as u8);
            assert_eq!(input, (THRUST | SPECIAL) as u8);
        }
    })
}

#[test]
fn intelligence_teleports_from_immediate_weapons_one_time_in_four() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut other = Fleet::new(&_battle, 1);
            // A PLAYER_SHIP weapon object stands for an IMMEDIATE_WEAPON.
            let beam = other.spawn_ship(1, 1200, 1000);
            let busy = (LEFT | RIGHT | WEAPON) as u8;
            // A running weapon counter keeps the engine pass from clearing WEAPON.
            fleet.fixture.ship_mut().weapon_counter = 1;

            let taken = seed_where(1, |d| d[0] & 3 == 0);
            let mut concerns = threat_concerns(beam, 6);
            let mut input = 0;
            let used = draws_consumed(taken, || {
                input = think(&mut fleet, ship, &mut concerns, busy);
            });
            // Steering, thrust and weapon are all dropped for the jump.
            assert_eq!(input, SPECIAL as u8);
            assert_eq!(used, 1);

            let declined = seed_where(1, |d| d[0] & 3 != 0);
            let mut concerns = threat_concerns(beam, 6);
            let used = draws_consumed(declined, || {
                input = think(&mut fleet, ship, &mut concerns, busy);
            });
            assert_eq!(input, (LEFT | RIGHT | THRUST | WEAPON) as u8);
            assert_eq!(used, 1);
        }
    })
}

#[test]
fn intelligence_ignores_threats_past_six_turns_without_touching_the_generator() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut other = Fleet::new(&_battle, 1);
            let beam = other.spawn_ship(1, 1200, 1000);
            let seed = seed_where(1, |d| d[0] & 3 == 0);

            let mut concerns = threat_concerns(beam, 7);
            let mut input = 0;
            let used = draws_consumed(seed, || {
                input = think(&mut fleet, ship, &mut concerns, SPECIAL as u8);
            });
            assert_eq!(input, THRUST as u8);
            assert_eq!(used, 0);

            // No threat at all.
            let mut concerns = empty_concerns();
            let used = draws_consumed(seed, || {
                think(&mut fleet, ship, &mut concerns, 0);
            });
            assert_eq!(used, 0);

            // A running teleporter counter skips the whole block.
            fleet.fixture.ship_mut().special_counter = 2;
            let mut concerns = threat_concerns(beam, 1);
            let used = draws_consumed(seed, || {
                think(&mut fleet, ship, &mut concerns, 0);
            });
            assert_eq!(used, 0);
        }
    })
}

#[test]
fn intelligence_teleports_from_homing_weapons_only_when_close_or_fighters() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut other = Fleet::new(&_battle, 1);
            let missile = other.spawn_ship(1, 3000, 1000);
            battle::get_mut(missile).state_flags = 0;
            let taken = seed_where(1, |d| d[0] & 3 == 0);
            let weapon_frames = other.fixture.frame_set(1);
            let special_frames = other.fixture.frame_set(2);

            // A seeking weapon aimed through the enemy's weapon frames: only a
            // hit next turn or a crew object (fighters) counts.
            other.fixture.descriptor_mut().ship_info.ship_flags = SEEKING_WEAPON as u16;
            battle::get_mut(missile).next.image.frames = weapon_frames.clone();
            for (which_turn, flags, expected) in [
                (1, 0, 1),
                (2, 0, 0),
                (2, CREW_OBJECT as u16, 1),
                (6, CREW_OBJECT as u16, 1),
            ] {
                battle::get_mut(missile).state_flags = flags;
                let mut concerns = threat_concerns(missile, which_turn);
                let used = draws_consumed(taken, || {
                    think(&mut fleet, ship, &mut concerns, 0);
                });
                assert_eq!(used, expected, "weapon turn {which_turn} flags {flags:#x}");
            }

            // The same frames do not count unless the race flags a seeking weapon.
            other.fixture.descriptor_mut().ship_info.ship_flags = 0;
            battle::get_mut(missile).state_flags = 0;
            let mut concerns = threat_concerns(missile, 1);
            let used = draws_consumed(taken, || {
                think(&mut fleet, ship, &mut concerns, 0);
            });
            assert_eq!(used, 0);

            // Seeking specials are matched against the special frames instead.
            other.fixture.descriptor_mut().ship_info.ship_flags = SEEKING_SPECIAL as u16;
            battle::get_mut(missile).next.image.frames = special_frames.clone();
            let mut concerns = threat_concerns(missile, 1);
            let used = draws_consumed(taken, || {
                think(&mut fleet, ship, &mut concerns, 0);
            });
            assert_eq!(used, 1);

            // A special-seeking race's weapon frames do not match its special flag.
            battle::get_mut(missile).next.image.frames = weapon_frames.clone();
            let mut concerns = threat_concerns(missile, 1);
            let used = draws_consumed(taken, || {
                think(&mut fleet, ship, &mut concerns, 0);
            });
            assert_eq!(used, 0);
        }
    })
}

#[test]
fn intelligence_skips_objects_that_cannot_intercept() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut other = Fleet::new(&_battle, 1);
            let shell = other.spawn_ship(1, 1150, 1150);
            battle::get_mut(shell).state_flags = 0;
            let taken = seed_where(1, |d| d[0] & 3 == 0);

            // Neither a PLAYER_SHIP nor a homing weapon, and stationary: the
            // PlotIntercept test fails, so the generator is never consulted. The
            // positive PlotIntercept result needs drawable pixel data, which the
            // engine's frame-intersection test reads; it is covered by the engine
            // rather than here.
            assert_eq!(battle::plot_intercept(shell, ship, 3, 0), 0);
            let mut concerns = threat_concerns(shell, 3);
            let mut input = 0;
            let used = draws_consumed(taken, || {
                input = think(&mut fleet, ship, &mut concerns, SPECIAL as u8);
            });
            assert_eq!(used, 0);
            assert_eq!(input, THRUST as u8);
        }
    })
}

#[test]
fn intelligence_stops_firing_when_the_skiff_cannot_afford_both() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Fleet::new(&_battle, 0);
            let ship = fleet.spawn_ship(0, 1000, 1000);
            let mut concerns = empty_concerns();
            // A running weapon counter keeps the engine pass from clearing WEAPON.
            fleet.fixture.ship_mut().weapon_counter = 1;

            // SPECIAL_ENERGY_COST << 1 is six: at six the weapon is dropped.
            fleet.fixture.descriptor_mut().ship_info.energy_level = 6;
            let input = think(&mut fleet, ship, &mut concerns, WEAPON as u8);
            assert_eq!(input, THRUST as u8);
            fleet.fixture.descriptor_mut().ship_info.energy_level = 7;
            let input = think(&mut fleet, ship, &mut concerns, WEAPON as u8);
            assert_eq!(input, (THRUST | WEAPON) as u8);
        }
    })
}

struct Skiff<'a> {
    fleet: Fleet<'a>,
    ship: ElementId,
    ship_frames: FrameSet,
    warp_frames: FrameSet,
}

impl<'a> Skiff<'a> {
    /// A skiff at (100, 200) facing 6, flying on its ship frames.
    fn new(battle: &'a Battle) -> Self {
        let mut fleet = Fleet::new(battle, 0);
        let ship_frames = FrameSet::with_bounds(16, [0, 0]);
        let warp_frames = FrameSet::with_bounds(8, [0, 0]);
        fleet.fixture.descriptor_mut().ship_data.ship = ship_frames.array();
        fleet.fixture.descriptor_mut().ship_data.special = warp_frames.array();
        let ship = fleet.spawn_ship(0, 100, 200);
        fleet.fixture.ship_mut().ship_facing = 6;
        let ship_array = fleet.fixture.frame_set(0);
        battle::get_mut(ship).current.image.frames = ship_array.clone();
        battle::get_mut(ship).next.image.frames = ship_array.clone();
        battle::get_mut(ship).current.image.frame = ship_frames.selection(6);
        battle::get_mut(ship).next.image.frame = ship_frames.selection(6);
        Self {
            fleet,
            ship,
            ship_frames,
            warp_frames,
        }
    }

    fn preprocess(&mut self) {
        self.fleet.fixture.preprocess();
    }

    fn warp_array(&mut self) -> uqm_engine::battle::views::Frames {
        self.fleet.fixture.frame_set(2)
    }

    fn ship_array(&mut self) -> uqm_engine::battle::views::Frames {
        self.fleet.fixture.frame_set(0)
    }
}

#[test]
fn skiff_stops_dead_unless_the_thrust_delay_is_running() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut skiff = Skiff::new(&_battle);
            let ship = skiff.ship;
            battle::get_mut(ship).velocity.travel_angle = 37;
            (battle::get_mut(ship).velocity).delta_components(640, -320);
            skiff.fleet.fixture.ship_mut().cur_status_flags = (SHIP_AT_MAX_SPEED | THRUST) as u16;

            // thrust_wait != 0: the engine is still coasting the thrust delay.
            battle::get_mut(ship).thrust = 3;
            skiff.preprocess();
            assert_eq!(velocity_of(ship), (640, -320));
            assert_eq!(
                skiff.fleet.fixture.ship_mut().cur_status_flags,
                (SHIP_AT_MAX_SPEED | THRUST) as u16
            );

            // thrust_wait == 0: ZeroVelocityComponents memsets the whole record,
            // TravelAngle included.
            battle::get_mut(ship).thrust = 0;
            skiff.preprocess();
            let velocity = battle::get(ship).velocity;
            assert_eq!(velocity.travel_angle, 0);
            assert_eq!((velocity.vector.width, velocity.vector.height), (0, 0));
            assert_eq!((velocity.fract.width, velocity.fract.height), (0, 0));
            assert_eq!((velocity.error.width, velocity.error.height), (0, 0));
            assert_eq!((velocity.incr.width, velocity.incr.height), (0, 0));
            assert_eq!(
                skiff.fleet.fixture.ship_mut().cur_status_flags,
                THRUST as u16
            );
        }
    })
}

#[test]
fn special_key_starts_the_teleport() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        let mut skiff = Skiff::new(&_battle);
        let ship = skiff.ship;
        battle::get_mut(ship).velocity.travel_angle = 37;
        (battle::get_mut(ship).velocity).delta_components(640, -320);
        battle::get_mut(ship).thrust = 3;
        skiff.fleet.fixture.ship_mut().cur_status_flags =
            (SHIP_AT_MAX_SPEED | LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;
        observation::enable();
        observation::drain();

        skiff.preprocess();

        assert_eq!(special_hooks::count(&observation::drain(), "arilou"), 1);
        assert_eq!(
            skiff.fleet.fixture.descriptor_mut().ship_info.energy_level,
            17
        );
        assert_eq!(skiff.fleet.fixture.ship_mut().special_counter, 2);
        assert_eq!(
            skiff.fleet.fixture.ship_mut().cur_status_flags,
            SPECIAL as u16
        );
        assert_eq!(velocity_of(ship), (0, 0));
        assert_eq!(battle::get(ship).velocity.travel_angle, 0);
        assert_eq!(
            battle::get(ship).state_flags,
            PLAYER_SHIP as u16 | (NONSOLID | FINITE_LIFE | CHANGING) as u16
        );
        assert_eq!(battle::get(ship).life, 5);
        assert_eq!(battle::get(ship).next.image.frames, skiff.warp_array());
        assert_eq!(index_of(battle::get(ship).next.image.frame.raw()), 0);
        assert_eq!(
            battle::get(ship).next.image.frame.raw(),
            skiff.warp_frames.frame(0),
            "the warp animation starts on its first frame"
        );
    })
}

#[test]
fn teleport_needs_the_key_a_ready_counter_and_energy() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        let mut skiff = Skiff::new(&_battle);
        let ship = skiff.ship;

        observation::enable();
        for (flags, counter, energy) in
            [(0, 0, 20), (SPECIAL as u16, 1, 20), (SPECIAL as u16, 0, 2)]
        {
            skiff.fleet.fixture.ship_mut().cur_status_flags = flags;
            skiff.fleet.fixture.ship_mut().special_counter = counter;
            skiff.fleet.fixture.descriptor_mut().ship_info.energy_level = energy;
            observation::drain();
            skiff.preprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "arilou"), 0);
            assert_eq!(battle::get(ship).state_flags, PLAYER_SHIP as u16);
            assert_eq!(
                skiff.fleet.fixture.descriptor_mut().ship_info.energy_level,
                energy
            );
            assert_eq!(skiff.fleet.fixture.ship_mut().special_counter, counter);
            assert_eq!(battle::get(ship).next.image.frames, skiff.ship_array());
        }
    })
}

/// Puts the skiff in mid-teleport the way the key press leaves it, with the
/// engine's own life countdown applied by hand.
fn start_teleport(skiff: &mut Skiff) {
    skiff.fleet.fixture.ship_mut().cur_status_flags = SPECIAL as u16;
    skiff.preprocess();
    assert_ne!(battle::get(skiff.ship).state_flags & NONSOLID as u16, 0);
    battle::get_mut(skiff.ship).state_flags &= !(CHANGING as u16);
}

fn set_life(ship: ElementId, life: u16) {
    battle::get_mut(ship).life = life;
}

#[test]
fn teleport_animates_jumps_at_life_three_and_returns_at_life_one() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut skiff = Skiff::new(&_battle);
            let ship = skiff.ship;
            start_teleport(&mut skiff);
            let warp = skiff.warp_frames.frame(0);
            assert_eq!(battle::get(ship).next.image.frame.raw(), warp);

            // Lives 5 and 4 play the warp frames forward.
            for (life, frame) in [(5, 1), (4, 2)] {
                set_life(ship, life);
                skiff.preprocess();
                assert_eq!(
                    index_of(battle::get(ship).next.image.frame.raw()),
                    frame,
                    "life {life}"
                );
                // The decrement is only applied to a local; the engine owns it.
                assert_eq!(battle::get(ship).life, life);
                assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
                battle::get_mut(ship).state_flags &= !(CHANGING as u16);
            }

            // Life 3: the local count reaches two, the animation holds its frame
            // and the skiff reappears at a random place in logical space. The
            // first draw places x and the second places y.
            let seed = seed_where(3, |d| {
                // Draws whose high bits matter: the modulus is taken on the
                // low sixteen bits (a u16) and 7680 does not divide 65536.
                d[0] > 0xFFFF && d[1] % 7680 != (d[1] as u16 as u32) % 7680
            });
            let sequence = draws(seed, 3);
            seed_random(seed);
            set_life(ship, 3);
            skiff.preprocess();
            // 320x240 screen: logical space is 256 * 32 by 240 * 32.
            let expect_x = (sequence[0] as u16 as i32 % 8192) & !3;
            let expect_y = (sequence[1] as u16 as i32 % 7680) & !3;
            assert_eq!(
                (
                    battle::get(ship).next.location.x,
                    battle::get(ship).next.location.y
                ),
                (expect_x as i16, expect_y as i16)
            );
            assert_eq!(random(), sequence[2], "two draws, x before y");
            assert_eq!(index_of(battle::get(ship).next.image.frame.raw()), 2);
            assert_eq!(battle::get(ship).life, 3);
            assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
            battle::get_mut(ship).state_flags &= !(CHANGING as u16);

            // Life 2: the local count is one, below two, so the frames run back.
            set_life(ship, 2);
            skiff.preprocess();
            assert_eq!(index_of(battle::get(ship).next.image.frame.raw()), 1);
            assert_eq!(
                (
                    battle::get(ship).next.location.x,
                    battle::get(ship).next.location.y
                ),
                (expect_x as i16, expect_y as i16)
            );
            battle::get_mut(ship).state_flags &= !(CHANGING as u16);

            // Life 1 is NORMAL_LIFE: the teleport ends.
            battle::get_mut(ship).current.location = Point { x: -9, y: 13 };
            battle::get_mut(ship).intersect_control.end_point = Point { x: 123, y: -456 };
            set_life(ship, 1);
            skiff.preprocess();
            let ship_array = skiff.ship_array();
            assert_eq!(
                battle::get(ship).state_flags,
                PLAYER_SHIP as u16 | (APPEARING | CHANGING) as u16
            );
            assert_eq!(battle::get(ship).current.image.frames, ship_array);
            assert_eq!(battle::get(ship).next.image.frames, ship_array);
            assert_eq!(index_of(battle::get(ship).current.image.frame.raw()), 6);
            assert_eq!(
                battle::get(ship).current.image.frame.raw(),
                battle::get(ship).next.image.frame.raw()
            );
            assert_eq!(
                battle::get(ship).current.image.frame.raw(),
                skiff.ship_frames.frame(6)
            );
            // InitIntersectStartPoint: WORLD_TO_DISPLAY is an arithmetic >> 2.
            let origin = battle::get(ship).intersect_control.intersect_stamp.origin;
            assert_eq!((origin.x, origin.y), (-3, 3));
            let end = battle::get(ship).intersect_control.end_point;
            assert_eq!((end.x, end.y), (123, -456));
        }
    })
}

#[test]
fn teleport_frame_ticks_every_counter_and_restores_the_pressed_keys() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut skiff = Skiff::new(&_battle);
            let ship = skiff.ship;
            start_teleport(&mut skiff);
            let mut starship = skiff.fleet.fixture.ship_mut();
            starship.weapon_counter = 4;
            starship.special_counter = 255;
            starship.energy_counter = 9;
            starship.cur_status_flags = (LEFT | SHIP_AT_MAX_SPEED) as u16;
            starship.old_status_flags = (THRUST | SPECIAL | 0x4000) as u16;
            drop(starship);
            battle::get_mut(ship).turn = 255;
            battle::get_mut(ship).thrust = 7;
            set_life(ship, 5);

            skiff.preprocess();

            assert_eq!(battle::get(ship).turn, 0);
            assert_eq!(battle::get(ship).thrust, 8);
            let starship = skiff.fleet.fixture.ship_mut();
            assert_eq!(starship.weapon_counter, 5);
            // u8 counters wrap rather than saturate.
            assert_eq!(starship.special_counter, 0);
            assert_eq!(starship.energy_counter, 10);
            // Held keys revert to the pre-teleport state; other bits are kept.
            assert_eq!(starship.cur_status_flags & KEYS, (THRUST | SPECIAL) as u16);
            assert_ne!(starship.cur_status_flags & SHIP_AT_MAX_SPEED as u16, 0);
        }
    })
}

#[test]
fn nonsolid_skiff_off_the_warp_frames_is_left_alone() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        let mut skiff = Skiff::new(&_battle);
        let ship = skiff.ship;
        battle::get_mut(ship).state_flags |= NONSOLID as u16;
        skiff.fleet.fixture.ship_mut().weapon_counter = 4;
        set_life(ship, 4);

        skiff.preprocess();

        assert_eq!(skiff.fleet.fixture.ship_mut().weapon_counter, 4);
        assert_eq!(
            battle::get(ship).state_flags,
            (PLAYER_SHIP | NONSOLID) as u16
        );
        assert_eq!(index_of(battle::get(ship).next.image.frame.raw()), 6);
    })
}

#[test]
fn init_intersect_start_preserves_endpoint_and_frame() {
    battle_context::run(|_| {
        let battle = Battle::new(4);
        let mut ship = Ship::new(&battle, SPECIES_ID_ARILOU_ID, 0);
        let frame = ship.element_mut().intersect_control.intersect_stamp.frame;
        ship.element_mut().current.location = Point { x: -9, y: 13 };
        ship.element_mut().next.location = Point { x: 400, y: 800 };
        ship.element_mut().intersect_control.end_point = Point { x: 123, y: -456 };
        crate::battle_context::with(|cx| cx.init_intersect_start(ship.id));
        let control = ship.element_mut().intersect_control;
        assert_eq!(
            (
                control.intersect_stamp.origin.x,
                control.intersect_stamp.origin.y
            ),
            (-3, 3)
        );
        assert_eq!((control.end_point.x, control.end_point.y), (123, -456));
        assert_eq!(control.intersect_stamp.frame, frame);
    })
}
