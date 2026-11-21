//! Exercise Shofixti's live callbacks against the linked engine.
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::ai_constants::WEAPON;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::CHANGING;
use uqm_rust::engine::battle::flags::COLLISION;
use uqm_rust::engine::battle::flags::DISAPPEARING;
use uqm_rust::engine::battle::flags::FINITE_LIFE;
use uqm_rust::engine::battle::flags::NONSOLID;
use uqm_rust::engine::battle::flags::STAMPFILL_PRIMITIVE;
use uqm_rust::engine::battle::record::ElementRecord;
use uqm_rust::engine::battle::record::EvaluateRecord;
use uqm_rust::engine::battle::EMPTY_PRIMITIVE;
use uqm_rust::engine::input_port::frames::KEY_WEAPON;
use uqm_rust::engine::species::species_id::SPECIES_ID_NO_ID;
use uqm_rust::engine::species::species_id::SPECIES_ID_SHOFIXTI_ID;
use uqm_rust::engine::world::activity::activity_flags::CHECK_ABORT;
use uqm_rust::engine::world::activity::activity_flags::IN_BATTLE;
use uqm_rust::engine::world::activity::activity_kinds::IN_ENCOUNTER;
use uqm_rust::engine::world::activity::activity_kinds::SUPER_MELEE;
use uqm_rust::engine::world::story::native_bits::END_SHOFIXTI_RECRUITED;
use uqm_rust::engine::world::story::native_bits::SHOFIXTI_RECRUITED;
use uqm_rust::graphics::geometry::Color;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::random;
use uqm_rust::math::seed_random;
use uqm_rust::math::velocity::VELOCITY_SHIFT;
use uqm_rust::ships::races::{intruder_preprocess, marine_collision};
use uqm_rust::supermelee::native::battle_picker::live::FleetSelection;
#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "support/battle.rs"]
mod shared;

use std::{mem, ptr};
use uqm_rust::battle::tactrans_native::reset_winner;
use uqm_rust::engine::test_support::native_probe::{graphics_status, set_graphics_status};
use uqm_rust::ships::native::{cosine, facing_to_angle, sine};

struct Battle {
    inner: shared::Battle,
    globals: uqm_rust::engine::WorldData,
    graphics_flags: u8,
    counters: [u8; 2],
}
impl Battle {
    unsafe fn new(capacity: u16) -> Self {
        let globals = crate::battle_context::with(|cx| cx.world.data);
        let graphics_flags = graphics_status();
        let counters = crate::battle_context::with(|cx| cx.b.counters);
        let inner = shared::Battle::new(capacity);
        crate::battle_context::with(|cx| cx.world.set_activity(IN_ENCOUNTER as _));
        crate::battle_context::with(|cx| uqm_rust::state::ffi::rust_init_game_state(cx.world));
        set_recruited(true);
        set_graphics_status(graphics_flags & !4);
        crate::battle_context::with(|cx| cx.b.counters = [10, 10]);
        crate::battle_context::with(|cx| reset_winner(cx.kernel));
        Self {
            inner,
            globals,
            graphics_flags,
            counters,
        }
    }
}
impl Drop for Battle {
    fn drop(&mut self) {
        crate::battle_context::with(|cx| reset_winner(cx.kernel));
        crate::battle_context::with(|cx| cx.world.data = self.globals);
        set_graphics_status(self.graphics_flags);
        crate::battle_context::with(|cx| cx.b.counters = self.counters);
    }
}

unsafe fn set_recruited(value: bool) {
    crate::battle_context::with(|cx| {
        uqm_rust::state::ffi::rust_set_game_state_bits(
            cx.world,
            SHOFIXTI_RECRUITED as i32,
            END_SHOFIXTI_RECRUITED as i32,
            value as u8,
        )
    });
}

use shared::FrameSet;
use shared::PoolShip;
struct Ship<'a> {
    inner: shared::Ship<'a>,
    handle: *mut std::ffi::c_void,
    ship_frames: FrameSet,
    weapon_frames: FrameSet,
    special_frames: FrameSet,
    captain: FrameSet,
}
impl<'a> Ship<'a> {
    unsafe fn new(battle: &'a Battle, player: i16) -> Self {
        let mut inner = shared::Ship::new(&battle.inner, SPECIES_ID_SHOFIXTI_ID, player);
        let handle = inner.handle;
        let ship_frames = FrameSet::new(16);
        let weapon_frames = FrameSet::new(16);
        let desc = inner.descriptor_mut();
        desc.ship_data.ship = ship_frames.array();
        desc.ship_data.weapon = weapon_frames.array();
        let special_frames = FrameSet::new(10);
        let captain = FrameSet::new(20);
        desc.ship_data.special = special_frames.array();
        desc.ship_data.captain_control.special = captain.selection(14);
        let frames = inner.frame_set(0);
        let el = inner.element_mut();
        el.current.image.frames = frames;
        el.current.image.frame = ship_frames.selection(0);
        el.next = el.current.clone();
        el.intersect_control.intersect_stamp.frame = ship_frames.frame(0);
        el.intersect_control.end_point = Point { x: 1000, y: 1000 };
        el.intersect_control.intersect_stamp.origin = Point { x: 1000, y: 1000 };
        Self {
            inner,
            handle,
            ship_frames,
            weapon_frames,
            special_frames,
            captain,
        }
    }

