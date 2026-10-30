#![allow(
    dead_code,
    reason = "shared fixture API used by independent race test binaries"
)]
use std::{ffi::c_void, mem, ptr};
use uqm_rust::battle::process_native::InitDisplayList;
use uqm_rust::collections::queue::{AllocLink, FreeLink, InitQueue, UninitQueue};
use uqm_rust::engine::battle::legacy::DisplayArray;
use uqm_rust::{
    engine::battle::{ElementId, RaceState, ShipId},
    ships::native::{abi::*, element, put_element},
};

pub struct Battle {
    display: *mut PRIMITIVE,
    dimensions: (i32, i32),
    graphics: u8,
}
impl Battle {
    pub fn new(capacity: COUNT) -> Self {
        // SAFETY: battle_context serializes this binary's game authorities.
        unsafe {
            let display = GlobData.Game_state.DisplayArray;
            let dimensions = (ScreenWidth, ScreenHeight);
            let graphics = uqm_rust::graphics::context_abi::_GraphicsStatusFlags;
            uqm_rust::graphics::context_abi::_GraphicsStatusFlags &= !4;
            assert!(InitQueue(
                &raw mut disp_q,
                capacity,
                size_of::<ELEMENT>() as u16
            ));
            let queues = &raw mut race_q;
            for queue in &mut *queues {
                assert!(InitQueue(queue, 32, size_of::<STARSHIP>() as u16));
            }
            InitDisplayList();
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
                display,
                dimensions,
                graphics,
            }
        }
    }
}
impl Drop for Battle {
    fn drop(&mut self) {
        // SAFETY: ships and elements created after this owner have already dropped.
        unsafe {
            UninitQueue(&raw mut disp_q);
            let queues = &raw mut race_q;
            for queue in &mut *queues {
                UninitQueue(queue);
            }
            GlobData.Game_state.DisplayArray = self.display;
            (ScreenWidth, ScreenHeight) = self.dimensions;
            uqm_rust::graphics::context_abi::_GraphicsStatusFlags = self.graphics;
        }
    }
}

struct Record<T>(*mut T);
impl<T> Record<T> {
    unsafe fn get(&self) -> &T {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe { &*self.0 }
    }
    unsafe fn get_mut(&mut self) -> &mut T {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe { &mut *self.0 }
    }
}

