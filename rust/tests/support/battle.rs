#![allow(
    dead_code,
    reason = "shared fixture API used by independent race test binaries"
)]
use std::{mem, ptr, rc::Rc};
use uqm_rust::battle::process_native::InitDisplayList;
use uqm_rust::engine::battle::schema::HLINK;
use uqm_rust::{
    engine::battle::{ElementId, RaceState, ShipId},
    ships::native::abi::*,
};

unsafe fn invoke_intelligence(
    ship: ShipId,
    subject: &mut uqm_rust::engine::battle::Subject,
    raw: &mut [EVALUATE_DESC],
    count: u16,
) {
    // SAFETY: fixtures serialize the context and retain their subject and concern operands.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let mut concerns = raw
                .iter()
                .map(|item| uqm_rust::engine::battle::Concern {
                    object: cx.elements().id(item.ObjectPtr.cast()),
                    facing: item.facing,
                    which_turn: item.which_turn,
                    movement: item.MoveState,
                })
                .collect::<Vec<_>>();
            cx.invoke_intelligence(ship, subject, &mut concerns, count);
            for (raw, item) in raw.iter_mut().zip(concerns) {
                *raw = EVALUATE_DESC {
                    ObjectPtr: item
                        .object
                        .map_or(ptr::null_mut(), |id| cx.legacy_element(id)),
                    facing: item.facing,
                    which_turn: item.which_turn,
                    MoveState: item.movement,
                };
            }
        });
    }
}

/// # Safety
/// Both operands belong to the serialized fixture's live element pool.
pub unsafe fn plot_intercept(
    a: *mut ElementRecord,
    b: *mut ElementRecord,
    turns: u16,
    margin: u16,
) -> u16 {
    // SAFETY: fixture operands and their retained frame resources live through the projection.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            cx.plot_intercept(
                &cx.weapon_origin(
                    cx.elements()
                        .id(a.cast())
                        .unwrap_or_else(|| panic!("fixture operand")),
                ),
                &cx.weapon_origin(
                    cx.elements()
                        .id(b.cast())
                        .unwrap_or_else(|| panic!("fixture operand")),
                ),
                turns,
                margin,
            )
        })
    }
}

/// # Safety
/// The caller retains the published, serialized fixture battle.
pub unsafe fn head() -> HLINK {
    // SAFETY: the fixture owns the active context for this synchronous lookup.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            cx.elements()
                .head()
                .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
        })
    }
}

/// # Safety
/// The caller retains the published, serialized fixture battle.
pub unsafe fn tail() -> HLINK {
    // SAFETY: the fixture owns the active context for this synchronous lookup.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            cx.elements()
                .tail()
                .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
        })
    }
}

/// # Safety
/// The operand must be a retained element allocation of the serialized fixture.
pub unsafe fn succ<T>(pointer: *mut T) -> HLINK {
    // SAFETY: the fixture retains the operand throughout this synchronous projection.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let id = cx
                .elements()
                .id(pointer.cast())
                .unwrap_or_else(|| panic!("fixture operand"));
            cx.elements()
                .succ(id)
                .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
        })
    }
}

/// # Safety
/// The operand must be a retained element allocation of the serialized fixture.
pub unsafe fn pred<T>(pointer: *mut T) -> HLINK {
    // SAFETY: the fixture retains the operand throughout this synchronous projection.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let id = cx
                .elements()
                .id(pointer.cast())
                .unwrap_or_else(|| panic!("fixture operand"));
            cx.elements()
                .pred(id)
                .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
        })
    }
}

