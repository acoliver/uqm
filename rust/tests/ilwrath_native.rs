//! Exercise the Ilwrath live descriptor and registered callbacks against the linked engine.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{prim as prim_of, Battle, FrameSet, Ship};
use std::{ffi::CStr, ptr};
use uqm_rust::ships::native::{abi::*, element, frame_index as index_of};

unsafe fn spawn_ship(fleet: &mut Ship<'_>, x: COORD, y: COORD) -> *mut ELEMENT {
    let ship = fleet.ptr();
    (*ship).current.location = POINT { x, y };
    (*ship).next.location = POINT { x, y };
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

unsafe fn color_of(ship: *mut ELEMENT) -> Rgba {
    let color = (*prim_of(ship)).color;
    (color.r, color.g, color.b, color.a)
}

unsafe fn paint(ship: *mut ELEMENT, kind: gfx_object, (r, g, b, a): Rgba) {
    let prim = prim_of(ship);
    (*prim).Type = kind as u8;
    (*prim).color = Color { r, g, b, a };
}

unsafe fn prim_kind(ship: *mut ELEMENT) -> u8 {
    (*prim_of(ship)).Type
}

unsafe fn preprocess(fleet: &mut Ship<'_>, ship: *mut ELEMENT) {
    assert_eq!(ship, fleet.ptr());
    fleet.preprocess();
}

/// An Avenger at (1000, 1000) facing 0 with a visible primitive.
unsafe fn avenger(fleet: &mut Ship<'_>) -> *mut ELEMENT {
    let ship = spawn_ship(fleet, 1000, 1000);
    fleet.ship_mut().ShipFacing = 0;
    ship
}

#[test]
fn avenger_descriptor_matches_the_c_template() {
    battle_context::run(|| {
        unsafe {
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
                CStr::from_ptr(desc.ship_info.race_strings_rsc),
                c"ship.ilwrath.text"
            );
            assert_eq!(
                CStr::from_ptr(desc.ship_info.icons_rsc),
                c"ship.ilwrath.icons"
            );
            assert_eq!(
                CStr::from_ptr(desc.ship_info.melee_icon_rsc),
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
                assert_eq!(CStr::from_ptr(data.ship_rsc[i]), expected_ship[i]);
                assert_eq!(CStr::from_ptr(data.weapon_rsc[i]), expected_weapon[i]);
                assert!(data.special_rsc[i].is_null());
                assert!(data.ship[i].is_null());
                assert!(data.weapon[i].is_null());
                assert!(data.special[i].is_null());
            }
            assert_eq!(
                CStr::from_ptr(data.captain_control.captain_rsc),
                c"ship.ilwrath.graphics.captain"
            );
            assert_eq!(
                CStr::from_ptr(data.victory_ditty_rsc),
                c"ship.ilwrath.ditty"
            );
            assert_eq!(CStr::from_ptr(data.ship_sounds_rsc), c"ship.ilwrath.sounds");

            // CLOSE_RANGE_WEAPON: DISPLAY_TO_WORLD (50).
            assert_eq!(desc.cyborg_control.ManeuverabilityIndex, 0);
            assert_eq!(desc.cyborg_control.WeaponRange, 200);
            assert!(desc.callbacks()[4] != 0);
            assert!(desc.callbacks()[1] != 0);
            assert!(desc.callbacks()[3] != 0);
            assert!(desc.callbacks()[2] == 0);
            assert!(desc.callbacks()[0] == 0);
            assert!(desc.data.is_null());
            assert!(desc.code_ref.is_null());
        }
    })
}

