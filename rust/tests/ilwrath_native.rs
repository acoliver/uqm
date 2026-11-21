//! Exercise the Ilwrath live descriptor and registered callbacks against the linked engine.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{Battle, FrameSet, Ship};
#[path = "support/special_hooks.rs"]
mod special_hooks;
use uqm_engine::battle::ai_constants::{
    ENEMY_SHIP_INDEX, ENEMY_WEAPON_INDEX, FIRES_FORE, FIRST_EMPTY_INDEX, IGNORE_SIMILAR, LEFT,
    MOVEMENT_STATE_AVOID, MOVEMENT_STATE_PURSUE, RIGHT, SPECIAL, THRUST, WEAPON,
};
use uqm_engine::battle::flags::{
    APPEARING, CHANGING, DISAPPEARING, FINITE_LIFE, NONSOLID, STAMPFILL_PRIMITIVE,
};
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::battle::{EMPTY_PRIMITIVE, STAMP_PRIMITIVE};
use uqm_engine::collections::observation;
use uqm_engine::graphics::geometry::Color;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::graphics::menu::frame_index as index_of;
use uqm_engine::species::species_id::SPECIES_ID_ILWRATH_ID;
use uqm_engine::world::activity::activity_flags::CHECK_PAUSE;
use uqm_engine::world::activity::activity_kinds::IN_ENCOUNTER;
use uqm_engine::world::IN_HYPERSPACE;

fn spawn_ship(fleet: &mut Ship<'_>, x: i16, y: i16) -> ElementId {
    let ship = fleet.id;
    battle::get_mut(ship).current.location = Point { x, y };
    battle::get_mut(ship).next.location = Point { x, y };
    ship
}
/// `COLLISION` from `element.h`, which the generated ABI does not export.
const ELEMENT_COLLISION: u16 = 1 << 7;

type Rgba = (u8, u8, u8, u8);

// BUILD_COLOR (MAKE_RGB15 (r, g, b), index): five-bit channels widen with
// `(c << 3) | (c >> 2)`, alpha is 0xFF and the palette index is dropped.
const BLACK: Rgba = (0, 0, 0, 255);
const WHITE: Rgba = (255, 255, 255, 255);
const CYAN_LIGHT: Rgba = (82, 255, 255, 255);
const CYAN_DARK: Rgba = (0, 165, 165, 255);
const BLUE_LIGHT: Rgba = (82, 82, 255, 255);
const BLUE_DARK: Rgba = (0, 0, 165, 255);

fn color_of(ship: ElementId) -> Rgba {
    let color = battle::prim(ship).color;
    (color.r, color.g, color.b, color.a)
}

fn paint(ship: ElementId, kind: u8, (r, g, b, a): Rgba) {
    let mut prim = battle::prim_mut(ship);
    prim.kind = kind;
    prim.color = Color { r, g, b, a };
}

fn prim_kind(ship: ElementId) -> u8 {
    battle::prim(ship).kind
}

fn preprocess(fleet: &mut Ship<'_>, ship: ElementId) {
    assert_eq!(ship, fleet.id);
    fleet.preprocess();
}

/// An Avenger at (1000, 1000) facing 0 with a visible primitive.
fn avenger(fleet: &mut Ship<'_>) -> ElementId {
    let ship = spawn_ship(fleet, 1000, 1000);
    fleet.ship_mut().ship_facing = 0;
    ship
}

