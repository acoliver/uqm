//! The module installation route through the production Outfit screen.
//!
//! Every test drives `run_outfit` or `run_starbase_menu` with ordinary menu
//! keys, the way the player does: Outfit, highlight Modules, select, then the
//! chooser's own keys. The ship is the one a new game starts with, read from
//! the C initializer, so prices, slots and fuel are the native ones. Nothing
//! here writes a module into a slot except the chooser.

use super::gfx::{Align, ContextId, FlashTarget, FrameId, Point, Rect, Stamp};
use super::ids::{MODULES_PMAP_ANIM, SISMODS_MASK_PMAP_ANIM};
use super::input::MenuPulses;
use super::module_chooser::ModuleChooserError;
use super::new_game_reference::{modules_menu_frames, outfit_modules_frames, PieceNumbers};
use super::platform::{ResUnitsUpdate, SisView, StarbaseError, StateKey, UiStyle, UnportedRoute};
use super::screen::{run_outfit, run_starbase_menu};
use super::sis_modules::ModuleType;
use super::test_support::{cancel, down, idle, left, right, select, up, Event, RecordingPlatform};
use crate::mainloop::types::{activity_flags, ActivityValue};

/// Where the radar strip draws (the same point the Outfit menu art uses).
const STRIP: Point = Point { x: 10, y: 184 };

/// Keys that take the Outfit screen from its first frame into the module
/// chooser and out of both again: initialise, highlight Modules, select
/// Modules (which opens the chooser in the same frame), the given chooser
/// keys, then cancel out of the chooser and cancel out of the Outfit screen.
fn module_route(chooser_keys: impl IntoIterator<Item = MenuPulses>) -> Vec<MenuPulses> {
    let mut keys = vec![idle(), right(), select()];
    keys.extend(chooser_keys);
    keys.extend([cancel(), cancel()]);
    keys
}

fn run_modules(
    platform: &RecordingPlatform,
    chooser_keys: impl IntoIterator<Item = MenuPulses>,
) -> Result<(), StarbaseError> {
    platform.push_pulses(module_route(chooser_keys));
    run_outfit(platform)
}

fn assert_nothing_leaked(platform: &RecordingPlatform) {
    let (frames, music) = platform.leaked_resources();
    assert!(frames.is_empty(), "frames leaked: {frames:?}");
    assert!(music.is_empty(), "music leaked: {music:?}");
}

fn loaded(events: &[Event], resource: &str) -> FrameId {
    events
        .iter()
        .find_map(|event| match event {
            Event::LoadGraphic(name, frame) if name == resource => Some(*frame),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{resource} was loaded"))
}

fn position(events: &[Event], wanted: impl Fn(&Event) -> bool) -> usize {
    events
        .iter()
        .position(wanted)
        .expect("expected event was recorded")
}

fn count(events: &[Event], wanted: impl Fn(&Event) -> bool) -> usize {
    events.iter().filter(|event| wanted(event)).count()
}

/// The ship as `platform` started with it, then changed by `edit`.
fn with(sis: &SisView, edit: impl FnOnce(&mut SisView)) -> SisView {
    let mut changed = sis.clone();
    edit(&mut changed);
    changed
}

/// Module numbers the radar strip showed, in order, taken from the stamps
/// drawn from the module menu art.
fn shown_modules(platform: &RecordingPlatform) -> Vec<usize> {
    let events = platform.events();
    let menu_set = loaded(&events, MODULES_PMAP_ANIM) / 1000;
    events
        .iter()
        .filter_map(|event| match event {
            Event::DrawStamp(Stamp { origin, frame })
                if *origin == STRIP && frame / 1000 == menu_set =>
            {
                Some(frame % 1000)
            }
            _ => None,
        })
        .collect()
}

fn flashes(platform: &RecordingPlatform) -> Vec<FlashTarget> {
    platform
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::Flash(target) => Some(*target),
            _ => None,
        })
        .collect()
}

fn failure_sounds(platform: &RecordingPlatform) -> usize {
    count(&platform.events(), |e| *e == Event::FailureSound)
}

fn price_texts(platform: &RecordingPlatform) -> Vec<String> {
    platform
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::Text(text)
                if text.align == Align::Right && text.baseline == (Point { x: 64, y: 235 }) =>
            {
                Some(text.text.clone())
            }
            _ => None,
        })
        .collect()
}

