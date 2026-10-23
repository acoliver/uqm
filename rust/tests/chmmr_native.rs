//! Linked callback checks against the Chmmr C rules, without a running renderer.

use std::{
    ffi::{c_void, CStr},
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::{
    abi::*, cosine, element, facing_to_angle, frame_index, put_element, sine,
};

unsafe extern "C" {
    fn init_chmmr() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
    fn RemoveElement(handle: HLINK);
    static mut DisplayArray: [PRIMITIVE; 330];
}

static ENGINE: Mutex<()> = Mutex::new(());

struct Battlefield {
    _engine: MutexGuard<'static, ()>,
    old_screen: (i32, i32),
    old_display: *mut PRIMITIVE,
}

impl Battlefield {
    fn new(capacity: COUNT) -> Self {
        let engine = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            let old_screen = (ScreenWidth, ScreenHeight);
            let old_display = GlobData.Game_state.DisplayArray;
            assert_ne!(
                InitQueue(&raw mut disp_q, capacity, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
                _engine: engine,
                old_screen,
                old_display,
            }
        }
    }
}

impl Drop for Battlefield {
    fn drop(&mut self) {
        unsafe {
            GlobData.Game_state.DisplayArray = self.old_display;
            (ScreenWidth, ScreenHeight) = self.old_screen;
            UninitQueue(&raw mut disp_q);
        }
    }
}

// Layouts from libs/graphics/drawable.h. These are real frame arrays and parent
// descriptors, so SetAbsFrameIndex and GetFrameRect exercise the engine ABI.
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
                bounds: [16, 12],
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
        let mut desc = Box::new(*init_chmmr());
        let mut ship_frames = FrameSet::new(16);
        let mut weapon_frames = FrameSet::new(41);
        let mut special_frames = FrameSet::new(12);
        desc.ship_data.ship = [ship_frames.frame(0); 3];
        desc.ship_data.weapon = [weapon_frames.frame(0); 3];
        desc.ship_data.special = [special_frames.frame(0); 3];
        let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
        ss.RaceDescPtr = &raw mut *desc;
        ss.playerNr = player;
        let handle = AllocElement();
        assert!(!handle.is_null());
        ss.hShip = handle;
        let el = element(handle);
        (*el).pParent = (&raw mut *ss).cast();
        (*el).playerNr = player;
        (*el).state_flags = PLAYER_SHIP as u16;
        (*el).__bindgen_anon_1.life_span = NORMAL_LIFE as u16;
        (*el).__bindgen_anon_2.crew_level = MAX_CREW_SIZE as u16;
        (*el).__bindgen_anon_3.mass_points = 10;
        (*el).current.location = POINT { x: 4000, y: 4000 };
        (*el).current.image.frame = ship_frames.frame(0);
        (*el).current.image.farray = desc.ship_data.ship.as_mut_ptr();
        (*el).next = (*el).current;
        (*prim(el)).Type = gfx_object_STAMP_PRIM as u8;
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

    unsafe fn laser(&mut self) -> HELEMENT {
        let mut h = ptr::null_mut();
        assert_eq!((self.desc.init_weapon_func.unwrap())(self.ptr(), &mut h), 1);
        if !h.is_null() {
            // ship.c attaches the owner after the race's weapon callback.
            (*element(h)).pParent = (&raw mut *self.ss).cast();
        }
        h
    }

    unsafe fn postprocess(&self) {
        (self.desc.postprocess_func.unwrap())(self.ptr());
    }

    unsafe fn delayed_spawn(&self) -> HELEMENT {
        (self.desc.preprocess_func.unwrap())(self.ptr());
        disp_q.tail
    }

    unsafe fn satellites(&self) -> Vec<HELEMENT> {
        let delay = self.delayed_spawn();
        let death = (*element(delay)).death_func.unwrap();
        RemoveElement(delay);
        FreeElement(delay);
        let tail = disp_q.tail;
        death(self.ptr());
        let mut h = (*element(tail)).succ;
        let mut satellites = Vec::new();
        while !h.is_null() {
            satellites.push(h);
            h = (*element(h)).succ;
        }
        satellites
    }
}

unsafe fn prim(el: *mut ELEMENT) -> *mut PRIMITIVE {
    GlobData
        .Game_state
        .DisplayArray
        .add((*el).PrimIndex as usize)
}

fn rgba(c: Color) -> [u8; 4] {
    [c.r, c.g, c.b, c.a]
}
fn point(p: POINT) -> (i16, i16) {
    (p.x, p.y)
}

unsafe fn velocity(el: *mut ELEMENT) -> (SIZE, SIZE) {
    let (mut x, mut y) = (0, 0);
    GetCurrentVelocityComponents(&mut (*el).velocity, &mut x, &mut y);
    (x, y)
}

fn velocity_fields(v: &VELOCITY_DESC) -> [i16; 9] {
    [
        v.TravelAngle as i16,
        v.vector.width,
        v.vector.height,
        v.fract.width,
        v.fract.height,
        v.error.width,
        v.error.height,
        v.incr.width,
        v.incr.height,
    ]
}

unsafe fn assert_rng_after(seed: u32, draws: usize) {
    let actual = TFB_Random();
    TFB_SeedRandom(seed);
    for _ in 0..draws {
        TFB_Random();
    }
    assert_eq!(actual, TFB_Random(), "RNG draws for seed {seed}");
}

