//! Exercise the Pkunk descriptor and callbacks against the linked engine.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "support/special_hooks.rs"]
mod special_hooks;

use std::sync::atomic::{AtomicUsize, Ordering};
use uqm_content::ships::native::{
    angle_to_facing, cosine, facing_to_angle, normalize_facing, sine,
};
use uqm_engine::battle::ai_constants::FIRES_FORE;
use uqm_engine::battle::ai_constants::FIRES_LEFT;
use uqm_engine::battle::ai_constants::FIRES_RIGHT;
use uqm_engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_engine::battle::ai_constants::LEFT;
use uqm_engine::battle::ai_constants::RIGHT;
use uqm_engine::battle::ai_constants::SPECIAL;
use uqm_engine::battle::ai_constants::THRUST;
use uqm_engine::battle::ai_constants::WEAPON;
use uqm_engine::battle::flags::{
    APPEARING, CHANGING, DISAPPEARING, FINITE_LIFE, NONSOLID, NORMAL_LIFE, PLAYER_SHIP,
    STAMPFILL_PRIMITIVE,
};
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::battle::{EMPTY_PRIMITIVE, STAMP_PRIMITIVE};
use uqm_engine::collections::observation;
use uqm_engine::graphics::geometry::Color;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::random;
use uqm_engine::math::seed_random;
use uqm_engine::math::velocity::{VelocityDesc, VELOCITY_SHIFT};
use uqm_engine::species::species_id::SPECIES_ID_PKUNK_ID;

// races.h / intel.h flags that are macros with casts rather than constants.
const STANDARD_RATING: u8 = 1 << 4;
const GOOD_RATING: u8 = 1 << 5;
const LOW_ON_ENERGY: u16 = 1 << 5;
const PLAY_VICTORY_DITTY: u16 = 1 << 9;

// LOG_SPACE_WIDTH / LOG_SPACE_HEIGHT for the 320x240 screen the tests install.
const SPACE_W: i32 = (320 - 64) * 4 * 8;
const SPACE_H: i32 = 240 * 4 * 8;

struct Battle {
    inner: battle::Battle,
}

impl Battle {
    fn new() -> Self {
        Self::with_capacity(64)
    }

    fn with_capacity(capacity: u16) -> Self {
        Self {
            inner: battle::Battle::new(capacity),
        }
    }
}

// The same engine-readable drawable layout used by the other native race tests.
use battle::FrameSet;

#[derive(Clone, Copy)]
struct SavedState {
    phoenix: Option<ElementId>,
    saved_preprocess: Option<uqm_engine::battle::ElementHook>,
    saved_postprocess: Option<uqm_engine::battle::ElementHook>,
    saved_death: Option<uqm_engine::battle::ElementHook>,
}

static MARK: AtomicUsize = AtomicUsize::new(0);

// Stand-ins for the engine callbacks Pkunk saves around an explosion. Each
// body differs so the functions keep distinct addresses.
fn engine_preprocess(_: &mut uqm_engine::battle::Cx<'_>, _: uqm_engine::battle::ElementId) {
    MARK.store(1, Ordering::SeqCst);
}
fn engine_postprocess(_: &mut uqm_engine::battle::Cx<'_>, _: uqm_engine::battle::ElementId) {
    MARK.store(2, Ordering::SeqCst);
}
fn engine_death(_: &mut uqm_engine::battle::Cx<'_>, _: uqm_engine::battle::ElementId) {
    MARK.store(3, Ordering::SeqCst);
}

fn addr(f: Option<uqm_engine::battle::ElementHook>) -> Option<usize> {
    f.map(|f| (f).address())
}

struct Ship<'a> {
    inner: battle::Ship<'a>,
    id: ElementId,
    ship_frames: FrameSet,
    weapon_frames: FrameSet,
}

impl<'a> Ship<'a> {
    fn new(battle: &'a Battle, player: i16) -> Self {
        let mut inner = battle::Ship::new(&battle.inner, SPECIES_ID_PKUNK_ID, player);
        let id = inner.id;
        let ship_frames = FrameSet::new(16);
        let weapon_frames = FrameSet::new(8);
        {
            let mut desc = inner.descriptor_mut();
            desc.ship_data.ship = ship_frames.array();
            desc.ship_data.weapon = weapon_frames.array();
        }
        let frames = inner.frame_set(0);
        let mut el = inner.element_mut();
        el.current.image.frames = frames;
        el.current.image.frame = ship_frames.selection(0);
        el.next = el.current.clone();
        el.intersect_control.intersect_stamp.frame = ship_frames.frame(0);
        el.intersect_control.end_point = Point { x: 1000, y: 1000 };
        el.intersect_control.intersect_stamp.origin = Point { x: 1000, y: 1000 };
        Self {
            inner,
            id,
            ship_frames,
            weapon_frames,
        }
    }

    fn fire(&mut self, out: &mut [Option<ElementId>]) -> u16 {
        crate::battle_context::with(|cx| {
            let origin = cx.weapon_origin(self.inner.id);
            let weapons = cx.spawn_weapons(self.inner.ship_id, origin);
            for (slot, weapon) in out.iter_mut().zip(&weapons) {
                *slot = *weapon;
            }
            weapons.len() as u16
        })
    }
    fn data(&mut self) -> SavedState {
        let uqm_engine::battle::RaceState::Pkunk {
            phoenix,
            saved_pre,
            saved_post,
            saved_death,
        } = self.inner.descriptor_mut().race_state
        else {
            panic!("Pkunk state");
        };
        SavedState {
            phoenix,
            saved_preprocess: saved_pre,
            saved_postprocess: saved_post,
            saved_death,
        }
    }
    fn intelligence(&mut self, concerns: &mut [Concern; 4]) {
        let mut subject = self.inner.subject();
        // The AI operand is a value copy, not a member of the live queue.
        // Its phoenix predecessor may be disposed during intelligence.
        self.inner.intelligence_subject(&mut subject, concerns, 0);
    }

    fn at(&mut self, x: i16, y: i16) {
        battle::get_mut(self.id).current.location = Point { x, y };
        battle::get_mut(self.id).next.location = Point { x, y };
    }

    fn preprocess(&mut self) {
        self.inner.preprocess();
    }

    /// A brand-new ship about to run its first preprocess.
    fn appearing(&mut self) {
        battle::get_mut(self.id).state_flags = (PLAYER_SHIP | APPEARING) as u16;
        battle::get_mut(self.id).target = None;
    }

    /// The ship `new_pkunk` leaves behind: appearing with hTarget set.
    fn reincarnating(&mut self, x: i16, y: i16, facing: u16) {
        self.at(x, y);
        self.inner.ship_mut().ship_facing = facing;
        battle::get_mut(self.id).state_flags = (PLAYER_SHIP | APPEARING) as u16;
        battle::get_mut(self.id).life = NORMAL_LIFE as u16;
        battle::get_mut(self.id).target = Some(self.id);
    }

    /// Drives the real hooks (the phoenix sees the death, then the death
    /// function explodes the ship) until the death function is `new_pkunk`.
    fn armed_new_pkunk(&mut self) -> uqm_engine::battle::ElementHook {
        self.appearing();
        seed_random(odd_seed());
        self.preprocess();
        let phoenix = self
            .data()
            .phoenix
            .unwrap_or_else(|| panic!("allocated phoenix"));
        self.inner.descriptor_mut().ship_info.crew_level = 0;
        hooks::invoke_element(battle::get(phoenix).death_func.unwrap(), phoenix);
        let explode = battle::get(self.id).death_func.unwrap();
        hooks::invoke_element(explode, self.id);
        battle::get(self.id).death_func.unwrap()
    }

