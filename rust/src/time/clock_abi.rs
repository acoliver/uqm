//! C clock API, including the save-compatible intrusive event queue.

#![allow(non_snake_case)]

use super::clock_bridge::{days_in_month, CEvent};
use crate::collections::queue::{AllocLink, HLink, InitQueue, InsertQueue, Queue, UninitQueue};

fn event_date(now: (u16, u16, u16), relative: bool, date: (u16, u16, u16)) -> (u16, u16, u16) {
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

unsafe fn insert_event(queue: *mut Queue, date: (u16, u16, u16), selector: u8) -> HLink {
    let new = AllocLink(queue);
    if new.is_null() {
        return new;
    }
    let event = new.cast::<CEvent>();
    (*event).year_index = date.0;
    (*event).month_index = date.1 as u8;
    (*event).day_index = date.2 as u8;
    (*event).func_index = selector;
    let mut at = (*queue).head;
    while !at.is_null() {
        let current = &*at.cast::<CEvent>();
        if date
            < (
                current.year_index,
                current.month_index as u16,
                current.day_index as u16,
            )
        {
            break;
        }
        at = current.succ;
    }
    InsertQueue(queue, new, at);
    new
}

unsafe fn process_day_events(
    queue: *mut Queue,
    today: (u16, u16, u16),
    mut dispatch: impl FnMut(u8),
) {
    loop {
        let head = (*queue).head;
        if head.is_null() {
            break;
        }
        let event = head.cast::<CEvent>();
        let date = (
            (*event).year_index,
            u16::from((*event).month_index),
            u16::from((*event).day_index),
        );
        // Saves produced by the missing dispatch implementation can contain
        // overdue events. Execute those callbacks instead of stranding the queue.
        if date > today {
            break;
        }
        let selector = (*event).func_index;
        crate::collections::queue::RemoveQueue(queue, head);
        dispatch(selector);
        crate::collections::queue::FreeLink(queue, head);
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::super::clock_bridge::*;
    use super::*;
    use std::ffi::c_void;
    use std::ptr;

    use crate::navigation::events::live::EventHandler;

    use crate::threading::legacy_abi::CreateMutex_Core;

    use crate::threading::legacy_abi::DestroyMutex;

    extern "C" {
        static mut GlobData: crate::ships::native::abi::GLOBDATA;
    }
    static mut CLOCK_MUTEX: *mut c_void = ptr::null_mut();

    /// # Safety
    /// Game-thread access to the live global state.
    #[no_mangle]
    pub unsafe extern "C" fn GetGameClock() -> *mut ClockState {
        ptr::addr_of_mut!(GlobData.Game_state.GameClock).cast()
    }

    pub(crate) unsafe fn finish_clock_day() {
        let clock = GetGameClock();
        crate::ui::sis::DrawStatusMessage(ptr::null());
        let today = (
            (*clock).year_index,
            u16::from((*clock).month_index),
            u16::from((*clock).day_index),
        );
        process_day_events(ptr::addr_of_mut!((*clock).event_q), today, |selector| {
            EventHandler(selector)
        });
    }

    /// # Safety
    /// Date pointers must be valid and writable; game-thread access only.
    #[no_mangle]
    pub unsafe extern "C" fn ValidateEvent(
        kind: u32,
        month: *mut u16,
        day: *mut u16,
        year: *mut u16,
    ) -> u32 {
        let clock = &*GetGameClock();
        let now = (
            clock.year_index,
            clock.month_index as u16,
            clock.day_index as u16,
        );
        let date = event_date(now, kind == 1, (*year, *month, *day));
        if kind == 1 {
            (*year, *month, *day) = date;
        }
        u32::from(date >= now)
    }

    /// # Safety
    /// Game-thread access to the initialized event queue.
    #[no_mangle]
    pub unsafe extern "C" fn AddEvent(
        kind: u32,
        mut month: u16,
        mut day: u16,
        mut year: u16,
        selector: u8,
    ) -> HLink {
        if kind == 1 && month == 0 && day == 0 && year == 0 {
            EventHandler(selector);
        } else if ValidateEvent(kind, &mut month, &mut day, &mut year) != 0 {
            return insert_event(
                ptr::addr_of_mut!((*GetGameClock()).event_q),
                (year, month, day),
                selector,
            );
        }
        ptr::null_mut()
    }

    /// # Safety
    /// Called once before accessing the game clock or event queue.
    #[no_mangle]
    pub unsafe extern "C" fn InitGameClock() -> u32 {
        if !InitQueue(
            ptr::addr_of_mut!((*GetGameClock()).event_q),
            NUM_EVENTS as u16,
            std::mem::size_of::<CEvent>() as u16,
        ) {
            return 0;
        }
        CLOCK_MUTEX = (CreateMutex_Core(c"Clock Mutex".as_ptr(), 1)).cast();
        rust_clock_init() as u32
    }

    /// # Safety
    /// The initialized clock must no longer be in use.
    #[no_mangle]
    pub unsafe extern "C" fn UninitGameClock() -> u32 {
        DestroyMutex((CLOCK_MUTEX).cast());
        CLOCK_MUTEX = ptr::null_mut();
        UninitQueue(ptr::addr_of_mut!((*GetGameClock()).event_q));
        rust_clock_uninit() as u32
    }

    /// # Safety
    /// Game-thread access to the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn SetGameClockRate(seconds: u16) {
        rust_clock_set_rate(seconds as i32);
    }
    /// # Safety
    /// Game-thread access to the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn GameClockTick() {
        rust_clock_tick();
    }
    /// # Safety
    /// Game-thread access to the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn MoveGameClockDays(days: u16) {
        rust_clock_advance_days(days as i32);
    }
    /// # Safety
    /// Called with the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn LockGameClock() {
        rust_clock_lock();
    }
    /// # Safety
    /// Called with the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn UnlockGameClock() {
        rust_clock_unlock();
    }
    /// # Safety
    /// Called with the initialized clock.
    #[no_mangle]
    pub unsafe extern "C" fn GameClockRunning() -> u32 {
        rust_clock_is_running() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_events_are_removed_before_callbacks_and_preserve_order() {
        let mut queue: Queue = unsafe { std::mem::zeroed() };
        assert!(unsafe {
            // SAFETY: The queue storage is exclusive and its element layout matches this pool.
            InitQueue(&mut queue, 8, std::mem::size_of::<CEvent>() as u16)
        });
        let queue = &mut queue as *mut Queue;
        unsafe {
            insert_event(queue, (2155, 3, 17), 0);
            insert_event(queue, (2156, 3, 17), 1);
            insert_event(queue, (2156, 3, 17), 2);
            insert_event(queue, (2156, 3, 18), 3);
            let mut seen = Vec::new();
            process_day_events(queue, (2156, 3, 17), |selector| {
                seen.push(selector);
                if selector == 1 {
                    insert_event(queue, (2156, 3, 17), 4);
                }
            });
            assert_eq!(seen, [0, 1, 2, 4]);
            assert_eq!((*(*queue).head.cast::<CEvent>()).func_index, 3);
            process_day_events(queue, (2156, 3, 18), |selector| seen.push(selector));
            assert_eq!(seen, [0, 1, 2, 4, 3]);
            assert!((*queue).head.is_null());
        }
        unsafe {
            // SAFETY: This scope owns the initialized pool and no element is used after teardown.
            UninitQueue(queue)
        };
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
        let mut queue: Queue = unsafe { std::mem::zeroed() };
        assert!(unsafe {
            // SAFETY: The queue storage is exclusive and its element layout matches this pool.
            InitQueue(&mut queue, 4, std::mem::size_of::<CEvent>() as u16)
        });
        unsafe {
            let late = insert_event(&mut queue, (2156, 1, 1), 1);
            let first = insert_event(&mut queue, (2155, 2, 18), 2);
            let second = insert_event(&mut queue, (2155, 2, 18), 3);
            let middle = insert_event(&mut queue, (2155, 12, 31), 4);
            assert_eq!(queue.head, first);
            assert_eq!((*first.cast::<CEvent>()).succ, second);
            assert_eq!((*second.cast::<CEvent>()).succ, middle);
            assert_eq!((*middle.cast::<CEvent>()).succ, late);
            assert_eq!(queue.tail, late);
            assert!(insert_event(&mut queue, (2155, 1, 1), 5).is_null());
        }
        unsafe {
            // SAFETY: This scope owns the initialized pool and no element is used after teardown.
            UninitQueue(&mut queue)
        };
    }
}
