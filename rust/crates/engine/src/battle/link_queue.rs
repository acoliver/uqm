//! Typed link-order views over fixed-capacity owned pools.

use super::super::super::battle::ShipId;
use crate::collections::pool::{Id, OrderedPool};
pub const NAME_OFFSET: u32 = 5;
pub const NUM_CAPTAINS_NAMES: u32 = 16;
pub struct RacePool {
    pub(super) pool: OrderedPool<super::super::super::battle::record::ShipRecord, 32>,
    side: u8,
}
impl RacePool {
    pub fn with_capacity(side: u8, capacity: usize) -> Self {
        assert!(side < 2, "invalid ship side {side}");
        Self {
            pool: OrderedPool::with_capacity(capacity),
            side,
        }
    }
    pub fn reset_capacity(&mut self, capacity: usize) {
        self.pool.reset_capacity(capacity);
    }
    pub fn reinit(&mut self) {
        self.pool.reinit();
    }
}

/// Record mapping shared by the battle and campaign queue views.
pub(crate) mod queue_record {
    pub trait Sealed {}
    impl Sealed for super::Starship {}
}
pub trait QueueRecord: queue_record::Sealed + Eq {
    type Record;
}
/// Campaign records allow whole-record mutation; battle ships use projections.
pub trait MutableQueueRecord: QueueRecord {}

impl QueueRecord for Starship {
    type Record = super::super::super::battle::record::ShipRecord;
}

use std::marker::PhantomData;

#[derive(Debug, PartialEq, Eq)]
pub enum Starship {}

#[derive(Debug, PartialEq, Eq)]
pub struct LinkId<T> {
    slot: usize,
    pool: u64,
    generation: u32,
    marker: PhantomData<T>,
}
impl<T> Copy for LinkId<T> {}
impl<T> Clone for LinkId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct LinkQueue<'a, T: QueueRecord, const N: usize = 32> {
    queue: QueueBorrow<'a, T::Record, N>,
    side: u8,
    marker: PhantomData<T>,
}

enum QueueBorrow<'a, T, const N: usize> {
    Shared(&'a OrderedPool<T, N>),
    Exclusive(&'a mut OrderedPool<T, N>),
}
impl<T, const N: usize> std::ops::Deref for QueueBorrow<'_, T, N> {
    type Target = OrderedPool<T, N>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Shared(q) => q,
            Self::Exclusive(q) => q,
        }
    }
}
impl<T, const N: usize> std::ops::DerefMut for QueueBorrow<'_, T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Exclusive(q) => q,
            Self::Shared(_) => panic!("cannot mutate a read-only queue view"),
        }
    }
}

/// Picker and fleet fields, independent of descriptor and element lifetimes.
#[derive(Clone, Copy, Debug)]
pub struct PickShip {
    pub species: u32,
    pub captain: u8,
    pub crew: u16,
    pub max_crew: u16,
    pub cost: u8,
    pub index: u16,
    pub energy_counter: u8,
    pub player: i16,
    pub control: u8,
}

