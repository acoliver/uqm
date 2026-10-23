//! Exercise the Pkunk descriptor and callbacks against the linked engine.

use std::{
    ffi::{c_void, CStr},
    mem, ptr, slice,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
};
use uqm_rust::ships::native::{
    abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_index, normalize_facing,
    put_element, sine,
};

unsafe extern "C" {
    fn init_pkunk() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
    fn RemoveElement(handle: HLINK);
    fn AllocStringTable(entries: i32, flags: i32) -> *mut c_void;
    fn CaptureStringTable(table: *mut c_void) -> *mut c_void;
    fn FreeStringTable(table: *mut c_void);
    static mut DisplayArray: [PRIMITIVE; 330];
}

// races.h / intel.h flags that are macros with casts rather than constants.
const STANDARD_RATING: u8 = 1 << 4;
const GOOD_RATING: u8 = 1 << 5;
const LOW_ON_ENERGY: u16 = 1 << 5;
const PLAY_VICTORY_DITTY: u16 = 1 << 9;

// LOG_SPACE_WIDTH / LOG_SPACE_HEIGHT for the 320x240 screen the tests install.
const SPACE_W: i32 = (320 - 64) * 4 * 8;
const SPACE_H: i32 = 240 * 4 * 8;

static ENGINE: Mutex<()> = Mutex::new(());

struct Battle {
    _guard: MutexGuard<'static, ()>,
    display: *mut PRIMITIVE,
    width: i32,
    height: i32,
}

impl Battle {
    fn new() -> Self {
        Self::with_capacity(64)
    }

    fn with_capacity(capacity: COUNT) -> Self {
        let guard = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            let display = GlobData.Game_state.DisplayArray;
            let width = ScreenWidth;
            let height = ScreenHeight;
            assert_ne!(
                InitQueue(&raw mut disp_q, capacity, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
                _guard: guard,
                display,
                width,
                height,
            }
        }
    }
}

impl Drop for Battle {
    fn drop(&mut self) {
        unsafe {
            UninitQueue(&raw mut disp_q);
            GlobData.Game_state.DisplayArray = self.display;
            ScreenWidth = self.width;
            ScreenHeight = self.height;
        }
    }
}

