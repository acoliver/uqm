//! Exercise the Arilou live descriptor and registered callbacks against the linked engine.
#![cfg(feature = "linked_c_archive")]

use std::ffi::{c_void, CStr};
use std::sync::{Mutex, MutexGuard};
use std::{mem, ptr};
use uqm_rust::ships::native::{abi::*, element, put_element};

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    fn init_arilou() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    static mut DisplayArray: [PRIMITIVE; 330];
}

/// The engine's queue, display list and screen size are process globals.
static BATTLE_LOCK: Mutex<()> = Mutex::new(());

/// Mirrors `FRAME_DESC` from `drawable.h` so the engine's frame functions can
/// walk a drawable built here.
#[repr(C)]
struct FrameDesc {
    kind: u16,
    index: u16,
    hot_spot: [i16; 2],
    bounds: [i16; 2],
    image: *mut c_void,
    parent: *mut DrawableDesc,
}

/// Mirrors `DRAWABLE_DESC`.
#[repr(C)]
struct DrawableDesc {
    flags: u8,
    max_index: u16,
    frame: *mut FrameDesc,
}

struct FrameSet {
    _drawable: Box<DrawableDesc>,
    frames: Vec<FrameDesc>,
}

impl FrameSet {
    fn new(count: u16) -> Self {
        let mut drawable = Box::new(DrawableDesc {
            flags: 0,
            max_index: count - 1,
            frame: ptr::null_mut(),
        });
        let parent: *mut DrawableDesc = &mut *drawable;
        let mut frames: Vec<FrameDesc> = (0..count)
            .map(|index| FrameDesc {
                kind: 0,
                index,
                hot_spot: [0, 0],
                bounds: [0, 0],
                image: ptr::null_mut(),
                parent,
            })
            .collect();
        drawable.frame = frames.as_mut_ptr();
        Self {
            _drawable: drawable,
            frames,
        }
    }

    fn frame(&mut self, index: usize) -> FRAME {
        (&raw mut self.frames[index]).cast()
    }
}

unsafe fn index_of(frame: FRAME) -> u16 {
    (*frame.cast::<FrameDesc>()).index
}

unsafe fn prim_of(element: *mut ELEMENT) -> *mut PRIMITIVE {
    (&raw mut DisplayArray)
        .cast::<PRIMITIVE>()
        .add(usize::from((*element).PrimIndex))
}

/// An initialized element queue and display list, with the screen size the
/// engine uses to size logical space.
struct Battle {
    _guard: MutexGuard<'static, ()>,
}

impl Battle {
    unsafe fn begin() -> Self {
        Self::begin_with_screen(320, 240)
    }

    unsafe fn begin_with_screen(width: i32, height: i32) -> Self {
        let guard = BATTLE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert_ne!(
            InitQueue(&raw mut disp_q, 16, mem::size_of::<ELEMENT>() as u16),
            0
        );
        InitDisplayList();
        GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
        ScreenWidth = width;
        ScreenHeight = height;
        Self { _guard: guard }
    }
}

impl Drop for Battle {
    fn drop(&mut self) {
        unsafe {
            UninitQueue(&raw mut disp_q);
        }
    }
}

/// One player's copy of the race descriptor and its STARSHIP; ships spawned
/// from it share both.
struct Fleet {
    desc: Box<RACE_DESC>,
    starship: Box<STARSHIP>,
}

impl Fleet {
    unsafe fn new() -> Self {
        let mut desc = Box::new(*init_arilou());
        let mut starship: Box<STARSHIP> = Box::new(mem::zeroed());
        starship.RaceDescPtr = &mut *desc;
        Self { desc, starship }
    }