    unsafe fn fire(&mut self, out: &mut [*mut std::ffi::c_void]) -> u16 {
        crate::battle_context::with(|cx| {
            let origin = cx.weapon_origin(self.inner.id);
            let weapons = cx.spawn_weapons(self.inner.ship_id, origin);
            for (slot, weapon) in out.iter_mut().zip(&weapons) {
                *slot = weapon.map_or(ptr::null_mut(), |id| cx.legacy_element(id).cast());
            }
            weapons.len() as u16
        })
    }
    fn ptr(&self) -> *mut ElementRecord {
        self.handle.cast()
    }
    unsafe fn ai(&mut self, concerns: &mut [EvaluateRecord; 5]) {
        let mut subject = self.inner.subject();
        self.inner
            .intelligence_subject(&mut subject, &mut concerns[..4], 0);
    }
    unsafe fn arm(&mut self) -> Option<uqm_rust::engine::battle::ElementHook> {
        self.inner
            .descriptor_mut()
            .ship_data
            .captain_control
            .special = self.captain.selection(16);
        self.inner.ship_mut().old_status_flags = 0;
        self.inner.ship_mut().cur_status_flags = SPECIAL as u16;
        let old = shared::tail();
        self.inner.postprocess();
        if old == shared::tail() {
            None
        } else {
            (*shared::element(shared::tail())).death_func
        }
    }
}
unsafe fn core(callback: uqm_rust::engine::battle::ElementFn, element: *mut ElementRecord) {
    let id = shared::id(element);
    crate::battle_context::with(|cx| callback(cx, id));
}

unsafe fn prim(e: *mut ElementRecord) -> *mut uqm_rust::engine::battle::DisplayPrimitive {
    // SAFETY: the caller passes a live fixture element.
    let index = unsafe { (*e).prim_index };
    crate::battle_context::with(|cx| std::ptr::from_mut(cx.display_primitive(index)))
}
fn rgba(c: Color) -> (u8, u8, u8, u8) {
    (c.r, c.g, c.b, c.a)
}
unsafe fn with_rng<T>(seed: u32, draws: usize, action: impl FnOnce() -> T) -> T {
    seed_random(seed);
    for _ in 0..draws {
        random();
    }
    let expected = random();
    seed_random(seed);
    let result = action();
    assert_eq!(random(), expected, "seed {seed}: expected {draws} draws");
    result
}
unsafe fn object<'a>(
    battle: &'a Battle,
    flags: u16,
    mass: u8,
    hp: u16,
    dx: i32,
    dy: i32,
) -> shared::Object<'a> {
    let mut object = shared::Object::new(&battle.inner);
    let e = object.ptr();
    (*e).state_flags = flags;
    (*e).player_nr = -1;
    (*e).life = 50;
    (*e).hits = hp;
    (*e).mass = mass;
    (*e).next.location = Point {
        x: (4000 + dx) as i16,
        y: (4000 + dy) as i16,
    };
    shared::put(e.cast());
    object
}

