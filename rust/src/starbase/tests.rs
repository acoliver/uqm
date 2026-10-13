//! Behaviour tests of the Starbase and Outfit screens against the recording
//! platform: call order, menu inputs, authoritative fuel and RU after
//! purchases, resource release on every exit, and the drawing commands.

use super::draw::{
    display_landers, draw_base_state_strings, draw_flagship_name, draw_flagship_stats,
    draw_outfit_menu_strings, draw_ship_piece, gauge_rect, redistribute_fuel, rotate_starbase,
    StarbaseAnimation,
};
use super::gfx::{Align, ContextId, FlashTarget, FontId, Point, Rect, Rgba, Stamp};
use super::ids::{
    MODULES_PMAP_ANIM, OUTFIT_MUSIC, OUTFIT_PMAP_ANIM, SISMODS_MASK_PMAP_ANIM, STARBASE_ANIM,
    STARBASE_MUSIC,
};
use super::platform::{
    MenuSounds, Platform, StarbaseError, StateKey, StatusMessageMode, UiStyle, UnportedRoute,
    ONE_SECOND,
};
use super::screen::{
    cleanup_after_visit, do_time_passage, run_outfit, run_starbase_menu, OutfitScreen,
    StarbaseScreen, StatusModeGuard,
};
use super::sis_modules::{fuel_bar_layout, ModuleSlots};
use super::starbase_menu::StarbaseState;
use super::test_support::{cancel, idle, left, press, right, select, up, Event, RecordingPlatform};
use super::visit::{entry_time_passage, TimePassage};
use crate::mainloop::types::{activity_flags, ActivityValue};

fn texts(events: &[Event]) -> Vec<(&str, i16, i16, Align)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Text(t) => Some((t.text.as_str(), t.baseline.x, t.baseline.y, t.align)),
            _ => None,
        })
        .collect()
}

fn count(events: &[Event], wanted: impl Fn(&Event) -> bool) -> usize {
    events.iter().filter(|event| wanted(event)).count()
}

fn position(events: &[Event], wanted: impl Fn(&Event) -> bool) -> usize {
    events
        .iter()
        .position(wanted)
        .expect("expected event was recorded")
}

fn assert_nothing_leaked(platform: &RecordingPlatform) {
    let (frames, music) = platform.leaked_resources();
    assert!(frames.is_empty(), "frames leaked: {frames:?}");
    assert!(music.is_empty(), "music leaked: {music:?}");
}

// --- Starbase menu drawing ---------------------------------------------

#[test]
fn first_draw_of_base_strings_skips_the_highlighted_entry_and_draws_depart() {
    let platform = RecordingPlatform::new();
    draw_base_state_strings(&platform, None, StarbaseState::TalkCommander);
    let events = platform.events();

    assert_eq!(
        texts(&events),
        vec![
            ("S424", 69, 157, Align::Center),
            ("S425", 69, 176, Align::Center),
            ("S426", 69, 195, Align::Center),
            ("S423", 69, 138, Align::Center),
        ]
    );
    // The unselected entries are drawn black, the selected one yellow.
    let yellow = position(&events, |e| {
        *e == Event::SetFg(Rgba::rgb15(0x1F, 0x1F, 0x0A))
    });
    let last_text = position(&events, |e| matches!(e, Event::Text(t) if t.text == "S423"));
    assert!(yellow < last_text);
    assert_eq!(events[0], Event::SetContext(ContextId::Screen));
    assert_eq!(events[1], Event::SetFont(FontId::StarCon));
    assert_eq!(events[2], Event::SetFg(Rgba::BLACK));
}

#[test]
fn highlight_redraws_the_old_entry_black_and_the_new_one_yellow() {
    let platform = RecordingPlatform::new();
    draw_base_state_strings(
        &platform,
        Some(StarbaseState::TalkCommander),
        StarbaseState::OutfitStarship,
    );
    let events = platform.events();
    assert_eq!(
        texts(&events),
        vec![
            ("S423", 69, 138, Align::Center),
            ("S424", 69, 157, Align::Center)
        ]
    );
}

