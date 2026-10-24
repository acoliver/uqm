//! Exercise Orz's native callbacks with the linked engine queue and physics.

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
    fn init_orz() -> *mut RACE_DESC;
    fn intruder_preprocess(e: *mut ELEMENT);
    fn marine_collision(a: *mut ELEMENT, pa: *mut POINT, b: *mut ELEMENT, pb: *mut POINT);
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: COUNT) -> BOOLEAN;
    fn UninitQueue(queue: *mut QUEUE) -> BOOLEAN;
    fn InitDisplayList();
    static mut DisplayArray: [PRIMITIVE; 330];
}

static ENGINE: Mutex<()> = Mutex::new(());

struct Battle {
    _guard: MutexGuard<'static, ()>,
    globals: GLOBDATA,
    screen: (i32, i32),
}

impl Battle {
    fn new() -> Self {
        Self::with_capacity(128)
    }

    fn with_capacity(capacity: COUNT) -> Self {
        let guard = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            let globals = GlobData;
            let screen = (ScreenWidth, ScreenHeight);
            assert_ne!(
                InitQueue(
                    &raw mut disp_q,
                    capacity,
                    mem::size_of::<ELEMENT>() as COUNT
                ),
                0
            );
            InitDisplayList();
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
            GlobData.Game_state.CurrentActivity = IN_ENCOUNTER as _;
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
                _guard: guard,
                globals,
                screen,
            }
        }
    }
}

impl Drop for Battle {
    fn drop(&mut self) {
        unsafe {
            UninitQueue(&raw mut disp_q);
            GlobData = self.globals;
            (ScreenWidth, ScreenHeight) = self.screen;
        }
    }
}

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
    _ship_frames: FrameSet,
    weapon_frames: FrameSet,
    special_frames: FrameSet,
}

impl Ship {
    unsafe fn new(player: SIZE) -> Self {
        let mut desc = Box::new(*init_orz());
        let ship_frames = FrameSet::new(16);
        let weapon_frames = FrameSet::new(16);
        let special_frames = FrameSet::new(32);
        desc.ship_data.ship = [ship_frames.frame(0); 3];
        desc.ship_data.weapon = [weapon_frames.frame(0); 3];
        desc.ship_data.special = [special_frames.frame(0); 3];
        desc.ship_info.icons = ship_frames.frame(0);
        let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
        ss.RaceDescPtr = &raw mut *desc;
        ss.playerNr = player;
        let handle = AllocElement();
        assert!(!handle.is_null());
        ss.hShip = handle;
        let e = element(handle);
        (*e).pParent = (&raw mut *ss).cast();
        (*e).playerNr = player;
        (*e).state_flags = PLAYER_SHIP as u16;
        (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
        (*e).__bindgen_anon_2.crew_level = 16;
        (*e).__bindgen_anon_3.mass_points = 4;
        (*e).current.location = POINT { x: 4000, y: 4000 };
        (*e).current.image.farray = (&raw mut desc.ship_data.ship).cast();
        (*e).current.image.frame = ship_frames.frame(0);
        (*e).next = (*e).current;
        (*e).IntersectControl.IntersectStamp.frame = ship_frames.frame(0);
        (*prim(e)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        put_element(handle);
        Self {
            desc,
            ss,
            handle,
            _ship_frames: ship_frames,
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

    unsafe fn install_turret(&mut self) -> *mut ELEMENT {
        (*self.ptr()).state_flags |= APPEARING as u16;
        (self.desc.preprocess_func.unwrap())(self.ptr());
        (*self.ptr()).state_flags &= !(APPEARING as u16);
        let turret = element((*self.ptr()).succ);
        assert!(!turret.is_null());
        assert_eq!((*turret).pParent, (&raw mut *self.ss).cast());
        turret
    }

    unsafe fn tick_turret(&mut self) -> *mut ELEMENT {
        let old = (*self.ptr()).succ;
        let e = element(old);
        (*e).__bindgen_anon_1.life_span = 0;
        ((*e).postprocess_func.unwrap())(e);
        RemoveElement(old);
        FreeElement(old);
        element((*self.ptr()).succ)
    }

    unsafe fn launch(&mut self) -> *mut ELEMENT {
        self.ss.cur_status_flags = (SPECIAL | WEAPON) as u16;
        self.ss.special_counter = 0;
        let tail = disp_q.tail;
        // Earlier objects have completed their appearing frame before this shot.
        (*element(tail)).state_flags &= !(APPEARING as u16);
        self.tick_turret();
        assert_ne!({ disp_q.tail }, tail);
        let marine = element(disp_q.tail);
        assert_eq!(
            (*marine).collision_func.unwrap() as usize,
            marine_collision as *const () as usize
        );
        (*marine).next = (*marine).current;
        marine
    }

    unsafe fn missile(&mut self) -> HELEMENT {
        let mut out = ptr::null_mut();
        assert_eq!(
            (self.desc.init_weapon_func.unwrap())(self.ptr(), &mut out),
            1
        );
        assert!(!out.is_null());
        let e = element(out);
        assert!((*e).pParent.is_null());
        (*e).pParent = (&raw mut *self.ss).cast();
        (*e).next = (*e).current;
        out
    }
}

unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
    (&raw mut DisplayArray)
        .cast::<PRIMITIVE>()
        .add((*e).PrimIndex as usize)
}

unsafe fn queued() -> Vec<HELEMENT> {
    let mut result = Vec::new();
    let mut h = disp_q.head;
    let mut previous = ptr::null_mut();
    while !h.is_null() {
        assert_eq!((*element(h)).pred, previous);
        result.push(h);
        assert!(result.len() <= 255, "display queue cycle");
        previous = h;
        h = (*element(h)).succ;
    }
    assert_eq!(previous, { disp_q.tail });
    result
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

unsafe fn seed_for_byte(byte: BYTE) -> u32 {
    (1..100_000)
        .find(|&seed| {
            TFB_SeedRandom(seed);
            TFB_Random() as BYTE == byte
        })
        .expect("seed for each intruder outcome boundary")
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

unsafe fn collide(marine: *mut ELEMENT, target: *mut ELEMENT) {
    marine_collision(marine, ptr::null_mut(), target, ptr::null_mut());
}

unsafe fn intelligence(ship: &mut Ship, concerns: &mut [EVALUATE_DESC; 4]) {
    // Isolate Orz's decisions after the real shared pass, with no movement concerns.
    (ship.desc.cyborg_control.intelligence_func.unwrap())(ship.ptr(), concerns.as_mut_ptr(), 0);
}

#[test]
fn descriptor_preserves_c_resources_characteristics_and_callback_abi() {
    let _battle = Battle::new();
    unsafe {
        let template = init_orz();
        assert_eq!(template, init_orz());
        let d = *template;
        assert_eq!(
            d.ship_info.ship_flags,
            (FIRES_FORE | SEEKING_SPECIAL) as u16
        );
        assert_eq!(d.ship_info.ship_cost, 23);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (16, 16));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (20, 20));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.orz.text"
        );
        assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.orz.icons");
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.orz.meleeicons"
        );
        assert!(d.ship_info.race_strings.is_null());
        assert!(d.ship_info.icons.is_null());
        assert!(d.ship_info.melee_icon.is_null());
        assert_eq!(d.fleet.strength, 60);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (3608, 2637));
        let c = d.characteristics;
        assert_eq!(
            (c.max_thrust, c.thrust_increment, c.energy_regeneration),
            (35, 5, 1)
        );
        assert_eq!(
            (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
            (6, 0, 6)
        );
        assert_eq!(
            (
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (1, 0, 4, 12, 4)
        );
        for (i, size) in ["large", "medium", "small"].iter().enumerate() {
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.orz.graphics.nemesis.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                format!("ship.orz.graphics.howitzer.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                format!("ship.orz.graphics.turret.{size}")
            );
            assert!(d.ship_data.ship[i].is_null());
            assert!(d.ship_data.weapon[i].is_null());
            assert!(d.ship_data.special[i].is_null());
        }
        assert_eq!(
            CStr::from_ptr(d.ship_data.captain_control.captain_rsc),
            c"ship.orz.graphics.captain"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.victory_ditty_rsc),
            c"ship.orz.ditty"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_sounds_rsc),
            c"ship.orz.sounds"
        );
        assert_eq!(d.cyborg_control.WeaponRange, 1440);
        assert_eq!(d.cyborg_control.ManeuverabilityIndex, 0);
        assert!(d.preprocess_func.is_some());
        assert!(d.init_weapon_func.is_some());
        assert!(d.cyborg_control.intelligence_func.is_some());
        assert!(d.postprocess_func.is_none());
        assert!(d.uninit_func.is_none());
        assert!(d.data.is_null());
        assert!(d.CodeRef.is_null());
        let _: ElementProcessFunc = Some(intruder_preprocess);
        let _: ElementCollisionFunc = Some(marine_collision);
        (*template).preprocess_func = None;
        (*template).init_weapon_func = None;
        (*template).cyborg_control.intelligence_func = None;
        let again = init_orz();
        assert_eq!(
            (*again).preprocess_func.unwrap() as usize,
            d.preprocess_func.unwrap() as usize
        );
        assert_eq!(
            (*again).init_weapon_func.unwrap() as usize,
            d.init_weapon_func.unwrap() as usize
        );
        assert_eq!(
            (*again).cyborg_control.intelligence_func.unwrap() as usize,
            d.cyborg_control.intelligence_func.unwrap() as usize
        );
    }
}