#[test]
fn flame_inherits_the_ship_velocity_and_starts_on_the_first_frame() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let flame_frames = FrameSet::with_bounds(16, [0, 0]);
            fleet.descriptor_mut().ship_data.weapon[0] = flame_frames.frame(0);
            let ship = spawn_ship(&mut fleet, 400, 500);
            // Facing 4 is angle 16, along +x (angle 0 points up the screen).
            fleet.ship_mut().ShipFacing = 4;
            DeltaVelocityComponents(&raw mut (*ship).velocity, 160, -64);

            let mut out = [ptr::null_mut()];
            out[0] = fleet.weapon_handles(1)[0];
            let flame = element(out[0]);
            assert_eq!((*flame).__bindgen_anon_1.life_span, 8);
            assert_eq!((*flame).__bindgen_anon_2.hit_points, 1);
            assert_eq!((*flame).__bindgen_anon_3.mass_points, 1);
            assert_eq!((*flame).__bindgen_anon_5.next_turn, 0);
            assert_eq!((*flame).__bindgen_anon_4.turn_wait, 0);
            assert_eq!(
                (*flame).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            // MissileBlock.index is 0 whatever the facing: the flame animates
            // from its first frame.
            assert_eq!(index_of((*flame).current.image.frame), 0);
            assert!((*flame).preprocess_func.is_some());
            assert!((*flame).collision_func.is_some());

            // Muzzle at 29 display pixels, pulled back one frame of the missile's
            // own travel (25 << 5 >> 5) and one frame of the ship's drift.
            assert_eq!(
                ((*flame).current.location.x, (*flame).current.location.y),
                (400 + 29 * 4 - 25 - (160 >> 5), 500 + (64 >> 5))
            );
            let (mut dx, mut dy) = (0, 0);
            GetCurrentVelocityComponents(&raw mut (*flame).velocity, &mut dx, &mut dy);
            assert_eq!((dx, dy), (25 * 32 + 160, -64));
        }
    })
}

#[test]
fn flame_advances_its_animation_every_next_turn_frames() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let flame_frames = FrameSet::with_bounds(4, [0, 0]);
            fleet.descriptor_mut().ship_data.weapon[0] = flame_frames.frame(0);
            spawn_ship(&mut fleet, 0, 0);
            let mut out = [ptr::null_mut()];
            out[0] = fleet.weapon_handles(1)[0];
            let flame = element(out[0]);
            let step = (*flame).preprocess_func.unwrap();
            (*flame).__bindgen_anon_5.next_turn = 2;

            // turn_wait starts at 0, so the first pass animates immediately and
            // reloads the delay from next_turn.
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(step, flame);
            assert_eq!(index_of((*flame).next.image.frame), 1);
            assert_ne!((*flame).state_flags & CHANGING as u16, 0);
            assert_eq!((*flame).__bindgen_anon_4.turn_wait, 2);

            (*flame).state_flags &= !(CHANGING as u16);
            (*flame).current.image.frame = (*flame).next.image.frame;
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(step, flame);
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(step, flame);
            assert_eq!((*flame).__bindgen_anon_4.turn_wait, 0);
            assert_eq!((*flame).state_flags & CHANGING as u16, 0);
            assert_eq!(index_of((*flame).next.image.frame), 1);

            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(step, flame);
            assert_eq!(index_of((*flame).next.image.frame), 2);
            assert_ne!((*flame).state_flags & CHANGING as u16, 0);

            // The animation wraps after the last frame.
            (*flame).current.image.frame = flame_frames.frame(3);
            (*flame).__bindgen_anon_4.turn_wait = 0;
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(step, flame);
            assert_eq!(index_of((*flame).next.image.frame), 0);
        }
    })
}

#[test]
fn flame_that_hits_goes_nonsolid_without_disappearing() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            spawn_ship(&mut fleet, 400, 500);
            let mut out = [ptr::null_mut()];
            out[0] = fleet.weapon_handles(1)[0];
            let flame = element(out[0]);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let target = spawn_ship(&mut other, 410, 500);
            // A target that neither has finite life nor the normal-life marker
            // takes no damage, which keeps the collision self-contained.
            (*target).state_flags = 0;
            (*target).__bindgen_anon_1.life_span = 5;
            let mut flame_point = POINT { x: 100, y: 125 };
            let mut target_point = POINT { x: 102, y: 125 };

            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                (*flame).collision_func.unwrap(),
                flame,
                &mut flame_point,
                target,
                &mut target_point,
                uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
            );

            // weapon_collision marks it DISAPPEARING | COLLISION | NONSOLID; the
            // Ilwrath callback then undoes DISAPPEARING only.
            assert_eq!((*flame).state_flags & DISAPPEARING as u16, 0);
            assert_ne!((*flame).state_flags & NONSOLID as u16, 0);
            assert_ne!((*flame).state_flags & ELEMENT_COLLISION, 0);
            assert_eq!((*flame).__bindgen_anon_1.life_span, 0);
            assert_eq!((*flame).__bindgen_anon_2.hit_points, 0);
        }
    })
}

