//! Exercise the Androsynth entry point and registered callbacks against the linked engine.

use std::{
    ffi::CStr,
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::{abi::*, element, put_element};

unsafe extern "C" {
    fn init_androsynth() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
    fn RemoveElement(handle: HLINK);
    static mut DisplayArray: [PRIMITIVE; 330];
}

/// The element queue and RNG are process-wide engine state.
static ENGINE: Mutex<()> = Mutex::new(());

struct Battlefield {
    _engine: MutexGuard<'static, ()>,
}

impl Battlefield {
    fn new() -> Self {
        let engine = ENGINE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            assert_ne!(
                InitQueue(&raw mut disp_q, 16, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
            // Targeting looks primitives up through the global game state.
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
        }
        Self { _engine: engine }
    }
}

impl Drop for Battlefield {
    fn drop(&mut self) {
        unsafe {
            GlobData.Game_state.DisplayArray = ptr::null_mut();
            UninitQueue(&raw mut disp_q);
        }
    }
}

/// A ship element with its own race descriptor copy, as the engine builds them.
struct Ship {
    desc: Box<RACE_DESC>,
    ss: Box<STARSHIP>,
    el: Box<ELEMENT>,
}

impl Ship {
    fn new(player: SIZE) -> Self {
        unsafe {
            let mut desc = Box::new(*init_androsynth());
            let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
            ss.RaceDescPtr = &raw mut *desc;
            ss.playerNr = player;
            let mut el: Box<ELEMENT> = Box::new(mem::zeroed());
            el.pParent = (&raw mut *ss).cast();
            el.playerNr = player;
            el.state_flags = PLAYER_SHIP as u16;
            Self { desc, ss, el }
        }
    }

    fn el(&mut self) -> *mut ELEMENT {
        &raw mut *self.el
    }

    fn ship_frames(&self) -> *mut FRAME {
        (&raw const self.desc.ship_data.ship)
            .cast::<FRAME>()
            .cast_mut()
    }

    fn special_frames(&self) -> *mut FRAME {
        (&raw const self.desc.ship_data.special)
            .cast::<FRAME>()
            .cast_mut()
    }

    fn guardian_form(mut self) -> Self {
        self.el.next.image.farray = self.ship_frames();
        self
    }

    fn blazer_form(mut self) -> Self {
        self.el.next.image.farray = self.special_frames();
        self
    }
}

unsafe extern "C" fn original_collision(
    _: *mut ELEMENT,
    _: *mut POINT,
    _: *mut ELEMENT,
    _: *mut POINT,
) {
}

fn address(func: ElementCollisionFunc) -> Option<usize> {
    func.map(|f| f as usize)
}

fn velocity_fields(v: &VELOCITY_DESC) -> [i32; 9] {
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

fn velocity_toward(speed: SIZE, facing: COUNT) -> [i32; 9] {
    unsafe {
        let mut v: VELOCITY_DESC = mem::zeroed();
        SetVelocityVector(&mut v, speed, facing);
        velocity_fields(&v)
    }
}

fn make_byte(lo: u8, hi: u8) -> u8 {
    (hi << 4) | lo
}

/// A battle target the missile can lock on to.
unsafe fn queue_enemy_ship(
    desc: &mut RACE_DESC,
    ss: &mut STARSHIP,
    location: POINT,
) -> *mut ELEMENT {
    desc.ship_info.crew_level = 5;
    ss.RaceDescPtr = desc;
    let handle = AllocElement();
    assert!(!handle.is_null());
    let enemy = element(handle);
    // Fresh elements carry NO_PRIM, which targeting treats as cloaked.
    let prim = (&raw mut DisplayArray).cast::<PRIMITIVE>();
    (*prim.add((*enemy).PrimIndex as usize)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
    (*enemy).playerNr = 1;
    (*enemy).state_flags = PLAYER_SHIP as u16;
    (*enemy).__bindgen_anon_1.life_span = 1;
    (*enemy).pParent = (ss as *mut STARSHIP).cast();
    (*enemy).current.location = location;
    (*enemy).next.location = location;
    put_element(handle);
    enemy
}

#[test]
fn descriptor_matches_the_c_race_data() {
    unsafe {
        let template = init_androsynth();
        assert_eq!(template, init_androsynth());
        let desc = *template;
        assert_eq!(
            desc.ship_info.ship_flags,
            (FIRES_FORE | SEEKING_WEAPON) as u16
        );
        assert_eq!(desc.ship_info.ship_cost, 15);
        assert_eq!(desc.ship_info.crew_level, 20);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.energy_level, 24);
        assert_eq!(desc.ship_info.max_energy, 24);
        let text = |p| CStr::from_ptr(p);
        assert_eq!(
            text(desc.ship_info.race_strings_rsc),
            c"ship.androsynth.text"
        );
        assert_eq!(text(desc.ship_info.icons_rsc), c"ship.androsynth.icons");
        assert_eq!(
            text(desc.ship_info.melee_icon_rsc),
            c"ship.androsynth.meleeicons"
        );
        assert_eq!(desc.fleet.strength, u16::MAX);
        assert_eq!(
            (desc.fleet.known_loc.x, desc.fleet.known_loc.y),
            (4999, 4999)
        );

        let c = desc.characteristics;
        assert_eq!(
            (c.max_thrust, c.thrust_increment, c.energy_regeneration),
            (24, 3, 1)
        );
        assert_eq!(
            (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
            (3, 2, 8)
        );
        assert_eq!((c.turn_wait, c.thrust_wait, c.weapon_wait), (4, 0, 0));
        assert_eq!((c.special_wait, c.ship_mass), (0, 6));

        let sizes = ["large", "medium", "small"];
        for (i, size) in sizes.iter().enumerate() {
            assert_eq!(
                text(desc.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.androsynth.graphics.guardian.{size}")
            );
            assert_eq!(
                text(desc.ship_data.weapon_rsc[i]).to_str().unwrap(),
                format!("ship.androsynth.graphics.bubble.{size}")
            );
            assert_eq!(
                text(desc.ship_data.special_rsc[i]).to_str().unwrap(),
                format!("ship.androsynth.graphics.blazer.{size}")
            );
        }
        assert_eq!(
            text(desc.ship_data.captain_control.captain_rsc),
            c"ship.androsynth.graphics.captain"
        );
        assert_eq!(
            text(desc.ship_data.victory_ditty_rsc),
            c"ship.androsynth.ditty"
        );
        assert_eq!(
            text(desc.ship_data.ship_sounds_rsc),
            c"ship.androsynth.sounds"
        );

        // LONG_RANGE_WEAPON >> 2
        assert_eq!(desc.cyborg_control.WeaponRange, 1000);
        assert_eq!(desc.cyborg_control.ManeuverabilityIndex, 0);
        assert!(desc.cyborg_control.intelligence_func.is_some());
        assert!(desc.uninit_func.is_some());
        assert!(desc.preprocess_func.is_some());
        assert!(desc.postprocess_func.is_some());
        assert!(desc.init_weapon_func.is_some());
        assert!(desc.data.is_null());
        assert!(desc.CodeRef.is_null());
    }
}

#[test]
fn bubble_is_launched_with_the_c_missile_block() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        ship.ss.ShipFacing = 4;
        ship.el.next.location = POINT { x: 1000, y: 2000 };
        let mut bubbles = [ptr::null_mut(); 1];
        let launch = ship.desc.init_weapon_func.unwrap();
        assert_eq!(launch(ship.el(), bubbles.as_mut_ptr()), 1);

        let bubble = element(bubbles[0]);
        assert!(!bubble.is_null());
        assert_eq!((*bubble).__bindgen_anon_1.life_span, 200);
        assert_eq!((*bubble).__bindgen_anon_2.hit_points, 3);
        assert_eq!((*bubble).__bindgen_anon_3.mass_points, 2);
        assert_eq!((*bubble).__bindgen_anon_4.turn_wait, 0);
        assert_eq!((*bubble).__bindgen_anon_5.blast_offset, 3);
        assert_ne!((*bubble).state_flags & IGNORE_SIMILAR as u16, 0);
        assert!((*bubble).preprocess_func.is_some());
        // DISPLAY_TO_WORLD (8) world units per tick along the ship facing.
        assert_eq!(velocity_fields(&(*bubble).velocity), velocity_toward(32, 4));
        FreeElement(bubbles[0]);
    }
}

#[test]
fn bubble_wait_nibbles_count_down_then_retarget_with_the_c_rng_order() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        let mut bubbles = [ptr::null_mut(); 1];
        let launch = ship.desc.init_weapon_func.unwrap();
        launch(ship.el(), bubbles.as_mut_ptr());
        let bubble = element(bubbles[0]);
        let preprocess = (*bubble).preprocess_func.unwrap();

        // thrust wait in the high nibble, turn wait in the low nibble.
        let launch_velocity = velocity_fields(&(*bubble).velocity);
        (*bubble).__bindgen_anon_4.turn_wait = 0x32;
        preprocess(bubble);
        assert_eq!((*bubble).__bindgen_anon_4.turn_wait, 0x21);
        preprocess(bubble);
        assert_eq!((*bubble).__bindgen_anon_4.turn_wait, 0x10);
        assert_eq!((*bubble).state_flags & CHANGING as u16, 0);
        assert_eq!(velocity_fields(&(*bubble).velocity), launch_velocity);

        // Thrust wait expires and the turn wait is zero: one draw for the new
        // thrust wait, then no target in the queue so one draw for the facing.
        (*bubble).__bindgen_anon_4.turn_wait = 0;
        TFB_SeedRandom(0x5eed);
        let thrust_draw = TFB_Random();
        let facing_draw = TFB_Random();
        TFB_SeedRandom(0x5eed);
        preprocess(bubble);
        let thrust_wait = (thrust_draw as u16 & 3) as u8;
        assert_eq!(
            (*bubble).__bindgen_anon_4.turn_wait,
            make_byte(2, thrust_wait)
        );
        assert_ne!((*bubble).state_flags & CHANGING as u16, 0);
        assert_eq!(
            velocity_fields(&(*bubble).velocity),
            velocity_toward(32, facing_draw as u16)
        );
        // Exactly two draws were consumed.
        let next_draw = TFB_Random();
        TFB_SeedRandom(0x5eed);
        TFB_Random();
        TFB_Random();
        assert_eq!(next_draw, TFB_Random());
        FreeElement(bubbles[0]);
    }
}

#[test]
fn bubble_steers_toward_the_nearest_enemy_with_a_random_wobble() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        let mut enemy_desc: Box<RACE_DESC> = Box::new(*init_androsynth());
        let mut enemy_ss: Box<STARSHIP> = Box::new(mem::zeroed());
        let mut bubbles = [ptr::null_mut(); 1];
        let launch = ship.desc.init_weapon_func.unwrap();
        launch(ship.el(), bubbles.as_mut_ptr());
        let bubble = element(bubbles[0]);
        let preprocess = (*bubble).preprocess_func.unwrap();
        (*bubble).current.location = POINT { x: 0, y: 0 };
        (*bubble).next.location = POINT { x: 0, y: 0 };

        // East of the bubble: a short turn (delta facing 4), then a positive
        // wobble. West of it: delta facing 12, then a negative wobble.
        for (location, turn_is_short) in [
            (POINT { x: 300, y: 0 }, true),
            (POINT { x: -300, y: 0 }, false),
        ] {
            let enemy = queue_enemy_ship(&mut enemy_desc, &mut enemy_ss, location);
            let mut probe = *bubble;
            let mut tracked_facing: COUNT = 0;
            let delta = TrackShip(&mut probe, &mut tracked_facing);
            assert_eq!((delta > 0 && delta < 8), turn_is_short, "delta {delta}");
            assert!(delta > 0 && delta != 8);

            (*bubble).__bindgen_anon_4.turn_wait = 0x30;
            (*bubble).hTarget = ptr::null_mut();
            TFB_SeedRandom(77);
            let wobble = TFB_Random() as u16 & 7;
            TFB_SeedRandom(77);
            preprocess(bubble);
            let facing = if turn_is_short {
                tracked_facing.wrapping_add(wobble)
            } else {
                tracked_facing.wrapping_sub(wobble)
            };
            assert_eq!((*bubble).__bindgen_anon_4.turn_wait, make_byte(2, 2));
            assert_eq!(
                velocity_fields(&(*bubble).velocity),
                velocity_toward(32, facing)
            );

            // Remove this enemy before trying the other side.
            let handle = disp_q.tail;
            assert_eq!(element(handle), enemy);
            RemoveElement(handle);
            FreeElement(handle);
        }
        FreeElement(bubbles[0]);
    }
}

