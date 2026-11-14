//! Clock entry points and save-compatible event scheduling.

#[cfg(test)]
#[path = "clock_abi/owner_tests.rs"]
mod owner_tests;

use super::clock_bridge::days_in_month;
use crate::engine::battle::link_queue::LinkId;
use crate::engine::clock::{GameClock, ScheduledEvent};

pub(crate) fn event_date(
    now: (u16, u16, u16),
    relative: bool,
    date: (u16, u16, u16),
) -> (u16, u16, u16) {
    let (mut year, mut month, mut day) = date;
    if relative {
        month = month.wrapping_add(now.1 - 1);
        year = year.wrapping_add(now.0).wrapping_add(month / 12);
        month = month % 12 + 1;
        day = day.wrapping_add(now.2);
        while day > days_in_month(month as u8, year) as u16 {
            day -= days_in_month(month as u8, year) as u16;
            month += 1;
            if month > 12 {
                month = 1;
                year = year.wrapping_add(1);
            }
        }
    }
    (year, month, day)
}

pub(crate) fn insert_event(
    clock: &mut GameClock,
    date: (u16, u16, u16),
    selector: u8,
) -> Option<LinkId<ScheduledEvent>> {
    let mut queue = clock.events();
    let new = queue.alloc()?;
    let event = queue.get_mut(new);
    event.year_index = date.0;
    event.month_index = date.1 as u8;
    event.day_index = date.2 as u8;
    event.func_index = selector;
    let mut at = queue.head();
    while let Some(id) = at {
        let current = queue.get(id);
        if date
            < (
                current.year_index,
                u16::from(current.month_index),
                u16::from(current.day_index),
            )
        {
            break;
        }
        at = queue.succ(id);
    }
    queue.insert_before(new, at);
    Some(new)
}

fn take_day_event(
    clock: &mut GameClock,
    today: (u16, u16, u16),
) -> Option<(LinkId<ScheduledEvent>, u8)> {
    let mut queue = clock.events();
    let id = queue.head()?;
    let event = queue.get(id);
    let date = (
        event.year_index,
        u16::from(event.month_index),
        u16::from(event.day_index),
    );
    if date > today {
        return None;
    }
    let selector = event.func_index;
    queue.remove(id);
    Some((id, selector))
}

pub(crate) fn schedule_event(
    world: &mut crate::engine::World,
    dispatch: &mut impl FnMut(&mut crate::engine::World, u8),
    kind: u32,
    month: u16,
    day: u16,
    year: u16,
    selector: u8,
) -> Option<LinkId<ScheduledEvent>> {
    if kind == 1 && month == 0 && day == 0 && year == 0 {
        dispatch(world, selector);
    } else {
        let clock = &world.data.game.game_clock;
        let now = (
            clock.year_index,
            u16::from(clock.month_index),
            u16::from(clock.day_index),
        );
        let date = event_date(now, kind == 1, (year, month, day));
        if date >= now {
            return insert_event(&mut world.clock, date, selector);
        }
    }
    None
}

pub fn add_event(
    cx: &mut crate::engine::battle::Cx<'_>,
    kind: u32,
    month: u16,
    day: u16,
    year: u16,
    selector: u8,
) -> Option<LinkId<ScheduledEvent>> {
    let mut random = cx.rng;
    let mut dispatch = crate::navigation::events::dispatcher(cx.screen_dimensions(), &mut random);
    schedule_event(cx.world, &mut dispatch, kind, month, day, year, selector)
}

fn dispatch_day_events(
    world: &mut crate::engine::World,
    today: (u16, u16, u16),
    mut dispatch: impl FnMut(&mut crate::engine::World, u8),
) {
    while let Some((id, selector)) = take_day_event(&mut world.clock, today) {
        dispatch(world, selector);
        world.clock.events().free(id);
    }
}

fn finish_clock_day(cx: &mut crate::engine::battle::Cx<'_>) {
    crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
    let clock = &cx.world.data.game.game_clock;
    let today = (
        clock.year_index,
        u16::from(clock.month_index),
        u16::from(clock.day_index),
    );
    let mut random = cx.rng;
    let dispatch = crate::navigation::events::dispatcher(cx.screen_dimensions(), &mut random);
    dispatch_day_events(cx.world, today, dispatch);
}

pub fn game_clock_tick(cx: &mut crate::engine::battle::Cx<'_>) {
    if super::clock_bridge::rust_clock_tick(cx.world) {
        finish_clock_day(cx);
    }
}

pub fn advance_days(cx: &mut crate::engine::battle::Cx<'_>, days: i32) {
    super::clock_bridge::log_clock_bridge(&format!("RUST_CLOCK_MOVE: {} days", days));
    for _ in 0..days {
        super::clock_bridge::next_clock_day_internal(&mut cx.world.data.game.game_clock);
        finish_clock_day(cx);
    }
    let clock = &mut cx.world.data.game.game_clock;
    clock.tick_count = clock.day_in_ticks;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_events_are_removed_before_callbacks_and_preserve_order() {
        let mut clock = GameClock::new();
        insert_event(&mut clock, (2155, 3, 17), 0);
        insert_event(&mut clock, (2156, 3, 17), 1);
        insert_event(&mut clock, (2156, 3, 17), 2);
        insert_event(&mut clock, (2156, 3, 18), 3);
        let mut seen = Vec::new();
        while let Some((id, selector)) = take_day_event(&mut clock, (2156, 3, 17)) {
            assert!(!clock.events_ref().linked(id));
            seen.push(selector);
            if selector == 1 {
                insert_event(&mut clock, (2156, 3, 17), 4);
            }
            clock.events().free(id);
        }
        assert_eq!(seen, [0, 1, 2, 4]);
        let queue = clock.events_ref();
        assert_eq!(queue.get(queue.head().unwrap()).func_index, 3);
        while let Some((id, selector)) = take_day_event(&mut clock, (2156, 3, 18)) {
            seen.push(selector);
            clock.events().free(id);
        }
        assert_eq!(seen, [0, 1, 2, 4, 3]);
        assert!(clock.events_ref().head().is_none());
    }

    #[test]
    fn relative_date_carries_months_days_and_leap_years() {
        assert_eq!(event_date((2155, 12, 31), true, (0, 0, 1)), (2156, 1, 1));
        assert_eq!(event_date((2156, 1, 31), true, (0, 1, 0)), (2156, 3, 2));
        assert_eq!(event_date((2156, 2, 28), true, (0, 0, 1)), (2156, 2, 29));
        assert_eq!(event_date((2155, 2, 17), false, (2154, 1, 1)), (2154, 1, 1));
    }

    #[test]
    fn events_are_sorted_and_same_date_events_keep_insertion_order() {
        let mut clock = GameClock::new();
        let late = insert_event(&mut clock, (2156, 1, 1), 1).unwrap();
        let first = insert_event(&mut clock, (2155, 2, 18), 2).unwrap();
        let second = insert_event(&mut clock, (2155, 2, 18), 3).unwrap();
        let middle = insert_event(&mut clock, (2155, 12, 31), 4).unwrap();
        let queue = clock.events_ref();
        assert_eq!(queue.head(), Some(first));
        assert_eq!(queue.succ(first), Some(second));
        assert_eq!(queue.succ(second), Some(middle));
        assert_eq!(queue.succ(middle), Some(late));
        assert_eq!(queue.tail(), Some(late));
        for _ in 4..crate::engine::clock::NUM_EVENTS {
            assert!(insert_event(&mut clock, (2156, 1, 1), 5).is_some());
        }
        assert!(insert_event(&mut clock, (2155, 1, 1), 6).is_none());
    }
}