#[test]
fn appearing_turret_is_inserted_immediately_after_ship_and_inputs_suppress_hull_and_gun() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        ship.ss.ShipFacing = 7;
        let turret = with_rng(2, 0, || ship.install_turret());
        assert_eq!(queued(), vec![ship.handle, turret.cast(), enemy.handle]);
        assert_eq!(
            (*turret).state_flags,
            (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16
        );
        assert_eq!((*turret).__bindgen_anon_1.life_span, 1);
        assert_eq!((*turret).current.image.frame, ship.special_frames.frame(7));
        assert_eq!(
            (*turret).current.image.farray,
            (&raw mut ship.desc.ship_data.special).cast()
        );
        assert!((*turret).postprocess_func.is_some());
        assert!((*turret).preprocess_func.is_none());
        assert!((*turret).collision_func.is_none());
        let preprocess = ship.desc.preprocess_func.unwrap();
        for (current, old, turn, weapon, expected_turn, expected_weapon) in [
            (SPECIAL | RIGHT | WEAPON, 0, 0, 0, 1, 1),
            (LEFT, SPECIAL, 0, 0, 1, 0),
            (SPECIAL | LEFT | WEAPON, 0, 2, 9, 2, 9),
            (SPECIAL | WEAPON, 0, 0, 0, 0, 1),
            (RIGHT | WEAPON, 0, 0, 0, 0, 0),
        ] {
            ship.ss.cur_status_flags = current as u16;
            ship.ss.old_status_flags = old as u16;
            ship.ss.weapon_counter = weapon;
            (*ship.ptr()).__bindgen_anon_4.turn_wait = turn;
            with_rng(2, 0, || preprocess(ship.ptr()));
            assert_eq!((*ship.ptr()).__bindgen_anon_4.turn_wait, expected_turn);
            assert_eq!(ship.ss.weapon_counter, expected_weapon);
        }
    }
}

#[test]
fn howitzer_rotates_before_firing_with_byte_wrap_and_exact_missile_fields() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(1);
        let turret = ship.install_turret();
        ship.at(i16::MAX - 10, i16::MIN + 10);
        (*ship.ptr()).next.location = POINT { x: -1234, y: 2345 };
        for (status, offset, wait, expected_offset, expected_wait) in [
            (SPECIAL | RIGHT, 255, 0, 0, 4),
            (SPECIAL | LEFT, 0, 0, 255, 4),
            (SPECIAL | LEFT | RIGHT, 3, 0, 4, 4),
            (SPECIAL | RIGHT, 9, 1, 9, 1),
            (RIGHT, 9, 0, 9, 0),
        ] {
            ship.ss.ShipFacing = 15;
            ship.ss.cur_status_flags = status as u16;
            (*turret).__bindgen_anon_4.turn_wait = wait;
            (*turret).__bindgen_anon_5.thrust_wait = offset;
            let h = with_rng(11, 0, || ship.missile());
            let missile = element(h);
            let face = normalize_facing(15 + i32::from(expected_offset));
            assert_eq!((*turret).__bindgen_anon_4.turn_wait, expected_wait);
            assert_eq!((*turret).__bindgen_anon_5.thrust_wait, expected_offset);
            assert_eq!((*missile).__bindgen_anon_1.life_span, 12);
            assert_eq!((*missile).__bindgen_anon_2.hit_points, 2);
            assert_eq!((*missile).__bindgen_anon_3.mass_points, 3);
            assert_eq!((*missile).__bindgen_anon_5.blast_offset, 1);
            assert_eq!((*missile).playerNr, 1);
            assert_eq!(
                (*missile).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!(
                (*missile).current.image.frame,
                ship.weapon_frames.frame(face as usize)
            );
            assert_eq!(
                (*missile).current.image.farray,
                (&raw mut ship.desc.ship_data.weapon).cast()
            );
            assert_eq!(
                velocity_fields(&(*missile).velocity),
                velocity_toward(120, face)
            );
            let a = facing_to_angle(i32::from(face));
            let dx = i32::from(cosine(a, 120 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
            let dy = i32::from(sine(a, 120 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
            assert_eq!(
                (*missile).current.location.x,
                (-1234 + i32::from(cosine(a, 56)) - dx) as COORD
            );
            assert_eq!(
                (*missile).current.location.y,
                (2345 + i32::from(sine(a, 56)) - dy) as COORD
            );
            assert!((*missile).preprocess_func.is_none());
            assert!((*missile).postprocess_func.is_none());
            FreeElement(h);
        }
    }
}

#[test]
fn howitzer_ignores_friendlies_but_damages_enemies_through_engine_collision() {
    let _battle = Battle::with_capacity(4);
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let enemy = Ship::new(1);
        let h = ship.missile();
        let missile = element(h);
        let collision = (*missile).collision_func.unwrap();
        let mut point = POINT { x: 10, y: 20 };
        with_rng(17, 0, || {
            collision(missile, &mut point, ship.ptr(), &mut POINT { x: 0, y: 0 })
        });
        assert_eq!((*ship.ptr()).__bindgen_anon_2.crew_level, 16);
        assert_eq!((*missile).__bindgen_anon_1.life_span, 12);
        with_rng(17, 0, || {
            collision(missile, &mut point, enemy.ptr(), &mut POINT { x: 0, y: 0 })
        });
        assert_eq!((*enemy.ptr()).__bindgen_anon_2.crew_level, 13);
        assert_eq!(enemy.desc.ship_info.crew_level, 13);
        assert_eq!((*missile).__bindgen_anon_1.life_span, 0);
        assert_eq!((*missile).__bindgen_anon_2.hit_points, 0);
        assert_ne!(
            (*missile).state_flags & (COLLISION | NONSOLID | DISAPPEARING) as u16,
            0
        );
        FreeElement(h);
    }
}

#[test]
fn turret_replacement_decrements_wait_once_rotates_and_updates_firing_quadrant() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        ship.ss.ShipFacing = 15;
        ship.ss.cur_status_flags = (SPECIAL | RIGHT) as u16;
        let tint = Color {
            r: 3,
            g: 7,
            b: 11,
            a: 99,
        };
        (*prim(ship.ptr())).Type = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
        (*prim(ship.ptr())).color = tint;
        (*ship.ptr()).next.location = POINT { x: 123, y: -456 };
        for (expected_offset, expected_wait) in [(1, 3), (1, 2), (1, 1), (1, 0), (2, 3)] {
            let turret = with_rng(5, 0, || ship.tick_turret());
            assert_eq!((*turret).__bindgen_anon_5.thrust_wait, expected_offset);
            assert_eq!((*turret).__bindgen_anon_4.turn_wait, expected_wait);
            assert_eq!((*turret).__bindgen_anon_1.life_span, 1);
            assert_eq!(
                (*turret).state_flags,
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | CHANGING | PRE_PROCESS | POST_PROCESS)
                    as u16
            );
            assert_eq!(
                ((*turret).current.location.x, (*turret).current.location.y),
                (123, -456)
            );
            assert_eq!((*turret).next.image.frame, (*turret).current.image.frame);
            assert_eq!(
                frame_index((*turret).current.image.frame),
                normalize_facing(15 + i32::from(expected_offset))
            );
            assert_eq!(
                (*prim(turret)).Type,
                gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM
            );
            assert_eq!(
                (
                    (*prim(turret)).color.r,
                    (*prim(turret)).color.g,
                    (*prim(turret)).color.b,
                    (*prim(turret)).color.a
                ),
                (3, 7, 11, 99)
            );
        }
        ship.ss.cur_status_flags = 0;
        for face in 0..16 {
            ship.ss.ShipFacing = face;
            let old = element((*ship.ptr()).succ);
            (*old).__bindgen_anon_5.thrust_wait = 0;
            ship.tick_turret();
            let expected = (FIRES_FORE << (normalize_facing(i32::from(face) + 2) / 4)) as u16;
            assert_eq!(
                ship.desc.ship_info.ship_flags,
                SEEKING_SPECIAL as u16 | expected
            );
        }
    }
}