#[test]
fn starbase_animation_draws_first_frame_then_paces_at_one_twentieth_second() {
    let platform = RecordingPlatform::new();
    let mut animation = StarbaseAnimation::default();
    rotate_starbase(&platform, &mut animation, Some(3000), 100);
    rotate_starbase(&platform, &mut animation, None, 100 + ONE_SECOND / 20 - 1);
    rotate_starbase(&platform, &mut animation, None, 100 + ONE_SECOND / 20);
    let stamps: Vec<_> = platform
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::DrawStamp(s) => Some(s),
            _ => None,
        })
        .collect();
    // Frame 0 first, then frame 1 once the delay passed; never in between.
    assert_eq!(
        stamps,
        vec![
            Stamp {
                origin: Point { x: 0, y: 4 },
                frame: 3000
            },
            Stamp {
                origin: Point { x: 0, y: 4 },
                frame: 3001
            },
        ]
    );
}

#[test]
fn starbase_animation_loops_back_to_frame_one_not_the_base_frame() {
    let platform = RecordingPlatform::new();
    let mut animation = StarbaseAnimation::default();
    rotate_starbase(&platform, &mut animation, Some(3063), 0);
    rotate_starbase(&platform, &mut animation, None, 10_000);
    let frames: Vec<_> = platform
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::DrawStamp(s) => Some(s.frame),
            _ => None,
        })
        .collect();
    assert_eq!(frames, vec![3063, 3001]);
}

// --- Starbase menu frames ----------------------------------------------

#[test]
fn starbase_first_frame_loads_art_and_music_draws_and_plays() {
    let platform = RecordingPlatform::new();
    platform
        .last_activity
        .set(ActivityValue::new(activity_flags::CHECK_LOAD | 0x0105));
    let mut screen = StarbaseScreen::new(&platform);

    assert_eq!(screen.frame(), Ok(true));
    let events = platform.events();

    let load_art = position(
        &events,
        |e| matches!(e, Event::LoadGraphic(n, _) if n == STARBASE_ANIM),
    );
    let load_music = position(
        &events,
        |e| matches!(e, Event::LoadMusic(n, _) if n == STARBASE_MUSIC),
    );
    let transition = position(&events, |e| *e == Event::ScreenTransition(3));
    let play = position(&events, |e| matches!(e, Event::PlayMusic(_)));
    assert!(load_art < load_music && load_music < transition && transition < play);
    assert!(events.contains(&Event::SetLastActivity(ActivityValue::new(0x0105))));
    assert!(events.contains(&Event::Flash(FlashTarget::Off)));
    assert!(events.contains(&Event::ClearDrawable));
    // Batch and unbatch are balanced around the first draw.
    assert_eq!(
        count(&events, |e| *e == Event::Batch),
        count(&events, |e| *e == Event::Unbatch)
    );
    assert!(events.contains(&Event::MenuSounds(MenuSounds::ARROWS_SELECT)));
    // The animation base frame is the first stamp, before any text.
    let stamp = position(&events, |e| matches!(e, Event::DrawStamp(_)));
    let first_text = position(&events, |e| matches!(e, Event::Text(_)));
    assert!(stamp < first_text);
}

#[test]
fn idle_starbase_frames_animate_and_sleep_one_thirtieth_second() {
    let platform = RecordingPlatform::new();
    let mut screen = StarbaseScreen::new(&platform);
    assert_eq!(screen.frame(), Ok(true));
    let before = platform.events().len();
    assert_eq!(screen.frame(), Ok(true));
    let events = platform.events()[before..].to_vec();
    assert_eq!(events.last(), Some(&Event::Sleep(ONE_SECOND / 30)));
    assert!(events.contains(&Event::Batch));
    assert!(events.contains(&Event::Unbatch));
}

#[test]
fn left_from_the_first_entry_wraps_to_depart_and_redraws_the_strings() {
    let platform = RecordingPlatform::new();
    platform.push_pulses([idle(), left()]);
    let mut screen = StarbaseScreen::new(&platform);
    // The scripted keys are read through the loop, so feed frames by hand.
    for _ in 0..2 {
        let pulses = platform.script.borrow_mut().pop_front().unwrap();
        platform.set_pulses(pulses);
        assert_eq!(screen.frame(), Ok(true));
    }
    let events = platform.events();
    let last_two: Vec<_> = texts(&events).into_iter().rev().take(2).collect();
    // Old entry (Talk) black, new entry (Depart) yellow.
    assert_eq!(last_two[0].0, "S426");
    assert_eq!(last_two[1].0, "S423");
}