unsafe fn exhaust() -> Vec<HELEMENT> {
    let mut handles = Vec::new();
    loop {
        let h = AllocElement();
        if h.is_null() {
            return handles;
        }
        handles.push(h);
    }
}

#[test]
fn descriptor_matches_the_entire_c_template_and_init_reinstalls_callbacks() {
    let _battle = Battlefield::new(8);
    unsafe {
        let template = init_chmmr();
        assert_eq!(template, init_chmmr());
        let d = *template;
        assert_eq!(
            d.ship_info.ship_flags,
            (FIRES_FORE | IMMEDIATE_WEAPON | SEEKING_SPECIAL | POINT_DEFENSE) as u16
        );
        assert_eq!(d.ship_info.ship_cost, 30);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (42, 42));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (42, 42));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.chmmr.text"
        );
        assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.chmmr.icons");
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.chmmr.meleeicons"
        );
        assert!(d.ship_info.race_strings.is_null());
        assert!(d.ship_info.icons.is_null());
        assert!(d.ship_info.melee_icon.is_null());
        assert_eq!((d.fleet.strength, point(d.fleet.known_loc)), (0, (0, 0)));
        let c = d.characteristics;
        assert_eq!(
            (c.max_thrust, c.thrust_increment, c.energy_regeneration),
            (35, 7, 1)
        );
        assert_eq!(
            (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
            (2, 1, 1)
        );
        assert_eq!(
            (
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (3, 5, 0, 0, 10)
        );
        for (i, size) in ["large", "medium", "small"].iter().enumerate() {
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.chmmr.graphics.avatar.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                format!("ship.chmmr.graphics.muzzle.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                format!("ship.chmmr.graphics.satellite.{size}")
            );
            assert!(d.ship_data.ship[i].is_null());
            assert!(d.ship_data.weapon[i].is_null());
            assert!(d.ship_data.special[i].is_null());
        }
        let captain = d.ship_data.captain_control;
        assert_eq!(
            CStr::from_ptr(captain.captain_rsc),
            c"ship.chmmr.graphics.captain"
        );
        assert!([
            captain.background,
            captain.turn,
            captain.thrust,
            captain.weapon,
            captain.special
        ]
        .iter()
        .all(|p| p.is_null()));
        assert_eq!(
            CStr::from_ptr(d.ship_data.victory_ditty_rsc),
            c"ship.chmmr.ditty"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_sounds_rsc),
            c"ship.chmmr.sounds"
        );
        assert!(d.ship_data.victory_ditty.is_null());
        assert!(d.ship_data.ship_sounds.is_null());
        assert_eq!(
            (
                d.cyborg_control.ManeuverabilityIndex,
                d.cyborg_control.WeaponRange
            ),
            (0, 200)
        );
        assert!(d.uninit_func.is_none());
        assert!(d.data.is_null());
        assert!(d.CodeRef.is_null());
        assert!(d.preprocess_func.is_some());
        assert!(d.postprocess_func.is_some());
        assert!(d.init_weapon_func.is_some());
        assert!(d.cyborg_control.intelligence_func.is_some());
        // init_chmmr restores callbacks even after preprocess disables itself.
        (*template).preprocess_func = None;
        (*template).postprocess_func = None;
        (*template).init_weapon_func = None;
        (*template).cyborg_control.intelligence_func = None;
        init_chmmr();
        assert!((*template).preprocess_func.is_some());
        assert!((*template).postprocess_func.is_some());
        assert!((*template).init_weapon_func.is_some());
        assert!((*template).cyborg_control.intelligence_func.is_some());
    }
}

#[test]
fn laser_uses_frame_hotspot_alignment_facing_rgba_and_cycle() {
    let _battle = Battlefield::new(16);
    unsafe {
        let mut ship = Ship::new(0);
        let colors = [
            [189, 0, 0, 255],
            [255, 24, 0, 255],
            [255, 140, 0, 255],
            [255, 24, 0, 255],
        ];
        for face in 0..16u16 {
            ship.ss.ShipFacing = face;
            ship.weapon_frames.frames[face as usize].hot_spot = [3, -5];
            (*ship.ptr()).next.location = POINT { x: -101, y: 32767 };
            for counter in 0..4u8 {
                ship.ss.special_counter = counter;
                TFB_SeedRandom(55);
                let h = ship.laser();
                assert!(!h.is_null());
                let laser = element(h);
                assert_eq!(point((*laser).current.location), (-116, -32752));
                assert_eq!((*laser).__bindgen_anon_2.hit_points, 1);
                assert_eq!((*laser).__bindgen_anon_3.mass_points, 2);
                assert_eq!((*laser).__bindgen_anon_1.life_span, 1);
                assert_eq!((*laser).__bindgen_anon_4.turn_wait, (counter + 1) % 4);
                assert!((*laser).death_func.is_some());
                assert_eq!((*prim(laser)).Type, gfx_object_LINE_PRIM as u8);
                assert_eq!(rgba((*prim(laser)).color), colors[counter as usize]);
                let a = facing_to_angle(i32::from(face));
                assert_eq!(
                    velocity(laser),
                    (
                        (i32::from(cosine(a, 600)) << VELOCITY_SHIFT) as SIZE,
                        (i32::from(sine(a, 600)) << VELOCITY_SHIFT) as SIZE
                    )
                );
                assert_rng_after(55, 0);
                FreeElement(h);
            }
        }
    }
}