/// Runs the Guardian to Blazer switch exactly as consecutive engine frames do:
/// preprocess picks the form, postprocess finishes the transformation.
unsafe fn enter_blazer(ship: &mut Ship) {
    ship.ss.cur_status_flags = (SPECIAL | WEAPON) as u16;
    let preprocess = ship.desc.preprocess_func.unwrap();
    let postprocess = ship.desc.postprocess_func.unwrap();
    preprocess(ship.el());
    postprocess(ship.el());
}

#[test]
fn special_switches_to_blazer_and_saves_the_collision_function() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        ship.el.collision_func = Some(original_collision);
        ship.el.__bindgen_anon_3.mass_points = 6;
        ship.el.__bindgen_anon_4.turn_wait = 3;
        ship.el.__bindgen_anon_5.thrust_wait = 3;

        let preprocess = ship.desc.preprocess_func.unwrap();
        ship.ss.cur_status_flags = (SPECIAL | WEAPON) as u16;
        preprocess(ship.el());
        assert_eq!(ship.el.next.image.farray, ship.special_frames());
        // Weapon is suppressed on the switching frame; special is still held.
        assert_eq!(ship.ss.cur_status_flags, SPECIAL as u16);
        assert_ne!(ship.el.state_flags & CHANGING as u16, 0);
        assert!(ship.desc.data.is_null());

        let postprocess = ship.desc.postprocess_func.unwrap();
        postprocess(ship.el());
        let c = ship.desc.characteristics;
        assert_eq!(c.energy_regeneration, 0xff, "BLAZER_DEGENERATION as BYTE");
        assert_eq!(ship.ss.energy_counter, 8);
        assert_eq!(ship.el.__bindgen_anon_4.turn_wait, 0);
        assert_eq!(ship.el.__bindgen_anon_5.thrust_wait, 0);
        assert_eq!(
            c.special_wait, 4,
            "special_wait keeps the Guardian turn_wait"
        );
        assert_eq!(c.turn_wait, 1);
        assert_eq!(ship.el.__bindgen_anon_3.mass_points, 1);

        // The engine copy owns a block that remembers the original handler.
        assert!(!ship.desc.data.is_null());
        let saved = *ship.desc.data.cast::<ElementCollisionFunc>();
        assert_eq!(address(saved), address(Some(original_collision)));
        assert!(ship.el.collision_func.is_some());
        assert_ne!(
            address(ship.el.collision_func),
            address(Some(original_collision))
        );
        assert!(
            init_androsynth().as_ref().unwrap().data.is_null(),
            "the template descriptor never owns collision data"
        );

        (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
        assert!(ship.desc.data.is_null());
        (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
        assert!(ship.desc.data.is_null());
    }
}