#[test]
fn muzzle_flash_requires_an_appearing_friendly_stamp_and_animates_from_current() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        ship.ss.ShipFacing = 4;
        ship.ss.cur_status_flags = WEAPON as u16;
        let h = ship.missile();
        put_element(h);
        let turret = with_rng(20, 0, || ship.tick_turret());
        let flash = element(disp_q.tail);
        assert_ne!(flash, element(h));
        assert_eq!((*flash).__bindgen_anon_1.life_span, 4);
        assert_eq!((*flash).current.image.frame, ship.special_frames.frame(16));
        assert_eq!(
            (*flash).state_flags,
            (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16
        );
        assert_eq!((*flash).pParent, (&raw mut *ship.ss).cast());
        let a = facing_to_angle(4);
        assert_eq!(
            (*turret).current.location.x,
            (4000 - i32::from(cosine(a, 8))) as COORD
        );
        assert_eq!(
            (*turret).current.location.y,
            (4000 - i32::from(sine(a, 8))) as COORD
        );
        assert_eq!(
            (*flash).current.location.x,
            (i32::from((*turret).current.location.x) + i32::from(cosine(a, 56))) as COORD
        );
        assert_eq!(
            (*flash).current.location.y,
            (i32::from((*turret).current.location.y) + i32::from(sine(a, 56))) as COORD
        );
        let animate = (*flash).preprocess_func.unwrap();
        (*flash).next.image.frame = ship.special_frames.frame(25);
        (*flash).__bindgen_anon_5.next_turn = 2;
        with_rng(20, 0, || animate(flash));
        assert_eq!((*flash).next.image.frame, ship.special_frames.frame(17));
        assert_eq!((*flash).__bindgen_anon_4.turn_wait, 2);
        animate(flash);
        assert_eq!((*flash).__bindgen_anon_4.turn_wait, 1);
        assert_eq!((*flash).next.image.frame, ship.special_frames.frame(17));
        (*flash).state_flags &= !(APPEARING as u16);
        let before = queued().len();
        ship.tick_turret();
        assert_eq!(queued().len(), before);
    }
}

#[test]
fn marines_launch_at_rear_with_exact_state_and_cost_only_after_allocation() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        for facing in [0, 4, 7, 15] {
            ship.ss.ShipFacing = facing;
            ship.at(i16::MAX - 10, i16::MIN + 10);
            let old_crew = ship.desc.ship_info.crew_level;
            let marine = with_rng(50, 0, || ship.launch());
            assert_eq!(
                (*marine).state_flags,
                (IGNORE_SIMILAR | APPEARING | CREW_OBJECT) as u16
            );
            assert_eq!((*marine).__bindgen_anon_1.life_span, 1);
            assert_eq!((*marine).__bindgen_anon_2.hit_points, 3);
            assert_eq!((*marine).__bindgen_anon_3.mass_points, 1);
            assert_eq!((*marine).__bindgen_anon_5.thrust_wait, 0);
            assert_eq!(
                (*marine).__bindgen_anon_4.turn_wait,
                (normalize_facing(i32::from(facing) + 8) << 4) as BYTE
            );
            assert_eq!((*marine).current.image.frame, ship.special_frames.frame(20));
            assert_eq!(
                (*marine).current.image.farray,
                (&raw mut ship.desc.ship_data.special).cast()
            );
            assert_eq!((*marine).pParent, (&raw mut *ship.ss).cast());
            assert_eq!((*prim(marine)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
            assert_eq!(velocity_fields(&(*marine).velocity), [0; 9]);
            assert!((*marine).preprocess_func.is_some());
            assert!((*marine).postprocess_func.is_none());
            assert!((*marine).death_func.is_none());
            let a = facing_to_angle(i32::from(facing));
            assert_eq!(
                (*marine).current.location.x,
                (i32::from(i16::MAX) - 10 - i32::from(cosine(a, 56))) as COORD
            );
            assert_eq!(
                (*marine).current.location.y,
                (i32::from(i16::MIN) + 10 - i32::from(sine(a, 56))) as COORD
            );
            assert_eq!(ship.desc.ship_info.crew_level, old_crew - 1);
            assert_eq!((*ship.ptr()).__bindgen_anon_2.crew_level, old_crew - 1);
            assert_eq!(ship.ss.special_counter, 12);
            assert_eq!(ship.desc.ship_info.energy_level, 20);
            // Retain an aligned cached target so launch eligibility never ties.
            (*ship.ptr()).hTarget = enemy.handle;
        }
    }
}

