//! Clock entry points and save-compatible event scheduling.

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

#[cfg(not(test))]
pub(crate) mod live {
    use super::super::clock_bridge::*;
    use super::*;
    use std::ptr;

    use crate::navigation::events::live::dispatch_event;

    pub(crate) unsafe fn finish_clock_day(world: &mut crate::engine::World) {
        let clock = &world.data.game.game_clock;
        let today = (
            clock.year_index,
            u16::from(clock.month_index),
            u16::from(clock.day_index),
        );
        while let Some((id, selector)) = take_day_event(&mut world.clock, today) {
            dispatch_event(world, selector);
            world.clock.events().free(id);
        }
    }

    /// # Safety
    /// Game-thread access to the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn GameClockTick() {
        crate::app::bridge::enter(|cx| {
            if rust_clock_tick(cx.world) {
                cx.with_legacy(|| crate::ui::sis::DrawStatusMessage(ptr::null()));
                finish_clock_day(cx.world);
            }
        });
    }
    /// # Safety
    /// Game-thread access to the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn MoveGameClockDays(days: u16) {
        advance_days(i32::from(days));
    }

    /// # Safety
    /// Game-thread entry with a published context and initialized clock.
    pub unsafe fn advance_days(days: i32) {
        log_clock_bridge(&format!("RUST_CLOCK_MOVE: {} days", days));
        crate::app::bridge::enter(|cx| {
            for _ in 0..days {
                next_clock_day_internal(&mut cx.world.data.game.game_clock);
                cx.with_legacy(|| crate::ui::sis::DrawStatusMessage(ptr::null()));
                finish_clock_day(cx.world);
            }
            let clock = &mut cx.world.data.game.game_clock;
            clock.tick_count = clock.day_in_ticks;
        });
    }
    /// # Safety
    /// Called with the initialized clock.
    #[no_mangle]
    pub extern "C" fn LockGameClock() {
        rust_clock_lock();
    }
    /// # Safety
    /// Called with the initialized clock.
    #[no_mangle]
    pub extern "C" fn UnlockGameClock() {
        rust_clock_unlock();
    }
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