    fn drain(&self) {
        for h in queued() {
            if h != self.id {
                battle_context::unlink(h);
                battle_context::dispose(h);
            }
        }
    }
}

fn queued() -> Vec<ElementId> {
    let mut result = Vec::new();
    let mut cursor = battle::head();
    while let Some(h) = cursor {
        result.push(h);
        assert!(result.len() <= 64, "display queue cycle");
        cursor = battle::succ(h);
    }
    result
}

fn velocity_fields(v: &VelocityDesc) -> [i32; 9] {
    [
        v.travel_angle.into(),
        v.vector.width.into(),
        v.vector.height.into(),
        v.fract.width.into(),
        v.fract.height.into(),
        v.error.width.into(),
        v.error.height.into(),
        v.incr.width.into(),
        v.incr.height.into(),
    ]
}

fn color_bytes(c: Color) -> [u8; 4] {
    [c.r, c.g, c.b, c.a]
}

fn rolls(seed: u32, count: usize) -> Vec<u32> {
    seed_random(seed);
    (0..count).map(|_| random()).collect()
}

/// Runs `run` from `seed` and checks it consumed exactly `draws` values.
fn with_rng<T>(seed: u32, draws: usize, run: impl FnOnce() -> T) -> T {
    let expected = rolls(seed, draws + 1)[draws];
    seed_random(seed);
    let result = run();
    assert_eq!(random(), expected, "RNG draws {draws}, seed {seed}");
    result
}

fn seed_with_first_roll(wanted: impl Fn(u32) -> bool) -> u32 {
    (1..100_000u32)
        .find(|&seed| wanted(rolls(seed, 1)[0]))
        .expect("a seed with the wanted first draw")
}

fn odd_seed() -> u32 {
    seed_with_first_roll(|r| r & 1 == 1)
}

fn even_seed() -> u32 {
    seed_with_first_roll(|r| r & 1 == 0)
}

/// `DISPLAY_ALIGN_X/Y` of one draw: u16 modulus, i16 narrowing, pixel snap.
fn align(roll: u32, extent: i32) -> i32 {
    i32::from(((i32::from(roll as u16)) % extent) as i16) & !3
}

/// `WRAP_X (DISPLAY_ALIGN_X (random ()))` spelled out the way the C
/// preprocessor does: the value operand appears once per use, so each use is
/// its own draw. Returns the coordinate and the index of the next unused roll.
fn expanded_coord(rolls: &[u32], start: usize, extent: i32) -> (i16, usize) {
    let mut next = start;
    let mut draw = || {
        let v = align(rolls[next], extent);
        next += 1;
        v
    };
    let value = if draw() < 0 {
        draw() + extent
    } else if draw() >= extent {
        draw() - extent
    } else {
        draw()
    };
    (value as u16 as i16, next)
}

/// `WRAP_VAL` for a value without side effects, stored back into a i16.
fn wrap_coord(v: i32, extent: i32) -> i16 {
    (if v < 0 {
        v + extent
    } else if v >= extent {
        v - extent
    } else {
        v
    }) as u16 as i16
}

/// Where the player-ship branch of `phoenix_transition` puts a trail image.
fn trail_origin(from: Point, facing: u16, life_span: u16) -> Point {
    let angle = facing_to_angle(i32::from(facing));
    let back = i32::from(life_span) - 1;
    let x = (i32::from(from.x) - i32::from(cosine(angle, 80)) * back) as i16;
    let y = (i32::from(from.y) - i32::from(sine(angle, 80)) * back) as i16;
    Point {
        x: wrap_coord(i32::from(x), SPACE_W),
        y: wrap_coord(i32::from(y), SPACE_H),
    }
}

fn nothing_saved(ship: &mut Ship) -> bool {
    let data = ship.data();
    data.saved_preprocess.is_none()
        && data.saved_postprocess.is_none()
        && data.saved_death.is_none()
}

fn block_is_zero(state: uqm_engine::battle::RaceState) -> bool {
    matches!(
        state,
        uqm_engine::battle::RaceState::Pkunk {
            phoenix: None,
            saved_pre: None,
            saved_post: None,
            saved_death: None
        }
    )
}

fn check_trail(ship: &mut Ship, trail: ElementId, at: Point, angle: u16) {
    assert_eq!(battle::get(trail).player_nr, -1);
    assert_eq!(
        battle::get(trail).state_flags,
        (APPEARING | FINITE_LIFE | NONSOLID) as u16
    );
    assert_eq!(battle::get(trail).life, 1);
    assert_eq!(battle::prim(trail).kind, STAMPFILL_PRIMITIVE);
    // BUILD_COLOR (MAKE_RGB15 (0x1F, 0x15, 0x00), 0x7A)
    assert_eq!(
        color_bytes(battle::prim(trail).color),
        [0xFF, 0xAD, 0x00, 0xFF]
    );
    assert_eq!(battle::get(trail).color_cycle_index, 0);
    assert_eq!(
        battle::get(trail).current.image.frame.raw(),
        battle::get(ship.id).current.image.frame.raw()
    );
    assert_eq!(
        battle::get(trail).current.image.frames,
        battle::get(ship.id).current.image.frames
    );
    assert_eq!(
        (
            battle::get(trail).current.location.x,
            battle::get(trail).current.location.y
        ),
        (at.x, at.y)
    );
    assert_eq!(battle::get(trail).mass, angle as u8);
    assert!(battle::get(trail).preprocess_func.is_some());
    assert!(battle::get(trail).death_func.is_some());
    assert!(battle::get(trail).postprocess_func.is_none());
    assert!(battle::get(trail).collision_func.is_none());
    assert_eq!(
        battle::get(trail).parent,
        uqm_engine::battle::Parent::Ship(ship.inner.ship_id)
    );
}

#[test]
fn descriptor_matches_every_c_initializer_field() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let d = battle::descriptor(SPECIES_ID_PKUNK_ID);
            assert_eq!(
                d.ship_info.ship_flags,
                (FIRES_FORE | FIRES_LEFT | FIRES_RIGHT) as u16
            );
            assert_eq!(d.ship_info.ship_cost, 20);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (8, 8));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (12, 12));
            assert_eq!(d.ship_info.race_strings_rsc.unwrap(), c"ship.pkunk.text");
            assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.pkunk.icons");
            assert_eq!(
                d.ship_info.melee_icon_rsc.unwrap(),
                c"ship.pkunk.meleeicons"
            );
            assert!(d.ship_info.race_strings.is_none());
            assert!(d.ship_info.icons.is_null());
            assert!(d.ship_info.melee_icon.is_null());
            // 666 / SPHERE_RADIUS_INCREMENT * 2
            assert_eq!(d.fleet.strength, 120);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (502, 401));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (64, 16, 0)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (1, 2, 0)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (0, 0, 0, 0, 1)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                // PKUNK_*_MASK_PMAP_ANIM are the fury graphics.
                assert_eq!(
                    d.ship_data.ship_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.pkunk.graphics.fury.{size}")
                );
                assert_eq!(
                    d.ship_data.weapon_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.pkunk.graphics.bug.{size}")
                );
                assert!(d.ship_data.special_rsc[i].is_none());
                assert!(d.ship_data.ship.view(i).is_absent());
                assert!(d.ship_data.weapon.view(i).is_absent());
                assert!(d.ship_data.special.view(i).is_absent());
            }
            let captain = &d.ship_data.captain_control;
            assert_eq!(captain.captain_rsc.unwrap(), c"ship.pkunk.graphics.captain");
            assert!(captain.background.is_null());
            assert!(captain.turn.is_null());
            assert!(captain.thrust.is_null());
            assert!(captain.weapon.is_null());
            assert!(captain.special.is_absent());
            assert_eq!(d.ship_data.victory_ditty_rsc.unwrap(), c"ship.pkunk.ditty");
            assert_eq!(d.ship_data.ship_sounds_rsc.unwrap(), c"ship.pkunk.sounds");
            assert!(d.ship_data.victory_ditty.is_null());
            assert!(d.ship_data.ship_sounds.is_none());
            assert_eq!(d.cyborg_control.maneuverability_index, 0);
            // CLOSE_RANGE_WEAPON + 1
            assert_eq!(d.cyborg_control.weapon_range, 201);
            assert!(d.callbacks()[4] != 0);
            assert!(d.callbacks()[1] != 0);
            assert!(d.callbacks()[2] != 0);
            assert!(d.callbacks()[3] != 0);
            assert!(d.callbacks()[0] != 0);

            assert!(block_is_zero(d.race_state));
        }
    })
}

