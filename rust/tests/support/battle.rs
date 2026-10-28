#![allow(
    dead_code,
    reason = "shared fixture API used by independent race test binaries"
)]
use std::{
    ffi::c_void,
    mem,
    ops::{Deref, DerefMut},
    ptr,
};
use uqm_rust::battle::process_native::{FreeElement, InitDisplayList, RemoveElement};
use uqm_rust::collections::queue::{AllocLink, FreeLink, InitQueue, UninitQueue};
use uqm_rust::engine::battle::legacy::DisplayArray;
use uqm_rust::{
    engine::battle::{ElementId, RaceState, ShipId},
    ships::native::{abi::*, element, put_element},
};

pub struct Battle {
    display: *mut PRIMITIVE,
    dimensions: (i32, i32),
}
impl Battle {
    pub fn new(capacity: COUNT) -> Self {
        // SAFETY: battle_context serializes this binary's game authorities.
        unsafe {
            let display = GlobData.Game_state.DisplayArray;
            let dimensions = (ScreenWidth, ScreenHeight);
            assert!(InitQueue(
                &raw mut disp_q,
                capacity,
                size_of::<ELEMENT>() as u16
            ));
            for side in 0..2 {
                assert!(InitQueue(
                    &raw mut race_q[side],
                    32,
                    size_of::<STARSHIP>() as u16
                ));
            }
            InitDisplayList();
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
                display,
                dimensions,
            }
        }
    }
}
impl Drop for Battle {
    fn drop(&mut self) {
        // SAFETY: ships and elements created after this owner have already dropped.
        unsafe {
            UninitQueue(&raw mut disp_q);
            for side in 0..2 {
                UninitQueue(&raw mut race_q[side]);
            }
            GlobData.Game_state.DisplayArray = self.display;
            (ScreenWidth, ScreenHeight) = self.dimensions;
        }
    }
}

pub struct Record<T>(*mut T);
impl<T> Deref for Record<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: this non-clone fixture owns the live slot, used between hook calls.
        unsafe { &*self.0 }
    }
}
impl<T> DerefMut for Record<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: this non-clone fixture owns the live slot, used between hook calls.
        unsafe { &mut *self.0 }
    }
}

