//! Native fleet construction and escort bookkeeping ported from build.c.

use crate::ships::native::abi::*;

const SHIP_COSTS: [COUNT; 23] = [
    1600, 3000, 1100, 2300, 2000, 500, 1800, 1600, 1000, 2200, 1200, 2300, 3600, 1700, 1000, 2100,
    4400, 700, 3000, 600, 1300, 3000, 2300,
];

fn matches_race(stored: BYTE, requested: COUNT) -> bool {
    COUNT::from(stored) == requested
}

unsafe fn ship_from_index(queue: *const QUEUE, mut index: COUNT) -> HLINK {
    let mut handle = (*queue).head;
    while index > 0 && !handle.is_null() {
        handle = (*handle.cast::<LINK>()).succ;
        index -= 1;
    }
    handle
}

unsafe fn escort_window(queue: *const QUEUE, window: &mut BYTE) -> HLINK {
    loop {
        let old = ship_from_index(queue, COUNT::from(*window));
        *window = window.wrapping_add(1);
        if old.is_null() || *window <= (*old.cast::<SHIP_FRAGMENT>()).index {
            return old;
        }
    }
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::*;
    use std::ptr;

    const INFINITE_RADIUS_NATIVE: u16 = u16::MAX;

    /// # Safety
    /// Queue must be initialized and use STARSHIP or SHIP_FRAGMENT links.
    #[no_mangle]
    pub unsafe extern "C" fn Build(queue: *mut QUEUE, species: SPECIES_ID) -> HLINK {
        if queue.is_null() {
            return ptr::null_mut();
        }
        crate::ships::ffi::build_ship(&mut *queue.cast(), species as i32)
    }

    /// # Safety
    /// Queue must be initialized and its links remain locked only while read.
    #[no_mangle]
    pub unsafe extern "C" fn GetStarShipFromIndex(queue: *mut QUEUE, index: COUNT) -> HLINK {
        ship_from_index(queue, index)
    }

    /// # Safety
    /// Queue and handle must refer to the same live linked record.
    #[no_mangle]
    pub unsafe extern "C" fn GetIndexFromStarShip(queue: *mut QUEUE, mut handle: HLINK) -> COUNT {
        let mut index: COUNT = 0;
        while handle != (*queue).head {
            let link = handle.cast::<LINK>();
            handle = (*link).pred;
            index = index.wrapping_add(1);
        }
        index
    }

    /// # Safety
    /// Queue must be initialized and links must contain SHIP_BASE records.
    #[no_mangle]
    pub unsafe extern "C" fn NameCaptain(queue: *mut QUEUE, species: SPECIES_ID) -> BYTE {
        assert!(
            matches!((*queue).object_size as usize, size if size == std::mem::size_of::<STARSHIP>() || size == std::mem::size_of::<SHIP_FRAGMENT>())
        );
        loop {
            let name = (((TFB_Random() as COUNT) & (NUM_CAPTAINS_NAMES - 1) as COUNT)
                + NAME_OFFSET as COUNT) as BYTE;
            let mut handle = (*queue).head;
            let mut matched = false;
            while !handle.is_null() {
                let ship = handle.cast::<SHIP_BASE>();
                let next = (*ship).succ;
                let test_name = if (*ship).SpeciesID == species {
                    (*ship).captains_name_index
                } else {
                    BYTE::MAX
                };
                if test_name == name {
                    matched = true;
                    break;
                }
                handle = next;
            }
            if !matched {
                return name;
            }
        }
    }

    /// # Safety
    /// Fleet queue and fragment handle must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetEscortByStarShipIndex(index: COUNT) -> HSHIPFRAG {
        let queue = &raw mut GlobData.Game_state.built_ship_q;
        let mut handle = (*queue).head;
        while !handle.is_null() {
            let frag = handle.cast::<SHIP_FRAGMENT>();
            let next = (*frag).succ;
            let matched = matches_race((*frag).index, index);
            if matched {
                return handle;
            }
            handle = next;
        }
        ptr::null_mut()
    }

    /// # Safety
    /// Global queue state and fleet resources must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn CountEscortShips(race: COUNT) -> COUNT {
        if GetStarShipFromIndex(&raw mut GlobData.Game_state.avail_race_q, race).is_null() {
            return 0;
        }
        let queue = &raw mut GlobData.Game_state.built_ship_q;
        let mut handle = (*queue).head;
        let mut count: COUNT = 0;
        while !handle.is_null() {
            let frag = handle.cast::<SHIP_FRAGMENT>();
            let next = (*frag).succ;
            if matches_race((*frag).race_id, race) {
                count = count.wrapping_add(1);
            }
            handle = next;
        }
        count
    }

    /// # Safety
    /// Global queue state and fleet resources must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn HaveEscortShip(race: COUNT) -> BOOLEAN {
        (CountEscortShips(race) > 0) as BOOLEAN
    }

    /// # Safety
    /// Global queue state and fleet resources must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn EscortFeasibilityStudy(race: COUNT) -> COUNT {
        let fleet = GetStarShipFromIndex(&raw mut GlobData.Game_state.avail_race_q, race);
        if fleet.is_null() {
            return 0;
        }
        (MAX_BUILT_SHIPS as COUNT)
            .wrapping_sub(CountLinks(&raw mut GlobData.Game_state.built_ship_q))
    }

    /// # Safety
    /// Global fleet queue and UI are initialized.
    #[no_mangle]
    pub unsafe extern "C" fn RemoveSomeEscortShips(race: COUNT, mut count: COUNT) -> COUNT {
        if count == 0 {
            return 0;
        }
        let queue = &raw mut GlobData.Game_state.built_ship_q;
        let mut handle = (*queue).head;
        while !handle.is_null() {
            let frag = handle.cast::<SHIP_FRAGMENT>();
            let next = (*frag).succ;
            let matching = matches_race((*frag).race_id, race);
            if matching {
                RemoveQueue(queue, handle);
                FreeLink(queue, handle);
                count -= 1;
                if count == 0 {
                    break;
                }
            }
            handle = next;
        }
        if count > 0 {
            DeltaSISGauges(
                UNDEFINED_DELTA as i16,
                UNDEFINED_DELTA as i16,
                UNDEFINED_DELTA as i32,
            );
        }
        count
    }

    /// # Safety
    /// Global fleet queue and UI are initialized.
    #[no_mangle]
    pub unsafe extern "C" fn RemoveEscortShips(race: COUNT) {
        let _ = RemoveSomeEscortShips(race, COUNT::MAX);
    }

    /// # Safety
    /// Global race queues and their records are initialized.
    #[no_mangle]
    pub unsafe extern "C" fn CheckAlliance(race: COUNT) -> COUNT {
        let fleet = GetStarShipFromIndex(&raw mut GlobData.Game_state.avail_race_q, race);
        if fleet.is_null() {
            return 0;
        }
        let record = fleet.cast::<FLEET_INFO>();
        (*record).allied_state
    }

    /// # Safety
    /// Global race queues and their records are initialized.
    #[no_mangle]
    pub unsafe extern "C" fn SetRaceAllied(race: COUNT, allied: BOOLEAN) -> COUNT {
        let fleet = GetStarShipFromIndex(&raw mut GlobData.Game_state.avail_race_q, race);
        if fleet.is_null() {
            return 0;
        }
        let record = fleet.cast::<FLEET_INFO>();
        if (*record).allied_state != DEAD_GUY as u16 {
            (*record).allied_state = if allied != 0 {
                GOOD_GUY as u16
            } else {
                BAD_GUY as u16
            };
        }
        1
    }

    /// # Safety
    /// Race queue and its fleet records must be initialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn StartSphereTracking(race: COUNT) -> COUNT {
        let queue = &raw mut GlobData.Game_state.avail_race_q;
        let fleet = GetStarShipFromIndex(queue, race);
        if fleet.is_null() {
            return 0;
        }
        let info = fleet.cast::<FLEET_INFO>();
        if (*info).actual_strength == 0 && (*info).allied_state == DEAD_GUY as u16 {
            return 0;
        }
        if (*info).actual_strength != 0
            && (*info).known_strength == 0
            && (*info).actual_strength != INFINITE_RADIUS_NATIVE
        {
            (*info).known_strength = 1;
            (*info).known_loc = (*info).loc;
        }
        race
    }

    /// # Safety
    /// Race and escort queues must contain live fleet and fragment records.
    #[no_mangle]
    pub unsafe extern "C" fn SetEscortCrewComplement(
        which_ship: COUNT,
        crew_level: COUNT,
        captain: BYTE,
    ) -> i32 {
        let races = &raw mut GlobData.Game_state.avail_race_q;
        let fleet = GetStarShipFromIndex(races, which_ship);
        if fleet.is_null() {
            return -1;
        }
        let template = fleet.cast::<FLEET_INFO>();
        let full_crew = (*template).crew_level;
        let queue = &raw mut GlobData.Game_state.built_ship_q;
        let mut handle = (*queue).head;
        let mut index: i32 = 0;
        while !handle.is_null() {
            let frag = handle.cast::<SHIP_FRAGMENT>();
            let next = (*frag).succ;
            if matches_race((*frag).race_id, which_ship) && (*frag).crew_level == full_crew {
                (*frag).crew_level = crew_level;
                (*frag).captains_name_index = captain;
                return index;
            }
            handle = next;
            index += 1;
        }
        -1
    }

    /// # Safety
    /// Destination must be a fragment queue; global race templates must be live.
    #[no_mangle]
    pub unsafe extern "C" fn CloneShipFragment(
        ship_index: COUNT,
        destination: *mut QUEUE,
        crew_level: COUNT,
    ) -> HSHIPFRAG {
        assert_eq!(
            (*destination).object_size as usize,
            std::mem::size_of::<SHIP_FRAGMENT>()
        );
        let races = &raw mut GlobData.Game_state.avail_race_q;
        let fleet = GetStarShipFromIndex(races, ship_index);
        if fleet.is_null() {
            return ptr::null_mut();
        }
        let template = fleet.cast::<FLEET_INFO>();
        let species = (*template).SpeciesID;
        let captain = if ship_index == SAMATRA_SHIP as COUNT {
            0
        } else {
            NameCaptain(destination, species)
        };
        let built = Build(destination, species);
        if !built.is_null() {
            let frag = built.cast::<SHIP_FRAGMENT>();
            (*frag).captains_name_index = captain;
            (*frag).race_strings = (*template).race_strings;
            (*frag).icons = (*template).icons;
            (*frag).melee_icon = (*template).melee_icon;
            (*frag).crew_level = if crew_level != 0 {
                crew_level
            } else {
                (*template).crew_level
            };
            (*frag).max_crew = (*template).max_crew;
            (*frag).energy_level = 0;
            (*frag).max_energy = (*template).max_energy;
            (*frag).race_id = ship_index as BYTE;
            (*frag).index = 0;
        }
        built
    }

    /// # Safety
    /// Race and escort queues and SIS display must be initialized; count must be positive.
    #[no_mangle]
    pub unsafe extern "C" fn AddEscortShips(race: COUNT, count: SIZE) -> COUNT {
        let races = &raw mut GlobData.Game_state.avail_race_q;
        if GetStarShipFromIndex(races, race).is_null() {
            return 0;
        }
        assert!(count > 0);
        let queue = &raw mut GlobData.Game_state.built_ship_q;
        let mut window: BYTE = 0;
        let mut added: COUNT = 0;
        while added < count as COUNT {
            let ship = CloneShipFragment(race, queue, 0);
            if ship.is_null() {
                break;
            }
            RemoveQueue(queue, ship);
            let old = escort_window(queue, &mut window);
            let frag = ship.cast::<SHIP_FRAGMENT>();
            (*frag).index = window.wrapping_sub(1);
            InsertQueue(queue, ship, old);
            added = added.wrapping_add(1);
        }
        DeltaSISGauges(
            UNDEFINED_DELTA as i16,
            UNDEFINED_DELTA as i16,
            UNDEFINED_DELTA as i32,
        );
        added
    }

    /// # Safety
    /// Escort queue must contain live fragments with race IDs in RACE_SHIP_COST.
    #[no_mangle]
    pub unsafe extern "C" fn CalculateEscortsWorth() -> COUNT {
        let queue = &raw mut GlobData.Game_state.built_ship_q;
        let mut handle = (*queue).head;
        let mut total: COUNT = 0;
        while !handle.is_null() {
            let frag = handle.cast::<SHIP_FRAGMENT>();
            let next = (*frag).succ;
            total = total.wrapping_add(SHIP_COSTS[(*frag).race_id as usize]);
            handle = next;
        }
        total
    }
}