// --- The route exists -----------------------------------------------------

#[test]
fn selecting_modules_opens_the_chooser_instead_of_failing() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    let before = platform.sis.borrow().clone();

    run_modules(&platform, []).expect("the Modules route is ported");

    assert_eq!(*platform.sis.borrow(), before);
    assert_nothing_leaked(&platform);
}

#[test]
fn the_first_chooser_frame_clears_the_radar_and_flashes_the_first_module_slot() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    run_modules(&platform, []).unwrap();
    let events = platform.events();

    let opened = position(&events, |e| {
        *e == Event::Flash(FlashTarget::Rect(Rect {
            corner: Point { x: 16, y: 116 },
            width: 14,
            height: 34,
        }))
    });
    // The radar strip is cleared, then the flash stops, then the strip shows
    // the (empty) slot and the slot flashes: `ClearSISRect`, `SetFlashRect
    // (NULL)`, `DrawModuleStrings`, `SetFlashRect (&flash_rect0)`.
    let cleared = events[..opened]
        .iter()
        .rposition(|e| *e == Event::Flash(FlashTarget::Off))
        .expect("the flash stops before the slot flashes");
    assert!(cleared < opened);
    assert!(events[..cleared].contains(&Event::SetContext(ContextId::Space)));
    // The strip backdrop was filled when the Outfit menu opened and when
    // Modules was highlighted, and fills again to clear the radar.
    let backdrop = |e: &Event| {
        matches!(e, Event::FillRect(r)
            if *r == Rect { corner: Point { x: 9, y: 173 }, width: 58, height: 11 })
    };
    assert_eq!(count(&events[..cleared], backdrop), 3);
    assert_eq!(
        count(&events[cleared..opened], |e| matches!(
            e,
            Event::FillRect(_)
        )),
        2,
        "an empty slot draws the backdrop and a black radar area"
    );
    assert_eq!(shown_modules(&platform), Vec::<usize>::new());
}

// --- Buying ---------------------------------------------------------------

#[test]
fn buying_a_fuel_tank_pays_its_price_and_fills_the_empty_slot() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    let before = platform.sis.borrow().clone();

    // Offer starts at the crew pod; storage bay, then fuel tank.
    run_modules(&platform, [select(), right(), right(), select()]).unwrap();

    let expected = with(&before, |sis| {
        sis.module_slots[0] = ModuleType::FuelTank as u8;
        sis.res_units = 1000 - 500;
    });
    assert_eq!(*platform.sis.borrow(), expected);
    assert_eq!(failure_sounds(&platform), 0);
    assert_nothing_leaked(&platform);

    let events = platform.events();
    let module_frame = loaded(&events, SISMODS_MASK_PMAP_ANIM);
    let write = position(&events, |e| {
        *e == Event::SetModuleSlot(0, ModuleType::FuelTank as u8)
    });
    let pay = position(&events, |e| {
        *e == Event::ShipStatus(ResUnitsUpdate::Change(-500))
    });
    let piece = position(&events, |e| {
        matches!(e, Event::DrawStamp(stamp)
            if stamp.frame == module_frame + ModuleType::FuelTank as usize
                && stamp.origin == Point { x: 17, y: 117 })
    });
    assert!(write < pay && pay < piece);
    // A second tank gives two bars' worth of capacity: ten fuel bars are
    // redrawn after the piece.
    let redrawn = events[piece..]
        .iter()
        .filter(|e| matches!(e, Event::FillRect(r) if r.width == 5 && r.height == 1))
        .count();
    assert_eq!(redrawn, 10);
    assert_eq!(count(&events, |e| matches!(e, Event::ShipStatus(_))), 2);
}

#[test]
fn a_purchase_for_exactly_the_balance_is_allowed() {
    let platform = RecordingPlatform::with_new_game_flagship(500);
    run_modules(&platform, [select(), right(), right(), select()]).unwrap();
    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.module_slots[0], ModuleType::FuelTank as u8);
    assert_eq!(sis.res_units, 0);
    assert_eq!(failure_sounds(&platform), 0);
}