#[test]
fn weapon_allocation_failure_still_returns_one_null_slot() {
    let _battle = Battlefield::new(2);
    unsafe {
        let mut ship = Ship::new(0);
        let held = exhaust();
        TFB_SeedRandom(9);
        assert!(ship.laser().is_null());
        assert_rng_after(9, 0);
        for h in held {
            FreeElement(h);
        }
    }
}

#[test]
fn laser_death_obeys_range_byte_gate_count_draw_and_animation() {
    let _battle = Battlefield::new(16);
    unsafe {
        let mut ship = Ship::new(0);
        let h = ship.laser();
        let laser = element(h);
        let death = (*laser).death_func.unwrap();
        let origin = (*ship.ptr()).current.location;
        let mut gates = [false; 2];
        for seed in 1..64 {
            for distance in [111i16, 112, 600] {
                TFB_SeedRandom(seed);
                let gate = TFB_Random() as u8 & 7 != 0;
                let draw = TFB_Random() as u16;
                gates[gate as usize] = true;
                (*laser).current.location = POINT {
                    x: origin.x + distance,
                    y: origin.y,
                };
                ship.ss.special_counter = 0;
                let tail = disp_q.tail;
                TFB_SeedRandom(seed);
                death(laser);
                assert_eq!(ship.ss.special_counter, 1);
                if gate && distance >= 112 {
                    let spot_handle = disp_q.tail;
                    assert_ne!(spot_handle, tail);
                    let spot = element(spot_handle);
                    let magnitude = draw % ((distance as u16 + 1) - 112) + 112;
                    assert_eq!(
                        point((*spot).current.location),
                        (origin.x + magnitude as i16, origin.y)
                    );
                    assert_eq!(frame_index((*spot).current.image.frame), 32);
                    assert_eq!(
                        (*spot).current.image.farray,
                        ship.desc.ship_data.weapon.as_mut_ptr()
                    );
                    assert_eq!((*spot).__bindgen_anon_1.life_span, 9);
                    assert_eq!((*spot).__bindgen_anon_5.next_turn, 0);
                    assert_eq!((*prim(spot)).Type, gfx_object_STAMP_PRIM as u8);
                    assert_eq!((*spot).pParent, (&raw mut *ship.ss).cast());
                    let animate = (*spot).preprocess_func.unwrap();
                    (*spot).__bindgen_anon_4.turn_wait = 1;
                    (*spot).__bindgen_anon_5.next_turn = 3;
                    animate(spot);
                    assert_eq!((*spot).__bindgen_anon_4.turn_wait, 0);
                    assert!((*spot).next.image.frame.is_null());
                    animate(spot);
                    assert_eq!(frame_index((*spot).next.image.frame), 33);
                    assert_eq!((*spot).__bindgen_anon_4.turn_wait, 3);
                    assert_ne!((*spot).state_flags & CHANGING as u16, 0);
                    assert_rng_after(seed, 2);
                    RemoveElement(spot_handle);
                    FreeElement(spot_handle);
                } else {
                    assert_eq!({ disp_q.tail }, tail);
                    assert_rng_after(seed, 1);
                }
            }
        }
        assert_eq!(gates, [true, true]);
        ship.ss.hShip = ptr::null_mut();
        TFB_SeedRandom(123);
        death(laser);
        assert_rng_after(123, 0);
        ship.ss.hShip = ship.handle;
        (*laser).current.location.x = origin.x + 600;
        let held = exhaust();
        TFB_SeedRandom(7);
        death(laser);
        assert_rng_after(7, 1);
        for h in held {
            FreeElement(h);
        }
        FreeElement(h);
    }
}

#[test]
fn muzzle_flash_requires_friendly_appearing_tail_line_and_even_cycle() {
    let _battle = Battlefield::new(16);
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.ShipFacing = 5;
        ship.ss.cur_status_flags = WEAPON as u16;
        ship.postprocess(); // tail is the ship itself
        assert_eq!({ disp_q.tail }, ship.handle);
        for (counter, player, appearing, kind, expected) in [
            (0, 0, true, gfx_object_LINE_PRIM, true),
            (2, 0, true, gfx_object_LINE_PRIM, true),
            (1, 0, true, gfx_object_LINE_PRIM, false),
            (3, 0, true, gfx_object_LINE_PRIM, false),
            (0, 1, true, gfx_object_LINE_PRIM, false),
            (0, 0, false, gfx_object_LINE_PRIM, false),
            (0, 0, true, gfx_object_STAMP_PRIM, false),
        ] {
            ship.ss.special_counter = counter;
            let h = ship.laser();
            let laser = element(h);
            (*laser).playerNr = player;
            if !appearing {
                (*laser).state_flags &= !(APPEARING as u16);
            }
            (*prim(laser)).Type = kind as u8;
            put_element(h);
            ship.postprocess();
            assert_eq!(ship.ss.special_counter, 0);
            if expected {
                let flash_handle = disp_q.tail;
                assert_ne!(flash_handle, h);
                let flash = element(flash_handle);
                assert_eq!(
                    point((*flash).current.location),
                    point((*ship.ptr()).next.location)
                );
                assert_eq!(frame_index((*flash).current.image.frame), 21);
                assert_eq!((*flash).__bindgen_anon_1.life_span, 1);
                assert_eq!((*prim(flash)).Type, gfx_object_STAMP_PRIM as u8);
                assert_eq!((*flash).pParent, (&raw mut *ship.ss).cast());
                RemoveElement(flash_handle);
                FreeElement(flash_handle);
            } else {
                assert_eq!({ disp_q.tail }, h);
            }
            RemoveElement(h);
            FreeElement(h);
        }
    }
}