#[test]
fn special_key_starts_the_cloak_and_spends_energy() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;

            preprocess(&mut fleet, ship);

            assert_eq!(prim_kind(ship), gfx_object_STAMPFILL_PRIM as u8);
            assert_eq!(color_of(ship), WHITE);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 13);
            assert_eq!(fleet.ship_mut().special_counter, 13);
            assert_ne!((*ship).state_flags & CHANGING as u16, 0);
            // The key state belongs to the engine: only a local copy is cleared.
            assert_eq!(fleet.ship_mut().cur_status_flags, SPECIAL as u16);
        }
    })
}

#[test]
fn cloak_needs_energy_and_a_ready_counter() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(16);
        GlobData.Game_state.CurrentActivity = 0;
        uqm_rust::state::ffi::rust_init_game_state();
        let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
        let ship = avenger(&mut fleet);
        paint(ship, gfx_object_STAMP_PRIM, WHITE);
        fleet.ship_mut().cur_status_flags = SPECIAL as u16;

        fleet.descriptor_mut().ship_info.energy_level = 2;
        preprocess(&mut fleet, ship);
        assert_eq!(prim_kind(ship), gfx_object_STAMP_PRIM as u8);
        assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 2);
        assert_eq!(fleet.ship_mut().special_counter, 0);

        fleet.descriptor_mut().ship_info.energy_level = 16;
        fleet.ship_mut().special_counter = 3;
        preprocess(&mut fleet, ship);
        assert_eq!(prim_kind(ship), gfx_object_STAMP_PRIM as u8);
        assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 16);

        fleet.ship_mut().special_counter = 0;
        fleet.ship_mut().cur_status_flags = 0;
        preprocess(&mut fleet, ship);
        assert_eq!(prim_kind(ship), gfx_object_STAMP_PRIM as u8);
        assert_eq!((*ship).state_flags & CHANGING as u16, 0);
    })
}

#[test]
fn cloak_fades_to_black_one_step_per_frame_and_drops_pursuers() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let hunter = spawn_ship(&mut other, 3000, 1000);
            (*hunter).hTarget = ship.cast();
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;
            preprocess(&mut fleet, ship);
            assert_eq!(prim_kind(ship), gfx_object_STAMPFILL_PRIM as u8);

            // The special counter is still running, so the key only walks the
            // color down: white, then four blues, then black.
            let walk = [CYAN_LIGHT, CYAN_DARK, BLUE_LIGHT, BLUE_DARK, BLACK];
            for expected in walk {
                (*ship).state_flags &= !(CHANGING as u16);
                assert!(!(*hunter).hTarget.is_null());
                preprocess(&mut fleet, ship);
                assert_eq!(color_of(ship), expected);
                assert_ne!((*ship).state_flags & CHANGING as u16, 0);
                assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 13);
                assert_eq!(fleet.ship_mut().special_counter, 13);
            }
            // Reaching black untargets the ship.
            assert!((*hunter).hTarget.is_null());
            assert_eq!(prim_kind(ship), gfx_object_STAMPFILL_PRIM as u8);

            // Fully cloaked: nothing more happens until the counter runs out.
            (*ship).state_flags &= !(CHANGING as u16);
            preprocess(&mut fleet, ship);
            assert_eq!(color_of(ship), BLACK);
            assert_eq!((*ship).state_flags & CHANGING as u16, 0);
        }
    })
}