// The same engine-readable drawable layout used by the other native race tests.
#[repr(C)]
struct FrameDesc {
    kind: u16,
    index: u16,
    hot_spot: [i16; 2],
    bounds: [i16; 2],
    image: *mut c_void,
    parent: *mut DrawableDesc,
}

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
        let parent = &raw mut *drawable;
        let mut frames: Vec<_> = (0..count)
            .map(|index| FrameDesc {
                kind: 0,
                index,
                hot_spot: [0, 0],
                bounds: [4, 4],
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

    fn frame(&self, index: usize) -> FRAME {
        (&raw const self.frames[index]).cast_mut().cast()
    }
}

/// Owns a string table so `GetSoundCount` reports a real size.
struct Sounds(*mut c_void);

impl Sounds {
    unsafe fn new(count: i32) -> Self {
        Self(AllocStringTable(count, 0))
    }

    unsafe fn first(&self) -> SOUND {
        CaptureStringTable(self.0).cast()
    }
}

impl Drop for Sounds {
    fn drop(&mut self) {
        unsafe { FreeStringTable(self.0) }
    }
}

/// Mirror of `PKUNK_DATA` in pkunk.c.
#[repr(C)]
#[derive(Clone, Copy)]
struct PkunkData {
    phoenix: HELEMENT,
    saved_preprocess: ElementProcessFunc,
    saved_postprocess: ElementProcessFunc,
    saved_death: ElementProcessFunc,
}

static MARK: AtomicUsize = AtomicUsize::new(0);

// Stand-ins for the engine callbacks Pkunk saves around an explosion. Each
// body differs so the functions keep distinct addresses.
unsafe extern "C" fn engine_preprocess(_: *mut ELEMENT) {
    MARK.store(1, Ordering::SeqCst);
}
unsafe extern "C" fn engine_postprocess(_: *mut ELEMENT) {
    MARK.store(2, Ordering::SeqCst);
}
unsafe extern "C" fn engine_death(_: *mut ELEMENT) {
    MARK.store(3, Ordering::SeqCst);
}

fn addr(f: ElementProcessFunc) -> Option<usize> {
    f.map(|f| f as usize)
}

struct Ship {
    desc: Box<RACE_DESC>,
    ss: Box<STARSHIP>,
    handle: HELEMENT,
    ship_frames: FrameSet,
    weapon_frames: FrameSet,
}

impl Ship {
    unsafe fn new(player: SIZE) -> Self {
        // The engine copies the descriptor `init_pkunk` returns.
        let mut desc = Box::new(*init_pkunk());
        let ship_frames = FrameSet::new(16);
        let weapon_frames = FrameSet::new(8);
        desc.ship_data.ship = [ship_frames.frame(0); 3];
        desc.ship_data.weapon = [weapon_frames.frame(0); 3];
        let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
        ss.RaceDescPtr = &raw mut *desc;
        ss.playerNr = player;
        let handle = AllocElement();
        assert!(!handle.is_null());
        let el = element(handle);
        (*el).pParent = (&raw mut *ss).cast();
        (*el).playerNr = player;
        (*el).state_flags = PLAYER_SHIP as u16;
        (*el).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
        (*el).__bindgen_anon_2.crew_level = 8;
        (*el).__bindgen_anon_3.mass_points = 1;
        (*el).current.location = POINT { x: 4000, y: 4000 };
        (*el).current.image.farray = (&raw mut desc.ship_data.ship).cast();
        (*el).current.image.frame = ship_frames.frame(0);
        (*el).next = (*el).current;
        (*el).IntersectControl.IntersectStamp.frame = ship_frames.frame(0);
        (*prim(el)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        ss.hShip = handle;
        put_element(handle);
        Self {
            desc,
            ss,
            handle,
            ship_frames,
            weapon_frames,
        }
    }

    fn ptr(&self) -> *mut ELEMENT {
        self.handle.cast()
    }

    unsafe fn data(&self) -> *mut PkunkData {
        self.desc.data.cast()
    }

    unsafe fn at(&mut self, x: COORD, y: COORD) {
        (*self.ptr()).current.location = POINT { x, y };
        (*self.ptr()).next.location = POINT { x, y };
    }

    unsafe fn preprocess(&self) {
        (self.desc.preprocess_func.unwrap())(self.ptr());
    }

    /// A brand-new ship about to run its first preprocess.
    unsafe fn appearing(&mut self) {
        (*self.ptr()).state_flags = (PLAYER_SHIP | APPEARING) as u16;
        (*self.ptr()).hTarget = ptr::null_mut();
    }

    /// The ship `new_pkunk` leaves behind: appearing with hTarget set.
    unsafe fn reincarnating(&mut self, x: COORD, y: COORD, facing: COUNT) {
        self.at(x, y);
        self.ss.ShipFacing = facing;
        (*self.ptr()).state_flags = (PLAYER_SHIP | APPEARING) as u16;
        (*self.ptr()).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
        (*self.ptr()).hTarget = self.handle;
    }

    /// Drives the real hooks (the phoenix sees the death, then the death
    /// function explodes the ship) until the death function is `new_pkunk`.
    unsafe fn armed_new_pkunk(&mut self) -> unsafe extern "C" fn(*mut ELEMENT) {
        self.appearing();
        TFB_SeedRandom(odd_seed());
        self.preprocess();
        let phoenix = element((*self.data()).phoenix);
        self.desc.ship_info.crew_level = 0;
        ((*phoenix).death_func.unwrap())(phoenix);
        let explode = (*self.ptr()).death_func.unwrap();
        explode(self.ptr());
        (*self.ptr()).death_func.unwrap()
    }

    unsafe fn drain(&self) {
        for h in queued() {
            if h != self.handle {
                RemoveElement(h);
                FreeElement(h);
            }
        }
    }
}

impl Drop for Ship {
    fn drop(&mut self) {
        unsafe {
            if let Some(uninit) = self.desc.uninit_func {
                uninit(&raw mut *self.desc);
            }
        }
    }
}

unsafe fn prim(el: *mut ELEMENT) -> *mut PRIMITIVE {
    (&raw mut DisplayArray)
        .cast::<PRIMITIVE>()
        .add((*el).PrimIndex as usize)
}

unsafe fn queued() -> Vec<HELEMENT> {
    let mut result = Vec::new();
    let mut h = disp_q.head;
    while !h.is_null() {
        result.push(h);
        assert!(result.len() <= 64, "display queue cycle");
        h = (*element(h)).succ;
    }
    result
}

fn velocity_fields(v: &VELOCITY_DESC) -> [i32; 9] {
    [
        v.TravelAngle.into(),
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

unsafe fn rolls(seed: u32, count: usize) -> Vec<u32> {
    TFB_SeedRandom(seed);
    (0..count).map(|_| TFB_Random()).collect()
}

/// Runs `run` from `seed` and checks it consumed exactly `draws` values.
unsafe fn with_rng<T>(seed: u32, draws: usize, run: impl FnOnce() -> T) -> T {
    let expected = rolls(seed, draws + 1)[draws];
    TFB_SeedRandom(seed);
    let result = run();
    assert_eq!(TFB_Random(), expected, "RNG draws {draws}, seed {seed}");
    result
}

unsafe fn seed_with_first_roll(wanted: impl Fn(u32) -> bool) -> u32 {
    (1..100_000u32)
        .find(|&seed| wanted(rolls(seed, 1)[0]))
        .expect("a seed with the wanted first draw")
}

unsafe fn odd_seed() -> u32 {
    seed_with_first_roll(|r| r & 1 == 1)
}

unsafe fn even_seed() -> u32 {
    seed_with_first_roll(|r| r & 1 == 0)
}

/// `DISPLAY_ALIGN_X/Y` of one draw: COUNT modulus, COORD narrowing, pixel snap.
fn align(roll: u32, extent: i32) -> i32 {
    i32::from(((i32::from(roll as u16)) % extent) as i16) & !3
}

/// `WRAP_X (DISPLAY_ALIGN_X (TFB_Random ()))` spelled out the way the C
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

/// `WRAP_VAL` for a value without side effects, stored back into a COORD.
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
unsafe fn trail_origin(from: POINT, facing: u16, life_span: u16) -> POINT {
    let angle = facing_to_angle(i32::from(facing));
    let back = i32::from(life_span) - 1;
    let x = (i32::from(from.x) - i32::from(cosine(angle, 80)) * back) as i16;
    let y = (i32::from(from.y) - i32::from(sine(angle, 80)) * back) as i16;
    POINT {
        x: wrap_coord(i32::from(x), SPACE_W),
        y: wrap_coord(i32::from(y), SPACE_H),
    }
}

unsafe fn nothing_saved(ship: &Ship) -> bool {
    let data = *ship.data();
    data.saved_preprocess.is_none()
        && data.saved_postprocess.is_none()
        && data.saved_death.is_none()
}

unsafe fn block_is_zero(data: *mut c_void) -> bool {
    slice::from_raw_parts(data.cast::<u8>(), mem::size_of::<PkunkData>())
        .iter()
        .all(|&b| b == 0)
}

unsafe fn check_trail(ship: &Ship, trail: *mut ELEMENT, at: POINT, angle: u16) {
    assert_eq!((*trail).playerNr, -1);
    assert_eq!(
        (*trail).state_flags,
        (APPEARING | FINITE_LIFE | NONSOLID) as u16
    );
    assert_eq!((*trail).__bindgen_anon_1.life_span, 1);
    assert_eq!(
        (*prim(trail)).Type,
        gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM
    );
    // BUILD_COLOR (MAKE_RGB15 (0x1F, 0x15, 0x00), 0x7A)
    assert_eq!(color_bytes((*prim(trail)).color), [0xFF, 0xAD, 0x00, 0xFF]);
    assert_eq!((*trail).colorCycleIndex, 0);
    assert_eq!(
        (*trail).current.image.frame,
        (*ship.ptr()).current.image.frame
    );
    assert_eq!(
        (*trail).current.image.farray,
        (*ship.ptr()).current.image.farray
    );
    assert_eq!(
        ((*trail).current.location.x, (*trail).current.location.y),
        (at.x, at.y)
    );
    assert_eq!((*trail).__bindgen_anon_3.mass_points, angle as u8);
    assert!((*trail).preprocess_func.is_some());
    assert!((*trail).death_func.is_some());
    assert!((*trail).postprocess_func.is_none());
    assert!((*trail).collision_func.is_none());
    assert_eq!((*trail).pParent, (&raw const *ship.ss).cast_mut().cast());
}

#[test]
fn descriptor_matches_every_c_initializer_field() {
    let _battle = Battle::new();
    unsafe {
        let mut d = *init_pkunk();
        assert_eq!(
            d.ship_info.ship_flags,
            (FIRES_FORE | FIRES_LEFT | FIRES_RIGHT) as u16
        );
        assert_eq!(d.ship_info.ship_cost, 20);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (8, 8));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (12, 12));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.pkunk.text"
        );
        assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.pkunk.icons");
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.pkunk.meleeicons"
        );
        assert!(d.ship_info.race_strings.is_null());
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
                CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.pkunk.graphics.fury.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                format!("ship.pkunk.graphics.bug.{size}")
            );
            assert!(d.ship_data.special_rsc[i].is_null());
            assert!(d.ship_data.ship[i].is_null());
            assert!(d.ship_data.weapon[i].is_null());
            assert!(d.ship_data.special[i].is_null());
        }
        let captain = d.ship_data.captain_control;
        assert_eq!(
            CStr::from_ptr(captain.captain_rsc),
            c"ship.pkunk.graphics.captain"
        );
        assert!(captain.background.is_null());
        assert!(captain.turn.is_null());
        assert!(captain.thrust.is_null());
        assert!(captain.weapon.is_null());
        assert!(captain.special.is_null());
        assert_eq!(
            CStr::from_ptr(d.ship_data.victory_ditty_rsc),
            c"ship.pkunk.ditty"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_sounds_rsc),
            c"ship.pkunk.sounds"
        );
        assert!(d.ship_data.victory_ditty.is_null());
        assert!(d.ship_data.ship_sounds.is_null());
        assert_eq!(d.cyborg_control.ManeuverabilityIndex, 0);
        // CLOSE_RANGE_WEAPON + 1
        assert_eq!(d.cyborg_control.WeaponRange, 201);
        assert!(d.cyborg_control.intelligence_func.is_some());
        assert!(d.preprocess_func.is_some());
        assert!(d.postprocess_func.is_some());
        assert!(d.init_weapon_func.is_some());
        assert!(d.uninit_func.is_some());
        assert!(d.CodeRef.is_null());
        assert!(!d.data.is_null());
        assert!(block_is_zero(d.data));
        (d.uninit_func.unwrap())(&mut d);
    }
}