#[test]
fn delayed_satellites_spawn_around_live_ship_with_three_ordered_rng_draws() {
    let _battle = Battlefield::new(16);
    unsafe {
        let ship = Ship::new(1);
        let delay = ship.delayed_spawn();
        let transit = element(delay);
        assert!(ship.desc.preprocess_func.is_none());
        assert_eq!((*transit).__bindgen_anon_1.life_span, 16);
        assert_eq!((*prim(transit)).Type, gfx_object_NUM_PRIMS as u8);
        assert_eq!(
            (*transit).state_flags,
            (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16
        );
        // The delayed element's position and player are not the spawn source.
        (*transit).playerNr = 0;
        (*transit).next.location = POINT { x: -1000, y: -2000 };
        (*ship.ptr()).next.location = POINT { x: 7000, y: 6000 };
        TFB_SeedRandom(77);
        let indices = [
            TFB_Random() as u16 & 7,
            TFB_Random() as u16 & 7,
            TFB_Random() as u16 & 7,
        ];
        TFB_SeedRandom(77);
        ((*transit).death_func.unwrap())(transit);
        let mut h = (*transit).succ;
        for (i, index) in indices.into_iter().enumerate() {
            assert!(!h.is_null());
            let s = element(h);
            let angle = (i as i32 * 64 + 1) / 3;
            assert_eq!((*s).playerNr, 1);
            assert_eq!((*s).__bindgen_anon_4.turn_wait, angle as u8);
            assert_eq!((*s).__bindgen_anon_1.life_span, 2);
            assert_eq!((*s).__bindgen_anon_2.hit_points, 10);
            assert_eq!((*s).__bindgen_anon_3.mass_points, 10);
            assert_eq!(
                point((*s).current.location),
                (7000 + cosine(angle, 256), 6000 + sine(angle, 256))
            );
            assert_eq!(frame_index((*s).current.image.frame), index);
            assert_eq!((*prim(s)).Type, gfx_object_STAMP_PRIM as u8);
            assert!((*s).preprocess_func.is_some());
            assert!((*s).postprocess_func.is_some());
            assert!((*s).death_func.is_some());
            assert!((*s).collision_func.is_some());
            h = (*s).succ;
        }
        assert!(h.is_null());
        assert_rng_after(77, 3);
    }
}

#[test]
fn spawn_and_preprocess_allocation_failures_do_not_draw_rng_or_rearm() {
    let _battle = Battlefield::new(3);
    unsafe {
        let mut ship = Ship::new(0);
        let delay = ship.delayed_spawn();
        let death = (*element(delay)).death_func.unwrap();
        TFB_SeedRandom(88);
        death(element(delay)); // only one free slot
        assert_eq!((*element(disp_q.tail)).__bindgen_anon_2.hit_points, 10);
        assert_rng_after(88, 1);
        TFB_SeedRandom(88);
        death(element(delay));
        assert_rng_after(88, 0);
        ship.ss.hShip = ptr::null_mut();
        TFB_SeedRandom(88);
        death(element(delay));
        assert_rng_after(88, 0);
        ship.ss.hShip = ship.handle;
        ship.desc.preprocess_func = (*init_chmmr()).preprocess_func;
        let tail = disp_q.tail;
        (ship.desc.preprocess_func.unwrap())(ship.ptr());
        assert!(ship.desc.preprocess_func.is_none());
        assert_eq!({ disp_q.tail }, tail);
    }
}

#[test]
fn satellites_wrap_space_cap_speed_and_preserve_half_space_ties() {
    let _battle = Battlefield::new(16);
    unsafe {
        let mut ship = Ship::new(0);
        let satellites = ship.satellites();
        let s = element(satellites[0]);
        let preprocess = (*s).preprocess_func.unwrap();
        for (dx, dy, wx, wy) in [
            (80, 0, 80, 0),
            (81, 0, 81, 0),
            (48, 64, 48, 64),
            (48, 65, 48, 65),
            (8190, 7677, -2, -3),
            (-8190, -7677, 2, 3),
            (4096, 3840, 4096, 3840),
            (-4096, -3840, -4096, -3840),
            (4097, 3841, -4095, -3839),
            (-4097, -3841, 4095, 3839),
        ] {
            (*s).__bindgen_anon_4.turn_wait = 63;
            (*s).__bindgen_anon_1.life_span = u16::MAX;
            (*s).current.image.frame = ship.special_frames.frame(7);
            let desired_x = i32::from((*ship.ptr()).next.location.x) + i32::from(cosine(0, 256));
            let desired_y = i32::from((*ship.ptr()).next.location.y) + i32::from(sine(0, 256));
            (*s).current.location = POINT {
                x: (desired_x - dx) as i16,
                y: (desired_y - dy) as i16,
            };
            ship.desc.ship_info.ship_flags &= !(POINT_DEFENSE as u16);
            preprocess(s);
            assert_eq!((*s).__bindgen_anon_1.life_span, 0);
            assert_eq!((*s).__bindgen_anon_4.turn_wait, 0);
            assert_eq!(frame_index((*s).next.image.frame), 0);
            assert_ne!(ship.desc.ship_info.ship_flags & POINT_DEFENSE as u16, 0);
            let expected = if i64::from(wx) * i64::from(wx) + i64::from(wy) * i64::from(wy) <= 6400
            {
                (
                    (wx << VELOCITY_SHIFT) as SIZE,
                    (wy << VELOCITY_SHIFT) as SIZE,
                )
            } else {
                let a = i32::from(ARCTAN(wx as SIZE, wy as SIZE));
                (cosine(a, 2560), sine(a, 2560))
            };
            assert_eq!(velocity(s), expected, "delta {dx}/{dy}");
        }
        ship.ss.hShip = ptr::null_mut();
        let before = velocity_fields(&(*s).velocity);
        preprocess(s);
        assert_eq!(velocity_fields(&(*s).velocity), before);
    }
}

#[test]
fn satellite_collision_is_noop_and_death_installs_four_tick_animation() {
    let _battle = Battlefield::new(16);
    unsafe {
        let mut ship = Ship::new(0);
        let satellites = ship.satellites();
        let s = element(satellites[0]);
        let mut other: ELEMENT = mem::zeroed();
        let mut p = POINT { x: 1, y: 2 };
        ((*s).collision_func.unwrap())(s, &mut p, &mut other, ptr::null_mut());
        assert_eq!((*s).__bindgen_anon_2.hit_points, 10);
        assert_eq!(point(p), (1, 2));
        (*s).state_flags |= DISAPPEARING as u16;
        ((*s).death_func.unwrap())(s);
        assert_eq!(ship.desc.ship_info.ship_flags & POINT_DEFENSE as u16, 0);
        assert_eq!((*s).state_flags & DISAPPEARING as u16, 0);
        assert_eq!(
            (*s).state_flags & (NONSOLID | FINITE_LIFE | CHANGING) as u16,
            (NONSOLID | FINITE_LIFE | CHANGING) as u16
        );
        assert_eq!((*s).__bindgen_anon_1.life_span, 4);
        assert_eq!((*s).__bindgen_anon_4.turn_wait, 1);
        assert_eq!((*s).__bindgen_anon_5.next_turn, 0);
        assert_eq!(frame_index((*s).next.image.frame), 8);
        assert!((*s).postprocess_func.is_none());
        assert!((*s).collision_func.is_none());
        assert!((*s).death_func.is_none());
        (*s).current = (*s).next;
        let animate = (*s).preprocess_func.unwrap();
        animate(s);
        assert_eq!(frame_index((*s).next.image.frame), 8);
        animate(s);
        assert_eq!(frame_index((*s).next.image.frame), 9);
        // A surviving satellite restores point defense on its next preprocess.
        let survivor = element(satellites[1]);
        ((*survivor).preprocess_func.unwrap())(survivor);
        assert_ne!(ship.desc.ship_info.ship_flags & POINT_DEFENSE as u16, 0);
        // Reinitializing the template does not mutate this ship's descriptor.
        init_chmmr();
        ship.desc.ship_info.ship_flags &= !(POINT_DEFENSE as u16);
        assert_ne!(
            (*init_chmmr()).ship_info.ship_flags & POINT_DEFENSE as u16,
            0
        );
    }
}

unsafe fn target(satellite: *mut ELEMENT, dx: i16, dy: i16, hp: u16) -> HELEMENT {
    let h = AllocElement();
    assert!(!h.is_null());
    let e = element(h);
    (*e).playerNr = 1;
    (*e).__bindgen_anon_2.hit_points = hp;
    (*e).__bindgen_anon_3.mass_points = 1;
    (*e).next.location = POINT {
        x: (*satellite).next.location.x.wrapping_add(dx),
        y: (*satellite).next.location.y.wrapping_add(dy),
    };
    (*prim(e)).Type = gfx_object_STAMP_PRIM as u8;
    put_element(h);
    h
}

unsafe fn defense(satellite: *mut ELEMENT) -> HELEMENT {
    ((*satellite).postprocess_func.unwrap())(satellite);
    let h = disp_q.tail;
    assert_eq!((*element(h)).hTarget, satellite.cast());
    assert_eq!((*element(h)).__bindgen_anon_1.life_span, 0);
    h
}

#[test]
fn point_defense_uses_backward_weakest_nearest_and_stable_tie_order() {
    let _battle = Battlefield::new(32);
    unsafe {
        let ship = Ship::new(0);
        let satellites = ship.satellites();
        let s = element(satellites[0]);
        (*s).next = (*s).current;
        let strong = target(s, 4, 0, 3);
        let far_weak = target(s, 200, 0, 1);
        let near_weak = target(s, 40, 0, 1);
        let tie = target(s, -40, 0, 1);
        let d = defense(s);
        // An even weaker candidate appended after the defense is not traversed.
        let after = target(s, 0, 4, 0);
        let before = disp_q.tail;
        TFB_SeedRandom(12);
        ((*element(d)).death_func.unwrap())(element(d));
        assert_ne!({ disp_q.tail }, before);
        let beam = element(disp_q.tail);
        assert_eq!(point((*beam).current.location), point((*s).next.location));
        assert_eq!(velocity(beam), (-40 << VELOCITY_SHIFT, 0));
        assert_eq!(rgba((*prim(beam)).color), [0, 8, 255, 255]);
        assert_eq!((*beam).pParent, (*s).pParent);
        assert!((*beam).hTarget.is_null());
        assert_eq!((*s).__bindgen_anon_5.thrust_wait, 2);
        assert_rng_after(12, 0);
        for h in [strong, far_weak, near_weak, tie, after] {
            assert!(!h.is_null());
        }
    }
}

#[test]
fn point_defense_respects_collision_cloak_range_and_full_width_hitpoints() {
    // (world dx, dy, hit points, excluded condition, should shoot)
    for (dx, dy, hp, exclusion, shoot) in [
        (256, 0, 1, 0, true),
        (259, 0, 1, 0, true),
        (260, 0, 1, 0, false),
        (180, 180, 1, 0, true),
        (184, 184, 1, 0, false),
        (40, 0, 256, 0, false),
        (32, 0, 255, 0, true),
        (36, 0, 255, 0, false),
        (40, 0, 1, 1, false),
        (40, 0, 1, 2, false),
        (40, 0, 1, 3, false),
        (40, 0, 1, 4, false),
        (40, 0, 1, 5, false),
        (40, 0, 1, 6, false),
        (40, 0, 1, 7, false),
        (40, 0, 1, 8, false),
        (40, 0, 1, 9, false),
        (40, 0, 1, 10, true),
    ] {
        let _battle = Battlefield::new(16);
        unsafe {
            let ship = Ship::new(0);
            let satellites = ship.satellites();
            let s = element(satellites[0]);
            (*s).next = (*s).current;
            let h = target(s, dx, dy, hp);
            let obj = element(h);
            match exclusion {
                1 => (*obj).playerNr = 0,
                2 => (*obj).playerNr = -1,
                3 => (*obj).state_flags = NONSOLID as u16,
                4 => (*obj).state_flags = DISAPPEARING as u16,
                5 => (*prim(obj)).Type = gfx_object_NUM_PRIMS as u8,
                6 => {
                    (*prim(obj)).Type = gfx_object_STAMPFILL_PRIM as u8;
                    (*prim(obj)).color = Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    };
                }
                7 => {
                    (*obj).state_flags = COLLISION as u16;
                    (*ship.ptr()).state_flags |= COLLISION as u16;
                }
                8 => {
                    (*obj).state_flags = IGNORE_SIMILAR as u16;
                    (*ship.ptr()).state_flags |= IGNORE_SIMILAR as u16;
                    (*obj).pParent = (*ship.ptr()).pParent;
                }
                9 => {
                    (*obj).__bindgen_anon_3.mass_points = 0;
                    (*ship.ptr()).__bindgen_anon_3.mass_points = 0;
                }
                10 => (*ship.ptr()).state_flags |= NONSOLID as u16,
                _ => {}
            }
            let d = defense(s);
            ((*element(d)).death_func.unwrap())(element(d));
            assert_eq!(
                disp_q.tail != d,
                shoot,
                "dx {dx} dy {dy} hp {hp} exclusion {exclusion}"
            );
            assert_eq!((*s).__bindgen_anon_5.thrust_wait, if shoot { 2 } else { 0 });
        }
    }
}