#[test]
fn decloak_walks_back_up_and_ends_as_a_plain_stamp() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, gfx_object_STAMPFILL_PRIM, BLACK);
            fleet.ship_mut().special_counter = 0;
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;

            // Pressing special while cloaked starts the decloak at the darkest
            // blue; it neither recloaks nor spends energy.
            preprocess(&mut fleet, ship);
            assert_eq!(color_of(ship), BLUE_DARK);
            assert_eq!(prim_kind(ship), gfx_object_STAMPFILL_PRIM as u8);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 16);
            assert_eq!(fleet.ship_mut().special_counter, 0);
            assert_ne!((*ship).state_flags & CHANGING as u16, 0);
            // The pressed key is still reported to the ship afterwards.
            assert_eq!(fleet.ship_mut().cur_status_flags, SPECIAL as u16);

            fleet.ship_mut().cur_status_flags = 0;
            for expected in [BLUE_LIGHT, CYAN_DARK, CYAN_LIGHT, WHITE] {
                (*ship).state_flags &= !(CHANGING as u16);
                preprocess(&mut fleet, ship);
                assert_eq!(color_of(ship), expected);
                assert_eq!(prim_kind(ship), gfx_object_STAMPFILL_PRIM as u8);
                assert_ne!((*ship).state_flags & CHANGING as u16, 0);
            }

            // The last step turns the fill primitive back into a normal stamp.
            (*ship).state_flags &= !(CHANGING as u16);
            preprocess(&mut fleet, ship);
            assert_eq!(prim_kind(ship), gfx_object_STAMP_PRIM as u8);
            assert_eq!(color_of(ship), WHITE);
            assert_ne!((*ship).state_flags & CHANGING as u16, 0);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 16);

            // A visible ship with no key held is left alone.
            (*ship).state_flags &= !(CHANGING as u16);
            preprocess(&mut fleet, ship);
            assert_eq!(prim_kind(ship), gfx_object_STAMP_PRIM as u8);
            assert_eq!((*ship).state_flags & CHANGING as u16, 0);
        }
    })
}

#[test]
fn firing_while_cloaked_decloaks_and_turns_toward_the_target() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship_frames = FrameSet::with_bounds(16, [0, 0]);
            let ship = avenger(&mut fleet);
            (*ship).next.image.frame = ship_frames.frame(0);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            // Straight along +x: ARCTAN gives angle 16, facing 4.
            spawn_ship(&mut other, 2000, 1000);
            paint(ship, gfx_object_STAMPFILL_PRIM, BLACK);
            fleet.ship_mut().special_counter = 9;
            fleet.ship_mut().cur_status_flags = WEAPON as u16;

            preprocess(&mut fleet, ship);

            assert_eq!(color_of(ship), BLUE_DARK);
            assert_eq!(fleet.ship_mut().ShipFacing, 4);
            assert_eq!(index_of((*ship).next.image.frame), 4);
            // turn_wait of zero is bumped so the new facing is not undone at once.
            assert_eq!((*ship).__bindgen_anon_4.turn_wait, 1);
            assert!((*ship).hTarget.is_null());
            assert_eq!(fleet.ship_mut().special_counter, 0);
            assert_ne!((*ship).state_flags & CHANGING as u16, 0);
            assert_eq!(fleet.ship_mut().cur_status_flags, WEAPON as u16);
        }
    })
}

#[test]
fn decloak_aim_leads_the_target_by_four_frames_of_motion() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship_frames = FrameSet::with_bounds(16, [0, 0]);
            let ship = avenger(&mut fleet);
            (*ship).next.image.frame = ship_frames.frame(0);
            // Moving +y by 250 world units a frame puts the ship 1000 units
            // further on after the four look-ahead frames.
            DeltaVelocityComponents(&raw mut (*ship).velocity, 0, 8000);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            spawn_ship(&mut other, 2000, 1000);
            paint(ship, gfx_object_STAMPFILL_PRIM, BLACK);
            fleet.ship_mut().cur_status_flags = WEAPON as u16;

            preprocess(&mut fleet, ship);

            // The aim point is (2000 - 1000, 1000 - 2000): angle 8, facing 2.
            // Without the look-ahead the ship would face 4.
            assert_eq!(fleet.ship_mut().ShipFacing, 2);
            assert_eq!(index_of((*ship).next.image.frame), 2);
        }
    })
}

