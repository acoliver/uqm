//! Master metadata owns its descriptors in the game lifecycle's typed ship queue.
use crate::engine::battle::kernel::MasterCatalog;

#[derive(Clone, Debug)]
pub(crate) struct ShipChoice {
    pub cost: u16,
    pub icon: crate::engine::battle::frames::FrameSelection,
    pub melee_icon: crate::engine::battle::frames::FrameSelection,
}

pub(crate) fn choices(queue: &MasterCatalog) -> Vec<ShipChoice> {
    use crate::engine::battle::frames::FrameSelection;
    let mut result = Vec::with_capacity(queue.count());
    let mut next = queue.head();
    while let Some(id) = next {
        let info = queue.get(id).descriptor.info();
        result.push(ShipChoice {
            cost: u16::from(info.ship_cost),
            icon: FrameSelection::retain(info.icons.cast()),
            melee_icon: FrameSelection::retain(info.melee_icon.cast()),
        });
        next = queue.succ(id);
    }
    result
}

pub(crate) fn choice_cost(catalog: &[ShipChoice], index: u16) -> u16 {
    catalog.get(usize::from(index)).map_or(0, |ship| ship.cost)
}

pub(crate) fn find(queue: &MasterCatalog, species: u32) -> i32 {
    let mut next = queue.head();
    let mut index = 0;
    while let Some(id) = next {
        if queue.get(id).species == species {
            return index;
        }
        next = queue.succ(id);
        index += 1;
    }
    -1
}

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::engine::battle::{
        kernel::CatalogEntry,
        record::{FLEET_STUFF, SHIP_INFO},
    };
    use crate::engine::{Kernel, World};

    #[derive(Clone, Copy)]
    pub struct MasterShip {
        pub species: u32,
        pub info: SHIP_INFO,
        pub fleet: FLEET_STUFF,
    }

    pub fn load_entry(world: &mut World, kernel: &mut Kernel, species: u32) {
        if species == crate::ships::native::abi::SPECIES_ID_ARILOU_ID {
            assert_eq!(kernel.master_queue.count(), 0, "catalog already loaded");
        }
        let descriptor = crate::engine::battle::RaceDesc::load(world, species, false)
            .unwrap_or_else(|| panic!("master species {species}"));
        let name = descriptor.catalog_name().to_vec();
        let queue = &mut kernel.master_queue;
        let id = queue
            .alloc(CatalogEntry {
                species,
                descriptor,
            })
            .unwrap_or_else(|| panic!("master catalog capacity"));
        let mut before = queue.head();
        while let Some(other) = before {
            if name.as_slice() < queue.get(other).descriptor.catalog_name() {
                break;
            }
            before = queue.succ(other);
        }
        queue.insert_before(id, before);
    }

    pub fn free(world: &mut World, kernel: &mut Kernel) {
        let queue = &mut kernel.master_queue;
        while let Some(id) = queue.head() {
            queue.remove(id);
            queue.free(id).descriptor.unload(world, true, false);
        }
        queue.reinit();
    }

    fn entry(queue: &MasterCatalog, id: crate::collections::pool::Id<CatalogEntry>) -> MasterShip {
        MasterShip {
            species: queue.get(id).species,
            info: *queue.get(id).descriptor.info(),
            fleet: *queue.get(id).descriptor.fleet(),
        }
    }

    pub fn master_in(kernel: &Kernel, index: u16) -> Option<MasterShip> {
        let queue = &kernel.master_queue;
        let mut next = queue.head();
        for _ in 0..index {
            next = queue.succ(next?);
        }
        next.map(|id| entry(queue, id))
    }

    pub fn snapshot(kernel: &Kernel) -> Vec<MasterShip> {
        let queue = &kernel.master_queue;
        let mut result = Vec::with_capacity(queue.count());
        let mut next = queue.head();
        while let Some(id) = next {
            result.push(entry(queue, id));
            next = queue.succ(id);
        }
        result
    }

    pub fn find_master_ship(kernel: &Kernel, species: u32) -> Option<MasterShip> {
        let queue = &kernel.master_queue;
        let mut next = queue.head();
        while let Some(id) = next {
            if queue.get(id).species == species {
                return Some(entry(queue, id));
            }
            next = queue.succ(id);
        }
        None
    }
}
#[cfg(not(test))]
pub use engine::{find_master_ship, master_in};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::battle::{kernel::CatalogEntry, record::RaceDescriptor, RaceDesc};

    fn push(queue: &mut MasterCatalog, species: u32, cost: u8) {
        let mut descriptor = RaceDescriptor::EMPTY;
        descriptor.ship_info.ship_cost = cost;
        let id = queue
            .alloc(CatalogEntry {
                species,
                descriptor: RaceDesc::fixture(Box::new(descriptor)),
            })
            .unwrap();
        queue.put(id);
    }

    #[test]
    fn cost_uses_the_owned_catalog_in_queue_order() {
        let mut kernel = crate::engine::Kernel::new();
        let ships = &mut kernel.master_queue;
        assert_eq!(choice_cost(&choices(ships), 0), 0);
        push(ships, 23, 23);
        push(ships, 1, 16);
        assert_eq!(choice_cost(&choices(ships), 0), 23);
        assert_eq!(choice_cost(&choices(ships), 1), 16);
        assert_eq!(choice_cost(&choices(ships), 2), 0);
        assert_eq!(choice_cost(&choices(ships), u16::MAX), 0);
        let retained = choices(ships);
        assert!(retained[0].icon.is_absent());
        assert!(retained[0].melee_icon.is_absent());
        let id = ships.head().unwrap();
        ships.remove(id);
        let retired = ships.free(id);
        assert_eq!(retired.species, 23);
        assert_eq!(retired.descriptor.info().ship_cost, 23);
        assert_eq!(choice_cost(&choices(ships), 0), 16);
    }

    #[test]
    fn lookup_uses_queue_order_and_reports_absent_species() {
        let mut ships = MasterCatalog::new();
        assert_eq!(find(&ships, 1), -1);
        for species in [23, 1, 2] {
            push(&mut ships, species, 0);
        }
        assert_eq!(find(&ships, 1), 1);
        assert_eq!(find(&ships, 2), 2);
        assert_eq!(find(&ships, 99), -1);
        ships.reinit();
        assert_eq!(find(&ships, 1), -1);
        for species in 1..=25 {
            push(&mut ships, species, 0);
        }
        assert!(ships
            .alloc(CatalogEntry {
                species: 26,
                descriptor: RaceDesc::fixture(Box::new(RaceDescriptor::EMPTY)),
            })
            .is_none());
    }
}
