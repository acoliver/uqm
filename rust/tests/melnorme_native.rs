//! Exercise the Melnorme descriptor and callbacks against the linked battle engine.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{prim, Battle, FrameSet, PoolElement, Ship};
use std::{ffi::CStr, mem};
use uqm_rust::engine::battle::ai_constants::ENEMY_SHIP_INDEX;
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_rust::engine::battle::ai_constants::IMMEDIATE_WEAPON;
use uqm_rust::engine::battle::ai_constants::LEFT;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_AVOID;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_NO_MOVEMENT;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_rust::engine::battle::ai_constants::RIGHT;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::ai_constants::THRUST;
use uqm_rust::engine::battle::ai_constants::WEAPON;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::CHANGING;
use uqm_rust::engine::battle::flags::COLLISION;
use uqm_rust::engine::battle::flags::DISAPPEARING;
use uqm_rust::engine::battle::flags::FINITE_LIFE;
use uqm_rust::engine::battle::flags::NONSOLID;
use uqm_rust::engine::battle::record::ElementRecord;
use uqm_rust::engine::battle::record::EvaluateRecord;
use uqm_rust::engine::battle::EMPTY_PRIMITIVE;
use uqm_rust::engine::battle::STAMP_PRIMITIVE;
use uqm_rust::engine::species::species_id::SPECIES_ID_MELNORME_ID;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::arctan;
use uqm_rust::math::battle_types::HALF_CIRCLE;
use uqm_rust::math::battle_types::ONE_SHIFT;
use uqm_rust::math::random;
use uqm_rust::math::seed_random;
use uqm_rust::math::velocity::VelocityDesc;
use uqm_rust::math::velocity::VELOCITY_SHIFT;
use uqm_rust::ships::native::{cosine, facing_to_angle, sine};

unsafe fn trader(battle: &Battle, player: i16) -> Ship<'_> {
    // SAFETY: the serialized fixture retains the battle pools and the ship's frame resources.
    unsafe {
        let mut ship = Ship::with_frames(
            battle,
            SPECIES_ID_MELNORME_ID,
            player,
            [16, 26, 16],
            [22, 18],
        );
        ship.element_mut().current.location = Point { x: 2000, y: 2000 };
        ship.element_mut().next = ship.element_mut().current.clone();
        ship
    }
}

unsafe fn pump(ship: &mut Ship<'_>) -> *mut ElementRecord {
    let handle = ship.weapon_handles(1)[0];
    assert!(!handle.is_null());
    let e = battle::element(handle);
    (*e).parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
    (*e).next = (*e).current.clone();
    battle::put(handle);
    e
}
unsafe fn confusion(ship: &mut Ship<'_>) -> *mut ElementRecord {
    ship.descriptor_mut().ship_info.energy_level = 42;
    ship.ship_mut().cur_status_flags = SPECIAL as u16;
    ship.ship_mut().special_counter = 0;
    let old_tail = battle::tail();
    ship.postprocess();
    let handle = battle::tail();
    assert_ne!(handle, old_tail);
    let e = battle::element(handle);
    (*e).next = (*e).current.clone();
    e
}
unsafe fn remove(e: *mut ElementRecord) {
    battle_context::unlink(e.cast());
    battle_context::dispose(e.cast());
}

unsafe fn queue() -> Vec<*mut std::ffi::c_void> {
    let mut handles = Vec::new();
    let mut h = battle::head();
    while !h.is_null() {
        handles.push(h);
        h = battle::succ(h);
    }
    handles
}

