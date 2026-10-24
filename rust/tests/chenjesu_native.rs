//! Exercise the Chenjesu descriptor and callbacks against the linked engine.

use std::{
    ffi::{c_void, CStr},
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::{
    abi::*, angle_to_facing, cosine, element, facing_to_angle, frame_index, normalize_facing,
    put_element, sine,
};

unsafe extern "C" {
    fn init_chenjesu() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
    fn RemoveElement(handle: HLINK);
    static mut DisplayArray: [PRIMITIVE; 330];
}

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

struct Ship {
    desc: Box<RACE_DESC>,
    ss: Box<STARSHIP>,
    handle: HELEMENT,
    ship_frames: FrameSet,
    weapon_frames: FrameSet,
    special_frames: FrameSet,
}

impl Ship {
    unsafe fn new(player: SIZE) -> Self {
        let mut desc = Box::new(*init_chenjesu());
        let ship_frames = FrameSet::new(16);
        let weapon_frames = FrameSet::new(10);
        let special_frames = FrameSet::new(6);
        desc.ship_data.ship = [ship_frames.frame(0); 3];
        desc.ship_data.weapon = [weapon_frames.frame(0); 3];
        desc.ship_data.special = [special_frames.frame(0); 3];
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
        (*el).__bindgen_anon_2.crew_level = 36;
        (*el).__bindgen_anon_3.mass_points = 10;
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
            special_frames,
        }
    }

    fn ptr(&self) -> *mut ELEMENT {
        self.handle.cast()
    }

    unsafe fn at(&mut self, x: COORD, y: COORD) {
        (*self.ptr()).current.location = POINT { x, y };
        (*self.ptr()).next.location = POINT { x, y };
    }

    unsafe fn crystal(&mut self) -> HELEMENT {
        let mut out = ptr::null_mut();
        assert_eq!(
            (self.desc.init_weapon_func.unwrap())(self.ptr(), &mut out),
            1
        );
        assert!(!out.is_null());
        let el = element(out);
        // Ownership is attached by ship_postprocess, not initialize_crystal.
        assert!((*el).pParent.is_null());
        (*el).pParent = (&raw mut *self.ss).cast();
        (*el).next = (*el).current;
        (*el).IntersectControl.IntersectStamp.frame = (*el).current.image.frame;
        out
    }

    unsafe fn doggy(&mut self) -> *mut ELEMENT {
        self.ss.cur_status_flags = SPECIAL as u16;
        self.ss.special_counter = 0;
        self.desc.ship_info.energy_level = 30;
        let old_tail = disp_q.tail;
        (self.desc.postprocess_func.unwrap())(self.ptr());
        let tail = disp_q.tail;
        assert_ne!(tail, old_tail);
        let dog = element(tail);
        (*dog).next = (*dog).current;
        dog
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

unsafe fn velocity_toward(speed: SIZE, facing: COUNT) -> [i32; 9] {
    let mut v = mem::zeroed();
    SetVelocityVector(&mut v, speed, facing);
    velocity_fields(&v)
}

unsafe fn with_rng<T>(seed: u32, draws: usize, run: impl FnOnce() -> T) -> T {
    TFB_SeedRandom(seed);
    for _ in 0..draws {
        TFB_Random();
    }
    let expected = TFB_Random();
    TFB_SeedRandom(seed);
    let result = run();
    assert_eq!(TFB_Random(), expected, "RNG draws {draws}, seed {seed}");
    result
}

unsafe fn intelligence(ship: &mut Ship, concerns: &mut [EVALUATE_DESC; 4]) {
    // The shared pass still clears WEAPON when ready, but has no movement
    // concerns to process. Chenjesu's own decisions inspect the array normally.
    (ship.desc.cyborg_control.intelligence_func.unwrap())(ship.ptr(), concerns.as_mut_ptr(), 0);
}

unsafe extern "C" fn sentinel(_: *mut ELEMENT) {}

#[test]
fn descriptor_matches_every_c_initializer_field() {
    let _battle = Battle::new();
    unsafe {
        let template = init_chenjesu();
        assert_eq!(template, init_chenjesu());
        let d = *template;
        assert_eq!(
            d.ship_info.ship_flags,
            (FIRES_FORE | SEEKING_SPECIAL | SEEKING_WEAPON) as u16
        );
        assert_eq!(d.ship_info.ship_cost, 28);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (36, 36));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (30, 30));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.chenjesu.text"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_info.icons_rsc),
            c"ship.chenjesu.icons"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.chenjesu.meleeicons"
        );
        assert!(d.ship_info.race_strings.is_null());
        assert!(d.ship_info.icons.is_null());
        assert!(d.ship_info.melee_icon.is_null());
        assert_eq!(d.fleet.strength, 0);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (0, 0));
        let c = d.characteristics;
        assert_eq!(
            (c.max_thrust, c.thrust_increment, c.energy_regeneration),
            (27, 3, 1)
        );
        assert_eq!(
            (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
            (5, 30, 4)
        );
        assert_eq!(
            (
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (6, 4, 0, 0, 10)
        );
        for (i, size) in ["large", "medium", "small"].iter().enumerate() {
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.chenjesu.graphics.broodhome.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                format!("ship.chenjesu.graphics.spark.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                format!("ship.chenjesu.graphics.doggy.{size}")
            );
            assert!(d.ship_data.ship[i].is_null());
            assert!(d.ship_data.weapon[i].is_null());
            assert!(d.ship_data.special[i].is_null());
        }
        let captain = d.ship_data.captain_control;
        assert_eq!(
            CStr::from_ptr(captain.captain_rsc),
            c"ship.chenjesu.graphics.captain"
        );
        assert!(captain.background.is_null());
        assert!(captain.turn.is_null());
        assert!(captain.thrust.is_null());
        assert!(captain.weapon.is_null());
        assert!(captain.special.is_null());
        assert_eq!(
            CStr::from_ptr(d.ship_data.victory_ditty_rsc),
            c"ship.chenjesu.ditty"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_sounds_rsc),
            c"ship.chenjesu.sounds"
        );
        assert!(d.ship_data.victory_ditty.is_null());
        assert!(d.ship_data.ship_sounds.is_null());
        assert_eq!(d.cyborg_control.ManeuverabilityIndex, 0);
        assert_eq!(d.cyborg_control.WeaponRange, 4000);
        assert!(d.cyborg_control.intelligence_func.is_some());
        assert!(d.preprocess_func.is_some());
        assert!(d.postprocess_func.is_some());
        assert!(d.init_weapon_func.is_some());
        assert!(d.uninit_func.is_none());
        assert!(d.data.is_null());
        assert!(d.CodeRef.is_null());

        // init reinstalls the four callbacks without resetting other fields.
        (*template).preprocess_func = None;
        (*template).postprocess_func = None;
        (*template).init_weapon_func = None;
        (*template).cyborg_control.intelligence_func = None;
        let again = init_chenjesu();
        assert_eq!(
            (*again).preprocess_func.map(|f| f as usize),
            d.preprocess_func.map(|f| f as usize)
        );
        assert_eq!(
            (*again).postprocess_func.map(|f| f as usize),
            d.postprocess_func.map(|f| f as usize)
        );
        assert_eq!(
            (*again).init_weapon_func.map(|f| f as usize),
            d.init_weapon_func.map(|f| f as usize)
        );
        assert_eq!(
            (*again)
                .cyborg_control
                .intelligence_func
                .map(|f| f as usize),
            d.cyborg_control.intelligence_func.map(|f| f as usize)
        );
    }
}