#[test]
fn satellite_defense_wait_underflows_on_dead_zero_wait_and_handles_exhaustion() {
    let _battle = Battlefield::new(8);
    unsafe {
        let ship = Ship::new(0);
        let satellites = ship.satellites();
        let s = element(satellites[0]);
        let post = (*s).postprocess_func.unwrap();
        (*s).__bindgen_anon_5.thrust_wait = 2;
        let tail = disp_q.tail;
        post(s);
        assert_eq!((*s).__bindgen_anon_5.thrust_wait, 1);
        post(s);
        assert_eq!((*s).__bindgen_anon_5.thrust_wait, 0);
        assert_eq!({ disp_q.tail }, tail);
        (*s).__bindgen_anon_1.life_span = 0;
        post(s);
        assert_eq!((*s).__bindgen_anon_5.thrust_wait, 255);
        (*s).__bindgen_anon_1.life_span = 2;
        (*s).__bindgen_anon_5.thrust_wait = 0;
        let held = exhaust();
        post(s);
        assert_eq!({ disp_q.tail }, tail);
        assert_eq!((*s).__bindgen_anon_5.thrust_wait, 0);
        for h in held {
            FreeElement(h);
        }
        (*s).next = (*s).current;
        target(s, 40, 0, 1);
        let d = defense(s);
        let held = exhaust();
        ((*element(d)).death_func.unwrap())(element(d));
        assert_eq!({ disp_q.tail }, d);
        assert_eq!((*s).__bindgen_anon_5.thrust_wait, 0);
        for h in held {
            FreeElement(h);
        }
    }
}