unsafe fn exhaust_queue() -> Vec<*mut std::ffi::c_void> {
    let mut handles = Vec::new();
    loop {
        let h = crate::battle_context::with(|cx| {
            cx.alloc_element()
                .map_or(std::ptr::null_mut(), |id| cx.elements().raw(id))
        });
        if h.is_null() {
            return handles;
        }
        handles.push(h);
    }
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

unsafe fn velocity_for(speed: i32, facing: u16) -> VelocityDesc {
    let angle = facing_to_angle(i32::from(facing)) as u16;
    let mut v = VelocityDesc::default();
    (v).set_components(
        i32::from(cosine(i32::from(angle), speed << VELOCITY_SHIFT)),
        i32::from(sine(i32::from(angle), speed << VELOCITY_SHIFT)),
    );
    v
}

unsafe fn assert_rng_after(seed: u32, draws: usize) {
    let after = random();
    seed_random(seed);
    for _ in 0..draws {
        random();
    }
    assert_eq!(after, random(), "expected {draws} random draws");
}

unsafe fn collide(pulse: *mut ElementRecord, target: *mut ElementRecord) {
    hooks::invoke_collision_pointer(
        (*pulse).collision_func.unwrap(),
        pulse,
        &mut Point { x: 0, y: 0 },
        target,
        &mut Point { x: 0, y: 0 },
        hooks::CollisionCall::Hook,
    );
}

#[test]
fn descriptor_matches_all_c_resources_values_and_empty_runtime_fields() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let d = battle::descriptor(SPECIES_ID_MELNORME_ID);
            assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(
                (
                    d.ship_info.ship_cost,
                    d.ship_info.crew_level,
                    d.ship_info.max_crew
                ),
                (18, 20, 20)
            );
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (42, 42));
            let s = |p: Option<&'static CStr>| p.unwrap();
            assert_eq!(s(d.ship_info.race_strings_rsc), c"ship.melnorme.text");
            assert_eq!(s(d.ship_info.icons_rsc), c"ship.melnorme.icons");
            assert_eq!(s(d.ship_info.melee_icon_rsc), c"ship.melnorme.meleeicons");
            assert!(d.ship_info.race_strings.is_none());
            assert!(d.ship_info.icons.is_null() && d.ship_info.melee_icon.is_null());
            assert_eq!(d.fleet.strength, u16::MAX);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (4999, 4999));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (36, 6, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (5, 20, 4)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (4, 4, 1, 20, 7)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    s(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.melnorme.graphics.trader.{size}")
                );
                assert_eq!(
                    s(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                    format!("ship.melnorme.graphics.pumpup.{size}")
                );
                assert_eq!(
                    s(d.ship_data.special_rsc[i]).to_str().unwrap(),
                    format!("ship.melnorme.graphics.confuse.{size}")
                );
            }
            let captain = &d.ship_data.captain_control;
            assert_eq!(s(captain.captain_rsc), c"ship.melnorme.graphics.captain");
            assert!(
                captain.background.is_null() && captain.turn.is_null() && captain.thrust.is_null()
            );
            assert!(captain.weapon.is_null() && captain.special.is_absent());
            assert_eq!(s(d.ship_data.victory_ditty_rsc), c"ship.melnorme.ditty");
            assert_eq!(s(d.ship_data.ship_sounds_rsc), c"ship.melnorme.sounds");
            assert!(d
                .ship_data
                .ship
                .views()
                .into_iter()
                .chain(d.ship_data.weapon.views())
                .chain(d.ship_data.special.views())
                .all(|f| f.is_absent()));
            assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_none());
            assert_eq!(
                (
                    d.cyborg_control.maneuverability_index,
                    d.cyborg_control.weapon_range
                ),
                (0, 1800)
            );
            assert!(d.callbacks()[2] != 0 && d.callbacks()[3] != 0 && d.callbacks()[4] != 0);
            assert!(d.callbacks()[1] == 0 && d.callbacks()[0] == 0);

            let mut owned = battle::descriptor(SPECIES_ID_MELNORME_ID);
            let loaded = FrameSet::with_bounds(26, [22, 18]);
            let frame = loaded.frame(0);
            owned.ship_info.energy_level = 37;
            owned.ship_data.weapon = loaded.array();
            owned.race_callbacks.post = uqm_rust::engine::battle::Selection::Disabled;
            let fresh = battle::descriptor(SPECIES_ID_MELNORME_ID);
            assert_eq!(fresh.callbacks(), d.callbacks());
            assert_eq!(fresh.ship_info.energy_level, 42);
            assert_eq!(
                (
                    owned.ship_info.energy_level,
                    owned.ship_data.weapon.view(0).raw()
                ),
                (37, frame)
            );
            assert_eq!(owned.callbacks()[2], 0);
        }
    })
}

#[test]
fn initial_pump_uses_the_c_missile_block_at_all_facings_and_coordinate_limits() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            for facing in 0..16 {
                ship.ship_mut().ship_facing = facing;
                (*ship.ptr()).next.location = Point {
                    x: i16::MAX,
                    y: i16::MIN,
                };
                seed_random(32);
                let h = ship.weapon_handles(1)[0];
                assert_rng_after(32, 0);
                let e = battle::element(h);
                assert!(!e.is_null());
                assert_eq!((*e).life, 2);
                assert_eq!((*e).hits, 2);
                assert_eq!((*e).mass, 2);
                assert_eq!((*e).turn, 0);
                assert_eq!((*e).thrust, 72);
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!((*e).player_nr, 0);
                assert_eq!((*e).current.image.frames, ship.frame_set(1));
                assert_eq!(((*e).current.image.frame.raw()).index(), 0);
                assert!((*e).preprocess_func.is_none());
                assert!((*e).postprocess_func.is_some() && (*e).collision_func.is_some());
                assert!(
                    (*e).parent == uqm_rust::engine::battle::Parent::None,
                    "the firing path, not the initializer, attaches the ship"
                );
                assert_eq!(
                    velocity_fields(&(*e).velocity),
                    velocity_fields(&velocity_for(96, facing))
                );
                // pixoffs and speed are both DISPLAY_TO_WORLD(24), but the two
                // separate i16 truncations still matter at diagonal facings.
                let a = facing_to_angle(facing.into());
                let dx = cosine(a, 96 << VELOCITY_SHIFT);
                let dy = sine(a, 96 << VELOCITY_SHIFT);
                let x = i16::MAX
                    .wrapping_add(cosine(a, 96))
                    .wrapping_sub(dx >> VELOCITY_SHIFT);
                let y = i16::MIN
                    .wrapping_add(sine(a, 96))
                    .wrapping_sub(dy >> VELOCITY_SHIFT);
                assert_eq!(((*e).current.location.x, (*e).current.location.y), (x, y));
                assert_eq!(((*e).next.location.x, (*e).next.location.y), (0, 0));
                assert_eq!((*prim(e)).kind, STAMP_PRIMITIVE);
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn appearing_charge_only_zeros_the_entire_velocity_record() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let e = pump(&mut ship);
            let before = queue();
            (*e).turn = 0x82;
            (*e).thrust = 13;
            ship.ship_mut().energy_counter = 19;
            seed_random(7);
            hooks::invoke_element_pointer((*e).postprocess_func.unwrap(), e);
            assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
            assert_eq!((*e).turn, 0x82);
            assert_eq!((*e).thrust, 13);
            assert_eq!(ship.ship_mut().energy_counter, 19);
            assert_eq!(queue(), before);
            assert_eq!((*prim(e)).kind, STAMP_PRIMITIVE);
            assert_rng_after(7, 0);
        }
    })
}