#[test]
fn crystal_launch_hold_release_and_count_truncation_match_c() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.ShipFacing = 7;
        ship.at(321, -654);
        let h = ship.crystal();
        let crystal = element(h);
        assert_eq!((*crystal).__bindgen_anon_1.life_span, 90);
        assert_eq!((*crystal).__bindgen_anon_2.hit_points, 10);
        assert_eq!((*crystal).__bindgen_anon_3.mass_points, 6);
        assert_eq!((*crystal).__bindgen_anon_5.blast_offset, 0);
        assert_eq!(
            (*crystal).state_flags,
            (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
        );
        assert_eq!((*crystal).playerNr, 0);
        assert_eq!((*crystal).current.image.frame, ship.weapon_frames.frame(0));
        assert_eq!(
            (*crystal).current.image.farray,
            (&raw mut ship.desc.ship_data.weapon).cast()
        );
        assert_eq!(
            velocity_fields(&(*crystal).velocity),
            velocity_toward(64, 7)
        );
        let angle = facing_to_angle(7);
        let vx = cosine(angle, 64 << VELOCITY_SHIFT);
        let vy = sine(angle, 64 << VELOCITY_SHIFT);
        assert_eq!(
            (*crystal).current.location.x,
            (321 + i32::from(cosine(angle, 64)) - (i32::from(vx) >> VELOCITY_SHIFT)) as i16
        );
        assert_eq!(
            (*crystal).current.location.y,
            (-654 + i32::from(sine(angle, 64)) - (i32::from(vy) >> VELOCITY_SHIFT)) as i16
        );
        assert!((*crystal).collision_func.is_some());
        assert!((*crystal).postprocess_func.is_none());
        let preprocess = (*crystal).preprocess_func.unwrap();
        ship.ss.cur_status_flags = WEAPON as u16;
        with_rng(21, 0, || preprocess(crystal));
        assert_eq!((*crystal).__bindgen_anon_1.life_span, 91);
        (*crystal).__bindgen_anon_1.life_span = u16::MAX;
        preprocess(crystal);
        assert_eq!((*crystal).__bindgen_anon_1.life_span, 0);
        ship.ss.cur_status_flags = 0;
        with_rng(21, 0, || preprocess(crystal));
        assert_eq!((*crystal).__bindgen_anon_1.life_span, 1);
        let release = (*crystal).postprocess_func.unwrap() as usize;
        ship.ss.cur_status_flags = WEAPON as u16;
        preprocess(crystal);
        assert_eq!((*crystal).__bindgen_anon_1.life_span, 2);
        assert_eq!((*crystal).postprocess_func.unwrap() as usize, release);
        FreeElement(h);
    }
}