#[test]
fn complete_descriptor_and_damaged_tanaka_katana_variant_are_fresh_each_call() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(8);
        for (activity, recruited, damaged) in [
            (0, false, false),
            (0x102, false, true),
            (2, true, false),
            (2, false, true),
            (0, false, false),
        ] {
            crate::battle_context::with(|cx| cx.world.set_activity(activity));
            set_recruited(recruited);
            let mut original = shared::descriptor(SPECIES_ID_SHOFIXTI_ID);
            let p = &raw mut original;
            let d = &*p;
            assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(d.ship_info.ship_cost, 5);
            assert_eq!(
                (
                    d.ship_info.crew_level,
                    d.ship_info.max_crew,
                    d.ship_info.energy_level,
                    d.ship_info.max_energy
                ),
                (6, 6, 4, 4)
            );
            assert_eq!(d.ship_info.race_strings_rsc.unwrap(), c"ship.shofixti.text");
            assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.shofixti.icons");
            assert_eq!(
                d.ship_info.melee_icon_rsc.unwrap(),
                c"ship.shofixti.meleeicons"
            );
            assert!(
                d.ship_info.race_strings.is_none()
                    && d.ship_info.icons.is_null()
                    && d.ship_info.melee_icon.is_null()
            );
            assert_eq!(
                (d.fleet.strength, d.fleet.known_loc.x, d.fleet.known_loc.y),
                (0, 0, 0)
            );
            let c = d.characteristics;
            assert_eq!(
                (
                    c.max_thrust,
                    c.thrust_increment,
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait
                ),
                if damaged {
                    (8, 4, 4, 3, 10)
                } else {
                    (35, 5, 1, 0, 3)
                }
            );
            assert_eq!(
                (
                    c.energy_regeneration,
                    c.energy_wait,
                    c.weapon_energy_cost,
                    c.special_energy_cost,
                    c.special_wait,
                    c.ship_mass
                ),
                (1, 9, 1, 0, 0, 1)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                let scout = if damaged { "oldscout" } else { "scout" };
                assert_eq!(
                    d.ship_data.ship_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.shofixti.graphics.{scout}.{size}")
                );
                assert_eq!(
                    d.ship_data.weapon_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.shofixti.graphics.missile.{size}")
                );
                if damaged {
                    assert!(d.ship_data.special_rsc[i].is_none());
                } else {
                    assert_eq!(
                        d.ship_data.special_rsc[i].unwrap().to_str().unwrap(),
                        format!("ship.shofixti.graphics.destruct.{size}")
                    );
                }
                assert!(
                    d.ship_data.ship.view(i).is_absent()
                        && d.ship_data.weapon.view(i).is_absent()
                        && d.ship_data.special.view(i).is_absent()
                );
            }
            assert_eq!(
                d.ship_data.captain_control.captain_rsc.unwrap(),
                if damaged {
                    c"ship.shofixti.graphics.oldcaptain"
                } else {
                    c"ship.shofixti.graphics.captain"
                }
            );
            let cap = &d.ship_data.captain_control;
            assert!(
                cap.background.is_null()
                    && cap.turn.is_null()
                    && cap.thrust.is_null()
                    && cap.weapon.is_null()
                    && cap.special.is_absent()
            );
            assert_eq!(
                d.ship_data.victory_ditty_rsc.unwrap(),
                c"ship.shofixti.ditty"
            );
            assert_eq!(
                d.ship_data.ship_sounds_rsc.unwrap(),
                c"ship.shofixti.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_none());
            assert_eq!(
                (
                    d.cyborg_control.maneuverability_index,
                    d.cyborg_control.weapon_range
                ),
                (0, 960)
            );
            assert!(d.callbacks()[4] != 0 && d.callbacks()[2] != 0 && d.callbacks()[3] != 0);
            assert!(d.callbacks()[1] == 0 && d.callbacks()[0] == 0);
            (*p).characteristics.max_thrust = 999;
            (*p).ship_info.crew_level = 0;
            (*p).race_callbacks.post = uqm_rust::engine::battle::Selection::Disabled;
            let fresh = shared::descriptor(SPECIES_ID_SHOFIXTI_ID);
            assert_eq!(fresh.ship_info.crew_level, 6);
            assert_ne!(fresh.callbacks()[2], 0);
        }
    })
}

#[test]
fn darts_use_next_location_exact_facing_offset_speed_and_failure_count() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(2);
        let mut ship = Ship::new(&_battle, 1);
        for facing in [0, 4, 7, 15, u16::MAX] {
            ship.inner.ship_mut().ship_facing = facing;
            (*ship.ptr()).next.location = Point {
                x: i16::MAX - 5,
                y: i16::MIN + 5,
            };
            let mut out = ptr::null_mut();
            with_rng(31, 0, || {
                assert_eq!(ship.fire(std::slice::from_mut(&mut out)), 1)
            });
            assert!(!out.is_null());
            let dart = shared::element(out);
            assert_eq!(
                (*dart).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!(
                ((*dart).life, (*dart).hits, (*dart).mass, (*dart).thrust),
                (10, 1, 1, 1)
            );
            assert_eq!((*dart).player_nr, 1);
            assert_eq!(((*dart).current.image.frame.raw()).index(), facing % 16);
            assert_eq!(
                (*dart).current.image.frame.raw(),
                ship.weapon_frames.frame((facing % 16) as usize)
            );
            assert_eq!((*dart).current.image.frames, ship.inner.frame_set(1));
            assert!(
                (*dart).preprocess_func.is_none()
                    && (*dart).postprocess_func.is_none()
                    && (*dart).death_func.is_none()
                    && (*dart).collision_func.is_some()
            );
            assert!((*dart).parent == uqm_rust::engine::battle::Parent::None);
            let angle = facing_to_angle(facing as i32) as u16;
            let vx = cosine(angle as i32, 96 << VELOCITY_SHIFT);
            let vy = sine(angle as i32, 96 << VELOCITY_SHIFT);
            let (velocity_x, velocity_y) = ((*dart).velocity).get_current_components();
            let (x, y) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((x, y), (vx, vy));
            let origin_x = ((i32::from(i16::MAX) - 5) + i32::from(cosine(angle as i32, 60))) as i16;
            let origin_y = ((i32::from(i16::MIN) + 5) + i32::from(sine(angle as i32, 60))) as i16;
            assert_eq!(
                (*dart).current.location.x,
                origin_x.wrapping_sub(vx >> VELOCITY_SHIFT)
            );
            assert_eq!(
                (*dart).current.location.y,
                origin_y.wrapping_sub(vy >> VELOCITY_SHIFT)
            );
            let mut failure = std::ptr::dangling_mut::<std::ffi::c_void>();
            with_rng(31, 0, || {
                assert_eq!(ship.fire(std::slice::from_mut(&mut failure)), 1)
            });
            assert!(failure.is_null());
            battle_context::dispose(out);
        }
    })
}