#[test]
fn avenger_descriptor_matches_the_c_template() {
    battle_context::run(|_| {
        {
            let desc = battle::descriptor(SPECIES_ID_ILWRATH_ID);
            assert_eq!(desc.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(desc.ship_info.ship_cost, 10);
            assert_eq!(
                (desc.ship_info.crew_level, desc.ship_info.max_crew),
                (22, 22)
            );
            assert_eq!(
                (desc.ship_info.energy_level, desc.ship_info.max_energy),
                (16, 16)
            );
            assert_eq!(
                desc.ship_info.race_strings_rsc.unwrap(),
                c"ship.ilwrath.text"
            );
            assert_eq!(desc.ship_info.icons_rsc.unwrap(), c"ship.ilwrath.icons");
            assert_eq!(
                desc.ship_info.melee_icon_rsc.unwrap(),
                c"ship.ilwrath.meleeicons"
            );
            // 1410 / SPHERE_RADIUS_INCREMENT * 2
            assert_eq!(desc.fleet.strength, 256);
            assert_eq!((desc.fleet.known_loc.x, desc.fleet.known_loc.y), (48, 1700));
            let c = &desc.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (25, 5, 4)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (1, 3, 4)
            );
            assert_eq!((c.turn_wait, c.thrust_wait, c.weapon_wait), (2, 0, 0));
            assert_eq!((c.special_wait, c.ship_mass), (13, 7));

            let data = &desc.ship_data;
            let expected_ship = [
                c"ship.ilwrath.graphics.avenger.large",
                c"ship.ilwrath.graphics.avenger.medium",
                c"ship.ilwrath.graphics.avenger.small",
            ];
            let expected_weapon = [
                c"ship.ilwrath.graphics.fire.large",
                c"ship.ilwrath.graphics.fire.medium",
                c"ship.ilwrath.graphics.fire.small",
            ];
            for i in 0..3 {
                assert_eq!(data.ship_rsc[i].unwrap(), expected_ship[i]);
                assert_eq!(data.weapon_rsc[i].unwrap(), expected_weapon[i]);
                assert!(data.special_rsc[i].is_none());
                assert!(data.ship.view(i).is_absent());
                assert!(data.weapon.view(i).is_absent());
                assert!(data.special.view(i).is_absent());
            }
            assert_eq!(
                data.captain_control.captain_rsc.unwrap(),
                c"ship.ilwrath.graphics.captain"
            );
            assert_eq!(data.victory_ditty_rsc.unwrap(), c"ship.ilwrath.ditty");
            assert_eq!(data.ship_sounds_rsc.unwrap(), c"ship.ilwrath.sounds");

            // CLOSE_RANGE_WEAPON: DISPLAY_TO_WORLD (50).
            assert_eq!(desc.cyborg_control.maneuverability_index, 0);
            assert_eq!(desc.cyborg_control.weapon_range, 200);
            assert!(desc.callbacks()[4] != 0);
            assert!(desc.callbacks()[1] != 0);
            assert!(desc.callbacks()[3] != 0);
            assert!(desc.callbacks()[2] == 0);
            assert!(desc.callbacks()[0] == 0);
        }
    })
}

#[test]
fn flame_inherits_the_ship_velocity_and_starts_on_the_first_frame() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let flame_frames = FrameSet::with_bounds(16, [0, 0]);
            fleet.descriptor_mut().ship_data.weapon = flame_frames.array();
            let ship = spawn_ship(&mut fleet, 400, 500);
            // Facing 4 is angle 16, along +x (angle 0 points up the screen).
            fleet.ship_mut().ship_facing = 4;
            (battle::get_mut(ship).velocity).delta_components(160, -64);

            let mut out = [None];
            out[0] = fleet.weapon_handles(1)[0];
            let flame = out[0].unwrap();
            assert_eq!(battle::get(flame).life, 8);
            assert_eq!(battle::get(flame).hits, 1);
            assert_eq!(battle::get(flame).mass, 1);
            assert_eq!(battle::get(flame).thrust, 0);
            assert_eq!(battle::get(flame).turn, 0);
            assert_eq!(
                battle::get(flame).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            // MissileBlock.index is 0 whatever the facing: the flame animates
            // from its first frame.
            assert_eq!(index_of(battle::get(flame).current.image.frame.raw()), 0);
            assert!(battle::get(flame).preprocess_func.is_some());
            assert!(battle::get(flame).collision_func.is_some());

            // Muzzle at 29 display pixels, pulled back one frame of the missile's
            // own travel (25 << 5 >> 5) and one frame of the ship's drift.
            assert_eq!(
                (
                    battle::get(flame).current.location.x,
                    battle::get(flame).current.location.y
                ),
                (400 + 29 * 4 - 25 - (160 >> 5), 500 + (64 >> 5))
            );
            let (velocity_x, velocity_y) = (battle::get(flame).velocity).get_current_components();
            let (dx, dy) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((dx, dy), (25 * 32 + 160, -64));
        }
    })
}

