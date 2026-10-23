//! Behaviour tests of the Shipyard: its rules, and the screen driven with
//! scripted menu keys against the recording platform. Prices, crew levels,
//! the crew price and the hangar layout come from the C sources.

use super::gfx::{Align, FlashTarget, Point, Rgba};
use super::ids::{HANGAR_COLOR_TAB, SHIPYARD_MUSIC, SHIPYARD_PMAP_ANIM, SISBLU_MASK_ANIM};
use super::input::MenuPulses;
use super::platform::{AlliedState, Escort, Platform, StarbaseError, StateKey, UnportedRoute};
use super::screen::{run_shipyard, run_starbase_menu};
use super::shipyard::{
    crew_dot, crew_pod_capacity, crew_transaction, dismiss_refund, ship_cost, slot_corner, Cursor,
    CREW_EXPENSE_THRESHOLD, HANGAR_DY, HANGAR_SHIPS, HANGAR_SHIPS_ROW, HANGAR_X, HANGAR_Y,
    SHIP_COST,
};
use super::sis_modules::ModuleType;
use super::test_support::{
    cancel, down, idle, left, press, right, select, up, Event, RecordingPlatform, HUMAN_SHIP,
    SHOFIXTI_SHIP,
};
use crate::mainloop::types::{activity_flags, ActivityValue};

const SHIPS_DIR_SOURCE: &str = "../sc2/src/uqm";

fn c_source(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/{SHIPS_DIR_SOURCE}/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the C source exists")
}

fn special() -> MenuPulses {
    press(|p| p.special = true)
}

fn assert_nothing_leaked(platform: &RecordingPlatform) {
    let (frames, music) = platform.leaked_resources();
    assert!(frames.is_empty(), "frames leaked: {frames:?}");
    assert!(music.is_empty(), "music leaked: {music:?}");
    assert!(
        platform.leaked_color_maps().is_empty(),
        "colour maps leaked"
    );
}

/// A new-game flagship with room for crew: crew pods in the slots.
fn platform_with(res_units: u32) -> RecordingPlatform {
    let platform = RecordingPlatform::with_new_game_flagship(res_units);
    {
        let mut sis = platform.sis.borrow_mut();
        sis.module_slots[1] = ModuleType::CrewPod as u8;
        sis.module_slots[2] = ModuleType::CrewPod as u8;
        sis.crew_enlisted = 12;
    }
    platform
}

/// Opens the Shipyard and enters the hangar, then plays `keys`, then leaves
/// the hangar and the Shipyard.
fn hangar_session(platform: &RecordingPlatform, keys: impl IntoIterator<Item = MenuPulses>) {
    platform.push_pulses([idle(), select()]);
    platform.push_pulses(keys);
    platform.push_pulses([cancel(), cancel()]);
    run_shipyard(platform).expect("the Shipyard runs to its end");
}

/// The flagship's crew editing: select on the flagship, `keys`, then leave.
fn flagship_crew(platform: &RecordingPlatform, keys: impl IntoIterator<Item = MenuPulses>) {
    let mut script = vec![select()];
    script.extend(keys);
    script.push(cancel());
    hangar_session(platform, script);
}

/// The crew editing of the escort in slot 0 (`down` moves off the flagship).
fn escort_crew(platform: &RecordingPlatform, keys: impl IntoIterator<Item = MenuPulses>) {
    let mut script = vec![down(), select()];
    script.extend(keys);
    script.push(cancel());
    hangar_session(platform, script);
}

fn center_texts(platform: &RecordingPlatform) -> Vec<String> {
    platform
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Text(t) if t.align == Align::Center => Some(t.text),
            _ => None,
        })
        .collect()
}

fn count(platform: &RecordingPlatform, wanted: impl Fn(&Event) -> bool) -> usize {
    platform
        .events()
        .iter()
        .filter(|event| wanted(event))
        .count()
}

// --- Through the Starbase menu ---------------------------------------------