pub struct Ship<'a> {
    battle: &'a Battle,
    desc: Box<RACE_DESC>,
    ss: Record<STARSHIP>,
    el: Record<ELEMENT>,
    pub handle: HELEMENT,
    pub id: ElementId,
    pub ship_id: ShipId,
    frames: Option<[FrameSet; 3]>,
    queued: bool,
}
impl<'a> Ship<'a> {
    pub fn bare(battle: &'a Battle, species: u32, player: SIZE) -> Self {
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
            let id = uqm_rust::app::bridge::enter(|cx| {
                cx.elements()
                    .id(handle)
                    .unwrap_or_else(|| panic!("allocated element missing from pool"))
            });
            let ship_id = ShipId {
                side: player as u8,
                slot: ((raw_ship as usize - race_q[player as usize].pq_tab as usize)
                    / size_of::<STARSHIP>()) as u8,
            };
            Self {
                battle,
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
    pub fn native(
        battle: &'a Battle,
        race: &'static uqm_rust::engine::battle::Race,
        player: SIZE,
    ) -> Self {
        let mut ship = Self::bare(battle, SPECIES_ID_VUX_ID, player);
        // SAFETY: construction runs in this fixture's serialized, published battle.
        *ship.desc =
            unsafe { uqm_rust::app::bridge::enter(|cx| (race.descriptor)(cx.world).into_legacy()) };
        ship
    }
    pub unsafe fn new(battle: &'a Battle, species: u32, player: SIZE) -> Self {
        // SAFETY: the caller retains the serialized battle and its frame resources.
        unsafe { Self::with_frames(battle, species, player, [16; 3], [4, 4]) }
    }
    /// # Safety
    /// Retain the serialized battle until this ship drops; do not retain projections across callbacks.
    pub unsafe fn with_frames(
        battle: &'a Battle,
        species: u32,
        player: SIZE,
        counts: [u16; 3],
        bounds: [i16; 2],
    ) -> Self {
        // SAFETY: the caller retains the serialized battle through fixture construction.
        unsafe {
            let mut ship = Self::bare(battle, species, player);
            let frames = counts.map(|count| FrameSet::with_bounds(count, bounds));
            ship.desc.ship_data.ship = [frames[0].frame(0); 3];
            ship.desc.ship_data.weapon = [frames[1].frame(0); 3];
            ship.desc.ship_data.special = [frames[2].frame(0); 3];
            ship.element_mut().collision_func =
                Some(uqm_rust::engine::battle::CollisionHook::Native(
                    uqm_rust::battle::ship_native::collision,
                ));
            ship.element_mut().set_life_span(NORMAL_LIFE as u16);
            let crew = ship.desc.ship_info.crew_level;
            ship.element_mut().set_hit_points(crew);
            let mass = ship.desc.characteristics.ship_mass;
            ship.element_mut().set_mass_points(mass);
            ship.element_mut().current.location = POINT { x: 4000, y: 4000 };
            ship.element_mut().current.image.farray = ship.ship_frames();
            ship.element_mut().current.image.frame = frames[0].frame(0);
            ship.element_mut().next = ship.element_mut().current;
            ship.element_mut().IntersectControl.IntersectStamp.frame = frames[0].frame(0);
            // SAFETY: this slot owns its allocated display primitive.

            (*prim(ship.ptr())).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;

            ship.frames = Some(frames);
            ship.queue();
            ship
        }
    }
    pub fn raw_frame(&self, role: usize, index: usize) -> FRAME {
        self.frames
            .as_ref()
            .unwrap_or_else(|| panic!("fixture frames"))[role]
            .frame(index)
    }
    pub fn make_opaque(&mut self, role: usize) {
        self.frames
            .as_mut()
            .unwrap_or_else(|| panic!("fixture frames"))[role]
            .make_opaque();
    }
    pub fn set_frame_bounds(&mut self, role: usize, index: usize, bounds: [i16; 2]) {
        self.frames
            .as_mut()
            .unwrap_or_else(|| panic!("fixture frames"))[role]
            .frames[index]
            .bounds = bounds;
    }
    pub fn weapon_handles(&mut self, expected: COUNT) -> Vec<HELEMENT> {
        // SAFETY: the owning battle and ship remain live throughout weapon creation.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let origin = cx.weapon_origin(self.id);
                let out = uqm_rust::engine::battle::Cx::spawn_weapons(cx, self.ship_id, origin);
                assert_eq!(out.len(), usize::from(expected));
                out.into_iter()
                    .map(|id| id.map_or(ptr::null_mut(), |id| cx.legacy_element(id).cast()))
                    .collect()
            })
        }
    }
    /// # Safety
    /// The subject and concerns must be live fixture operands retained through the callback.
    pub unsafe fn think(
        &mut self,
        subject: *mut ELEMENT,
        concerns: &mut [EVALUATE_DESC; 4],
        count: u16,
    ) {
        // SAFETY: callers supply this fixture's live subject, without retaining a record borrow.
        unsafe {
            uqm_rust::engine::battle::Cx::invoke_intelligence(
                self.ss.0 as usize,
                subject,
                concerns,
                count,
            );
        }
    }
    pub fn set_hot_spot(&mut self, role: usize, index: usize, hot_spot: [i16; 2]) {
        self.frames
            .as_mut()
            .unwrap_or_else(|| panic!("loaded fixture frames"))[role]
            .frames[index]
            .hot_spot = hot_spot;
    }
    /// # Safety
    /// The source must be a live element in this fixture's retained battle pool.
    pub unsafe fn initialize_from(&self, source: *mut ELEMENT, expected: u16) -> Vec<HELEMENT> {
        // SAFETY: the caller retains all pool records through the synchronous Native dispatch.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let source = cx
                    .elements()
                    .id(source.cast())
                    .unwrap_or_else(|| panic!("weapon source"));
                let origin = cx.weapon_origin(source);
                let out = cx.spawn_weapons(self.ship_id, origin);
                assert_eq!(out.len(), usize::from(expected));
                out.into_iter()
                    .map(|id| id.map_or(ptr::null_mut(), |id| cx.legacy_element(id).cast()))
                    .collect()
            })
        }
    }
    pub fn frame(&self, role: usize, index: u16) -> uqm_rust::engine::battle::views::Frame {
        self.assert_live();
        // SAFETY: the fixture retains its descriptor and loaded frame sets through projection.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                cx.absolute_frame(cx.ship_frame(self.ship_id, role), index)
            })
        }
    }
    pub fn frame_set(&self, role: usize) -> uqm_rust::engine::battle::views::Frames {
        self.assert_live();
        // SAFETY: the fixture retains its descriptor and loaded frame sets through projection.
        unsafe { uqm_rust::app::bridge::enter(|cx| cx.ship_frame_set(self.ship_id, role)) }
    }
    pub fn queue(&mut self) {
        assert!(!self.queued);
        // SAFETY: this fixture owns the unlinked, allocated slot.
        unsafe {
            put_element(self.handle);
        }
        self.queued = true;
    }
    fn assert_live(&self) {
        // SAFETY: only a checked generation is inspected while the fixture retains the battle.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                assert!(cx.elements().contains(self.id), "retired fixture record")
            });
        }
    }
    /// # Safety
    /// Do not retain record references across any engine or fixture hook call.
    pub unsafe fn element_mut(&mut self) -> &mut ELEMENT {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.assert_live();
            self.el.get_mut()
        }
    }
    /// # Safety
    /// Do not retain record references across any engine or fixture hook call.
    pub unsafe fn ship_mut(&mut self) -> &mut STARSHIP {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.assert_live();
            self.ss.get_mut()
        }
    }
    /// # Safety
    /// Do not retain descriptor references across any engine or fixture hook call.
    pub unsafe fn descriptor_mut(&mut self) -> &mut RACE_DESC {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.

        self.assert_live();
        &mut self.desc
    }
    pub fn snapshot(&self) -> uqm_rust::engine::battle::views::ElementState {
        // SAFETY: the fixture retains its battle and checked element through this value projection.
        unsafe { uqm_rust::app::bridge::enter(|cx| cx.element(self.id)) }
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
    pub unsafe fn guardian_form(mut self) -> Self {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.element_mut().next.image.farray = self.ship_frames();
            self
        }
    }
    pub unsafe fn blazer_form(mut self) -> Self {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.element_mut().next.image.farray = self.special_frames();
            self
        }
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
                uqm_rust::engine::battle::Cx::invoke_ship_process(cx, self.ship_id, post);
            });
        }
    }
    pub fn weapon(&mut self, expected: COUNT) -> Vec<Option<ElementId>> {
        // SAFETY: the owning battle and ship remain live throughout weapon creation.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let origin = cx.weapon_origin(self.id);
                let out = uqm_rust::engine::battle::Cx::spawn_weapons(cx, self.ship_id, origin);
                assert_eq!(out.len(), usize::from(expected));
                for &id in out.iter().flatten() {
                    cx.set_parent(id, Some(self.ship_id));
                    let current = cx.element(id).current;
                    cx.copy_visual_to_next(id, current);
                    cx.set_intersect_frame(id, current.image.frame);
                }
                out
            })
        }
    }
    /// # Safety
    /// Do not retain the legacy pointer across callbacks or disposal.
    pub unsafe fn weapon_record(&self, id: Option<ElementId>) -> *mut ELEMENT {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                cx.legacy_element(id.unwrap_or_else(|| panic!("{}", "allocated weapon")))
            })
        }
    }
    pub unsafe fn intelligence(
        &mut self,
        subject: &mut ELEMENT,
        concerns: &mut [EVALUATE_DESC; 4],
        count: u16,
    ) {
        // SAFETY: subject and four concerns live through this synchronous callback.
        unsafe {
            uqm_rust::engine::battle::Cx::invoke_intelligence(
                self.ss.0 as usize,
                subject,
                concerns,
                count,
            );
        }
    }
    pub unsafe fn intelligence(
        &mut self,
        subject: &mut ELEMENT,
        concerns: &mut [EVALUATE_DESC],
        count: u16,
    ) {
        // SAFETY: subject and the supplied concerns live through this synchronous callback.
        unsafe {
            uqm_rust::engine::battle::Cx::invoke_intelligence(
                self.ss.0 as usize,
                subject,
                concerns,
                count,
            );
        }
    }
    pub fn uninit(&mut self) {
        if let Some(race) = self.desc.race {
            if let Some(uninit) = race.uninit {
                // SAFETY: the fixture retains the checked ship through its typed teardown.
                unsafe {
                    uqm_rust::app::bridge::enter(|cx| {
                        let mut stats = cx.race_stats(self.ship_id);
                        uninit(cx.world, &mut stats);
                        *cx.race_stats_mut(self.ship_id) = stats;
                    });
                }
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
impl Drop for Ship<'_> {
    fn drop(&mut self) {
        self.uninit();
        // SAFETY: this fixture uniquely owns its element and ship slots.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                if cx.elements().contains(self.id) {
                    if cx.elements().linked(self.id) {
                        cx.unlink(self.id);
                    }
                    cx.dispose(self.id);
                }
            });
            FreeLink(
                &raw mut race_q[self.ship_id.side as usize],
                self.ss.0.cast(),
            );
        }
    }
}
pub fn descriptor(species: u32) -> RACE_DESC {
    // SAFETY: the caller owns the serialized test context.
    unsafe { uqm_rust::ships::races::legacy_descriptor(species) }
}