pub struct Battle {
    dimensions: (i32, i32),
    graphics: u8,
}
impl Battle {
    pub fn new(capacity: COUNT) -> Self {
        // SAFETY: battle_context serializes this binary's game authorities.
        unsafe {
            let dimensions = (ScreenWidth, ScreenHeight);
            let graphics = uqm_rust::graphics::context_abi::_GraphicsStatusFlags;
            uqm_rust::graphics::context_abi::_GraphicsStatusFlags &= !4;
            uqm_rust::app::bridge::battle(|battle| {
                (*uqm_rust::engine::battle::record::Kernel::current())
                    .reset_element_capacity(usize::from(capacity));
            });
            uqm_rust::app::bridge::battle(|battle| {
                for side in 0..2 {
                    (*uqm_rust::engine::battle::record::Kernel::current())
                        .reset_ship_capacity(side, 32);
                }
            });
            InitDisplayList();
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
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
            uqm_rust::app::bridge::battle(|battle| {
                (*uqm_rust::engine::battle::record::Kernel::current()).reset_element_capacity(0)
            });
            uqm_rust::app::bridge::battle(|battle| {
                for side in 0..2 {
                    (*uqm_rust::engine::battle::record::Kernel::current())
                        .reset_ship_capacity(side, 0);
                }
            });
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
    ss: Record<ShipRecord>,
    el: Record<ElementRecord>,
    pub handle: HLINK,
    pub id: ElementId,
    pub ship_id: ShipId,
    frames: Option<[FrameSet; 3]>,
    queued: bool,
}
impl<'a> Ship<'a> {
    pub fn bare(battle: &'a Battle, species: u32, player: SIZE) -> Self {
        // SAFETY: construction runs inside the serialized, published test battle.
        unsafe {
            let desc = Box::new(descriptor(species));
            let (raw_ship, ship_id) = uqm_rust::app::bridge::battle(|_| {
                let mut ships =
                    (*uqm_rust::engine::battle::record::Kernel::current()).ships(player as usize);
                let link = ships
                    .alloc()
                    .unwrap_or_else(|| panic!("fixture ship pool exhausted"));
                (ships.raw(link).cast::<ShipRecord>(), ships.ship_id(link))
            });
            (*raw_ship).descriptor = Some(desc);
            (*raw_ship).playerNr = player;
            (*raw_ship).SpeciesID = species;
            let handle = uqm_rust::app::bridge::enter(|cx| {
                cx.alloc_element()
                    .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
            });
            assert!(!handle.is_null());
            let el = element(handle);
            (*el).playerNr = player;
            (*el).state_flags = PLAYER_SHIP as u16;
            let id = uqm_rust::app::bridge::enter(|cx| {
                cx.elements()
                    .id(handle)
                    .unwrap_or_else(|| panic!("allocated element missing from pool"))
            });
            (*raw_ship).hShip = Some(id);
            (*el).parent = uqm_rust::engine::battle::Parent::Ship(ship_id);
            Self {
                battle,
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
        unsafe {
            *ship.descriptor_mut() =
                uqm_rust::app::bridge::enter(|cx| (race.descriptor)(cx.world).into_legacy());
        }
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
            ship.descriptor_mut().ship_data.ship = frames[0].array();
            ship.descriptor_mut().ship_data.weapon = frames[1].array();
            ship.descriptor_mut().ship_data.special = frames[2].array();
            ship.element_mut().collision_func =
                Some(uqm_rust::engine::battle::CollisionHook::Native(
                    uqm_rust::battle::ship_native::collision,
                ));
            ship.element_mut().set_life_span(NORMAL_LIFE as u16);
            let crew = ship.descriptor_mut().ship_info.crew_level;
            ship.element_mut().set_hit_points(crew);
            let mass = ship.descriptor_mut().characteristics.ship_mass;
            ship.element_mut().set_mass_points(mass);
            ship.element_mut().current.location = POINT { x: 4000, y: 4000 };
            ship.element_mut().current.image.frames = ship.ship_frames();
            ship.element_mut().current.image.frame = frames[0].selection(0);
            ship.element_mut().next = ship.element_mut().current.clone();
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
        // SAFETY: this fixture retains the registered frame throughout its bounds update.
        unsafe {
            (*self
                .raw_frame(role, index)
                .cast::<uqm_rust::graphics::native_abi::frame_desc>())
            .Bounds = uqm_rust::graphics::native_abi::EXTENT {
                width: bounds[0],
                height: bounds[1],
            };
        }
    }
    pub fn weapon_handles(&mut self, expected: COUNT) -> Vec<HLINK> {
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
        subject: *mut ElementRecord,
        concerns: &mut [EVALUATE_DESC; 4],
        count: u16,
    ) {
        // SAFETY: callers supply this fixture's live subject, without retaining a record borrow.
        unsafe {
            let id = uqm_rust::app::bridge::enter(|cx| {
                cx.elements()
                    .id(subject.cast())
                    .unwrap_or_else(|| panic!("fixture subject is absent"))
            });
            let mut value = uqm_rust::app::bridge::enter(|cx| cx.weapon_origin(id));
            invoke_intelligence(self.ship_id, &mut value, concerns, count);
            *subject = value.into();
        }
    }
    pub fn set_hot_spot(&mut self, role: usize, index: usize, hot_spot: [i16; 2]) {
        // SAFETY: this fixture retains the registered frame throughout its hotspot update.
        unsafe {
            (*self
                .raw_frame(role, index)
                .cast::<uqm_rust::graphics::native_abi::frame_desc>())
            .HotSpot = uqm_rust::graphics::native_abi::POINT {
                x: hot_spot[0],
                y: hot_spot[1],
            };
        }
    }
    /// # Safety
    /// The source must be a live element in this fixture's retained battle pool.
    pub unsafe fn initialize_from(&self, source: *mut ElementRecord, expected: u16) -> Vec<HLINK> {
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
    pub fn frame(
        &self,
        role: usize,
        index: u16,
    ) -> uqm_rust::engine::battle::views::FrameSelection {
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
            put(self.handle);
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
    pub unsafe fn element_mut(&mut self) -> &mut ElementRecord {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.assert_live();
            self.el.get_mut()
        }
    }
    /// # Safety
    /// Do not retain record references across any engine or fixture hook call.
    pub unsafe fn ship_mut(&mut self) -> &mut ShipRecord {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.assert_live();
            self.ss.get_mut()
        }
    }
    /// # Safety
    /// Do not retain descriptor references across any engine or fixture hook call.
    pub unsafe fn descriptor_mut(&mut self) -> &mut RaceDescriptor {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.

        unsafe {
            self.assert_live();
            self.ss
                .get_mut()
                .descriptor
                .as_deref_mut()
                .unwrap_or_else(|| panic!("fixture owns its loaded descriptor"))
        }
    }
    pub fn snapshot(&self) -> uqm_rust::engine::battle::views::ElementState {
        // SAFETY: the fixture retains its battle and checked element through this value projection.
        unsafe { uqm_rust::app::bridge::enter(|cx| cx.element(self.id)) }
    }
    pub fn ptr(&self) -> *mut ElementRecord {
        self.el.0
    }
    pub fn el(&mut self) -> *mut ElementRecord {
        self.ptr()
    }
    fn descriptor(&self) -> &RaceDescriptor {
        // SAFETY: the serialized fixture retains its owned ship slot.
        unsafe {
            self.ss
                .get()
                .descriptor
                .as_deref()
                .unwrap_or_else(|| panic!("fixture owns its loaded descriptor"))
        }
    }
    pub fn ship_frames(&self) -> uqm_rust::engine::battle::views::Frames {
        self.frame_set(0)
    }
    pub fn special_frames(&self) -> uqm_rust::engine::battle::views::Frames {
        self.frame_set(2)
    }
    pub unsafe fn guardian_form(mut self) -> Self {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.element_mut().next.image.frames = self.ship_frames();
            self
        }
    }
    pub unsafe fn blazer_form(mut self) -> Self {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            self.element_mut().next.image.frames = self.special_frames();
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
                    cx.copy_visual_to_next(id, current.clone());
                    cx.set_intersect_frame(id, current.image.frame);
                }
                out
            })
        }
    }
    /// # Safety
    /// Do not retain the legacy pointer across callbacks or disposal.
    pub unsafe fn weapon_record(&self, id: Option<ElementId>) -> *mut ElementRecord {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                cx.legacy_element(id.unwrap_or_else(|| panic!("{}", "allocated weapon")))
            })
        }
    }
    pub unsafe fn intelligence(
        &mut self,
        subject: &mut ElementRecord,
        concerns: &mut [EVALUATE_DESC],
        count: u16,
    ) {
        // SAFETY: subject is a live fixture slot and its concerns remain live during dispatch.
        unsafe {
            let id = uqm_rust::app::bridge::enter(|cx| {
                cx.elements()
                    .id((subject as *mut ElementRecord).cast())
                    .unwrap_or_else(|| panic!("fixture subject is absent"))
            });
            let mut value = uqm_rust::app::bridge::enter(|cx| cx.weapon_origin(id));
            invoke_intelligence(self.ship_id, &mut value, concerns, count);
            *subject = value.into();
        }
    }
    pub fn subject(&self) -> uqm_rust::engine::battle::Subject {
        self.assert_live();
        // SAFETY: this fixture retains its live slot and all resources through projection.
        unsafe { uqm_rust::app::bridge::enter(|cx| cx.weapon_origin(self.id)) }
    }
    pub unsafe fn intelligence_subject(
        &mut self,
        subject: &mut uqm_rust::engine::battle::Subject,
        concerns: &mut [EVALUATE_DESC],
        count: u16,
    ) {
        // SAFETY: the fixture retains the value's resources and concern operands during dispatch.
        unsafe {
            invoke_intelligence(self.ship_id, subject, concerns, count);
        }
    }
    pub fn uninit(&mut self) {
        // The replacement path can consume the descriptor before fixture teardown.
        // SAFETY: the fixture still owns its live ship slot.
        let race = unsafe { self.ss.get().descriptor.as_deref().and_then(|d| d.race) };
        if let Some(race) = race {
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
    pub fn saved_collision(&self) -> Option<uqm_rust::engine::battle::CollisionHook> {
        let RaceState::Androsynth { collision } = self.descriptor().race_state else {
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
            uqm_rust::app::bridge::battle(|_| {
                let mut ships = (*uqm_rust::engine::battle::record::Kernel::current())
                    .ships(self.ship_id.side() as usize);
                let link = ships.link(self.ship_id);
                if ships.linked(link) {
                    ships.remove(link);
                }
                ships.free(link);
            });
        }
    }
}
pub fn descriptor(species: u32) -> RaceDescriptor {
    // SAFETY: the caller owns the serialized test context.
    unsafe { uqm_rust::ships::races::descriptor(species).into_legacy() }
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
pub unsafe fn id(pointer: *mut ElementRecord) -> ElementId {
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            cx.elements()
                .id(pointer.cast())
                .unwrap_or_else(|| panic!("{}", "pool record"))
        })
    }
}