#[test]
fn held_charge_replaces_the_element_and_uses_the_old_level_for_damage() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            for level in 0..4u8 {
                for reverse in [0, 0x80] {
                    let e = pump(&mut ship);
                    (*e).state_flags &= !(APPEARING as u16);
                    (*e).turn = reverse | level;
                    (*e).thrust = 1;
                    (*e).current.image.frame = ship.frame(1, (usize::from(level) * 5 + 2) as u16);
                    ship.ship_mut().cur_status_flags = WEAPON as u16;
                    ship.ship_mut().old_status_flags = WEAPON as u16;
                    ship.ship_mut().energy_counter = 0;
                    seed_random(12);
                    hooks::invoke_element_pointer((*e).postprocess_func.unwrap(), e);
                    let p = battle::element(battle::tail());
                    assert_ne!(p, e);
                    let next_level = (level + 1).min(3);
                    assert_eq!((*p).turn, reverse | next_level);
                    assert_eq!((*p).thrust, 72);
                    assert_eq!(
                        ((*p).current.image.frame.raw()).index(),
                        u16::from(next_level) * 5 + 2
                    );
                    assert_eq!((*p).hits, 2u16 << level);
                    assert_eq!((*p).mass, 2u8 << level);
                    assert_eq!((*p).life, 2);
                    assert_eq!(ship.ship_mut().weapon_counter, 1);
                    assert_eq!(ship.ship_mut().energy_counter, 4);
                    assert_eq!(
                        (*p).parent,
                        uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                    );
                    assert!((*p).preprocess_func.is_none() && (*p).postprocess_func.is_some());
                    assert_eq!((*prim(e)).kind, EMPTY_PRIMITIVE);
                    assert_ne!((*e).state_flags & NONSOLID as u16, 0);
                    assert_eq!(
                        velocity_fields(&(*p).velocity),
                        velocity_fields(&velocity_for(96, ship.ship_mut().ship_facing))
                    );
                    assert_rng_after(12, 0);
                    remove(p);
                    remove(e);
                }
            }
        }
    })
}

#[test]
fn charge_wait_wraps_as_a_byte_and_hit_points_truncate_after_the_c_assignment_chain() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let e = pump(&mut ship);
            (*e).state_flags &= !(APPEARING as u16);
            (*e).turn = 8;
            (*e).thrust = 0;
            (*e).current.image.frame = ship.frame(1, 0);
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.ship_mut().old_status_flags = WEAPON as u16;
            hooks::invoke_element_pointer((*e).postprocess_func.unwrap(), e);
            let p = battle::element(battle::tail());
            assert_eq!((*p).thrust, 255);
            assert_eq!((*p).turn, 8);
            assert_eq!(((*p).current.image.frame.raw()).index(), 1);
            assert_eq!((*p).hits, 512);
            assert_eq!((*p).mass, 0);
        }
    })
}

#[test]
fn release_requires_both_weapon_status_words_to_hold_and_uses_full_velocity_units() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            for (current, old) in [(0, 0), (WEAPON, 0), (0, WEAPON), (WEAPON, WEAPON)] {
                for facing in 0..16 {
                    let e = pump(&mut ship);
                    (*e).state_flags &= !(APPEARING as u16);
                    ship.ship_mut().cur_status_flags = current as u16;
                    ship.ship_mut().old_status_flags = old as u16;
                    ship.ship_mut().ship_facing = facing;
                    ship.ship_mut().weapon_counter = 9;
                    seed_random(14);
                    hooks::invoke_element_pointer((*e).postprocess_func.unwrap(), e);
                    let p = battle::element(battle::tail());
                    let held = current & old & WEAPON != 0;
                    assert_eq!((*p).life, if held { 2 } else { 10 });
                    assert_eq!((*p).preprocess_func.is_some(), !held);
                    assert_eq!((*p).postprocess_func.is_some(), held);
                    assert_eq!(ship.ship_mut().weapon_counter, if held { 1 } else { 9 });
                    assert_eq!(
                        velocity_fields(&(*p).velocity),
                        velocity_fields(&velocity_for(if held { 96 } else { 180 }, facing))
                    );
                    assert_rng_after(14, 0);
                    remove(p);
                    remove(e);
                }
            }
        }
    })
}