pub unsafe fn call_element(name: &str, id: ElementId) {
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.

    let address = uqm_rust::ships::hook_registry::hooks()
        .into_iter()
        .find_map(|(address, n)| (n == name).then_some(address))
        .unwrap_or_else(|| panic!("{}", "registered element hook"));
    // SAFETY: the serialized fixture owns the checked id and its installed callback.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let e = &cx.elements()[id];
            let callback = [e.preprocess_func, e.postprocess_func, e.death_func]
                .into_iter()
                .flatten()
                .find(|h| h.address() == address)
                .unwrap_or_else(|| panic!("{}", "named hook installed on element"));
            uqm_rust::battle::process_native::invoke_element::invoke_element(cx, callback, id);
        });
    }
}

pub fn call_collision(name: &str, a: ElementId, pa: &mut POINT, b: ElementId, pb: &mut POINT) {
    let address = uqm_rust::ships::hook_registry::hooks()
        .into_iter()
        .find_map(|(address, n)| (n == name).then_some(address))
        .unwrap_or_else(|| panic!("{}", "registered collision hook"));
    // SAFETY: both checked elements belong to the current serialized battle.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let hook = cx
                .element(a)
                .collide
                .unwrap_or_else(|| panic!("{}", "installed collision hook"));
            assert_eq!(
                hook.address(),
                address,
                "named collision installed on element"
            );
            uqm_rust::battle::process_native::invoke_collision::invoke_collision(
                cx, hook, a, pa, b, pb,
            );
        })
    }
}
/// # Safety
/// The pointer must be live pool storage belonging to the current test battle.
pub unsafe fn id(pointer: *mut ELEMENT) -> ElementId {
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            cx.elements()
                .id(pointer.cast())
                .unwrap_or_else(|| panic!("{}", "pool record"))
        })
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
    image: *mut c_void,
}
impl FrameSet {
    pub fn new(count: u16) -> Self {
        Self::with_bounds(count, [4, 4])
    }
    pub fn with_bounds(count: u16, bounds: [i16; 2]) -> Self {
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
                bounds,
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
    pub fn make_opaque(&mut self) {
        use uqm_rust::graphics::{
            native_canvas::{
                lifecycle::TFB_DrawCanvas_New_Paletted, pixels::TFB_DrawCanvas_SetPixelIndexes,
            },
            tfb_draw_abi::{TFB_DrawImage_Delete, TFB_DrawImage_New},
        };
        // SAFETY: the fixture owns these small SDL canvas/image resources and frees them on drop.
        unsafe {
            let canvas = TFB_DrawCanvas_New_Paletted(4, 4, ptr::null_mut(), -1);
            assert!(!canvas.is_null());
            assert_ne!(
                TFB_DrawCanvas_SetPixelIndexes(canvas, [1u8; 16].as_ptr(), 4, 4),
                0
            );
            if !self.image.is_null() {
                TFB_DrawImage_Delete(self.image.cast());
            }
            self.image = TFB_DrawImage_New(canvas).cast();
            assert!(!self.image.is_null());
            for frame in &mut self.frames {
                frame.image = self.image;
            }
        }
    }
    pub fn frame(&self, index: usize) -> FRAME {
        (&raw const self.frames[index]).cast_mut().cast()
    }
}
impl Drop for FrameSet {
    fn drop(&mut self) {
        if !self.image.is_null() {
            // SAFETY: this frame set uniquely owns the image and its canvas.
            unsafe {
                uqm_rust::graphics::tfb_draw_abi::TFB_DrawImage_Delete(self.image.cast());
            }
        }
    }
}
pub unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
    unsafe {
        (&raw mut DisplayArray)
            .cast::<PRIMITIVE>()
            .add((*e).PrimIndex as usize)
    }
}
pub unsafe fn queued() -> Vec<HELEMENT> {
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
    unsafe {
        let mut all = Vec::new();
        let mut h = disp_q.head;
        while !h.is_null() {
            all.push(h);
            assert!(all.len() <= 128, "queue cycle");
            h = (*element(h)).succ;
        }
        all
    }
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
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
    unsafe {
        let mut v = mem::zeroed();
        SetVelocityVector(&mut v, speed, face);
        velocity(&v)
    }
}
pub unsafe fn with_rng<T>(seed: u32, draws: usize, run: impl FnOnce() -> T) -> T {
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.

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

pub struct Object<'a> {
    battle: &'a Battle,
    el: Record<ELEMENT>,
    pub id: ElementId,
}
impl<'a> Object<'a> {
    pub fn new(battle: &'a Battle) -> Self {
        // SAFETY: object slots belong to the serialized test battle.
        unsafe {
            let handle = AllocElement();
            assert!(!handle.is_null());
            let id = uqm_rust::app::bridge::enter(|cx| {
                cx.elements()
                    .id(handle)
                    .unwrap_or_else(|| panic!("allocated element missing from pool"))
            });
            Self {
                battle,
                el: Record(handle.cast()),
                id,
            }
        }
    }
    /// # Safety
    /// Do not retain record references across any engine or fixture hook call.
    pub unsafe fn element_mut(&mut self) -> &mut ELEMENT {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                assert!(cx.elements().contains(self.id), "retired fixture object")
            });
            self.el.get_mut()
        }
    }
    pub fn ptr(&mut self) -> *mut ELEMENT {
        self.el.0
    }
}
impl Drop for Object<'_> {
    fn drop(&mut self) {
        // SAFETY: the fixture owns this unlinked object slot.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                if cx.elements().contains(self.id) {
                    if cx.elements().linked(self.id) {
                        cx.unlink(self.id);
                    }
                    cx.dispose(self.id);
                }
            });
        }
    }
}

