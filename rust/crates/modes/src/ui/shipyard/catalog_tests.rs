//! Exercise the real roster helpers against independent owned campaigns.

use super::shipyard_catalog as catalog;
use crate::engine::{world::queues::FleetInfo, Game, World};
use crate::ships::native::abi::{BYTE, GOOD_GUY};

fn with_roster(states: &[u16], check: impl FnOnce(&World)) {
    let mut game = Game::new();
    {
        let mut queue = game.world.avail_races();
        for (index, &allied_state) in states.iter().enumerate() {
            let Some(id) = queue.alloc() else {
                panic!("the roster fixture must fit its fixed-capacity queue");
            };
            *queue.get_mut(id) = FleetInfo {
                species_id: 100 + index as u32,
                allied_state,
                crew_level: 10 + index as u16,
                ..Default::default()
            };
            queue.put(id);
        }
    }
    check(&game.world);
}

#[test]
#[serial_test::serial]
fn empty_roster_has_no_available_escort() {
    with_roster(&[], |world| {
        assert_eq!(catalog::get_available_race_count(world), 0);
        assert!(catalog::get_available_race_from_index(world, 0).is_none());
        assert!(catalog::get_available_race_from_index(world, BYTE::MAX).is_none());
    });
}

#[test]
#[serial_test::serial]
fn selection_filters_allies_but_returns_the_original_roster_offset() {
    let allied = GOOD_GUY as u16;
    with_roster(&[0, allied, allied | 0x80, 0, allied], |world| {
        assert_eq!(catalog::get_available_race_count(world), 2);
        for (selection, offset) in [1, 4].into_iter().enumerate() {
            let Some((index, fleet)) =
                catalog::get_available_race_from_index(world, selection as BYTE)
            else {
                panic!("the allied roster entry must be selectable");
            };
            assert_eq!(index, offset);
            assert_eq!(fleet.species_id, 100 + u32::from(offset));
            assert_eq!(fleet.crew_level, 10 + offset);
            assert_eq!(fleet.allied_state, allied);
        }
        assert!(catalog::get_available_race_from_index(world, 2).is_none());
        assert!(catalog::get_available_race_from_index(world, BYTE::MAX).is_none());
    });
}

#[test]
#[serial_test::serial]
fn a_nonempty_roster_without_allies_has_no_selection() {
    with_roster(&[0, GOOD_GUY as u16 | 0x80, 0], |world| {
        assert_eq!(catalog::get_available_race_count(world), 0);
        assert!(catalog::get_available_race_from_index(world, 0).is_none());
    });
}

#[test]
fn independent_rosters_use_the_supplied_owner() {
    let empty = World::new();
    let mut allied = World::new();
    let mut queue = allied.avail_races();
    let id = queue.alloc().unwrap();
    queue.get_mut(id).allied_state = GOOD_GUY as u16;
    queue.put(id);
    assert_eq!(catalog::get_available_race_count(&empty), 0);
    assert_eq!(catalog::get_available_race_count(&allied), 1);
    assert!(catalog::get_available_race_from_index(&empty, 0).is_none());
    assert!(catalog::get_available_race_from_index(&allied, 0).is_some());
    assert_eq!(catalog::get_available_race_count(&empty), 0);
}