#[test]
fn fired_charge_animation_bounces_at_each_five_frame_boundary_without_changing_level() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let e = pump(&mut ship);
            (*e).state_flags &= !(APPEARING as u16);
            ship.ship_mut().cur_status_flags = 0;
            hooks::invoke_element_pointer((*e).postprocess_func.unwrap(), e);
            let p = battle::element(battle::tail());
            let pre = (*p).preprocess_func.unwrap();
            for level in 0..4u8 {
                for (offset, reverse, next, next_reverse) in [
                    (0, 0, 1, 0),
                    (4, 0, 3, 0x80),
                    (3, 0x80, 2, 0x80),
                    (0, 0x80, 1, 0),
                ] {
                    (*p).current.image.frame =
                        ship.frame(1, (usize::from(level) * 5 + offset) as u16);
                    (*p).next.image.frame = (*p).current.image.frame.clone();
                    (*p).turn = level | reverse;
                    (*p).thrust = 2;
                    (*p).state_flags &= !(CHANGING as u16);
                    seed_random(11);
                    hooks::invoke_element_pointer(pre, p);
                    assert_eq!((*p).thrust, 1);
                    assert_eq!((*p).turn, level | next_reverse);
                    assert_eq!(
                        ((*p).next.image.frame.raw()).index(),
                        u16::from(level) * 5 + next
                    );
                    assert_ne!((*p).state_flags & CHANGING as u16, 0);
                    assert_rng_after(11, 0);
                }
            }
            (*p).thrust = 1;
            (*p).state_flags &= !(CHANGING as u16);
            let frame = (*p).next.image.frame.raw();
            hooks::invoke_element_pointer(pre, p);
            assert_eq!((*p).thrust, 0);
            assert_eq!((*p).next.image.frame.raw(), frame);
            assert_eq!((*p).state_flags & CHANGING as u16, 0);
            hooks::invoke_element_pointer(pre, p);
            assert_eq!((*p).thrust, 255);
            assert_ne!((*p).state_flags & CHANGING as u16, 0);
        }
    })
}

#[test]
fn pump_collision_restores_the_shared_blast_counter_and_sets_the_level_blast_animation() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            for (level, width) in [(0, 22), (1, 23), (2, 1026), (3, 22)] {
                let e = pump(&mut ship);
                (*e).turn = 0x80 | level;
                (*e).thrust = 67;
                let index = usize::from(level) * 5;
                ship.set_frame_bounds(1, index, [width, 18]);
                (*e).next.image.frame = ship.frame(1, index as u16);
                (*e).velocity.travel_angle = 0;
                let mut obstacle = PoolElement::new();
                let mut point = Point { x: 20, y: 30 };
                let old_tail = battle::tail();
                seed_random(33);
                hooks::invoke_collision_pointer(
                    (*e).collision_func.unwrap(),
                    e,
                    &mut point,
                    &mut *obstacle,
                    &mut point,
                    hooks::CollisionCall::Hook,
                );
                assert_ne!({ battle::tail() }, old_tail);
                let blast = battle::element(battle::tail());
                assert_eq!((*e).thrust, 67);
                assert_eq!((*e).hits, 0);
                assert_eq!((*blast).life, 3 + u16::from(level));
                assert_eq!((*blast).turn, 0);
                assert_eq!((*blast).thrust, 0);
                assert_eq!((*blast).current.image.frames, (*e).next.image.frames);
                assert_eq!(((*blast).current.image.frame.raw()).index(), 20);
                let offset = i32::from((width >> 1) as u8) << ONE_SHIFT;
                assert_eq!((*blast).current.location.x, 80 + cosine(0, offset));
                assert_eq!((*blast).current.location.y, 120 + sine(0, offset));
                let animate = (*blast).preprocess_func.unwrap();
                (*blast).turn = 1;
                (*blast).thrust = 3;
                (*blast).state_flags &= !(CHANGING as u16);
                hooks::invoke_element_pointer(animate, blast);
                assert_eq!((*blast).turn, 0);
                assert_eq!((*blast).state_flags & CHANGING as u16, 0);
                hooks::invoke_element_pointer(animate, blast);
                assert_eq!(((*blast).next.image.frame.raw()).index(), 21);
                assert_eq!((*blast).turn, 3);
                assert_ne!((*blast).state_flags & CHANGING as u16, 0);
                assert_rng_after(33, 0);
                // Already-collided weapons do not allocate a second blast.
                let before = queue();
                hooks::invoke_collision_pointer(
                    (*e).collision_func.unwrap(),
                    e,
                    &mut point,
                    &mut *obstacle,
                    &mut point,
                    hooks::CollisionCall::Hook,
                );
                assert_eq!(queue(), before);
                assert_eq!((*e).thrust, 67);
                remove(blast);
                remove(e);
            }
        }
    })
}