#[test]
fn every_init_copies_the_template_and_gives_the_copy_its_own_zeroed_block() {
    let _battle = Battle::new();
    unsafe {
        let first_ptr = init_pkunk();
        let mut first = *first_ptr;
        let second_ptr = init_pkunk();
        let mut second = *second_ptr;
        // As the C `new_pkunk_desc`: one static the caller copies each time.
        assert_eq!(first_ptr, second_ptr);
        assert!(!first.data.is_null() && !second.data.is_null());
        assert_ne!(first.data, second.data);
        assert_eq!((*second_ptr).data, second.data);
        assert!(block_is_zero(first.data) && block_is_zero(second.data));

        // Instance state lives only in the instance's block.
        (*first.data.cast::<PkunkData>()).phoenix = 0x10usize as HELEMENT;
        (*second.data.cast::<PkunkData>()).saved_death = Some(engine_death);
        assert!((*second.data.cast::<PkunkData>()).phoenix.is_null());
        assert!((*first.data.cast::<PkunkData>()).saved_death.is_none());

        // uninit frees exactly the block its descriptor owns, once.
        (first.uninit_func.unwrap())(&mut first);
        assert!(first.data.is_null());
        (first.uninit_func.unwrap())(&mut first);
        assert!(first.data.is_null());
        assert!(!second.data.is_null());
        assert_eq!(
            addr((*second.data.cast::<PkunkData>()).saved_death),
            addr(Some(engine_death))
        );
        (second.uninit_func.unwrap())(&mut second);
        assert!(second.data.is_null());

        // The template never kept an instance block: a later init starts clean.
        let mut third = *init_pkunk();
        assert!(!third.data.is_null());
        assert!(block_is_zero(third.data));
        (third.uninit_func.unwrap())(&mut third);
    }
}

#[test]
fn ships_keep_independent_phoenix_state() {
    let _battle = Battle::new();
    unsafe {
        let mut a = Ship::new(0);
        let mut b = Ship::new(1);
        assert_ne!(a.desc.data, b.desc.data);
        a.appearing();
        b.appearing();
        with_rng(odd_seed(), 1, || a.preprocess());
        assert!(!(*a.data()).phoenix.is_null());
        assert!((*b.data()).phoenix.is_null());
        // Only A's descriptor stopped asking for the one-time preprocess.
        assert!(a.desc.preprocess_func.is_none());
        assert!(b.desc.preprocess_func.is_some());
    }
}

#[test]
fn bug_missiles_launch_fore_left_and_right_inheriting_ship_velocity() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(1);
        for facing in 0..16u16 {
            for ship_speed in [0i16, 14] {
                ship.ss.ShipFacing = facing;
                ship.at(321, -654);
                SetVelocityVector(
                    &raw mut (*ship.ptr()).velocity,
                    ship_speed,
                    (facing + 3) % 16,
                );
                let (mut dx, mut dy) = (0, 0);
                GetCurrentVelocityComponents(&raw mut (*ship.ptr()).velocity, &mut dx, &mut dy);
                let mut out = [1usize as HELEMENT; 3];
                let before = queued();
                let count = with_rng(5, 0, || {
                    (ship.desc.init_weapon_func.unwrap())(ship.ptr(), out.as_mut_ptr())
                });
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
                    let m = element(h);
                    assert_eq!((*m).__bindgen_anon_1.life_span, 5);
                    assert_eq!((*m).__bindgen_anon_2.hit_points, 1);
                    assert_eq!((*m).__bindgen_anon_3.mass_points, 1);
                    assert_eq!((*m).__bindgen_anon_5.blast_offset, 1);
                    assert_eq!(
                        (*m).state_flags,
                        (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                    );
                    assert_eq!((*m).playerNr, 1);
                    assert_eq!((*m).current.image.frame, ship.weapon_frames.frame(0));
                    assert_eq!(
                        (*m).current.image.farray,
                        (&raw mut ship.desc.ship_data.weapon).cast()
                    );
                    assert_eq!((*prim(m)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
                    assert!((*m).collision_func.is_some());
                    assert!((*m).postprocess_func.is_none());
                    assert!((*m).death_func.is_none());
                    assert!((*m).pParent.is_null());
                    assert!((*m).preprocess_func.is_some());

                    // initialize_missile at speed DISPLAY_TO_WORLD (24), then the
                    // ship's own motion is added to the missile.
                    let angle = facing_to_angle(i32::from(face));
                    let vx = cosine(angle, 96 << VELOCITY_SHIFT);
                    let vy = sine(angle, 96 << VELOCITY_SHIFT);
                    let mut expected: VELOCITY_DESC = mem::zeroed();
                    SetVelocityComponents(&mut expected, vx, vy);
                    DeltaVelocityComponents(&mut expected, dx, dy);
                    assert_eq!(
                        velocity_fields(&(*m).velocity),
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
                        ((*m).current.location.x, (*m).current.location.y),
                        (x as i16, y as i16),
                        "facing {facing}, missile {i}"
                    );
                    FreeElement(h);
                }
            }
        }
    }
}