#[test]
fn shipyard_hires_flagship_crew_through_the_starbase_menu() {
    let platform = platform_with(5000);
    let before = platform.sis.borrow().clone();
    let crew_cost = platform.crew_cost.get();
    assert!(crew_pod_capacity(&before.module_slots) > u32::from(before.crew_enlisted));
    platform.push_pulses([
        idle(),
        right(),
        right(),
        select(), // Shipyard
        idle(),   // the Shipyard opens
        select(), // Crew: the hangar opens
        select(), // the flagship: edit its crew
        up(),     // hire one crew member
        cancel(), // leave crew editing
        cancel(), // leave the hangar
        cancel(), // leave the Shipyard
        idle(),
        right(),
        select(), // Depart
    ]);

    assert_eq!(run_starbase_menu(&platform), Ok(()));

    let after = platform.sis.borrow().clone();
    assert_eq!(after.crew_enlisted, before.crew_enlisted + 1);
    assert_eq!(after.res_units, before.res_units - u32::from(crew_cost));
    assert_eq!(platform.state(StateKey::CrewPurchased0), 1);
    assert_eq!(platform.state(StateKey::CrewPurchased1), 0);
    assert_nothing_leaked(&platform);
}

#[test]
fn leaving_the_shipyard_returns_to_the_starbase_menu_with_its_art_released() {
    let platform = platform_with(5000);
    platform.push_pulses([
        idle(),
        right(),
        right(),
        select(), // Shipyard
        idle(),
        cancel(), // leave the Shipyard
        idle(),
        right(),
        select(), // Depart
    ]);
    assert_eq!(run_starbase_menu(&platform), Ok(()));
    assert_nothing_leaked(&platform);
    assert_eq!(
        count(
            &platform,
            |e| matches!(e, Event::LoadGraphic(name, _) if name == SHIPYARD_PMAP_ANIM)
        ),
        1
    );
}

#[test]
fn the_shipyard_loads_its_art_colour_map_and_music_and_releases_them() {
    let platform = platform_with(5000);
    platform.push_pulses([idle(), select(), cancel(), cancel()]);
    run_shipyard(&platform).unwrap();
    let events = platform.events();
    for resource in [SHIPYARD_PMAP_ANIM, SISBLU_MASK_ANIM] {
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::LoadGraphic(name, _) if name == resource)));
    }
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::LoadColorMap(name, _) if name == HANGAR_COLOR_TAB)));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::LoadMusic(name, _) if name == SHIPYARD_MUSIC)));
    assert!(events.contains(&Event::StopMusic));
    assert_nothing_leaked(&platform);
}

// --- Flagship crew -----------------------------------------------------------

#[test]
fn hiring_flagship_crew_pays_the_crew_price_and_draws_the_crew_member() {
    let platform = platform_with(1000);
    let before = platform.sis.borrow().clone();
    flagship_crew(&platform, [up(), up()]);
    let after = platform.sis.borrow().clone();
    let price = u32::from(platform.crew_cost.get());
    assert_eq!(after.crew_enlisted, before.crew_enlisted + 2);
    assert_eq!(after.res_units, before.res_units - 2 * price);
    assert_eq!(
        count(
            &platform,
            |e| matches!(e, Event::CrewGaugeDelta(1, d) if *d == -(price as i32))
        ),
        2
    );
    let (point, color) = crew_dot(
        &before.module_slots,
        before.crew_enlisted,
        super::platform::UiStyle::ThreeDo,
    )
    .unwrap();
    let events = platform.events();
    let drawn = events
        .windows(2)
        .any(|pair| pair == [Event::SetFg(color), Event::DrawPoint(point)]);
    assert!(drawn, "the new crew member is drawn at {point:?}");
    assert_nothing_leaked(&platform);
}

#[test]
fn flagship_crew_is_refused_without_the_ru_and_without_pod_room() {
    let poor = platform_with(2);
    flagship_crew(&poor, [up()]);
    assert_eq!(poor.sis.borrow().res_units, 2);
    assert_eq!(poor.sis.borrow().crew_enlisted, 12);
    assert_eq!(count(&poor, |e| *e == Event::FailureSound), 1);
    assert_eq!(count(&poor, |e| matches!(e, Event::CrewGaugeDelta(..))), 0);

    let full = platform_with(5000);
    let capacity = crew_pod_capacity(&full.sis.borrow().module_slots);
    full.sis.borrow_mut().crew_enlisted = capacity as u16;
    flagship_crew(&full, [up()]);
    assert_eq!(full.sis.borrow().crew_enlisted, capacity as u16);
    assert_eq!(full.sis.borrow().res_units, 5000);
    assert_eq!(count(&full, |e| *e == Event::FailureSound), 1);
}

