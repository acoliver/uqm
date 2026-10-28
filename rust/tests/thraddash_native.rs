//! Live Thraddash callbacks, using the engine queue, frames, velocity and RNG.

use uqm_rust::ships::races::init_thraddash;
#[path = "support/battle_context.rs"]
mod battle_context;

use std::{
    ffi::{c_void, CStr},
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::{
    abi::*, cosine, element, facing_to_angle, frame_index, put_element, sine,
};

unsafe extern "C" {
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
    static mut DisplayArray: [PRIMITIVE; 330];
    static mut _GraphicsStatusFlags: BYTE;
    fn TFB_DrawCanvas_New_Paletted(
        w: i32,
        h: i32,
        palette: *const Color,
        transparent: i32,
    ) -> *mut c_void;
    fn TFB_DrawCanvas_SetPixelIndexes(
        canvas: *mut c_void,
        pixels: *const BYTE,
        w: i32,
        h: i32,
    ) -> BOOLEAN;
    fn TFB_DrawImage_New(canvas: *mut c_void) -> *mut c_void;
    fn TFB_DrawImage_Delete(image: *mut c_void);
}

static ENGINE: Mutex<()> = Mutex::new(());

struct Battle {
    _guard: MutexGuard<'static, ()>,
    display: *mut PRIMITIVE,
    width: i32,
    height: i32,
    graphics: BYTE,
}

impl Battle {
    fn new(capacity: COUNT) -> Self {
        let guard = ENGINE.lock().unwrap();
        unsafe {
            let display = GlobData.Game_state.DisplayArray;
            let width = ScreenWidth;
            let height = ScreenHeight;
            let graphics = _GraphicsStatusFlags;
            _GraphicsStatusFlags &= !4;
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
                graphics,
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
            _GraphicsStatusFlags = self.graphics;
        }
    }
}

// Engine-readable FRAME_DESC and DRAWABLE_DESC, as in arilou_native tests.
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
    image: *mut c_void,
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
            image: ptr::null_mut(),
        }
    }

    fn frame(&self, index: usize) -> FRAME {
        (&raw const self.frames[index]).cast_mut().cast()
    }

    unsafe fn make_opaque(&mut self) {
        let canvas = TFB_DrawCanvas_New_Paletted(4, 4, ptr::null(), -1);
        assert!(!canvas.is_null());
        assert_ne!(
            TFB_DrawCanvas_SetPixelIndexes(canvas, [1u8; 16].as_ptr(), 4, 4),
            0
        );
        self.image = TFB_DrawImage_New(canvas);
        assert!(!self.image.is_null());
        for frame in &mut self.frames {
            frame.image = self.image;
        }
    }
}

impl Drop for FrameSet {
    fn drop(&mut self) {
        if !self.image.is_null() {
            unsafe {
                TFB_DrawImage_Delete(self.image);
            }
        }
    }
}

struct Ship {
    desc: Box<RACE_DESC>,
    ss: Box<STARSHIP>,
    handle: HELEMENT,
    _ship_frames: FrameSet,
    horn_frames: FrameSet,
    napalm_frames: FrameSet,
}