#[test]
fn low_energy_special_flashes_text_without_switching() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        ship.desc.ship_info.energy_level = 1;
        ship.ss.cur_status_flags = (SPECIAL | WEAPON) as u16;
        let preprocess = ship.desc.preprocess_func.unwrap();
        preprocess(ship.el());
        assert_eq!(ship.el.next.image.farray, ship.ship_frames());
        assert_eq!(ship.el.state_flags & CHANGING as u16, 0);
        // The status word written back is the one read before DeltaEnergy.
        assert_eq!(ship.ss.cur_status_flags, (SPECIAL | WEAPON) as u16);
        assert_eq!(ship.desc.ship_info.energy_level, 1);
    }
}

#[test]
fn blazer_thrusts_on_its_own_and_turns_one_facing_per_tick() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        enter_blazer(&mut ship);
        let preprocess = ship.desc.preprocess_func.unwrap();

        // Thrust wait ticks down before the blazer accelerates.
        ship.el.__bindgen_anon_5.thrust_wait = 2;
        ship.ss.cur_status_flags = (THRUST | WEAPON | SPECIAL | LEFT) as u16;
        preprocess(ship.el());
        assert_eq!(ship.el.__bindgen_anon_5.thrust_wait, 1);
        assert_eq!(ship.ss.cur_status_flags, LEFT as u16);
        assert_eq!(velocity_fields(&ship.el.velocity), [0; 9]);

        ship.el.__bindgen_anon_5.thrust_wait = 0;
        ship.ss.ShipFacing = 5;
        ship.ss.cur_status_flags = (THRUST | WEAPON | SPECIAL | LEFT) as u16;
        preprocess(ship.el());
        assert_eq!(
            ship.ss.cur_status_flags,
            (LEFT | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
        );
        assert_eq!(velocity_fields(&ship.el.velocity), velocity_toward(60, 4));

        ship.ss.ShipFacing = 15;
        ship.ss.cur_status_flags = RIGHT as u16;
        preprocess(ship.el());
        // facing 16 wraps to 0
        assert_eq!(velocity_fields(&ship.el.velocity), velocity_toward(60, 0));

        // A blocked turn keeps the facing.
        ship.el.__bindgen_anon_4.turn_wait = 1;
        ship.ss.ShipFacing = 15;
        ship.ss.cur_status_flags = RIGHT as u16;
        preprocess(ship.el());
        assert_eq!(velocity_fields(&ship.el.velocity), velocity_toward(60, 15));
    }
}