#[test]
fn a_purchase_one_ru_short_is_refused_and_changes_nothing() {
    let platform = RecordingPlatform::with_new_game_flagship(499);
    let before = platform.sis.borrow().clone();

    // The refused select leaves the offer open, so cancel it before the
    // route's own cancels.
    run_modules(&platform, [select(), right(), right(), select(), cancel()]).unwrap();

    assert_eq!(*platform.sis.borrow(), before);
    assert_eq!(failure_sounds(&platform), 1);
    let events = platform.events();
    assert_eq!(count(&events, |e| matches!(e, Event::SetModuleSlot(..))), 0);
    assert_eq!(count(&events, |e| matches!(e, Event::ShipStatus(_))), 1);
    assert_nothing_leaked(&platform);
}

#[test]
fn a_drive_is_bought_for_an_empty_thruster_slot() {
    let platform = RecordingPlatform::with_new_game_flagship(700);
    let before = platform.sis.borrow().clone();

    // Up from the module slots reaches the jets, up again the drives.
    run_modules(&platform, [up(), up(), select(), select()]).unwrap();

    let expected = with(&before, |sis| {
        sis.drive_slots[0] = 1;
        sis.res_units = 200;
    });
    assert_eq!(*platform.sis.borrow(), expected);
    assert!(platform.events().contains(&Event::SetDriveSlot(0, 1)));
}

#[test]
fn a_turning_jet_is_bought_for_an_empty_jet_slot() {
    let platform = RecordingPlatform::with_new_game_flagship(700);
    let before = platform.sis.borrow().clone();

    // Jet slot 0 holds jets: move to slot 1 and buy there.
    run_modules(&platform, [up(), right(), select(), select()]).unwrap();

    let expected = with(&before, |sis| {
        sis.jet_slots[1] = 2;
        sis.res_units = 200;
    });
    assert_eq!(*platform.sis.borrow(), expected);
    assert!(platform.events().contains(&Event::SetJetSlot(1, 2)));
}

#[test]
fn a_lander_is_bought_for_an_empty_berth() {
    let platform = RecordingPlatform::with_new_game_flagship(700);
    let before = platform.sis.borrow().clone();

    // Up from the jets reaches the drives, and up again the lander berths.
    // Berth 0 holds the only lander: move to berth 1.
    run_modules(&platform, [up(), up(), up(), right(), select(), select()]).unwrap();

    let expected = with(&before, |sis| {
        sis.num_landers = 2;
        sis.res_units = 200;
    });
    assert_eq!(*platform.sis.borrow(), expected);
    let events = platform.events();
    assert!(events.contains(&Event::SetNumLanders(2)));
    // The lander row is redrawn: two lander sprites now.
    let module_frame = loaded(&events, SISMODS_MASK_PMAP_ANIM);
    let write = position(&events, |e| *e == Event::SetNumLanders(2));
    let sprites = events[write..]
        .iter()
        .filter(|e| matches!(e, Event::DrawStamp(s) if s.frame == module_frame && s.origin.y == 67))
        .count();
    assert_eq!(sprites, 2);
}

// --- Removing -------------------------------------------------------------

#[test]
fn removing_a_module_refunds_its_price_and_empties_the_slot() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    let before = platform.sis.borrow().clone();

    // Slot 1 holds the fuel tank; the 1000 fuel fits the reserve alone.
    run_modules(&platform, [right(), select()]).unwrap();

    let expected = with(&before, |sis| {
        sis.module_slots[1] = 22;
        sis.res_units = 500;
    });
    assert_eq!(*platform.sis.borrow(), expected);
    let events = platform.events();
    assert!(events.contains(&Event::SetModuleSlot(1, 22)));
    assert!(events.contains(&Event::ShipStatus(ResUnitsUpdate::Change(500))));
    assert_eq!(failure_sounds(&platform), 0);
    assert_nothing_leaked(&platform);
}

#[test]
fn a_fuel_tank_holding_fuel_the_reserve_cannot_hold_stays() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    platform.sis.borrow_mut().fuel_on_board = 2000;
    let before = platform.sis.borrow().clone();

    run_modules(&platform, [right(), select()]).unwrap();

    assert_eq!(*platform.sis.borrow(), before);
    assert_eq!(failure_sounds(&platform), 1);
    assert_eq!(
        count(&platform.events(), |e| matches!(
            e,
            Event::SetModuleSlot(..)
        )),
        0
    );
}

