#![allow(
    dead_code,
    reason = "shared fixture API used by independent race test binaries"
)]
//! Battle fixtures addressed by element and ship ids.
//!
//! `get` copies a record out of the fixture, so reads never hold a borrow.
//! `get_mut`, `prim_mut` and the `Ship`/`Object` record accessors lend the live
//! record until the returned guard drops; holding one across an engine call
//! panics with "fixture context is already lent".
use std::cell::RefMut;
use std::rc::Rc;
use uqm_engine::battle::flags::NORMAL_LIFE;
use uqm_engine::battle::flags::PLAYER_SHIP;
use uqm_engine::battle::record::ElementRecord;
use uqm_engine::battle::record::RaceDescriptor;
use uqm_engine::battle::record::ShipRecord;
use uqm_engine::battle::Concern;
use uqm_engine::battle::DisplayPrimitive;
use uqm_engine::battle::STAMP_PRIMITIVE;
use uqm_engine::battle::{ElementId, RaceState, ShipId};
use uqm_engine::graphics::drawable::FrameId;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::random;
use uqm_engine::math::seed_random;
use uqm_engine::math::velocity::VelocityDesc;
use uqm_engine::species::species_id::SPECIES_ID_VUX_ID;
use uqm_modes::battle::process_native::init_display_list;
use uqm_modes::battle::AiOps;

/// A copy of a fixture record. It cannot be written, so a write meant for the
/// live record fails to compile instead of being lost.
pub struct Snapshot<T>(T);
impl<T> std::ops::Deref for Snapshot<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

/// Copy a live element record.
#[track_caller]
pub fn get(id: ElementId) -> Snapshot<ElementRecord> {
    Snapshot(get_mut(id).clone())
}

/// Lend a live element record until the guard drops.
#[track_caller]
pub fn get_mut(id: ElementId) -> RefMut<'static, ElementRecord> {
    RefMut::map(crate::battle_context::kernel(), |kernel| {
        kernel.fixture_elements().fixture_record_mut(id)
    })
}

/// Copy an element's display primitive.
#[track_caller]
pub fn prim(id: ElementId) -> Snapshot<DisplayPrimitive> {
    Snapshot(prim_mut(id).clone())
}

/// Lend an element's display primitive until the guard drops.
#[track_caller]
pub fn prim_mut(id: ElementId) -> RefMut<'static, DisplayPrimitive> {
    RefMut::map(crate::battle_context::kernel(), |kernel| {
        let index = kernel.fixture_elements().fixture_record_mut(id).prim_index;
        kernel.display.get_mut(index)
    })
}

#[track_caller]
fn ship_record(id: ShipId) -> RefMut<'static, ShipRecord> {
    RefMut::map(crate::battle_context::kernel(), |kernel| {
        kernel.fixture_ship_mut(id)
    })
}

pub fn plot_intercept(a: ElementId, b: ElementId, turns: u16, margin: u16) -> u16 {
    crate::battle_context::with(|cx| {
        cx.plot_intercept(&cx.weapon_origin(a), &cx.weapon_origin(b), turns, margin)
    })
}

pub fn head() -> Option<ElementId> {
    crate::battle_context::with(|cx| cx.elements().head())
}

pub fn tail() -> Option<ElementId> {
    crate::battle_context::with(|cx| cx.elements().tail())
}

pub fn succ(id: ElementId) -> Option<ElementId> {
    crate::battle_context::with(|cx| cx.elements().succ(id))
}

pub fn pred(id: ElementId) -> Option<ElementId> {
    crate::battle_context::with(|cx| cx.elements().pred(id))
}

