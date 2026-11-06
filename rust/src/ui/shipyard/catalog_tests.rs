//! Exercise the private legacy roster helpers with a real published campaign owner.

#[path = "catalog.rs"]
mod catalog;

use crate::engine::{
    battle::link_queue::LinkQueue,
    world::queues::{Fleet, FleetInfo},
    Game, World,
};
use crate::navigation::abi::{BYTE, COUNT, GOOD_GUY};

fn avail_race_queue<R>(read: impl FnOnce(&LinkQueue<'_, Fleet, 24>) -> R) -> R {
    World::current().read(|world| read(&world.avail_races_ref()))
}

fn with_roster(states: &[u16], check: impl FnOnce()) {
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
    // SAFETY: this serialized fixture retains the unique game for the full handoff.
    unsafe { crate::app::bridge::with_game(&mut game, check) };
}

#[test]
#[serial_test::serial]
fn empty_roster_has_no_available_escort() {
    with_roster(&[], || {
        assert_eq!(catalog::get_available_race_count(), 0);
        assert!(catalog::get_available_race_from_index(0).is_none());
        assert!(catalog::get_available_race_from_index(BYTE::MAX).is_none());
    });
}

#[test]
#[serial_test::serial]
fn selection_filters_allies_but_returns_the_original_roster_offset() {
    let allied = GOOD_GUY as u16;
    with_roster(&[0, allied, allied | 0x80, 0, allied], || {
        assert_eq!(catalog::get_available_race_count(), 2);
        for (selection, offset) in [1, 4].into_iter().enumerate() {
            let Some((index, fleet)) = catalog::get_available_race_from_index(selection as BYTE)
            else {
                panic!("the allied roster entry must be selectable");
            };
            assert_eq!(index, offset);
            assert_eq!(fleet.species_id, 100 + u32::from(offset));
            assert_eq!(fleet.crew_level, 10 + offset);
            assert_eq!(fleet.allied_state, allied);
        }
        assert!(catalog::get_available_race_from_index(2).is_none());
        assert!(catalog::get_available_race_from_index(BYTE::MAX).is_none());
    });
}

#[test]
#[serial_test::serial]
fn a_nonempty_roster_without_allies_has_no_selection() {
    with_roster(&[0, GOOD_GUY as u16 | 0x80, 0], || {
        assert_eq!(catalog::get_available_race_count(), 0);
        assert!(catalog::get_available_race_from_index(0).is_none());
    });
}