#[test]
fn every_init_copies_the_template_and_gives_the_copy_its_own_zeroed_state() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        let mut first = Ship::new(&_battle, 0);
        let mut second = Ship::new(&_battle, 1);
        assert!(block_is_zero(first.inner.descriptor_mut().race_state));
        assert!(block_is_zero(second.inner.descriptor_mut().race_state));
        let phoenix = first.inner.id;
        first.inner.descriptor_mut().race_state = uqm_engine::battle::RaceState::Pkunk {
            phoenix: Some(phoenix),
            saved_pre: None,
            saved_post: None,
            saved_death: None,
        };
        second.inner.descriptor_mut().race_state = uqm_engine::battle::RaceState::Pkunk {
            phoenix: None,
            saved_pre: None,
            saved_post: None,
            saved_death: Some(uqm_engine::battle::ElementHook::Native(engine_death)),
        };
        assert!(second.data().phoenix.is_none());
        assert!(first.data().saved_death.is_none());
        first.inner.uninit();
        assert!(matches!(
            first.inner.descriptor_mut().race_state,
            uqm_engine::battle::RaceState::None
        ));
        first.inner.uninit();
        assert!(matches!(
            first.inner.descriptor_mut().race_state,
            uqm_engine::battle::RaceState::None
        ));
        assert_eq!(
            addr(second.data().saved_death),
            addr(Some(uqm_engine::battle::ElementHook::Native(engine_death)))
        );
        second.inner.uninit();
        assert!(matches!(
            second.inner.descriptor_mut().race_state,
            uqm_engine::battle::RaceState::None
        ));
        let mut third = Ship::new(&_battle, 0);
        assert!(block_is_zero(third.inner.descriptor_mut().race_state));
        third.inner.uninit();
    })
}

#[test]
fn ships_keep_independent_phoenix_state() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut a = Ship::new(&_battle, 0);
            let mut b = Ship::new(&_battle, 1);
            assert_ne!(a.inner.ship_id, b.inner.ship_id);
            a.appearing();
            b.appearing();
            with_rng(odd_seed(), 1, || a.preprocess());
            assert!(!a.data().phoenix.is_none());
            assert!(b.data().phoenix.is_none());
            // Only A's descriptor stopped asking for the one-time preprocess.
            assert!(a.inner.descriptor_mut().callbacks()[1] == 0);
            assert!(b.inner.descriptor_mut().callbacks()[1] != 0);
        }
    })
}

#[test]
fn bug_missiles_launch_fore_left_and_right_inheriting_ship_velocity() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 1);
            for facing in 0..16u16 {
                for ship_speed in [0i16, 14] {
                    ship.inner.ship_mut().ship_facing = facing;
                    ship.at(321, -654);
                    uqm_modes::battle::velocity::set_velocity_vector(
                        &mut battle::get_mut(ship.id).velocity,
                        ship_speed,
                        (facing + 3) % 16,
                    );
                    let (velocity_x, velocity_y) =
                        (battle::get(ship.id).velocity).get_current_components();
                    let (dx, dy) = (velocity_x as i16, velocity_y as i16);
                    let mut out = [Some(ship.id); 3];
                    let before = queued();
                    let count = with_rng(5, 0, || ship.fire(&mut out));
                    assert_eq!(count, 3);
                    assert_eq!(
                        queued(),
                        before,
                        "weapons are not queued by the weapon func"
                    );
                    for (i, &h) in out.iter().enumerate() {
                        // Fore, +90 degrees, and +270 degrees (the i == 2 extra quadrant).
                        let turn = [0, 4, 12][i];
                        let face = normalize_facing(i32::from(facing) + turn);
                        let m = h.unwrap();
                        assert_eq!(battle::get(m).life, 5);
                        assert_eq!(battle::get(m).hits, 1);
                        assert_eq!(battle::get(m).mass, 1);
                        assert_eq!(battle::get(m).thrust, 1);
                        assert_eq!(
                            battle::get(m).state_flags,
                            (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                        );
                        assert_eq!(battle::get(m).player_nr, 1);
                        assert_eq!(
                            battle::get(m).current.image.frame.raw(),
                            ship.weapon_frames.frame(0)
                        );
                        assert_eq!(battle::get(m).current.image.frames, ship.inner.frame_set(1));
                        assert_eq!(battle::prim(m).kind, STAMP_PRIMITIVE);
                        assert!(battle::get(m).collision_func.is_some());
                        assert!(battle::get(m).postprocess_func.is_none());
                        assert!(battle::get(m).death_func.is_none());
                        assert!(battle::get(m).parent == uqm_engine::battle::Parent::None);
                        assert!(battle::get(m).preprocess_func.is_some());

                        // initialize_missile at speed DISPLAY_TO_WORLD (24), then the
                        // ship's own motion is added to the missile.
                        let angle = facing_to_angle(i32::from(face));
                        let vx = cosine(angle, 96 << VELOCITY_SHIFT);
                        let vy = sine(angle, 96 << VELOCITY_SHIFT);
                        let mut expected: VelocityDesc = VelocityDesc::new();
                        (expected).set_components(i32::from(vx), i32::from(vy));
                        (expected).delta_components(i32::from(dx), i32::from(dy));
                        assert_eq!(
                            velocity_fields(&battle::get(m).velocity),
                            velocity_fields(&expected),
                            "facing {facing}, missile {i}"
                        );
                        // PKUNK_OFFSET (15 display pixels), minus the launch velocity
                        // and the ship velocity, both in world units.
                        let x = 321 + i32::from(cosine(angle, 60))
                            - (i32::from(vx) >> VELOCITY_SHIFT)
                            - (i32::from(dx) >> VELOCITY_SHIFT);
                        let y = -654 + i32::from(sine(angle, 60))
                            - (i32::from(vy) >> VELOCITY_SHIFT)
                            - (i32::from(dy) >> VELOCITY_SHIFT);
                        assert_eq!(
                            (
                                battle::get(m).current.location.x,
                                battle::get(m).current.location.y
                            ),
                            (x as i16, y as i16),
                            "facing {facing}, missile {i}"
                        );
                        battle_context::dispose(h.unwrap());
                    }
                }
            }
        }
    })
}