impl Ship {
    unsafe fn new(player: SIZE) -> Self {
        let mut desc = Box::new(*init_thraddash());
        let ship_frames = FrameSet::new(16);
        let horn_frames = FrameSet::new(16);
        let napalm_frames = FrameSet::new(10);
        desc.ship_data.ship = [ship_frames.frame(0); 3];
        desc.ship_data.weapon = [horn_frames.frame(0); 3];
        desc.ship_data.special = [napalm_frames.frame(0); 3];
        let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
        ss.RaceDescPtr = &raw mut *desc;
        ss.playerNr = player;
        let handle = AllocElement();
        assert!(!handle.is_null());
        let e = element(handle);
        (*e).playerNr = player;
        (*e).pParent = (&raw mut *ss).cast();
        (*e).state_flags = PLAYER_SHIP as u16;
        (*e).collision_func = Some(uqm_rust::engine::battle::CollisionHook::Native(
            uqm_rust::battle::ship_native::collision,
        ));
        (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as u16;
        (*e).__bindgen_anon_2.crew_level = 8;
        (*e).__bindgen_anon_3.mass_points = 7;
        (*e).current.location = POINT { x: 4000, y: 4000 };
        (*e).current.image.frame = ship_frames.frame(0);
        (*e).current.image.farray = (&raw mut desc.ship_data.ship).cast();
        (*e).next = (*e).current;
        (*prim(e)).Type = gfx_object_STAMP_PRIM as u8;
        ss.hShip = handle;
        put_element(handle);
        Self {
            desc,
            ss,
            handle,
            _ship_frames: ship_frames,
            horn_frames,
            napalm_frames,
        }
    }

    fn ptr(&self) -> *mut ELEMENT {
        self.handle.cast()
    }

    unsafe fn preprocess(&self) {
        self.desc.preprocess_func.unwrap()(self.ptr());
    }

    unsafe fn fire(&self) -> HELEMENT {
        let mut h = ptr::null_mut();
        assert_eq!(self.desc.init_weapon_func.unwrap()(self.ptr(), &mut h), 1);
        h
    }

    unsafe fn burn(&mut self) -> *mut ELEMENT {
        self.ss.cur_status_flags |= SPECIAL as u16;
        let head = disp_q.head;
        self.preprocess();
        let trail = disp_q.head;
        assert_ne!(trail, head);
        element(trail)
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

unsafe fn velocity(e: *mut ELEMENT) -> (SIZE, SIZE) {
    let (mut x, mut y) = (0, 0);
    GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut x, &mut y);
    (x, y)
}

unsafe fn with_rng<T>(seed: DWORD, used: usize, action: impl FnOnce() -> T) -> T {
    TFB_SeedRandom(seed);
    for _ in 0..used {
        TFB_Random();
    }
    let expected = TFB_Random();
    TFB_SeedRandom(seed);
    let result = action();
    assert_eq!(TFB_Random(), expected, "RNG consumption: {used}");
    result
}

fn concerns() -> [EVALUATE_DESC; 4] {
    [EVALUATE_DESC {
        ObjectPtr: ptr::null_mut(),
        facing: 0,
        which_turn: 0,
        MoveState: MOVEMENT_STATE_NO_MOVEMENT,
    }; 4]
}

unsafe fn think(ship: &mut Ship, c: &mut [EVALUATE_DESC; 4], input: u8, count: COUNT) -> u8 {
    ship.ss.ship_input_state = input;
    ship.desc.cyborg_control.intelligence_func.unwrap()(ship.ptr(), c.as_mut_ptr(), count);
    ship.ss.ship_input_state
}

#[test]
fn descriptor_matches_all_c_initializer_fields_and_resources() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let p = init_thraddash();
            let d = *p;
            assert_eq!(p, init_thraddash());
            assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(d.ship_info.ship_cost, 10);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (8, 8));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (24, 24));
            assert_eq!(
                CStr::from_ptr(d.ship_info.race_strings_rsc),
                c"ship.thraddash.text"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.icons_rsc),
                c"ship.thraddash.icons"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.melee_icon_rsc),
                c"ship.thraddash.meleeicons"
            );
            assert!(
                d.ship_info.race_strings.is_null()
                    && d.ship_info.icons.is_null()
                    && d.ship_info.melee_icon.is_null()
            );
            assert_eq!(d.fleet.strength, 150);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (2535, 8358));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (28, 7, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (2, 1, 6)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (1, 0, 12, 0, 7)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                for (rsc, name) in [
                    (d.ship_data.ship_rsc[i], "torch"),
                    (d.ship_data.weapon_rsc[i], "horn"),
                    (d.ship_data.special_rsc[i], "napalm"),
                ] {
                    assert_eq!(
                        CStr::from_ptr(rsc).to_str().unwrap(),
                        format!("ship.thraddash.graphics.{name}.{size}")
                    );
                }
                assert!(
                    d.ship_data.ship[i].is_null()
                        && d.ship_data.weapon[i].is_null()
                        && d.ship_data.special[i].is_null()
                );
            }
            let captain = d.ship_data.captain_control;
            assert_eq!(
                CStr::from_ptr(captain.captain_rsc),
                c"ship.thraddash.graphics.captain"
            );
            assert!(
                captain.background.is_null()
                    && captain.turn.is_null()
                    && captain.thrust.is_null()
                    && captain.weapon.is_null()
                    && captain.special.is_null()
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.victory_ditty_rsc),
                c"ship.thraddash.ditty"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_sounds_rsc),
                c"ship.thraddash.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_null());
            assert_eq!(
                (
                    d.cyborg_control.ManeuverabilityIndex,
                    d.cyborg_control.WeaponRange
                ),
                (0, 900)
            );
            assert!(
                d.preprocess_func.is_some()
                    && d.init_weapon_func.is_some()
                    && d.cyborg_control.intelligence_func.is_some()
            );
            assert!(
                d.postprocess_func.is_none()
                    && d.uninit_func.is_none()
                    && d.data.is_null()
                    && d.code_ref.is_null()
            );
            (*p).preprocess_func = None;
            (*p).init_weapon_func = None;
            (*p).cyborg_control.intelligence_func = None;
            init_thraddash();
            assert_eq!(
                (*p).preprocess_func.map(|f| f as usize),
                d.preprocess_func.map(|f| f as usize)
            );
            assert_eq!(
                (*p).init_weapon_func.map(|f| f as usize),
                d.init_weapon_func.map(|f| f as usize)
            );
            assert_eq!(
                (*p).cyborg_control.intelligence_func.map(|f| f as usize),
                d.cyborg_control.intelligence_func.map(|f| f as usize)
            );
        }
    })
}