#[test]
fn blazer_clamps_turn_wait_against_vux_slowing() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        enter_blazer(&mut ship);
        ship.desc.characteristics.turn_wait = 5;
        ship.desc.characteristics.special_wait = 4;
        ship.el.__bindgen_anon_5.thrust_wait = 1;
        ship.ss.cur_status_flags = 0;
        let preprocess = ship.desc.preprocess_func.unwrap();
        preprocess(ship.el());
        assert_eq!(ship.desc.characteristics.turn_wait, 1);
        assert_eq!(ship.desc.characteristics.special_wait, 8);
    }
}

#[test]
fn exhausted_blazer_reverts_and_restores_the_saved_collision_function() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        ship.el.collision_func = Some(original_collision);
        enter_blazer(&mut ship);
        let blazer_handler = address(ship.el.collision_func);
        let blazer_farray = ship.special_frames();
        assert_eq!(ship.el.next.image.farray, blazer_farray);

        // Moving, turning and out of energy.
        SetVelocityVector(&raw mut ship.el.velocity, 60, 3);
        ship.desc.ship_info.energy_level = 0;
        ship.ss.cur_status_flags =
            (LEFT | RIGHT | THRUST | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
        ship.el.state_flags &= !(CHANGING as u16);
        let preprocess = ship.desc.preprocess_func.unwrap();
        preprocess(ship.el());

        assert_eq!(ship.el.next.image.farray, ship.ship_frames());
        assert_ne!(ship.el.state_flags & CHANGING as u16, 0);
        assert_eq!(velocity_fields(&ship.el.velocity), [0; 9]);
        assert_eq!(ship.ss.cur_status_flags, 0);
        assert_eq!(ship.desc.characteristics.turn_wait, 4);
        assert_eq!(ship.desc.characteristics.energy_regeneration, 1);
        assert_eq!(ship.el.__bindgen_anon_3.mass_points, 6);
        assert_ne!(address(ship.el.collision_func), blazer_handler);
        assert_eq!(
            address(ship.el.collision_func),
            address(Some(original_collision))
        );

        // The block stays with the descriptor until uninit frees it once.
        assert!(!ship.desc.data.is_null());
        (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
        assert!(ship.desc.data.is_null());
    }
}