#[test]
fn glory_ai_toggles_the_switch_and_checks_all_deterministic_threat_branches() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(8);
        let mut ship = Ship::new(&_battle, 0);
        let enemy = Ship::new(&_battle, 1);
        let mut concerns: [EvaluateRecord; 5] = mem::zeroed();
        for (remaining, enemy_turn, immediate, crew, special_available, start, counter, expected) in [
            (6, 5, false, 6, true, false, 0, true),
            (6, 5, false, 6, true, true, 0, false),
            (6, 5, false, 6, true, true, 1, true),
            (6, 5, false, 6, true, false, 1, false),
            (5, 4, false, 6, true, false, 0, true),
            (5, 5, false, 6, true, false, 0, false),
            (5, 5, true, 1, true, false, 0, true),
            (5, 5, true, 2, true, false, 0, false),
            (6, 4, true, 1, false, false, 0, false),
        ] {
            ship.inner
                .descriptor_mut()
                .ship_data
                .captain_control
                .special = ship.captain.selection(20 - remaining);
            ship.inner.descriptor_mut().ship_data.special = if special_available {
                ship.special_frames.array()
            } else {
                uqm_rust::engine::battle::frames::FrameArray::EMPTY
            };
            ship.inner.ship_mut().ship_input_state =
                WEAPON as u8 | if start { SPECIAL as u8 } else { 0 };
            ship.inner.ship_mut().special_counter = counter;
            (*ship.ptr()).hits = crew;
            (*enemy.ptr()).current.location = Point { x: 6000, y: 6000 };
            (*enemy.ptr()).mass = 0;
            concerns[0].object_ptr = enemy.ptr();
            concerns[0].which_turn = enemy_turn;
            concerns[2].object_ptr = if immediate {
                enemy.ptr()
            } else {
                ptr::null_mut()
            };
            with_rng(88, 0, || ship.ai(&mut concerns));
            assert_eq!(
                ship.inner.ship_mut().ship_input_state & WEAPON as u8,
                0,
                "shared AI runs even during special cooldown"
            );
            assert_eq!(ship.inner.ship_mut().ship_input_state & SPECIAL as u8 != 0, expected, "remaining {remaining}, turn {enemy_turn}, immediate {immediate}, crew {crew}, start {start}, counter {counter}");
        }
    })
}

#[test]
fn glory_ai_intercept_damage_and_random_parity_short_circuit_exactly() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(8);
        let mut ship = Ship::new(&_battle, 0);
        let mut threat = Ship::new(&_battle, 1);
        ship.ship_frames.make_opaque();
        threat.ship_frames.make_opaque();
        set_graphics_status(graphics_status() | 4);
        ship.inner
            .descriptor_mut()
            .ship_data
            .captain_control
            .special = ship.captain.selection(15);
        (*threat.ptr()).state_flags = 0;
        assert_ne!(
            shared::plot_intercept(threat.ptr(), ship.ptr(), 2, 0),
            0,
            "opaque overlapping frames must intercept"
        );
        let mut concerns: [EvaluateRecord; 5] = mem::zeroed();
        concerns[2].object_ptr = threat.ptr();
        for seed in 1..32 {
            seed_random(seed);
            let parity = random() & 1 != 0;
            (*threat.ptr()).mass = 6;
            ship.inner.ship_mut().ship_input_state = 0;
            with_rng(seed, 1, || ship.ai(&mut concerns));
            assert_eq!(
                ship.inner.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                parity
            );
            (*threat.ptr()).mass = 5;
            ship.inner.ship_mut().ship_input_state = 0;
            with_rng(seed, 0, || ship.ai(&mut concerns));
            assert_eq!(ship.inner.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
        (*threat.ptr()).current.location.x += 500;
        (*threat.ptr()).mass = 255;
        assert_eq!(shared::plot_intercept(threat.ptr(), ship.ptr(), 2, 0), 0);
        with_rng(55, 0, || ship.ai(&mut concerns));
        assert_eq!(ship.inner.ship_mut().ship_input_state & SPECIAL as u8, 0);
    })
}

#[test]
fn both_switch_edges_advance_and_only_the_third_remaining_frame_detonates() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(16);
        let mut ship = Ship::new(&_battle, 0);

        for (old, current, index) in [
            (0, 0, 14),
            (0, SPECIAL as u16, 15),
            (SPECIAL as u16, SPECIAL as u16, 15),
            (SPECIAL as u16, 0, 16),
            (0, SPECIAL as u16, 17),
        ] {
            ship.inner.ship_mut().old_status_flags = old;
            ship.inner.ship_mut().cur_status_flags = current;
            with_rng(11, 0, || ship.inner.postprocess());
            assert_eq!(
                (ship
                    .inner
                    .descriptor_mut()
                    .ship_data
                    .captain_control
                    .special
                    .raw())
                .index(),
                index
            );
            assert_eq!((*ship.ptr()).hits, if index == 17 { 0 } else { 6 });
        }
        let deferred = shared::element(shared::tail());
        assert_ne!(deferred, ship.ptr());
        assert_eq!(
            (*deferred).state_flags,
            (APPEARING | NONSOLID | FINITE_LIFE) as u16
        );
        assert_eq!((*deferred).life, 0);
        assert_eq!((*deferred).parent, (*ship.ptr()).parent);
        assert_eq!(
            ((*deferred).next.location.x, (*deferred).next.location.y),
            (4000, 4000)
        );
        assert!((*deferred).target.is_none() && (*deferred).death_func.is_some());
        assert_eq!(ship.inner.descriptor_mut().ship_info.crew_level, 0);
        assert_eq!((*ship.ptr()).life, 0);
        assert_ne!((*ship.ptr()).state_flags & NONSOLID as u16, 0);
        assert!((*ship.ptr()).death_func.is_some());
    })
}

