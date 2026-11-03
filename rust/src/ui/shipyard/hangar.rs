use super::shipyard_layout::{
    HANGAR_DY, HANGAR_SHIPS_ROW, HANGAR_X_COORDS, HANGAR_Y, SHIP_WIN_WIDTH,
};
use crate::navigation::abi::*;
use std::{mem, ptr};

#[derive(Clone, Copy)]
pub(super) struct ShipWindow {
    pub(super) ship: *mut SHIP_FRAGMENT,
    pub(super) finished: POINT,
    pub(super) image: STAMP,
    pub(super) left_door: STAMP,
    pub(super) right_door: STAMP,
}

/// # Safety
/// The game thread retains the menu drawable and initialized fragment records.
pub(super) unsafe fn prepare_windows(
    queue: &mut QUEUE,
    menu: &MENU_STATE,
    mut window: COUNT,
    yanked: Option<&mut SHIP_FRAGMENT>,
) -> ([ShipWindow; MAX_BUILT_SHIPS as usize], COUNT) {
    // SAFETY: window records contain only nullable pointers and integer drawing coordinates.
    let mut windows: [ShipWindow; MAX_BUILT_SHIPS as usize] = unsafe { mem::zeroed() };
    let mut num_ships: COUNT = 1;
    if let Some(yanked) = yanked {
        let info = &mut windows[0];
        info.ship = ptr::from_mut(yanked);
        info.left_door.origin = POINT {
            x: -(SHIP_WIN_WIDTH >> 1),
            y: 0,
        };
        info.right_door.origin = POINT {
            x: SHIP_WIN_WIDTH >> 1,
            y: 0,
        };
        // SAFETY: the caller retains the menu drawable throughout preparation.
        info.left_door.frame = unsafe { IncFrameIndex(menu.ModuleFrame) };
        // SAFETY: both doors are frames in the same retained menu drawable.
        info.right_door.frame = unsafe { IncFrameIndex(info.left_door.frame) };
        info.image.origin = POINT {
            x: (SHIP_WIN_WIDTH >> 1) + 1,
            y: SHIP_WIN_WIDTH >> 1,
        };
        info.image.frame = yanked.melee_icon;
        info.finished.x = HANGAR_X_COORDS[window as usize % HANGAR_SHIPS_ROW as usize];
        info.finished.y = HANGAR_Y + HANGAR_DY * (window / HANGAR_SHIPS_ROW as COUNT) as i16;
    } else {
        let mut handle;
        if window == COUNT::MAX {
            handle = queue.head;
            // SAFETY: the caller retains every record in this initialized queue.
            num_ships = unsafe { CountLinks(queue) };
        } else {
            let tail = queue.tail;
            // SAFETY: the tail belongs to the exclusively borrowed queue.
            unsafe { RemoveQueue(queue, tail) };
            handle = queue.head;
            while !handle.is_null() {
                // SAFETY: each retained link is an initialized ship fragment.
                let ship = unsafe { &*handle.cast::<SHIP_FRAGMENT>() };
                if ship.index as COUNT > window {
                    break;
                }
                handle = ship.succ;
            }
            // SAFETY: the retained tail is detached, and handle is in this queue or null.
            unsafe { InsertQueue(queue, tail, handle) };
            handle = tail;
            // SAFETY: the queue exclusively retains the inserted fragment.
            unsafe { (*handle.cast::<SHIP_FRAGMENT>()).index = window as BYTE };
        }
        for i in 0..num_ships {
            // SAFETY: each retained fragment is exclusively borrowed during preparation.
            let ship = unsafe { &mut *handle.cast::<SHIP_FRAGMENT>() };
            let next = ship.succ;
            let info = &mut windows[i as usize];
            info.ship = ptr::from_mut(ship);
            info.left_door.origin = POINT { x: -1, y: 0 };
            info.right_door.origin = POINT { x: 1, y: 0 };
            // SAFETY: the caller retains the menu drawable throughout preparation.
            info.left_door.frame = unsafe { IncFrameIndex(menu.ModuleFrame) };
            // SAFETY: both doors are frames in the same retained menu drawable.
            info.right_door.frame = unsafe { IncFrameIndex(info.left_door.frame) };
            info.image.origin = POINT {
                x: (SHIP_WIN_WIDTH >> 1) + 1,
                y: SHIP_WIN_WIDTH >> 1,
            };
            info.image.frame = ship.melee_icon;
            window = ship.index as COUNT;
            info.finished.x = HANGAR_X_COORDS[window as usize % HANGAR_SHIPS_ROW as usize];
            info.finished.y = HANGAR_Y + HANGAR_DY * (window / HANGAR_SHIPS_ROW as COUNT) as i16;
            handle = next;
        }
    }
    (windows, num_ships)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> MENU_STATE {
        // SAFETY: menu records contain nullable handles and integers; null frames are supported.
        unsafe { mem::zeroed() }
    }

    #[test]
    fn empty_hangar_has_no_ship_windows() {
        let mut queue = QUEUE::default();
        // SAFETY: this fixture owns its empty queue and menu; no drawable is dereferenced.
        let (windows, count) = unsafe { prepare_windows(&mut queue, &menu(), COUNT::MAX, None) };
        assert_eq!(count, 0);
        assert!(windows.iter().all(|w| w.ship.is_null()));
    }

    #[test]
    fn existing_ships_keep_queue_order_and_row_positions() {
        // SAFETY: ship fragments contain nullable handles and integers.
        let mut ships: [SHIP_FRAGMENT; 2] = unsafe { mem::zeroed() };
        ships[0].index = 1;
        ships[1].index = 7;
        ships[0].succ = ptr::from_mut(&mut ships[1]).cast();
        ships[1].pred = ptr::from_mut(&mut ships[0]).cast();
        let mut queue = QUEUE {
            head: ptr::from_mut(&mut ships[0]).cast(),
            tail: ptr::from_mut(&mut ships[1]).cast(),
            ..QUEUE::default()
        };
        // SAFETY: the fixture retains both linked records throughout preparation.
        let (windows, count) = unsafe { prepare_windows(&mut queue, &menu(), COUNT::MAX, None) };
        assert_eq!(count, 2);
        assert_eq!(windows[0].ship, ptr::from_mut(&mut ships[0]));
        assert_eq!(windows[1].ship, ptr::from_mut(&mut ships[1]));
        assert_eq!(windows[0].finished.x, HANGAR_X_COORDS[1]);
        assert_eq!(windows[0].finished.y, HANGAR_Y);
        assert_eq!(windows[1].finished.y, HANGAR_Y + HANGAR_DY);
        assert_eq!(windows[0].left_door.origin.x, -1);
        assert_eq!(windows[1].right_door.origin.x, 1);
    }

    #[test]
    fn new_tail_ship_is_inserted_before_the_next_occupied_slot() {
        // SAFETY: ship fragments contain nullable handles and integers.
        let mut ships: [SHIP_FRAGMENT; 3] = unsafe { mem::zeroed() };
        for (ship, slot) in ships.iter_mut().zip([1, 6, 0]) {
            ship.index = slot;
        }
        for i in 0..2 {
            ships[i].succ = ptr::from_mut(&mut ships[i + 1]).cast();
            ships[i + 1].pred = ptr::from_mut(&mut ships[i]).cast();
        }
        let mut queue = QUEUE {
            head: ptr::from_mut(&mut ships[0]).cast(),
            tail: ptr::from_mut(&mut ships[2]).cast(),
            ..QUEUE::default()
        };
        // SAFETY: the fixture retains all records and the mutable queue header.
        let (windows, count) = unsafe { prepare_windows(&mut queue, &menu(), 3, None) };
        assert_eq!(count, 1);
        assert_eq!(windows[0].ship, ptr::from_mut(&mut ships[2]));
        assert_eq!(ships[2].index, 3);
        assert_eq!(ships[0].succ, ptr::from_ref(&ships[2]).cast_mut().cast());
        assert_eq!(ships[2].succ, ptr::from_ref(&ships[1]).cast_mut().cast());
        assert_eq!(queue.tail, ptr::from_mut(&mut ships[1]).cast());
        assert_eq!(windows[0].finished.x, HANGAR_X_COORDS[3]);
    }

    #[test]
    fn removed_ship_uses_open_doors_without_touching_the_queue() {
        let mut queue = QUEUE::default();
        // SAFETY: ship fragments contain nullable handles and integers.
        let mut removed: SHIP_FRAGMENT = unsafe { mem::zeroed() };
        let ship = ptr::from_mut(&mut removed);
        // SAFETY: the fixture retains the removed record and menu until preparation returns.
        let (windows, count) =
            unsafe { prepare_windows(&mut queue, &menu(), 7, Some(&mut removed)) };
        assert_eq!(count, 1);
        assert_eq!(windows[0].ship, ship);
        assert_eq!(windows[0].left_door.origin.x, -(SHIP_WIN_WIDTH >> 1));
        assert_eq!(windows[0].right_door.origin.x, SHIP_WIN_WIDTH >> 1);
        assert_eq!(windows[0].finished.x, HANGAR_X_COORDS[1]);
        assert_eq!(windows[0].finished.y, HANGAR_Y + HANGAR_DY);
        assert!(queue.head.is_null());
    }
}