#[test]
fn bug_missile_allocation_failure_leaves_null_slots_but_still_reports_three() {
    battle_context::run(|_| {
        // The ship takes one slot, so exactly one missile fits.
        let _battle = Battle::with_capacity(2);
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.inner.ship_mut().ship_facing = 6;
            let mut out = [Some(ship.id); 3];
            let count = ship.fire(&mut out);
            assert_eq!(count, 3);
            assert!(out[0].is_some());
            assert!(out[1].is_none());
            assert!(out[2].is_none());
            assert_eq!(
                (battle::get(out[0].unwrap()).current.image.frame.raw()).index(),
                0,
                "the first missile is the forward one"
            );
            battle_context::dispose(out[0].unwrap());
        }
    })
}

#[test]
fn bug_missiles_animate_one_frame_per_blast_offset_delay() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            let mut out = [None; 3];
            ship.fire(&mut out);
            let m = out[0].unwrap();
            let animate = battle::get(m).preprocess_func.unwrap();
            battle::get_mut(m).current.image.frame = ship.weapon_frames.selection(2);
            battle::get_mut(m).state_flags &= !(CHANGING as u16);
            assert_eq!(battle::get(m).turn, 0);
            hooks::invoke_element(animate, m);
            assert_eq!((battle::get(m).next.image.frame.raw()).index(), 3);
            assert_ne!(battle::get(m).state_flags & CHANGING as u16, 0);
            // next_turn shares its byte with the missile's blast offset (1).
            assert_eq!(battle::get(m).turn, 1);

            battle::get_mut(m).state_flags &= !(CHANGING as u16);
            battle::get_mut(m).next.image.frame = ship.weapon_frames.selection(6);
            hooks::invoke_element(animate, m);
            assert_eq!(battle::get(m).turn, 0);
            assert_eq!(battle::get(m).state_flags & CHANGING as u16, 0);
            assert_eq!((battle::get(m).next.image.frame.raw()).index(), 6);

            battle::get_mut(m).current.image.frame = ship.weapon_frames.selection(7);
            hooks::invoke_element(animate, m);
            assert_eq!((battle::get(m).next.image.frame.raw()).index(), 0);
            assert_ne!(battle::get(m).state_flags & CHANGING as u16, 0);
            assert_eq!(battle::get(m).turn, 1);
            for h in out {
                battle_context::dispose(h.unwrap());
            }
        }
    })
}

#[test]
fn first_preprocess_with_an_even_draw_spawns_no_phoenix() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            let start_status = (LEFT | THRUST | WEAPON | LOW_ON_ENERGY as u32) as u16;
            ship.appearing();
            ship.inner.ship_mut().cur_status_flags = start_status;
            with_rng(even_seed(), 1, || ship.preprocess());
            assert_eq!(queued(), vec![ship.id]);
            assert!(ship.data().phoenix.is_none());
            // A brand-new ship is preprocessed only once.
            assert!(ship.inner.descriptor_mut().callbacks()[1] == 0);
            assert_eq!(ship.inner.ship_mut().cur_status_flags, start_status);
            assert_ne!(battle::get(ship.id).state_flags & APPEARING as u16, 0);
        }
    })
}

#[test]
fn first_preprocess_with_an_odd_draw_queues_a_phoenix_at_the_head() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 1);
            let start_status = (LEFT | THRUST | WEAPON | LOW_ON_ENERGY as u32) as u16;
            ship.appearing();
            ship.inner.ship_mut().cur_status_flags = start_status;
            with_rng(odd_seed(), 1, || ship.preprocess());
            let all = queued();
            assert_eq!(all.len(), 2);
            let phoenix = all[0];
            assert_eq!(all[1], ship.id);
            assert_eq!(ship.data().phoenix, Some(all[0]));
            assert_eq!(battle::get(phoenix).player_nr, 1);
            assert_eq!(
                battle::get(phoenix).state_flags,
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16
            );
            assert_eq!(battle::get(phoenix).life, 1);
            assert!(battle::get(phoenix).death_func.is_some());
            assert!(battle::get(phoenix).preprocess_func.is_none());
            assert!(battle::get(phoenix).postprocess_func.is_none());
            assert_eq!(
                battle::get(phoenix).parent,
                uqm_engine::battle::Parent::Ship(ship.inner.ship_id)
            );
            assert!(ship.inner.descriptor_mut().callbacks()[1] == 0);
            assert_eq!(ship.inner.ship_mut().cur_status_flags, start_status);
            // hTarget stays clear: this is not a reincarnation.
            assert!(battle::get(ship.id).target.is_none());
            assert_eq!(
                battle::get(ship.id).state_flags,
                (PLAYER_SHIP | APPEARING) as u16,
                "no phoenix transition on a new ship"
            );
        }
    })
}

#[test]
fn phoenix_allocation_failure_still_consumes_the_draw_and_records_no_phoenix() {
    battle_context::run(|_| {
        let _battle = Battle::with_capacity(1);
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.appearing();
            let id = ship.inner.id;
            if let uqm_engine::battle::RaceState::Pkunk { phoenix, .. } =
                &mut ship.inner.descriptor_mut().race_state
            {
                *phoenix = Some(id);
            } else {
                panic!("Pkunk state");
            }
            with_rng(odd_seed(), 1, || ship.preprocess());
            assert_eq!(queued(), vec![ship.id]);
            assert!(ship.data().phoenix.is_none());
            assert!(ship.inner.descriptor_mut().callbacks()[1] == 0);
        }
    })
}

#[test]
fn reincarnation_preprocess_queues_four_trails_around_the_ship() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            let seed = odd_seed();
            let cases = [
                (0u16, 4000i16, 4000i16),
                (5, 100, 7600),
                (15, 8000, 50),
                (9, 0, 0),
                (12, 8191, 7679),
            ];
            for (facing, x, y) in cases {
                ship.reincarnating(x, y, facing);
                ship.inner.ship_mut().cur_status_flags = 0x3ff;
                battle::get_mut(ship.id).velocity = VelocityDesc::new();
                with_rng(seed, 1, || ship.preprocess());

                let all = queued();
                assert_eq!(all.len(), 6, "phoenix, ship and four trails");
                assert_eq!(all[1], ship.id);
                assert_eq!(ship.data().phoenix, Some(all[0]));
                assert_eq!(
                    battle::get(all[0]).state_flags,
                    (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16
                );

                let el = ship.id;
                assert_eq!(battle::get(el).life, 12);
                assert_eq!(battle::prim(el).kind, EMPTY_PRIMITIVE);
                assert_eq!(
                    battle::get(el).state_flags,
                    (PLAYER_SHIP | APPEARING | NONSOLID | FINITE_LIFE | CHANGING) as u16
                );
                assert_eq!(
                    ship.inner.ship_mut().ship_facing,
                    facing,
                    "facing is restored"
                );
                // Still reincarnating: the one-time callback stays and only the
                // input bits are cleared.
                assert!(ship.inner.descriptor_mut().callbacks()[1] != 0);
                assert_eq!(
                    ship.inner.ship_mut().cur_status_flags,
                    0x3ff & !((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16)
                );
                assert_eq!(ship.data().phoenix, Some(all[0]));

                for (k, &h) in all[2..].iter().enumerate() {
                    let trail_facing =
                        normalize_facing(i32::from(facing) + angle_to_facing(8 + 16 * k as i32));
                    let angle = facing_to_angle(i32::from(trail_facing)) as u16;
                    let expected = trail_origin(Point { x, y }, trail_facing, 12);
                    check_trail(&mut ship, h, expected, angle);
                }
                ship.drain();
            }
        }
    })
}