#[test]
fn released_crystal_queues_eight_fragments_in_facing_order() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(1);
        let h = ship.crystal();
        let crystal = element(h);
        (*crystal).next.location = POINT { x: 5000, y: 6000 };
        ((*crystal).preprocess_func.unwrap())(crystal);
        let before = queued().len();
        with_rng(90, 0, || ((*crystal).postprocess_func.unwrap())(crystal));
        let all = queued();
        assert_eq!(all.len(), before + 8);
        for (i, &fragment_handle) in all[before..].iter().enumerate() {
            let fragment = element(fragment_handle);
            let face = i as u16 * 2;
            assert_eq!((*fragment).pParent, (&raw mut *ship.ss).cast());
            assert_eq!((*fragment).playerNr, 1);
            assert_eq!(
                (*fragment).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!((*fragment).__bindgen_anon_1.life_span, 10);
            assert_eq!((*fragment).__bindgen_anon_2.hit_points, 1);
            assert_eq!((*fragment).__bindgen_anon_3.mass_points, 2);
            assert_eq!((*fragment).__bindgen_anon_5.blast_offset, 2);
            assert_eq!((*fragment).current.image.frame, ship.weapon_frames.frame(1));
            assert_eq!(
                (*fragment).current.image.farray,
                (*crystal).next.image.farray
            );
            assert!((*fragment).preprocess_func.is_none());
            assert!((*fragment).postprocess_func.is_none());
            assert!((*fragment).death_func.is_none());
            assert!((*fragment).collision_func.is_some());
            assert_eq!(
                velocity_fields(&(*fragment).velocity),
                velocity_toward(64, face)
            );
            let a = facing_to_angle(i32::from(face));
            let dx = i32::from(cosine(a, 64 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
            let dy = i32::from(sine(a, 64 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
            assert_eq!(
                (
                    (*fragment).current.location.x,
                    (*fragment).current.location.y
                ),
                ((5000 - dx) as i16, (6000 - dy) as i16)
            );
            assert_eq!((*fragment).pred, all[before + i - 1]);
        }
        assert!((*element(disp_q.tail)).succ.is_null());
        FreeElement(h);
    }
}

#[test]
fn allocation_failure_preserves_weapon_count_and_partial_fragment_order() {
    let _battle = Battle::with_capacity(4);
    unsafe {
        let mut ship = Ship::new(0);
        let crystal_handle = ship.crystal();
        let crystal = element(crystal_handle);
        ((*crystal).preprocess_func.unwrap())(crystal);
        with_rng(3, 0, || ((*crystal).postprocess_func.unwrap())(crystal));
        let all = queued();
        assert_eq!(all.len(), 3);
        assert_eq!(
            velocity_fields(&(*element(all[1])).velocity),
            velocity_toward(64, 0)
        );
        assert_eq!(
            velocity_fields(&(*element(all[2])).velocity),
            velocity_toward(64, 2)
        );
        let mut out = 1usize as HELEMENT;
        assert_eq!(
            (ship.desc.init_weapon_func.unwrap())(ship.ptr(), &mut out),
            1
        );
        assert!(out.is_null());
        FreeElement(crystal_handle);
    }
}

#[test]
fn crystal_collision_creates_sparkles_at_the_target_and_animates_current_frame() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let h = ship.crystal();
        let crystal = element(h);
        let mut target: ELEMENT = mem::zeroed();
        target.playerNr = 1;
        target.current.location = POINT { x: 1234, y: -4321 };
        // Not NORMAL_LIFE: the generic weapon collision must not damage it.
        target.__bindgen_anon_1.life_span = 2;
        let mut pa = POINT { x: 20, y: 30 };
        let mut pb = POINT { x: 40, y: 50 };
        let collide = (*crystal).collision_func.unwrap();
        with_rng(17, 0, || collide(crystal, &mut pa, &mut target, &mut pb));
        let sparkle = element(disp_q.tail);
        assert_ne!(sparkle, ship.ptr());
        assert_eq!(
            ((*sparkle).current.location.x, (*sparkle).current.location.y),
            (1234, -4321)
        );
        assert_eq!((*sparkle).__bindgen_anon_1.life_span, 8);
        assert_eq!((*sparkle).__bindgen_anon_4.turn_wait, 0);
        assert_eq!((*sparkle).__bindgen_anon_5.next_turn, 0);
        assert_eq!(
            (*sparkle).current.image.farray,
            (*crystal).next.image.farray
        );
        assert_eq!((*sparkle).current.image.frame, ship.weapon_frames.frame(2));
        assert_eq!((*crystal).__bindgen_anon_1.life_span, 0);
        assert_eq!((*crystal).__bindgen_anon_2.hit_points, 0);
        assert_eq!(
            (*crystal).state_flags & (COLLISION | NONSOLID | DISAPPEARING) as u16,
            (COLLISION | NONSOLID | DISAPPEARING) as u16
        );
        let animate = (*sparkle).preprocess_func.unwrap();
        (*sparkle).next.image.frame = ship.weapon_frames.frame(8);
        with_rng(17, 0, || animate(sparkle));
        assert_eq!(frame_index((*sparkle).next.image.frame), 3);
        assert_ne!((*sparkle).state_flags & CHANGING as u16, 0);
        (*sparkle).__bindgen_anon_4.turn_wait = 2;
        (*sparkle).__bindgen_anon_5.next_turn = 3;
        (*sparkle).state_flags &= !(CHANGING as u16);
        animate(sparkle);
        assert_eq!((*sparkle).__bindgen_anon_4.turn_wait, 1);
        assert_eq!((*sparkle).state_flags & CHANGING as u16, 0);
        animate(sparkle);
        assert_eq!((*sparkle).__bindgen_anon_4.turn_wait, 0);
        (*sparkle).current.image.frame = ship.weapon_frames.frame(9);
        animate(sparkle);
        assert_eq!((*sparkle).next.image.frame, ship.weapon_frames.frame(0));
        assert_eq!((*sparkle).__bindgen_anon_4.turn_wait, 3);
        let tail = disp_q.tail;
        collide(crystal, &mut pa, &mut target, &mut pb);
        assert_eq!(
            { disp_q.tail },
            tail,
            "already-collided crystal creates no second blast"
        );
        FreeElement(h);
    }
}

#[test]
fn doggy_spawns_behind_ship_with_exact_state_and_coordinate_truncation() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        for facing in [0, 4, 7, 8, 15, u16::MAX] {
            ship.ss.ShipFacing = facing;
            ship.at(i16::MAX - 10, i16::MIN + 10);
            let dog = with_rng(7, 0, || ship.doggy());
            let angle = (facing_to_angle(i32::from(facing)) + HALF_CIRCLE as i32) as COUNT;
            let expected_facing = normalize_facing(angle_to_facing(i32::from(angle)));
            assert_eq!((*dog).playerNr, 0);
            assert_eq!((*dog).state_flags, APPEARING as u16);
            assert_eq!((*dog).__bindgen_anon_1.life_span, 1);
            assert_eq!((*dog).__bindgen_anon_2.hit_points, 3);
            assert_eq!((*dog).__bindgen_anon_3.mass_points, 4);
            assert_eq!((*dog).__bindgen_anon_5.thrust_wait, 0);
            assert_eq!((*prim(dog)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
            assert_eq!(
                (*dog).current.image.farray,
                (&raw mut ship.desc.ship_data.special).cast()
            );
            assert_eq!((*dog).current.image.frame, ship.special_frames.frame(0));
            assert_eq!((*dog).pParent, (&raw mut *ship.ss).cast());
            assert!((*dog).hTarget.is_null());
            assert!((*dog).preprocess_func.is_some());
            assert!((*dog).postprocess_func.is_none());
            assert!((*dog).collision_func.is_some());
            assert!((*dog).death_func.is_some());
            assert_eq!(
                velocity_fields(&(*dog).velocity),
                velocity_toward(32, expected_facing)
            );
            let dx = cosine(i32::from(angle), 34 << ONE_SHIFT);
            let dy = sine(i32::from(angle), 34 << ONE_SHIFT);
            assert_eq!(
                (*dog).current.location.x,
                ((i32::from(i16::MAX) - 10) + i32::from(dx)) as i16
            );
            assert_eq!(
                (*dog).current.location.y,
                ((i32::from(i16::MIN) + 10) + i32::from(dy)) as i16
            );
            assert_ne!((*ship.ptr()).state_flags & DEFY_PHYSICS as u16, 0);
            assert_eq!(ship.desc.ship_info.energy_level, 0);
            assert_eq!(ship.ss.energy_counter, 4);
            assert_eq!(ship.ss.special_counter, 1);
        }
    }
}

#[test]
fn special_checks_key_counter_and_energy_but_always_resets_the_counter() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        for (key, counter, energy, spawn, low) in [
            (false, 0, 30, false, false),
            (true, 3, 30, true, false),
            (true, 4, 30, false, false),
            (true, 255, 30, false, false),
            (true, 0, 29, false, true),
            (true, 0, 0, false, true),
        ] {
            let before = queued().len();
            ship.ss.cur_status_flags = if key { SPECIAL as u16 } else { 0 };
            ship.ss.special_counter = counter;
            ship.desc.ship_info.energy_level = energy;
            (*ship.ptr()).state_flags &= !(DEFY_PHYSICS as u16);
            with_rng(1, 0, || (ship.desc.postprocess_func.unwrap())(ship.ptr()));
            assert_eq!(queued().len(), before + usize::from(spawn));
            assert_eq!(ship.ss.special_counter, 1);
            assert_eq!(
                ship.desc.ship_info.energy_level,
                if spawn { 0 } else { energy }
            );
            assert_eq!(ship.ss.cur_status_flags & (1 << 5) != 0, low);
            assert_eq!((*ship.ptr()).state_flags & DEFY_PHYSICS as u16 != 0, spawn);
        }
    }
}