#[test]
fn confusion_special_obeys_energy_input_and_cooldown_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            for (energy, status, counter, fired) in [
                (19, SPECIAL, 0, false),
                (20, SPECIAL, 0, true),
                (42, SPECIAL, 1, false),
                (42, 0, 0, false),
            ] {
                let mut ship = trader(&_battle, 1);
                ship.ship_mut().cur_status_flags = status as u16;
                ship.ship_mut().special_counter = counter;
                ship.descriptor_mut().ship_info.energy_level = energy;
                ship.descriptor_mut().characteristics.special_wait = 29;
                let before = queue();
                seed_random(23);
                ship.postprocess();
                assert_eq!(
                    ship.descriptor_mut().ship_info.energy_level,
                    if fired { energy - 20 } else { energy }
                );
                assert_eq!(
                    ship.ship_mut().special_counter,
                    if fired { 29 } else { counter }
                );
                if fired {
                    let e = battle::element(battle::tail());
                    assert_eq!((*e).life, 20);
                    assert_eq!((*e).hits, 200);
                    assert_eq!((*e).mass, 0);
                    assert_eq!((*e).thrust, 4);
                    assert_eq!(
                        (*e).state_flags,
                        (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                    );
                    assert_eq!((*e).player_nr, 1);
                    assert_eq!(
                        (*e).parent,
                        uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                    );
                    assert_eq!((*e).current.image.frames, ship.frame_set(2));
                    assert_eq!(((*e).current.image.frame.raw()).index(), 0);
                    assert_eq!(
                        velocity_fields(&(*e).velocity),
                        velocity_fields(&velocity_for(120, ship.ship_mut().ship_facing))
                    );
                    assert!((*e).preprocess_func.is_some() && (*e).collision_func.is_some());
                    assert!((*e).postprocess_func.is_none());
                    assert_eq!(ship.ship_mut().energy_counter, 4);
                    remove(e);
                } else {
                    assert_eq!(queue(), before);
                    if energy == 19 {
                        assert_ne!(
                            ship.ship_mut().cur_status_flags & 32,
                            0,
                            "DeltaEnergy sets LOW_ON_ENERGY"
                        );
                    }
                }
                assert_rng_after(23, 0);
            }
        }
    })
}

#[test]
fn failed_initialization_returns_one_null_slot_and_special_still_spends_energy() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let exhausted = exhaust_queue();
            assert!(!exhausted.is_empty());
            seed_random(18);
            let h = ship.weapon_handles(1)[0];
            assert!(h.is_null());
            let before = queue();
            ship.descriptor_mut().ship_info.energy_level = 42;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 22);
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.ship_mut().energy_counter, 4);
            assert_eq!(queue(), before);
            assert_rng_after(18, 0);
            for h in exhausted {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn solid_confusion_animation_wraps_in_the_first_eight_frames() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let e = confusion(&mut ship);
            let pre = (*e).preprocess_func.unwrap();
            for (index, expected) in [(0, 1), (6, 7), (7, 0), (15, 0)] {
                (*e).current.image.frame = ship.frame(2, index as u16);
                (*e).state_flags &= !(CHANGING as u16);
                seed_random(17);
                hooks::invoke_element_pointer(pre, e);
                assert_eq!(((*e).next.image.frame.raw()).index(), expected);
                assert_ne!((*e).state_flags & CHANGING as u16, 0);
                assert_rng_after(17, 0);
            }
        }
    })
}

#[test]
fn confusion_collision_inserts_a_controller_at_the_head_with_exactly_one_rng_draw() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let mut victim = trader(&_battle, 1);
            let mut seen = [false; 2];
            for seed in 1..33 {
                let e = confusion(&mut ship);
                let old_head = battle::head();
                seed_random(seed);
                let bit = (random() as u8 & 1) as usize;
                seed_random(seed);
                collide(e, victim.ptr());
                let h = battle::head();
                assert_ne!(h, old_head);
                let controller = battle::element(h);
                assert_eq!(battle::succ(controller), old_head);
                assert_eq!((*controller).player_nr, 0);
                assert_eq!(
                    (*controller).state_flags,
                    (FINITE_LIFE | NONSOLID | CHANGING) as u16
                );
                assert_eq!((*controller).life, 400);
                assert_eq!((*controller).turn, 1 << bit);
                assert_eq!((*controller).target, victim.ship_mut().ship_element);
                assert_eq!(
                    (*controller).parent,
                    uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                );
                assert_eq!((*controller).current.image.frames, (*e).next.image.frames);
                assert_eq!(((*controller).current.image.frame.raw()).index(), 8);
                assert_eq!(
                    (*controller).next.image.frame.raw(),
                    (*controller).current.image.frame.raw()
                );
                assert_eq!((*prim(controller)).kind, EMPTY_PRIMITIVE);
                assert_eq!((*e).hits, 0);
                assert_eq!((*e).life, 0);
                assert_eq!(
                    (*e).state_flags & (DISAPPEARING | COLLISION | NONSOLID) as u16,
                    (DISAPPEARING | COLLISION | NONSOLID) as u16
                );
                assert_rng_after(seed, 1);
                seen[bit] = true;
                remove(controller);
                remove(e);
            }
            assert_eq!(seen, [true, true]);
        }
    })
}