#[test]
fn bug_missile_allocation_failure_leaves_null_slots_but_still_reports_three() {
    // The ship takes one slot, so exactly one missile fits.
    let _battle = Battle::with_capacity(2);
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.ShipFacing = 6;
        let mut out = [1usize as HELEMENT; 3];
        let count = (ship.desc.init_weapon_func.unwrap())(ship.ptr(), out.as_mut_ptr());
        assert_eq!(count, 3);
        assert!(!out[0].is_null());
        assert!(out[1].is_null());
        assert!(out[2].is_null());
        assert_eq!(
            frame_index((*element(out[0])).current.image.frame),
            0,
            "the first missile is the forward one"
        );
        FreeElement(out[0]);
    }
}

#[test]
fn bug_missiles_animate_one_frame_per_blast_offset_delay() {
    let _battle = Battle::new();
    unsafe {
        let ship = Ship::new(0);
        let mut out = [ptr::null_mut(); 3];
        (ship.desc.init_weapon_func.unwrap())(ship.ptr(), out.as_mut_ptr());
        let m = element(out[0]);
        let animate = (*m).preprocess_func.unwrap();
        (*m).current.image.frame = ship.weapon_frames.frame(2);
        (*m).state_flags &= !(CHANGING as u16);
        assert_eq!((*m).__bindgen_anon_4.turn_wait, 0);
        animate(m);
        assert_eq!(frame_index((*m).next.image.frame), 3);
        assert_ne!((*m).state_flags & CHANGING as u16, 0);
        // next_turn shares its byte with the missile's blast offset (1).
        assert_eq!((*m).__bindgen_anon_4.turn_wait, 1);

        (*m).state_flags &= !(CHANGING as u16);
        (*m).next.image.frame = ship.weapon_frames.frame(6);
        animate(m);
        assert_eq!((*m).__bindgen_anon_4.turn_wait, 0);
        assert_eq!((*m).state_flags & CHANGING as u16, 0);
        assert_eq!(frame_index((*m).next.image.frame), 6);

        (*m).current.image.frame = ship.weapon_frames.frame(7);
        animate(m);
        assert_eq!(frame_index((*m).next.image.frame), 0);
        assert_ne!((*m).state_flags & CHANGING as u16, 0);
        assert_eq!((*m).__bindgen_anon_4.turn_wait, 1);
        for h in out {
            FreeElement(h);
        }
    }
}

#[test]
fn first_preprocess_with_an_even_draw_spawns_no_phoenix() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let start_status = (LEFT | THRUST | WEAPON | LOW_ON_ENERGY as u32) as u16;
        ship.appearing();
        ship.ss.cur_status_flags = start_status;
        with_rng(even_seed(), 1, || ship.preprocess());
        assert_eq!(queued(), vec![ship.handle]);
        assert!((*ship.data()).phoenix.is_null());
        // A brand-new ship is preprocessed only once.
        assert!(ship.desc.preprocess_func.is_none());
        assert_eq!(ship.ss.cur_status_flags, start_status);
        assert_ne!((*ship.ptr()).state_flags & APPEARING as u16, 0);
    }
}

#[test]
fn first_preprocess_with_an_odd_draw_queues_a_phoenix_at_the_head() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(1);
        let start_status = (LEFT | THRUST | WEAPON | LOW_ON_ENERGY as u32) as u16;
        ship.appearing();
        ship.ss.cur_status_flags = start_status;
        with_rng(odd_seed(), 1, || ship.preprocess());
        let all = queued();
        assert_eq!(all.len(), 2);
        let phoenix = element(all[0]);
        assert_eq!(all[1], ship.handle);
        assert_eq!((*ship.data()).phoenix, all[0]);
        assert_eq!((*phoenix).playerNr, 1);
        assert_eq!(
            (*phoenix).state_flags,
            (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16
        );
        assert_eq!((*phoenix).__bindgen_anon_1.life_span, 1);
        assert!((*phoenix).death_func.is_some());
        assert!((*phoenix).preprocess_func.is_none());
        assert!((*phoenix).postprocess_func.is_none());
        assert_eq!((*phoenix).pParent, (&raw mut *ship.ss).cast());
        assert!(ship.desc.preprocess_func.is_none());
        assert_eq!(ship.ss.cur_status_flags, start_status);
        // hTarget stays clear: this is not a reincarnation.
        assert!((*ship.ptr()).hTarget.is_null());
        assert_eq!(
            (*ship.ptr()).state_flags,
            (PLAYER_SHIP | APPEARING) as u16,
            "no phoenix transition on a new ship"
        );
    }
}

#[test]
fn phoenix_allocation_failure_still_consumes_the_draw_and_records_no_phoenix() {
    let _battle = Battle::with_capacity(1);
    unsafe {
        let mut ship = Ship::new(0);
        ship.appearing();
        (*ship.data()).phoenix = 0x40usize as HELEMENT;
        with_rng(odd_seed(), 1, || ship.preprocess());
        assert_eq!(queued(), vec![ship.handle]);
        assert!((*ship.data()).phoenix.is_null());
        assert!(ship.desc.preprocess_func.is_none());
    }
}