#[test]
fn tractor_applies_mass_division_speed_flags_and_head_inserted_blue_shadows() {
    for (mass, initial_vx, expected_flags) in [
        (10u8, 0i16, 0u32),
        (10, 1082, SHIP_AT_MAX_SPEED),
        (10, 1083, SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED),
        (99, 0, 0),
    ] {
        let _battle = Battlefield::new(32);
        unsafe {
            let ship = Ship::new(0);
            let mut enemy = Ship::new(1);
            (*enemy.ptr()).__bindgen_anon_3.mass_points = mass;
            (*enemy.ptr()).current.location = POINT { x: 3000, y: 3728 };
            (*enemy.ptr()).next = (*enemy.ptr()).current;
            (*ship.ptr()).hTarget = enemy.handle;
            SetVelocityComponents(&mut (*enemy.ptr()).velocity, initial_vx, 0);
            enemy.ss.cur_status_flags = (LEFT | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            let mut ship = ship;
            ship.ss.cur_status_flags = SPECIAL as u16;
            ship.ss.special_counter = 3;
            TFB_SeedRandom(99);
            ship.postprocess();
            let vx = initial_vx + (384 / i32::from(mass)) as i16;
            assert_eq!(velocity(enemy.ptr()), (vx, 0));
            assert_eq!(enemy.ss.cur_status_flags, (LEFT | expected_flags) as u16);
            assert_eq!(ship.desc.ship_info.energy_level, 41);
            assert_eq!(ship.ss.energy_counter, 1);
            assert_eq!(ship.ss.special_counter, 0);
            let mut h = disp_q.head;
            for (offset, blue) in [(60, 57), (42, 74), (28, 99), (17, 115), (8, 132)] {
                let shadow = element(h);
                assert_ne!(h, ship.handle);
                assert_eq!((*shadow).playerNr, 1);
                assert_eq!((*shadow).pParent, (&raw mut *enemy.ss).cast());
                assert_eq!(
                    (*shadow).state_flags,
                    (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | POST_PROCESS) as u16
                );
                assert_eq!((*shadow).__bindgen_anon_1.life_span, 1);
                assert_eq!(point((*shadow).current.location), (3000 + offset * 4, 3728));
                assert_eq!(
                    point((*shadow).next.location),
                    point((*shadow).current.location)
                );
                assert_eq!(
                    (*shadow).current.image.frame,
                    (*enemy.ptr()).next.image.frame
                );
                assert_eq!(velocity(shadow), (vx, 0));
                assert_eq!((*prim(shadow)).Type, gfx_object_STAMPFILL_PRIM as u8);
                assert_eq!(rgba((*prim(shadow)).color), [0, 0, blue, 255]);
                h = (*shadow).succ;
            }
            assert_eq!(h, ship.handle);
            assert_eq!({ disp_q.tail }, enemy.handle);
            assert_rng_after(99, 0);
        }
    }
}

#[test]
fn tractor_energy_target_gravity_and_allocation_boundaries() {
    for (energy, mass, has_target, exhaust_queue) in [
        (0u8, 10u8, true, false),
        (1, 100, true, false),
        (1, 255, true, false),
        (1, 10, false, false),
        (1, 10, true, true),
    ] {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship = Ship::new(0);
            let enemy = Ship::new(1);
            (*enemy.ptr()).__bindgen_anon_3.mass_points = mass;
            (*enemy.ptr()).current.location = POINT { x: 3000, y: 3728 };
            (*enemy.ptr()).next = (*enemy.ptr()).current;
            if has_target {
                (*ship.ptr()).hTarget = enemy.handle;
            } else {
                // TrackShip must see no target, including through its queue scan.
                (*enemy.ptr()).state_flags = 0;
            }
            ship.desc.ship_info.energy_level = energy;
            ship.ss.cur_status_flags = SPECIAL as u16;
            ship.ss.special_counter = 3;
            let held = if exhaust_queue { exhaust() } else { Vec::new() };
            let tail = disp_q.tail;
            TFB_SeedRandom(22);
            ship.postprocess();
            assert_eq!(ship.ss.special_counter, 0);
            assert_eq!(ship.desc.ship_info.energy_level, 0);
            assert_eq!({ disp_q.tail }, tail);
            assert_eq!({ disp_q.head }, ship.handle);
            let affected = energy != 0 && mass < 100 && has_target;
            assert_eq!(
                velocity(enemy.ptr()),
                if affected { (38, 0) } else { (0, 0) }
            );
            assert_rng_after(22, 0);
            for h in held {
                FreeElement(h);
            }
        }
    }
}