impl<'a, T: QueueRecord, const N: usize> LinkQueue<'a, T, N> {
    fn identify(&self, id: Id<T::Record>) -> LinkId<T> {
        LinkId {
            slot: id.index(),
            generation: id.generation(),
            pool: self.queue.storage_identity(),
            marker: PhantomData,
        }
    }
    pub(super) fn key(&self, id: LinkId<T>) -> Id<T::Record> {
        assert_eq!(
            id.pool,
            self.queue.storage_identity(),
            "ship link belongs to another pool"
        );
        let key = self
            .queue
            .id_at(id.slot)
            .unwrap_or_else(|| panic!("ship link addresses free storage"));
        assert_eq!(key.generation(), id.generation, "stale ship link");
        key
    }
    pub fn contains(&self, id: LinkId<T>) -> bool {
        id.pool == self.queue.storage_identity()
            && self
                .queue
                .id_at(id.slot)
                .is_some_and(|key| key.generation() == id.generation)
    }
    pub fn id(&self, address: usize) -> Option<LinkId<T>> {
        if address == 0 {
            return None;
        }
        let id = (0..N).find_map(|slot| {
            let id = self.queue.id_at(slot)?;
            (std::ptr::from_ref(self.queue.get(id)) as usize == address).then(|| self.identify(id))
        });
        assert!(id.is_some(), "ship link addresses free or foreign storage");
        id
    }
    pub fn slot(&self, id: LinkId<T>) -> u8 {
        self.key(id).index() as u8
    }
    /// Retired parents retain their journal identity, without reading a freed record.
    pub fn journal_address(&self, id: LinkId<T>) -> usize {
        assert_eq!(
            id.pool,
            self.queue.storage_identity(),
            "foreign journal identity"
        );
        self.queue.retired_address(id.slot)
    }
    pub fn into_record(self, id: LinkId<T>) -> &'a T::Record {
        let key = self.key(id);
        match self.queue {
            QueueBorrow::Shared(queue) => queue.get(key),
            QueueBorrow::Exclusive(queue) => queue.get(key),
        }
    }

    pub fn head(&self) -> Option<LinkId<T>> {
        self.queue.head().map(|id| self.identify(id))
    }
    pub fn tail(&self) -> Option<LinkId<T>> {
        self.queue.tail().map(|id| self.identify(id))
    }
    pub fn succ(&self, id: LinkId<T>) -> Option<LinkId<T>> {
        self.queue.succ(self.key(id)).map(|id| self.identify(id))
    }
    pub fn pred(&self, id: LinkId<T>) -> Option<LinkId<T>> {
        self.queue.pred(self.key(id)).map(|id| self.identify(id))
    }
    pub fn linked(&self, id: LinkId<T>) -> bool {
        self.queue.linked(self.key(id))
    }
    pub fn find(&self, mut matches: impl FnMut(&T::Record) -> bool) -> Option<LinkId<T>> {
        let mut next = self.head();
        while let Some(id) = next {
            if matches(self.record(id)) {
                return Some(id);
            }
            next = self.succ(id);
        }
        None
    }

    pub fn at(&self, mut index: usize) -> Option<LinkId<T>> {
        let mut next = self.head();
        while let Some(id) = next {
            if index == 0 {
                return Some(id);
            }
            index -= 1;
            next = self.succ(id);
        }
        None
    }
    pub fn count(&self) -> usize {
        self.queue.count()
    }
}