#[test]
fn failed_doggy_allocation_still_spends_the_energy() {
    let _battle = Battle::with_capacity(1);
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.cur_status_flags = SPECIAL as u16;
        (ship.desc.postprocess_func.unwrap())(ship.ptr());
        assert_eq!(queued(), vec![ship.handle]);
        assert_eq!(ship.desc.ship_info.energy_level, 0);
        assert_eq!(ship.ss.energy_counter, 4);
        assert_eq!(ship.ss.special_counter, 1);
        assert_eq!((*ship.ptr()).state_flags & DEFY_PHYSICS as u16, 0);
    }
}

#[test]
fn ship_preprocess_matches_byte_promotion_and_wrap_boundaries() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let preprocess = ship.desc.preprocess_func.unwrap();
        for (special, expected) in [(0, 0), (1, 1), (2, 6), (251, 255), (252, 0), (255, 3)] {
            ship.ss.special_counter = special;
            ship.ss.weapon_counter = 255;
            ship.ss.cur_status_flags = WEAPON as u16;
            ship.ss.old_status_flags = WEAPON as u16;
            with_rng(5, 0, || preprocess(ship.ptr()));
            assert_eq!(ship.ss.special_counter, expected);
            assert_eq!(ship.ss.weapon_counter, 0);
        }
        for (current, old) in [(0, 0), (WEAPON, 0), (0, WEAPON), (SPECIAL, WEAPON)] {
            ship.ss.cur_status_flags = current as u16;
            ship.ss.old_status_flags = old as u16;
            ship.ss.weapon_counter = 13;
            preprocess(ship.ptr());
            assert_eq!(ship.ss.weapon_counter, 13);
        }
    }
}