#[test]
fn reincarnation_trails_are_limited_by_free_elements() {
    battle_context::run(|_| {
        // Ship + two trails; the phoenix draw is even so none is queued.
        let _battle = Battle::with_capacity(3);
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.reincarnating(4000, 4000, 3);
            with_rng(even_seed(), 1, || ship.preprocess());
            let all = queued();
            assert_eq!(all.len(), 3);
            for (k, &h) in all[1..].iter().enumerate() {
                let trail_facing = normalize_facing(3 + angle_to_facing(8 + 16 * k as i32));
                let angle = facing_to_angle(i32::from(trail_facing)) as u16;
                let expected = trail_origin(Point { x: 4000, y: 4000 }, trail_facing, 12);
                check_trail(&mut ship, h, expected, angle);
            }
            // A trail that cannot allocate keeps its callback to retry next frame.
            let first_trail = all[1];
            let callback = addr(battle::get(first_trail).preprocess_func);
            hooks::invoke_element(
                battle::get(first_trail).preprocess_func.unwrap(),
                first_trail,
            );
            assert_eq!(queued(), all);
            assert_eq!(addr(battle::get(first_trail).preprocess_func), callback);
        }
    })
}

#[test]
fn reincarnation_ends_when_life_span_returns_to_normal() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.reincarnating(4000, 4000, 2);
            with_rng(even_seed(), 1, || ship.preprocess());
            ship.drain();
            let el = ship.id;

            // While the animation is running only the input bits are cleared.
            battle::get_mut(el).state_flags &= !(APPEARING as u16);
            battle::get_mut(el).life = 5;
            ship.inner.ship_mut().cur_status_flags = 0x3ff;
            ship.preprocess();
            assert_eq!(ship.inner.ship_mut().cur_status_flags, 0x3e0);
            assert!(ship.inner.descriptor_mut().callbacks()[1] != 0);
            assert_eq!(battle::prim(el).kind, EMPTY_PRIMITIVE);

            // At NORMAL_LIFE the ship becomes solid again.
            battle::get_mut(el).life = NORMAL_LIFE as u16;
            battle::get_mut(el).current.image.frame = ship.ship_frames.selection(5);
            battle::get_mut(el).next.image.frame = ship.ship_frames.selection(9);
            battle::get_mut(el).next.image.frames = ship.inner.frame_set(0);
            battle::get_mut(el).current.location = Point { x: 400, y: -800 };
            battle::get_mut(el).next.location = Point { x: 1201, y: 77 };
            battle::get_mut(el).velocity = VelocityDesc::new();
            uqm_modes::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(el).velocity,
                9,
                6,
            );
            ship.inner.ship_mut().cur_status_flags = 0x3ff;
            ship.preprocess();
            assert_eq!((battle::get(el).current.image.frame.raw()).index(), 5);
            assert_eq!((battle::get(el).next.image.frame.raw()).index(), 5);
            assert_eq!(battle::prim(el).kind, STAMP_PRIMITIVE);
            let control = battle::get(el).intersect_control;
            assert_eq!(
                (
                    control.intersect_stamp.origin.x,
                    control.intersect_stamp.origin.y
                ),
                (100, -200)
            );
            assert_eq!((control.end_point.x, control.end_point.y), (300, 19));
            // next.image.frame was overwritten before InitIntersectFrame runs.
            assert_eq!((control.intersect_stamp.frame).index(), 5);
            assert_eq!(velocity_fields(&battle::get(el).velocity), [0; 9]);
            assert_eq!(
                battle::get(el).state_flags & (NONSOLID | FINITE_LIFE) as u16,
                0,
                "solid and no longer finite"
            );
            assert_ne!(battle::get(el).state_flags & CHANGING as u16, 0);
            assert!(ship.inner.descriptor_mut().callbacks()[1] == 0);
            assert_eq!(ship.inner.ship_mut().cur_status_flags, 0x3e0);
        }
    })
}

#[test]
fn trails_hop_forward_until_the_ship_is_solid_again() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.reincarnating(4000, 4000, 0);
            with_rng(even_seed(), 1, || ship.preprocess());
            let first = queued()[1];
            let mut parent = first;
            let mut from = battle::get(first).current.location;
            let angle = i32::from(battle::get(first).mass);
            for hop in 0..3 {
                let before = queued().len();
                let step = battle::get(parent).preprocess_func.unwrap();
                hooks::invoke_element(step, parent);
                // The old trail stops hopping; one new trail takes its place.
                assert!(battle::get(parent).preprocess_func.is_none(), "hop {hop}");
                assert_eq!(queued().len(), before + 1);
                let child = *queued().last().unwrap();
                let to = Point {
                    x: (i32::from(from.x) + i32::from(cosine(angle, 80))) as i16,
                    y: (i32::from(from.y) + i32::from(sine(angle, 80))) as i16,
                };
                check_trail(&mut ship, child, to, angle as u16);
                parent = child;
                from = to;
            }

            // Once the ship is solid the pending trail simply stops.
            battle::get_mut(ship.id).state_flags &= !(NONSOLID as u16);
            let before = queued();
            hooks::invoke_element(battle::get(parent).preprocess_func.unwrap(), parent);
            assert!(battle::get(parent).preprocess_func.is_none());
            assert_eq!(queued(), before);
        }
    })
}

#[test]
fn trail_death_steps_through_the_twelve_ember_colors() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.reincarnating(4000, 4000, 0);
            with_rng(even_seed(), 1, || ship.preprocess());
            let trail = queued()[1];
            let death = battle::get(trail).death_func.unwrap();
            // BUILD_COLOR (MAKE_RGB15 (r, g, b), _) for the C table, as bytes.
            let table: [[u8; 3]; 12] = [
                [0xFF, 0xAD, 0x00],
                [0xFF, 0x8C, 0x00],
                [0xFF, 0x73, 0x00],
                [0xFF, 0x52, 0x00],
                [0xFF, 0x39, 0x00],
                [0xFF, 0x18, 0x00],
                [0xFF, 0x00, 0x00],
                [0xDE, 0x00, 0x00],
                [0xBD, 0x00, 0x00],
                [0x9C, 0x00, 0x00],
                [0x7B, 0x00, 0x00],
                [0x5A, 0x00, 0x00],
            ];
            for (index, rgb) in table.iter().enumerate().skip(1) {
                battle::get_mut(trail).state_flags |= DISAPPEARING as u16;
                battle::get_mut(trail).state_flags &= !(CHANGING as u16);
                battle::get_mut(trail).life = 0;
                hooks::invoke_element(death, trail);
                assert_eq!(battle::get(trail).color_cycle_index as usize, index);
                assert_eq!(battle::get(trail).life, 1);
                assert_eq!(
                    color_bytes(battle::prim(trail).color),
                    [rgb[0], rgb[1], rgb[2], 0xFF]
                );
                assert_eq!(battle::get(trail).state_flags & DISAPPEARING as u16, 0);
                assert_ne!(battle::get(trail).state_flags & CHANGING as u16, 0);
            }

            // The twelfth step runs off the table: the trail is allowed to vanish.
            let last_color = color_bytes(battle::prim(trail).color);
            battle::get_mut(trail).state_flags |= DISAPPEARING as u16;
            battle::get_mut(trail).state_flags &= !(CHANGING as u16);
            battle::get_mut(trail).life = 0;
            hooks::invoke_element(death, trail);
            assert_eq!(battle::get(trail).color_cycle_index, 12);
            assert_eq!(battle::get(trail).life, 0);
            assert_ne!(battle::get(trail).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(battle::get(trail).state_flags & CHANGING as u16, 0);
            assert_eq!(color_bytes(battle::prim(trail).color), last_color);
        }
    })
}