#[test]
fn firing_while_cloaked_without_a_target_keeps_the_facing() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(16);
        GlobData.Game_state.CurrentActivity = 0;
        uqm_rust::state::ffi::rust_init_game_state();
        let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
        let ship_frames = FrameSet::with_bounds(16, [0, 0]);
        let ship = avenger(&mut fleet);
        (*ship).next.image.frame = ship_frames.frame(0);
        paint(ship, gfx_object_STAMPFILL_PRIM, BLACK);
        fleet.ship_mut().ShipFacing = 6;
        fleet.ship_mut().cur_status_flags = WEAPON as u16;

        preprocess(&mut fleet, ship);

        assert_eq!(color_of(ship), BLUE_DARK);
        assert_eq!(fleet.ship_mut().ShipFacing, 6);
        assert_eq!(index_of((*ship).next.image.frame), 0);
        assert_eq!((*ship).__bindgen_anon_4.turn_wait, 0);
        assert!((*ship).hTarget.is_null());
    })
}

#[test]
fn firing_needs_energy_to_break_the_cloak() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(16);
        GlobData.Game_state.CurrentActivity = 0;
        uqm_rust::state::ffi::rust_init_game_state();
        let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
        let ship = avenger(&mut fleet);
        paint(ship, gfx_object_STAMPFILL_PRIM, BLACK);
        fleet.ship_mut().special_counter = 5;
        fleet.ship_mut().cur_status_flags = WEAPON as u16;
        fleet.descriptor_mut().ship_info.energy_level = 0;

        preprocess(&mut fleet, ship);

        assert_eq!(color_of(ship), BLACK);
        assert_eq!(fleet.ship_mut().special_counter, 5);
        assert_eq!((*ship).state_flags & CHANGING as u16, 0);
    })
}

fn enemy_in_range(which_turn: COUNT) -> EVALUATE_DESC {
    EVALUATE_DESC {
        ObjectPtr: ptr::null_mut(),
        facing: 0,
        which_turn,
        MoveState: MOVEMENT_STATE_AVOID,
    }
}

/// Runs the Ilwrath intelligence with no concerns for `ship_intelligence` to
/// act on, so only the Avenger's own decisions show in the input state.
unsafe fn think(
    fleet: &mut Ship<'_>,
    ship: *mut ELEMENT,
    concerns: &mut [EVALUATE_DESC; 4],
    input: u8,
) -> u8 {
    fleet.ship_mut().ship_input_state = input;
    fleet.think(ship, concerns, 0);
    fleet.ship_mut().ship_input_state
}

fn concerns_with(enemy: *mut ELEMENT, which_turn: COUNT) -> [EVALUATE_DESC; 4] {
    let mut concerns = [enemy_in_range(0); 4];
    concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy;
    concerns[ENEMY_SHIP_INDEX as usize].which_turn = which_turn;
    concerns
}

#[test]
fn intelligence_pursues_and_ignores_incoming_fire_when_flaming() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);

            let mut concerns = concerns_with(enemy, 10);
            concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr = enemy;
            think(&mut fleet, ship, &mut concerns, 0);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].MoveState,
                MOVEMENT_STATE_PURSUE
            );
            assert!(concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr.is_null());

            // Beyond ten turns the weapon concern survives for the dodge logic.
            let mut concerns = concerns_with(enemy, 11);
            concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr = enemy;
            think(&mut fleet, ship, &mut concerns, 0);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].MoveState,
                MOVEMENT_STATE_PURSUE
            );
            assert_eq!(concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr, enemy);
        }
    })
}