#[test]
fn the_price_of_a_crew_member_is_enough_ru() {
    let platform = platform_with(0);
    let price = u32::from(platform.crew_cost.get());
    platform.sis.borrow_mut().res_units = price;
    flagship_crew(&platform, [up()]);
    assert_eq!(platform.sis.borrow().res_units, 0);
    assert_eq!(platform.sis.borrow().crew_enlisted, 13);
}

#[test]
fn dismissing_flagship_crew_refunds_the_crew_price_and_erases_the_dot() {
    let platform = platform_with(1000);
    let before = platform.sis.borrow().clone();
    flagship_crew(&platform, [down()]);
    let after = platform.sis.borrow().clone();
    assert_eq!(after.crew_enlisted, before.crew_enlisted - 1);
    assert_eq!(
        after.res_units,
        before.res_units + u32::from(platform.crew_cost.get())
    );
    let (point, _) = crew_dot(
        &after.module_slots,
        after.crew_enlisted,
        super::platform::UiStyle::ThreeDo,
    )
    .unwrap();
    let events = platform.events();
    assert!(events
        .windows(2)
        .any(|pair| pair == [Event::SetFg(Rgba::BLACK), Event::DrawPoint(point)]));

    let empty = platform_with(1000);
    empty.sis.borrow_mut().crew_enlisted = 0;
    flagship_crew(&empty, [down()]);
    assert_eq!(empty.sis.borrow().res_units, 1000);
    assert_eq!(count(&empty, |e| *e == Event::FailureSound), 1);
}

#[test]
fn crew_purchases_are_counted_and_the_price_rises_past_the_threshold() {
    let platform = platform_with(5000);
    platform
        .states
        .borrow_mut()
        .insert(StateKey::CrewPurchased0, (999 & 0xFF) as u32);
    platform
        .states
        .borrow_mut()
        .insert(StateKey::CrewPurchased1, (999 >> 8) as u32);
    let base = platform.crew_cost.get();

    flagship_crew(&platform, [up()]);
    assert_eq!(platform.state(StateKey::CrewPurchased0), 1000 & 0xFF);
    assert_eq!(platform.state(StateKey::CrewPurchased1), 1000 >> 8);
    assert_eq!(platform.crew_cost.get(), base + 2);
    assert_eq!(platform.sis.borrow().res_units, 5000 - u32::from(base));

    // The dismissed crew member at exactly the threshold returns the old
    // price, and the price falls back.
    let ru = platform.sis.borrow().res_units;
    let second = platform_with(ru);
    second.sis.borrow_mut().crew_enlisted = platform.sis.borrow().crew_enlisted;
    second.crew_cost.set(base + 2);
    second
        .states
        .borrow_mut()
        .insert(StateKey::CrewPurchased0, 1000 & 0xFF);
    second
        .states
        .borrow_mut()
        .insert(StateKey::CrewPurchased1, 1000 >> 8);
    flagship_crew(&second, [down()]);
    assert_eq!(second.sis.borrow().res_units, ru + u32::from(base));
    assert_eq!(second.crew_cost.get(), base);
    assert_eq!(second.state(StateKey::CrewPurchased0), 999 & 0xFF);
    assert_eq!(second.state(StateKey::CrewPurchased1), 999 >> 8);
}

#[test]
fn an_allied_shofixti_stops_the_crew_purchase_count() {
    let platform = platform_with(5000);
    platform.races.borrow_mut()[usize::from(SHOFIXTI_SHIP)].allied = AlliedState::GoodGuy;
    flagship_crew(&platform, [up()]);
    assert_eq!(platform.sis.borrow().crew_enlisted, 13);
    assert_eq!(platform.state(StateKey::CrewPurchased0), 0);
}

#[test]
fn a_crew_update_that_disagrees_with_the_transaction_is_an_error() {
    let platform = platform_with(5000);
    platform.corrupt_crew.set(true);
    platform.push_pulses([idle(), select(), select(), up()]);
    assert!(matches!(
        run_shipyard(&platform),
        Err(StarbaseError::CrewMismatch { .. })
    ));
    assert_nothing_leaked(&platform);
}