#[test]
fn doggy_collision_drains_only_enemy_ships_and_extends_only_short_waits() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        let dog = ship.doggy();
        let collide = (*dog).collision_func.unwrap();
        for (energy, remaining) in [(0, 0), (9, 0), (10, 0), (11, 1), (30, 20)] {
            enemy.desc.ship_info.energy_level = energy;
            enemy.ss.energy_counter = 99;
            (*dog).__bindgen_anon_5.thrust_wait = 0;
            with_rng(101, 0, || {
                collide(dog, ptr::null_mut(), enemy.ptr(), ptr::null_mut())
            });
            assert_eq!(enemy.desc.ship_info.energy_level, remaining);
            assert_eq!(
                enemy.ss.energy_counter, 4,
                "even zero drain calls DeltaEnergy"
            );
            assert_ne!((*dog).state_flags & COLLISION as u16, 0);
            assert_eq!((*dog).__bindgen_anon_5.thrust_wait, 6);
        }
        for (wait, expected) in [(0, 6), (1, 7), (2, 8), (3, 9), (4, 4), (255, 255)] {
            (*dog).__bindgen_anon_5.thrust_wait = wait;
            ship.desc.ship_info.energy_level = 17;
            collide(dog, ptr::null_mut(), ship.ptr(), ptr::null_mut());
            assert_eq!((*dog).__bindgen_anon_5.thrust_wait, expected);
            assert_eq!(ship.desc.ship_info.energy_level, 17);
        }
        let mut object: ELEMENT = mem::zeroed();
        object.playerNr = 1;
        object.state_flags = FINITE_LIFE as u16;
        (*dog).state_flags &= !(COLLISION as u16);
        collide(dog, ptr::null_mut(), &mut object, ptr::null_mut());
        assert_eq!((*dog).state_flags & COLLISION as u16, 0);
        // The generic collision runs first and damages the DOGI on planets.
        object.state_flags = 0;
        object.__bindgen_anon_3.mass_points = 101;
        collide(dog, ptr::null_mut(), &mut object, ptr::null_mut());
        assert_eq!((*dog).__bindgen_anon_2.hit_points, 2);
        assert_ne!((*dog).state_flags & COLLISION as u16, 0);
    }
}

#[test]
fn doggy_death_preserves_unrelated_fields_and_starts_six_frame_animation() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let dog = ship.doggy();
        (*dog).state_flags |= (DISAPPEARING | COLLISION | DEFY_PHYSICS) as u16;
        (*dog).postprocess_func = Some(sentinel);
        (*dog).hTarget = ship.handle;
        (*dog).__bindgen_anon_4.turn_wait = 9;
        (*dog).__bindgen_anon_5.thrust_wait = 7;
        let preprocess = (*dog).preprocess_func.unwrap() as usize;
        let old_flags = (*dog).state_flags;
        let counter = ship.ss.special_counter;
        with_rng(88, 0, || ((*dog).death_func.unwrap())(dog));
        assert_eq!(
            (*dog).state_flags,
            (old_flags & !(DISAPPEARING as u16)) | (NONSOLID | FINITE_LIFE) as u16
        );
        assert_eq!((*dog).__bindgen_anon_1.life_span, 6);
        assert_eq!(velocity_fields(&(*dog).velocity), [0; 9]);
        assert_eq!((*dog).__bindgen_anon_4.turn_wait, 0);
        assert_eq!((*dog).__bindgen_anon_5.next_turn, 0);
        assert!((*dog).death_func.is_none());
        assert!((*dog).collision_func.is_none());
        assert_ne!((*dog).preprocess_func.unwrap() as usize, preprocess);
        assert_eq!(
            (*dog).postprocess_func.unwrap() as usize,
            sentinel as *const () as usize
        );
        assert_eq!((*dog).hTarget, ship.handle);
        assert_eq!((*dog).pParent, (&raw mut *ship.ss).cast());
        assert_eq!((*dog).__bindgen_anon_2.hit_points, 3);
        assert_eq!((*dog).__bindgen_anon_3.mass_points, 4);
        let animate = (*dog).preprocess_func.unwrap();
        for frame in 1..=6 {
            with_rng(88, 0, || animate(dog));
            assert_eq!(frame_index((*dog).next.image.frame), frame % 6);
            (*dog).current = (*dog).next;
        }
        assert_eq!(
            ship.ss.special_counter, counter,
            "dead DOGIs no longer count as active"
        );
    }
}