#[path = "ship_pool.rs"]
mod ship_pool;
pub type PoolShip = ship_pool::PoolShip;

/// An unlinked pool slot for legacy callback operands.
pub struct PoolElement {
    pointer: *mut ELEMENT,
    id: ElementId,
}
impl PoolElement {
    /// # Safety
    /// The serialized fixture must retain its battle pools until this owner drops.
    pub unsafe fn new() -> Self {
        // SAFETY: the caller retains the initialized, serialized element pool.
        unsafe {
            let pointer = AllocElement().cast::<ELEMENT>();
            assert!(!pointer.is_null(), "fixture element pool exhausted");
            let id = id(pointer);
            Self { pointer, id }
        }
    }
}
impl std::ops::Deref for PoolElement {
    type Target = ELEMENT;
    fn deref(&self) -> &ELEMENT {
        // SAFETY: this owner retains the allocated operand slot.
        unsafe { &*self.pointer }
    }
}
impl std::ops::DerefMut for PoolElement {
    fn deref_mut(&mut self) -> &mut ELEMENT {
        // SAFETY: this exclusive fixture borrow owns the operand slot.
        unsafe { &mut *self.pointer }
    }
}
impl Drop for PoolElement {
    fn drop(&mut self) {
        // SAFETY: the construction contract retains the serialized fixture pools.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                if cx.elements().contains(self.id) {
                    if cx.elements().linked(self.id) {
                        cx.unlink(self.id);
                    }
                    cx.dispose(self.id);
                }
            });
        }
    }
}