#[test]
fn a_crew_pod_stays_while_crew_needs_it_and_goes_when_the_ship_is_empty() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    let before = platform.sis.borrow().clone();
    assert_eq!(before.crew_enlisted, 50);
    run_modules(&platform, [right(), right(), select()]).unwrap();
    assert_eq!(*platform.sis.borrow(), before);
    assert_eq!(failure_sounds(&platform), 1);

    let platform = RecordingPlatform::with_new_game_flagship(0);
    platform.sis.borrow_mut().crew_enlisted = 0;
    let before = platform.sis.borrow().clone();
    run_modules(&platform, [right(), right(), select()]).unwrap();
    let expected = with(&before, |sis| {
        sis.module_slots[2] = 22;
        sis.res_units = 2000;
    });
    assert_eq!(*platform.sis.borrow(), expected);
    assert_eq!(failure_sounds(&platform), 0);
}

#[test]
fn a_storage_bay_stays_while_cargo_needs_it_and_goes_when_the_hold_is_empty() {
    let eight_rights = [right(); 8];

    let platform = RecordingPlatform::with_new_game_flagship(0);
    platform.sis.borrow_mut().total_element_mass = 1;
    let before = platform.sis.borrow().clone();
    run_modules(&platform, eight_rights.into_iter().chain([select()])).unwrap();
    assert_eq!(*platform.sis.borrow(), before);
    assert_eq!(failure_sounds(&platform), 1);

    let platform = RecordingPlatform::with_new_game_flagship(0);
    let before = platform.sis.borrow().clone();
    run_modules(&platform, eight_rights.into_iter().chain([select()])).unwrap();
    let expected = with(&before, |sis| {
        sis.module_slots[8] = 22;
        sis.res_units = 750;
    });
    assert_eq!(*platform.sis.borrow(), expected);
}

#[test]
fn removing_the_only_lander_refunds_its_price() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    let before = platform.sis.borrow().clone();

    run_modules(&platform, [up(), up(), up(), select()]).unwrap();

    let expected = with(&before, |sis| {
        sis.num_landers = 0;
        sis.res_units = 500;
    });
    assert_eq!(*platform.sis.borrow(), expected);
}

#[test]
fn removing_a_drive_and_a_jet_refunds_their_prices() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    let before = platform.sis.borrow().clone();

    // Jet slot 0 holds jets. Drive slot 5 holds a thruster: five rights.
    let keys = [
        up(),
        select(),
        up(),
        right(),
        right(),
        right(),
        right(),
        right(),
        select(),
    ];
    run_modules(&platform, keys).unwrap();

    let expected = with(&before, |sis| {
        sis.jet_slots[0] = 21;
        sis.drive_slots[5] = 20;
        sis.res_units = 1000;
    });
    assert_eq!(*platform.sis.borrow(), expected);
}

// --- Moving the cursor ----------------------------------------------------

#[test]
fn the_cursor_flashes_each_kind_of_slot_where_the_native_screen_does() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    // Module slot 0, jets, drives, landers, then lander berth 1.
    run_modules(&platform, [up(), up(), up(), right()]).unwrap();

    let rect = |x, y, width, height| {
        FlashTarget::Rect(Rect {
            corner: Point { x, y },
            width,
            height,
        })
    };
    let slot_flashes: Vec<_> = flashes(&platform)
        .into_iter()
        .filter(|flash| *flash != FlashTarget::Off)
        .collect();
    assert_eq!(
        slot_flashes,
        [
            // The Outfit menu flash, then the chooser's slots.
            FlashTarget::Menu3do,
            rect(16, 116, 14, 34),
            rect(69, 93, 9, 10),
            rect(32, 85, 8, 6),
            rect(23, 66, 13, 15),
            rect(38, 66, 13, 15),
            // Cancelling out of the chooser restores the Outfit menu flash.
            FlashTarget::Menu3do,
        ]
    );
}