#[test]
fn flame_advances_its_animation_every_next_turn_frames() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let flame_frames = FrameSet::with_bounds(4, [0, 0]);
            fleet.descriptor_mut().ship_data.weapon = flame_frames.array();
            spawn_ship(&mut fleet, 0, 0);
            let mut out = [None];
            out[0] = fleet.weapon_handles(1)[0];
            let flame = out[0].unwrap();
            let step = battle::get(flame).preprocess_func.unwrap();
            battle::get_mut(flame).thrust = 2;

            // turn_wait starts at 0, so the first pass animates immediately and
            // reloads the delay from next_turn.
            hooks::invoke_element(step, flame);
            assert_eq!(index_of(battle::get(flame).next.image.frame.raw()), 1);
            assert_ne!(battle::get(flame).state_flags & CHANGING as u16, 0);
            assert_eq!(battle::get(flame).turn, 2);

            battle::get_mut(flame).state_flags &= !(CHANGING as u16);
            battle::get_mut(flame).current.image.frame =
                battle::get(flame).next.image.frame.clone();
            hooks::invoke_element(step, flame);
            hooks::invoke_element(step, flame);
            assert_eq!(battle::get(flame).turn, 0);
            assert_eq!(battle::get(flame).state_flags & CHANGING as u16, 0);
            assert_eq!(index_of(battle::get(flame).next.image.frame.raw()), 1);

            hooks::invoke_element(step, flame);
            assert_eq!(index_of(battle::get(flame).next.image.frame.raw()), 2);
            assert_ne!(battle::get(flame).state_flags & CHANGING as u16, 0);

            // The animation wraps after the last frame.
            battle::get_mut(flame).current.image.frame = flame_frames.selection(3);
            battle::get_mut(flame).turn = 0;
            hooks::invoke_element(step, flame);
            assert_eq!(index_of(battle::get(flame).next.image.frame.raw()), 0);
        }
    })
}

#[test]
fn flame_that_hits_goes_nonsolid_without_disappearing() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            spawn_ship(&mut fleet, 400, 500);
            let mut out = [None];
            out[0] = fleet.weapon_handles(1)[0];
            let flame = out[0].unwrap();
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let target = spawn_ship(&mut other, 410, 500);
            // A target that neither has finite life nor the normal-life marker
            // takes no damage, which keeps the collision self-contained.
            battle::get_mut(target).state_flags = 0;
            battle::get_mut(target).life = 5;
            let mut flame_point = Point { x: 100, y: 125 };
            let mut target_point = Point { x: 102, y: 125 };

            hooks::invoke_collision(
                battle::get(flame).collision_func.unwrap(),
                flame,
                &mut flame_point,
                target,
                &mut target_point,
            );

            // weapon_collision marks it DISAPPEARING | COLLISION | NONSOLID; the
            // Ilwrath callback then undoes DISAPPEARING only.
            assert_eq!(battle::get(flame).state_flags & DISAPPEARING as u16, 0);
            assert_ne!(battle::get(flame).state_flags & NONSOLID as u16, 0);
            assert_ne!(battle::get(flame).state_flags & ELEMENT_COLLISION, 0);
            assert_eq!(battle::get(flame).life, 0);
            assert_eq!(battle::get(flame).hits, 0);
        }
    })
}

#[test]
fn special_key_starts_the_cloak_and_spends_energy() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, STAMP_PRIMITIVE, WHITE);
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;
            observation::enable();
            observation::drain();

            preprocess(&mut fleet, ship);

            assert_eq!(special_hooks::count(&observation::drain(), "ilwrath"), 1);
            assert_eq!(prim_kind(ship), STAMPFILL_PRIMITIVE);
            assert_eq!(color_of(ship), WHITE);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 13);
            assert_eq!(fleet.ship_mut().special_counter, 13);
            assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
            // The key state belongs to the engine: only a local copy is cleared.
            assert_eq!(fleet.ship_mut().cur_status_flags, SPECIAL as u16);
        }
    })
}