#[test]
fn marine_limit_counts_only_live_matching_nonfinite_nondisappearing_elements() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let mut marines = Vec::new();
        for _ in 0..8 {
            marines.push(ship.launch());
        }
        assert_eq!(ship.desc.ship_info.crew_level, 8);
        ship.ss.special_counter = 0;
        (*element(disp_q.tail)).state_flags &= !(APPEARING as u16);
        let before = queued().len();
        with_rng(5, 0, || ship.tick_turret());
        assert_eq!(queued().len(), before);
        assert_eq!(ship.ss.special_counter, 0);
        for (i, flags) in [(0, FINITE_LIFE), (1, DISAPPEARING)] {
            (*marines[i]).state_flags |= flags as u16;
            ship.launch();
        }
        (*marines[2]).__bindgen_anon_1.life_span = 0;
        ship.launch();
        (*marines[3]).current.image.farray = (&raw mut enemy.desc.ship_data.special).cast();
        ship.launch();
        assert_eq!(ship.desc.ship_info.crew_level, 4);
    }
}

#[test]
fn launch_gates_require_both_keys_counter_zero_crew_and_a_trackable_enemy() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let enemy = Ship::new(1);
        for (status, counter, crew, cloaked) in [
            (SPECIAL, 0, 16, false),
            (WEAPON, 0, 16, false),
            (SPECIAL | WEAPON, 1, 16, false),
            (SPECIAL | WEAPON, 0, 1, false),
            (SPECIAL | WEAPON, 0, 16, true),
        ] {
            ship.ss.cur_status_flags = status as u16;
            ship.ss.special_counter = counter;
            ship.desc.ship_info.crew_level = crew;
            (*ship.ptr()).__bindgen_anon_2.crew_level = crew;
            (*ship.ptr()).hTarget = ptr::null_mut();
            (*prim(enemy.ptr())).Type = if cloaked {
                gfx_object_NUM_PRIMS
            } else {
                gfx_object_STAMP_PRIM
            } as GRAPHICS_PRIM;
            let before = queued().len();
            // No appearing tail stamp, so this cannot create a muzzle flash.
            (*enemy.ptr()).state_flags &= !(APPEARING as u16);
            with_rng(3, 0, || ship.tick_turret());
            assert_eq!(queued().len(), before);
            assert_eq!(ship.desc.ship_info.crew_level, crew);
            assert_eq!(ship.ss.special_counter, counter);
        }
    }
}

#[test]
fn boarding_claims_first_unused_slot_and_immediately_kills_one_enemy_crew_without_rng() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let mut marines = Vec::new();
        for slot in 0..3 {
            let marine = ship.launch();
            (*enemy.ptr()).state_flags |= COLLISION as u16;
            with_rng(10, 0, || collide(marine, enemy.ptr()));
            assert_eq!((*marine).__bindgen_anon_4.turn_wait, slot);
            assert_eq!((*marine).__bindgen_anon_5.thrust_wait, 12);
            assert_eq!(
                (*marine).next.image.frame,
                ship.special_frames.frame(22 + slot as usize)
            );
            assert_eq!(
                (*marine).preprocess_func.unwrap() as usize,
                intruder_preprocess as *const () as usize
            );
            assert_eq!(
                (*marine).state_flags & (NONSOLID | CREW_OBJECT | COLLISION) as u16,
                NONSOLID as u16
            );
            assert_eq!((*prim(marine)).Type, gfx_object_NUM_PRIMS as GRAPHICS_PRIM);
            assert_eq!((*enemy.ptr()).state_flags & COLLISION as u16, 0);
            marines.push(marine);
        }
        assert_eq!(enemy.desc.ship_info.crew_level, 13);
        (*marines[1]).__bindgen_anon_1.life_span = 0;
        (*marines[1]).state_flags |= DISAPPEARING as u16;
        let marine = ship.launch();
        collide(marine, enemy.ptr());
        assert_eq!((*marine).__bindgen_anon_4.turn_wait, 1);
        assert_eq!((*marine).next.image.frame, ship.special_frames.frame(23));
        let crew = enemy.desc.ship_info.crew_level;
        collide(marine, enemy.ptr());
        assert_eq!(
            enemy.desc.ship_info.crew_level, crew,
            "nonsolid intruder cannot board twice"
        );
    }
}

#[test]
fn marine_collision_handles_return_planets_nonships_and_last_enemy_crew() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        with_rng(8, 0, || collide(marine, ship.ptr()));
        assert_eq!(ship.desc.ship_info.crew_level, 16);
        assert_eq!((*marine).__bindgen_anon_1.life_span, 0);
        assert_eq!((*marine).__bindgen_anon_2.hit_points, 0);
        assert_ne!(
            (*marine).state_flags & (DISAPPEARING | NONSOLID | FINITE_LIFE) as u16,
            0
        );
        let marine = ship.launch();
        let mut object: ELEMENT = mem::zeroed();
        object.playerNr = 1;
        object.__bindgen_anon_3.mass_points = 100;
        (*marine).__bindgen_anon_4.turn_wait = 0xa0;
        (*marine).__bindgen_anon_5.thrust_wait = 7;
        collide(marine, &mut object);
        assert_eq!((*marine).__bindgen_anon_4.turn_wait, 0xa5);
        assert_eq!((*marine).__bindgen_anon_5.thrust_wait, 4);
        assert_ne!((*marine).state_flags & COLLISION as u16, 0);
        assert_eq!((*marine).__bindgen_anon_1.life_span, 1);
        (*marine).state_flags &= !(COLLISION as u16);
        object.__bindgen_anon_3.mass_points = 101;
        collide(marine, &mut object);
        assert_eq!((*marine).__bindgen_anon_1.life_span, 0);
        assert_eq!((*marine).__bindgen_anon_2.hit_points, 0);
        assert_eq!(
            (*marine).state_flags & (NONSOLID | FINITE_LIFE) as u16,
            (NONSOLID | FINITE_LIFE) as u16
        );
        let marine = ship.launch();
        enemy.desc.ship_info.crew_level = 1;
        (*enemy.ptr()).__bindgen_anon_2.crew_level = 1;
        with_rng(8, 0, || collide(marine, enemy.ptr()));
        assert_eq!((*enemy.ptr()).__bindgen_anon_2.crew_level, 0);
        assert_eq!((*enemy.ptr()).__bindgen_anon_1.life_span, 0);
        assert_eq!((*marine).state_flags & NONSOLID as u16, 0);
        assert_ne!(
            (*marine).preprocess_func.unwrap() as usize,
            intruder_preprocess as *const () as usize
        );
        assert_eq!((*marine).__bindgen_anon_4.turn_wait & 15, 5);
    }
}