#[test]
fn reincarnation_preprocess_queues_four_trails_around_the_ship() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
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
            ship.ss.cur_status_flags = 0x3ff;
            (*ship.ptr()).velocity = mem::zeroed();
            with_rng(seed, 1, || ship.preprocess());

            let all = queued();
            assert_eq!(all.len(), 6, "phoenix, ship and four trails");
            assert_eq!(all[1], ship.handle);
            assert_eq!((*ship.data()).phoenix, all[0]);
            assert_eq!(
                (*element(all[0])).state_flags,
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16
            );

            let el = ship.ptr();
            assert_eq!((*el).__bindgen_anon_1.life_span, 12);
            assert_eq!((*prim(el)).Type, gfx_object_NUM_PRIMS as GRAPHICS_PRIM);
            assert_eq!(
                (*el).state_flags,
                (PLAYER_SHIP | APPEARING | NONSOLID | FINITE_LIFE | CHANGING) as u16
            );
            assert_eq!(ship.ss.ShipFacing, facing, "facing is restored");
            // Still reincarnating: the one-time callback stays and only the
            // input bits are cleared.
            assert!(ship.desc.preprocess_func.is_some());
            assert_eq!(
                ship.ss.cur_status_flags,
                0x3ff & !((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16)
            );
            assert_eq!(ship.data().read().phoenix, all[0]);

            for (k, &h) in all[2..].iter().enumerate() {
                let trail_facing =
                    normalize_facing(i32::from(facing) + angle_to_facing(8 + 16 * k as i32));
                let angle = facing_to_angle(i32::from(trail_facing)) as u16;
                let expected = trail_origin(POINT { x, y }, trail_facing, 12);
                check_trail(&ship, element(h), expected, angle);
            }
            ship.drain();
        }
    }
}

#[test]
fn reincarnation_trails_are_limited_by_free_elements() {
    // Ship + two trails; the phoenix draw is even so none is queued.
    let _battle = Battle::with_capacity(3);
    unsafe {
        let mut ship = Ship::new(0);
        ship.reincarnating(4000, 4000, 3);
        with_rng(even_seed(), 1, || ship.preprocess());
        let all = queued();
        assert_eq!(all.len(), 3);
        for (k, &h) in all[1..].iter().enumerate() {
            let trail_facing = normalize_facing(3 + angle_to_facing(8 + 16 * k as i32));
            let angle = facing_to_angle(i32::from(trail_facing)) as u16;
            let expected = trail_origin(POINT { x: 4000, y: 4000 }, trail_facing, 12);
            check_trail(&ship, element(h), expected, angle);
        }
        // A trail that cannot allocate keeps its callback to retry next frame.
        let first_trail = element(all[1]);
        let callback = addr((*first_trail).preprocess_func);
        ((*first_trail).preprocess_func.unwrap())(first_trail);
        assert_eq!(queued(), all);
        assert_eq!(addr((*first_trail).preprocess_func), callback);
    }
}

#[test]
fn reincarnation_ends_when_life_span_returns_to_normal() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.reincarnating(4000, 4000, 2);
        with_rng(even_seed(), 1, || ship.preprocess());
        ship.drain();
        let el = ship.ptr();

        // While the animation is running only the input bits are cleared.
        (*el).state_flags &= !(APPEARING as u16);
        (*el).__bindgen_anon_1.life_span = 5;
        ship.ss.cur_status_flags = 0x3ff;
        ship.preprocess();
        assert_eq!(ship.ss.cur_status_flags, 0x3e0);
        assert!(ship.desc.preprocess_func.is_some());
        assert_eq!((*prim(el)).Type, gfx_object_NUM_PRIMS as GRAPHICS_PRIM);

        // At NORMAL_LIFE the ship becomes solid again.
        (*el).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
        (*el).current.image.frame = ship.ship_frames.frame(5);
        (*el).next.image.frame = ship.ship_frames.frame(9);
        (*el).next.image.farray = (&raw mut ship.desc.ship_data.ship).cast();
        (*el).current.location = POINT { x: 400, y: -800 };
        (*el).next.location = POINT { x: 1201, y: 77 };
        (*el).velocity = mem::zeroed();
        SetVelocityVector(&raw mut (*el).velocity, 9, 6);
        ship.ss.cur_status_flags = 0x3ff;
        ship.preprocess();
        assert_eq!(frame_index((*el).current.image.frame), 5);
        assert_eq!(frame_index((*el).next.image.frame), 5);
        assert_eq!((*prim(el)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
        let control = (*el).IntersectControl;
        assert_eq!(
            (
                control.IntersectStamp.origin.x,
                control.IntersectStamp.origin.y
            ),
            (100, -200)
        );
        assert_eq!((control.EndPoint.x, control.EndPoint.y), (300, 19));
        // next.image.frame was overwritten before InitIntersectFrame runs.
        assert_eq!(frame_index(control.IntersectStamp.frame), 5);
        assert_eq!(velocity_fields(&(*el).velocity), [0; 9]);
        assert_eq!(
            (*el).state_flags & (NONSOLID | FINITE_LIFE) as u16,
            0,
            "solid and no longer finite"
        );
        assert_ne!((*el).state_flags & CHANGING as u16, 0);
        assert!(ship.desc.preprocess_func.is_none());
        assert_eq!(ship.ss.cur_status_flags, 0x3e0);
    }
}

#[test]
fn trails_hop_forward_until_the_ship_is_solid_again() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.reincarnating(4000, 4000, 0);
        with_rng(even_seed(), 1, || ship.preprocess());
        let first = element(queued()[1]);
        let mut parent = first;
        let mut from = (*first).current.location;
        let angle = i32::from((*first).__bindgen_anon_3.mass_points);
        for hop in 0..3 {
            let before = queued().len();
            let step = (*parent).preprocess_func.unwrap();
            step(parent);
            // The old trail stops hopping; one new trail takes its place.
            assert!((*parent).preprocess_func.is_none(), "hop {hop}");
            assert_eq!(queued().len(), before + 1);
            let child = element(*queued().last().unwrap());
            let to = POINT {
                x: (i32::from(from.x) + i32::from(cosine(angle, 80))) as i16,
                y: (i32::from(from.y) + i32::from(sine(angle, 80))) as i16,
            };
            check_trail(&ship, child, to, angle as u16);
            parent = child;
            from = to;
        }

        // Once the ship is solid the pending trail simply stops.
        (*ship.ptr()).state_flags &= !(NONSOLID as u16);
        let before = queued();
        ((*parent).preprocess_func.unwrap())(parent);
        assert!((*parent).preprocess_func.is_none());
        assert_eq!(queued(), before);
    }
}