#[test]
fn phoenix_death_hook_keeps_a_living_ship_and_arms_reincarnation_for_a_dead_one() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            let el = ship.id;
            battle::get_mut(el).preprocess_func =
                Some(uqm_engine::battle::ElementHook::Native(engine_preprocess));
            battle::get_mut(el).postprocess_func =
                Some(uqm_engine::battle::ElementHook::Native(engine_postprocess));
            battle::get_mut(el).death_func =
                Some(uqm_engine::battle::ElementHook::Native(engine_death));
            ship.appearing();
            with_rng(odd_seed(), 1, || ship.preprocess());
            let phoenix = ship.data().phoenix.unwrap();
            let hook = battle::get(phoenix).death_func.unwrap();

            // Crew left: the phoenix survives another frame and nothing else moves.
            battle::get_mut(phoenix).state_flags |= DISAPPEARING as u16;
            battle::get_mut(phoenix).life = 0;
            hooks::invoke_element(hook, phoenix);
            assert_eq!(battle::get(phoenix).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(battle::get(phoenix).life, 1);
            assert_eq!(battle::get(el).mass, 1);
            assert_eq!(
                addr(battle::get(el).death_func),
                addr(Some(uqm_engine::battle::ElementHook::Native(engine_death)))
            );
            assert!(nothing_saved(&mut ship));

            // MAX_SHIP_MASS * 10 is the warp-out marker: mass 100 is 101 here.
            ship.inner.descriptor_mut().ship_info.crew_level = 0;
            for warping in [100u8, 101, 255] {
                battle::get_mut(el).mass = warping;
                hooks::invoke_element(hook, phoenix);
                assert_eq!(battle::get(el).mass, warping);
                assert_eq!(
                    addr(battle::get(el).death_func),
                    addr(Some(uqm_engine::battle::ElementHook::Native(engine_death)))
                );
                assert!(nothing_saved(&mut ship));
            }

            // Anything else is a death Pkunk walks away from.
            for mass in [0u8, 1, 10, 99] {
                battle::get_mut(el).mass = mass;
                battle::get_mut(el).preprocess_func =
                    Some(uqm_engine::battle::ElementHook::Native(engine_preprocess));
                battle::get_mut(el).death_func =
                    Some(uqm_engine::battle::ElementHook::Native(engine_death));
                if let uqm_engine::battle::RaceState::Pkunk { saved_death, .. } =
                    &mut ship.inner.descriptor_mut().race_state
                {
                    *saved_death = None;
                } else {
                    panic!("Pkunk state");
                }
                hooks::invoke_element(hook, phoenix);
                assert_eq!(battle::get(el).mass, 11, "mass {mass}");
                let data = ship.data();
                assert_eq!(
                    addr(data.saved_preprocess),
                    addr(Some(uqm_engine::battle::ElementHook::Native(
                        engine_preprocess
                    )))
                );
                assert_eq!(
                    addr(data.saved_postprocess),
                    addr(Some(uqm_engine::battle::ElementHook::Native(
                        engine_postprocess
                    )))
                );
                assert_eq!(
                    addr(data.saved_death),
                    addr(Some(uqm_engine::battle::ElementHook::Native(engine_death)))
                );
                assert!(battle::get(el).death_func.is_some());
                assert_ne!(
                    addr(battle::get(el).death_func),
                    addr(Some(uqm_engine::battle::ElementHook::Native(engine_death)))
                );
            }
        }
    })
}