#[test]
fn marine_collision_ignores_finite_targets_dead_marines_and_in_progress_collisions() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        for (life, marine_flags, target_flags) in [
            (0, IGNORE_SIMILAR | CREW_OBJECT, PLAYER_SHIP),
            (1, IGNORE_SIMILAR | CREW_OBJECT | NONSOLID, PLAYER_SHIP),
            (1, IGNORE_SIMILAR | CREW_OBJECT | COLLISION, PLAYER_SHIP),
            (1, IGNORE_SIMILAR | CREW_OBJECT, PLAYER_SHIP | FINITE_LIFE),
        ] {
            (*marine).__bindgen_anon_1.life_span = life;
            (*marine).state_flags = marine_flags as u16;
            (*marine).__bindgen_anon_4.turn_wait = 0x90;
            (*enemy.ptr()).state_flags = target_flags as u16;
            with_rng(5, 0, || collide(marine, enemy.ptr()));
            assert_eq!(enemy.desc.ship_info.crew_level, 16);
            assert_eq!((*marine).state_flags, marine_flags as u16);
            assert_eq!((*marine).__bindgen_anon_4.turn_wait, 0x90);
        }
    }
}

#[test]
fn intruder_rng_byte_boundaries_distinguish_death_damage_and_no_action() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        collide(marine, enemy.ptr());
        let intruder_frame = (*marine).next.image.frame;
        for byte in [0, 15, 16, 143, 144, 255] {
            (*marine).__bindgen_anon_1.life_span = 1;
            (*marine).state_flags = (IGNORE_SIMILAR | NONSOLID) as u16;
            (*marine).__bindgen_anon_5.thrust_wait = 1;
            enemy.desc.ship_info.crew_level = 10;
            (*enemy.ptr()).__bindgen_anon_2.crew_level = 10;
            (*enemy.ptr()).next.location = POINT { x: -123, y: 456 };
            let seed = seed_for_byte(byte);
            with_rng(seed, 1, || intruder_preprocess(marine));
            assert_eq!((*marine).__bindgen_anon_1.life_span, u16::from(byte >= 16));
            assert_eq!((*marine).state_flags & DISAPPEARING as u16 != 0, byte < 16);
            let damaged = (16..144).contains(&byte);
            assert_eq!(
                enemy.desc.ship_info.crew_level,
                if damaged { 9 } else { 10 }
            );
            assert_eq!(
                (*marine).__bindgen_anon_5.thrust_wait,
                if damaged { 13 } else { 12 }
            );
            assert_eq!(
                (*marine).next.image.frame,
                intruder_frame,
                "silhouette changes do not replace marine identity frame"
            );
            assert_eq!(
                ((*marine).next.location.x, (*marine).next.location.y),
                (-123, 456)
            );
            assert_eq!(
                (*marine).preprocess_func.unwrap() as usize,
                intruder_preprocess as *const () as usize
            );
        }
    }
}

#[test]
fn intruder_wait_resets_attack_silhouette_without_rng_then_counts_down() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        collide(marine, enemy.ptr());
        (*marine).__bindgen_anon_5.thrust_wait = 13;
        with_rng(8, 0, || intruder_preprocess(marine));
        assert_eq!((*marine).__bindgen_anon_5.thrust_wait, 11);
        assert_eq!(enemy.desc.ship_info.crew_level, 15);
        for expected in (1..=10).rev() {
            with_rng(8, 0, || intruder_preprocess(marine));
            assert_eq!((*marine).__bindgen_anon_5.thrust_wait, expected);
        }
        let seed = seed_for_byte(16);
        enemy.desc.ship_info.crew_level = 1;
        (*enemy.ptr()).__bindgen_anon_2.crew_level = 1;
        with_rng(seed, 1, || intruder_preprocess(marine));
        assert_eq!((*enemy.ptr()).__bindgen_anon_1.life_span, 0);
        assert_eq!(
            (*marine).__bindgen_anon_1.life_span,
            1,
            "return begins on the next preprocess"
        );
        with_rng(seed, 1, || intruder_preprocess(marine));
        assert_eq!((*marine).next.image.frame, ship.special_frames.frame(21));
        assert_eq!((*marine).state_flags & NONSOLID as u16, 0);
    }
}

#[test]
fn intruder_returns_when_enemy_is_dead_or_absent_and_randomizes_facing_once() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        collide(marine, enemy.ptr());
        for absent in [false, true] {
            (*enemy.ptr()).__bindgen_anon_2.crew_level = 0;
            if absent {
                RemoveElement(enemy.handle);
            }
            for seed in 1..17 {
                (*marine).state_flags = (IGNORE_SIMILAR | NONSOLID) as u16;
                (*marine).__bindgen_anon_5.thrust_wait = 0;
                (*marine).preprocess_func = Some(intruder_preprocess);
                (*prim(marine)).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                TFB_SeedRandom(seed);
                let facing = normalize_facing(i32::from(TFB_Random() as BYTE));
                with_rng(seed, 1, || intruder_preprocess(marine));
                assert_eq!((*marine).__bindgen_anon_4.turn_wait, (facing << 4) as BYTE);
                assert_eq!((*marine).__bindgen_anon_5.thrust_wait, 0);
                assert_eq!(
                    (*marine).state_flags,
                    (IGNORE_SIMILAR | CHANGING | CREW_OBJECT) as u16
                );
                assert_eq!((*prim(marine)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
                assert_eq!((*marine).current.image.frame, ship.special_frames.frame(21));
                assert_eq!((*marine).next.image.frame, ship.special_frames.frame(21));
                assert_ne!(
                    (*marine).preprocess_func.unwrap() as usize,
                    intruder_preprocess as *const () as usize
                );
            }
            if absent {
                put_element(enemy.handle);
            }
        }
    }
}

#[test]
fn returning_marine_tracks_owner_cannot_board_again_and_replenishes_crew() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        collide(marine, enemy.ptr());
        (*enemy.ptr()).__bindgen_anon_2.crew_level = 0;
        intruder_preprocess(marine);
        (*marine).current.location = POINT { x: 4000, y: 4500 };
        (*marine).__bindgen_anon_4.turn_wait = 0;
        with_rng(10, 0, || ((*marine).preprocess_func.unwrap())(marine));
        assert_eq!((*marine).state_flags & IGNORE_SIMILAR as u16, 0);
        assert_eq!((*marine).next.image.frame, ship.special_frames.frame(21));
        (*enemy.ptr()).__bindgen_anon_2.crew_level = 15;
        collide(marine, enemy.ptr());
        assert_eq!(enemy.desc.ship_info.crew_level, 15);
        assert_eq!((*marine).state_flags & NONSOLID as u16, 0);
        with_rng(10, 0, || collide(marine, ship.ptr()));
        assert_eq!(ship.desc.ship_info.crew_level, 16);
        assert_eq!((*ship.ptr()).__bindgen_anon_2.crew_level, 16);
        assert_eq!((*marine).__bindgen_anon_1.life_span, 0);
    }
}

#[test]
fn owner_death_kills_both_free_and_boarded_marines_without_rng() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let free = ship.launch();
        let boarded = ship.launch();
        collide(boarded, enemy.ptr());
        (*ship.ptr()).__bindgen_anon_2.crew_level = 0;
        (*ship.ptr()).__bindgen_anon_1.life_span = 1;
        (*ship.ptr()).state_flags |= (FINITE_LIFE | NONSOLID) as u16;
        with_rng(9, 0, || ((*free).preprocess_func.unwrap())(free));
        assert_eq!((*free).__bindgen_anon_1.life_span, 0);
        assert_eq!((*free).__bindgen_anon_4.turn_wait, 0);
        assert_eq!(
            (*free).state_flags & (DISAPPEARING | NONSOLID) as u16,
            (DISAPPEARING | NONSOLID) as u16
        );
        with_rng(9, 0, || intruder_preprocess(boarded));
        assert_eq!((*boarded).__bindgen_anon_1.life_span, 0);
        assert_ne!((*boarded).state_flags & DISAPPEARING as u16, 0);
        assert_eq!(
            (*boarded).preprocess_func.unwrap() as usize,
            intruder_preprocess as *const () as usize
        );
    }
}