#[test]
fn talk_releases_art_then_flushes_input_runs_conversation_and_restores_the_mark() {
    let platform = RecordingPlatform::new();
    // init, select Talk, re-init, then leave through abort.
    platform.push_pulses([idle(), select(), idle(), idle()]);
    platform.abort_after_pulses.set(Some(4));
    run_starbase_menu(&platform).unwrap();
    let events = platform.events();

    let destroy = position(&events, |e| matches!(e, Event::DestroyDrawable(_)));
    let flush = position(&events, |e| *e == Event::FlushInput);
    let conversation = position(&events, |e| *e == Event::Conversation);
    let mark = position(&events, |e| {
        *e == Event::SetState(StateKey::GlobalFlagsAndData, 0xFF)
    });
    assert!(destroy < flush && flush < conversation && conversation < mark);
    assert_eq!(platform.state(StateKey::GlobalFlagsAndData), 0xFF);
    // The menu came back: art was loaded twice.
    assert_eq!(
        count(
            &events,
            |e| matches!(e, Event::LoadGraphic(n, _) if n == STARBASE_ANIM)
        ),
        2
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn depart_clears_starbase_visited_and_releases_everything() {
    let platform = RecordingPlatform::new();
    platform
        .states
        .borrow_mut()
        .insert(StateKey::StarbaseVisited, 1);
    platform.push_pulses([idle(), left(), select()]);
    run_starbase_menu(&platform).unwrap();

    assert_eq!(platform.state(StateKey::StarbaseVisited), 0);
    let events = platform.events();
    assert!(events.contains(&Event::SetState(StateKey::StarbaseVisited, 0)));
    assert_nothing_leaked(&platform);
    // The screen context is restored when the menu ends.
    let tail = &events[events.len() - 2..];
    assert_eq!(tail, [Event::RestoreContext, Event::Docked(false)]);
}

#[test]
fn abort_and_load_end_the_menu_without_clearing_starbase_visited() {
    for flag in [activity_flags::CHECK_ABORT, activity_flags::CHECK_LOAD] {
        let platform = RecordingPlatform::new();
        platform
            .states
            .borrow_mut()
            .insert(StateKey::StarbaseVisited, 1);
        platform.push_pulses([idle(), idle()]);
        let mut screen = StarbaseScreen::new(&platform);
        assert_eq!(screen.frame(), Ok(true));
        platform.activity.set(ActivityValue::new(flag));
        assert_eq!(screen.frame(), Ok(false));

        assert_eq!(platform.state(StateKey::StarbaseVisited), 1);
        drop(screen);
        assert_nothing_leaked(&platform);
    }
}

#[test]
fn shipyard_fails_fast_with_a_typed_error_and_releases_resources() {
    let platform = RecordingPlatform::new();
    platform.push_pulses([idle(), right(), right(), select()]);
    let result = run_starbase_menu(&platform);

    assert_eq!(
        result,
        Err(StarbaseError::Unported(UnportedRoute::Shipyard))
    );
    assert_nothing_leaked(&platform);
    // No stand-in sound, no success marker.
    assert!(!platform.events().contains(&Event::FailureSound));
    assert_eq!(platform.state(StateKey::StarbaseVisited), 0);
}

#[test]
fn art_load_failure_releases_what_was_already_loaded() {
    let platform = RecordingPlatform::new();
    *platform.failing_resource.borrow_mut() = Some(STARBASE_MUSIC.to_owned());
    platform.push_pulses([idle()]);
    let result = run_starbase_menu(&platform);
    assert_eq!(
        result,
        Err(StarbaseError::ResourceLoad(STARBASE_MUSIC.to_owned()))
    );
    assert_nothing_leaked(&platform);
}

// --- Outfit: drawing ------------------------------------------------------

#[test]
fn fuel_bars_show_filled_and_empty_cells_for_the_fuel_on_board() {
    let platform = RecordingPlatform::new();
    let mut sis = platform.sis.borrow().clone();
    sis.fuel_on_board = 3500;
    let slots = ModuleSlots::from_raw(&sis.module_slots).unwrap();
    let layout = fuel_bar_layout(&slots, sis.fuel_on_board).unwrap();
    redistribute_fuel(&platform, &layout);
    let events = platform.events();

    let fills: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::FillRect(r) => Some(*r),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 5);
    // Levels 1000, 2000 and 3000 are below 3500: three filled bars.
    assert_eq!(fills.iter().filter(|r| r.width == 3).count(), 3);
    assert_eq!(fills.iter().filter(|r| r.width == 5).count(), 2);
    assert_eq!(count(&events, |e| matches!(e, Event::DrawPoint(_))), 6);
    assert_eq!(events.first(), Some(&Event::SetContext(ContextId::Space)));
    assert_eq!(events.last(), Some(&Event::RestoreContext));
}

#[test]
fn drive_piece_draws_top_then_side_view_at_the_drive_slot() {
    let platform = RecordingPlatform::new();
    let sis = platform.sis.borrow().clone();
    draw_ship_piece(&platform, &sis, 5000, 1, 2, false);
    let stamps: Vec<_> = platform
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::DrawStamp(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(
        stamps,
        vec![
            Stamp {
                origin: Point { x: 33 + 24, y: 86 },
                frame: 5001
            },
            Stamp {
                origin: Point { x: 31 + 24, y: 56 },
                frame: 5026
            },
        ]
    );
}

#[test]
fn first_module_slot_piece_draws_its_frame_borders_and_the_next_slot() {
    let platform = RecordingPlatform::new();
    let sis = platform.sis.borrow().clone();
    draw_ship_piece(&platform, &sis, 5000, 5, 0, false);
    let events = platform.events();
    assert_eq!(count(&events, |e| matches!(e, Event::FillRect(_))), 12);
    assert_eq!(count(&events, |e| matches!(e, Event::DrawStamp(_))), 4);
    assert_eq!(events[0], Event::SetFg(Rgba::BLACK));
}

#[test]
fn landers_draw_present_ones_as_sprites_and_missing_ones_as_black_shapes() {
    let platform = RecordingPlatform::new();
    let sis = platform.sis.borrow().clone();
    display_landers(&platform, &sis, 0, 6000);
    let events = platform.events();
    assert_eq!(count(&events, |e| matches!(e, Event::DrawStamp(_))), 3);
    assert_eq!(
        count(&events, |e| matches!(e, Event::DrawFilledStamp(_))),
        7
    );
    assert!(events.contains(&Event::DrawFilledStamp(Stamp {
        origin: Point {
            x: 24 + 3 * 15,
            y: 67
        },
        frame: 6000,
    })));
}

#[test]
fn threedo_fuel_menu_draws_the_art_frame_and_the_fuel_cost() {
    let platform = RecordingPlatform::new();
    draw_outfit_menu_strings(&platform, UiStyle::ThreeDo, 0, 20);
    let events = platform.events();
    let radar_x = 4 + (320 - 64) - 250;
    let radar_y = 10 + (240 - 13) - 53;

    assert_eq!(events.first(), Some(&Event::PreUpdateFlash));
    assert_eq!(events.last(), Some(&Event::PostUpdateFlash));
    assert!(events.contains(&Event::FillRect(Rect {
        corner: Point {
            x: radar_x - 1,
            y: radar_y - 11
        },
        width: 58,
        height: 11,
    })));
    assert!(events.contains(&Event::DrawStamp(Stamp {
        origin: Point {
            x: radar_x,
            y: radar_y
        },
        frame: 9021,
    })));
    assert_eq!(
        texts(&events),
        vec![("20", radar_x + 56 - 2, radar_y + 53 - 2, Align::Right)]
    );
}

#[test]
fn threedo_menu_shows_the_cost_only_on_the_fuel_entry() {
    let platform = RecordingPlatform::new();
    draw_outfit_menu_strings(&platform, UiStyle::ThreeDo, 3, 20);
    let events = platform.events();
    assert!(texts(&events).is_empty());
    assert!(events.contains(&Event::DrawStamp(Stamp {
        origin: Point { x: 10, y: 184 },
        frame: 9024,
    })));
}

#[test]
fn pc_fuel_menu_draws_a_framed_list_with_the_cost_in_the_fuel_label() {
    let platform = RecordingPlatform::new();
    draw_outfit_menu_strings(&platform, UiStyle::Pc, 1, 20);
    let events = platform.events();
    let labels: Vec<_> = texts(&events).into_iter().map(|t| t.0.to_owned()).collect();
    assert_eq!(labels, vec!["S526(20)", "S527", "S528", "S529"]);
    assert_eq!(count(&events, |e| matches!(e, Event::DrawRect(_))), 2);
    // The art stamp is not drawn in the PC hierarchy.
    assert_eq!(count(&events, |e| matches!(e, Event::DrawStamp(_))), 0);
    // The selected entry gets a highlight bar before its text.
    let highlight = position(&events, |e| {
        *e == Event::SetFg(Rgba::rgb15(0x0A, 0x0A, 0x1F))
    });
    let selected = position(&events, |e| matches!(e, Event::Text(t) if t.text == "S527"));
    assert!(highlight < selected);
}

#[test]
fn flagship_name_is_the_uppercase_title_over_the_ship_name() {
    let platform = RecordingPlatform::new();
    let sis = platform.sis.borrow().clone();
    draw_flagship_name(&platform, UiStyle::ThreeDo, &sis);
    let events = platform.events();
    assert_eq!(
        texts(&events),
        vec![("S442 VINDICATOR", 121, 8, Align::Center)]
    );
    assert!(events.contains(&Event::FillRect(Rect {
        corner: Point { x: 0, y: 1 },
        width: 242,
        height: 7,
    })));
    assert!(events.contains(&Event::SetFg(Rgba::rgb15(0x14, 0x0A, 0x00))));
}

#[test]
fn flagship_stats_report_thrust_turning_energy_and_fuel() {
    let platform = RecordingPlatform::new();
    let sis = platform.sis.borrow().clone();
    draw_flagship_stats(&platform, &sis);
    let events = platform.events();
    let drawn = texts(&events);
    let values: Vec<_> = drawn
        .iter()
        .filter(|t| t.3 == Align::Right && t.1 == 240)
        .map(|t| t.0)
        .collect();
    // Six thrusters: (10 + 12) * 4; four jets: 1 + 17 - 9; energy 2.2 per
    // 10 seconds; one tank: 60 units.
    assert_eq!(values, vec!["  88", "   9", " 2.2", "  60"]);
}

// --- Outfit: frames and fuel purchases -----------------------------------

fn outfit_with_script(
    platform: &RecordingPlatform,
    keys: impl IntoIterator<Item = super::input::MenuPulses>,
) -> Result<(), StarbaseError> {
    platform.push_pulses(keys);
    run_outfit(platform)
}

fn buy_fuel_script(presses: usize) -> Vec<super::input::MenuPulses> {
    let mut keys = vec![idle(), select()];
    keys.extend(std::iter::repeat_with(up).take(presses));
    keys.push(select());
    keys.extend([right(), right(), right(), select()]);
    keys
}

#[test]
fn fifty_unit_purchases_end_at_a_full_tank_and_the_exact_remaining_ru() {
    let platform = RecordingPlatform::new();
    outfit_with_script(&platform, buy_fuel_script(51)).unwrap();

    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.fuel_on_board, 6000);
    assert_eq!(sis.res_units, 1500);

    let events = platform.events();
    let deltas = events
        .iter()
        .filter(|e| matches!(e, Event::GaugeDelta(d) if d.fuel == 100 && d.res_units == -20))
        .count();
    assert_eq!(deltas, 50);
    // The 51st press found a full tank.
    assert_eq!(count(&events, |e| *e == Event::FailureSound), 1);
    assert_nothing_leaked(&platform);
}

#[test]
fn purchases_stop_when_the_ru_run_out() {
    let platform = RecordingPlatform::new();
    platform.sis.borrow_mut().res_units = 100;
    outfit_with_script(&platform, buy_fuel_script(6)).unwrap();

    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.fuel_on_board, 1500);
    assert_eq!(sis.res_units, 0);
    assert_eq!(count(&platform.events(), |e| *e == Event::FailureSound), 1);
}