#[test]
fn reincarnation_death_explodes_then_new_pkunk_restores_a_fresh_ship() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            let el = ship.id;
            let pkunk_preprocess = ship
                .inner
                .descriptor_mut()
                .race
                .unwrap()
                .preprocess
                .unwrap();
            battle::get_mut(el).preprocess_func =
                Some(uqm_engine::battle::ElementHook::Native(engine_preprocess));
            battle::get_mut(el).postprocess_func =
                Some(uqm_engine::battle::ElementHook::Native(engine_postprocess));
            battle::get_mut(el).death_func =
                Some(uqm_engine::battle::ElementHook::Native(engine_death));
            ship.at(2000, 3000);

            // The brand-new ship spawns its phoenix, which later sees the death.
            ship.appearing();
            with_rng(odd_seed(), 1, || ship.preprocess());
            let phoenix = ship.data().phoenix.unwrap();
            ship.inner.descriptor_mut().ship_info.crew_level = 0;
            hooks::invoke_element(battle::get(phoenix).death_func.unwrap(), phoenix);
            assert_eq!(battle::get(el).mass, 11);
            let reincarnation = battle::get(el).death_func.unwrap();

            // The armed death function explodes the ship (without ship_death).
            battle::get_mut(el).target = Some(ship.id);
            battle::get_mut(el).state_flags |= DISAPPEARING as u16;
            uqm_modes::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(el).velocity,
                20,
                3,
            );
            ship.inner.descriptor_mut().ship_info.energy_level = 9;
            hooks::invoke_element(reincarnation, el);
            assert_eq!(battle::get(el).life, 36);
            assert_eq!(battle::get(el).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(
                battle::get(el).state_flags & (FINITE_LIFE | NONSOLID) as u16,
                (FINITE_LIFE | NONSOLID) as u16
            );
            assert!(battle::get(el).target.is_none());
            assert_eq!(velocity_fields(&battle::get(el).velocity), [0; 9]);
            assert_eq!(ship.inner.descriptor_mut().ship_info.energy_level, 0);
            assert_ne!(
                addr(battle::get(el).preprocess_func),
                addr(Some(uqm_engine::battle::ElementHook::Native(
                    engine_preprocess
                )))
            );
            assert_ne!(
                addr(battle::get(el).postprocess_func),
                addr(Some(uqm_engine::battle::ElementHook::Native(
                    engine_postprocess
                )))
            );
            assert_ne!(
                addr(battle::get(el).death_func),
                addr(Some(uqm_engine::battle::ElementHook::Native(engine_death)))
            );
            assert_ne!(addr(battle::get(el).death_func), addr(Some(reincarnation)));

            // When the explosion ends the engine runs the new death function.
            // Leave the ship in a state a Vux-slowed, long-dead Pkunk would have.
            ship.inner.descriptor_mut().race_callbacks.pre =
                uqm_engine::battle::Selection::Disabled;
            ship.inner.descriptor_mut().ship_info.crew_level = 0;
            ship.inner.descriptor_mut().ship_info.energy_level = 0;
            {
                let c = &mut ship.inner.descriptor_mut().characteristics;
                (
                    c.max_thrust,
                    c.thrust_increment,
                    c.turn_wait,
                    c.thrust_wait,
                    c.special_wait,
                ) = (1, 1, 9, 9, 9);
            }
            ship.inner.ship_mut().ship_input_state = 0xff;
            ship.inner.ship_mut().cur_status_flags = 0xffff;
            ship.inner.ship_mut().old_status_flags = 0xffff;
            ship.inner.ship_mut().energy_counter = 9;
            ship.inner.ship_mut().weapon_counter = 9;
            ship.inner.ship_mut().special_counter = 9;
            ship.inner.ship_mut().ship_facing = 3;
            battle::get_mut(el).hits = 7;
            battle::get_mut(el).turn = 9;
            battle::get_mut(el).thrust = 9;
            battle::get_mut(el).life = 3;
            battle::get_mut(el).mass = 11;
            battle::get_mut(el).target = None;
            battle::prim_mut(el).kind = EMPTY_PRIMITIVE;

            let seed = 777;
            let draws = rolls(seed, 7);
            let (x, after_x) = expanded_coord(&draws, 1, SPACE_W);
            let (y, after_y) = expanded_coord(&draws, after_x, SPACE_H);
            assert_eq!((after_x, after_y), (4, 7), "three draws per coordinate");
            let facing = (draws[0] & 15) as u16;
            with_rng(seed, 7, || {
                hooks::invoke_element(battle::get(el).death_func.unwrap(), el)
            });

            assert_eq!(
                battle::get(el).state_flags,
                (APPEARING | PLAYER_SHIP | IGNORE_SIMILAR) as u16
            );
            assert_eq!(battle::get(el).mass, 1);
            assert_eq!(
                addr(battle::get(el).preprocess_func),
                addr(Some(uqm_engine::battle::ElementHook::Native(
                    engine_preprocess
                )))
            );
            assert_eq!(
                addr(battle::get(el).postprocess_func),
                addr(Some(uqm_engine::battle::ElementHook::Native(
                    engine_postprocess
                )))
            );
            assert_eq!(
                addr(battle::get(el).death_func),
                addr(Some(uqm_engine::battle::ElementHook::Native(engine_death)))
            );
            assert_eq!(
                Some(ship.inner.descriptor_mut().callbacks()[1]),
                Some(pkunk_preprocess as usize),
                "the one-time callback is armed again"
            );
            {
                let descriptor = ship.inner.descriptor_mut();
                assert_eq!(
                    (
                        descriptor.ship_info.crew_level,
                        descriptor.ship_info.energy_level
                    ),
                    (8, 12)
                );
            }
            let c = ship.inner.descriptor_mut().characteristics;
            assert_eq!(
                (
                    c.max_thrust,
                    c.thrust_increment,
                    c.turn_wait,
                    c.thrust_wait,
                    c.special_wait
                ),
                (64, 16, 0, 0, 0)
            );
            assert_eq!(ship.inner.ship_mut().ship_input_state, 0);
            assert_eq!(ship.inner.ship_mut().cur_status_flags, PLAY_VICTORY_DITTY);
            assert_eq!(ship.inner.ship_mut().old_status_flags, 0);
            {
                let record = ship.inner.ship_mut();
                assert_eq!(
                    (
                        record.energy_counter,
                        record.weapon_counter,
                        record.special_counter
                    ),
                    (0, 0, 0)
                );
            }
            assert_eq!(battle::get(el).hits, 0);
            assert_eq!(battle::get(el).turn, 0);
            assert_eq!(battle::get(el).thrust, 0);
            assert_eq!(battle::get(el).life, 1);
            assert_eq!(ship.inner.ship_mut().ship_facing, facing);
            assert_eq!(
                battle::get(el).current.image.frames,
                ship.inner.frame_set(0)
            );
            assert_eq!((battle::get(el).current.image.frame.raw()).index(), facing);
            assert_eq!(battle::prim(el).kind, STAMP_PRIMITIVE);
            assert_eq!(
                (
                    battle::get(el).current.location.x,
                    battle::get(el).current.location.y
                ),
                (x, y)
            );
            assert_eq!(
                battle::get(el).target,
                Some(ship.id),
                "marks the next preprocess as a reincarnation"
            );
        }
    })
}

#[test]
fn new_pkunk_redraws_its_position_while_a_planet_pulls_on_it() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            // A seed whose second candidate position is clear of the first.
            let far = |a: i32, b: i32, extent: i32| {
                let mut d = b - a;
                if d > extent / 2 {
                    d -= extent;
                } else if d < -extent / 2 {
                    d += extent;
                }
                (d.abs() >> 2) > 300
            };
            let (seed, draws) = (1..5000u32)
                .map(|seed| (seed, rolls(seed, 13)))
                .find(|(_, r)| {
                    let (x1, n) = expanded_coord(r, 1, SPACE_W);
                    let (y1, n) = expanded_coord(r, n, SPACE_H);
                    let (x2, n) = expanded_coord(r, n, SPACE_W);
                    let (y2, _) = expanded_coord(r, n, SPACE_H);
                    far(i32::from(x1), i32::from(x2), SPACE_W)
                        || far(i32::from(y1), i32::from(y2), SPACE_H)
                })
                .expect("a seed whose retry lands clear of the planet");
            let (x1, n) = expanded_coord(&draws, 1, SPACE_W);
            let (y1, n) = expanded_coord(&draws, n, SPACE_H);
            let (x2, n) = expanded_coord(&draws, n, SPACE_W);
            let (y2, n) = expanded_coord(&draws, n, SPACE_H);
            assert_eq!(
                n, 13,
                "one facing draw, then three draws per coordinate, twice"
            );

            let mut ship = Ship::new(&_battle, 0);
            let new_pkunk = ship.armed_new_pkunk();
            // GRAVITY_MASS (mass + 1): a planet sitting on the first candidate.
            let planet_owner = battle::Object::new(&_battle.inner);
            let planet = planet_owner.id;
            let planet_handle = planet;
            battle::get_mut(planet).state_flags = 0;
            battle::get_mut(planet).mass = 200;
            battle::get_mut(planet).current.location = Point { x: x1, y: y1 };
            battle::get_mut(planet).next = battle::get(planet).current.clone();
            battle::get_mut(planet).current.image.frames = ship.inner.frame_set(0);
            battle::get_mut(planet).current.image.frame = ship.ship_frames.selection(0);
            battle::prim_mut(planet).kind = STAMP_PRIMITIVE;
            battle::put(planet_handle);

            let el = ship.id;
            with_rng(seed, 13, || hooks::invoke_element(new_pkunk, el));
            assert_eq!(
                (
                    battle::get(el).current.location.x,
                    battle::get(el).current.location.y
                ),
                (x2, y2),
                "the candidate on top of the planet is rejected"
            );
            assert_eq!(ship.inner.ship_mut().ship_facing, (draws[0] & 15) as u16);
        }
    })
}