#[test]
fn doggy_wait_counts_before_tracking_and_random_roaming_consumes_one_draw() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let dog = ship.doggy();
        let preprocess = (*dog).preprocess_func.unwrap();
        SetVelocityVector(&raw mut (*dog).velocity, 11, 3);
        let original = velocity_fields(&(*dog).velocity);
        (*dog).__bindgen_anon_5.thrust_wait = 1;
        ship.ss.special_counter = 255;
        with_rng(42, 0, || preprocess(dog));
        assert_eq!(ship.ss.special_counter, 0);
        assert_eq!((*dog).__bindgen_anon_5.thrust_wait, 0);
        assert_eq!(velocity_fields(&(*dog).velocity), original);
        for seed in 1..33 {
            SetVelocityVector(&raw mut (*dog).velocity, 11, 3);
            TFB_SeedRandom(seed);
            let face = (TFB_Random() & 15) as u16;
            with_rng(seed, 1, || preprocess(dog));
            assert_eq!(
                velocity_fields(&(*dog).velocity),
                if face == 3 {
                    original
                } else {
                    velocity_toward(32, face)
                }
            );
            assert!((*dog).hTarget.is_null());
        }
    }
}

#[test]
fn doggy_steering_uses_exact_rear_facing_boundaries_not_trackships_single_step() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        let dog = ship.doggy();
        let preprocess = (*dog).preprocess_func.unwrap();
        (*dog).current.location = POINT { x: 4000, y: 4500 };
        (*dog).next.location = (*dog).current.location;
        // Enemy due north: raw facing 0, TrackShip's one-step result is 3.
        for (delta, expected) in [(6, 0), (7, 4), (8, 12), (9, 12), (10, 0)] {
            (*enemy.ptr()).current.image.frame = enemy.ship_frames.frame((16 - delta) as usize);
            (*dog).hTarget = ptr::null_mut();
            SetVelocityVector(&raw mut (*dog).velocity, 11, 4);
            let original = velocity_fields(&(*dog).velocity);
            with_rng(99, 0, || preprocess(dog));
            assert_eq!((*dog).hTarget, enemy.handle);
            assert_eq!(
                velocity_fields(&(*dog).velocity),
                if expected == 4 {
                    original
                } else {
                    velocity_toward(32, expected)
                },
                "delta {delta}"
            );
        }
        // TrackShip itself draws once for a half-circle tie. DOGI recomputes
        // its heading afterward, so the random tie-break does not choose it.
        (*enemy.ptr()).current.image.frame = enemy.ship_frames.frame(0);
        for seed in 1..17 {
            SetVelocityVector(&raw mut (*dog).velocity, 32, 8);
            (*dog).hTarget = enemy.handle;
            with_rng(seed, 1, || preprocess(dog));
            assert_eq!(velocity_fields(&(*dog).velocity), velocity_toward(32, 0));
        }
    }
}

#[test]
fn doggy_uses_current_unwrapped_coordinates_and_truncates_arctan_arguments() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        let dog = ship.doggy();
        let preprocess = (*dog).preprocess_func.unwrap();
        for (dog_x, enemy_x, expected) in [(-30000, 30000, 12), (8000, 100, 12), (30000, -30000, 4)]
        {
            (*dog).current.location = POINT { x: dog_x, y: 4000 };
            (*dog).next.location = POINT { x: 1000, y: 4000 };
            enemy.at(enemy_x, 4000);
            (*enemy.ptr()).next.location = POINT { x: 2000, y: 4000 };
            (*enemy.ptr()).current.image.frame = enemy.ship_frames.frame(expected as usize);
            (*dog).hTarget = enemy.handle;
            SetVelocityVector(&raw mut (*dog).velocity, 32, 0);
            assert_eq!(
                normalize_facing(angle_to_facing(i32::from(ARCTAN(
                    (i32::from(enemy_x) - i32::from(dog_x)) as SIZE,
                    0
                )))),
                expected
            );
            with_rng(61, 0, || preprocess(dog));
            assert_eq!(
                velocity_fields(&(*dog).velocity),
                velocity_toward(32, expected)
            );
        }
    }
}