#[test]
fn deferred_glory_damage_obeys_solidity_marine_identity_radius_and_gravity() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(64);
        let mut ship = Ship::new(&_battle, 0);
        let mut enemy = Ship::new(&_battle, 1);
        enemy.inner.descriptor_mut().ship_info.max_crew = 50;
        enemy.inner.descriptor_mut().ship_info.crew_level = 50;
        (*enemy.ptr()).hits = 50;
        let mut objects = Vec::new();
        for (flags, mass, dx, dy, expected) in [
            (0, 1, 0, 0, 11),
            (COLLISION, 1, 0, 0, 11),
            (NONSOLID, 1, 0, 0, 30),
            (DISAPPEARING, 1, 0, 0, 30),
            (0, 100, 0, 0, 11),
            (0, 101, 0, 0, 30),
            (0, 1, 180 * 4, 0, 29),
            (0, 1, -(180 * 4 + 3), 0, 29),
            (0, 1, 181 * 4, 0, 30),
            (0, 1, 127 * 4, -127 * 4, 29),
            (0, 1, 128 * 4, 128 * 4, 30),
        ] {
            objects.push((object(&_battle, flags as u16, mass, 30, dx, dy), expected));
        }
        let mut marine_owner = object(&_battle, (NONSOLID | DISAPPEARING) as u16, 1, 1, 0, 0);
        let marine = marine_owner.ptr();
        (*marine).preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
            intruder_preprocess,
        ));
        (*marine).collision_func = Some(uqm_rust::engine::battle::CollisionHook::Native(
            marine_collision,
        ));
        let mut preprocess_only_owner = object(&_battle, NONSOLID as u16, 1, 1, 0, 0);
        let preprocess_only = preprocess_only_owner.ptr();
        (*preprocess_only).preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
            intruder_preprocess,
        ));
        let mut collision_only_owner = object(&_battle, NONSOLID as u16, 1, 1, 0, 0);
        let collision_only = collision_only_owner.ptr();
        (*collision_only).collision_func = Some(uqm_rust::engine::battle::CollisionHook::Native(
            marine_collision,
        ));
        let mut fragile_owner = object(&_battle, 0, 1, 19, 0, 0);
        let fragile = fragile_owner.ptr();
        let kill = ship.arm().unwrap();
        let deferred = shared::element(shared::tail());
        assert_eq!((*enemy.ptr()).hits, 50, "damage must be deferred");
        with_rng(2, 0, || hooks::invoke_element_pointer(kill, deferred));
        assert_eq!((*enemy.ptr()).hits, 31);
        assert_eq!(enemy.inner.descriptor_mut().ship_info.crew_level, 31);
        for (object, hp) in &mut objects {
            let obj = object.ptr();
            assert_eq!((*obj).hits, *hp);
            assert_eq!((*obj).life, 50);
        }
        for obj in [marine, fragile] {
            assert_eq!((*obj).hits, 0);
            assert_eq!((*obj).life, 0);
        }
        for obj in [preprocess_only, collision_only] {
            assert_eq!((*obj).hits, 1);
        }
        enemy.inner.descriptor_mut().ship_info.crew_level = 1;
        (*enemy.ptr()).hits = 1;
        hooks::invoke_element_pointer(kill, deferred);
        assert_eq!((*enemy.ptr()).life, 0);
        assert_eq!(enemy.inner.descriptor_mut().ship_info.crew_level, 0);
    })
}

#[test]
fn failed_deferred_allocation_still_kills_the_ship() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(1);
        let mut ship = Ship::new(&_battle, 0);
        assert!(with_rng(4, 0, || ship.arm()).is_none());
        assert_eq!({ shared::tail() }, ship.handle);
        assert_eq!((*ship.ptr()).life, 0);
        assert_eq!((*ship.ptr()).hits, 0);
        assert_eq!(ship.inner.descriptor_mut().ship_info.crew_level, 0);
        assert!((*ship.ptr()).death_func.is_some());
    })
}