#[test]
fn trail_death_steps_through_the_twelve_ember_colors() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.reincarnating(4000, 4000, 0);
        with_rng(even_seed(), 1, || ship.preprocess());
        let trail = element(queued()[1]);
        let death = (*trail).death_func.unwrap();
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
            (*trail).state_flags |= DISAPPEARING as u16;
            (*trail).state_flags &= !(CHANGING as u16);
            (*trail).__bindgen_anon_1.life_span = 0;
            death(trail);
            assert_eq!((*trail).colorCycleIndex as usize, index);
            assert_eq!((*trail).__bindgen_anon_1.life_span, 1);
            assert_eq!(
                color_bytes((*prim(trail)).color),
                [rgb[0], rgb[1], rgb[2], 0xFF]
            );
            assert_eq!((*trail).state_flags & DISAPPEARING as u16, 0);
            assert_ne!((*trail).state_flags & CHANGING as u16, 0);
        }

        // The twelfth step runs off the table: the trail is allowed to vanish.
        let last_color = color_bytes((*prim(trail)).color);
        (*trail).state_flags |= DISAPPEARING as u16;
        (*trail).state_flags &= !(CHANGING as u16);
        (*trail).__bindgen_anon_1.life_span = 0;
        death(trail);
        assert_eq!((*trail).colorCycleIndex, 12);
        assert_eq!((*trail).__bindgen_anon_1.life_span, 0);
        assert_ne!((*trail).state_flags & DISAPPEARING as u16, 0);
        assert_eq!((*trail).state_flags & CHANGING as u16, 0);
        assert_eq!(color_bytes((*prim(trail)).color), last_color);
    }
}

#[test]
fn phoenix_death_hook_keeps_a_living_ship_and_arms_reincarnation_for_a_dead_one() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let el = ship.ptr();
        (*el).preprocess_func = Some(engine_preprocess);
        (*el).postprocess_func = Some(engine_postprocess);
        (*el).death_func = Some(engine_death);
        ship.appearing();
        with_rng(odd_seed(), 1, || ship.preprocess());
        let phoenix = element((*ship.data()).phoenix);
        let hook = (*phoenix).death_func.unwrap();

        // Crew left: the phoenix survives another frame and nothing else moves.
        (*phoenix).state_flags |= DISAPPEARING as u16;
        (*phoenix).__bindgen_anon_1.life_span = 0;
        hook(phoenix);
        assert_eq!((*phoenix).state_flags & DISAPPEARING as u16, 0);
        assert_eq!((*phoenix).__bindgen_anon_1.life_span, 1);
        assert_eq!((*el).__bindgen_anon_3.mass_points, 1);
        assert_eq!(addr((*el).death_func), addr(Some(engine_death)));
        assert!(nothing_saved(&ship));

        // MAX_SHIP_MASS * 10 is the warp-out marker: mass 100 is 101 here.
        ship.desc.ship_info.crew_level = 0;
        for warping in [100u8, 101, 255] {
            (*el).__bindgen_anon_3.mass_points = warping;
            hook(phoenix);
            assert_eq!((*el).__bindgen_anon_3.mass_points, warping);
            assert_eq!(addr((*el).death_func), addr(Some(engine_death)));
            assert!(nothing_saved(&ship));
        }

        // Anything else is a death Pkunk walks away from.
        for mass in [0u8, 1, 10, 99] {
            (*el).__bindgen_anon_3.mass_points = mass;
            (*el).preprocess_func = Some(engine_preprocess);
            (*el).death_func = Some(engine_death);
            (*ship.data()).saved_death = None;
            hook(phoenix);
            assert_eq!((*el).__bindgen_anon_3.mass_points, 11, "mass {mass}");
            let data = *ship.data();
            assert_eq!(addr(data.saved_preprocess), addr(Some(engine_preprocess)));
            assert_eq!(addr(data.saved_postprocess), addr(Some(engine_postprocess)));
            assert_eq!(addr(data.saved_death), addr(Some(engine_death)));
            assert!((*el).death_func.is_some());
            assert_ne!(addr((*el).death_func), addr(Some(engine_death)));
        }
    }
}