#[test]
fn down_key_sells_fuel_back_at_the_unit_price() {
    let platform = RecordingPlatform::new();
    let keys = vec![
        idle(),
        select(),
        press(|p| p.down = true),
        select(),
        right(),
        right(),
        right(),
        select(),
    ];
    outfit_with_script(&platform, keys).unwrap();
    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.fuel_on_board, 900);
    assert_eq!(sis.res_units, 2520);
}

#[test]
fn page_keys_buy_a_whole_bar_at_a_time() {
    let platform = RecordingPlatform::new();
    let keys = vec![
        idle(),
        select(),
        press(|p| p.page_up = true),
        select(),
        right(),
        right(),
        right(),
        select(),
    ];
    outfit_with_script(&platform, keys).unwrap();
    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.fuel_on_board, 2000);
    assert_eq!(sis.res_units, 2500 - 200);
}

#[test]
fn nothing_is_bought_without_entering_fuel_adjust() {
    let platform = RecordingPlatform::new();
    let page = press(|p| p.page_up = true);
    let keys = vec![idle(), page, page, right(), right(), right(), select()];
    outfit_with_script(&platform, keys).unwrap();
    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.fuel_on_board, 1000);
    assert_eq!(sis.res_units, 2500);
    assert!(!platform
        .events()
        .iter()
        .any(|e| matches!(e, Event::GaugeDelta(_))));
}