#[test]
fn ai_exact_travel_angle_and_turn_gates_follow_shared_intelligence() {
    let _battle = Battlefield::new(16);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[0].ObjectPtr = enemy.ptr();
        concerns[0].which_turn = 13;
        let ai = ship.desc.cyborg_control.intelligence_func.unwrap();
        for own_angle in 0..64u16 {
            for enemy_angle in 0..64u16 {
                (*ship.ptr()).velocity.TravelAngle = own_angle;
                (*enemy.ptr()).velocity.TravelAngle = enemy_angle;
                ship.ss.ship_input_state = (LEFT | SPECIAL | WEAPON) as u8;
                // Shared intelligence clears WEAPON when weapon_counter is zero.
                ai(ship.ptr(), concerns.as_mut_ptr(), 0);
                let angle = (i32::from(own_angle) - (i32::from(enemy_angle) + 32) + 16) & 63;
                assert_eq!(
                    ship.ss.ship_input_state,
                    LEFT as u8 | if angle > 32 { SPECIAL as u8 } else { 0 },
                    "angles {own_angle}/{enemy_angle}"
                );
            }
        }
        (*ship.ptr()).velocity.TravelAngle = 0;
        (*enemy.ptr()).velocity.TravelAngle = 0;
        for (turn, counter, weapon_counter, has_enemy, expected) in [
            (12, 0, 0, true, false),
            (13, 0, 0, true, true),
            (13, 1, 0, true, false),
            (13, 0, 1, true, false),
            (13, 0, 0, false, false),
        ] {
            concerns[0].which_turn = turn;
            concerns[0].ObjectPtr = if has_enemy {
                enemy.ptr()
            } else {
                ptr::null_mut()
            };
            ship.ss.special_counter = counter;
            ship.ss.weapon_counter = weapon_counter;
            ship.ss.ship_input_state = (WEAPON | SPECIAL) as u8;
            TFB_SeedRandom(101);
            ai(ship.ptr(), concerns.as_mut_ptr(), 0);
            assert_eq!(ship.ss.ship_input_state & SPECIAL as u8 != 0, expected);
            assert_eq!(
                ship.ss.ship_input_state & WEAPON as u8 != 0,
                weapon_counter != 0
            );
            assert_rng_after(101, 0);
        }
    }
}