#[test]
fn doggy_acquires_nearest_visible_ship_but_retains_an_existing_cloaked_target() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut far = Ship::new(1);
        let mut near = Ship::new(1);
        let dog = ship.doggy();
        (*dog).current.location = POINT { x: 4000, y: 4000 };
        (*dog).next = (*dog).current;
        near.at(4100, 4000);
        far.at(4000, 3500);
        let preprocess = (*dog).preprocess_func.unwrap();
        SetVelocityVector(&raw mut (*dog).velocity, 32, 0);
        with_rng(4, 0, || preprocess(dog));
        assert_eq!((*dog).hTarget, near.handle);
        assert_eq!(velocity_fields(&(*dog).velocity), velocity_toward(32, 4));
        (*prim(near.ptr())).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
        SetVelocityVector(&raw mut (*dog).velocity, 32, 0);
        with_rng(4, 0, || preprocess(dog));
        assert_eq!(
            (*dog).hTarget,
            near.handle,
            "TrackShip retains a live prior target even when cloaked"
        );
        (*dog).hTarget = ptr::null_mut();
        SetVelocityVector(&raw mut (*dog).velocity, 32, 0);
        with_rng(4, 0, || preprocess(dog));
        assert_eq!((*dog).hTarget, far.handle);
        assert_eq!(velocity_fields(&(*dog).velocity), velocity_toward(32, 0));
    }
}

#[test]
fn ai_pursuit_boundaries_match_turn_maneuverability_range_and_seeking_flags() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        let cases = [
            (16, 45, 0, false, true),
            (17, 45, 0, false, false),
            (16, 44, 0, false, false),
            (0, 45, 0, false, true),
            (17, 25, 3000, true, true),
            (17, 26, 3000, true, false),
            (17, 25, 2999, true, false),
            (17, 25, 3000, false, false),
            (65535, 0, 4000, true, true),
            (17, 65535, 4000, true, false),
        ];
        // Avoid the launch probe while checking movement-state decisions.
        ship.ss.weapon_counter = 1;
        for (turn, maneuver, range, seeking, pursue) in cases {
            enemy.desc.cyborg_control.ManeuverabilityIndex = maneuver;
            enemy.desc.cyborg_control.WeaponRange = range;
            enemy.desc.ship_info.ship_flags = if seeking { SEEKING_WEAPON as u16 } else { 0 };
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize] = EVALUATE_DESC {
                ObjectPtr: enemy.ptr(),
                facing: 0,
                which_turn: turn,
                MoveState: MOVEMENT_STATE_ENTICE,
            };
            with_rng(6, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(
                concerns[ENEMY_SHIP_INDEX as usize].MoveState,
                if pursue {
                    MOVEMENT_STATE_PURSUE
                } else {
                    MOVEMENT_STATE_ENTICE
                },
                "turn {turn}, maneuver {maneuver}, range {range}, seeking {seeking}"
            );
        }
    }
}

#[test]
fn ai_special_budget_uses_less_than_four_and_energy_less_than_or_equal_to_cost() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        for counter in [0, 1, 3, 4, 255] {
            for energy in [0, 29, 30, 31] {
                for weapon in [false, true] {
                    ship.ss.weapon_counter = 1;
                    ship.ss.special_counter = counter;
                    ship.desc.ship_info.energy_level = energy;
                    ship.ss.ship_input_state =
                        SPECIAL as u8 | if weapon { WEAPON as u8 } else { 0 };
                    let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                    concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
                    with_rng(81, 0, || intelligence(&mut ship, &mut concerns));
                    assert_eq!(
                        ship.ss.ship_input_state & SPECIAL as u8 != 0,
                        counter < 4 && energy <= 30 && !weapon,
                        "counter {counter}, energy {energy}, weapon {weapon}"
                    );
                }
            }
        }
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        ship.ss.ship_input_state = SPECIAL as u8;
        with_rng(81, 0, || intelligence(&mut ship, &mut concerns));
        assert_eq!(ship.ss.ship_input_state & SPECIAL as u8, 0);
    }
}

#[test]
fn ai_weapon_threat_selection_and_release_cooldown_use_exact_boundaries() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut threat: ELEMENT = mem::zeroed();
        for (counter, turn, state, selected) in [
            (1, 8, MOVEMENT_STATE_ENTICE, true),
            (1, 9, MOVEMENT_STATE_ENTICE, false),
            (0, 8, MOVEMENT_STATE_ENTICE, false),
            (2, 8, MOVEMENT_STATE_ENTICE, false),
            (1, 8, MOVEMENT_STATE_AVOID, false),
        ] {
            ship.ss.old_status_flags = WEAPON as u16;
            ship.ss.ship_input_state = (SPECIAL | WEAPON) as u8;
            ship.ss.weapon_counter = 1;
            ship.ss.special_counter = counter;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                ObjectPtr: &mut threat,
                facing: 0,
                which_turn: turn,
                MoveState: state,
            };
            with_rng(51, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(ship.ss.weapon_counter, if selected { 3 } else { 1 });
            assert_eq!(ship.ss.ship_input_state & WEAPON as u8 != 0, !selected);
            assert_eq!(ship.ss.ship_input_state & SPECIAL as u8 != 0, selected);
        }
        // A matching crystal without an enemy ship terminates the backwards
        // scan, then follows the same no-crystal release path.
        let h = ship.crystal();
        put_element(h);
        ship.ss.old_status_flags = WEAPON as u16;
        ship.ss.weapon_counter = 0;
        ship.ss.special_counter = 1;
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
            ObjectPtr: &mut threat,
            facing: 0,
            which_turn: 8,
            MoveState: MOVEMENT_STATE_ENTICE,
        };
        intelligence(&mut ship, &mut concerns);
        assert_eq!(ship.ss.weapon_counter, 3);
        assert_eq!(ship.ss.ship_input_state & WEAPON as u8, 0);
    }
}