pub struct Battle {
    graphics: u8,
}
impl Battle {
    pub fn new(capacity: u16) -> Self {
        let graphics = uqm_engine::test_support::native_probe::graphics_status();
        uqm_engine::test_support::native_probe::set_graphics_status(graphics & !4);
        crate::battle_context::with(|cx| {
            cx.kernel_mut()
                .reset_element_capacity(usize::from(capacity));
        });
        crate::battle_context::with(|cx| {
            for side in 0..2 {
                cx.kernel_mut().reset_ship_capacity(side, 32);
            }
        });
        crate::battle_context::with(|cx| {
            let scaled = cx.kernel_mut().configuration.preferences().melee_scale == 0;
            init_display_list(cx, scaled);
        });
        Self { graphics }
    }
}
impl Drop for Battle {
    fn drop(&mut self) {
        crate::battle_context::with(|cx| cx.kernel_mut().reset_element_capacity(0));
        crate::battle_context::with(|cx| {
            for side in 0..2 {
                cx.kernel_mut().reset_ship_capacity(side, 0);
            }
        });
        uqm_engine::test_support::native_probe::set_graphics_status(self.graphics);
    }
}

fn alloc_element() -> ElementId {
    crate::battle_context::with(|cx| cx.alloc_element())
        .unwrap_or_else(|| panic!("fixture element pool exhausted"))
}

fn release_element(id: ElementId) {
    crate::battle_context::with(|cx| {
        if cx.elements().contains(id) {
            if cx.elements().linked(id) {
                uqm_modes::battle::sounds::engine::unlink(cx, id);
            }
            cx.dispose(id);
        }
    });
}