#[test]
fn horn_uses_next_location_exact_facing_speed_and_offsets_without_inheriting_velocity() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(1);
            SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 1000, -1000);
            (*ship.ptr()).next.location = POINT {
                x: i16::MAX - 5,
                y: i16::MIN + 5,
            };
            for facing in 0..16 {
                ship.ss.ShipFacing = facing;
                let h = with_rng(101, 0, || ship.fire());
                assert!(!h.is_null());
                let e = element(h);
                let angle = facing_to_angle(i32::from(facing));
                let vx = cosine(angle, 120 << VELOCITY_SHIFT);
                let vy = sine(angle, 120 << VELOCITY_SHIFT);
                assert_eq!(velocity(e), (vx, vy));
                assert_eq!(
                    (*e).current.location.x,
                    (i32::from(i16::MAX) - 5 + i32::from(cosine(angle, 36))
                        - (i32::from(vx) >> VELOCITY_SHIFT)) as i16
                );
                assert_eq!(
                    (*e).current.location.y,
                    (i32::from(i16::MIN) + 5 + i32::from(sine(angle, 36))
                        - (i32::from(vy) >> VELOCITY_SHIFT)) as i16
                );
                assert_eq!(
                    (*e).current.image.frame,
                    ship.horn_frames.frame(facing as usize)
                );
                assert_eq!(
                    (*e).current.image.farray,
                    (&raw mut ship.desc.ship_data.weapon).cast()
                );
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!((*e).__bindgen_anon_1.life_span, 15);
                assert_eq!((*e).__bindgen_anon_2.hit_points, 2);
                assert_eq!((*e).__bindgen_anon_3.mass_points, 1);
                assert_eq!((*e).__bindgen_anon_5.blast_offset, 3);
                assert_eq!((*e).playerNr, 1);
                assert!(
                    (*e).preprocess_func.is_none()
                        && (*e).postprocess_func.is_none()
                        && (*e).death_func.is_none()
                );
                assert!((*e).collision_func.is_some());
                assert!((*e).pParent.is_null());
                assert_eq!((*prim(e)).Type, gfx_object_STAMP_PRIM as u8);
                assert_eq!(queued(), vec![ship.handle]);
                FreeElement(h);
            }
        }
    })
}

#[test]
fn failed_horn_and_napalm_allocation_keep_the_c_return_and_energy_semantics() {
    battle_context::run(|| {
        let _battle = Battle::new(1);
        unsafe {
            let mut ship = Ship::new(0);
            assert!(with_rng(2, 0, || ship.fire()).is_null());
            ship.ss.cur_status_flags = SPECIAL as u16;
            with_rng(2, 0, || ship.preprocess());
            assert_eq!(ship.desc.ship_info.energy_level, 23);
            assert_eq!(ship.ss.energy_counter, 6);
            assert_eq!(velocity(ship.ptr()), (0, -12 * 32));
            assert_eq!((*ship.ptr()).__bindgen_anon_5.thrust_wait, 1);
            assert_eq!(queued(), vec![ship.handle]);
            assert_eq!(
                (
                    ship.desc.characteristics.max_thrust,
                    ship.desc.characteristics.thrust_increment
                ),
                (28, 7)
            );
        }
    })
}