#[test]
fn cloak_needs_energy_and_a_ready_counter() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        crate::battle_context::with(|cx| cx.world.set_activity(0));
        crate::battle_context::with(|cx| uqm_engine::state::ffi::rust_init_game_state(cx.world));
        let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
        let ship = avenger(&mut fleet);
        paint(ship, STAMP_PRIMITIVE, WHITE);
        fleet.ship_mut().cur_status_flags = SPECIAL as u16;

        fleet.descriptor_mut().ship_info.energy_level = 2;
        observation::enable();
        observation::drain();
        preprocess(&mut fleet, ship);
        assert_eq!(special_hooks::count(&observation::drain(), "ilwrath"), 0);
        assert_eq!(prim_kind(ship), STAMP_PRIMITIVE);
        assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 2);
        assert_eq!(fleet.ship_mut().special_counter, 0);

        fleet.descriptor_mut().ship_info.energy_level = 16;
        fleet.ship_mut().special_counter = 3;
        preprocess(&mut fleet, ship);
        assert_eq!(prim_kind(ship), STAMP_PRIMITIVE);
        assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 16);

        fleet.ship_mut().special_counter = 0;
        fleet.ship_mut().cur_status_flags = 0;
        preprocess(&mut fleet, ship);
        assert_eq!(prim_kind(ship), STAMP_PRIMITIVE);
        assert_eq!(battle::get(ship).state_flags & CHANGING as u16, 0);
    })
}

#[test]
fn cloak_fades_to_black_one_step_per_frame_and_drops_pursuers() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let hunter = spawn_ship(&mut other, 3000, 1000);
            battle::get_mut(hunter).target = Some(ship);
            paint(ship, STAMP_PRIMITIVE, WHITE);
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;
            preprocess(&mut fleet, ship);
            assert_eq!(prim_kind(ship), STAMPFILL_PRIMITIVE);

            // The special counter is still running, so the key only walks the
            // color down: white, then four blues, then black.
            let walk = [CYAN_LIGHT, CYAN_DARK, BLUE_LIGHT, BLUE_DARK, BLACK];
            for expected in walk {
                battle::get_mut(ship).state_flags &= !(CHANGING as u16);
                assert!(!battle::get(hunter).target.is_none());
                preprocess(&mut fleet, ship);
                assert_eq!(color_of(ship), expected);
                assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
                assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 13);
                assert_eq!(fleet.ship_mut().special_counter, 13);
            }
            // Reaching black untargets the ship.
            assert!(battle::get(hunter).target.is_none());
            assert_eq!(prim_kind(ship), STAMPFILL_PRIMITIVE);

            // Fully cloaked: nothing more happens until the counter runs out.
            battle::get_mut(ship).state_flags &= !(CHANGING as u16);
            preprocess(&mut fleet, ship);
            assert_eq!(color_of(ship), BLACK);
            assert_eq!(battle::get(ship).state_flags & CHANGING as u16, 0);
        }
    })
}

#[test]
fn decloak_walks_back_up_and_ends_as_a_plain_stamp() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, STAMPFILL_PRIMITIVE, BLACK);
            fleet.ship_mut().special_counter = 0;
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;

            // Pressing special while cloaked starts the decloak at the darkest
            // blue; it neither recloaks nor spends energy.
            observation::enable();
            observation::drain();
            preprocess(&mut fleet, ship);
            // Decloaking clears the local SPECIAL before the cloak check, so nothing activates.
            assert_eq!(special_hooks::count(&observation::drain(), "ilwrath"), 0);
            assert_eq!(color_of(ship), BLUE_DARK);
            assert_eq!(prim_kind(ship), STAMPFILL_PRIMITIVE);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 16);
            assert_eq!(fleet.ship_mut().special_counter, 0);
            assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
            // The pressed key is still reported to the ship afterwards.
            assert_eq!(fleet.ship_mut().cur_status_flags, SPECIAL as u16);

            fleet.ship_mut().cur_status_flags = 0;
            for expected in [BLUE_LIGHT, CYAN_DARK, CYAN_LIGHT, WHITE] {
                battle::get_mut(ship).state_flags &= !(CHANGING as u16);
                preprocess(&mut fleet, ship);
                assert_eq!(color_of(ship), expected);
                assert_eq!(prim_kind(ship), STAMPFILL_PRIMITIVE);
                assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
            }

            // The last step turns the fill primitive back into a normal stamp.
            battle::get_mut(ship).state_flags &= !(CHANGING as u16);
            preprocess(&mut fleet, ship);
            assert_eq!(prim_kind(ship), STAMP_PRIMITIVE);
            assert_eq!(color_of(ship), WHITE);
            assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 16);

            // A visible ship with no key held is left alone.
            battle::get_mut(ship).state_flags &= !(CHANGING as u16);
            preprocess(&mut fleet, ship);
            assert_eq!(prim_kind(ship), STAMP_PRIMITIVE);
            assert_eq!(battle::get(ship).state_flags & CHANGING as u16, 0);
        }
    })
}