impl LinkQueue<'_, Starship> {
    pub fn captain_name(&self, species: u32, mut random: impl FnMut() -> u32) -> u8 {
        loop {
            let name =
                ((random() as u16 & (NUM_CAPTAINS_NAMES - 1) as u16) + NAME_OFFSET as u16) as u8;
            let mut next = self.head();
            let mut used = false;
            while let Some(id) = next {
                let ship = self.get(id);
                if ship.species == species && ship.captain == name {
                    used = true;
                    break;
                }
                next = self.succ(id);
            }
            if !used {
                return name;
            }
        }
    }
    pub fn by_index(&self, mut index: u16, used: bool) -> Option<ShipId> {
        let mut link = self.head();
        while let Some(id) = link {
            let ship = self.get(id);
            if used {
                if ship.species != 0 {
                    if index == 0 {
                        return Some(self.ship_id(id));
                    }
                    index = index.wrapping_sub(1);
                }
            } else if ship.index == index {
                return if ship.species == 0 {
                    None
                } else {
                    Some(self.ship_id(id))
                };
            }
            link = self.succ(id);
        }
        None
    }
    pub fn build(&mut self, species: u32) -> Option<LinkId<Starship>> {
        let id = self.alloc()?;
        self.record_mut(id).SpeciesID = species;
        self.put(id);
        Some(id)
    }
    pub fn get(&self, id: LinkId<Starship>) -> PickShip {
        let s = self.record(id);
        PickShip {
            species: s.SpeciesID,
            captain: s.captains_name_index,
            crew: s.crew_level,
            max_crew: s.max_crew,
            cost: s.ship_cost,
            index: s.index,
            energy_counter: s.energy_counter,
            player: s.playerNr,
            control: s.control,
        }
    }
    pub fn set(&mut self, id: LinkId<Starship>, value: PickShip) {
        let s = self.record_mut(id);
        s.SpeciesID = value.species;
        s.captains_name_index = value.captain;
        s.crew_level = value.crew;
        s.max_crew = value.max_crew;
        s.ship_cost = value.cost;
        s.index = value.index;
        s.energy_counter = value.energy_counter;
        s.playerNr = value.player;
        s.control = value.control;
    }
    pub fn edit(&mut self, id: LinkId<Starship>, edit: impl FnOnce(&mut PickShip)) {
        let mut value = self.get(id);
        edit(&mut value);
        self.set(id, value);
    }
    pub fn clear_descriptor(&mut self, id: LinkId<Starship>) {
        self.record_mut(id).descriptor = None;
    }
    pub fn ship_id(&self, id: LinkId<Starship>) -> ShipId {
        self.raw(id);
        ShipId {
            side: self.side,
            slot: id.slot as u8,
            gen: id.generation,
            owner: id.pool,
        }
    }
    pub fn link(&self, id: ShipId) -> LinkId<Starship> {
        assert_eq!(id.side, self.side, "ship belongs to another side");
        assert_eq!(
            id.owner,
            self.queue.storage_identity(),
            "ship belongs to another pool"
        );
        let key = self
            .queue
            .id_at(usize::from(id.slot))
            .unwrap_or_else(|| panic!("invalid ship {id:?}"));
        assert_eq!(id.gen, key.generation(), "stale ship {id:?}");
        self.identify(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_preserve_append_remove_reverse_order_and_exhaustion() {
        let mut pool = RacePool::with_capacity(0, 3);
        let mut ships = LinkQueue::<Starship>::new(&mut pool);
        assert_eq!(ships.count(), 0);
        let a = ships.alloc().unwrap();
        let b = ships.alloc().unwrap();
        let c = ships.alloc().unwrap();
        assert!(ships.alloc().is_none());
        ships.put(b);
        ships.put(a);
        ships.put(c);
        assert_eq!(ships.count(), 3);
        assert_eq!(ships.head(), Some(b));
        assert_eq!(ships.succ(b), Some(a));
        assert_eq!(ships.pred(c), Some(a));
        assert_eq!(ships.tail(), Some(c));
        ships.remove(a);
        assert_eq!(ships.succ(b), Some(c));
        assert_eq!(ships.pred(c), Some(b));
        assert_eq!(ships.count(), 2);
        ships.put(a);
        assert_eq!(ships.tail(), Some(a));
        assert_eq!(ships.id(ships.raw(a) as usize), Some(a));
        ships.reinit();
        assert_eq!(ships.count(), 0);
        assert!(ships.head().is_none());
        assert!(ships.tail().is_none());
    }
    #[test]
    fn build_sets_species_and_edits_picker_fields_without_touching_links() {
        let mut pool = RacePool::with_capacity(0, 2);
        let mut ships = LinkQueue::<Starship>::new(&mut pool);
        let a = ships.build(11).unwrap();
        let b = ships.build(2).unwrap();
        let mut ship = ships.get(a);
        assert_eq!(ship.species, 11);
        ship.index = 9;
        ship.crew = 7;
        ships.set(a, ship);
        assert_eq!(ships.get(a).index, 9);
        assert_eq!(ships.get(a).crew, 7);
        assert_eq!(ships.succ(a), Some(b));
        assert_eq!(ships.pred(b), Some(a));
    }
    #[test]
    fn invalid_lifecycle_operations_fail_before_touching_links() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let mut pool = RacePool::with_capacity(0, 2);
        let mut ships = LinkQueue::<Starship>::new(&mut pool);
        let id = ships.alloc().unwrap();
        assert!(catch_unwind(AssertUnwindSafe(|| ships.remove(id))).is_err());
        ships.put(id);
        assert!(catch_unwind(AssertUnwindSafe(|| ships.put(id))).is_err());
        assert_eq!(ships.count(), 1);
        ships.remove(id);
        let raw = ships.raw(id);
        ships.free(id);
        assert!(catch_unwind(AssertUnwindSafe(|| ships.get(id))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| ships.id(raw as usize))).is_err());
        assert!(ships.id(0).is_none());
        let replacement = ships.alloc().unwrap();
        assert_eq!(replacement.slot, id.slot);
        assert_ne!(replacement.generation, id.generation);
        assert!(!ships.contains(id));
        assert!(catch_unwind(AssertUnwindSafe(|| ships.raw(id))).is_err());
        assert_eq!(ships.get(replacement).species, 0);
        ships.clear_descriptor(replacement);
        let ship_id = ships.ship_id(replacement);
        assert_eq!(ships.link(ship_id), replacement);
        ships.reinit();
        assert!(catch_unwind(AssertUnwindSafe(|| ships.raw(replacement))).is_err());
    }
}