#[test]
fn transforming_again_replaces_the_saved_block_instead_of_leaking_it() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).guardian_form();
        ship.el.collision_func = Some(original_collision);
        enter_blazer(&mut ship);
        let first_block = ship.desc.data;
        assert!(!first_block.is_null());

        // Revert, restore the original handler, then transform a second time.
        ship.desc.ship_info.energy_level = 0;
        ship.ss.cur_status_flags = 0;
        let preprocess = ship.desc.preprocess_func.unwrap();
        preprocess(ship.el());
        ship.desc.ship_info.energy_level = 24;
        assert_eq!(
            address(ship.el.collision_func),
            address(Some(original_collision))
        );
        enter_blazer(&mut ship);
        assert!(!ship.desc.data.is_null());
        assert_eq!(
            address(*ship.desc.data.cast::<ElementCollisionFunc>()),
            address(Some(original_collision))
        );
        (ship.desc.uninit_func.unwrap())(&raw mut *ship.desc);
        assert!(ship.desc.data.is_null());
    }
}

unsafe fn intelligence(ship: &mut Ship, concerns: &mut [EVALUATE_DESC; 4]) {
    // A zero concern count lets only the Androsynth decisions run; the shared
    // cyborg pass has nothing to evaluate.
    let ai = ship.desc.cyborg_control.intelligence_func.unwrap();
    ai(ship.el(), concerns.as_mut_ptr(), 0);
}