#[test]
fn cancel_leaves_fuel_adjust_before_it_leaves_the_screen() {
    let platform = RecordingPlatform::new();
    let keys = vec![idle(), select(), up(), cancel(), cancel()];
    outfit_with_script(&platform, keys).unwrap();
    let events = platform.events();
    // Menu flash at entry, gauge flash while adjusting, menu flash again
    // when cancel leaves the adjust state.
    let flashes: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Flash(target) => Some(*target),
            _ => None,
        })
        .collect();
    assert_eq!(
        flashes[..3],
        [
            FlashTarget::Menu3do,
            FlashTarget::Rect(gauge_rect(false)),
            FlashTarget::Rect(gauge_rect(false)),
        ]
    );
    assert_eq!(flashes.last(), Some(&FlashTarget::Menu3do));
    assert_eq!(platform.sis.borrow().fuel_on_board, 1100);
    assert_nothing_leaked(&platform);
}

#[test]
fn entry_rounds_fuel_to_the_nearest_unit_after_the_first_draw() {
    for (before, after) in [(1049, 1000), (1050, 1100), (5999, 6000)] {
        let platform = RecordingPlatform::new();
        platform.sis.borrow_mut().fuel_on_board = before;
        let mut screen = OutfitScreen::new(&platform);
        assert_eq!(screen.frame(), Ok(true));
        let events = platform.events();
        assert!(events.contains(&Event::SetFuelOnBoard(after)));
        let flash = position(&events, |e| *e == Event::Flash(FlashTarget::Menu3do));
        let round = position(&events, |e| matches!(e, Event::SetFuelOnBoard(_)));
        assert!(flash < round);
        assert_eq!(events.last(), Some(&Event::SetContext(ContextId::Status)));
    }
}