#[test]
fn afterburner_inserts_owned_invisible_napalm_at_the_head_in_newest_first_order() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(1);
            ship.ss.ShipFacing = 7;
            (*ship.ptr()).next.location = POINT { x: -9, y: 13 };
            let first = with_rng(31, 0, || ship.burn());
            assert_eq!(queued(), vec![first.cast(), ship.handle]);
            let second = with_rng(31, 0, || ship.burn());
            assert_eq!(queued(), vec![second.cast(), first.cast(), ship.handle]);
            for e in [first, second] {
                assert_eq!((*e).pParent, (&raw mut *ship.ss).cast());
                assert!((*e).hTarget.is_null());
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR | NONSOLID | PRE_PROCESS) as u16
                );
                assert_eq!((*e).__bindgen_anon_1.life_span, 48);
                assert_eq!((*e).__bindgen_anon_2.hit_points, 1);
                assert_eq!((*e).__bindgen_anon_3.mass_points, 2);
                assert_eq!((*e).__bindgen_anon_4.turn_wait, 5);
                assert_eq!((*e).__bindgen_anon_5.blast_offset, 0);
                assert_eq!((*e).playerNr, 1);
                assert_eq!((*e).current.image.frame, ship.napalm_frames.frame(9));
                assert_eq!((*e).next.image.frame, (*e).current.image.frame);
                assert_eq!((*e).next.image.farray, (*e).current.image.farray);
                assert_eq!(((*e).current.location.x, (*e).current.location.y), (-9, 13));
                assert_eq!(((*e).next.location.x, (*e).next.location.y), (-9, 13));
                assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
                assert_eq!((*prim(e)).Type, gfx_object_NUM_PRIMS as u8);
                assert!((*e).preprocess_func.is_some() && (*e).collision_func.is_some());
            }
            assert_eq!(ship.desc.ship_info.energy_level, 22);
        }
    })
}

#[test]
fn boost_uses_temporary_characteristics_and_preserves_the_inertial_thrust_result() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship = Ship::new(0);
            for (speed, face, old, status, wait) in [
                (0, 0, 0, 0, 0),
                (28, 0, 0, SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED, 0),
                (72, 0, SPECIAL, SHIP_AT_MAX_SPEED, 3),
                (
                    80,
                    4,
                    SPECIAL,
                    SHIP_AT_MAX_SPEED | SHIP_IN_GRAVITY_WELL,
                    255,
                ),
            ] {
                ship.ss.ShipFacing = face;
                SetVelocityVector(&raw mut (*ship.ptr()).velocity, speed, 0);
                ship.ss.old_status_flags = old as u16;
                ship.ss.cur_status_flags = (SPECIAL | THRUST | status) as u16;
                (*ship.ptr()).__bindgen_anon_5.thrust_wait = wait;
                let mut expected_desc = *ship.desc;
                expected_desc.characteristics.max_thrust = 72;
                expected_desc.characteristics.thrust_increment = 12;
                let mut expected_ss = *ship.ss;
                expected_ss.RaceDescPtr = &mut expected_desc;
                if old == 0 {
                    expected_ss.cur_status_flags &=
                        !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                }
                let mut expected = *ship.ptr();
                expected.pParent = (&raw mut expected_ss).cast();
                let thrust_status = inertial_thrust(&mut expected);
                with_rng(7, 0, || ship.burn());
                assert_eq!(
                    velocity_fields(&(*ship.ptr()).velocity),
                    velocity_fields(&expected.velocity)
                );
                assert_eq!(
                    ship.ss.cur_status_flags,
                    (SPECIAL | THRUST) as u16 | thrust_status
                );
                assert_eq!(
                    (*ship.ptr()).__bindgen_anon_5.thrust_wait,
                    if wait == 0 { 1 } else { wait }
                );
                assert_eq!(
                    (
                        ship.desc.characteristics.max_thrust,
                        ship.desc.characteristics.thrust_increment
                    ),
                    (28, 7)
                );
            }
        }
    })
}

#[test]
fn repeated_boost_reaches_72_and_release_marks_coasting_beyond_normal_speed() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(0);
            for frame in 1..=7 {
                with_rng(45, 0, || ship.burn());
                assert_eq!(velocity(ship.ptr()), (0, -32 * (frame.min(6) * 12)));
                assert_eq!(
                    ship.ss.cur_status_flags & SHIP_AT_MAX_SPEED as u16 != 0,
                    frame == 7
                );
                ship.ss.old_status_flags = ship.ss.cur_status_flags;
            }
            let before = queued();
            ship.ss.cur_status_flags &= !(SPECIAL as u16);
            ship.preprocess();
            assert_eq!(
                ship.ss.cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
            );
            assert_eq!(queued(), before);
            assert_eq!(ship.desc.ship_info.energy_level, 17);
            assert_eq!(velocity(ship.ptr()), (0, -72 * 32));
        }
    })
}