pub struct Ship<'a> {
    battle: &'a Battle,
    pub id: ElementId,
    pub ship_id: ShipId,
    frames: Option<[FrameSet; 3]>,
    queued: bool,
}
impl<'a> Ship<'a> {
    pub fn bare(battle: &'a Battle, species: u32, player: i16) -> Self {
        let descriptor = Box::new(descriptor(species));
        let ship_id = crate::battle_context::with(|cx| {
            let mut ships = cx.kernel.ships(player as usize);
            let link = ships
                .alloc()
                .unwrap_or_else(|| panic!("fixture ship pool exhausted"));
            ships.ship_id(link)
        });
        {
            let mut ship = ship_record(ship_id);
            ship.descriptor = Some(descriptor);
            ship.player_nr = player;
            ship.species_id = species;
        }
        let id = alloc_element();
        {
            let mut element = get_mut(id);
            element.player_nr = player;
            element.state_flags = PLAYER_SHIP as u16;
            element.parent = uqm_engine::battle::Parent::Ship(ship_id);
        }
        ship_record(ship_id).ship_element = Some(id);
        Self {
            battle,
            id,
            ship_id,
            frames: None,
            queued: false,
        }
    }
    pub fn native(
        battle: &'a Battle,
        race: &'static uqm_engine::battle::Race,
        player: i16,
    ) -> Self {
        let mut ship = Self::bare(battle, SPECIES_ID_VUX_ID, player);
        let descriptor =
            crate::battle_context::with(|cx| (race.descriptor)(cx.world).into_legacy());
        *ship.descriptor_mut() = descriptor;
        ship
    }
    pub fn new(battle: &'a Battle, species: u32, player: i16) -> Self {
        Self::with_frames(battle, species, player, [16; 3], [4, 4])
    }
    pub fn with_frames(
        battle: &'a Battle,
        species: u32,
        player: i16,
        counts: [u16; 3],
        bounds: [i16; 2],
    ) -> Self {
        let mut ship = Self::bare(battle, species, player);
        let frames = counts.map(|count| FrameSet::with_bounds(count, bounds));
        let (crew, mass) = {
            let mut descriptor = ship.descriptor_mut();
            descriptor.ship_data.ship = frames[0].array();
            descriptor.ship_data.weapon = frames[1].array();
            descriptor.ship_data.special = frames[2].array();
            (
                descriptor.ship_info.crew_level,
                descriptor.characteristics.ship_mass,
            )
        };
        let ship_frames = ship.ship_frames();
        {
            let mut element = ship.element_mut();
            element.collision_func = Some(uqm_engine::battle::CollisionHook::Native(
                uqm_engine::battle::motion::collision,
            ));
            element.set_life_span(NORMAL_LIFE as u16);
            element.set_hit_points(crew);
            element.set_mass_points(mass);
            element.current.location = Point { x: 4000, y: 4000 };
            element.current.image.frames = ship_frames;
            element.current.image.frame = frames[0].selection(0);
            element.next = element.current.clone();
            element.intersect_control.intersect_stamp.frame = frames[0].frame(0);
        }
        prim_mut(ship.id).kind = STAMP_PRIMITIVE;
        ship.frames = Some(frames);
        ship.queue();
        ship
    }
    pub fn raw_frame(&self, role: usize, index: usize) -> FrameId {
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
        self.raw_frame(role, index)
            .set_bounds(uqm_engine::graphics::geometry::Extent::new(
                bounds[0], bounds[1],
            ));
    }
    /// Spawn this ship's weapons from its own element without adopting them.
    pub fn weapon_handles(&mut self, expected: u16) -> Vec<Option<ElementId>> {
        self.initialize_from(self.id, expected)
    }
    /// Run the ship's AI on `subject` and write the evaluated subject back to it.
    pub fn think(&mut self, subject: ElementId, concerns: &mut [Concern], count: u16) {
        let mut value = crate::battle_context::with(|cx| cx.weapon_origin(subject));
        self.intelligence_subject(&mut value, concerns, count);
        *get_mut(subject) = value.into();
    }
    pub fn set_hot_spot(&mut self, role: usize, index: usize, hot_spot: [i16; 2]) {
        self.raw_frame(role, index)
            .set_hot_spot(uqm_engine::graphics::geometry::Point {
                x: hot_spot[0],
                y: hot_spot[1],
            });
    }
    /// Spawn this ship's weapons from `source` without adopting them.
    pub fn initialize_from(&self, source: ElementId, expected: u16) -> Vec<Option<ElementId>> {
        crate::battle_context::with(|cx| {
            let origin = cx.weapon_origin(source);
            let out = cx.spawn_weapons(self.ship_id, origin);
            assert_eq!(out.len(), usize::from(expected));
            out
        })
    }
    pub fn frame(&self, role: usize, index: u16) -> uqm_engine::battle::views::FrameSelection {
        self.assert_live();
        crate::battle_context::with(|cx| {
            cx.absolute_frame(cx.ship_frame(self.ship_id, role), index)
        })
    }
    pub fn frame_set(&self, role: usize) -> uqm_engine::battle::views::Frames {
        self.assert_live();
        crate::battle_context::with(|cx| cx.ship_frame_set(self.ship_id, role))
    }
    pub fn queue(&mut self) {
        assert!(!self.queued);
        put(self.id);
        self.queued = true;
    }
    #[track_caller]
    fn assert_live(&self) {
        crate::battle_context::with(|cx| {
            assert!(cx.elements().contains(self.id), "retired fixture record")
        });
    }
    /// Lend the ship's element record until the guard drops.
    #[track_caller]
    pub fn element_mut(&mut self) -> RefMut<'static, ElementRecord> {
        self.assert_live();
        get_mut(self.id)
    }
    /// Lend the ship's pool record until the guard drops.
    #[track_caller]
    pub fn ship_mut(&mut self) -> RefMut<'static, ShipRecord> {
        self.assert_live();
        ship_record(self.ship_id)
    }
    /// Lend the ship's loaded descriptor until the guard drops.
    #[track_caller]
    pub fn descriptor_mut(&mut self) -> RefMut<'static, RaceDescriptor> {
        self.assert_live();
        RefMut::map(ship_record(self.ship_id), |ship| {
            ship.descriptor
                .as_deref_mut()
                .unwrap_or_else(|| panic!("fixture owns its loaded descriptor"))
        })
    }
    pub fn snapshot(&self) -> uqm_engine::battle::views::ElementState {
        crate::battle_context::with(|cx| cx.element(self.id))
    }
    pub fn ship_frames(&self) -> uqm_engine::battle::views::Frames {
        self.frame_set(0)
    }
    pub fn special_frames(&self) -> uqm_engine::battle::views::Frames {
        self.frame_set(2)
    }
    pub fn guardian_form(mut self) -> Self {
        let frames = self.ship_frames();
        self.element_mut().next.image.frames = frames;
        self
    }
    pub fn blazer_form(mut self) -> Self {
        let frames = self.special_frames();
        self.element_mut().next.image.frames = frames;
        self
    }
    pub fn preprocess(&mut self) {
        self.process(false);
    }
    pub fn postprocess(&mut self) {
        self.process(true);
    }
    fn process(&mut self, post: bool) {
        crate::battle_context::with(|cx| {
            uqm_engine::battle::Cx::invoke_ship_process(cx, self.ship_id, post);
        });
    }
    /// Spawn this ship's weapons and adopt them as its own.
    pub fn weapon(&mut self, expected: u16) -> Vec<Option<ElementId>> {
        crate::battle_context::with(|cx| {
            let origin = cx.weapon_origin(self.id);
            let out = uqm_engine::battle::Cx::spawn_weapons(cx, self.ship_id, origin);
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
    pub fn subject(&self) -> uqm_engine::battle::Subject {
        self.assert_live();
        crate::battle_context::with(|cx| cx.weapon_origin(self.id))
    }
    pub fn intelligence_subject(
        &mut self,
        subject: &mut uqm_engine::battle::Subject,
        concerns: &mut [Concern],
        count: u16,
    ) {
        crate::battle_context::with(|cx| {
            cx.invoke_intelligence(self.ship_id, subject, concerns, count);
        });
    }
    pub fn uninit(&mut self) {
        // The replacement path can consume the descriptor before fixture teardown.
        let race = ship_record(self.ship_id)
            .descriptor
            .as_deref()
            .and_then(|descriptor| descriptor.race);
        if let Some(uninit) = race.and_then(|race| race.uninit) {
            crate::battle_context::with(|cx| {
                let mut stats = cx.race_stats(self.ship_id);
                uninit(cx.world, &mut stats);
                *cx.race_stats_mut(self.ship_id) = stats;
            });
        }
    }
    pub fn saved_collision(&self) -> Option<uqm_engine::battle::CollisionHook> {
        let ship = ship_record(self.ship_id);
        let descriptor = ship
            .descriptor
            .as_deref()
            .unwrap_or_else(|| panic!("fixture owns its loaded descriptor"));
        let RaceState::Androsynth { collision } = descriptor.race_state else {
            panic!("no saved Androsynth collision")
        };
        collision
    }
}
impl Drop for Ship<'_> {
    fn drop(&mut self) {
        self.uninit();
        release_element(self.id);
        crate::battle_context::with(|cx| {
            let mut ships = cx.kernel.ships(self.ship_id.side() as usize);
            let link = ships.link(self.ship_id);
            if ships.linked(link) {
                ships.remove(link);
            }
            ships.free(link);
        });
    }
}
pub fn descriptor(species: u32) -> RaceDescriptor {
    crate::battle_context::with(|cx| {
        uqm_content::ships::races::descriptor(cx.world, species).into_legacy()
    })
}

pub fn call_element(name: &str, id: ElementId) {
    let address = uqm_modes::battle::hook_registry::hooks()
        .into_iter()
        .find_map(|(address, n)| (n == name).then_some(address))
        .unwrap_or_else(|| panic!("{}", "registered element hook"));
    crate::battle_context::with(|cx| {
        let e = &cx.elements()[id];
        let callback = [e.preprocess_func, e.postprocess_func, e.death_func]
            .into_iter()
            .flatten()
            .find(|h| h.address() == address)
            .unwrap_or_else(|| panic!("{}", "named hook installed on element"));
        uqm_modes::battle::process_native::invoke_element::invoke_element(cx, callback, id);
    });
}

pub fn call_collision(name: &str, a: ElementId, pa: &mut Point, b: ElementId, pb: &mut Point) {
    let address = uqm_modes::battle::hook_registry::hooks()
        .into_iter()
        .find_map(|(address, n)| (n == name).then_some(address))
        .unwrap_or_else(|| panic!("{}", "registered collision hook"));
    crate::battle_context::with(|cx| {
        let hook = cx
            .element(a)
            .collide
            .unwrap_or_else(|| panic!("{}", "installed collision hook"));
        assert_eq!(
            hook.address(),
            address,
            "named collision installed on element"
        );
        uqm_modes::battle::process_native::invoke_collision::invoke_collision(
            cx, hook, a, pa, b, pb,
        );
    })
}

#[derive(Clone)]
pub struct FrameSet(Rc<NativeFrameSet>);
impl FrameSet {
    pub fn new(count: u16) -> Self {
        Self::with_bounds(count, [4, 4])
    }
    pub fn with_bounds(count: u16, bounds: [i16; 2]) -> Self {
        let native = NativeFrameSet::new(count);
        for index in 0..usize::from(count) {
            native
                .frame(index)
                .set_bounds(uqm_engine::graphics::geometry::Extent::new(
                    bounds[0], bounds[1],
                ));
        }
        Self(Rc::new(native))
    }
    pub fn selection(&self, index: usize) -> uqm_engine::battle::frames::FrameSelection {
        self.0.selection(index)
    }

    pub fn array(&self) -> uqm_engine::battle::frames::FrameArray {
        self.0.array()
    }
    pub fn make_opaque(&mut self) {
        for index in 0..self.0.count() {
            uqm_engine::test_support::native_probe::make_opaque(self.0.frame(index));
        }
    }
    pub fn frame(&self, index: usize) -> FrameId {
        self.0.frame(index)
    }
}
/// The display queue from head to tail.
pub fn queued() -> Vec<ElementId> {
    crate::battle_context::with(|cx| queued_in(cx))
}
pub fn queued_in(cx: &uqm_engine::battle::Cx<'_>) -> Vec<ElementId> {
    let mut all = Vec::new();
    let mut next = cx.elements().head();
    while let Some(id) = next {
        all.push(id);
        next = cx.elements().succ(id);
    }
    all
}

pub fn velocity(v: &VelocityDesc) -> [i32; 9] {
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
pub fn toward(speed: i16, face: u16) -> [i32; 9] {
    let mut v = uqm_modes::battle::velocity::VelocityDesc::new();
    uqm_modes::battle::velocity::set_velocity_vector(&mut v, speed, face);
    velocity(&v)
}
pub fn with_rng<T>(seed: u32, draws: usize, run: impl FnOnce() -> T) -> T {
    seed_random(seed);
    for _ in 0..draws {
        random();
    }
    let expected = random();
    seed_random(seed);
    let result = run();
    assert_eq!(random(), expected, "RNG draws {draws}, seed {seed}");
    result
}

/// An element allocated for the fixture; it is disposed when the owner drops.
pub struct Object<'a> {
    battle: &'a Battle,
    pub id: ElementId,
}
impl<'a> Object<'a> {
    pub fn new(battle: &'a Battle) -> Self {
        Self {
            battle,
            id: alloc_element(),
        }
    }
    /// Lend the object's element record until the guard drops.
    #[track_caller]
    pub fn element_mut(&mut self) -> RefMut<'static, ElementRecord> {
        get_mut(self.id)
    }
}
impl Drop for Object<'_> {
    fn drop(&mut self) {
        release_element(self.id);
    }
}