    /// Allocates a queued player ship element at `(x, y)` with a visible
    /// primitive, owned by this fleet.
    unsafe fn spawn_ship(&mut self, player: SIZE, x: COORD, y: COORD) -> *mut ELEMENT {
        let handle = AllocElement();
        assert!(!handle.is_null());
        let ship = element(handle);
        (*ship).playerNr = player;
        (*ship).state_flags = PLAYER_SHIP as u16;
        (*ship).__bindgen_anon_1.life_span = NORMAL_LIFE as u16;
        (*ship).current.location = POINT { x, y };
        (*ship).next.location = POINT { x, y };
        (*ship).pParent = (&mut *self.starship as *mut STARSHIP).cast();
        self.starship.playerNr = player;
        self.starship.hShip = handle;
        (*prim_of(ship)).Type = gfx_object_STAMP_PRIM as u8;
        put_element(handle);
        ship
    }
}

type Rgba = (u8, u8, u8, u8);

const KEYS: u16 = (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;

/// Draws `count` values from the engine generator after seeding it.
unsafe fn draws(seed: DWORD, count: usize) -> Vec<DWORD> {
    TFB_SeedRandom(seed);
    (0..count).map(|_| TFB_Random()).collect()
}

/// First seed whose opening draws satisfy `accept`.
unsafe fn seed_where(count: usize, accept: impl Fn(&[DWORD]) -> bool) -> DWORD {
    (1..1_000_000)
        .find(|&seed| accept(&draws(seed, count)))
        .expect("a suitable seed exists")
}

/// Seeds the generator, runs `action`, and reports how many draws it took.
unsafe fn draws_consumed(seed: DWORD, action: impl FnOnce()) -> usize {
    let sequence = draws(seed, 16);
    TFB_SeedRandom(seed);
    action();
    let next = TFB_Random();
    sequence
        .iter()
        .position(|&value| value == next)
        .expect("generator advanced by fewer than sixteen draws")
}

unsafe fn velocity_of(element: *mut ELEMENT) -> (SIZE, SIZE) {
    let (mut dx, mut dy) = (0, 0);
    GetCurrentVelocityComponents(&raw mut (*element).velocity, &mut dx, &mut dy);
    (dx, dy)
}

#[test]
fn skiff_descriptor_matches_the_c_template() {
    unsafe {
        let template = init_arilou();
        assert_eq!(template, init_arilou());
        let desc = &*template;
        assert_eq!(desc.ship_info.ship_flags, IMMEDIATE_WEAPON as u16);
        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!((desc.ship_info.crew_level, desc.ship_info.max_crew), (6, 6));
        assert_eq!(
            (desc.ship_info.energy_level, desc.ship_info.max_energy),
            (20, 20)
        );
        assert_eq!(
            CStr::from_ptr(desc.ship_info.race_strings_rsc),
            c"ship.arilou.text"
        );
        assert_eq!(
            CStr::from_ptr(desc.ship_info.icons_rsc),
            c"ship.arilou.icons"
        );
        assert_eq!(
            CStr::from_ptr(desc.ship_info.melee_icon_rsc),
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
            assert_eq!(CStr::from_ptr(data.ship_rsc[i]), expected_ship[i]);
            assert_eq!(CStr::from_ptr(data.special_rsc[i]), expected_special[i]);
            assert!(data.weapon_rsc[i].is_null());
            assert!(data.ship[i].is_null());
            assert!(data.weapon[i].is_null());
            assert!(data.special[i].is_null());
        }
        assert_eq!(
            CStr::from_ptr(data.captain_control.captain_rsc),
            c"ship.arilou.graphics.captain"
        );
        assert_eq!(CStr::from_ptr(data.victory_ditty_rsc), c"ship.arilou.ditty");
        assert_eq!(CStr::from_ptr(data.ship_sounds_rsc), c"ship.arilou.sounds");

        // LASER_RANGE >> 1, with LASER_RANGE = DISPLAY_TO_WORLD (109).
        assert_eq!(desc.cyborg_control.ManeuverabilityIndex, 0);
        assert_eq!(desc.cyborg_control.WeaponRange, 218);
        assert!(desc.cyborg_control.intelligence_func.is_some());
        assert!(desc.preprocess_func.is_some());
        assert!(desc.init_weapon_func.is_some());
        assert!(desc.postprocess_func.is_none());
        assert!(desc.uninit_func.is_none());
        assert!(desc.data.is_null());
        assert!(desc.CodeRef.is_null());
    }
}

unsafe fn fire(fleet: &mut Fleet, ship: *mut ELEMENT) -> *mut ELEMENT {
    let mut out = [ptr::null_mut()];
    assert_eq!(
        fleet.desc.init_weapon_func.unwrap()(ship, out.as_mut_ptr()),
        1
    );
    element(out[0])
}

#[test]
fn laser_snaps_to_the_enemy_within_the_turn_and_is_yellow() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut other = Fleet::new();
        // Straight along +x: ARCTAN gives angle 16, facing 4, four facings
        // from the ship's facing 0.
        other.spawn_ship(1, 2000, 1000);
        fleet.starship.ShipFacing = 0;

        let laser = fire(&mut fleet, ship);

        // Angle 16 points along +x. The beam starts nine display pixels out
        // and its end point lies LASER_RANGE (109 display pixels) away.
        assert_eq!(
            ((*laser).current.location.x, (*laser).current.location.y),
            (1000 + 9 * 4, 1000)
        );
        assert_eq!(velocity_of(laser), ((109 - 9) * 4 * 32, 0));
        assert_eq!(
            (*laser).state_flags,
            (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
        );
        assert_eq!((*laser).__bindgen_anon_1.life_span, 1);
        assert_eq!((*laser).__bindgen_anon_2.hit_points, 1);
        assert_eq!((*laser).__bindgen_anon_3.mass_points, 1);
        assert_eq!((*laser).__bindgen_anon_5.blast_offset, 1);
        assert_eq!((*laser).playerNr, 0);
        assert!((*laser).collision_func.is_some());
        assert!((*ship).hTarget.is_null());

        // BUILD_COLOR (MAKE_RGB15 (0x1F, 0x1F, 0x0A), 0x0E) on a line.
        let prim = prim_of(laser);
        assert_eq!((*prim).Type, gfx_object_LINE_PRIM as u8);
        let color: Rgba = (
            (*prim).color.r,
            (*prim).color.g,
            (*prim).color.b,
            (*prim).color.a,
        );
        assert_eq!(color, (255, 255, 82, 255));
    }
}