#[test]
fn no_key_and_exhausted_energy_leave_velocity_and_queue_alone() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 123, -456);
            let v = velocity_fields(&(*ship.ptr()).velocity);
            for (current, old, expected) in [
                (0, SPECIAL, 0),
                (SHIP_AT_MAX_SPEED, 0, SHIP_AT_MAX_SPEED),
                (
                    SHIP_AT_MAX_SPEED,
                    SPECIAL,
                    SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED,
                ),
            ] {
                ship.ss.cur_status_flags = current as u16;
                ship.ss.old_status_flags = old as u16;
                ship.preprocess();
                assert_eq!(ship.ss.cur_status_flags, expected as u16);
                assert_eq!(ship.desc.ship_info.energy_level, 24);
            }
            ship.desc.ship_info.energy_level = 0;
            ship.ss.energy_counter = 17;
            ship.ss.cur_status_flags = (SPECIAL | SHIP_AT_MAX_SPEED) as u16;
            with_rng(8, 0, || ship.preprocess());
            assert_eq!(
                ship.ss.cur_status_flags,
                (SPECIAL | SHIP_AT_MAX_SPEED | (1 << 5)) as u16
            );
            assert_eq!(ship.ss.energy_counter, 17);
            assert_eq!((*ship.ptr()).__bindgen_anon_5.thrust_wait, 0);
            assert_eq!(velocity_fields(&(*ship.ptr()).velocity), v);
            assert_eq!(queued(), vec![ship.handle]);
        }
    })
}

#[test]
fn napalm_materializes_without_decaying_and_initializes_the_intersection_from_next_image() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            let e = ship.burn();
            let next_frames = FrameSet::new(10);
            let mut next_array = [next_frames.frame(0); 3];
            (*e).next.image.farray = next_array.as_mut_ptr();
            (*e).next.image.frame = next_frames.frame(7);
            (*e).current.location = POINT { x: -9, y: 13 };
            (*e).next.location = POINT { x: 42, y: -5 };
            SetVelocityComponents(&raw mut (*e).velocity, 1001, -4003);
            (*e).velocity.error.width = 19;
            let callback = (*e).preprocess_func.unwrap();
            with_rng(66, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(callback, e)
            });
            assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
            assert_eq!((*e).state_flags & NONSOLID as u16, 0);
            assert_ne!((*e).state_flags & APPEARING as u16, 0);
            assert_eq!((*e).state_flags & CHANGING as u16, 0);
            assert_eq!((*prim(e)).Type, gfx_object_STAMP_PRIM as u8);
            let i = (*e).IntersectControl;
            assert_eq!(
                (i.IntersectStamp.origin.x, i.IntersectStamp.origin.y),
                (-3, 3)
            );
            assert_eq!((i.EndPoint.x, i.EndPoint.y), (10, -2));
            assert_eq!(i.IntersectStamp.frame, next_frames.frame(7));
            assert_eq!((*e).__bindgen_anon_4.turn_wait, 5);
            assert_eq!((*e).current.image.frame, ship.napalm_frames.frame(9));
            assert_eq!((*e).next.image.frame, next_frames.frame(7));
        }
    })
}

#[test]
fn napalm_decay_wait_and_loop_boundary_match_c_including_last_frame_wrap() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            let e = ship.burn();
            let callback = (*e).preprocess_func.unwrap();
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(callback, e);
            (*e).state_flags &= !(APPEARING as u16);
            for wait in (0..5).rev() {
                SetVelocityComponents(&raw mut (*e).velocity, 77, 99);
                with_rng(3, 0, || {
                    uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                        callback, e,
                    )
                });
                assert_eq!((*e).__bindgen_anon_4.turn_wait, wait);
                assert_eq!((*e).next.image.frame, ship.napalm_frames.frame(9));
                assert_eq!((*e).state_flags & CHANGING as u16, 0);
                assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
            }
            for (frame, life, expected) in [
                (9, 48, 8),
                (7, 37, 6),
                (6, 37, 9),
                (6, 36, 5),
                (6, 0, 5),
                (0, 1, 9),
            ] {
                (*e).current.image.frame = ship.napalm_frames.frame(frame);
                (*e).next.image.frame = ship.napalm_frames.frame(2);
                (*e).__bindgen_anon_1.life_span = life;
                (*e).__bindgen_anon_4.turn_wait = 0;
                (*e).state_flags &= !(CHANGING as u16);
                with_rng(3, 0, || {
                    uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                        callback, e,
                    )
                });
                assert_eq!(frame_index((*e).next.image.frame), expected);
                assert_eq!((*e).__bindgen_anon_4.turn_wait, 5);
                assert_ne!((*e).state_flags & CHANGING as u16, 0);
                assert_eq!((*e).__bindgen_anon_1.life_span, life);
            }
        }
    })
}