#[test]
fn module_slots_wrap_and_the_flash_follows_the_slot() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    run_modules(&platform, [left(), right()]).unwrap();
    let slot_flashes: Vec<_> = flashes(&platform)
        .into_iter()
        .filter(|flash| *flash != FlashTarget::Off && *flash != FlashTarget::Menu3do)
        .collect();
    let at = |item: i16| {
        FlashTarget::Rect(Rect {
            corner: Point {
                x: 16 + 12 * item,
                y: 116,
            },
            width: 14,
            height: 34,
        })
    };
    // Slot 0, left wraps to slot 15, right wraps back to slot 0.
    assert_eq!(slot_flashes, [at(0), at(15), at(0)]);
}

#[test]
fn the_strip_shows_what_the_cursor_is_on() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    // Slot 1 holds a fuel tank (500 RU); slot 2 a crew pod (2000 RU).
    run_modules(&platform, [right(), right()]).unwrap();
    assert_eq!(
        shown_modules(&platform),
        [ModuleType::FuelTank as usize, ModuleType::CrewPod as usize]
    );
    // The first price is the fuel price the Outfit menu draws when it opens.
    assert_eq!(price_texts(&platform), ["20", "500", "2000"]);
}

#[test]
fn the_offer_skips_modules_that_are_not_for_sale_and_wraps() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    // Slot 3 cannot hold weapons. For sale there: crew pod, storage bay,
    // fuel tank and dynamo unit. Wrap from the dynamo unit back to the pod,
    // and backward from the pod to the dynamo unit.
    let keys = [
        right(),
        right(),
        right(),
        select(),
        right(),
        right(),
        right(),
        right(),
        left(),
        cancel(),
    ];
    run_modules(&platform, keys).unwrap();
    let crew = ModuleType::CrewPod as usize;
    let storage = ModuleType::StorageBay as usize;
    let tank = ModuleType::FuelTank as usize;
    let dynamo = ModuleType::DynamoUnit as usize;
    assert_eq!(
        shown_modules(&platform),
        // slot 3 selected (and the strip of slots 1, 2, 3 on the way), the
        // offer, then storage, tank, dynamo, wrap to pod, backward to dynamo.
        [
            ModuleType::FuelTank as usize,
            ModuleType::CrewPod as usize,
            crew,
            storage,
            tank,
            dynamo,
            crew,
            dynamo,
        ]
    );
}

#[test]
fn only_slot_zero_and_the_last_three_may_be_offered_the_gun() {
    let gun = ModuleType::GunWeapon as usize;
    let dynamo = ModuleType::DynamoUnit as usize;

    let platform = RecordingPlatform::with_new_game_flagship(0);
    run_modules(
        &platform,
        [select(), right(), right(), right(), right(), cancel()],
    )
    .unwrap();
    let shown = shown_modules(&platform);
    assert_eq!(&shown[3..5], [dynamo, gun]);

    // Slot 14 may hold a weapon too.
    let platform = RecordingPlatform::with_new_game_flagship(0);
    let mut keys = vec![left(), left(), select()];
    keys.extend([right(); 4]);
    keys.push(cancel());
    run_modules(&platform, keys).unwrap();
    assert!(shown_modules(&platform).contains(&gun));
}

#[test]
fn offers_are_flashed_with_the_menu_frame_and_cancel_goes_back_to_the_slots() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    run_modules(&platform, [select(), cancel()]).unwrap();
    let slot_flashes: Vec<_> = flashes(&platform)
        .into_iter()
        .filter(|flash| *flash != FlashTarget::Off)
        .collect();
    let slot_zero = FlashTarget::Rect(Rect {
        corner: Point { x: 16, y: 116 },
        width: 14,
        height: 34,
    });
    assert_eq!(
        slot_flashes,
        [
            // Initial flash of the Outfit menu, then the chooser opens.
            FlashTarget::Menu3do,
            slot_zero,
            FlashTarget::MenuAny,
            slot_zero,
            FlashTarget::Menu3do,
        ]
    );
}