fn concern_slot(index: u32) -> usize {
    index as usize
}

#[test]
fn blazer_ai_ignores_crew_and_avoids_only_finite_life_weapons() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0).blazer_form();
        let mut other: ELEMENT = mem::zeroed();
        let weapon_slot = concern_slot(ENEMY_WEAPON_INDEX);

        for (flags, expect_avoid) in [
            (FINITE_LIFE as u16, true),
            ((FINITE_LIFE | CREW_OBJECT) as u16, false),
            (0, false),
        ] {
            let mut weapon: ELEMENT = mem::zeroed();
            weapon.state_flags = flags;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[concern_slot(CREW_OBJECT_INDEX)].ObjectPtr = &raw mut other;
            concerns[weapon_slot].ObjectPtr = &raw mut weapon;
            concerns[weapon_slot].MoveState = MOVEMENT_STATE_ENTICE;
            intelligence(&mut ship, &mut concerns);
            assert!(concerns[concern_slot(CREW_OBJECT_INDEX)]
                .ObjectPtr
                .is_null());
            if expect_avoid {
                assert_eq!(concerns[weapon_slot].ObjectPtr, &raw mut weapon);
                assert_eq!(concerns[weapon_slot].MoveState, MOVEMENT_STATE_AVOID);
            } else {
                assert!(concerns[weapon_slot].ObjectPtr.is_null());
            }
        }

        // Weapons that are not luring keep their state.
        let mut weapon: ELEMENT = mem::zeroed();
        weapon.state_flags = 0;
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[weapon_slot].ObjectPtr = &raw mut weapon;
        concerns[weapon_slot].MoveState = MOVEMENT_STATE_PURSUE;
        intelligence(&mut ship, &mut concerns);
        assert_eq!(concerns[weapon_slot].ObjectPtr, &raw mut weapon);
        assert_eq!(concerns[weapon_slot].MoveState, MOVEMENT_STATE_PURSUE);
    }
}