#[test]
fn confusion_reuses_the_first_same_player_nonsolid_special_even_if_it_is_a_trail() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let mut victim = trader(&_battle, 1);
            let another = trader(&_battle, 1);
            let pulse = confusion(&mut ship);
            let frames = ship.frame_set(2);
            let mut candidates = Vec::new();
            for (player, frames, flags) in [
                (1, frames.clone(), NONSOLID),
                (0, ship.frame_set(1), NONSOLID),
                (0, frames.clone(), FINITE_LIFE),
                (0, frames.clone(), FINITE_LIFE | NONSOLID | CHANGING),
                (0, frames.clone(), FINITE_LIFE | NONSOLID),
            ] {
                let h = crate::battle_context::with(|cx| {
                    cx.alloc_element()
                        .map_or(std::ptr::null_mut(), |id| cx.elements().raw(id))
                });
                assert!(!h.is_null());
                let e = battle::element(h);
                (*e).player_nr = player;
                (*e).current.image.frames = frames;
                (*e).current.image.frame = ship.frame(2, 11);
                (*e).state_flags = flags as u16;
                (*e).life = 1;
                (*e).target = victim.ship_mut().ship_element;
                (*e).parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
                (*prim(e)).kind = STAMP_PRIMITIVE;
                battle::put(h);
                candidates.push(e);
            }
            let chosen = candidates[3];
            let exhausted = exhaust_queue();
            let before = queue();
            seed_random(51);
            let direction = 1 << (random() as u8 & 1);
            seed_random(51);
            collide(pulse, another.ptr());
            assert_eq!(
                queue(),
                before,
                "reuse does not move or allocate an element"
            );
            assert_eq!((*chosen).life, 400);
            assert_eq!((*chosen).turn, direction);
            assert_eq!(
                (*chosen).target,
                victim.ship_mut().ship_element,
                "reuse does not retarget"
            );
            assert_eq!(((*chosen).current.image.frame.raw()).index(), 11);
            assert!(
                (*chosen).preprocess_func.is_none(),
                "reuse does not initialize the callback"
            );
            assert_eq!((*prim(chosen)).kind, STAMP_PRIMITIVE);
            for &i in &[0, 1, 2, 4] {
                assert_eq!((*candidates[i]).life, 1);
            }
            assert_rng_after(51, 1);
            for h in exhausted {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn confusion_ignores_nonships_and_consumes_no_rng_when_controller_allocation_fails() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let victim = trader(&_battle, 1);
            let e = confusion(&mut ship);
            let mut obstacle = PoolElement::new();
            let before = queue();
            let flags = (*e).state_flags;
            seed_random(62);
            collide(e, &mut *obstacle);
            assert_eq!(queue(), before);
            assert_eq!((*e).state_flags, flags);
            assert_eq!((*e).hits, 200);
            assert_rng_after(62, 0);
            let exhausted = exhaust_queue();
            seed_random(62);
            collide(e, victim.ptr());
            assert_eq!(queue(), before);
            assert_eq!((*e).life, 0);
            assert_eq!((*e).hits, 0);
            assert_ne!((*e).state_flags & NONSOLID as u16, 0);
            assert_rng_after(62, 0);
            for h in exhausted {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn controller_follows_target_overrides_controls_wraps_animation_and_emits_one_frame_trails() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let mut victim = trader(&_battle, 1);
            let e = confusion(&mut ship);
            collide(e, victim.ptr());
            let controller = battle::element(battle::head());
            let pre = (*controller).preprocess_func.unwrap();
            for direction in [LEFT, RIGHT] {
                (*controller).turn = direction as u8;
                (*controller).current.image.frame = ship.frame(2, 15);
                (*victim.ptr()).next.location = Point { x: -123, y: 456 };
                victim.ship_mut().ship_input_state =
                    (LEFT | RIGHT | THRUST | WEAPON | SPECIAL | 0x80) as u8;
                let before = queue().len();
                seed_random(88);
                hooks::invoke_element_pointer(pre, controller);
                assert_eq!(
                    victim.ship_mut().ship_input_state,
                    (THRUST | WEAPON | 0x80 | direction) as u8
                );
                assert_eq!(
                    ((*controller).next.location.x, (*controller).next.location.y),
                    (-123, 456)
                );
                assert_eq!(((*controller).next.image.frame.raw()).index(), 8);
                assert_eq!(queue().len(), before + 1);
                let trail = battle::element(battle::tail());
                assert_eq!((*trail).player_nr, 0);
                assert_eq!(
                    (*trail).state_flags,
                    (FINITE_LIFE | NONSOLID | CHANGING) as u16
                );
                assert_eq!((*trail).life, 1);
                assert_eq!((*trail).turn, 0);
                assert_eq!((*trail).target, victim.ship_mut().ship_element);
                assert_eq!((*trail).parent, (*controller).parent);
                assert_eq!(
                    (*trail).current.image.frame.raw(),
                    (*controller).next.image.frame.raw()
                );
                assert_eq!(
                    (*trail).next.image.frame.raw(),
                    (*controller).next.image.frame.raw()
                );
                assert_eq!((*prim(trail)).kind, STAMP_PRIMITIVE);
                assert_eq!((*trail).preprocess_func.unwrap().address(), (pre).address());
                // The visible one-frame copy follows, but does not override inputs
                // or emit another copy because its turn_wait is zero.
                victim.ship_mut().ship_input_state = SPECIAL as u8;
                (*victim.ptr()).next.location.x = 800;
                let count = queue().len();
                hooks::invoke_element_pointer(pre, trail);
                assert_eq!((*trail).next.location.x, 800);
                assert_eq!(victim.ship_mut().ship_input_state, SPECIAL as u8);
                assert_eq!(queue().len(), count);
                assert_rng_after(88, 0);
                remove(trail);
            }
            (*controller).target = None;
            let before = queue();
            hooks::invoke_element_pointer(pre, controller);
            assert_eq!((*controller).life, 0);
            assert_ne!((*controller).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(queue(), before);
        }
    })
}