// --- Escort crew and ships -----------------------------------------------

#[test]
fn escort_crew_is_dismissed_for_the_crew_price_and_hired_back() {
    let platform = platform_with(1000);
    let price = u32::from(platform.crew_cost.get());
    // A full ship accepts no crew.
    escort_crew(&platform, [up()]);
    assert_eq!(count(&platform, |e| *e == Event::FailureSound), 1);
    assert_eq!(platform.sis.borrow().res_units, 1000);

    let platform = platform_with(1000);
    escort_crew(&platform, [down(), up()]);
    assert_eq!(
        platform.sis.borrow().res_units,
        1000,
        "dismiss then hire is even"
    );
    assert_eq!(platform.escorts.borrow()[0].crew_level, 18);
    let texts = center_texts(&platform);
    assert!(texts.contains(&"17/18".to_owned()));
    assert!(texts.contains(&"18".to_owned()));
    assert_eq!(
        count(&platform, |e| matches!(e, Event::SetEscortCrew(0, 17))),
        1
    );
    assert_eq!(
        count(
            &platform,
            |e| matches!(e, Event::GaugeDelta(d) if d.res_units == price as i32)
        ),
        1
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn an_escort_hire_needs_the_crew_price_even_for_an_empty_ship() {
    let platform = platform_with(0);
    platform.escorts.borrow_mut()[0].crew_level = 0;
    escort_crew(&platform, [up()]);
    assert_eq!(count(&platform, |e| *e == Event::FailureSound), 1);
    assert_eq!(
        platform.escorts.borrow().len(),
        0,
        "the empty ship is scrapped on leaving"
    );
}

#[test]
fn selling_every_crew_member_sells_the_ship_for_its_price() {
    let platform = platform_with(1000);
    let price = u32::from(platform.crew_cost.get());
    escort_crew(&platform, vec![down(); 18]);
    // 17 crew members return the crew price, the last one the ship price.
    let expected = 1000 + 17 * price + u32::from(ship_cost(HUMAN_SHIP).unwrap());
    assert_eq!(platform.sis.borrow().res_units, expected);
    assert!(platform.escorts.borrow().is_empty());
    assert!(center_texts(&platform).contains(&"SCRAP".to_owned()));
    assert_eq!(count(&platform, |e| matches!(e, Event::RemoveEscort(0))), 1);
    assert_eq!(
        count(&platform, |e| matches!(
            e,
            Event::ShipStatus(super::platform::ResUnitsUpdate::Redraw)
        )),
        1
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn dismissing_from_a_scrap_ship_fails_twice_as_the_c_code_does() {
    let platform = platform_with(1000);
    platform.escorts.borrow_mut()[0].crew_level = 0;
    escort_crew(&platform, [down()]);
    assert_eq!(count(&platform, |e| *e == Event::FailureSound), 2);
}

#[test]
fn the_ship_buyer_navigates_the_allied_races_and_buys_at_the_c_price() {
    let platform = platform_with(5000);
    platform.races.borrow_mut()[0].allied = AlliedState::GoodGuy; // Arilou, then Human
                                                                  // Slot 1 is empty: `right` off the Earthling Cruiser in slot 0.
    let script = vec![
        down(),   // slot 0
        right(),  // slot 1
        select(), // choose a ship to build
        right(),  // Earthling Cruiser
        right(),  // wraps to the Arilou Skiff
        left(),   // wraps back to the Earthling Cruiser
        select(), // buy it
        up(),     // hire one more crew member
    ];
    let mut keys = script;
    keys.push(cancel()); // leave crew editing
    hangar_session(&platform, keys);

    let cost = u32::from(SHIP_COST[usize::from(HUMAN_SHIP)]);
    let crew_price = u32::from(platform.crew_cost.get());
    assert_eq!(platform.sis.borrow().res_units, 5000 - cost - crew_price);
    let escorts = platform.escorts.borrow().clone();
    assert_eq!(escorts.len(), 2);
    assert_eq!(escorts[1].slot, 1);
    assert_eq!(escorts[1].race_id, HUMAN_SHIP);
    assert_eq!(escorts[1].crew_level, 2);
    assert_eq!(
        count(
            &platform,
            |e| matches!(e, Event::AddEscort(race, 1) if *race == HUMAN_SHIP)
        ),
        1
    );
    // The ship price is shown in the race panel.
    assert!(platform.events().iter().any(
        |e| matches!(e, Event::Text(t) if t.align == Align::Right && t.text == cost.to_string())
    ));
    assert_nothing_leaked(&platform);
}

#[test]
fn a_ship_costs_exactly_its_price_and_refuses_one_ru_less() {
    let cost = u32::from(SHIP_COST[usize::from(HUMAN_SHIP)]);
    let exact = platform_with(cost);
    hangar_session(&exact, [down(), right(), select(), select(), cancel()]);
    assert_eq!(exact.sis.borrow().res_units, 0);
    assert_eq!(exact.escorts.borrow().len(), 2);

    let short = platform_with(cost - 1);
    hangar_session(&short, [down(), right(), select(), select(), cancel()]);
    assert_eq!(short.sis.borrow().res_units, cost - 1);
    assert_eq!(short.escorts.borrow().len(), 1);
    assert_eq!(count(&short, |e| *e == Event::FailureSound), 1);
    assert_eq!(count(&short, |e| matches!(e, Event::AddEscort(..))), 0);
}

#[test]
fn a_full_fleet_queue_refuses_the_ship() {
    let platform = platform_with(5000);
    platform.queue_full.set(true);
    hangar_session(&platform, [down(), right(), select(), select(), cancel()]);
    assert_eq!(platform.sis.borrow().res_units, 5000);
    assert_eq!(count(&platform, |e| *e == Event::FailureSound), 1);
    assert_eq!(platform.escorts.borrow().len(), 1);
}

#[test]
fn cancelling_the_ship_choice_returns_to_navigating_the_slots() {
    let platform = platform_with(5000);
    hangar_session(&platform, [down(), right(), select(), cancel(), left()]);
    assert_eq!(platform.escorts.borrow().len(), 1);
    assert_eq!(platform.sis.borrow().res_units, 5000);
    let events = platform.events();
    assert!(events.contains(&Event::Flash(FlashTarget::MenuAny)));
}

#[test]
fn a_fleet_with_no_allied_race_cannot_buy_a_ship() {
    let platform = platform_with(5000);
    platform.races.borrow_mut()[usize::from(HUMAN_SHIP)].allied = AlliedState::BadGuy;
    platform.push_pulses([idle(), select(), down(), right(), select()]);
    assert_eq!(
        run_shipyard(&platform),
        Err(StarbaseError::NoShipsAvailable)
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn a_ship_race_outside_the_price_table_is_an_error() {
    let platform = platform_with(5000);
    let escort = platform.escorts.borrow()[0];
    platform.escorts.borrow_mut()[0] = Escort {
        race_id: 40,
        ..escort
    };
    platform.push_pulses([idle(), select()]);
    assert_eq!(run_shipyard(&platform), Err(StarbaseError::UnknownRace(40)));
    assert_nothing_leaked(&platform);
}

// --- Navigation, exits and unported routes ---------------------------------

#[test]
fn the_shipyard_menu_wraps_and_the_exit_entry_leaves() {
    let platform = platform_with(5000);
    // Crew -> Exit (previous wraps) -> select leaves.
    platform.push_pulses([idle(), left(), select()]);
    run_shipyard(&platform).unwrap();
    assert_nothing_leaked(&platform);
    assert_eq!(count(&platform, |e| *e == Event::Sleep(840 / 20)), 1);
}

#[test]
fn leaving_and_reentering_the_hangar_keeps_the_shipyard_open() {
    let platform = platform_with(5000);
    platform.push_pulses([
        idle(),
        select(), // hangar
        cancel(), // back to the menu
        select(), // hangar again
        cancel(),
        cancel(), // leave the Shipyard
    ]);
    run_shipyard(&platform).unwrap();
    assert_eq!(count(&platform, |e| matches!(e, Event::LoadMusic(..))), 1);
    assert_nothing_leaked(&platform);
}

#[test]
fn save_load_ends_the_shipyard_with_the_typed_failure_and_releases_everything() {
    let platform = platform_with(5000);
    platform.push_pulses([idle(), right(), select()]);
    assert_eq!(
        run_shipyard(&platform),
        Err(StarbaseError::Unported(UnportedRoute::SaveLoad))
    );
    assert_nothing_leaked(&platform);
    assert!(!platform.events().contains(&Event::FailureSound));
}

#[test]
fn the_special_key_on_an_escort_is_the_typed_ship_spin_failure() {
    let platform = platform_with(5000);
    platform.push_pulses([idle(), select(), down(), special()]);
    assert_eq!(
        run_shipyard(&platform),
        Err(StarbaseError::Unported(UnportedRoute::ShipSpin))
    );
    assert_nothing_leaked(&platform);

    // On the flagship or an empty slot the key does nothing.
    let quiet = platform_with(5000);
    hangar_session(&quiet, [special(), down(), right(), special()]);
    assert_eq!(quiet.sis.borrow().res_units, 5000);
}

#[test]
fn an_abort_in_the_hangar_leaves_through_the_shipyard_and_releases_everything() {
    let platform = platform_with(5000);
    platform.push_pulses([idle(), select(), down(), idle(), idle()]);
    platform.abort_after_pulses.set(Some(4));
    run_shipyard(&platform).unwrap();
    assert_nothing_leaked(&platform);
    assert_eq!(
        platform.current_activity(),
        ActivityValue::new(activity_flags::CHECK_ABORT)
    );
}

#[test]
fn a_resource_that_fails_to_load_releases_what_was_loaded() {
    for resource in [
        SHIPYARD_PMAP_ANIM,
        HANGAR_COLOR_TAB,
        SHIPYARD_MUSIC,
        SISBLU_MASK_ANIM,
    ] {
        let platform = platform_with(5000);
        *platform.failing_resource.borrow_mut() = Some(resource.to_owned());
        platform.push_pulses([idle()]);
        assert_eq!(
            run_shipyard(&platform),
            Err(StarbaseError::ResourceLoad(resource.to_owned())),
            "{resource}"
        );
        assert_nothing_leaked(&platform);
    }
}

#[test]
fn crew_that_do_not_fit_the_pods_are_an_error_when_the_blueprint_is_drawn() {
    let platform = platform_with(5000);
    platform.sis.borrow_mut().crew_enlisted = 500;
    platform.push_pulses([idle()]);
    assert_eq!(
        run_shipyard(&platform),
        Err(StarbaseError::CrewExceedsPods { crew: 500 })
    );
    assert_nothing_leaked(&platform);
}

// --- Rules ---------------------------------------------------------------------

#[test]
fn the_cursor_moves_like_dms_move_cursor() {
    let start = Cursor::START;
    assert!(start.is_flagship());
    // Down from the flagship wraps to the first escort row.
    assert_eq!(start.moved(0, 1).slot(), 0);
    let first = start.moved(0, 1);
    assert!(!first.is_flagship());
    assert_eq!(first.slot(), 0);
    // Up from the first row reaches the flagship; up again the last row.
    assert!(first.moved(0, -1).is_flagship());
    assert_eq!(start.moved(0, -1).slot(), HANGAR_SHIPS_ROW);
    // Columns wrap.
    assert_eq!(first.moved(-1, 0).slot(), HANGAR_SHIPS_ROW - 1);
    assert_eq!(first.moved(-1, 0).moved(1, 0).slot(), 0);
    // The flagship ignores horizontal moves and remembers its column.
    assert_eq!(start.moved(1, 0), start);
    let third = first.moved(1, 0).moved(1, 0);
    // Two rows down from the first escort row is the flagship.
    let flagship = third.moved(0, 1).moved(0, 1);
    assert!(flagship.is_flagship());
    assert_eq!(flagship.slot(), 2);
    assert_eq!(flagship.moved(0, 1).slot(), 2);
}

#[test]
fn crew_transactions_follow_crew_transaction() {
    // Crossing the threshold upward raises the price.
    let up = crew_transaction(999, 3, 1);
    assert_eq!(
        (up.purchased, up.crew_cost, up.cost_changed),
        (1000, 5, true)
    );
    // Staying above it does not.
    let above = crew_transaction(1000, 5, 1);
    assert_eq!(
        (above.purchased, above.crew_cost, above.cost_changed),
        (1001, 5, false)
    );
    // Crossing downward lowers it.
    let down = crew_transaction(1000, 5, -1);
    assert_eq!(
        (down.purchased, down.crew_cost, down.cost_changed),
        (999, 3, true)
    );
    // The count never goes below zero or wraps past 0x7FFF.
    assert_eq!(crew_transaction(0, 3, -1).purchased, 0);
    assert_eq!(crew_transaction(0x7FFF, 5, 1).purchased, 0x7FFF);
    assert_eq!(dismiss_refund(5, CREW_EXPENSE_THRESHOLD as i16), 3);
    assert_eq!(dismiss_refund(5, 1001), 5);
}

#[test]
fn crew_members_are_seated_from_the_last_pod_backwards() {
    let mut slots = [super::sis_modules::EMPTY_MODULE_SLOT; 16];
    slots[1] = ModuleType::CrewPod as u8;
    slots[4] = ModuleType::CrewPod as u8;
    // The first crew member sits in the last pod, slot 4.
    let (point, color) = crew_dot(&slots, 0, super::platform::UiStyle::Pc).unwrap();
    assert_eq!(
        point,
        Point {
            x: 27 + 4 * 12,
            y: 34
        }
    );
    assert_eq!(color, Rgba::rgb15(0x0A, 0x1E, 0x09));
    // Seat 5 starts the second row: two pixels up and back to column 0.
    let (point, _) = crew_dot(&slots, 5, super::platform::UiStyle::Pc).unwrap();
    assert_eq!(
        point,
        Point {
            x: 27 + 4 * 12,
            y: 32
        }
    );
    // Crew member 50 sits in the next pod.
    let (point, _) = crew_dot(&slots, 50, super::platform::UiStyle::ThreeDo).unwrap();
    assert_eq!(point, Point { x: 27 + 12, y: 34 });
    assert!(crew_dot(&slots, 100, super::platform::UiStyle::Pc).is_none());
}

#[test]
fn prices_and_layout_are_the_c_values() {
    let races = c_source("races.h");
    let table = races
        .split("#define RACE_SHIP_COST")
        .nth(1)
        .and_then(|rest| rest.split("#define LOG_TO_IP").next())
        .expect("RACE_SHIP_COST in races.h");
    let costs: Vec<u16> = table
        .lines()
        .filter_map(|line| {
            line.trim()
                .trim_start_matches('\\')
                .trim()
                .split(',')
                .next()
        })
        .filter_map(|field| field.trim().parse().ok())
        .collect();
    assert_eq!(costs, SHIP_COST.to_vec());

    let layout = &crate::ui::shipyard_layout::HANGAR_X_COORDS;
    assert_eq!(*layout, &[0, 38, 76, 131, 169, 207]);
    assert_eq!(crate::ui::shipyard_layout::HANGAR_SHIPS_ROW, 6);
    assert_eq!(crate::ui::shipyard_layout::HANGAR_Y, 88);
    assert_eq!(crate::ui::shipyard_layout::HANGAR_DY, 84);
    assert_eq!(
        crate::ui::shipyard_layout::HANGAR_ROWS * crate::ui::shipyard_layout::HANGAR_SHIPS_ROW,
        12
    );
    assert_eq!(HANGAR_X, [0, 38, 76, 131, 169, 207]);
    assert_eq!((HANGAR_Y, HANGAR_DY, HANGAR_SHIPS), (88, 84, 12));
    assert_eq!(slot_corner(7), Point { x: 38, y: 88 + 84 });
    for (resource, header) in [
        (SHIPYARD_PMAP_ANIM, "igfxres.h"),
        (SISBLU_MASK_ANIM, "igfxres.h"),
        (SHIPYARD_MUSIC, "imusicre.h"),
        (HANGAR_COLOR_TAB, "istrtab.h"),
    ] {
        assert!(
            c_source(header).contains(&format!("\"{resource}\"")),
            "{resource}"
        );
    }
    let sis = c_source("sis.h");
    assert!(sis.contains("#define CREW_EXPENSE_THRESHOLD 1000"));
    assert!(sis.contains("#define CREW_POD_CAPACITY 50"));
}
