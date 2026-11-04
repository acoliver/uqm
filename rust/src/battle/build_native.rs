//! Native fleet construction and escort bookkeeping ported from build.c.

use crate::engine::battle::link_queue::{LinkId, LinkQueue};
use crate::engine::world::queues::Fragment;
use crate::engine::World;

#[cfg(not(test))]
pub fn captain_name() -> u8 {
    ((crate::math::random() as u16 & 15) + 5) as u8
}

/// # Safety
/// The game thread owns the initialized SIS display, with no owner borrow retained.
#[cfg(not(test))]
pub unsafe fn refresh_escorts() {
    // SAFETY: the legacy caller has released its campaign mutation before drawing.
    unsafe { crate::ui::sis::DeltaSISGauges(32767, 32767, 32767) }
}

const SHIP_COSTS: [u16; 23] = [
    1600, 3000, 1100, 2300, 2000, 500, 1800, 1600, 1000, 2200, 1200, 2300, 3600, 1700, 1000, 2100,
    4400, 700, 3000, 600, 1300, 3000, 2300,
];

fn matches_race(stored: u8, requested: u16) -> bool {
    u16::from(stored) == requested
}

fn escort_window(queue: &LinkQueue<'_, Fragment, 12>, window: &mut u8) -> Option<LinkId<Fragment>> {
    loop {
        let old = queue.at(usize::from(*window));
        *window = window.wrapping_add(1);
        if old.is_none_or(|id| *window <= queue.get(id).index) {
            return old;
        }
    }
}

pub fn count_escort_ships(world: &World, race: u16) -> u16 {
    if world.avail_races_ref().at(usize::from(race)).is_none() {
        return 0;
    }
    let ships = world.built_ships_ref();
    let mut next = ships.head();
    let mut count: u16 = 0;
    while let Some(id) = next {
        if matches_race(ships.get(id).race_id, race) {
            count = count.wrapping_add(1);
        }
        next = ships.succ(id);
    }
    count
}

pub fn escort_feasibility(world: &World, race: u16) -> u16 {
    if world.avail_races_ref().at(usize::from(race)).is_none() {
        return 0;
    }
    12u16.wrapping_sub(world.built_ships_ref().count() as u16)
}

pub fn remove_escort_ships(world: &mut World, race: u16, mut count: u16) -> u16 {
    if count == 0 {
        return 0;
    }
    let mut ships = world.built_ships();
    let mut next = ships.head();
    while let Some(id) = next {
        next = ships.succ(id);
        if matches_race(ships.get(id).race_id, race) {
            ships.remove(id);
            ships.free(id);
            count -= 1;
            if count == 0 {
                break;
            }
        }
    }
    count
}

pub fn check_alliance(world: &World, race: u16) -> u16 {
    let fleets = world.avail_races_ref();
    fleets
        .at(usize::from(race))
        .map_or(0, |id| fleets.get(id).allied_state)
}

pub fn set_race_allied(world: &mut World, race: u16, allied: u32) -> u16 {
    let mut fleets = world.avail_races();
    let Some(id) = fleets.at(usize::from(race)) else {
        return 0;
    };
    let record = fleets.get_mut(id);
    if record.allied_state != 0 {
        record.allied_state = if allied != 0 { 1 } else { 2 };
    }
    1
}

pub fn start_sphere_tracking(world: &mut World, race: u16) -> u16 {
    let mut fleets = world.avail_races();
    let Some(id) = fleets.at(usize::from(race)) else {
        return 0;
    };
    let info = fleets.get_mut(id);
    if info.actual_strength == 0 && info.allied_state == 0 {
        return 0;
    }
    if info.actual_strength != 0 && info.known_strength == 0 && info.actual_strength != u16::MAX {
        info.known_strength = 1;
        info.known_loc = info.loc;
    }
    race
}

pub fn set_escort_crew(world: &mut World, race: u16, crew: u16, captain: u8) -> i32 {
    let fleets = world.avail_races_ref();
    let Some(template) = fleets.at(usize::from(race)) else {
        return -1;
    };
    let full_crew = fleets.get(template).crew_level;
    let mut ships = world.built_ships();
    let mut next = ships.head();
    let mut index = 0;
    while let Some(id) = next {
        next = ships.succ(id);
        let ship = ships.get_mut(id);
        if matches_race(ship.race_id, race) && ship.crew_level == full_crew {
            ship.crew_level = crew;
            ship.captains_name_index = captain;
            return index;
        }
        index += 1;
    }
    -1
}