#[test]
fn controller_still_overrides_controls_when_trail_allocation_fails() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let mut victim = trader(&_battle, 1);
            let pulse = confusion(&mut ship);
            collide(pulse, victim.ptr());
            let controller = battle::element(battle::head());
            (*controller).turn = RIGHT as u8;
            (*controller).current.image.frame = ship.frame(2, 8);
            let exhausted = exhaust_queue();
            let before = queue();
            victim.ship_mut().ship_input_state = (LEFT | SPECIAL | WEAPON) as u8;
            seed_random(9);
            hooks::invoke_element_pointer((*controller).preprocess_func.unwrap(), controller);
            assert_eq!(victim.ship_mut().ship_input_state, (RIGHT | WEAPON) as u8);
            assert_eq!(((*controller).next.image.frame.raw()).index(), 9);
            assert_eq!(queue(), before);
            assert_rng_after(9, 0);
            for h in exhausted {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn ai_without_targets_preserves_c_toggle_thresholds_counters_and_rng_order() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            for (energy, special_counter, weapon_counter, initial, expected) in [
                (19, 0, 0, 0, 0),
                (20, 0, 0, 0, 0),
                (24, 0, 0, 0, 0),
                (25, 0, 0, 0, WEAPON),
                (0, 1, 0, 0, WEAPON),
                (19, 0, 1, 0, WEAPON),
                (24, 0, 1, 0, WEAPON),
                (42, 0, 2, 0, 0),
                (19, 0, 2, WEAPON, WEAPON),
                (20, 0, 2, WEAPON, WEAPON | SPECIAL),
                (42, 1, 2, SPECIAL | LEFT, LEFT),
            ] {
                let mut ship = trader(&_battle, 0);
                ship.descriptor_mut().ship_info.energy_level = energy;
                ship.ship_mut().special_counter = special_counter;
                ship.ship_mut().weapon_counter = weapon_counter;
                ship.ship_mut().ship_input_state = initial as u8;
                (*ship.ptr()).turn = 255;
                (*ship.ptr()).thrust = 255;
                let mut concerns: [EvaluateRecord; 4] = mem::zeroed();
                let init = ship.descriptor_mut().callbacks()[3];
                let before = queue();
                seed_random(77);
                ship.think(ship.ptr(), &mut concerns, 0);
                assert_eq!(
                    ship.ship_mut().ship_input_state,
                    expected as u8,
                    "energy {energy}, special {special_counter}, weapon {weapon_counter}"
                );
                assert_eq!(ship.ship_mut().weapon_counter, weapon_counter);
                assert_eq!(ship.descriptor_mut().callbacks()[3], init);
                assert_eq!((*ship.ptr()).turn, 255);
                assert_eq!((*ship.ptr()).thrust, 255);
                assert_eq!(queue(), before);
                assert_rng_after(77, 0);
            }
        }
    })
}

#[test]
fn ai_enemy_move_state_matches_energy_held_weapon_and_immediate_weapon_rules() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let mut enemy = trader(&_battle, 1);
            ship.ship_mut().special_counter = 1;
            for (energy, held, immediate, expected) in [
                (24, false, false, MOVEMENT_STATE_ENTICE),
                (24, false, true, MOVEMENT_STATE_ENTICE),
                (25, false, false, MOVEMENT_STATE_PURSUE),
                (24, true, false, MOVEMENT_STATE_PURSUE),
                (25, false, true, MOVEMENT_STATE_AVOID),
                (24, true, true, MOVEMENT_STATE_AVOID),
            ] {
                ship.descriptor_mut().ship_info.energy_level = energy;
                ship.ship_mut().old_status_flags = if held { WEAPON as u16 } else { 0 };
                enemy.descriptor_mut().ship_info.ship_flags = if immediate {
                    IMMEDIATE_WEAPON as u16
                } else {
                    0
                };
                let mut concerns: [EvaluateRecord; 4] = mem::zeroed();
                concerns[ENEMY_SHIP_INDEX as usize].object_ptr = enemy.ptr();
                concerns[ENEMY_SHIP_INDEX as usize].move_state = MOVEMENT_STATE_AVOID;
                ship.think(ship.ptr(), &mut concerns, 0);
                assert_eq!(concerns[ENEMY_SHIP_INDEX as usize].move_state, expected);
            }
        }
    })
}

#[test]
fn ai_tests_the_confusion_trajectory_and_restores_waits_and_initializer() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let enemy = trader(&_battle, 1);
            // The first pass has zero concerns. The special pass always evaluates
            // the enemy slot and must not move a ship whose waits were zero.
            (*ship.ptr()).turn = 0;
            (*ship.ptr()).thrust = 0;
            let init = ship.descriptor_mut().callbacks()[3];
            for (energy, distance, expected) in [
                (19, 320, 0),
                (20, 320, SPECIAL),
                (24, 320, SPECIAL),
                (25, 320, WEAPON | SPECIAL),
                (25, 4000, WEAPON),
            ] {
                ship.descriptor_mut().ship_info.energy_level = energy;
                ship.ship_mut().ship_input_state = 0;
                ship.ship_mut().weapon_counter = 0;
                ship.ship_mut().special_counter = 0;
                (*enemy.ptr()).current.location = Point {
                    x: 2000 + cosine(0, distance),
                    y: 2000 + sine(0, distance),
                };
                (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
                let mut concerns: [EvaluateRecord; 4] = mem::zeroed();
                concerns[ENEMY_SHIP_INDEX as usize].object_ptr = enemy.ptr();
                concerns[ENEMY_SHIP_INDEX as usize].move_state = MOVEMENT_STATE_NO_MOVEMENT;
                let before = queue();
                seed_random(95);
                ship.think(ship.ptr(), &mut concerns, 0);
                assert_eq!(
                    ship.ship_mut().ship_input_state,
                    expected as u8,
                    "energy {energy}, distance {distance}"
                );
                assert_eq!((*ship.ptr()).turn, 0);
                assert_eq!((*ship.ptr()).thrust, 0);
                assert_eq!(ship.ship_mut().ship_facing, 0);
                assert_eq!(ship.ship_mut().weapon_counter, 0);
                assert_eq!(ship.descriptor_mut().callbacks()[3], init);
                assert_eq!(queue(), before, "AI test missiles are freed, not queued");
                assert_rng_after(95, 0);
            }
        }
    })
}