pub struct Ship {
    pub desc: Box<RACE_DESC>,
    pub ss: Record<STARSHIP>,
    pub el: Record<ELEMENT>,
    pub handle: HELEMENT,
    pub id: ElementId,
    pub ship_id: ShipId,
    frames: Option<[FrameSet; 3]>,
    queued: bool,
}
impl Ship {
    pub fn bare(species: u32, player: SIZE) -> Self {
        // SAFETY: construction runs inside the serialized, published test battle.
        unsafe {
            let mut desc = Box::new(descriptor(species));
            let raw_ship = AllocLink(&raw mut race_q[player as usize]).cast::<STARSHIP>();
            assert!(!raw_ship.is_null());
            raw_ship.write(mem::zeroed());
            (*raw_ship).RaceDescPtr = &raw mut *desc;
            (*raw_ship).playerNr = player;
            (*raw_ship).SpeciesID = species;
            let handle = AllocElement();
            assert!(!handle.is_null());
            let el = element(handle);
            (*el).pParent = raw_ship.cast();
            (*el).playerNr = player;
            (*el).state_flags = PLAYER_SHIP as u16;
            (*raw_ship).hShip = handle;
            let id = uqm_rust::app::bridge::enter(|cx| cx.b.elements.id(handle).unwrap());
            let ship_id = ShipId {
                side: player as u8,
                slot: ((raw_ship as usize - race_q[player as usize].pq_tab as usize)
                    / size_of::<STARSHIP>()) as u8,
            };
            Self {
                desc,
                ss: Record(raw_ship),
                el: Record(el),
                handle,
                id,
                ship_id,
                frames: None,
                queued: false,
            }
        }
    }
    pub fn new(species: u32, player: SIZE) -> Self {
        let mut ship = Self::bare(species, player);
        let frames = [FrameSet::new(16), FrameSet::new(16), FrameSet::new(16)];
        ship.desc.ship_data.ship = [frames[0].frame(0); 3];
        ship.desc.ship_data.weapon = [frames[1].frame(0); 3];
        ship.desc.ship_data.special = [frames[2].frame(0); 3];
        ship.el.set_life_span(NORMAL_LIFE as u16);
        let crew = ship.desc.ship_info.crew_level;
        ship.el.set_hit_points(crew);
        let mass = ship.desc.characteristics.ship_mass;
        ship.el.set_mass_points(mass);
        ship.el.current.location = POINT { x: 4000, y: 4000 };
        ship.el.current.image.farray = ship.ship_frames();
        ship.el.current.image.frame = frames[0].frame(0);
        ship.el.next = ship.el.current;
        ship.el.IntersectControl.IntersectStamp.frame = frames[0].frame(0);
        // SAFETY: this slot owns its allocated display primitive.
        unsafe {
            (*prim(ship.ptr())).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        }
        ship.frames = Some(frames);
        ship.queue();
        ship
    }
    pub fn queue(&mut self) {
        assert!(!self.queued);
        // SAFETY: this fixture owns the unlinked, allocated slot.
        unsafe {
            put_element(self.handle);
        }
        self.queued = true;
    }
    pub fn ptr(&self) -> *mut ELEMENT {
        self.el.0
    }
    pub fn el(&mut self) -> *mut ELEMENT {
        self.ptr()
    }
    pub fn ship_frames(&self) -> *mut FRAME {
        (&raw const self.desc.ship_data.ship).cast_mut().cast()
    }
    pub fn special_frames(&self) -> *mut FRAME {
        (&raw const self.desc.ship_data.special).cast_mut().cast()
    }
    pub fn guardian_form(mut self) -> Self {
        self.el.next.image.farray = self.ship_frames();
        self
    }
    pub fn blazer_form(mut self) -> Self {
        self.el.next.image.farray = self.special_frames();
        self
    }
    pub fn preprocess(&mut self) {
        self.process(false);
    }
    pub fn postprocess(&mut self) {
        self.process(true);
    }
    fn process(&mut self, post: bool) {
        // SAFETY: no fixture record borrow crosses the context handoff.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                uqm_rust::battle::process_native::invoke_race::invoke_ship_process(
                    cx,
                    self.ship_id,
                    post,
                );
            });
        }
    }
    pub fn weapon(&mut self, expected: COUNT) -> Vec<HELEMENT> {
        let mut out = [ptr::null_mut(); 6];
        // SAFETY: the projected subject and six output slots are live for this call.
        unsafe {
            assert_eq!(
                uqm_rust::battle::process_native::invoke_race::invoke_weapon(
                    self.ss.0 as usize,
                    self.ptr(),
                    out.as_mut_ptr()
                ),
                expected
            );
        }
        for &h in &out[..expected as usize] {
            if !h.is_null() {
                // SAFETY: outputs are live, newly allocated elements.
                unsafe {
                    let e = element(h);
                    (*e).pParent = self.ss.0.cast();
                    (*e).next = (*e).current;
                    (*e).IntersectControl.IntersectStamp.frame = (*e).current.image.frame;
                }
            }
        }
        out[..expected as usize].to_vec()
    }
    pub fn intelligence(&mut self, concerns: &mut [EVALUATE_DESC; 4]) {
        // SAFETY: subject and four concerns live through this synchronous callback.
        unsafe {
            uqm_rust::battle::process_native::invoke_race::invoke_intelligence(
                self.ss.0 as usize,
                self.ptr(),
                concerns.as_mut_ptr(),
                0,
            );
        }
    }
    pub fn uninit(&mut self) {
        if let Some(race) = self.desc.race {
            if let Some(uninit) = race.uninit {
                uninit(&mut self.desc);
            }
        }
    }
    pub fn saved_collision(&self) -> ElementCollisionFunc {
        let RaceState::Androsynth { collision } = self.desc.race_state else {
            panic!("no saved Androsynth collision")
        };
        collision
    }
}
impl Drop for Ship {
    fn drop(&mut self) {
        self.uninit();
        // SAFETY: this fixture uniquely owns its element and ship slots.
        unsafe {
            if self.queued {
                RemoveElement(self.handle);
            }
            FreeElement(self.handle);
            FreeLink(
                &raw mut race_q[self.ship_id.side as usize],
                self.ss.0.cast(),
            );
        }
    }
}
pub fn descriptor(species: u32) -> RACE_DESC {
    let race = uqm_rust::ships::races::race(species).expect("converted test race");
    (race.descriptor)()
}

pub fn call_element(name: &str, id: ElementId) {
    let address = uqm_rust::ships::hook_registry::hooks()
        .into_iter()
        .find_map(|(address, n)| (n == name).then_some(address))
        .expect("registered element hook");
    // SAFETY: the serialized fixture owns the checked id and its installed callback.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let e = &cx.b.elements[id];
            let callback = [e.preprocess_func, e.postprocess_func, e.death_func]
                .into_iter()
                .flatten()
                .find(|h| h.address() == address)
                .expect("named hook installed on element");
            uqm_rust::battle::process_native::invoke_element::invoke_element(cx, callback, id);
        });
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

pub struct Object {
    pub el: Record<ELEMENT>,
    pub id: ElementId,
}
impl Object {
    pub fn new() -> Self {
        // SAFETY: object slots belong to the serialized test battle.
        unsafe {
            let handle = AllocElement();
            assert!(!handle.is_null());
            let id = uqm_rust::app::bridge::enter(|cx| cx.b.elements.id(handle).unwrap());
            Self {
                el: Record(handle.cast()),
                id,
            }
        }
    }
    pub fn ptr(&mut self) -> *mut ELEMENT {
        self.el.0
    }
}
impl Deref for Object {
    type Target = ELEMENT;
    fn deref(&self) -> &ELEMENT {
        &self.el
    }
}
impl DerefMut for Object {
    fn deref_mut(&mut self) -> &mut ELEMENT {
        &mut self.el
    }
}
impl Drop for Object {
    fn drop(&mut self) {
        // SAFETY: the fixture owns this unlinked object slot.
        unsafe {
            FreeElement(self.el.0.cast());
        }
    }
}