#[test]
fn intelligence_classifies_stationary_threshold_and_turn_limit_without_rng() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            let enemy = Ship::new(1);
            ship.ss.special_counter = 1;
            for (x, y, turn, movement) in [
                (0, 0, 8, MOVEMENT_STATE_PURSUE),
                (512, 0, 8, MOVEMENT_STATE_PURSUE),
                (-512, 0, 8, MOVEMENT_STATE_PURSUE),
                (513, 0, 8, MOVEMENT_STATE_ENTICE),
                (512, 1, 8, MOVEMENT_STATE_ENTICE),
                (0, -513, 9, MOVEMENT_STATE_PURSUE),
                (i16::MAX, i16::MIN, 8, MOVEMENT_STATE_ENTICE),
            ] {
                SetVelocityComponents(&raw mut (*enemy.ptr()).velocity, x, y);
                let mut c = concerns();
                c[0].ObjectPtr = enemy.ptr();
                c[0].which_turn = turn;
                let input = with_rng(4, 0, || {
                    think(&mut ship, &mut c, (SPECIAL | WEAPON) as u8, 0)
                });
                assert_eq!(c[0].MoveState, movement, "velocity {x},{y}, turn {turn}");
                assert_eq!(
                    input, SPECIAL as u8,
                    "shared pass clears a ready weapon before special-counter exit"
                );
            }
        }
    })
}

#[test]
fn pursuit_boost_requires_three_energy_ready_turn_and_no_active_speed_limit() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            let enemy = Ship::new(1);
            for (energy, wait, input, current, boost) in [
                (2, 0, 0, 0, false),
                (3, 0, 0, 0, true),
                (3, 1, 0, 0, false),
                (3, 0, LEFT, 0, false),
                (3, 0, RIGHT, 0, false),
                (3, 0, 0, SPECIAL | SHIP_AT_MAX_SPEED, false),
                (3, 0, 0, SPECIAL | SHIP_BEYOND_MAX_SPEED, false),
                (3, 0, 0, SPECIAL, true),
                (3, 0, 0, SHIP_AT_MAX_SPEED, true),
            ] {
                ship.desc.ship_info.energy_level = energy;
                ship.ss.cur_status_flags = current as u16;
                (*ship.ptr()).__bindgen_anon_4.turn_wait = wait;
                let mut c = concerns();
                c[0].ObjectPtr = enemy.ptr();
                c[0].which_turn = 9;
                let result = with_rng(5, 0, || {
                    think(&mut ship, &mut c, (input | THRUST | SPECIAL) as u8, 0)
                });
                assert_eq!(result & SPECIAL as u8 != 0, boost);
                assert_eq!(result & THRUST as u8 != 0, !boost);
            }
            let mut c = concerns();
            assert_eq!(think(&mut ship, &mut c, SPECIAL as u8, 0), 0);
        }
    })
}