#[test]
fn laser_without_a_target_follows_the_ship_facing() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        fleet.starship.ShipFacing = 8;

        let laser = fire(&mut fleet, ship);

        // Facing 8 is angle 32, straight down the screen.
        assert_eq!(
            ((*laser).current.location.x, (*laser).current.location.y),
            (1000, 1000 + 9 * 4)
        );
        assert_eq!(velocity_of(laser), (0, (109 - 9) * 4 * 32));
    }
}

#[test]
fn laser_aims_exactly_at_a_target_directly_behind() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut other = Fleet::new();
        // Eight facings away: TrackShip turns a random way by one step, but
        // Arilou adds the whole delta, so the beam points straight at it.
        other.spawn_ship(1, 1000, 2000);
        fleet.starship.ShipFacing = 0;

        let seed = seed_where(1, |d| d[0] & 1 == 0);
        TFB_SeedRandom(seed);
        let laser = fire(&mut fleet, ship);
        assert_eq!(velocity_of(laser), (0, (109 - 9) * 4 * 32));

        let seed = seed_where(1, |d| d[0] & 1 == 1);
        TFB_SeedRandom(seed);
        let laser = fire(&mut fleet, ship);
        assert_eq!(velocity_of(laser), (0, (109 - 9) * 4 * 32));
    }
}

/// Runs the Arilou intelligence with no concerns for `ship_intelligence` to
/// act on, so only the Skiff's own decisions show in the input state.
unsafe fn think(
    fleet: &mut Fleet,
    ship: *mut ELEMENT,
    concerns: &mut [EVALUATE_DESC; 4],
    input: u8,
) -> u8 {
    fleet.starship.ship_input_state = input;
    fleet.desc.cyborg_control.intelligence_func.unwrap()(ship, concerns.as_mut_ptr(), 0);
    fleet.starship.ship_input_state
}

fn empty_concerns() -> [EVALUATE_DESC; 4] {
    [EVALUATE_DESC {
        ObjectPtr: ptr::null_mut(),
        facing: 0,
        which_turn: 0,
        MoveState: MOVEMENT_STATE_PURSUE,
    }; 4]
}