use std::ptr;
impl<'a, T: QueueRecord, const N: usize> LinkQueue<'a, T, N> {
    pub fn from_pool(queue: &'a mut OrderedPool<T::Record, N>) -> Self {
        Self {
            queue: QueueBorrow::Exclusive(queue),
            side: 0,
            marker: PhantomData,
        }
    }
    pub fn borrow_pool(queue: &'a OrderedPool<T::Record, N>) -> Self {
        Self {
            queue: QueueBorrow::Shared(queue),
            side: 0,
            marker: PhantomData,
        }
    }
    pub fn raw(&self, id: LinkId<T>) -> *mut std::ffi::c_void {
        ptr::from_ref(self.record(id)).cast_mut().cast()
    }
    pub fn record(&self, id: LinkId<T>) -> &T::Record {
        self.queue.get(self.key(id))
    }
    pub(super) fn record_mut(&mut self, id: LinkId<T>) -> &mut T::Record {
        let key = self.key(id);
        self.queue.get_mut(key)
    }
    pub fn alloc(&mut self) -> Option<LinkId<T>>
    where
        T::Record: Default,
    {
        let key = self.queue.alloc(T::Record::default())?;
        Some(self.identify(key))
    }

    pub fn insert_before(&mut self, id: LinkId<T>, before: Option<LinkId<T>>) {
        let key = self.key(id);
        let before = before.map(|id| self.key(id));
        self.queue.insert_before(key, before);
    }
    pub fn put(&mut self, id: LinkId<T>) {
        let key = self.key(id);
        self.queue.put(key);
    }
    pub fn remove(&mut self, id: LinkId<T>) {
        let key = self.key(id);
        self.queue.remove(key);
    }
    pub fn free(&mut self, id: LinkId<T>) {
        let key = self.key(id);
        self.queue.free(key);
    }
    pub fn reinit(&mut self) {
        self.queue.reinit();
    }
}

impl LinkQueue<'_, Starship> {
    pub fn facing(&self, id: LinkId<Starship>) -> u16 {
        self.record(id).ship_facing
    }
    pub fn element_handle(
        &self,
        id: LinkId<Starship>,
    ) -> Option<super::super::super::battle::ElementId> {
        self.record(id).hShip
    }
}

/// Picker resources retained independently of campaign fragment pointer fields.
#[derive(Clone, Debug)]
pub struct ShipPickerResources {
    pub(super) strings: Option<crate::strings::owned_table::TextEntry>,
    icon: crate::battle::frames::FrameSelection,
}
impl ShipPickerResources {
    pub fn new(
        strings: Option<crate::strings::owned_table::TextEntry>,
        icon: crate::battle::frames::FrameSelection,
    ) -> Self {
        Self { strings, icon }
    }
    pub fn label(&self, index: u16) -> (&std::ffi::CStr, u16) {
        self.strings
            .as_ref()
            .map_or((c"", 0), |table| table.label(index))
    }
    pub fn icon(&self) -> crate::battle::views::FrameRef<'_> {
        self.icon.view()
    }
}