#[test]
fn intelligence_fires_flames_in_range_and_unwinds_the_cloak_input() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);
            let keys = (LEFT | RIGHT | SPECIAL) as u8;

            // Visible: SPECIAL is dropped, WEAPON added, steering untouched.
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
            let mut concerns = concerns_with(enemy, 6);
            let input = think(&mut fleet, ship, &mut concerns, keys);
            assert_eq!(input, (LEFT | RIGHT | WEAPON) as u8);

            // Cloaked: the source reads `&= ~LEFT | RIGHT`, which only masks off
            // LEFT, so RIGHT survives and THRUST is added.
            paint(ship, gfx_object_STAMPFILL_PRIM, BLACK);
            let mut concerns = concerns_with(enemy, 6);
            let input = think(&mut fleet, ship, &mut concerns, keys);
            assert_eq!(input, (RIGHT | THRUST | WEAPON) as u8);
            let input = think(&mut fleet, ship, &mut concerns, LEFT as u8);
            assert_eq!(input, (THRUST | WEAPON) as u8);

            // Primitives past NUM_PRIMS also count as cloaked.
            paint(ship, gfx_object_NUM_PRIMS, WHITE);
            let mut concerns = concerns_with(enemy, 6);
            let input = think(&mut fleet, ship, &mut concerns, LEFT as u8);
            assert_eq!(input, (THRUST | WEAPON) as u8);
        }
    })
}

#[test]
fn intelligence_flame_window_depends_on_enemy_and_incoming_fire_timing() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
            fleet.ship_mut().special_counter = 5;
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);

            // Between seven and ten turns the flame also needs fire inbound
            // within ten turns of its own.
            let mut concerns = concerns_with(enemy, 8);
            concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 10;
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), WEAPON as u8);

            let mut concerns = concerns_with(enemy, 8);
            concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 11;
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), 0);

            let mut concerns = concerns_with(enemy, 11);
            concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 1;
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), 0);

            let mut concerns = concerns_with(enemy, 6);
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), WEAPON as u8);
        }
    })
}

#[test]
fn intelligence_cloaks_when_idle_and_ready() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
            let mut concerns = concerns_with(ptr::null_mut(), 0);

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
            paint(ship, gfx_object_STAMPFILL_PRIM, BLACK);
            assert_eq!(think(&mut fleet, ship, &mut concerns, SPECIAL as u8), 0);
            // A running special counter leaves the input alone.
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
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
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
            let mut concerns = concerns_with(ptr::null_mut(), 0);
            let probe = c"PROBE_ILWRATH_ENCOUNTER".as_ptr();

            // LOBYTE (CurrentActivity): CHECK_PAUSE sits in the high byte.
            let encounter = (IN_ENCOUNTER | CHECK_PAUSE) as u16;
            GlobData.Game_state.CurrentActivity = encounter;
            uqm_rust::state::ffi::rust_set_game_state(probe, 1);
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), 0);

            uqm_rust::state::ffi::rust_set_game_state(probe, 0);
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), SPECIAL as u8);

            // Outside an encounter the probe flag is irrelevant.
            uqm_rust::state::ffi::rust_set_game_state(probe, 1);
            GlobData.Game_state.CurrentActivity = (IN_HYPERSPACE | CHECK_PAUSE) as u16;
            assert_eq!(think(&mut fleet, ship, &mut concerns, 0), SPECIAL as u8);
            uqm_rust::state::ffi::rust_set_game_state(probe, 0);
        }
    })
}

#[test]
fn intelligence_runs_the_engine_pursuit_when_the_enemy_is_known() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            GlobData.Game_state.CurrentActivity = 0;
            uqm_rust::state::ffi::rust_init_game_state();
            let mut fleet = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 0, [16; 3], [0, 0]);
            let ship = avenger(&mut fleet);
            paint(ship, gfx_object_STAMP_PRIM, WHITE);
            let mut other = Ship::with_frames(&_battle, SPECIES_ID_ILWRATH_ID, 1, [16; 3], [0, 0]);
            let enemy = spawn_ship(&mut other, 1500, 1000);
            (*enemy).__bindgen_anon_3.mass_points = 1;

            // Concern entries past the first four count as empty for the engine.
            let mut concerns = concerns_with(enemy, 5);
            fleet.ship_mut().ship_input_state = 0;
            (*ship).__bindgen_anon_4.turn_wait = 0;
            (*ship).__bindgen_anon_5.thrust_wait = 0;
            fleet.think(ship, &mut concerns, FIRST_EMPTY_INDEX as COUNT);
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].MoveState,
                MOVEMENT_STATE_PURSUE
            );
            assert_ne!(fleet.ship_mut().ship_input_state & WEAPON as u8, 0);
        }
    })
}