#[test]
fn tractor_tracking_consumes_only_the_half_turn_rng_before_shadow_allocation() {
    for seed in [1, 17, 12345] {
        let _battle = Battlefield::new(4);
        unsafe {
            let mut ship = Ship::new(0);
            let enemy = Ship::new(1);
            // South is exactly half a circle from TrackShip's initial facing.
            (*enemy.ptr()).current.location = POINT { x: 4000, y: 5000 };
            (*enemy.ptr()).next = (*enemy.ptr()).current;
            ship.ss.cur_status_flags = SPECIAL as u16;
            assert!((*ship.ptr()).hTarget.is_null());
            TFB_SeedRandom(seed);
            ship.postprocess();
            assert_eq!((*ship.ptr()).hTarget, enemy.handle);
            assert_eq!(velocity(enemy.ptr()), (0, -38));
            assert_eq!(ship.desc.ship_info.energy_level, 41);
            // Two free slots produce offsets 8 then 17; later allocations fail.
            let second = element(disp_q.head);
            let first = element((*second).succ);
            assert_eq!(point((*second).current.location), (4000, 5000 - 17 * 4));
            assert_eq!(point((*first).current.location), (4000, 5000 - 8 * 4));
            assert_eq!((*first).succ, ship.handle);
            assert_rng_after(seed, 1);
        }
    }
}

#[test]
fn laser_death_preserves_size_subtraction_and_unsigned_count_magnitude() {
    let _battle = Battlefield::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let h = ship.laser();
        let laser = element(h);
        (*ship.ptr()).current.location = POINT { x: 32760, y: 32760 };
        (*laser).current.location = POINT { x: -8, y: -8 };
        (*laser).__bindgen_anon_4.turn_wait = 255;
        let mut saw_large_magnitude = false;
        for seed in 1..128 {
            TFB_SeedRandom(seed);
            let gate = TFB_Random() as u8 & 7 != 0;
            let draw = TFB_Random() as COUNT;
            let distance_squared = 2_147_483_648u32;
            let magnitude = (i32::from(draw) % (i32::from(square_root(distance_squared)) + 1 - 112)
                + 112) as COUNT;
            let tail = disp_q.tail;
            TFB_SeedRandom(seed);
            ((*laser).death_func.unwrap())(laser);
            assert_eq!(ship.ss.special_counter, 255);
            if gate {
                saw_large_magnitude |= magnitude > i16::MAX as u16;
                let spot_handle = disp_q.tail;
                let spot = element(spot_handle);
                let a = i32::from(ARCTAN(i16::MIN, i16::MIN));
                assert_eq!(
                    point((*spot).current.location),
                    (
                        32760i16.wrapping_add(cosine(a, i32::from(magnitude))),
                        32760i16.wrapping_add(sine(a, i32::from(magnitude))),
                    )
                );
                assert_rng_after(seed, 2);
                RemoveElement(spot_handle);
                FreeElement(spot_handle);
            } else {
                assert_eq!({ disp_q.tail }, tail);
                assert_rng_after(seed, 1);
            }
        }
        assert!(saw_large_magnitude);
        FreeElement(h);
    }
}

#[test]
fn unrequested_special_and_failed_muzzle_allocation_only_reset_cycle() {
    let _battle = Battlefield::new(3);
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.special_counter = 3;
        TFB_SeedRandom(81);
        ship.postprocess();
        assert_eq!(ship.ss.special_counter, 0);
        assert_eq!(ship.desc.ship_info.energy_level, 42);
        assert_eq!({ disp_q.head }, ship.handle);
        assert_rng_after(81, 0);
        let h = ship.laser();
        put_element(h);
        let held = exhaust();
        ship.ss.cur_status_flags = WEAPON as u16;
        ship.ss.special_counter = 2;
        ship.postprocess();
        assert_eq!(ship.ss.special_counter, 0);
        assert_eq!({ disp_q.tail }, h);
        for h in held {
            FreeElement(h);
        }
    }
}