/// Read-only resource projection borrowed from its owning ship queue.
pub struct PickerResources<'a> {
    owner: &'a ShipPickerResources,
}
impl PickerResources<'_> {
    pub fn strings(&self) -> crate::graphics::native_abi::STRING {
        self.owner
            .strings
            .as_ref()
            .map_or(std::ptr::null_mut(), |table| table.as_raw().cast())
    }
    pub fn label(&self, index: u16) -> (&std::ffi::CStr, u16) {
        self.owner.label(index)
    }
    pub fn icon(&self) -> crate::battle::views::FrameRef<'_> {
        self.owner.icon()
    }
}
impl LinkQueue<'_, Starship> {
    pub fn set_picker_resources(&mut self, id: LinkId<Starship>, resources: ShipPickerResources) {
        let ship = self.record_mut(id);
        ship.picker_resources = Some(resources);
    }
    pub fn resources(&self, id: LinkId<Starship>) -> PickerResources<'_> {
        PickerResources {
            owner: self
                .record(id)
                .picker_resources
                .as_ref()
                .unwrap_or_else(|| panic!("ship has no admitted picker resources")),
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl RacePool {
    pub fn fixture_pool(
        &mut self,
    ) -> &mut OrderedPool<super::super::super::battle::record::ShipRecord, 32> {
        &mut self.pool
    }
    pub fn fixture_records(
        &self,
    ) -> &OrderedPool<super::super::super::battle::record::ShipRecord, 32> {
        &self.pool
    }
}

#[cfg(test)]
mod picker_owner_tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn picker_labels_and_icons_remain_owned_after_the_admitting_view_drops() {
        use crate::battle::frames::FrameSelection;
        use crate::graphics::{
            context_abi::ContextFrame,
            drawable_abi::{AllocDrawable, DestroyDrawable},
            pixmap_abi::CaptureDrawable,
        };
        // SAFETY: this serialized fixture owns the registered allocation and initializes its frame.
        let (drawable, icon) = unsafe {
            let drawable = AllocDrawable(1);
            let icon = CaptureDrawable(drawable);
            (*icon).Bounds = crate::graphics::native_abi::EXTENT {
                width: 2,
                height: 3,
            };
            (drawable, icon)
        };
        let resources = ShipPickerResources::new(
            crate::strings::owned_table::LoadedTextTable::fixture(vec![
                c"captain".to_bytes_with_nul().to_vec(),
                Vec::new(),
            ])
            .entry(0),
            FrameSelection::retain(icon),
        );
        let mut world = crate::World::default();
        world.retain_fleet_resources(1, resources);
        let fragment = world.npc_built_ships().alloc().unwrap();
        world.npc_built_ships().get_mut(fragment).icons = std::ptr::dangling_mut();
        world.npc_built_ships().get_mut(fragment).race_strings = std::ptr::dangling_mut();
        let mut pool = RacePool::with_capacity(0, 1);
        let mut ships = LinkQueue::<Starship>::new(&mut pool);
        let id = ships.build(1).unwrap();
        ships.set_picker_resources(id, world.fleet_resources(1));
        world.clear_fleet_resources();
        // SAFETY: the fixture consumes its native release authority; the picker retains the allocation.
        unsafe { DestroyDrawable(drawable) };
        assert_eq!(ships.resources(id).label(2), (c"captain", 7));
        assert_eq!(ships.resources(id).label(1), (c"", 0));
        assert_eq!(ships.resources(id).icon().raw(), icon);
        let frame = ContextFrame::retain(std::ptr::NonNull::new(icon));
        assert_eq!(frame.rect().extent.width, 2);
        assert_eq!(frame.rect().extent.height, 3);
        drop(frame);
        ships.remove(id);
        ships.free(id);
        assert!(std::panic::catch_unwind(|| FrameSelection::retain(icon)).is_err());
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    #[test]
    fn foreign_pools_and_sides_are_rejected_before_record_access() {
        let mut a = RacePool::with_capacity(0, 1);
        let mut b = RacePool::with_capacity(0, 1);
        let mut c = RacePool::with_capacity(1, 1);
        let mut a = LinkQueue::<Starship>::new(&mut a);
        let mut b = LinkQueue::<Starship>::new(&mut b);
        let mut c = LinkQueue::<Starship>::new(&mut c);
        let foreign = a.alloc().unwrap();
        let ship = a.ship_id(foreign);
        let local = b.alloc().unwrap();
        let other_side = c.alloc().unwrap();
        assert_eq!(c.ship_id(other_side).side(), 1);
        assert!(!b.contains(foreign));
        assert!(catch_unwind(AssertUnwindSafe(|| b.get(foreign))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| b.link(ship))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| c.link(ship))).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| b.edit(foreign, |pick| pick.crew = 9))).is_err());
        assert_eq!(b.get(local).crew, 0);
    }
}

#[cfg(test)]
mod pool_view_tests {
    use super::*;
    #[derive(Debug, PartialEq, Eq)]
    enum Value {}
    impl queue_record::Sealed for Value {}
    impl MutableQueueRecord for Value {}
    impl QueueRecord for Value {
        type Record = u32;
    }