#[test]
fn outfit_first_frame_loads_its_art_and_draws_the_screen_in_order() {
    let platform = RecordingPlatform::new();
    let mut screen = OutfitScreen::new(&platform);
    assert_eq!(screen.frame(), Ok(true));
    let events = platform.events();

    let loads: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::LoadGraphic(n, _) | Event::LoadMusic(n, _) => Some(n.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        loads,
        vec![
            MODULES_PMAP_ANIM,
            OUTFIT_MUSIC,
            SISMODS_MASK_PMAP_ANIM,
            OUTFIT_PMAP_ANIM
        ]
    );
    let frame = position(&events, |e| *e == Event::SisFrame);
    let message = position(&events, |e| *e == Event::SisMessage("S424".to_owned()));
    let title = position(&events, |e| *e == Event::SisTitle("S422".to_owned()));
    let transition = position(&events, |e| *e == Event::ScreenTransition(3));
    let music = position(&events, |e| matches!(e, Event::PlayMusic(_)));
    assert!(frame < message && message < title && title < transition && transition < music);
    // The background art is drawn once and released at once.
    assert_eq!(
        count(&events, |e| matches!(e, Event::DestroyDrawable(_))),
        1
    );
    assert_eq!(
        count(&events, |e| *e == Event::Batch),
        count(&events, |e| *e == Event::Unbatch)
    );
}

#[test]
fn fuel_adjust_uses_its_own_menu_sounds_and_paces_each_frame() {
    let platform = RecordingPlatform::new();
    platform.push_pulses([idle(), select(), idle(), up()]);
    let mut screen = OutfitScreen::new(&platform);
    for _ in 0..4 {
        let pulses = platform.script.borrow_mut().pop_front().unwrap();
        platform.set_pulses(pulses);
        assert_eq!(screen.frame(), Ok(true));
    }
    let events = platform.events();
    assert!(events.contains(&Event::MenuSounds(MenuSounds::FUEL_ADJUST)));
    assert_eq!(count(&events, |e| *e == Event::Sleep(ONE_SECOND / 30)), 2);
}