#[test]
fn enticing_weapon_takes_priority_and_uses_rounded_wrapped_travel_angle() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            let enemy = Ship::new(1);
            let weapon = Ship::new(1);
            (*ship.ptr()).__bindgen_anon_4.turn_wait = 1;
            for facing in 0..16 {
                ship.ss.ShipFacing = facing;
                for angle in 0..64 {
                    (*weapon.ptr()).velocity.travel_angle = angle;
                    let mut c = concerns();
                    c[0].ObjectPtr = enemy.ptr();
                    c[0].which_turn = 9;
                    c[ENEMY_WEAPON_INDEX as usize].ObjectPtr = weapon.ptr();
                    c[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_ENTICE;
                    let input = with_rng(73, 0, || think(&mut ship, &mut c, 0, 0));
                    let expected =
                        ((((i32::from(angle) + 32 + 8 + 2) >> 2) - i32::from(facing)) & 15) > 4;
                    assert_eq!(
                        input & SPECIAL as u8 != 0,
                        expected,
                        "facing {facing}, angle {angle}"
                    );
                }
            }
            (*weapon.ptr()).velocity.travel_angle = 32;
            ship.ss.ShipFacing = 0;
            for (energy, wait, input, expected) in [
                (0, 1, THRUST, THRUST | SPECIAL),
                (1, 1, THRUST, SPECIAL),
                (1, 0, 0, SPECIAL),
                (1, 0, LEFT, LEFT),
            ] {
                ship.desc.ship_info.energy_level = energy;
                (*ship.ptr()).__bindgen_anon_4.turn_wait = wait;
                let mut c = concerns();
                c[ENEMY_WEAPON_INDEX as usize].ObjectPtr = weapon.ptr();
                c[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_ENTICE;
                assert_eq!(think(&mut ship, &mut c, input as u8, 0), expected as u8);
            }
        }
    })
}

#[test]
fn entice_ship_geometry_uses_next_locations_and_size_truncation() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            let enemy = Ship::new(1);
            SetVelocityComponents(&raw mut (*enemy.ptr()).velocity, 513, 0);
            for (ship_y, enemy_y, boost) in [
                (4000, 4400, true),
                (4000, 3600, false),
                (i16::MAX - 1, i16::MIN + 1, true),
            ] {
                (*ship.ptr()).next.location = POINT { x: 4000, y: ship_y };
                (*enemy.ptr()).next.location = POINT {
                    x: 4000,
                    y: enemy_y,
                };
                let mut c = concerns();
                c[0].ObjectPtr = enemy.ptr();
                c[0].which_turn = 8;
                let input = with_rng(19, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(c[0].MoveState, MOVEMENT_STATE_ENTICE);
                assert_eq!(input & SPECIAL as u8 != 0, boost);
                assert_eq!(input & THRUST as u8 != 0, !boost);
            }
        }
    })
}

#[test]
fn shared_ai_steers_before_thraddash_decides_whether_to_boost() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship = Ship::new(0);
            let enemy = Ship::new(1);
            (*enemy.ptr()).current.location = POINT { x: 4400, y: 4000 };
            (*enemy.ptr()).next = (*enemy.ptr()).current;
            ship.ss.weapon_counter = 1;
            let mut c = concerns();
            c[0].ObjectPtr = enemy.ptr();
            c[0].which_turn = 9;
            let input = with_rng(8, 0, || think(&mut ship, &mut c, 0, 1));
            assert_ne!(input & RIGHT as u8, 0);
            assert_eq!(input & (LEFT | SPECIAL) as u8, 0);
            assert_eq!(frame_index((*ship.ptr()).next.image.frame), 1);
        }
    })
}

#[test]
fn owner_cleanup_during_collision_scan_invalidates_the_active_napalm() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(0);
            let mut enemy = Ship::new(1);
            let trail = ship.burn();
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*trail).preprocess_func.unwrap(),
                trail,
            );
            (*trail).state_flags &= !(APPEARING as u16);
            (*trail).state_flags |= PRE_PROCESS as u16;
            (*trail).__bindgen_anon_1.life_span = 20;
            // Avoid a spawn overlap, so the unfixed traversal reaches its callback.
            (*enemy.ptr()).state_flags |= (PRE_PROCESS | FINITE_LIFE) as u16;
            (*enemy.ptr()).collision_func = Some(uqm_rust::engine::battle::CollisionHook::Native(
                uqm_rust::battle::ship_native::collision,
            ));
            (*enemy.ptr()).current.location = (*trail).current.location;
            (*enemy.ptr()).next = (*enemy.ptr()).current;
            ship.napalm_frames.make_opaque();
            enemy._ship_frames.make_opaque();
            (*enemy.ptr()).IntersectControl = (*trail).IntersectControl;
            (*enemy.ptr()).IntersectControl.IntersectStamp.frame = enemy._ship_frames.frame(0);
            (*ship.ptr()).__bindgen_anon_1.life_span = 0;
            (*ship.ptr()).death_func = Some(uqm_rust::engine::battle::ElementHook::Native(
                uqm_rust::battle::tactrans_native::cleanup_dead_ship,
            ));
            _GraphicsStatusFlags |= 4;
            let result = uqm_rust::app::bridge::enter(|cx| {
                let next = cx.elements().id(ship.handle);
                let element = cx.elements().id(trail.cast()).unwrap();
                uqm_rust::battle::process_native::process_collisions(
                    cx,
                    next,
                    element,
                    MAX_TIME_VALUE as TIME_VALUE,
                    PRE_PROCESS as u16,
                )
            });
            assert_ne!(result & COLLISION as u16, 0);
            assert!((*trail).collision_func.is_none());
            assert_ne!((*trail).state_flags & (NONSOLID | DISAPPEARING) as u16, 0);
            assert_eq!(enemy.desc.ship_info.crew_level, 8);
        }
    })
}