fn threat_concerns(threat: *mut ELEMENT, which_turn: COUNT) -> [EVALUATE_DESC; 4] {
    let mut concerns = empty_concerns();
    concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr = threat;
    concerns[ENEMY_WEAPON_INDEX as usize].which_turn = which_turn;
    concerns
}

#[test]
fn intelligence_always_thrusts_and_entices_the_enemy() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut concerns = empty_concerns();

        let input = think(&mut fleet, ship, &mut concerns, 0);
        assert_eq!(input, THRUST as u8);
        assert_eq!(
            concerns[ENEMY_SHIP_INDEX as usize].MoveState,
            MOVEMENT_STATE_ENTICE
        );
        // A stale SPECIAL is dropped when the teleporter is ready.
        let input = think(&mut fleet, ship, &mut concerns, SPECIAL as u8);
        assert_eq!(input, THRUST as u8);
        // A running teleporter counter keeps whatever SPECIAL was asked for.
        fleet.starship.special_counter = 1;
        let input = think(&mut fleet, ship, &mut concerns, SPECIAL as u8);
        assert_eq!(input, (THRUST | SPECIAL) as u8);
    }
}

#[test]
fn intelligence_teleports_from_immediate_weapons_one_time_in_four() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut other = Fleet::new();
        // A PLAYER_SHIP weapon object stands for an IMMEDIATE_WEAPON.
        let beam = other.spawn_ship(1, 1200, 1000);
        let busy = (LEFT | RIGHT | WEAPON) as u8;
        // A running weapon counter keeps the engine pass from clearing WEAPON.
        fleet.starship.weapon_counter = 1;

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
}

#[test]
fn intelligence_ignores_threats_past_six_turns_without_touching_the_generator() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut other = Fleet::new();
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
        fleet.starship.special_counter = 2;
        let mut concerns = threat_concerns(beam, 1);
        let used = draws_consumed(seed, || {
            think(&mut fleet, ship, &mut concerns, 0);
        });
        assert_eq!(used, 0);
    }
}

#[test]
fn intelligence_teleports_from_homing_weapons_only_when_close_or_fighters() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut other = Fleet::new();
        let missile = other.spawn_ship(1, 3000, 1000);
        (*missile).state_flags = 0;
        let taken = seed_where(1, |d| d[0] & 3 == 0);
        let weapon_frames = (&raw mut other.desc.ship_data.weapon).cast::<FRAME>();
        let special_frames = (&raw mut other.desc.ship_data.special).cast::<FRAME>();

        // A seeking weapon aimed through the enemy's weapon frames: only a
        // hit next turn or a crew object (fighters) counts.
        other.desc.ship_info.ship_flags = SEEKING_WEAPON as u16;
        (*missile).next.image.farray = weapon_frames;
        for (which_turn, flags, expected) in [
            (1, 0, 1),
            (2, 0, 0),
            (2, CREW_OBJECT as u16, 1),
            (6, CREW_OBJECT as u16, 1),
        ] {
            (*missile).state_flags = flags;
            let mut concerns = threat_concerns(missile, which_turn);
            let used = draws_consumed(taken, || {
                think(&mut fleet, ship, &mut concerns, 0);
            });
            assert_eq!(used, expected, "weapon turn {which_turn} flags {flags:#x}");
        }

        // The same frames do not count unless the race flags a seeking weapon.
        other.desc.ship_info.ship_flags = 0;
        (*missile).state_flags = 0;
        let mut concerns = threat_concerns(missile, 1);
        let used = draws_consumed(taken, || {
            think(&mut fleet, ship, &mut concerns, 0);
        });
        assert_eq!(used, 0);

        // Seeking specials are matched against the special frames instead.
        other.desc.ship_info.ship_flags = SEEKING_SPECIAL as u16;
        (*missile).next.image.farray = special_frames;
        let mut concerns = threat_concerns(missile, 1);
        let used = draws_consumed(taken, || {
            think(&mut fleet, ship, &mut concerns, 0);
        });
        assert_eq!(used, 1);

        // A special-seeking race's weapon frames do not match its special flag.
        (*missile).next.image.farray = weapon_frames;
        let mut concerns = threat_concerns(missile, 1);
        let used = draws_consumed(taken, || {
            think(&mut fleet, ship, &mut concerns, 0);
        });
        assert_eq!(used, 0);
    }
}