#[test]
fn taunt_waits_out_its_cooldown_and_needs_the_key_and_missing_energy() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.inner.descriptor_mut().install_sound_fixture(4);

            ship.inner.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.inner.descriptor_mut().ship_info.energy_level = 5;

            observation::enable();
            // The cooldown only counts down; no energy, no draw.
            ship.inner.descriptor_mut().characteristics.special_wait = 3;
            with_rng(1, 0, || ship.inner.postprocess());
            assert_eq!(ship.inner.descriptor_mut().characteristics.special_wait, 2);
            assert_eq!(ship.inner.descriptor_mut().ship_info.energy_level, 5);
            assert_eq!(special_hooks::count(&observation::drain(), "pkunk"), 0);

            // No key, or already full: nothing happens, and no draw is spent.
            ship.inner.descriptor_mut().characteristics.special_wait = 0;
            ship.inner.ship_mut().cur_status_flags = (LEFT | WEAPON) as u16;
            with_rng(1, 0, || ship.inner.postprocess());
            ship.inner.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.inner.descriptor_mut().ship_info.energy_level = 12;
            with_rng(1, 0, || ship.inner.postprocess());
            assert_eq!(ship.inner.descriptor_mut().characteristics.special_wait, 0);
            assert_eq!(ship.inner.descriptor_mut().ship_info.energy_level, 12);
            assert_eq!(special_hooks::count(&observation::drain(), "pkunk"), 0);

            // Taunting buys two energy (capped), arms the 16-frame cooldown and
            // goes through DeltaEnergy's bookkeeping.
            seed_random(31);
            for (before, after) in [(5u8, 7u8), (11, 12), (0, 2)] {
                ship.inner.descriptor_mut().characteristics.special_wait = 0;
                ship.inner.descriptor_mut().ship_info.energy_level = before;
                ship.inner.ship_mut().energy_counter = 9;
                ship.inner.ship_mut().cur_status_flags = SPECIAL as u16 | LOW_ON_ENERGY;
                ship.inner.ship_mut().special_counter = 77;
                ship.inner.postprocess();
                assert_eq!(special_hooks::count(&observation::drain(), "pkunk"), 1);
                assert_eq!(ship.inner.descriptor_mut().ship_info.energy_level, after);
                assert_eq!(ship.inner.descriptor_mut().characteristics.special_wait, 16);
                // DeltaEnergy installs the ship's energy_wait (zero for Pkunk).
                assert_eq!(ship.inner.ship_mut().energy_counter, 0);
                assert_eq!(ship.inner.ship_mut().cur_status_flags & LOW_ON_ENERGY, 0);
                assert_eq!(ship.inner.ship_mut().special_counter, 77);
            }
        }
    })
}

#[test]
fn taunt_sound_draws_avoid_the_previous_sound_and_init_forgets_it() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.inner.descriptor_mut().install_sound_fixture(4);

            let taunt = |ship: &mut Ship| {
                ship.inner.descriptor_mut().characteristics.special_wait = 0;
                ship.inner.descriptor_mut().ship_info.energy_level = 0;
                ship.inner.ship_mut().cur_status_flags = SPECIAL as u16;
                ship.inner.postprocess();
            };
            // Sounds 0 and 1 are reserved: with four sounds the pick is 2 or 3.
            let pick = |roll: u32| 2 + i32::from(roll as u16) % 2;

            // Over several taunts the redraws follow the remembered last sound.
            let calls = 8;
            let replay = |seed: u32| {
                seed_random(seed);
                let (mut last, mut draws) = (0, 0usize);
                for _ in 0..calls {
                    loop {
                        let next = pick(random());
                        draws += 1;
                        if next != last {
                            last = next;
                            break;
                        }
                    }
                }
                draws
            };
            let seed = (1..500u32)
                .find(|&s| replay(s) > calls)
                .expect("a seed that repeats a sound at least once");
            let draws = replay(seed);
            seed_random(seed);
            for _ in 0..calls {
                taunt(&mut ship);
            }
            assert_eq!(random(), rolls(seed, draws + 1)[draws]);

            // Whatever sound came first is "last" for the next taunt, so the same
            // opening draw is rejected until a different sound turns up...
            let opening = rolls(seed, 16);
            let first = pick(opening[0]);
            let redraws = opening.iter().position(|&r| pick(r) != first).unwrap() + 1;
            assert!(redraws > 1);
            with_rng(seed, 1, || {
                // ...which a fresh init guarantees by forgetting the last sound.
                let mut fresh = Ship::new(&_battle, 1);
                fresh.inner.uninit();
                taunt(&mut ship);
            });
            with_rng(seed, redraws, || taunt(&mut ship));
        }
    })
}

#[test]
fn ai_drops_the_phoenix_only_for_standard_rated_pilots() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);
            ship.appearing();
            with_rng(odd_seed(), 1, || ship.preprocess());
            let phoenix = ship.data().phoenix;

            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            ship.inner.descriptor_mut().ship_info.energy_level = 12;

            for control in [0u8, 1 << 1, GOOD_RATING, 1 << 6] {
                ship.inner.ship_mut().control = control;
                with_rng(3, 0, || {
                    ship.intelligence(&mut concerns);
                });
                assert_eq!(ship.data().phoenix, phoenix, "control {control:#x}");
                assert_eq!(queued().len(), 2);
            }

            ship.inner.ship_mut().control = STANDARD_RATING | (1 << 1);
            with_rng(3, 0, || {
                ship.intelligence(&mut concerns);
            });
            assert!(ship.data().phoenix.is_none());
            assert_eq!(queued(), vec![ship.id]);
            // Nothing left to drop the second time.
            with_rng(3, 0, || {
                ship.intelligence(&mut concerns);
            });
            assert_eq!(queued(), vec![ship.id]);
        }
    })
}

#[test]
fn ai_taunts_when_short_of_energy_and_throttles_on_the_low_byte_of_one_draw() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);

            // (u8) truncates before the comparison: a draw above 255 whose low
            // byte is under 20 counts.
            let low = seed_with_first_roll(|r| r & 0xff < 20 && r > 0xff);
            let high = seed_with_first_roll(|r| r & 0xff >= 20);
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            ship.inner.ship_mut().weapon_counter = 1;
            // (energy, special_counter, seed, draws, taunts)
            let cases = [
                (0u8, 0u8, 1u32, 0usize, true),
                (11, 0, 1, 0, true),
                (11, 1, low, 1, true),
                (11, 255, low, 1, true),
                (11, 1, high, 1, false),
                (12, 0, 1, 0, false),
                (12, 1, low, 0, false),
            ];
            for (energy, counter, seed, draws, taunts) in cases {
                for start in [0u8, SPECIAL as u8] {
                    ship.inner.descriptor_mut().ship_info.energy_level = energy;
                    ship.inner.ship_mut().special_counter = counter;
                    ship.inner.ship_mut().ship_input_state = start | (LEFT | THRUST) as u8;
                    with_rng(seed, draws, || {
                        ship.intelligence(&mut concerns);
                    });
                    assert_eq!(
                        ship.inner.ship_mut().ship_input_state,
                        (LEFT | THRUST) as u8 | if taunts { SPECIAL as u8 } else { 0 },
                        "energy {energy}, counter {counter}, start {start:#x}"
                    );
                }
            }
        }
    })
}

#[test]
fn ai_finishes_with_the_shared_pass_after_deciding_on_the_taunt() {
    battle_context::run(|_| {
        let _battle = Battle::new();
        {
            let mut ship = Ship::new(&_battle, 0);

            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            ship.inner.descriptor_mut().ship_info.energy_level = 4;
            ship.inner.ship_mut().special_counter = 0;
            // ship_intelligence clears WEAPON while the weapon counter is idle.
            for (counter, weapon_kept) in [(0u8, false), (1, true)] {
                ship.inner.ship_mut().weapon_counter = counter;
                ship.inner.ship_mut().ship_input_state = WEAPON as u8;
                with_rng(9, 0, || {
                    ship.intelligence(&mut concerns);
                });
                assert_eq!(
                    ship.inner.ship_mut().ship_input_state & WEAPON as u8 != 0,
                    weapon_kept
                );
                assert_ne!(ship.inner.ship_mut().ship_input_state & SPECIAL as u8, 0);
            }
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