#[test]
fn marine_thrust_restores_owner_state_and_inserts_preprocessed_five_color_ion_trail() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        (*marine).current.location = POINT { x: 4000, y: 4500 };
        // Returning marine heads straight at its owner and ignores the enemy.
        (*marine).state_flags &= !(IGNORE_SIMILAR as u16);
        (*marine).__bindgen_anon_4.turn_wait = 0;
        ship.ss.ShipFacing = 7;
        ship.ss.cur_status_flags = (SPECIAL | SHIP_IN_GRAVITY_WELL) as u16;
        let preprocess = (*marine).preprocess_func.unwrap();
        for (speed, trail) in [(8, true), (16, true), (24, true), (32, true), (32, false)] {
            let before = queued().len();
            with_rng(9, 0, || preprocess(marine));
            assert_eq!(
                velocity_fields(&(*marine).velocity),
                velocity_toward(speed, 0)
            );
            assert_eq!(queued().len(), before + usize::from(trail));
            assert_eq!(ship.ss.ShipFacing, 7);
            assert_eq!(
                ship.ss.cur_status_flags,
                (SPECIAL | SHIP_IN_GRAVITY_WELL) as u16
            );
            assert_eq!(ship.desc.characteristics.max_thrust, 35);
            assert_eq!(ship.desc.characteristics.thrust_increment, 5);
        }
        assert_eq!(
            (*marine).__bindgen_anon_5.thrust_wait,
            (SHIP_AT_MAX_SPEED >> 6) as BYTE
        );
        let ion = element(disp_q.head);
        assert_eq!((*ion).playerNr, NEUTRAL_PLAYER_NUM as SIZE);
        assert_eq!((*ion).pParent, (&raw mut *ship.ss).cast());
        assert_eq!(
            (*ion).state_flags,
            (APPEARING | FINITE_LIFE | NONSOLID | PRE_PROCESS) as u16
        );
        assert_eq!((*ion).__bindgen_anon_1.life_span, 0);
        assert_eq!((*ion).__bindgen_anon_5.thrust_wait, 1);
        assert_eq!((*ion).colorCycleIndex, 0);
        assert_eq!((*prim(ion)).Type, gfx_object_POINT_PRIM as GRAPHICS_PRIM);
        assert_eq!(
            (
                (*prim(ion)).color.r,
                (*prim(ion)).color.g,
                (*prim(ion)).color.b
            ),
            (255, 173, 0)
        );
        assert_eq!(
            (*ion).current.location.x,
            (4000 + i32::from(cosine(32, 8))) as COORD
        );
        assert_eq!(
            (*ion).current.location.y,
            (4500 + i32::from(sine(32, 8))) as COORD
        );
        assert_eq!((*ion).next.location.y, (*ion).current.location.y);
        let cycle = (*ion).death_func.unwrap();
        for (index, rgb) in [
            (1, (255, 115, 0)),
            (2, (255, 24, 0)),
            (3, (222, 0, 0)),
            (4, (156, 0, 0)),
        ] {
            (*ion).state_flags |= DISAPPEARING as u16;
            with_rng(9, 0, || cycle(ion));
            assert_eq!((*ion).colorCycleIndex, index);
            assert_eq!((*ion).__bindgen_anon_1.life_span, 1);
            assert_eq!((*ion).state_flags & DISAPPEARING as u16, 0);
            assert_ne!((*ion).state_flags & CHANGING as u16, 0);
            assert_eq!(
                (
                    (*prim(ion)).color.r,
                    (*prim(ion)).color.g,
                    (*prim(ion)).color.b
                ),
                rgb
            );
        }
        (*ion).__bindgen_anon_1.life_span = 0;
        (*ion).state_flags |= DISAPPEARING as u16;
        with_rng(9, 0, || cycle(ion));
        assert_eq!((*ion).colorCycleIndex, 5);
        assert_eq!((*ion).__bindgen_anon_1.life_span, 0);
        assert_ne!((*ion).state_flags & DISAPPEARING as u16, 0);
    }
}

#[test]
fn marine_collision_delay_precedes_tracking_and_half_circle_turn_uses_one_draw() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        let preprocess = (*marine).preprocess_func.unwrap();
        (*marine).current.location = POINT { x: 4000, y: 4500 };
        (*marine).state_flags &= !(IGNORE_SIMILAR as u16);
        (*marine).__bindgen_anon_4.turn_wait = 0x85;
        (*marine).__bindgen_anon_5.thrust_wait = 7;
        let before = queued().len();
        for expected in (0x80..=0x84).rev() {
            with_rng(5, 0, || preprocess(marine));
            assert_eq!((*marine).__bindgen_anon_4.turn_wait, expected);
            assert_eq!((*marine).__bindgen_anon_5.thrust_wait, 7);
            assert_eq!(queued().len(), before);
            assert_eq!(velocity_fields(&(*marine).velocity), [0; 9]);
        }
        (*marine).__bindgen_anon_5.thrust_wait = 0;
        for seed in 1..17 {
            (*marine).__bindgen_anon_4.turn_wait = 0x80;
            TFB_SeedRandom(seed);
            let step = ((TFB_Random() as BYTE & 1) as i32) * 2 - 1;
            with_rng(seed, 1, || preprocess(marine));
            assert_eq!(
                (*marine).__bindgen_anon_4.turn_wait,
                (normalize_facing(8 + step) << 4) as BYTE
            );
            assert_eq!(queued().len(), before);
        }
    }
}

#[test]
fn marine_prediction_uses_wrapped_distance_but_unwrapped_future_position_with_size_casts() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        let preprocess = (*marine).preprocess_func.unwrap();
        (*marine).state_flags &= !(IGNORE_SIMILAR as u16);
        for (marine_x, owner_x) in [(8000, 100), (-30000, 30000), (30000, -30000)] {
            (*marine).current.location = POINT {
                x: marine_x,
                y: 4000,
            };
            ship.at(owner_x, 4000);
            (*marine).__bindgen_anon_4.turn_wait = 0;
            (*marine).__bindgen_anon_5.thrust_wait = 0;
            let desired = normalize_facing(angle_to_facing(i32::from(ARCTAN(
                (i32::from(owner_x) - i32::from(marine_x)) as SIZE,
                0,
            ))));
            assert!(desired == 4 || desired == 12);
            with_rng(4, 0, || preprocess(marine));
            let expected = if desired == 4 { 1 } else { 15 };
            assert_eq!((*marine).__bindgen_anon_4.turn_wait, expected << 4);
        }
        ship.at(4000, 4000);
        (*marine).current.location = POINT { x: 4000, y: 4500 };
        SetVelocityVector(&raw mut (*ship.ptr()).velocity, 64, 4);
        (*marine).__bindgen_anon_4.turn_wait = 0;
        with_rng(4, 0, || preprocess(marine));
        // floor(500 / 64) = 7 turns of eastward target movement.
        assert_eq!((*marine).__bindgen_anon_4.turn_wait, 0x10);
    }
}

