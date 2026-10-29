#![allow(
    dead_code,
    reason = "shared live-engine fixture used by independent race test binaries"
)]

use std::{ffi::c_void, mem, ptr};
use uqm_rust::ships::native::{abi::*, element, put_element};

unsafe extern "C" {
    static mut DisplayArray: [PRIMITIVE; 330];
}
#[path = "../support/battle.rs"]
mod battle;
pub use battle::{Battle, PoolElement};
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
pub struct FrameSet {
    _drawable: Box<DrawableDesc>,
    frames: Vec<FrameDesc>,
}
impl FrameSet {
    pub fn new(count: u16) -> Self {
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
    pub fn frame(&self, index: usize) -> FRAME {
        (&raw const self.frames[index]).cast_mut().cast()
    }
}
pub struct Record<T>(*mut T);
impl<T> std::ops::Deref for Record<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: this fixture record owns a live slot until its battle drops.
        unsafe { &*self.0 }
    }
}
impl<T> std::ops::DerefMut for Record<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: this exclusive fixture borrow owns the live slot.
        unsafe { &mut *self.0 }
    }
}

pub unsafe fn core(callback: uqm_rust::engine::battle::ElementFn, element: *mut ELEMENT) {
    // SAFETY: The caller retains a live element and the published game-thread context.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let id = cx
                .elements()
                .id(element.cast())
                .unwrap_or_else(|| panic!("fixture element missing from pool"));
            callback(cx, id);
        });
    }
}

pub struct Ship {
    pub desc: Box<RACE_DESC>,
    pub ss: Record<STARSHIP>,
    pub handle: HELEMENT,
    pub ship_frames: FrameSet,
    pub weapon_frames: FrameSet,
    pub special_frames: FrameSet,
}
impl Ship {
    pub unsafe fn new(init: unsafe extern "C" fn() -> *mut RACE_DESC, player: SIZE) -> Self {
        // SAFETY: the caller initialized the serialized fixture pools and retains all loaded frame sets.
        unsafe {
            let mut desc = Box::new(*init());
            let ship_frames = FrameSet::new(16);
            let weapon_frames = FrameSet::new(16);
            let special_frames = FrameSet::new(16);
            desc.ship_data.ship = [ship_frames.frame(0); 3];
            desc.ship_data.weapon = [weapon_frames.frame(0); 3];
            desc.ship_data.special = [special_frames.frame(0); 3];
            let raw = uqm_rust::collections::queue::AllocLink(&raw mut race_q[player as usize])
                .cast::<STARSHIP>();
            assert!(!raw.is_null());
            raw.write(mem::zeroed());
            let mut ss = Record(raw);
            ss.RaceDescPtr = &raw mut *desc;
            ss.playerNr = player;
            let handle = AllocElement();
            assert!(!handle.is_null());
            let e = element(handle);
            (*e).pParent = (&raw mut *ss).cast();
            (*e).playerNr = player;
            (*e).state_flags = PLAYER_SHIP as u16;
            (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
            (*e).__bindgen_anon_2.crew_level = desc.ship_info.crew_level;
            (*e).__bindgen_anon_3.mass_points = desc.characteristics.ship_mass;
            (*e).current.location = POINT { x: 4000, y: 4000 };
            (*e).current.image.farray = (&raw mut desc.ship_data.ship).cast();
            (*e).current.image.frame = ship_frames.frame(0);
            (*e).next = (*e).current;
            (*e).IntersectControl.IntersectStamp.frame = ship_frames.frame(0);
            (*prim(e)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
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
    }
    pub fn ptr(&self) -> *mut ELEMENT {
        self.handle.cast()
    }
    pub unsafe fn weapon(&mut self, expected: COUNT) -> Vec<HELEMENT> {
        let mut out = [ptr::null_mut(); 6];
        assert_eq!(
            (self.desc.init_weapon_func.unwrap())(self.ptr(), out.as_mut_ptr()),
            expected
        );
        for &h in &out[..expected as usize] {
            if !h.is_null() {
                let e = element(h);
                (*e).pParent = (&raw mut *self.ss).cast();
                (*e).next = (*e).current;
                (*e).IntersectControl.IntersectStamp.frame = (*e).current.image.frame;
            }
        }
        out[..expected as usize].to_vec()
    }
    pub unsafe fn intelligence(&mut self, concerns: &mut [EVALUATE_DESC; 4]) {
        (self.desc.cyborg_control.intelligence_func.unwrap())(self.ptr(), concerns.as_mut_ptr(), 0);
    }
}
impl Drop for Ship {
    fn drop(&mut self) {
        // SAFETY: this fixture owns its element and ship slots in the retained battle pools.
        unsafe {
            uqm_rust::collections::queue::FreeLink(
                &raw mut race_q[self.ss.playerNr as usize],
                self.ss.0.cast(),
            );
        }
    }
}
pub unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
    (&raw mut DisplayArray)
        .cast::<PRIMITIVE>()
        .add((*e).PrimIndex as usize)
}
pub unsafe fn queued() -> Vec<HELEMENT> {
    let mut all = Vec::new();
    let mut h = disp_q.head;
    while !h.is_null() {
        all.push(h);
        assert!(all.len() <= 128, "queue cycle");
        h = (*element(h)).succ;
    }
    all
}
pub fn velocity(v: &VELOCITY_DESC) -> [i32; 9] {
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
pub unsafe fn toward(speed: SIZE, face: COUNT) -> [i32; 9] {
    let mut v = mem::zeroed();
    SetVelocityVector(&mut v, speed, face);
    velocity(&v)
}
pub unsafe fn with_rng<T>(seed: u32, draws: usize, run: impl FnOnce() -> T) -> T {
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