#[test]
fn intelligence_skips_objects_that_cannot_intercept() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut other = Fleet::new();
        let shell = other.spawn_ship(1, 1150, 1150);
        (*shell).state_flags = 0;
        let taken = seed_where(1, |d| d[0] & 3 == 0);

        // Neither a PLAYER_SHIP nor a homing weapon, and stationary: the
        // PlotIntercept test fails, so the generator is never consulted. The
        // positive PlotIntercept result needs drawable pixel data, which the
        // engine's frame-intersection test reads; it is covered by the engine
        // rather than here.
        assert_eq!(PlotIntercept(shell, ship, 3, 0), 0);
        let mut concerns = threat_concerns(shell, 3);
        let mut input = 0;
        let used = draws_consumed(taken, || {
            input = think(&mut fleet, ship, &mut concerns, SPECIAL as u8);
        });
        assert_eq!(used, 0);
        assert_eq!(input, THRUST as u8);
    }
}

#[test]
fn intelligence_stops_firing_when_the_skiff_cannot_afford_both() {
    unsafe {
        let _battle = Battle::begin();
        let mut fleet = Fleet::new();
        let ship = fleet.spawn_ship(0, 1000, 1000);
        let mut concerns = empty_concerns();
        // A running weapon counter keeps the engine pass from clearing WEAPON.
        fleet.starship.weapon_counter = 1;

        // SPECIAL_ENERGY_COST << 1 is six: at six the weapon is dropped.
        fleet.desc.ship_info.energy_level = 6;
        let input = think(&mut fleet, ship, &mut concerns, WEAPON as u8);
        assert_eq!(input, THRUST as u8);
        fleet.desc.ship_info.energy_level = 7;
        let input = think(&mut fleet, ship, &mut concerns, WEAPON as u8);
        assert_eq!(input, (THRUST | WEAPON) as u8);
    }
}

struct Skiff {
    fleet: Fleet,
    ship: *mut ELEMENT,
    ship_frames: FrameSet,
    warp_frames: FrameSet,
}

impl Skiff {
    /// A skiff at (100, 200) facing 6, flying on its ship frames.
    unsafe fn new() -> Self {
        let mut fleet = Fleet::new();
        let mut ship_frames = FrameSet::new(16);
        let mut warp_frames = FrameSet::new(8);
        fleet.desc.ship_data.ship[0] = ship_frames.frame(0);
        fleet.desc.ship_data.special[0] = warp_frames.frame(0);
        let ship = fleet.spawn_ship(0, 100, 200);
        fleet.starship.ShipFacing = 6;
        let ship_array = (&raw mut fleet.desc.ship_data.ship).cast::<FRAME>();
        (*ship).current.image.farray = ship_array;
        (*ship).next.image.farray = ship_array;
        (*ship).current.image.frame = ship_frames.frame(6);
        (*ship).next.image.frame = ship_frames.frame(6);
        Self {
            fleet,
            ship,
            ship_frames,
            warp_frames,
        }
    }

    unsafe fn preprocess(&self) {
        self.fleet.desc.preprocess_func.unwrap()(self.ship);
    }

    unsafe fn warp_array(&mut self) -> *mut FRAME {
        (&raw mut self.fleet.desc.ship_data.special).cast()
    }

    unsafe fn ship_array(&mut self) -> *mut FRAME {
        (&raw mut self.fleet.desc.ship_data.ship).cast()
    }
}