    #[test]
    fn consumed_views_keep_owner_lifetimes_and_reject_invalid_mutations() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let mut pool = OrderedPool::<u32, 4>::new();
        let mut view = LinkQueue::<Value, 4>::from_pool(&mut pool);
        let id = view.alloc().unwrap();
        assert_eq!(view.slot(id), 0);
        let address = view.raw(id) as usize;
        assert_eq!(view.journal_address(id), address);
        *view.into_record_mut(id) = 17;
        let shared = LinkQueue::<Value, 4>::borrow_pool(&pool);
        assert_eq!(*shared.into_record(id), 17);
        assert!(catch_unwind(AssertUnwindSafe(|| {
            LinkQueue::<Value, 4>::borrow_pool(&pool).into_record_mut(id);
        }))
        .is_err());
        let mut other = OrderedPool::<u32, 4>::new();
        let foreign = LinkQueue::<Value, 4>::from_pool(&mut other)
            .alloc()
            .unwrap();
        assert!(catch_unwind(AssertUnwindSafe(|| {
            LinkQueue::<Value, 4>::borrow_pool(&pool).into_record(foreign);
        }))
        .is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| {
            LinkQueue::<Value, 4>::borrow_pool(&pool).journal_address(foreign);
        }))
        .is_err());
        LinkQueue::<Value, 4>::from_pool(&mut pool).free(id);
        let view = LinkQueue::<Value, 4>::borrow_pool(&pool);
        assert!(!view.contains(id));
        assert_eq!(view.journal_address(id), address);
        assert!(catch_unwind(AssertUnwindSafe(|| {
            LinkQueue::<Value, 4>::from_pool(&mut pool).into_record_mut(id);
        }))
        .is_err());
    }

    #[test]
    fn find_visits_link_order_and_ignores_unlinked_allocations() {
        let mut pool = OrderedPool::<u32, 4>::new();
        let mut view = LinkQueue::<Value, 4>::from_pool(&mut pool);
        assert!(view.find(|_| true).is_none());
        let a = view.alloc().unwrap();
        let b = view.alloc().unwrap();
        let c = view.alloc().unwrap();
        let unlinked = view.alloc().unwrap();
        *view.get_mut(a) = 2;
        *view.get_mut(b) = 2;
        *view.get_mut(c) = 1;
        *view.get_mut(unlinked) = 42;
        view.put(c);
        view.put(a);
        view.put(b);
        let mut visited = Vec::new();
        assert_eq!(
            view.find(|value| {
                visited.push(*value);
                *value == 2
            }),
            Some(a)
        );
        assert_eq!(visited, [1, 2]);
        assert!(view.find(|value| *value == 42).is_none());
    }

    #[test]
    fn typed_pool_views_keep_order_capacity_and_generation_checks() {
        let mut pool = OrderedPool::<u32, 4>::with_capacity(3);
        let mut view = LinkQueue::<Value, 4>::from_pool(&mut pool);
        let a = view.alloc().unwrap();
        let b = view.alloc().unwrap();
        let c = view.alloc().unwrap();
        assert!(view.alloc().is_none());
        *view.get_mut(a) = 17;
        view.put(a);
        view.put(c);
        view.insert_before(b, Some(c));
        assert_eq!(view.count(), 3);
        assert_eq!(view.succ(a), Some(b));
        assert_eq!(view.pred(c), Some(b));
        view.remove(b);
        assert_eq!(view.succ(b), Some(c));
        view.free(b);
        assert!(!view.contains(b));
        let replacement = view.alloc().unwrap();
        assert_ne!(replacement, b);
        assert_eq!(replacement.slot, b.slot);
        assert_eq!(*view.get_mut(replacement), 0);
        assert_eq!(*view.record(a), 17);
        let view = LinkQueue::<Value, 4>::borrow_pool(&pool);
        assert_eq!(view.head(), Some(a));
        assert_eq!(view.tail(), Some(c));
        assert_eq!(view.count(), 2);
    }
}

impl<'a> LinkQueue<'a, Starship> {
    pub fn new(queue: &'a mut RacePool) -> Self {
        Self {
            side: queue.side,
            queue: QueueBorrow::Exclusive(&mut queue.pool),
            marker: PhantomData,
        }
    }
    pub fn borrow(queue: &'a RacePool) -> Self {
        Self {
            side: queue.side,
            queue: QueueBorrow::Shared(&queue.pool),
            marker: PhantomData,
        }
    }
}

impl<'a, T: MutableQueueRecord, const N: usize> LinkQueue<'a, T, N> {
    pub fn into_record_mut(self, id: LinkId<T>) -> &'a mut T::Record {
        let key = self.key(id);
        match self.queue {
            QueueBorrow::Exclusive(queue) => queue.get_mut(key),
            QueueBorrow::Shared(_) => panic!("cannot mutate a read-only queue view"),
        }
    }
    pub fn get_mut(&mut self, id: LinkId<T>) -> &mut T::Record {
        self.record_mut(id)
    }
}
