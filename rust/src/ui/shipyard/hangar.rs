use super::*;

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
        // SAFETY: the caller retains the mutable queue, fragment records and menu drawable.
        unsafe {
            let mut handle;
            if window == COUNT::MAX {
                handle = queue.head;
                num_ships = CountLinks(queue);
            } else {
                let tail = queue.tail;
                RemoveQueue(queue, tail);
                handle = queue.head;
                while !handle.is_null() {
                    let ship = handle.cast::<SHIP_FRAGMENT>();
                    if (*ship).index as COUNT > window {
                        break;
                    }
                    handle = (*ship).succ;
                }
                InsertQueue(queue, tail, handle);
                handle = tail;
                (*handle.cast::<SHIP_FRAGMENT>()).index = window as BYTE;
            }
            for i in 0..num_ships {
                let ship = handle.cast::<SHIP_FRAGMENT>();
                let next = (*ship).succ;
                let info = &mut windows[i as usize];
                info.ship = ship;
                info.left_door.origin = POINT { x: -1, y: 0 };
                info.right_door.origin = POINT { x: 1, y: 0 };
                info.left_door.frame = IncFrameIndex(menu.ModuleFrame);
                info.right_door.frame = IncFrameIndex(info.left_door.frame);
                info.image.origin = POINT {
                    x: (SHIP_WIN_WIDTH >> 1) + 1,
                    y: SHIP_WIN_WIDTH >> 1,
                };
                info.image.frame = (*ship).melee_icon;
                window = (*ship).index as COUNT;
                info.finished.x = HANGAR_X_COORDS[window as usize % HANGAR_SHIPS_ROW as usize];
                info.finished.y =
                    HANGAR_Y + HANGAR_DY * (window / HANGAR_SHIPS_ROW as COUNT) as i16;
                handle = next;
            }
        }
    }
    (windows, num_ships)
}