#[test]
fn held_charge_uses_the_same_ping_pong_animation_as_a_released_pulse() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.ship_mut().old_status_flags = WEAPON as u16;
            for (index, turn, expected_index, expected_turn) in [
                (0, 0, 1, 0),
                (4, 0, 3, 0x80),
                (7, 0x81, 6, 0x81),
                (5, 0x81, 6, 1),
                (14, 2, 13, 0x82),
                (15, 0x83, 16, 3),
            ] {
                let e = pump(&mut ship);
                (*e).state_flags &= !(APPEARING as u16);
                (*e).current.image.frame = ship.frame(1, index as u16);
                (*e).turn = turn;
                (*e).thrust = 2;
                hooks::invoke_element_pointer((*e).postprocess_func.unwrap(), e);
                let p = battle::element(battle::tail());
                assert_eq!((*p).thrust, 1);
                assert_eq!((*p).turn, expected_turn);
                assert_eq!(((*p).current.image.frame.raw()).index(), expected_index);
                assert_eq!((*p).state_flags & CHANGING as u16, 0);
                remove(p);
                remove(e);
            }
        }
    })
}

#[test]
fn ai_first_pass_tests_the_full_speed_pump_and_toggles_an_existing_charge_to_release() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let enemy = trader(&_battle, 1);
            // Blocking movement lets the real shared AI test each projectile
            // without changing the facing used by the second pass.
            (*ship.ptr()).turn = 1;
            (*ship.ptr()).thrust = 1;
            for (distance, counter, expected) in [
                (1400, 0, WEAPON | SPECIAL),
                (1400, 1, SPECIAL),
                (2000, 0, WEAPON | SPECIAL),
                (2000, 1, WEAPON | SPECIAL),
                (2600, 0, WEAPON),
            ] {
                ship.descriptor_mut().ship_info.energy_level = 42;
                ship.ship_mut().ship_input_state = 0;
                ship.ship_mut().special_counter = 0;
                ship.ship_mut().weapon_counter = counter;
                (*enemy.ptr()).current.location = Point {
                    x: 2000 + cosine(0, distance),
                    y: 2000 + sine(0, distance),
                };
                (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
                let mut concerns: [EvaluateRecord; 4] = mem::zeroed();
                concerns[ENEMY_SHIP_INDEX as usize].object_ptr = enemy.ptr();
                let before = queue();
                seed_random(71);
                ship.think(ship.ptr(), &mut concerns, 1);
                assert_eq!(
                    ship.ship_mut().ship_input_state,
                    expected as u8,
                    "distance {distance}, counter {counter}"
                );
                assert_eq!(ship.ship_mut().weapon_counter, counter);
                assert_eq!((*ship.ptr()).turn, 1);
                assert_eq!((*ship.ptr()).thrust, 1);
                assert_eq!(queue(), before);
                assert_rng_after(71, 0);
            }
        }
    })
}

#[test]
fn ai_pursuit_uses_the_shared_half_turn_rng_draw_without_extra_draws() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        unsafe {
            let mut ship = trader(&_battle, 0);
            let enemy = trader(&_battle, 1);
            let dx = cosine(HALF_CIRCLE as i32, 400);
            let dy = sine(HALF_CIRCLE as i32, 400);
            assert_eq!(arctan(dx, dy), HALF_CIRCLE);
            (*enemy.ptr()).current.location = Point {
                x: 2000 + dx,
                y: 2000 + dy,
            };
            (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
            ship.ship_mut().weapon_counter = 2;
            ship.ship_mut().special_counter = 1;
            for seed in 1..9 {
                ship.ship_mut().ship_facing = 0;
                ship.ship_mut().ship_input_state = 0;
                (*ship.ptr()).turn = 0;
                (*ship.ptr()).thrust = 1;
                let mut concerns: [EvaluateRecord; 4] = mem::zeroed();
                concerns[ENEMY_SHIP_INDEX as usize].object_ptr = enemy.ptr();
                concerns[ENEMY_SHIP_INDEX as usize].which_turn = 1;
                seed_random(seed);
                let left = random() & 1 != 0;
                seed_random(seed);
                ship.think(ship.ptr(), &mut concerns, 1);
                assert_eq!(
                    ship.ship_mut().ship_input_state,
                    if left { LEFT } else { RIGHT } as u8
                );
                assert_eq!(ship.ship_mut().ship_facing, if left { 15 } else { 1 });
                assert_eq!(ship.ship_mut().weapon_counter, 2);
                assert_rng_after(seed, 1);
            }
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