#[test]
fn napalm_and_horn_collide_with_battle_objects_without_optional_callback_skips() {
    battle_context::run(|| {
        // The non-solid categories deliberately have no collision callback, just
        // like their constructors. Solid categories use the live collision code.
        for (flags, hp, mass, damage) in [
            (PLAYER_SHIP, 8, 7, true),
            (0, 1, 3, true),           // asteroid
            (0, 200, 200, false),      // planet
            (FINITE_LIFE, 3, 1, true), // missile
            (FINITE_LIFE, 1, 1, true), // laser
            (FINITE_LIFE | CREW_OBJECT, 1, 0, true),
            (FINITE_LIFE | NONSOLID, 1, 1, false), // blast/rubble/ion trail
            (FINITE_LIFE | DISAPPEARING, 1, 1, false),
        ] {
            for napalm in [false, true] {
                let _battle = Battle::new(16);
                unsafe {
                    let mut ship = Ship::new(0);
                    let mut target = Ship::new(1);
                    let weapon = if napalm {
                        let trail = ship.burn();
                        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                            (*trail).preprocess_func.unwrap(),
                            trail,
                        );
                        ship.napalm_frames.make_opaque();
                        trail
                    } else {
                        let horn = element(ship.fire());
                        (*horn).next = (*horn).current;
                        ship.horn_frames.make_opaque();
                        horn
                    };
                    (*weapon).state_flags &= !(APPEARING as u16);
                    (*weapon).state_flags |= PRE_PROCESS as u16;
                    (*weapon).IntersectControl.IntersectStamp.origin = POINT { x: 1000, y: 1000 };
                    (*weapon).IntersectControl.EndPoint = POINT { x: 1000, y: 1000 };
                    (*weapon).IntersectControl.IntersectStamp.frame = (*weapon).next.image.frame;
                    target._ship_frames.make_opaque();
                    let t = target.ptr();
                    (*t).state_flags = flags as u16 | PRE_PROCESS as u16;
                    (*t).__bindgen_anon_1.life_span = NORMAL_LIFE as u16;
                    (*t).__bindgen_anon_2.hit_points = hp;
                    (*t).__bindgen_anon_3.mass_points = mass;
                    (*t).collision_func = if flags & (NONSOLID | DISAPPEARING) == 0 {
                        Some(uqm_rust::engine::battle::CollisionHook::Native(
                            uqm_rust::battle::ship_native::collision,
                        ))
                    } else {
                        None
                    };
                    (*t).IntersectControl = (*weapon).IntersectControl;
                    (*t).IntersectControl.IntersectStamp.frame = target._ship_frames.frame(0);
                    let mut context: uqm_rust::graphics::native_abi::context_desc = mem::zeroed();
                    let old_context = uqm_rust::graphics::context_abi::SetContext(&mut context);
                    uqm_rust::app::bridge::enter(|cx| {
                        let next = cx.elements().id(target.handle);
                        let element = cx.elements().id(weapon.cast()).unwrap();
                        uqm_rust::battle::process_native::process_collisions(
                            cx,
                            next,
                            element,
                            MAX_TIME_VALUE as TIME_VALUE,
                            PRE_PROCESS as u16,
                        )
                    });
                    uqm_rust::graphics::context_abi::SetContext(old_context);
                    let expected = if damage {
                        hp - if napalm { 2.min(hp) } else { 1 }
                    } else {
                        hp
                    };
                    assert_eq!(
                        (*t).__bindgen_anon_2.hit_points,
                        expected,
                        "flags={flags:x} napalm={napalm}"
                    );
                    assert!((*weapon).collision_func.is_some());
                }
            }
        }
    })
}