#[test]
fn glory_death_selects_the_winner_preserves_prior_winners_and_counts_the_death() {
    battle_context::run(|_| unsafe {
        for (other_crew, other_mass, prior, expected_self, ditty) in [
            (6, 1, 0, false, true),
            (0, 1, 0, true, true),
            (0, 11, 0, false, true),
            (6, 12, 0, true, true),
            (6, 1, 1, true, false),
            (6, 1, 2, false, true),
        ] {
            let _battle = Battle::new(8);
            let mut ship = Ship::new(&_battle, 0);
            let mut other = Ship::new(&_battle, 1);
            other.inner.descriptor_mut().ship_info.crew_level = other_crew;
            (*other.ptr()).mass = other_mass;
            if prior == 1 {
                crate::battle_context::with(|cx| {
                    uqm_rust::battle::tactrans_native::set_winner_ship(cx, Some(ship.inner.ship_id))
                });
            }
            if prior == 2 {
                crate::battle_context::with(|cx| {
                    uqm_rust::battle::tactrans_native::set_winner_ship(
                        cx,
                        Some(other.inner.ship_id),
                    )
                });
            }
            ship.arm();
            let death = (*ship.ptr()).death_func.unwrap();
            with_rng(13, 0, || hooks::invoke_element_pointer(death, ship.ptr()));
            let expected = if expected_self {
                &raw mut *ship.inner.ship_mut()
            } else {
                &raw mut *other.inner.ship_mut()
            };
            assert_eq!(
                crate::battle_context::with(|cx| cx.winner()),
                Some(if expected_self {
                    ship.inner.ship_id
                } else {
                    other.inner.ship_id
                })
            );
            assert_eq!((*expected).cur_status_flags & 0x200 != 0, ditty);
            assert_eq!(crate::battle_context::with(|cx| cx.b.counters[0]), 9);
            assert_eq!((*ship.ptr()).life, 36);
            assert_eq!(ship.inner.descriptor_mut().ship_info.energy_level, 0);
            assert_eq!((*ship.ptr()).velocity.travel_angle, 0);
            assert!(
                (*ship.ptr()).preprocess_func.is_some()
                    && (*ship.ptr()).postprocess_func.is_some()
                    && (*ship.ptr()).death_func.is_some()
            );
        }
    })
}

#[test]
fn glory_flash_switch_and_explosion_color_sequence_use_twelve_frame_ship_death() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(8);
        let mut ship = Ship::new(&_battle, 0);
        ship.arm();
        hooks::invoke_element_pointer((*ship.ptr()).death_func.unwrap(), ship.ptr());
        let flash = (*ship.ptr()).preprocess_func.unwrap();
        for (life, color) in [
            (36, (255, 255, 255, 255)),
            (35, (255, 255, 82, 255)),
            (34, (255, 255, 255, 255)),
        ] {
            (*ship.ptr()).life = life;
            with_rng(3, 0, || hooks::invoke_element_pointer(flash, ship.ptr()));
            assert_eq!((*prim(ship.ptr())).kind, STAMPFILL_PRIMITIVE);
            assert_eq!(rgba((*prim(ship.ptr())).color), color);
            assert_ne!((*ship.ptr()).state_flags & CHANGING as u16, 0);
        }
        (*ship.ptr()).life = 33;
        with_rng(3, 0, || hooks::invoke_element_pointer(flash, ship.ptr()));
        assert!((*ship.ptr()).preprocess_func.is_none());
        assert_eq!((*prim(ship.ptr())).kind, EMPTY_PRIMITIVE);
        let explosion = shared::element(shared::tail());
        assert_ne!(explosion, ship.ptr());
        assert_eq!(
            (*explosion).state_flags,
            (APPEARING | FINITE_LIFE | NONSOLID) as u16
        );
        assert_eq!((*explosion).player_nr, -1);
        assert_eq!((*explosion).life, 10);
        assert_eq!((*explosion).hits, 0);
        assert_eq!((*explosion).mass, 0);
        assert_eq!((*explosion).parent, (*ship.ptr()).parent);
        assert_eq!(
            (*explosion).current.image.frame.raw(),
            ship.special_frames.frame(0)
        );
        assert_eq!((*explosion).current.image.frames, ship.inner.frame_set(2));
        assert_eq!(
            (
                (*explosion).current.location.x,
                (*explosion).current.location.y
            ),
            (4000, 4000)
        );
        assert_eq!((*prim(explosion)).kind, STAMPFILL_PRIMITIVE);
        assert_eq!(rgba((*prim(explosion)).color), (255, 255, 255, 255));
        assert!((*explosion).postprocess_func.is_none() && (*explosion).death_func.is_none());
        let animate = (*explosion).preprocess_func.unwrap();
        for color in [
            (255, 255, 82, 255),
            (255, 82, 82, 255),
            (165, 82, 0, 255),
            (165, 0, 0, 255),
            (165, 0, 0, 255),
        ] {
            (*explosion).next.image.frame = ship.special_frames.selection(7);
            with_rng(3, 0, || hooks::invoke_element_pointer(animate, explosion));
            assert_eq!(
                ((*explosion).next.image.frame.raw()).index(),
                1,
                "advance current, not next frame"
            );
            assert_eq!(rgba((*prim(explosion)).color), color);
        }
        (*prim(explosion)).color = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 0,
        };
        hooks::invoke_element_pointer(animate, explosion);
        assert_eq!(
            rgba((*prim(explosion)).color),
            (255, 255, 255, 0),
            "sameColor includes alpha"
        );
    })
}