struct Opponent {
    ship: Ship,
}

impl Opponent {
    fn new(weapon_range: u16, crew: u16, maneuverability: u16) -> Self {
        let mut ship = Ship::new(1);
        ship.desc.cyborg_control.WeaponRange = weapon_range;
        ship.desc.cyborg_control.ManeuverabilityIndex = maneuverability;
        ship.el.__bindgen_anon_2.crew_level = crew;
        Self { ship }
    }
}

struct Duel {
    ship: Ship,
    enemy: Opponent,
    concerns: [EVALUATE_DESC; 4],
}

impl Duel {
    fn new(enemy: Opponent, which_turn: u16) -> Self {
        let mut duel = Self {
            ship: Ship::new(0).guardian_form(),
            enemy,
            concerns: unsafe { mem::zeroed() },
        };
        let slot = &mut duel.concerns[concern_slot(ENEMY_SHIP_INDEX)];
        slot.ObjectPtr = duel.enemy.ship.el();
        slot.which_turn = which_turn;
        duel
    }

    fn weapon_threat(mut self, threat: *mut ELEMENT, which_turn: u16) -> Self {
        let slot = &mut self.concerns[concern_slot(ENEMY_WEAPON_INDEX)];
        slot.ObjectPtr = threat;
        slot.which_turn = which_turn;
        self
    }

    fn run(&mut self) {
        unsafe { intelligence(&mut self.ship, &mut self.concerns) }
    }

    fn enemy_state(&self) -> MOVEMENT_STATE {
        self.concerns[concern_slot(ENEMY_SHIP_INDEX)].MoveState
    }

    fn input(&self) -> u8 {
        self.ship.ss.ship_input_state
    }
}

#[test]
fn guardian_ai_entices_only_under_the_c_conditions() {
    let _battle = Battlefield::new();
    let long = 1000;
    // (special counter, energy, enemy range, enemy crew, enemy maneuverability,
    //  which_turn, expected enticing)
    let cases: [(u8, u8, u16, u16, u16, u16, bool); 13] = [
        (0, 24, long, 5, 0, 10, false),
        (1, 24, long, 5, 0, 10, true),
        (1, 24, long, 5, 0, 17, false),
        (0, 7, long, 5, 0, 10, true),
        (0, 8, long, 5, 0, 10, false),
        (0, 24, 200, 4, 0, 10, true),
        (0, 24, 201, 4, 0, 10, false),
        (0, 24, 200, 3, 0, 10, false),
        (0, 24, long, 10, 26, 10, true),
        (0, 24, long, 9, 26, 10, false),
        (0, 24, long, 10, 25, 10, false),
        (0, 24, long, 10, 26, 16, true),
        (0, 24, long, 10, 26, 17, false),
    ];
    for (counter, energy, range, crew, maneuver, turn, enticing) in cases {
        let mut duel = Duel::new(Opponent::new(range, crew, maneuver), turn);
        duel.ship.ss.special_counter = counter;
        duel.ship.desc.ship_info.energy_level = energy;
        duel.run();
        let expected = if enticing {
            MOVEMENT_STATE_ENTICE
        } else {
            MOVEMENT_STATE_PURSUE
        };
        assert_eq!(
            duel.enemy_state(),
            expected,
            "counter {counter} energy {energy} range {range} crew {crew} \
             maneuver {maneuver} turn {turn}"
        );
    }
}