#[derive(Clone)]
pub struct FrameSet(Rc<NativeFrameSet>);
impl FrameSet {
    pub fn new(count: u16) -> Self {
        Self::with_bounds(count, [4, 4])
    }
    pub fn with_bounds(count: u16, bounds: [i16; 2]) -> Self {
        let native = NativeFrameSet::new(count);
        // SAFETY: the fixture owns all frame slots in this registered drawable.
        unsafe {
            for index in 0..count {
                (*native
                    .frame(index as usize)
                    .cast::<uqm_rust::graphics::native_abi::frame_desc>())
                .Bounds = uqm_rust::graphics::native_abi::EXTENT {
                    width: bounds[0],
                    height: bounds[1],
                };
            }
        }
        Self(Rc::new(native))
    }
    pub fn selection(&self, index: usize) -> uqm_rust::engine::battle::frames::FrameSelection {
        self.0.selection(index)
    }

    pub fn array(&self) -> uqm_rust::engine::battle::frames::FrameArray {
        self.0.array()
    }
    pub fn make_opaque(&mut self) {
        use uqm_rust::graphics::{
            native_canvas::{
                lifecycle::TFB_DrawCanvas_New_Paletted, pixels::TFB_DrawCanvas_SetPixelIndexes,
            },
            tfb_draw_abi::{TFB_DrawImage_Delete, TFB_DrawImage_New},
        };
        // SAFETY: each registered frame owns a separate canvas and image; allocation leases retain them.
        unsafe {
            for index in 0..=(*self.0 .0).MaxIndex {
                let frame = &mut *(*self.0 .0).Frame.add(index as usize);
                let canvas = TFB_DrawCanvas_New_Paletted(4, 4, ptr::null_mut(), -1);
                assert!(!canvas.is_null());
                assert_ne!(
                    TFB_DrawCanvas_SetPixelIndexes(canvas, [1u8; 16].as_ptr(), 4, 4),
                    0
                );
                if !frame.image.is_null() {
                    TFB_DrawImage_Delete(frame.image);
                }
                frame.image = TFB_DrawImage_New(canvas);
                assert!(!frame.image.is_null());
            }
        }
    }
    pub fn frame(&self, index: usize) -> FRAME {
        self.0.frame(index)
    }
}
pub unsafe fn prim(e: *mut ElementRecord) -> *mut PRIMITIVE {
    // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| std::ptr::from_mut(cx.display_primitive((*e).PrimIndex)))
    }
}
pub unsafe fn queued() -> Vec<HLINK> {
    // SAFETY: the serialized fixture retains the published context throughout observation.
    unsafe { uqm_rust::app::bridge::enter(|cx| queued_in(cx)) }
}
pub fn queued_in(cx: &uqm_rust::engine::battle::Cx<'_>) -> Vec<HLINK> {
    let mut all = Vec::new();
    let mut next = cx.elements().head();
    while let Some(id) = next {
        all.push(cx.elements().raw(id));
        next = cx.elements().succ(id);
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
    el: Record<ElementRecord>,
    pub id: ElementId,
}
impl<'a> Object<'a> {
    pub fn new(battle: &'a Battle) -> Self {
        // SAFETY: object slots belong to the serialized test battle.
        unsafe {
            let handle = uqm_rust::app::bridge::enter(|cx| {
                cx.alloc_element()
                    .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
            });
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
    pub unsafe fn element_mut(&mut self) -> &mut ElementRecord {
        // SAFETY: The caller retains the serialized battle and all operand records through this synchronous operation.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                assert!(cx.elements().contains(self.id), "retired fixture object")
            });
            self.el.get_mut()
        }
    }
    pub fn ptr(&mut self) -> *mut ElementRecord {
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
    pointer: *mut ElementRecord,
    id: ElementId,
}
impl PoolElement {
    /// # Safety
    /// The serialized fixture must retain its battle pools until this owner drops.
    pub unsafe fn new() -> Self {
        // SAFETY: the caller retains the initialized, serialized element pool.
        unsafe {
            let pointer = uqm_rust::app::bridge::enter(|cx| {
                cx.alloc_element()
                    .map_or(ptr::null_mut(), |id| cx.elements().raw(id))
            })
            .cast::<ElementRecord>();
            assert!(!pointer.is_null(), "fixture element pool exhausted");
            let id = id(pointer);
            Self { pointer, id }
        }
    }
}
impl std::ops::Deref for PoolElement {
    type Target = ElementRecord;
    fn deref(&self) -> &ElementRecord {
        // SAFETY: this owner retains the allocated operand slot.
        unsafe { &*self.pointer }
    }
}
impl std::ops::DerefMut for PoolElement {
    fn deref_mut(&mut self) -> &mut ElementRecord {
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

/// # Safety
/// The supplied pointer is a retained fixture record owned by the active battle.
pub unsafe fn put(handle: *mut std::ffi::c_void) {
    // SAFETY: the fixture owns the published context and this allocated slot.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let id = cx
                .elements()
                .id(handle)
                .unwrap_or_else(|| panic!("fixture element"));
            cx.put(id);
        });
    }
}
/// Cast a retained fixture handle without dereferencing it.
pub fn element(
    handle: *mut std::ffi::c_void,
) -> *mut uqm_rust::engine::battle::record::ElementRecord {
    handle.cast()
}