#[test]
fn a_bomb_in_the_hull_starts_the_cursor_past_the_bomb_slots_and_skips_the_landers() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    platform
        .states
        .borrow_mut()
        .insert(StateKey::ChmmrBombState, 3);
    // From the first free module slot: down skips the landers and reaches the
    // drives; up from the drives skips the landers and returns to modules.
    run_modules(&platform, [down(), up()]).unwrap();
    let slot_flashes: Vec<_> = flashes(&platform)
        .into_iter()
        .filter(|flash| *flash != FlashTarget::Off && *flash != FlashTarget::Menu3do)
        .collect();
    let module_ten = FlashTarget::Rect(Rect {
        corner: Point {
            x: 16 + 12 * 10,
            y: 116,
        },
        width: 14,
        height: 34,
    });
    let drive_zero = FlashTarget::Rect(Rect {
        corner: Point { x: 32, y: 85 },
        width: 8,
        height: 6,
    });
    assert_eq!(slot_flashes, [module_ten, drive_zero, module_ten]);
}

// --- Leaving --------------------------------------------------------------

#[test]
fn cancel_restores_the_outfit_menu_and_the_ship_status() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    run_modules(&platform, []).unwrap();
    let events = platform.events();

    // The Modules entry of the Outfit menu is drawn again, then the menu
    // flashes, then the ship status is redrawn (`ClearSISRect
    // (DRAW_SIS_DISPLAY)`). The entry was drawn once before, when the
    // player highlighted it.
    let leave = events
        .iter()
        .rposition(|e| *e == Event::Flash(FlashTarget::Menu3do))
        .expect("the menu flashes again");
    let menu_art = |e: &Event| matches!(e, Event::DrawStamp(stamp) if stamp.frame == 9022 && stamp.origin == STRIP);
    assert_eq!(count(&events[..leave], menu_art), 2);
    assert_eq!(
        events[leave..]
            .iter()
            .find(|e| matches!(e, Event::ShipStatus(_))),
        Some(&Event::ShipStatus(ResUnitsUpdate::Redraw))
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn the_outfit_screen_goes_on_after_the_chooser_and_the_chooser_opens_again() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    // Modules, back out, Modules again, back out, then Exit.
    let keys = [
        idle(),
        right(),
        select(),
        cancel(),
        select(),
        cancel(),
        cancel(),
    ];
    platform.push_pulses(keys);
    run_outfit(&platform).unwrap();
    let opens = count(&platform.events(), |e| {
        *e == Event::Flash(FlashTarget::Rect(Rect {
            corner: Point { x: 16, y: 116 },
            width: 14,
            height: 34,
        }))
    });
    assert_eq!(opens, 2);
    assert!(platform.script.borrow().is_empty());
    assert_nothing_leaked(&platform);
}

#[test]
fn save_load_after_the_chooser_still_fails_with_its_typed_error() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    platform.push_pulses([idle(), right(), select(), cancel(), right(), select()]);
    assert_eq!(
        run_outfit(&platform),
        Err(StarbaseError::Unported(UnportedRoute::SaveLoad))
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn abort_inside_the_chooser_leaves_the_ship_alone_and_releases_everything() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    let before = platform.sis.borrow().clone();
    platform.abort_after_pulses.set(Some(5));
    // Offer open at the fifth pulse, when the abort arrives.
    platform.push_pulses([idle(), right(), select(), select(), idle()]);

    run_outfit(&platform).unwrap();

    assert_eq!(*platform.sis.borrow(), before);
    assert_eq!(failure_sounds(&platform), 0);
    assert_nothing_leaked(&platform);
}

#[test]
fn a_pending_load_does_not_close_the_chooser() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    platform
        .activity
        .set(ActivityValue::new(activity_flags::CHECK_LOAD));
    run_modules(&platform, [select(), right(), right(), select()]).unwrap();
    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.module_slots[0], ModuleType::FuelTank as u8);
    assert_eq!(sis.res_units, 500);
}

// --- Errors ---------------------------------------------------------------