#[test]
fn skiff_stops_dead_unless_the_thrust_delay_is_running() {
    unsafe {
        let _battle = Battle::begin();
        let mut skiff = Skiff::new();
        let ship = skiff.ship;
        (*ship).velocity.TravelAngle = 37;
        DeltaVelocityComponents(&raw mut (*ship).velocity, 640, -320);
        skiff.fleet.starship.cur_status_flags = (SHIP_AT_MAX_SPEED | THRUST) as u16;

        // thrust_wait != 0: the engine is still coasting the thrust delay.
        (*ship).__bindgen_anon_5.thrust_wait = 3;
        skiff.preprocess();
        assert_eq!(velocity_of(ship), (640, -320));
        assert_eq!(
            skiff.fleet.starship.cur_status_flags,
            (SHIP_AT_MAX_SPEED | THRUST) as u16
        );

        // thrust_wait == 0: ZeroVelocityComponents memsets the whole record,
        // TravelAngle included.
        (*ship).__bindgen_anon_5.thrust_wait = 0;
        skiff.preprocess();
        let velocity = (*ship).velocity;
        assert_eq!(velocity.TravelAngle, 0);
        assert_eq!((velocity.vector.width, velocity.vector.height), (0, 0));
        assert_eq!((velocity.fract.width, velocity.fract.height), (0, 0));
        assert_eq!((velocity.error.width, velocity.error.height), (0, 0));
        assert_eq!((velocity.incr.width, velocity.incr.height), (0, 0));
        assert_eq!(skiff.fleet.starship.cur_status_flags, THRUST as u16);
    }
}