#[cfg(not(test))]
pub use engine::{
    AddEscortShips, Build, CalculateEscortsWorth, CheckAlliance, CloneShipFragment,
    CountEscortShips, EscortFeasibilityStudy, GetEscortByStarShipIndex, GetIndexFromStarShip,
    GetStarShipFromIndex, HaveEscortShip, NameCaptain, RemoveEscortShips, RemoveSomeEscortShips,
    SetEscortCrewComplement, SetRaceAllied, StartSphereTracking,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{mem, ptr};

    #[test]
    fn race_and_window_matching_preserves_count_width() {
        assert!(matches_race(255, 255));
        assert!(!matches_race(0, 256));
        assert!(!matches_race(255, 65535));
    }

    #[test]
    fn queue_lookup_and_escort_slots_follow_real_links() {
        unsafe {
            let mut queue: QUEUE = mem::zeroed();
            let mut first: SHIP_FRAGMENT = mem::zeroed();
            let mut second: SHIP_FRAGMENT = mem::zeroed();
            first.index = 0;
            second.index = 2;
            first.succ = (&raw mut second).cast();
            second.pred = (&raw mut first).cast();
            queue.head = (&raw mut first).cast();
            let mut window = 0;
            assert_eq!(escort_window(&queue, &mut window), (&raw mut second).cast());
            assert_eq!(window - 1, 1);
            assert_eq!(ship_from_index(&queue, 0), (&raw mut first).cast());
            assert_eq!(ship_from_index(&queue, 1), (&raw mut second).cast());
            assert_eq!(ship_from_index(&queue, 2), ptr::null_mut());
            queue.head = ptr::null_mut();
            window = 0;
            assert!(escort_window(&queue, &mut window).is_null());
            assert_eq!(window - 1, 0);
        }
    }

    #[test]
    fn worth_table_preserves_every_races_ship_cost_in_ru() {
        assert_eq!(
            SHIP_COSTS,
            [
                1600, 3000, 1100, 2300, 2000, 500, 1800, 1600, 1000, 2200, 1200, 2300, 3600, 1700,
                1000, 2100, 4400, 700, 3000, 600, 1300, 3000, 2300,
            ]
        );
    }
}