#[test]
fn a_gauge_that_does_not_apply_the_price_is_an_error_and_releases_everything() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    platform.corrupt_res_units.set(true);
    let result = run_modules(&platform, [select(), right(), right(), select()]);
    assert_eq!(
        result,
        Err(StarbaseError::ResUnitsMismatch {
            expected: 500,
            actual: 501
        })
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn a_drive_slot_with_a_foreign_byte_is_an_error_not_a_guess() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    platform.sis.borrow_mut().drive_slots[3] = 21;
    let result = run_modules(&platform, []);
    assert_eq!(
        result,
        Err(StarbaseError::ModuleChooser(
            ModuleChooserError::DriveSlot {
                index: 3,
                value: 21
            }
        ))
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn offering_from_a_price_list_with_nothing_for_sale_is_an_error() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    platform.sis.borrow_mut().module_cost = [0; 20];
    let result = run_modules(&platform, [select(), right()]);
    assert!(matches!(
        result,
        Err(StarbaseError::ModuleChooser(
            ModuleChooserError::NothingToOffer { slot: 0, .. }
        ))
    ));
    assert_nothing_leaked(&platform);
}

// --- Fonts and observation ------------------------------------------------

#[test]
fn pc_fonts_redraw_the_flagship_statistics_after_a_change() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    platform.font_style.set(UiStyle::Pc);
    run_modules(&platform, [select(), right(), right(), select()]).unwrap();
    let events = platform.events();
    let write = position(&events, |e| matches!(e, Event::SetModuleSlot(..)));
    // The statistics block sets its font effects to 4 and 5.
    assert!(events[write..].contains(&Event::SetFontEffect(Some(4))));
    assert!(events[write..].contains(&Event::SetFontEffect(Some(5))));

    let platform = RecordingPlatform::with_new_game_flagship(1000);
    run_modules(&platform, [select(), right(), right(), select()]).unwrap();
    let events = platform.events();
    let write = position(&events, |e| matches!(e, Event::SetModuleSlot(..)));
    assert!(!events[write..].contains(&Event::SetFontEffect(Some(4))));
}

#[test]
fn the_whole_starbase_visit_buys_a_module_and_is_observed_as_docked_throughout() {
    let platform = RecordingPlatform::with_new_game_flagship(1000);
    let before = platform.sis.borrow().clone();
    platform
        .states
        .borrow_mut()
        .insert(StateKey::StarbaseVisited, 1);

    let mut keys = vec![idle(), right(), select()];
    keys.extend(module_route([select(), right(), right(), select()]));
    keys.extend([idle(), right(), right(), select()]);
    platform.push_pulses(keys);

    run_starbase_menu(&platform).unwrap();

    let expected = with(&before, |sis| {
        sis.module_slots[0] = ModuleType::FuelTank as u8;
        sis.res_units = 500;
    });
    assert_eq!(*platform.sis.borrow(), expected);
    assert_eq!(platform.state(StateKey::StarbaseVisited), 0);
    assert!(platform.script.borrow().is_empty());
    assert_nothing_leaked(&platform);

    let docked: Vec<_> = platform
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::Docked(active) => Some(*active),
            _ => None,
        })
        .collect();
    assert_eq!(docked, [true, false]);
}

#[test]
fn every_frame_the_chooser_draws_exists_in_the_art() {
    let platform = RecordingPlatform::with_new_game_flagship(0);
    // Walk every module on offer for a module slot, then offer each other
    // kind of slot's piece.
    let mut keys = vec![select()];
    keys.extend([right(); 11]);
    keys.extend([cancel(), up(), right(), select(), cancel()]);
    keys.extend([up(), select(), cancel()]);
    keys.extend([up(), right(), select(), cancel()]);
    run_modules(&platform, keys).unwrap();

    let events = platform.events();
    let menu_set = loaded(&events, MODULES_PMAP_ANIM) / 1000;
    let module_set = loaded(&events, SISMODS_MASK_PMAP_ANIM) / 1000;
    let menu_frames = modules_menu_frames();
    let module_frames = outfit_modules_frames();
    for event in &events {
        if let Event::DrawStamp(stamp) = event {
            let index = stamp.frame % 1000;
            if stamp.frame / 1000 == menu_set {
                assert!(index < menu_frames, "menu frame {index}");
            } else if stamp.frame / 1000 == module_set {
                assert!(index < module_frames, "module frame {index}");
            }
        }
    }

    // What the chooser offers are the purchasable pieces of the C enum.
    let purchasable = PieceNumbers::from_schema().get("NUM_PURCHASE_MODULES") as usize;
    assert!(shown_modules(&platform)
        .iter()
        .all(|piece| *piece < purchasable));
    assert!(!shown_modules(&platform).is_empty());
}