#[test]
fn marine_gravity_avoidance_uses_mass_threshold_distance_and_one_draw_for_head_on() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        (*marine).current.location = POINT { x: 4000, y: 4000 };
        let planet_h = AllocElement();
        assert!(!planet_h.is_null());
        let planet = element(planet_h);
        (*planet).__bindgen_anon_3.mass_points = 101;
        (*planet).current.location = POINT { x: 4100, y: 4000 };
        put_element(planet_h);
        let face = normalize_facing(angle_to_facing(i32::from(ARCTAN(100, 0))));
        let preprocess = (*marine).preprocess_func.unwrap();
        let before = queued().len();
        for seed in 1..17 {
            (*marine).__bindgen_anon_4.turn_wait = (face << 4) as BYTE;
            (*marine).__bindgen_anon_5.thrust_wait = 0;
            SetVelocityVector(&raw mut (*marine).velocity, 8, face);
            TFB_SeedRandom(seed);
            let step = ((TFB_Random() as SIZE & 1) as i32) * 2 - 1;
            with_rng(seed, 1, || preprocess(marine));
            assert_eq!(
                (*marine).__bindgen_anon_4.turn_wait,
                (normalize_facing(i32::from(face) + step) << 4) as BYTE
            );
            assert_ne!(
                (*marine).__bindgen_anon_5.thrust_wait & (SHIP_IN_GRAVITY_WELL >> 6) as BYTE,
                0
            );
            assert_eq!(queued().len(), before);
        }
        for (distance, in_well) in [(1020, true), (1021, false)] {
            (*planet).current.location.x = 4000 + distance;
            (*marine).__bindgen_anon_4.turn_wait = (face << 4) as BYTE;
            (*marine).__bindgen_anon_5.thrust_wait = 0;
            let draws = usize::from(in_well);
            with_rng(3, draws, || preprocess(marine));
            assert_eq!(
                (*marine).__bindgen_anon_5.thrust_wait & (SHIP_IN_GRAVITY_WELL >> 6) as BYTE != 0,
                in_well
            );
        }
    }
}

#[test]
fn ai_turret_half_circle_consumes_trackship_draw_then_its_own_tie_break() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let turret = ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4000, 4500);
        (*turret).current.location = POINT { x: 4000, y: 4000 };
        ship.ss.special_counter = 2;
        ship.ss.weapon_counter = 1;
        ship.ss.ShipFacing = 0;
        for seed in 1..33 {
            (*turret).__bindgen_anon_4.turn_wait = 0;
            (*turret).__bindgen_anon_5.thrust_wait = 0;
            (*turret).hTarget = ptr::null_mut();
            ship.ss.ship_input_state = 0;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
            TFB_SeedRandom(seed);
            TFB_Random();
            let second_bit = TFB_Random() as BYTE & 1;
            with_rng(seed, 2, || intelligence(&mut ship, &mut concerns));
            assert_eq!(
                ship.ss.ship_input_state,
                (SPECIAL | if second_bit == 0 { RIGHT } else { LEFT }) as BYTE
            );
            assert_eq!((*turret).__bindgen_anon_4.turn_wait, 0);
            assert_eq!((*turret).hTarget, enemy.handle);
        }
    }
}

#[test]
fn ai_launch_budget_obeys_crew_point_defense_maneuverability_turn_and_marine_count() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let a = ship.launch();
        let b = ship.launch();
        let turret = element((*ship.ptr()).succ);
        (*turret).__bindgen_anon_4.turn_wait = 1;
        ship.ss.weapon_counter = 1;
        for (counter, crew, defense, maneuver, turn, active, expected) in [
            (0, 5, false, 24, 13, 2, true),
            (0, 5, false, 25, 13, 2, false),
            (0, 5, false, 25, 12, 2, true),
            (0, 5, false, 25, 13, 1, true),
            (0, 4, false, 0, 1, 1, false),
            (0, 5, true, 0, 1, 1, false),
            (2, 5, false, 0, 1, 1, false),
        ] {
            ship.ss.special_counter = counter;
            ship.ss.ship_input_state = SPECIAL as BYTE;
            ship.desc.ship_info.crew_level = crew;
            enemy.desc.ship_info.ship_flags = if defense { POINT_DEFENSE as u16 } else { 0 };
            enemy.desc.cyborg_control.ManeuverabilityIndex = maneuver;
            (*a).__bindgen_anon_1.life_span = 1;
            (*b).__bindgen_anon_1.life_span = u16::from(active == 2);
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize] = EVALUATE_DESC {
                ObjectPtr: enemy.ptr(),
                facing: 0,
                which_turn: turn,
                MoveState: MOVEMENT_STATE_ENTICE,
            };
            with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(
                ship.ss.ship_input_state,
                if expected {
                    (SPECIAL | WEAPON) as BYTE
                } else {
                    0
                }
            );
        }
        ship.ss.special_counter = 1;
        ship.ss.ship_input_state = (SPECIAL | LEFT) as BYTE;
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
        with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
        assert_eq!(ship.ss.ship_input_state, (SPECIAL | LEFT) as BYTE);
        concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = ptr::null_mut();
        with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
        assert_eq!(ship.ss.ship_input_state, LEFT as BYTE);
    }
}

#[test]
fn ai_entice_coasting_clears_hull_turn_only_at_c_angle_boundaries() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        let turret = ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        (*turret).__bindgen_anon_4.turn_wait = 1;
        ship.ss.special_counter = 2;
        ship.ss.weapon_counter = 1;
        let direction = ARCTAN(500, 0);
        for (difference, turn, thrust, fast, expected_clear) in [
            (11, 23, false, true, false),
            (12, 23, false, true, true),
            (51, 23, false, true, true),
            (52, 23, false, true, false),
            (53, 23, false, true, false),
            (32, 24, false, true, false),
            (32, 23, true, true, false),
            (32, 23, false, false, false),
        ] {
            (*ship.ptr()).velocity.travel_angle =
                ((u32::from(direction) + difference) & 63) as COUNT;
            ship.ss.cur_status_flags = if fast { SHIP_AT_MAX_SPEED as u16 } else { 0 };
            ship.ss.ship_input_state = (LEFT | if thrust { THRUST } else { 0 }) as BYTE;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize] = EVALUATE_DESC {
                ObjectPtr: enemy.ptr(),
                facing: 0,
                which_turn: turn,
                MoveState: MOVEMENT_STATE_ENTICE,
            };
            with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(ship.ss.ship_input_state & LEFT as BYTE == 0, expected_clear);
        }
    }
}

#[test]
fn exhausted_queue_preserves_weapon_return_count_and_still_rotates_turret() {
    let _battle = Battle::with_capacity(2);
    unsafe {
        let mut ship = Ship::new(0);
        let turret = ship.install_turret();
        ship.ss.cur_status_flags = (SPECIAL | LEFT) as u16;
        let mut out = 1usize as HELEMENT;
        with_rng(6, 0, || {
            assert_eq!(
                (ship.desc.init_weapon_func.unwrap())(ship.ptr(), &mut out),
                1
            )
        });
        assert!(out.is_null());
        assert_eq!((*turret).__bindgen_anon_4.turn_wait, 4);
        assert_eq!((*turret).__bindgen_anon_5.thrust_wait, 255);
        assert_eq!(queued(), vec![ship.handle, turret.cast()]);
    }
}