#[test]
fn menu_moves_redraw_the_menu_strings_and_sleep_a_twentieth_second() {
    let platform = RecordingPlatform::new();
    let mut screen = OutfitScreen::new(&platform);
    assert_eq!(screen.frame(), Ok(true));
    let before = platform.events().len();
    platform.set_pulses(right());
    assert_eq!(screen.frame(), Ok(true));
    let events = platform.events()[before..].to_vec();
    assert_eq!(
        events.first(),
        Some(&Event::MenuSounds(MenuSounds::ARROWS_SELECT))
    );
    assert!(events.contains(&Event::DrawStamp(Stamp {
        origin: Point { x: 10, y: 184 },
        frame: 9022,
    })));
    assert_eq!(events.last(), Some(&Event::Sleep(ONE_SECOND / 20)));
}

#[test]
fn gauge_update_that_disagrees_with_the_transaction_is_an_error() {
    let platform = RecordingPlatform::new();
    platform.corrupt_gauge.set(true);
    let result = outfit_with_script(&platform, [idle(), select(), up()]);
    assert_eq!(
        result,
        Err(StarbaseError::GaugeMismatch {
            expected_fuel: 1100,
            expected_res_units: 2480,
            actual_fuel: 1200,
            actual_res_units: 2480,
        })
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn module_installation_opens_its_chooser_and_save_load_fails_fast() {
    // Modules is ported: selecting it opens the chooser, and cancelling the
    // chooser and the Outfit screen ends the visit without an error. The
    // purchases and removals are proved in `module_route_tests`.
    let platform = RecordingPlatform::new();
    let result = outfit_with_script(&platform, [idle(), right(), select(), cancel(), cancel()]);
    assert_eq!(result, Ok(()));
    assert_nothing_leaked(&platform);

    let platform = RecordingPlatform::new();
    let result = outfit_with_script(&platform, [idle(), right(), right(), select()]);
    assert_eq!(
        result,
        Err(StarbaseError::Unported(UnportedRoute::SaveLoad))
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn outfit_art_load_failure_releases_art_loaded_before_it() {
    let platform = RecordingPlatform::new();
    *platform.failing_resource.borrow_mut() = Some(OUTFIT_PMAP_ANIM.to_owned());
    let result = outfit_with_script(&platform, [idle()]);
    assert_eq!(
        result,
        Err(StarbaseError::ResourceLoad(OUTFIT_PMAP_ANIM.to_owned()))
    );
    assert_nothing_leaked(&platform);
}

#[test]
fn abort_during_outfit_releases_everything() {
    let platform = RecordingPlatform::new();
    platform.abort_after_pulses.set(Some(3));
    outfit_with_script(&platform, [idle(), select(), up(), up()]).unwrap();
    assert_nothing_leaked(&platform);
}

// --- Whole route: Starbase -> Outfit -> buy fuel -> Depart ---------------

#[test]
fn starbase_to_outfit_to_fuel_purchase_to_depart_changes_only_what_the_player_bought() {
    let platform = RecordingPlatform::new();
    platform
        .states
        .borrow_mut()
        .insert(StateKey::StarbaseVisited, 1);

    let mut keys = vec![idle(), right(), select()];
    keys.extend(buy_fuel_script(51));
    keys.extend([idle(), right(), right(), select()]);
    platform.push_pulses(keys);

    run_starbase_menu(&platform).unwrap();

    let sis = platform.sis.borrow().clone();
    assert_eq!(sis.fuel_on_board, 6000);
    assert_eq!(sis.res_units, 1500);
    assert_eq!(platform.state(StateKey::StarbaseVisited), 0);
    assert!(platform.script.borrow().is_empty());
    assert_nothing_leaked(&platform);

    // Every change to the SIS state came from a gauge delta the player's
    // key presses caused, plus the entry rounding of an already round value.
    let events = platform.events();
    assert_eq!(count(&events, |e| matches!(e, Event::GaugeDelta(_))), 50);
    assert!(events.contains(&Event::SetFuelOnBoard(1000)));
}

// --- Visit tail ----------------------------------------------------------

#[test]
fn time_passage_fades_to_black_then_advances_fourteen_days() {
    let platform = RecordingPlatform::new();
    let passage = entry_time_passage(true, 0).unwrap();
    do_time_passage(&platform, passage);
    let events = platform.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0], Event::Fade(2 * ONE_SECOND));
    assert!(matches!(events[1], Event::SleepUntil(_)));
    assert_eq!(events[2], Event::AdvanceDays(14));
}

#[test]
fn time_passage_uses_the_given_days() {
    let platform = RecordingPlatform::new();
    do_time_passage(
        &platform,
        TimePassage {
            fade_seconds: 1,
            days: 3,
        },
    );
    assert!(platform.events().contains(&Event::AdvanceDays(3)));
}

#[test]
fn cleanup_returns_to_interplanetary_space_through_a_faked_load() {
    let platform = RecordingPlatform::new();
    platform
        .states
        .borrow_mut()
        .insert(StateKey::GlobalFlagsAndData, 0xFF);
    cleanup_after_visit(&platform);
    assert_eq!(platform.state(StateKey::GlobalFlagsAndData), 0);
    assert_eq!(
        platform.current_activity(),
        ActivityValue::new(activity_flags::CHECK_LOAD)
    );
    assert!(platform
        .events()
        .iter()
        .any(|e| matches!(e, Event::SetNextActivity(_))));
}

#[test]
fn cleanup_leaves_a_pending_load_or_abort_alone() {
    for flag in [activity_flags::CHECK_LOAD, activity_flags::CHECK_ABORT] {
        let platform = RecordingPlatform::new();
        platform.activity.set(ActivityValue::new(flag));
        platform
            .states
            .borrow_mut()
            .insert(StateKey::GlobalFlagsAndData, 0xFF);
        cleanup_after_visit(&platform);
        assert_eq!(platform.state(StateKey::GlobalFlagsAndData), 0xFF);
        assert!(platform.events().is_empty());
    }
}

#[test]
fn status_message_mode_is_restored_when_the_guard_ends() {
    let platform = RecordingPlatform::new();
    {
        let _guard = StatusModeGuard::enter(&platform, StatusMessageMode::ResUnits);
    }
    assert_eq!(
        platform.events(),
        vec![
            Event::SetMessageMode(StatusMessageMode::ResUnits),
            Event::SetMessageMode(StatusMessageMode::Date),
        ]
    );
}

#[test]
fn a_new_visit_starts_from_fresh_screen_state() {
    // Two separate visits behave the same: nothing is carried in statics.
    let first = RecordingPlatform::new();
    first.push_pulses([idle(), left(), select()]);
    run_starbase_menu(&first).unwrap();
    let second = RecordingPlatform::new();
    second.push_pulses([idle(), left(), select()]);
    run_starbase_menu(&second).unwrap();
    assert_eq!(first.events(), second.events());
}

#[test]
fn the_menu_is_observed_as_docked_for_the_whole_visit_and_cleared_after() {
    let platform = RecordingPlatform::new();
    platform.push_pulses([idle(), left(), select()]);
    run_starbase_menu(&platform).unwrap();
    let events = platform.events();
    let docked: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Docked(active) => Some(*active),
            _ => None,
        })
        .collect();
    assert_eq!(docked, vec![true, false]);
    assert_eq!(events.first(), Some(&Event::Docked(true)));
    assert_eq!(events.last(), Some(&Event::Docked(false)));
}

#[test]
fn the_docked_mark_is_cleared_when_a_route_fails() {
    let platform = RecordingPlatform::new();
    platform.push_pulses([idle(), right(), right(), select()]);
    assert!(run_starbase_menu(&platform).is_err());
    assert_eq!(platform.events().last(), Some(&Event::Docked(false)));
}

#[test]
fn the_commander_conversation_is_not_observed_as_docked() {
    let platform = RecordingPlatform::new();
    platform.push_pulses([idle(), select(), idle(), left(), select()]);
    run_starbase_menu(&platform).unwrap();
    let events = platform.events();
    let conversation = position(&events, |e| *e == Event::Conversation);
    assert_eq!(events[conversation - 1], Event::Docked(false));
    assert_eq!(events[conversation + 1], Event::Docked(true));
}