#[test]
fn reincarnation_death_explodes_then_new_pkunk_restores_a_fresh_ship() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let el = ship.ptr();
        let pkunk_preprocess = ship.desc.preprocess_func.unwrap();
        (*el).preprocess_func = Some(engine_preprocess);
        (*el).postprocess_func = Some(engine_postprocess);
        (*el).death_func = Some(engine_death);
        ship.at(2000, 3000);

        // The brand-new ship spawns its phoenix, which later sees the death.
        ship.appearing();
        with_rng(odd_seed(), 1, || ship.preprocess());
        let phoenix = element((*ship.data()).phoenix);
        ship.desc.ship_info.crew_level = 0;
        ((*phoenix).death_func.unwrap())(phoenix);
        assert_eq!((*el).__bindgen_anon_3.mass_points, 11);
        let reincarnation = (*el).death_func.unwrap();

        // The armed death function explodes the ship (without ship_death).
        (*el).hTarget = ship.handle;
        (*el).state_flags |= DISAPPEARING as u16;
        SetVelocityVector(&raw mut (*el).velocity, 20, 3);
        ship.desc.ship_info.energy_level = 9;
        reincarnation(el);
        assert_eq!((*el).__bindgen_anon_1.life_span, 36);
        assert_eq!((*el).state_flags & DISAPPEARING as u16, 0);
        assert_eq!(
            (*el).state_flags & (FINITE_LIFE | NONSOLID) as u16,
            (FINITE_LIFE | NONSOLID) as u16
        );
        assert!((*el).hTarget.is_null());
        assert_eq!(velocity_fields(&(*el).velocity), [0; 9]);
        assert_eq!(ship.desc.ship_info.energy_level, 0);
        assert_ne!(addr((*el).preprocess_func), addr(Some(engine_preprocess)));
        assert_ne!(addr((*el).postprocess_func), addr(Some(engine_postprocess)));
        assert_ne!(addr((*el).death_func), addr(Some(engine_death)));
        assert_ne!(addr((*el).death_func), addr(Some(reincarnation)));

        // When the explosion ends the engine runs the new death function.
        // Leave the ship in a state a Vux-slowed, long-dead Pkunk would have.
        ship.desc.preprocess_func = None;
        ship.desc.ship_info.crew_level = 0;
        ship.desc.ship_info.energy_level = 0;
        let c = &mut ship.desc.characteristics;
        (
            c.max_thrust,
            c.thrust_increment,
            c.turn_wait,
            c.thrust_wait,
            c.special_wait,
        ) = (1, 1, 9, 9, 9);
        ship.ss.ship_input_state = 0xff;
        ship.ss.cur_status_flags = 0xffff;
        ship.ss.old_status_flags = 0xffff;
        ship.ss.energy_counter = 9;
        ship.ss.weapon_counter = 9;
        ship.ss.special_counter = 9;
        ship.ss.ShipFacing = 3;
        (*el).__bindgen_anon_2.crew_level = 7;
        (*el).__bindgen_anon_4.turn_wait = 9;
        (*el).__bindgen_anon_5.thrust_wait = 9;
        (*el).__bindgen_anon_1.life_span = 3;
        (*el).__bindgen_anon_3.mass_points = 11;
        (*el).hTarget = ptr::null_mut();
        (*prim(el)).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;

        let seed = 777;
        let draws = rolls(seed, 7);
        let (x, after_x) = expanded_coord(&draws, 1, SPACE_W);
        let (y, after_y) = expanded_coord(&draws, after_x, SPACE_H);
        assert_eq!((after_x, after_y), (4, 7), "three draws per coordinate");
        let facing = (draws[0] & 15) as u16;
        with_rng(seed, 7, || ((*el).death_func.unwrap())(el));

        assert_eq!(
            (*el).state_flags,
            (APPEARING | PLAYER_SHIP | IGNORE_SIMILAR) as u16
        );
        assert_eq!((*el).__bindgen_anon_3.mass_points, 1);
        assert_eq!(addr((*el).preprocess_func), addr(Some(engine_preprocess)));
        assert_eq!(addr((*el).postprocess_func), addr(Some(engine_postprocess)));
        assert_eq!(addr((*el).death_func), addr(Some(engine_death)));
        assert_eq!(
            addr(ship.desc.preprocess_func),
            addr(Some(pkunk_preprocess)),
            "the one-time callback is armed again"
        );
        assert_eq!(
            (
                ship.desc.ship_info.crew_level,
                ship.desc.ship_info.energy_level
            ),
            (8, 12)
        );
        let c = ship.desc.characteristics;
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
        assert_eq!(ship.ss.ship_input_state, 0);
        assert_eq!(ship.ss.cur_status_flags, PLAY_VICTORY_DITTY);
        assert_eq!(ship.ss.old_status_flags, 0);
        assert_eq!(
            (
                ship.ss.energy_counter,
                ship.ss.weapon_counter,
                ship.ss.special_counter
            ),
            (0, 0, 0)
        );
        assert_eq!((*el).__bindgen_anon_2.crew_level, 0);
        assert_eq!((*el).__bindgen_anon_4.turn_wait, 0);
        assert_eq!((*el).__bindgen_anon_5.thrust_wait, 0);
        assert_eq!((*el).__bindgen_anon_1.life_span, 1);
        assert_eq!(ship.ss.ShipFacing, facing);
        assert_eq!(
            (*el).current.image.farray,
            (&raw mut ship.desc.ship_data.ship).cast()
        );
        assert_eq!(frame_index((*el).current.image.frame), facing);
        assert_eq!((*prim(el)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
        assert_eq!(((*el).current.location.x, (*el).current.location.y), (x, y));
        assert_eq!(
            (*el).hTarget,
            ship.handle,
            "marks the next preprocess as a reincarnation"
        );
    }
}