#[test]
fn failed_initial_turret_allocation_does_not_change_ship_or_counters() {
    let _battle = Battle::with_capacity(1);
    unsafe {
        let mut ship = Ship::new(0);
        (*ship.ptr()).state_flags |= APPEARING as u16;
        ship.ss.weapon_counter = 7;
        ship.ss.special_counter = 9;
        with_rng(4, 0, || (ship.desc.preprocess_func.unwrap())(ship.ptr()));
        assert_eq!(queued(), vec![ship.handle]);
        assert_eq!((*ship.ptr()).state_flags, (PLAYER_SHIP | APPEARING) as u16);
        assert_eq!(ship.ss.weapon_counter, 7);
        assert_eq!(ship.ss.special_counter, 9);
    }
}

#[test]
fn failed_replacement_and_marine_allocation_leave_crew_and_special_budget_untouched() {
    for capacity in [3, 4] {
        let _battle = Battle::with_capacity(capacity);
        unsafe {
            let mut ship = Ship::new(0);
            let old = ship.install_turret();
            let mut enemy = Ship::new(1);
            enemy.at(4500, 4000);
            ship.ss.cur_status_flags = (SPECIAL | WEAPON) as u16;
            (*old).__bindgen_anon_1.life_span = 0;
            // Facing 0 when replacement allocation fails; facing 0's angle when it succeeds.
            with_rng(3, 0, || ((*old).postprocess_func.unwrap())(old));
            assert_eq!(ship.desc.ship_info.crew_level, 16);
            assert_eq!((*ship.ptr()).__bindgen_anon_2.crew_level, 16);
            assert_eq!(ship.ss.special_counter, 0);
            assert_eq!((*prim(old)).Type, gfx_object_NUM_PRIMS as GRAPHICS_PRIM);
            assert_eq!(queued().len(), capacity as usize);
            assert_eq!(
                (*ship.ptr()).hTarget,
                enemy.handle,
                "tracking eligibility runs before failed marine allocation"
            );
        }
    }
}

#[test]
fn failed_muzzle_flash_allocation_does_not_recoil_and_failed_ion_allocation_still_thrusts() {
    {
        let _battle = Battle::with_capacity(4);
        unsafe {
            let mut ship = Ship::new(0);
            ship.install_turret();
            ship.ss.cur_status_flags = WEAPON as u16;
            let h = ship.missile();
            put_element(h);
            let turret = ship.tick_turret();
            assert_eq!(queued().len(), 3);
            assert_eq!(
                ((*turret).current.location.x, (*turret).current.location.y),
                (4000, 4000)
            );
            assert_eq!({ disp_q.tail }, h);
        }
    }
    {
        let _battle = Battle::with_capacity(5);
        unsafe {
            let mut ship = Ship::new(0);
            ship.install_turret();
            let mut enemy = Ship::new(1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            let filler = AllocElement();
            assert!(!filler.is_null());
            put_element(filler);
            (*marine).state_flags &= !(IGNORE_SIMILAR as u16);
            (*marine).current.location = POINT { x: 4000, y: 4500 };
            (*marine).__bindgen_anon_4.turn_wait = 0;
            let before = queued();
            with_rng(1, 0, || ((*marine).preprocess_func.unwrap())(marine));
            assert_eq!(queued(), before);
            assert_eq!(velocity_fields(&(*marine).velocity), velocity_toward(8, 0));
            assert_eq!(ship.desc.characteristics.max_thrust, 35);
            assert_eq!(ship.desc.characteristics.thrust_increment, 5);
        }
    }
}

#[test]
fn marine_prefers_visible_enemy_then_returns_to_visible_owner_or_keeps_heading() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        enemy.at(4500, 4500);
        (*marine).current.location = POINT { x: 4000, y: 4500 };
        let preprocess = (*marine).preprocess_func.unwrap();
        for (enemy_visible, owner_visible, expected_facing, returning) in [
            (true, true, 1, false),
            (false, true, 0, true),
            (false, false, 0, false),
        ] {
            (*prim(enemy.ptr())).Type = if enemy_visible {
                gfx_object_STAMP_PRIM
            } else {
                gfx_object_NUM_PRIMS
            } as GRAPHICS_PRIM;
            (*prim(ship.ptr())).Type = if owner_visible {
                gfx_object_STAMP_PRIM
            } else {
                gfx_object_NUM_PRIMS
            } as GRAPHICS_PRIM;
            (*marine).state_flags = (IGNORE_SIMILAR | CREW_OBJECT) as u16;
            (*marine).__bindgen_anon_4.turn_wait = 0;
            (*marine).__bindgen_anon_5.thrust_wait = 0;
            (*marine).velocity = mem::zeroed();
            (*marine).next.image.frame = ship.special_frames.frame(20);
            let before = queued().len();
            with_rng(8, 0, || preprocess(marine));
            assert_eq!((*marine).__bindgen_anon_4.turn_wait, expected_facing << 4);
            assert_eq!(
                (*marine).state_flags & IGNORE_SIMILAR as u16 == 0,
                returning
            );
            assert_eq!(
                (*marine).next.image.frame,
                ship.special_frames.frame(if returning { 21 } else { 20 })
            );
            assert_eq!(queued().len(), before + usize::from(returning));
            assert_eq!(
                velocity_fields(&(*marine).velocity),
                if returning {
                    velocity_toward(8, 0)
                } else {
                    [0; 9]
                }
            );
        }
    }
}

#[test]
fn marine_without_target_clears_stale_gravity_and_decelerates_only_when_overspeed() {
    let _battle = Battle::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.install_turret();
        let mut enemy = Ship::new(1);
        enemy.at(4500, 4000);
        let marine = ship.launch();
        (*prim(ship.ptr())).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
        (*prim(enemy.ptr())).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
        let preprocess = (*marine).preprocess_func.unwrap();
        for overspeed in [false, true] {
            (*marine).__bindgen_anon_4.turn_wait = 0;
            (*marine).__bindgen_anon_5.thrust_wait = ((SHIP_IN_GRAVITY_WELL
                | if overspeed { SHIP_BEYOND_MAX_SPEED } else { 0 })
                >> 6) as BYTE;
            SetVelocityVector(&raw mut (*marine).velocity, 40, 8);
            let before = queued().len();
            with_rng(9, 0, || preprocess(marine));
            assert_eq!(
                (*marine).__bindgen_anon_5.thrust_wait & (SHIP_IN_GRAVITY_WELL >> 6) as BYTE,
                0
            );
            assert_eq!((*marine).__bindgen_anon_4.turn_wait, 0);
            assert_eq!(
                velocity_fields(&(*marine).velocity),
                velocity_toward(if overspeed { 32 } else { 40 }, 8)
            );
            assert_eq!(queued().len(), before + usize::from(overspeed));
            assert_eq!(ship.ss.ShipFacing, 0);
            assert_eq!(ship.desc.characteristics.max_thrust, 35);
        }
    }
}