#[test]
fn firing_while_cloaked_decloaks_and_turns_toward_the_target() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship_frames = FrameSet::with_bounds(16, [0, 0]);
            let ship = avenger(&mut fleet);
            battle::get_mut(ship).next.image.frame = ship_frames.selection(0);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            // Straight along +x: ARCTAN gives angle 16, facing 4.
            spawn_ship(&mut other, 2000, 1000);
            paint(ship, STAMPFILL_PRIMITIVE, BLACK);
            fleet.ship_mut().special_counter = 9;
            fleet.ship_mut().cur_status_flags = WEAPON as u16;

            preprocess(&mut fleet, ship);

            assert_eq!(color_of(ship), BLUE_DARK);
            assert_eq!(fleet.ship_mut().ship_facing, 4);
            assert_eq!(index_of(battle::get(ship).next.image.frame.raw()), 4);
            // turn_wait of zero is bumped so the new facing is not undone at once.
            assert_eq!(battle::get(ship).turn, 1);
            assert!(battle::get(ship).target.is_none());
            assert_eq!(fleet.ship_mut().special_counter, 0);
            assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
            assert_eq!(fleet.ship_mut().cur_status_flags, WEAPON as u16);
        }
    })
}

#[test]
fn decloak_aim_leads_the_target_by_four_frames_of_motion() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship_frames = FrameSet::with_bounds(16, [0, 0]);
            let ship = avenger(&mut fleet);
            battle::get_mut(ship).next.image.frame = ship_frames.selection(0);
            // Moving +y by 250 world units a frame puts the ship 1000 units
            // further on after the four look-ahead frames.
            (battle::get_mut(ship).velocity).delta_components(0, 8000);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            spawn_ship(&mut other, 2000, 1000);
            paint(ship, STAMPFILL_PRIMITIVE, BLACK);
            fleet.ship_mut().cur_status_flags = WEAPON as u16;

            preprocess(&mut fleet, ship);

            // The aim point is (2000 - 1000, 1000 - 2000): angle 8, facing 2.
            // Without the look-ahead the ship would face 4.
            assert_eq!(fleet.ship_mut().ship_facing, 2);
            assert_eq!(index_of(battle::get(ship).next.image.frame.raw()), 2);
        }
    })
}

#[test]
fn firing_while_cloaked_without_a_target_keeps_the_facing() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        crate::battle_context::with(|cx| cx.world.set_activity(0));
        crate::battle_context::with(|cx| uqm_engine::state::ffi::rust_init_game_state(cx.world));
        let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
        let ship_frames = FrameSet::with_bounds(16, [0, 0]);
        let ship = avenger(&mut fleet);
        battle::get_mut(ship).next.image.frame = ship_frames.selection(0);
        paint(ship, STAMPFILL_PRIMITIVE, BLACK);
        fleet.ship_mut().ship_facing = 6;
        fleet.ship_mut().cur_status_flags = WEAPON as u16;

        preprocess(&mut fleet, ship);

        assert_eq!(color_of(ship), BLUE_DARK);
        assert_eq!(fleet.ship_mut().ship_facing, 6);
        assert_eq!(index_of(battle::get(ship).next.image.frame.raw()), 0);
        assert_eq!(battle::get(ship).turn, 0);
        assert!(battle::get(ship).target.is_none());
    })
}