#[test]
fn new_pkunk_redraws_its_position_while_a_planet_pulls_on_it() {
    let _battle = Battle::new();
    unsafe {
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

        let mut ship = Ship::new(0);
        let new_pkunk = ship.armed_new_pkunk();
        // GRAVITY_MASS (mass + 1): a planet sitting on the first candidate.
        let planet_handle = AllocElement();
        let planet = element(planet_handle);
        (*planet).state_flags = 0;
        (*planet).__bindgen_anon_3.mass_points = 200;
        (*planet).current.location = POINT { x: x1, y: y1 };
        (*planet).next = (*planet).current;
        (*planet).current.image.farray = (&raw mut ship.desc.ship_data.ship).cast();
        (*planet).current.image.frame = ship.ship_frames.frame(0);
        (*prim(planet)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        put_element(planet_handle);

        let el = ship.ptr();
        with_rng(seed, 13, || new_pkunk(el));
        assert_eq!(
            ((*el).current.location.x, (*el).current.location.y),
            (x2, y2),
            "the candidate on top of the planet is rejected"
        );
        assert_eq!(ship.ss.ShipFacing, (draws[0] & 15) as u16);
    }
}

#[test]
fn new_pkunk_macro_expansion_draws_again_for_a_negative_coordinate() {
    let _battle = Battle::new();
    unsafe {
        // A 1280-wide screen makes LOG_SPACE_WIDTH exceed what a COORD holds, so
        // the COORD narrowing can go negative and WRAP_VAL takes its first arm.
        ScreenWidth = 1280;
        let extent_x = (1280 - 64) * 4 * 8;
        let (seed, draws) = (1..20_000u32)
            .map(|seed| (seed, rolls(seed, 8)))
            .find(|(_, r)| align(r[1], extent_x) < 0)
            .expect("a seed with a negative first X draw");
        let (x, n) = expanded_coord(&draws, 1, extent_x);
        let (y, n) = expanded_coord(&draws, n, SPACE_H);
        assert_eq!(n, 1 + 2 + 3, "two draws for X, three for Y");
        // The second draw plus the extent is the wrapped value.
        assert_eq!(x, (align(draws[2], extent_x) + extent_x) as u16 as i16);

        let mut ship = Ship::new(0);
        let new_pkunk = ship.armed_new_pkunk();
        let el = ship.ptr();
        with_rng(seed, 6, || new_pkunk(el));
        assert_eq!(((*el).current.location.x, (*el).current.location.y), (x, y));
    }
}

#[test]
fn taunt_waits_out_its_cooldown_and_needs_the_key_and_missing_energy() {
    let _battle = Battle::new();
    unsafe {
        let sounds = Sounds::new(4);
        let mut ship = Ship::new(0);
        ship.desc.ship_data.ship_sounds = sounds.first();
        let post = ship.desc.postprocess_func.unwrap();
        ship.ss.cur_status_flags = SPECIAL as u16;
        ship.desc.ship_info.energy_level = 5;

        // The cooldown only counts down; no energy, no draw.
        ship.desc.characteristics.special_wait = 3;
        with_rng(1, 0, || post(ship.ptr()));
        assert_eq!(ship.desc.characteristics.special_wait, 2);
        assert_eq!(ship.desc.ship_info.energy_level, 5);

        // No key, or already full: nothing happens, and no draw is spent.
        ship.desc.characteristics.special_wait = 0;
        ship.ss.cur_status_flags = (LEFT | WEAPON) as u16;
        with_rng(1, 0, || post(ship.ptr()));
        ship.ss.cur_status_flags = SPECIAL as u16;
        ship.desc.ship_info.energy_level = 12;
        with_rng(1, 0, || post(ship.ptr()));
        assert_eq!(ship.desc.characteristics.special_wait, 0);
        assert_eq!(ship.desc.ship_info.energy_level, 12);

        // Taunting buys two energy (capped), arms the 16-frame cooldown and
        // goes through DeltaEnergy's bookkeeping.
        TFB_SeedRandom(31);
        for (before, after) in [(5u8, 7u8), (11, 12), (0, 2)] {
            ship.desc.characteristics.special_wait = 0;
            ship.desc.ship_info.energy_level = before;
            ship.ss.energy_counter = 9;
            ship.ss.cur_status_flags = SPECIAL as u16 | LOW_ON_ENERGY;
            ship.ss.special_counter = 77;
            post(ship.ptr());
            assert_eq!(ship.desc.ship_info.energy_level, after);
            assert_eq!(ship.desc.characteristics.special_wait, 16);
            // DeltaEnergy installs the ship's energy_wait (zero for Pkunk).
            assert_eq!(ship.ss.energy_counter, 0);
            assert_eq!(ship.ss.cur_status_flags & LOW_ON_ENERGY, 0);
            assert_eq!(ship.ss.special_counter, 77);
        }
    }
}

#[test]
fn taunt_sound_draws_avoid_the_previous_sound_and_init_forgets_it() {
    let _battle = Battle::new();
    unsafe {
        let sounds = Sounds::new(4);
        let mut ship = Ship::new(0);
        ship.desc.ship_data.ship_sounds = sounds.first();
        let post = ship.desc.postprocess_func.unwrap();
        let taunt = |ship: &mut Ship| {
            ship.desc.characteristics.special_wait = 0;
            ship.desc.ship_info.energy_level = 0;
            ship.ss.cur_status_flags = SPECIAL as u16;
            post(ship.ptr());
        };
        // Sounds 0 and 1 are reserved: with four sounds the pick is 2 or 3.
        let pick = |roll: u32| 2 + i32::from(roll as u16) % 2;

        // Over several taunts the redraws follow the remembered last sound.
        let calls = 8;
        let replay = |seed: u32| {
            TFB_SeedRandom(seed);
            let (mut last, mut draws) = (0, 0usize);
            for _ in 0..calls {
                loop {
                    let next = pick(TFB_Random());
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
        TFB_SeedRandom(seed);
        for _ in 0..calls {
            taunt(&mut ship);
        }
        assert_eq!(TFB_Random(), rolls(seed, draws + 1)[draws]);

        // Whatever sound came first is "last" for the next taunt, so the same
        // opening draw is rejected until a different sound turns up...
        let opening = rolls(seed, 16);
        let first = pick(opening[0]);
        let redraws = opening.iter().position(|&r| pick(r) != first).unwrap() + 1;
        assert!(redraws > 1);
        with_rng(seed, 1, || {
            // ...which a fresh init guarantees by forgetting the last sound.
            let mut fresh = *init_pkunk();
            (fresh.uninit_func.unwrap())(&mut fresh);
            taunt(&mut ship);
        });
        with_rng(seed, redraws, || taunt(&mut ship));
    }
}

#[test]
fn ai_drops_the_phoenix_only_for_standard_rated_pilots() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.appearing();
        with_rng(odd_seed(), 1, || ship.preprocess());
        let phoenix = (*ship.data()).phoenix;
        assert!(!phoenix.is_null());
        let intelligence = ship.desc.cyborg_control.intelligence_func.unwrap();
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        ship.desc.ship_info.energy_level = 12;

        for control in [0u8, 1 << 1, GOOD_RATING, 1 << 6] {
            ship.ss.control = control;
            with_rng(3, 0, || intelligence(ship.ptr(), concerns.as_mut_ptr(), 0));
            assert_eq!((*ship.data()).phoenix, phoenix, "control {control:#x}");
            assert_eq!(queued().len(), 2);
        }

        ship.ss.control = STANDARD_RATING | (1 << 1);
        with_rng(3, 0, || intelligence(ship.ptr(), concerns.as_mut_ptr(), 0));
        assert!((*ship.data()).phoenix.is_null());
        assert_eq!(queued(), vec![ship.handle]);
        // Nothing left to drop the second time.
        with_rng(3, 0, || intelligence(ship.ptr(), concerns.as_mut_ptr(), 0));
        assert_eq!(queued(), vec![ship.handle]);
    }
}

#[test]
fn ai_taunts_when_short_of_energy_and_throttles_on_the_low_byte_of_one_draw() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let intelligence = ship.desc.cyborg_control.intelligence_func.unwrap();
        // (BYTE) truncates before the comparison: a draw above 255 whose low
        // byte is under 20 counts.
        let low = seed_with_first_roll(|r| r & 0xff < 20 && r > 0xff);
        let high = seed_with_first_roll(|r| r & 0xff >= 20);
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        ship.ss.weapon_counter = 1;
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
                ship.desc.ship_info.energy_level = energy;
                ship.ss.special_counter = counter;
                ship.ss.ship_input_state = start | (LEFT | THRUST) as u8;
                with_rng(seed, draws, || {
                    intelligence(ship.ptr(), concerns.as_mut_ptr(), 0)
                });
                assert_eq!(
                    ship.ss.ship_input_state,
                    (LEFT | THRUST) as u8 | if taunts { SPECIAL as u8 } else { 0 },
                    "energy {energy}, counter {counter}, start {start:#x}"
                );
            }
        }
    }
}

#[test]
fn ai_finishes_with_the_shared_pass_after_deciding_on_the_taunt() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let intelligence = ship.desc.cyborg_control.intelligence_func.unwrap();
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        ship.desc.ship_info.energy_level = 4;
        ship.ss.special_counter = 0;
        // ship_intelligence clears WEAPON while the weapon counter is idle.
        for (counter, weapon_kept) in [(0u8, false), (1, true)] {
            ship.ss.weapon_counter = counter;
            ship.ss.ship_input_state = WEAPON as u8;
            with_rng(9, 0, || intelligence(ship.ptr(), concerns.as_mut_ptr(), 0));
            assert_eq!(ship.ss.ship_input_state & WEAPON as u8 != 0, weapon_kept);
            assert_ne!(ship.ss.ship_input_state & SPECIAL as u8, 0);
        }
    }
}