#[test]
fn failed_visual_explosion_allocation_still_hides_the_ship() {
    battle_context::run(|_| unsafe {
        let _battle = Battle::new(1);
        let mut ship = Ship::new(&_battle, 0);
        ship.arm();
        hooks::invoke_element_pointer((*ship.ptr()).death_func.unwrap(), ship.ptr());
        (*ship.ptr()).life = 33;
        hooks::invoke_element_pointer((*ship.ptr()).preprocess_func.unwrap(), ship.ptr());
        assert_eq!({ shared::tail() }, ship.handle);
        assert_eq!((*prim(ship.ptr())).kind, EMPTY_PRIMITIVE);
        assert!((*ship.ptr()).preprocess_func.is_none());
    })
}

fn sample_weapon(input: &uqm_rust::engine::input::InputSession, side: usize, pressed: bool) {
    input.set_player_key(side, KEY_WEAPON as usize, pressed);
    let sampled = input.immediate();
    input.advance(
        sampled,
        sampled,
        uqm_rust::time::counter_abi::get_time_counter(),
    );
}

#[test]
fn glory_mutual_loss_applies_both_rosters_and_selects_both_replacements() {
    battle_context::run(|input| {
        use uqm_rust::{
            battle::tactrans_native as transitions,
            pickship::ffi::get_encounter_star_ship_with_picker,
        };
        let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
        std::fs::create_dir_all(&fixture_root).unwrap();
        let directory = tempfile::tempdir_in(fixture_root).unwrap();
        std::fs::create_dir(directory.path().join("font")).unwrap();
        std::fs::write(directory.path().join("font/not-a-glyph.txt"), []).unwrap();
        // SAFETY: this serialized fixture owns its content mount and all registered drawing resources.
        unsafe {
            use uqm_rust::graphics::drawable::LoadedDrawable;
            let content =
                uqm_engine::test_support::native_session::ContentMount::new(directory.path());
            let font = uqm_rust::graphics::font::LoadedFont::load_file(c"font")
                .unwrap_or_else(|| panic!("font fixture"));
            drop(content);
            let context = uqm_rust::graphics::context::LoadedContext::create(c"melee-death-test");
            let screens = uqm_rust::engine::test_support::native_probe::DrawQueue::init();
            let picker = LoadedDrawable::create(2, 149, 55, 2).unwrap();
            let status = LoadedDrawable::create(2, 1, 1, 4).unwrap();
            let saved_graphics = crate::battle_context::with(|cx| {
                let saved = (
                    cx.kernel.off_screen_context,
                    cx.kernel.tiny_font,
                    cx.kernel.status_frame,
                );
                cx.kernel.off_screen_context = Some(context.context());
                cx.kernel.tiny_font = Some(font.font());
                cx.kernel.status_frame = status.first_frame();
                saved
            });
            for computer in [false, true] {
                for initial in [[2, 2], [1, 2], [2, 1], [1, 1]] {
                    let _battle = Battle::new(16);
                    let saved_controls = input.player_controls();
                    crate::battle_context::with(|cx| {
                        cx.b.selection_frames = Some(std::array::from_fn(|player| {
                            picker.lease().context_frame(player)
                        }));
                    });
                    crate::battle_context::with(|cx| {
                        cx.world.set_activity(SUPER_MELEE | IN_BATTLE)
                    });
                    crate::battle_context::with(|cx| cx.b.counters = initial);
                    let mut ship = Ship::new(&_battle, 0);
                    let mut other = Ship::new(&_battle, 1);
                    ship.inner.ship_mut().species_id = SPECIES_ID_SHOFIXTI_ID;
                    other.inner.ship_mut().species_id = SPECIES_ID_SHOFIXTI_ID;
                    let mut reserves = [PoolShip::new(0), PoolShip::new(1)];
                    let inputs = std::array::from_fn::<_, 2, _>(|side| {
                        uqm_rust::engine::battle::input::InputContext::new(
                            side as u16,
                            if computer && side == 1 { 2 } else { 1 },
                        )
                    });
                    let saved_inputs = crate::battle_context::with(|cx| {
                        std::array::from_fn::<_, 2, _>(|side| {
                            cx.kernel
                                .replace_player_input(side, Some(inputs[side].clone()))
                        })
                    });
                    for side in 0..2 {
                        let active = if side == 0 {
                            &raw mut *ship.inner.ship_mut()
                        } else {
                            &raw mut *other.inner.ship_mut()
                        };
                        reserves[side].species_id = SPECIES_ID_SHOFIXTI_ID;
                        reserves[side].index = 1;
                        reserves[side].player_nr = side as i16;
                        crate::battle_context::with(|cx| {
                            let mut ships = cx.kernel.ships(side);
                            let active = ships.id(active as usize).unwrap();
                            ships.put(active);
                            if initial[side] > 1 {
                                let reserve = ships.link(reserves[side].ship_id);
                                ships.put(reserve);
                            }
                        });
                    }
                    // Arm the real glory device, execute its area damage, then
                    // process both deaths within the same simulated frame.
                    let blast = ship.arm().unwrap();
                    hooks::invoke_element_pointer(blast, shared::element(shared::tail()));
                    assert_eq!(
                        (
                            ship.inner.descriptor_mut().ship_info.crew_level,
                            other.inner.descriptor_mut().ship_info.crew_level
                        ),
                        (0, 0)
                    );
                    hooks::invoke_element_pointer((*ship.ptr()).death_func.unwrap(), ship.ptr());
                    core(transitions::ship_death, other.ptr());
                    let remaining = crate::battle_context::with(|cx| cx.b.counters);
                    assert_eq!(remaining, [initial[0] - 1, initial[1] - 1]);
                    assert_eq!(
                        (
                            ship.inner.ship_mut().species_id,
                            other.inner.ship_mut().species_id
                        ),
                        (SPECIES_ID_NO_ID, SPECIES_ID_NO_ID)
                    );
                    core(transitions::cleanup_dead_ship, ship.ptr());
                    core(transitions::cleanup_dead_ship, other.ptr());
                    crate::battle_context::with(|cx| transitions::stop_ditty(cx.kernel));
                    let expected_selection = initial == [2, 2];
                    let mut selected = [false; 2];
                    for side in 0..2 {
                        let outgoing = if side == 0 { ship.ptr() } else { other.ptr() };
                        // The fixture owns its frame buffers, rather than the
                        // resource loader used by production free_ship.
                        let desc = if side == 0 {
                            ship.inner.descriptor_mut()
                        } else {
                            other.inner.descriptor_mut()
                        };
                        desc.ship_data.ship = uqm_rust::engine::battle::frames::FrameArray::EMPTY;
                        desc.ship_data.weapon = uqm_rust::engine::battle::frames::FrameArray::EMPTY;
                        desc.ship_data.special =
                            uqm_rust::engine::battle::frames::FrameArray::EMPTY;
                        crate::battle_context::with(|cx| {
                            let outgoing = cx
                                .elements()
                                .id(std::ptr::NonNull::new(outgoing.cast()))
                                .unwrap();
                            transitions::new_ship_with_picker(cx, outgoing, |cx, last, player| {
                                let activity = cx.world.activity_control();
                                let runtime = cx.kernel_mut().input_runtime();
                                {
                                    assert_eq!(player as usize, side);
                                    let chosen = get_encounter_star_ship_with_picker(
                                        cx,
                                        runtime.input(),
                                        activity,
                                        Some(last),
                                        player,
                                        |cx, player| {
                                            selected[side] = true;
                                            let mut state = FleetSelection::default();
                                            let p = &mut state.player[player as usize];
                                            p.selecting = true;
                                            p.col = 1;
                                            // Expire the computer's display delay without a sleep.
                                            p.entered_at = u32::MAX - 419;
                                            if !(computer && side == 1) {
                                                runtime
                                                    .input()
                                                    .set_player_control(side, side as u32);
                                                sample_weapon(runtime.input(), side, true);
                                            }
                                            let result = {
                                                let mut observer =
                                                    uqm_rust::engine::observe::Unobserved;
                                                let mut io = uqm_rust::engine::observe::Io {
                                                    observer: &mut observer,
                                                };
                                                uqm_rust::supermelee::native::battle_picker::live::select_ship(cx, &mut io, runtime.input(), &inputs[side], &mut state)
                                            };
                                            sample_weapon(runtime.input(), side, false);
                                            assert_eq!(result, 1);
                                            assert!(state.player[side].done);
                                            assert_eq!(
                                                state.player[side].ship,
                                                Some(reserves[side].ship_id)
                                            );
                                            state.player[side].ship
                                        },
                                    );
                                    assert_eq!(chosen.is_some(), expected_selection);
                                    chosen.is_some() as u32
                                }
                            })
                        });
                    }
                    assert_eq!(selected, [expected_selection; 2]);
                    let remaining = crate::battle_context::with(|cx| cx.b.counters);
                    assert_eq!(remaining, [initial[0] - 1, initial[1] - 1]);
                    assert_eq!(
                        crate::battle_context::with(|cx| cx.world.activity()) & CHECK_ABORT,
                        0
                    );
                    assert_eq!(
                        crate::battle_context::with(|cx| cx.world.activity()) & IN_BATTLE != 0,
                        expected_selection
                    );
                    crate::battle_context::with(|cx| {
                        for (side, input) in saved_inputs.into_iter().enumerate() {
                            cx.kernel.replace_player_input(side, input);
                        }
                    });
                    crate::battle_context::with(|cx| cx.b.selection_frames = None);
                    for (side, template) in saved_controls.into_iter().enumerate() {
                        input.set_player_control(side, template);
                    }
                }
            }
            crate::battle_context::with(|cx| {
                (
                    cx.kernel.off_screen_context,
                    cx.kernel.tiny_font,
                    cx.kernel.status_frame,
                ) = saved_graphics;
            });
            drop((font, context, picker, status, screens));
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