pub fn add_escort_ships(
    world: &mut World,
    race: u16,
    count: i16,
    mut random: impl FnMut() -> u8,
) -> u16 {
    let fleets = world.avail_races_ref();
    let Some(id) = fleets.at(usize::from(race)) else {
        return 0;
    };
    let template = *fleets.get(id);
    assert!(count > 0);
    let mut ships = world.built_ships();
    let mut window = 0;
    let mut added: u16 = 0;
    while added < count as u16 {
        let Some(id) = ships.clone_fragment(&template, race as u8, 0, &mut random) else {
            break;
        };
        ships.remove(id);
        let old = escort_window(&ships, &mut window);
        ships.get_mut(id).index = window.wrapping_sub(1);
        ships.insert_before(id, old);
        added = added.wrapping_add(1);
    }
    added
}

pub fn escorts_worth(world: &World) -> u16 {
    let ships = world.built_ships_ref();
    let mut next = ships.head();
    let mut total: u16 = 0;
    while let Some(id) = next {
        total = total.wrapping_add(SHIP_COSTS[usize::from(ships.get(id).race_id)]);
        next = ships.succ(id);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_and_window_matching_preserves_count_width() {
        assert!(matches_race(255, 255));
        assert!(!matches_race(0, 256));
        assert!(!matches_race(255, 65535));
    }

    #[test]
    fn queue_lookup_and_escort_slots_follow_real_links() {
        let mut world = World::new();
        let mut ships = world.built_ships();
        let first = ships.alloc().unwrap();
        let second = ships.alloc().unwrap();
        ships.get_mut(first).index = 0;
        ships.get_mut(second).index = 2;
        ships.put(first);
        ships.put(second);
        let mut window = 0;
        assert_eq!(escort_window(&ships, &mut window), Some(second));
        assert_eq!(window - 1, 1);
        assert_eq!(ships.at(0), Some(first));
        assert_eq!(ships.at(1), Some(second));
        assert_eq!(ships.at(2), None);
        ships.reinit();
        window = 0;
        assert_eq!(escort_window(&ships, &mut window), None);
        assert_eq!(window - 1, 0);
    }

    #[test]
    fn escort_updates_keep_hangar_order_crew_limits_and_full_pool_rng() {
        let mut world = World::new();
        {
            let mut fleets = world.avail_races();
            let id = fleets.alloc().unwrap();
            let fleet = fleets.get_mut(id);
            fleet.SpeciesID = 1;
            fleet.crew_level = 10;
            fleet.max_crew = 20;
            fleet.allied_state = 1;
            fleets.put(id);
        }
        assert_eq!(escort_feasibility(&world, 0), 12);
        assert_eq!(escort_feasibility(&world, 256), 0);
        assert_eq!(check_alliance(&world, 0), 1);
        assert_eq!(count_escort_ships(&world, 256), 0);
        let mut name = 0;
        let mut random = || {
            name = (name + 1) & 15;
            name + 5
        };
        assert_eq!(add_escort_ships(&mut world, 0, 3, &mut random), 3);
        let removed = world.built_ships_ref().head().unwrap();
        assert_eq!(remove_escort_ships(&mut world, 0, 1), 0);
        assert!(!world.built_ships_ref().contains(removed));
        assert_eq!(add_escort_ships(&mut world, 0, 1, &mut random), 1);
        let ships = world.built_ships_ref();
        for index in 0..3 {
            assert_eq!(ships.get(ships.at(index).unwrap()).index as usize, index);
        }
        assert_eq!(count_escort_ships(&world, 0), 3);
        assert_eq!(escorts_worth(&world), 4800);
        assert_eq!(set_escort_crew(&mut world, 0, 1, 21), 0);
        assert_eq!(
            world
                .built_ships_ref()
                .get(world.built_ships_ref().head().unwrap())
                .crew_level,
            1
        );
        assert_eq!(set_escort_crew(&mut world, 256, 1, 21), -1);
        assert_eq!(add_escort_ships(&mut world, 0, 20, &mut random), 9);
        assert_eq!(escort_feasibility(&world, 0), 0);
        let mut draws = 0;
        assert_eq!(
            add_escort_ships(&mut world, 0, 1, || {
                draws += 1;
                20
            }),
            0
        );
        assert_eq!(draws, 1);
        assert_eq!(remove_escort_ships(&mut world, 0, u16::MAX), u16::MAX - 12);
        assert_eq!(world.built_ships_ref().count(), 0);
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
