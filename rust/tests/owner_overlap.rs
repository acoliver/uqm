//! Legacy callers may borrow disjoint fields without reacquiring their owner.
use std::{mem, ptr};
use uqm_rust::{
    app::bridge,
    engine::{Game, World},
    ships::native::abi::*,
};

#[test]
#[serial_test::serial]
fn sis_slot_snapshot_releases_world_before_support_ship_drawing() {
    let mut game = Game::new();
    game.world.data.flagship.module_slots.fill(255);
    game.world.data.flagship.drive_slots.fill(255);
    game.world.data.flagship.jet_slots.fill(255);
    game.world.data.flagship.crew_enlisted = 12;
    // SAFETY: the serialized fixture owns the lifecycle; absent render handles
    // are valid for the inactive graphics backend used by this test.
    unsafe {
        bridge::with_game(&mut game, || {
            uqm_rust::ui::sis::DeltaSISGauges(i16::MAX, i16::MAX, i32::from(i16::MAX));
        });
    }
    assert_eq!(game.world.data.flagship.crew_enlisted, 12);
    assert_eq!(game.world.data.flagship.module_slots, [255; 16]);
}

#[test]
#[serial_test::serial]
fn summary_picker_clones_from_the_already_borrowed_race_queue() {
    let mut game = Game::new();
    // SAFETY: this fixture exclusively owns both initialized queues for the publication.
    unsafe {
        bridge::with_game(&mut game, || {
            World::current().write(|world| {
                let races = &raw mut world.data.game.avail_race_q;
                let destination = &raw mut world.data.game.built_ship_q;
                assert_ne!(InitQueue(races, 1, mem::size_of::<FLEET_INFO>() as u16), 0);
                assert_ne!(
                    InitQueue(destination, 1, mem::size_of::<SHIP_FRAGMENT>() as u16),
                    0
                );
                let template = AllocLink(races).cast::<FLEET_INFO>();
                assert!(!template.is_null());
                (*template).SpeciesID = SPECIES_ID_EARTHLING_ID;
                (*template).crew_level = 12;
                (*template).max_crew = 18;
                PutQueue(races, template.cast());
                let cloned = CloneShipFragment(races, 0, destination, 0).cast::<SHIP_FRAGMENT>();
                assert!(!cloned.is_null());
                assert_eq!((*cloned).crew_level, 12);
                assert_eq!((*cloned).max_crew, 18);
                assert_eq!((*cloned).SpeciesID, SPECIES_ID_EARTHLING_ID);
                UninitQueue(destination);
                UninitQueue(races);
            });
        });
    }
}

#[test]
#[serial_test::serial]
fn restored_star_lookup_finishes_before_selecting_the_owned_star() {
    use uqm_rust::engine::{
        world::{CStarDesc, CATALOG_LEN},
        StoryKey,
    };
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
    game.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 0);
    // SAFETY: zero is valid for catalog descriptors, and the fixture keeps the table live.
    let mut catalog: Box<[CStarDesc; CATALOG_LEN]> = Box::new(unsafe { mem::zeroed() });
    for star in catalog.iter_mut() {
        star.star_pt.y = i16::MAX;
    }
    catalog[0].star_pt.x = 1752;
    catalog[0].star_pt.y = 1450;
    // SAFETY: the serialized fixture owns the temporary catalog and game lifecycle.
    unsafe {
        let previous = uqm_rust::planets::starmap::star_array;
        uqm_rust::planets::starmap::star_array = catalog.as_mut_ptr().cast();
        bridge::with_game(&mut game, || {
            let mut point = POINT { x: 1752, y: 1450 };
            let selected = uqm_rust::planets::starmap::FindStar(ptr::null_mut(), &mut point, 0, 0);
            assert_eq!(selected.cast::<CStarDesc>(), catalog.as_mut_ptr());
            World::current().write(|world| {
                world.select_star(selected.cast::<CStarDesc>().as_ref(), Some(&catalog));
            });
        });
        uqm_rust::planets::starmap::star_array = previous;
    }
    assert_eq!(
        game.world.star_pointer(Some(&catalog)),
        catalog.as_mut_ptr()
    );
}