#[test]
fn ai_crystal_intercept_holds_or_releases_at_the_fragment_margin() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        let h = ship.crystal();
        let crystal = element(h);
        put_element(h);
        (*crystal).current.location = POINT { x: 4000, y: 4000 };
        SetVelocityVector(&raw mut (*crystal).velocity, 64, 0);
        for (x, y, hold, immediate) in [
            (4000, 3000, true, false),
            (4320, 3000, true, false),
            (4321, 3000, false, false),
            (4000, 3800, false, true),
        ] {
            enemy.at(x, y);
            let intercept = PlotIntercept(crystal, enemy.ptr(), 90, 320);
            if immediate {
                assert_eq!(intercept, 1);
                assert_eq!(PlotIntercept(crystal, enemy.ptr(), 90, 0), 0);
            } else {
                assert_eq!(intercept > 1, hold, "({x}, {y}) intercept {intercept}");
            }
            ship.ss.old_status_flags = WEAPON as u16;
            ship.ss.weapon_counter = 0;
            ship.ss.special_counter = 4;
            ship.ss.ship_input_state = (SPECIAL | WEAPON) as u8;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
            with_rng(63, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(
                ship.ss.ship_input_state & WEAPON as u8 != 0,
                hold,
                "({x}, {y})"
            );
            assert_eq!(ship.ss.ship_input_state & SPECIAL as u8, 0);
        }
    }
}

#[test]
fn ai_scans_from_tail_skips_every_noncrystal_case_and_stops_at_the_first_match() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        enemy.at(4000, 3000);
        let miss_h = ship.crystal();
        let miss = element(miss_h);
        (*miss).current.location = POINT { x: 1000, y: 4000 };
        SetVelocityVector(&raw mut (*miss).velocity, 64, 0);
        put_element(miss_h);
        let hit_h = ship.crystal();
        let hit = element(hit_h);
        (*hit).current.location = POINT { x: 4000, y: 4000 };
        SetVelocityVector(&raw mut (*hit).velocity, 64, 0);
        put_element(hit_h);
        for invalid in 0..5 {
            let h = AllocElement();
            assert!(!h.is_null());
            let e = element(h);
            let prim_index = (*e).PrimIndex;
            *e = *hit;
            (*e).PrimIndex = prim_index;
            (*e).pred = ptr::null_mut();
            (*e).succ = ptr::null_mut();
            match invalid {
                0 => (*e).state_flags |= NONSOLID as u16,
                1 => (*e).next.image.farray = (&raw mut ship.desc.ship_data.special).cast(),
                2 => (*e).preprocess_func = None,
                3 => (*e).__bindgen_anon_1.life_span = 0,
                4 => (*e).playerNr = 1,
                _ => unreachable!(),
            }
            put_element(h);
        }
        ship.ss.old_status_flags = WEAPON as u16;
        ship.ss.special_counter = 4;
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
        with_rng(30, 0, || intelligence(&mut ship, &mut concerns));
        assert_ne!(
            ship.ss.ship_input_state & WEAPON as u8,
            0,
            "newest matching crystal intercepts"
        );
        RemoveElement(hit_h);
        FreeElement(hit_h);
        with_rng(30, 0, || intelligence(&mut ship, &mut concerns));
        assert_eq!(
            ship.ss.ship_input_state & WEAPON as u8,
            0,
            "older matching crystal misses"
        );
    }
}

#[test]
fn ai_launch_probe_uses_the_selected_target_and_respects_weapon_cooldown() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        ship.ss.ShipFacing = 0;
        ship.ss.special_counter = 4;
        for (x, counter, fire) in [(4000, 0, true), (4400, 0, false), (4000, 1, false)] {
            enemy.at(x, 3000);
            ship.ss.old_status_flags = 0;
            ship.ss.weapon_counter = counter;
            ship.ss.ship_input_state = SPECIAL as u8;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
            let before = queued();
            with_rng(8, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(ship.ss.ship_input_state & WEAPON as u8 != 0, fire);
            assert_eq!(
                queued(),
                before,
                "launch probes never enter the display queue"
            );
        }
        ship.ss.weapon_counter = 0;
        ship.ss.special_counter = 1;
        let mut threat: ELEMENT = mem::zeroed();
        threat.current.location = POINT { x: 4000, y: 3000 };
        threat.next = threat.current;
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
            ObjectPtr: &mut threat,
            facing: 0,
            which_turn: 8,
            MoveState: MOVEMENT_STATE_ENTICE,
        };
        with_rng(8, 0, || intelligence(&mut ship, &mut concerns));
        assert_ne!(ship.ss.ship_input_state & WEAPON as u8, 0);
        assert_eq!(ship.ss.ship_input_state & SPECIAL as u8, 0);
    }
}