#[test]
fn guardian_ai_morphs_when_threatened_or_when_outranging_is_possible() {
    let _battle = Battlefield::new();
    unsafe {
        let mut missile: ELEMENT = mem::zeroed();
        let mut run = |range, crew, maneuver, turn, energy, threat: Option<u16>, preset: bool| {
            let mut duel = Duel::new(Opponent::new(range, crew, maneuver), turn);
            duel.ship.desc.ship_info.energy_level = energy;
            if preset {
                duel.ship.ss.ship_input_state = SPECIAL as u8;
            }
            if let Some(threat_turn) = threat {
                duel = duel.weapon_threat(&raw mut missile, threat_turn);
            }
            duel.run();
            duel.input() & SPECIAL as u8 != 0
        };

        // A close threat always opens the blazer.
        assert!(run(1000, 5, 0, 17, 24, Some(4), false));
        assert!(run(1000, 5, 0, 17, 7, Some(4), false));
        assert!(!run(1000, 5, 0, 17, 24, Some(5), false));
        // Enemy outranges twice our weapon range (2000).
        assert!(run(2000, 5, 0, 17, 24, None, false));
        assert!(!run(1999, 5, 0, 17, 24, None, false));
        // Without enough energy the lure is not attempted.
        assert!(!run(2000, 5, 0, 17, 7, None, false));
        // Nearby, fragile or short-ranged targets are lured below turn 16.
        assert!(run(1000, 5, 0, 15, 24, None, false));
        assert!(!run(1000, 5, 0, 16, 24, None, false));
        assert!(!run(200, 5, 0, 15, 24, None, false));
        assert!(run(200, 3, 0, 15, 24, None, false));
        assert!(!run(1000, 10, 26, 15, 24, None, false));
        assert!(run(1000, 10, 25, 15, 24, None, false));
        // A held special request is dropped when nothing justifies it.
        assert!(!run(1000, 5, 0, 17, 24, None, true));
        // And is kept while the special is cooling down.
        let mut duel = Duel::new(Opponent::new(1000, 5, 0), 17);
        duel.ship.ss.special_counter = 3;
        duel.ship.ss.ship_input_state = SPECIAL as u8;
        duel.run();
        assert_ne!(duel.input() & SPECIAL as u8, 0);
    }
}

#[test]
fn guardian_ai_fires_when_close_or_when_the_target_is_ahead_of_the_drift() {
    let _battle = Battlefield::new();
    unsafe {
        // Keep the special in cooldown so only the weapon decision runs, and
        // let the enemy hold any state other than pursuit.
        let fire = |turn: u16, enemy_north: bool, weapon_counter: u8| {
            let mut duel = Duel::new(Opponent::new(1000, 5, 0), turn);
            duel.ship.ss.special_counter = 1;
            duel.ship.ss.weapon_counter = weapon_counter;
            duel.ship.el.next.location = POINT { x: 5000, y: 5000 };
            let dy = if enemy_north { -400 } else { 400 };
            duel.enemy.ship.el.next.location = POINT {
                x: 5000,
                y: 5000 + dy,
            };
            SetVelocityVector(&raw mut duel.ship.el.velocity, 20, 0);
            duel.run();
            assert_eq!(duel.enemy_state(), MOVEMENT_STATE_ENTICE);
            duel.input() & WEAPON as u8 != 0
        };

        assert_eq!(ARCTAN(0, -400), 0);
        assert_eq!(ARCTAN(0, 400), HALF_CIRCLE as u16);

        assert!(fire(4, false, 0), "point blank");
        assert!(fire(8, true, 0), "drifting toward the target");
        assert!(!fire(8, false, 0), "drifting away from the target");
        assert!(!fire(13, true, 0), "too far for the drift rule");
        assert!(!fire(4, true, 1), "weapon cooling down");

        // The blazer request suppresses firing.
        let mut duel = Duel::new(Opponent::new(2000, 5, 0), 4);
        duel.run();
        assert_ne!(duel.input() & SPECIAL as u8, 0);
        assert_eq!(duel.input() & WEAPON as u8, 0);
    }
}