#[test]
fn firing_needs_energy_to_break_the_cloak() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        crate::battle_context::with(|cx| cx.world.set_activity(0));
        crate::battle_context::with(|cx| uqm_engine::state::ffi::rust_init_game_state(cx.world));
        let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
        let ship = avenger(&mut fleet);
        paint(ship, STAMPFILL_PRIMITIVE, BLACK);
        fleet.ship_mut().special_counter = 5;
        fleet.ship_mut().cur_status_flags = WEAPON as u16;
        fleet.descriptor_mut().ship_info.energy_level = 0;

        preprocess(&mut fleet, ship);

        assert_eq!(color_of(ship), BLACK);
        assert_eq!(fleet.ship_mut().special_counter, 5);
        assert_eq!(battle::get(ship).state_flags & CHANGING as u16, 0);
    })
}

fn enemy_in_range(which_turn: u16) -> Concern {
    Concern {
        object: None,
        facing: 0,
        which_turn,
        movement: MOVEMENT_STATE_AVOID,
    }
}

/// Runs the Ilwrath intelligence with no concerns for `ship_intelligence` to
/// act on, so only the Avenger's own decisions show in the input state.
fn think(fleet: &mut Ship<'_>, ship: ElementId, concerns: &mut [Concern; 4], input: u8) -> u8 {
    fleet.ship_mut().ship_input_state = input;
    fleet.think(ship, concerns, 0);
    fleet.ship_mut().ship_input_state
}

fn concerns_with(enemy: Option<ElementId>, which_turn: u16) -> [Concern; 4] {
    let mut concerns = [enemy_in_range(0); 4];
    concerns[ENEMY_SHIP_INDEX as usize].object = enemy;
    concerns[ENEMY_SHIP_INDEX as usize].which_turn = which_turn;
    concerns
}

#[test]
fn intelligence_pursues_and_ignores_incoming_fire_when_flaming() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);

            let mut concerns = concerns_with(Some(enemy), 10);
            concerns[ENEMY_WEAPON_INDEX as usize].object = Some(enemy);
            think(&mut fleet, ship, &mut concerns, 0);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].movement,
                MOVEMENT_STATE_PURSUE
            );
            assert!(concerns[ENEMY_WEAPON_INDEX as usize].object.is_none());

            // Beyond ten turns the weapon concern survives for the dodge logic.
            let mut concerns = concerns_with(Some(enemy), 11);
            concerns[ENEMY_WEAPON_INDEX as usize].object = Some(enemy);
            think(&mut fleet, ship, &mut concerns, 0);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].movement,
                MOVEMENT_STATE_PURSUE
            );
            assert_eq!(concerns[ENEMY_WEAPON_INDEX as usize].object, Some(enemy));
        }
    })
}

#[test]
fn intelligence_fires_flames_in_range_and_unwinds_the_cloak_input() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);
            let keys = (LEFT | RIGHT | SPECIAL) as u8;

            // Visible: SPECIAL is dropped, WEAPON added, steering untouched.
            paint(ship, STAMP_PRIMITIVE, WHITE);
            let mut concerns = concerns_with(Some(enemy), 6);
            let input = think(&mut fleet, ship, &mut concerns, keys);
            assert_eq!(input, (LEFT | RIGHT | WEAPON) as u8);

            // Cloaked: the source reads `&= ~LEFT | RIGHT`, which only masks off
            // LEFT, so RIGHT survives and THRUST is added.
            paint(ship, STAMPFILL_PRIMITIVE, BLACK);
            let mut concerns = concerns_with(Some(enemy), 6);
            let input = think(&mut fleet, ship, &mut concerns, keys);
            assert_eq!(input, (RIGHT | THRUST | WEAPON) as u8);
            let input = think(&mut fleet, ship, &mut concerns, LEFT as u8);
            assert_eq!(input, (THRUST | WEAPON) as u8);

            // Primitives past NUM_PRIMS also count as cloaked.
            paint(ship, EMPTY_PRIMITIVE, WHITE);
            let mut concerns = concerns_with(Some(enemy), 6);
            let input = think(&mut fleet, ship, &mut concerns, LEFT as u8);
            assert_eq!(input, (THRUST | WEAPON) as u8);
        }
    })
}