#[test]
fn special_key_starts_the_teleport() {
    unsafe {
        let _battle = Battle::begin();
        let mut skiff = Skiff::new();
        let ship = skiff.ship;
        (*ship).velocity.TravelAngle = 37;
        DeltaVelocityComponents(&raw mut (*ship).velocity, 640, -320);
        (*ship).__bindgen_anon_5.thrust_wait = 3;
        skiff.fleet.starship.cur_status_flags =
            (SHIP_AT_MAX_SPEED | LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;

        skiff.preprocess();

        assert_eq!(skiff.fleet.desc.ship_info.energy_level, 17);
        assert_eq!(skiff.fleet.starship.special_counter, 2);
        assert_eq!(skiff.fleet.starship.cur_status_flags, SPECIAL as u16);
        assert_eq!(velocity_of(ship), (0, 0));
        assert_eq!((*ship).velocity.TravelAngle, 0);
        assert_eq!(
            (*ship).state_flags,
            PLAYER_SHIP as u16 | (NONSOLID | FINITE_LIFE | CHANGING) as u16
        );
        assert_eq!((*ship).__bindgen_anon_1.life_span, 5);
        assert_eq!((*ship).next.image.farray, skiff.warp_array());
        assert_eq!(index_of((*ship).next.image.frame), 0);
        assert_eq!(
            (*ship).next.image.frame,
            skiff.warp_frames.frame(0),
            "the warp animation starts on its first frame"
        );
    }
}

#[test]
fn teleport_needs_the_key_a_ready_counter_and_energy() {
    unsafe {
        let _battle = Battle::begin();
        let mut skiff = Skiff::new();
        let ship = skiff.ship;

        for (flags, counter, energy) in
            [(0, 0, 20), (SPECIAL as u16, 1, 20), (SPECIAL as u16, 0, 2)]
        {
            skiff.fleet.starship.cur_status_flags = flags;
            skiff.fleet.starship.special_counter = counter;
            skiff.fleet.desc.ship_info.energy_level = energy;
            skiff.preprocess();
            assert_eq!((*ship).state_flags, PLAYER_SHIP as u16);
            assert_eq!(skiff.fleet.desc.ship_info.energy_level, energy);
            assert_eq!(skiff.fleet.starship.special_counter, counter);
            assert_eq!((*ship).next.image.farray, skiff.ship_array());
        }
    }
}

/// Puts the skiff in mid-teleport the way the key press leaves it, with the
/// engine's own life countdown applied by hand.
unsafe fn start_teleport(skiff: &mut Skiff) {
    skiff.fleet.starship.cur_status_flags = SPECIAL as u16;
    skiff.preprocess();
    assert_ne!((*skiff.ship).state_flags & NONSOLID as u16, 0);
    (*skiff.ship).state_flags &= !(CHANGING as u16);
}

unsafe fn set_life(ship: *mut ELEMENT, life: COUNT) {
    (*ship).__bindgen_anon_1.life_span = life;
}

#[test]
fn teleport_animates_jumps_at_life_three_and_returns_at_life_one() {
    unsafe {
        let _battle = Battle::begin();
        let mut skiff = Skiff::new();
        let ship = skiff.ship;
        start_teleport(&mut skiff);
        let warp = skiff.warp_frames.frame(0);
        assert_eq!((*ship).next.image.frame, warp);

        // Lives 5 and 4 play the warp frames forward.
        for (life, frame) in [(5, 1), (4, 2)] {
            set_life(ship, life);
            skiff.preprocess();
            assert_eq!(index_of((*ship).next.image.frame), frame, "life {life}");
            // The decrement is only applied to a local; the engine owns it.
            assert_eq!((*ship).__bindgen_anon_1.life_span, life);
            assert_ne!((*ship).state_flags & CHANGING as u16, 0);
            (*ship).state_flags &= !(CHANGING as u16);
        }

        // Life 3: the local count reaches two, the animation holds its frame
        // and the skiff reappears at a random place in logical space. The
        // first draw places x and the second places y.
        let seed = seed_where(3, |d| {
            // Draws whose high bits matter: the modulus is taken on the
            // low sixteen bits (a COUNT) and 7680 does not divide 65536.
            d[0] > 0xFFFF && d[1] % 7680 != (d[1] as u16 as u32) % 7680
        });
        let sequence = draws(seed, 3);
        TFB_SeedRandom(seed);
        set_life(ship, 3);
        skiff.preprocess();
        // 320x240 screen: logical space is 256 * 32 by 240 * 32.
        let expect_x = (sequence[0] as u16 as i32 % 8192) & !3;
        let expect_y = (sequence[1] as u16 as i32 % 7680) & !3;
        assert_eq!(
            ((*ship).next.location.x, (*ship).next.location.y),
            (expect_x as i16, expect_y as i16)
        );
        assert_eq!(TFB_Random(), sequence[2], "two draws, x before y");
        assert_eq!(index_of((*ship).next.image.frame), 2);
        assert_eq!((*ship).__bindgen_anon_1.life_span, 3);
        assert_ne!((*ship).state_flags & CHANGING as u16, 0);
        (*ship).state_flags &= !(CHANGING as u16);

        // Life 2: the local count is one, below two, so the frames run back.
        set_life(ship, 2);
        skiff.preprocess();
        assert_eq!(index_of((*ship).next.image.frame), 1);
        assert_eq!(
            ((*ship).next.location.x, (*ship).next.location.y),
            (expect_x as i16, expect_y as i16)
        );
        (*ship).state_flags &= !(CHANGING as u16);

        // Life 1 is NORMAL_LIFE: the teleport ends.
        (*ship).current.location = POINT { x: -9, y: 13 };
        set_life(ship, 1);
        skiff.preprocess();
        let ship_array = skiff.ship_array();
        assert_eq!(
            (*ship).state_flags,
            PLAYER_SHIP as u16 | (APPEARING | CHANGING) as u16
        );
        assert_eq!((*ship).current.image.farray, ship_array);
        assert_eq!((*ship).next.image.farray, ship_array);
        assert_eq!(index_of((*ship).current.image.frame), 6);
        assert_eq!((*ship).current.image.frame, (*ship).next.image.frame);
        assert_eq!((*ship).current.image.frame, skiff.ship_frames.frame(6));
        // InitIntersectStartPoint: WORLD_TO_DISPLAY is an arithmetic >> 2.
        let origin = (*ship).IntersectControl.IntersectStamp.origin;
        assert_eq!((origin.x, origin.y), (-3, 3));
    }
}

#[test]
fn teleport_destination_scales_with_the_screen_size() {
    unsafe {
        let _battle = Battle::begin_with_screen(640, 480);
        let mut skiff = Skiff::new();
        let ship = skiff.ship;
        start_teleport(&mut skiff);
        // 640x480: 576 * 32 = 18432 by 480 * 32 = 15360, neither of which
        // divides 65536, and both fit a COORD.
        let seed = seed_where(2, |d| {
            d[0] % 18432 != (d[0] as u16 as u32) % 18432
                && d[1] % 15360 != (d[1] as u16 as u32) % 15360
        });
        let sequence = draws(seed, 2);
        TFB_SeedRandom(seed);
        set_life(ship, 3);
        skiff.preprocess();
        let expect_x = (sequence[0] as u16 as i32 % 18432) & !3;
        let expect_y = (sequence[1] as u16 as i32 % 15360) & !3;
        assert_eq!(
            ((*ship).next.location.x, (*ship).next.location.y),
            (expect_x as i16, expect_y as i16)
        );
    }
}

#[test]
fn teleport_wraps_negative_coord_remainders_back_into_logical_space() {
    unsafe {
        // At 1280x720 the horizontal extent (1216 * 32 = 38912) exceeds
        // i16::MAX, so a remainder cast to COORD can go negative and WRAP_X
        // adds the extent back.
        let _battle = Battle::begin_with_screen(1280, 720);
        let mut skiff = Skiff::new();
        let ship = skiff.ship;
        start_teleport(&mut skiff);
        let seed = seed_where(2, |d| (d[0] as u16 as i32 % 38912) > i32::from(i16::MAX));
        let sequence = draws(seed, 2);
        TFB_SeedRandom(seed);
        set_life(ship, 3);
        skiff.preprocess();
        let remainder = sequence[0] as u16 as i32 % 38912;
        let as_coord = i32::from(remainder as i16) & !3;
        assert!(as_coord < 0);
        let expect_x = as_coord + 38912;
        assert_eq!((*ship).next.location.x, expect_x as i16);
        let expect_y = (sequence[1] as u16 as i32 % (720 * 32)) & !3;
        assert_eq!((*ship).next.location.y, expect_y as i16);
    }
}

#[test]
fn teleport_frame_ticks_every_counter_and_restores_the_pressed_keys() {
    unsafe {
        let _battle = Battle::begin();
        let mut skiff = Skiff::new();
        let ship = skiff.ship;
        start_teleport(&mut skiff);
        let starship = &mut skiff.fleet.starship;
        starship.weapon_counter = 4;
        starship.special_counter = 255;
        starship.energy_counter = 9;
        (*ship).__bindgen_anon_4.turn_wait = 255;
        (*ship).__bindgen_anon_5.thrust_wait = 7;
        starship.cur_status_flags = (LEFT | SHIP_AT_MAX_SPEED) as u16;
        starship.old_status_flags = (THRUST | SPECIAL | 0x4000) as u16;
        set_life(ship, 5);

        skiff.preprocess();

        let starship = &skiff.fleet.starship;
        assert_eq!(starship.weapon_counter, 5);
        // BYTE counters wrap rather than saturate.
        assert_eq!(starship.special_counter, 0);
        assert_eq!(starship.energy_counter, 10);
        assert_eq!((*ship).__bindgen_anon_4.turn_wait, 0);
        assert_eq!((*ship).__bindgen_anon_5.thrust_wait, 8);
        // Held keys revert to the pre-teleport state; other bits are kept.
        assert_eq!(starship.cur_status_flags & KEYS, (THRUST | SPECIAL) as u16);
        assert_ne!(starship.cur_status_flags & SHIP_AT_MAX_SPEED as u16, 0);
    }
}

#[test]
fn nonsolid_skiff_off_the_warp_frames_is_left_alone() {
    unsafe {
        let _battle = Battle::begin();
        let mut skiff = Skiff::new();
        let ship = skiff.ship;
        (*ship).state_flags |= NONSOLID as u16;
        skiff.fleet.starship.weapon_counter = 4;
        set_life(ship, 4);

        skiff.preprocess();

        assert_eq!(skiff.fleet.starship.weapon_counter, 4);
        assert_eq!((*ship).state_flags, (PLAYER_SHIP | NONSOLID) as u16);
        assert_eq!(index_of((*ship).next.image.frame), 6);
    }
}