pub struct NativeFrameSet(*mut uqm_rust::graphics::native_abi::drawable_desc);
impl NativeFrameSet {
    pub fn new(count: u16) -> Self {
        // SAFETY: the serialized fixture owns this drawable and all of its frame slots.
        unsafe {
            let drawable = uqm_rust::graphics::drawable_abi::AllocDrawable(count);
            assert!(!drawable.is_null());
            (*drawable).MaxIndex = count - 1;
            for index in 0..count {
                let frame = &mut *(*drawable).Frame.add(usize::from(index));
                frame.Index = index;
                frame.Bounds = uqm_rust::graphics::native_abi::EXTENT {
                    width: 4,
                    height: 4,
                };
            }
            Self(drawable)
        }
    }
    pub fn frame(&self, index: usize) -> FRAME {
        // SAFETY: the fixture retains the allocation throughout the returned handle's use.
        unsafe {
            assert!(index <= usize::from((*self.0).MaxIndex));
            (*self.0).Frame.add(index).cast()
        }
    }
    pub fn selection(&self, index: usize) -> uqm_rust::engine::battle::frames::FrameSelection {
        uqm_rust::engine::battle::frames::FrameSelection::retain(self.frame(index).cast())
    }

    pub fn array(&self) -> uqm_rust::engine::battle::frames::FrameArray {
        // SAFETY: these leases retain the drawable after the fixture's logical release.
        unsafe {
            let frame = self.frame(0);
            uqm_rust::engine::battle::frames::FrameArray::from_native([frame.cast(); 3])
        }
    }
}
impl Drop for NativeFrameSet {
    fn drop(&mut self) {
        // SAFETY: this fixture uniquely owns the drawable's logical release.
        unsafe {
            uqm_rust::graphics::drawable_abi::DestroyDrawable(self.0);
        }
    }
}