#[test]
fn intelligence_flame_window_depends_on_enemy_and_incoming_fire_timing() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, STAMP_PRIMITIVE, WHITE);
            fleet.ship_mut().special_counter = 5;
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);

            // Between seven and ten turns the flame also needs fire inbound
            // within ten turns of its own.
            let mut concerns = concerns_with(Some(enemy), 8);
            concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 10;
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), WEAPON as u8);

            let mut concerns = concerns_with(Some(enemy), 8);
            concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 11;
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), 0);

            let mut concerns = concerns_with(Some(enemy), 11);
            concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 1;
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), 0);

            let mut concerns = concerns_with(Some(enemy), 6);
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), WEAPON as u8);
        }
    })
}

#[test]
fn intelligence_cloaks_when_idle_and_ready() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, STAMP_PRIMITIVE, WHITE);
            let mut concerns = concerns_with(None, 0);

            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), SPECIAL as u8);
            // Firing and cloaking are exclusive. A running weapon counter keeps the
            // engine's own pass from clearing the key first.
            fleet.ship_mut().weapon_counter = 1;
            assert_eq!(
                think(&mut fleet, ship, &mut concerns, WEAPON as u8),
                WEAPON as u8
            );
            fleet.ship_mut().weapon_counter = 0;
            // Already cloaked: the stale SPECIAL is dropped and not re-requested.
            paint(ship, STAMPFILL_PRIMITIVE, BLACK);
            assert_eq!(think(&mut fleet, ship, &mut concerns, SPECIAL as u8), 0);
            // A running special counter leaves the input alone.
            paint(ship, STAMP_PRIMITIVE, WHITE);
            fleet.ship_mut().special_counter = 4;
            assert_eq!(
                think(&mut fleet, ship, &mut concerns, SPECIAL as u8),
                SPECIAL as u8
            );
        }
    })
}

#[test]
fn intelligence_skips_cloaking_during_the_probe_encounter() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, STAMP_PRIMITIVE, WHITE);
            let mut concerns = concerns_with(None, 0);
            let probe = uqm_engine::StoryKey::PROBE_ILWRATH_ENCOUNTER;

            // LOBYTE (CurrentActivity): CHECK_PAUSE sits in the high byte.
            let encounter = IN_ENCOUNTER | CHECK_PAUSE;
            crate::battle_context::with(|cx| cx.world.set_activity(encounter));
            crate::battle_context::with(|cx| cx.world.set_story_state(probe, 1));
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), 0);

            crate::battle_context::with(|cx| cx.world.set_story_state(probe, 0));
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), SPECIAL as u8);

            // Outside an encounter the probe flag is irrelevant.
            crate::battle_context::with(|cx| cx.world.set_story_state(probe, 1));
            crate::battle_context::with(|cx| {
                cx.world.set_activity(IN_HYPERSPACE as u16 | CHECK_PAUSE)
            });
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), SPECIAL as u8);
            crate::battle_context::with(|cx| cx.world.set_story_state(probe, 0));
        }
    })
}

#[test]
fn intelligence_runs_the_engine_pursuit_when_the_enemy_is_known() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            crate::battle_context::with(|cx| cx.world.set_activity(0));
            crate::battle_context::with(|cx| {
                uqm_engine::state::ffi::rust_init_game_state(cx.world)
            });
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, STAMP_PRIMITIVE, WHITE);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);
            battle::get_mut(enemy).mass = 1;

            // Concern entries past the first four count as empty for the engine.
            let mut concerns = concerns_with(Some(enemy), 5);
            fleet.ship_mut().ship_input_state = 0;
            battle::get_mut(ship).turn = 0;
            battle::get_mut(ship).thrust = 0;
            fleet.think(ship, &mut concerns, FIRST_EMPTY_INDEX as u16);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].movement,
                MOVEMENT_STATE_PURSUE
            );
            assert_ne!(fleet.ship_mut().ship_input_state & WEAPON as u8, 0);
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