#[path = "ship_pool.rs"]
mod ship_pool;
pub type PoolShip = ship_pool::PoolShip;

/// An unlinked element slot owned outside any battle fixture value.
pub struct PoolElement {
    pub id: ElementId,
}
impl PoolElement {
    pub fn new() -> Self {
        Self {
            id: alloc_element(),
        }
    }
}
impl Drop for PoolElement {
    fn drop(&mut self) {
        release_element(self.id);
    }
}

pub fn put(id: ElementId) {
    crate::battle_context::with(|cx| cx.put(id));
}

pub struct NativeFrameSet(uqm_engine::graphics::drawable::LoadedDrawable);
impl NativeFrameSet {
    pub fn new(count: u16) -> Self {
        Self(
            uqm_engine::graphics::drawable::LoadedDrawable::create(0, 4, 4, count)
                .unwrap_or_else(|| panic!("fixture drawable")),
        )
    }
    pub fn count(&self) -> usize {
        usize::from(self.0.first_frame().count())
    }
    pub fn frame(&self, index: usize) -> FrameId {
        assert!(index < self.count());
        self.0.first_frame().at(index as u16)
    }
    pub fn selection(&self, index: usize) -> uqm_engine::battle::frames::FrameSelection {
        uqm_engine::battle::frames::FrameSelection::retain(self.frame(index))
    }

    pub fn array(&self) -> uqm_engine::battle::frames::FrameArray {
        let frame = self.frame(0);
        uqm_engine::battle::frames::FrameArray::from_native([frame; 3])
    }
}
