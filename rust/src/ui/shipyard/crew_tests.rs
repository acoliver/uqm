//! Exercise the production crew accounting without native graphics.

#[path = "crew.rs"]
mod crew;

use crate::engine::{
    battle::{Battle, Cx},
    world::{queues::FleetInfo, story::GameState},
    Game, StoryKey,
};
use crate::navigation::abi::{GOOD_GUY, SHOFIXTI_SHIP};

fn transact(initial: u16, delta: i16, allied: bool) -> (u16, u8, Vec<(u16, u8)>) {
    let mut game = Game::new();
    game.world.story = Some(GameState::new());
    game.world
        .set_story_state(StoryKey::CREW_PURCHASED0, u32::from(initial as u8));
    game.world
        .set_story_state(StoryKey::CREW_PURCHASED1, u32::from((initial >> 8) as u8));
    game.world.data.game.crew_cost = 3;
    if allied {
        let mut races = game.world.avail_races();
        for index in 0..=SHOFIXTI_SHIP {
            let id = races
                .alloc()
                .unwrap_or_else(|| panic!("the fleet fixture fits its pool"));
            *races.get_mut(id) = FleetInfo {
                allied_state: if index == SHOFIXTI_SHIP {
                    GOOD_GUY as u16
                } else {
                    0
                },
                ..Default::default()
            };
            races.put(id);
        }
    }
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    );
    let mut repaints = Vec::new();
    crew::crew_transaction(&mut cx, delta, |cx| {
        repaints.push((
            crew::purchased_crew(cx.world) as u16,
            cx.world.data.game.crew_cost,
        ));
    });
    (
        crew::purchased_crew(cx.world) as u16,
        cx.world.data.game.crew_cost,
        repaints,
    )
}

#[test]
fn price_crossings_repaint_after_price_change_before_purchase_writeback() {
    assert_eq!(transact(995, 5, false), (1000, 5, vec![(995, 5)]));
    assert_eq!(transact(1000, -1, false), (999, 1, vec![(1000, 1)]));
    assert_eq!(transact(995, 4, false), (999, 3, vec![]));
    assert_eq!(transact(1001, -1, false), (1000, 3, vec![]));
}

#[test]
fn zero_and_signed_purchase_boundaries_preserve_legacy_saturation() {
    assert_eq!(transact(1000, 0, false), (1000, 3, vec![]));
    assert_eq!(transact(0, -1, false), (0, 3, vec![]));
    assert_eq!(transact(0x7fff, 1, false), (0x7fff, 3, vec![]));
    assert_eq!(transact(0x7fff, 0x7fff, false), (0x7fff, 3, vec![]));
    assert_eq!(transact(0, i16::MIN, false), (0, 3, vec![]));
    assert_eq!(transact(255, 2, false), (257, 3, vec![]));
}

#[test]
fn shofixti_alliance_keeps_purchase_bytes_but_not_price_crossing_side_effects() {
    assert_eq!(transact(995, 5, true), (995, 5, vec![(995, 5)]));
    assert_eq!(transact(1000, -1, true), (1000, 1, vec![(1000, 1)]));
}

#[test]
fn hangar_labels_distinguish_scrap_partial_full_and_overfull_crew() {
    let mut game = Game::new();
    let mut races = game.world.avail_races();
    let race = races
        .alloc()
        .unwrap_or_else(|| panic!("the fleet fixture fits its pool"));
    races.get_mut(race).crew_level = 30;
    races.put(race);
    let mut ships = game.world.built_ships();
    let ship = ships
        .alloc()
        .unwrap_or_else(|| panic!("the escort fixture fits its pool"));
    ships.put(ship);
    for (count, active, text) in [
        (0, false, "SCRAP\0"),
        (1, true, "1/30\0"),
        (29, true, "29/30\0"),
        (30, true, "30\0"),
        (31, true, "31\0"),
    ] {
        game.world.built_ships().get_mut(ship).crew_level = count;
        assert_eq!(crew::label(&mut game.world, ship), (active, text.into()));
    }
}

#[test]
#[should_panic(expected = "a hangar ship must have a fleet template")]
fn a_missing_hangar_template_is_an_invariant_failure() {
    let mut game = Game::new();
    let mut ships = game.world.built_ships();
    let ship = ships
        .alloc()
        .unwrap_or_else(|| panic!("the escort fixture fits its pool"));
    ships.put(ship);
    crew::label(&mut game.world, ship);
}
