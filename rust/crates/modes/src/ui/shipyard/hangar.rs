use super::shipyard_layout::{
    HANGAR_DY, HANGAR_SHIPS_ROW, HANGAR_X_COORDS, HANGAR_Y, SHIP_WIN_WIDTH,
};
use crate::engine::battle::link_queue::{LinkId, LinkQueue};
use crate::engine::world::queues::Fragment;
use crate::graphics::{drawable::FrameId, geometry::Point, intersection::Stamp};

#[derive(Clone, Copy)]
pub(super) struct ShipWindow {
    pub(super) ship: Option<LinkId<Fragment>>,
    pub(super) finished: Point,
    pub(super) image: Stamp,
    pub(super) left_door: Stamp,
    pub(super) right_door: Stamp,
}

pub(super) fn prepare_windows(
    queue: &mut LinkQueue<'_, Fragment, 12>,
    module_frame: FrameId,
    mut window: u16,
    yanked: Option<LinkId<Fragment>>,
) -> (
    [ShipWindow; crate::starbase::shipyard::HANGAR_SHIPS as usize],
    u16,
) {
    let stamp = Stamp {
        origin: Point { x: 0, y: 0 },
        frame: crate::graphics::drawable::FrameId::NULL,
    };
    let mut windows = [ShipWindow {
        ship: None,
        finished: Point { x: 0, y: 0 },
        image: stamp,
        left_door: stamp,
        right_door: stamp,
    }; crate::starbase::shipyard::HANGAR_SHIPS as usize];
    let mut num_ships: u16 = 1;
    if let Some(id) = yanked {
        let ship = queue.get(id);
        let info = &mut windows[0];
        info.ship = Some(id);
        info.left_door.origin = Point {
            x: -(SHIP_WIN_WIDTH >> 1),
            y: 0,
        };
        info.right_door.origin = Point {
            x: SHIP_WIN_WIDTH >> 1,
            y: 0,
        };
        info.left_door.frame = crate::graphics::menu::relative_frame(module_frame, 1);
        info.right_door.frame = crate::graphics::menu::relative_frame(info.left_door.frame, 1);
        info.image.origin = Point {
            x: (SHIP_WIN_WIDTH >> 1) + 1,
            y: SHIP_WIN_WIDTH >> 1,
        };
        info.image.frame = ship.melee_icon;
        info.finished.x = HANGAR_X_COORDS[window as usize % HANGAR_SHIPS_ROW as usize];
        info.finished.y = HANGAR_Y + HANGAR_DY * (window / HANGAR_SHIPS_ROW as u16) as i16;
    } else {
        let mut handle;
        if window == u16::MAX {
            handle = queue.head();
            num_ships = queue.count() as u16;
        } else {
            let Some(tail) = queue.tail() else {
                panic!("a newly purchased hangar ship must be linked");
            };
            queue.remove(tail);
            handle = queue.head();
            while let Some(id) = handle {
                if u16::from(queue.get(id).index) > window {
                    break;
                }
                handle = queue.succ(id);
            }
            queue.insert_before(tail, handle);
            handle = Some(tail);
            queue.get_mut(tail).index = window as u8;
        }
        for info in &mut windows[..usize::from(num_ships)] {
            let Some(id) = handle else {
                panic!("hangar ship count must match its live links");
            };
            let ship = queue.get(id);
            handle = queue.succ(id);
            info.ship = Some(id);
            info.left_door.origin = Point { x: -1, y: 0 };
            info.right_door.origin = Point { x: 1, y: 0 };
            info.left_door.frame = crate::graphics::menu::relative_frame(module_frame, 1);
            info.right_door.frame = crate::graphics::menu::relative_frame(info.left_door.frame, 1);
            info.image.origin = Point {
                x: (SHIP_WIN_WIDTH >> 1) + 1,
                y: SHIP_WIN_WIDTH >> 1,
            };
            info.image.frame = ship.melee_icon;
            window = u16::from(ship.index);
            info.finished.x = HANGAR_X_COORDS[window as usize % HANGAR_SHIPS_ROW as usize];
            info.finished.y = HANGAR_Y + HANGAR_DY * (window / HANGAR_SHIPS_ROW as u16) as i16;
        }
    }
    (windows, num_ships)
}

pub(super) fn draw_background(module_frame: FrameId) {
    use crate::graphics::menu as draw;
    let frame = draw::frame(module_frame, 0);
    if super::shipyard_layout::USE_3DO_HANGAR {
        draw::stamp(frame, Point { x: 0, y: 0 });
    } else {
        let old = draw::clip();
        let mut clip = old;
        clip.corner.x -= 1;
        clip.extent.width += 2;
        clip.extent.height += 1;
        draw::set_clip(clip);
        draw::stamp(frame, Point { x: 0, y: 0 });
        draw::set_clip(old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::World;

    #[test]
    fn empty_hangar_has_no_ship_windows() {
        let mut world = World::new();
        let mut queue = world.built_ships();
        let (windows, count) = prepare_windows(
            &mut queue,
            crate::graphics::drawable::FrameId::NULL,
            u16::MAX,
            None,
        );
        assert_eq!(count, 0);
        assert!(windows.iter().all(|window| window.ship.is_none()));
    }

    #[test]
    fn existing_ships_keep_queue_order_and_row_positions() {
        let mut world = World::new();
        let mut queue = world.built_ships();
        let first = queue.alloc().unwrap();
        queue.get_mut(first).index = 1;
        queue.put(first);
        let second = queue.alloc().unwrap();
        queue.get_mut(second).index = 7;
        queue.put(second);
        let (windows, count) = prepare_windows(
            &mut queue,
            crate::graphics::drawable::FrameId::NULL,
            u16::MAX,
            None,
        );
        assert_eq!(count, 2);
        assert_eq!(windows[0].ship, Some(first));
        assert_eq!(windows[1].ship, Some(second));
        assert_eq!(windows[0].finished.x, HANGAR_X_COORDS[1]);
        assert_eq!(windows[0].finished.y, HANGAR_Y);
        assert_eq!(windows[1].finished.y, HANGAR_Y + HANGAR_DY);
        assert_eq!(windows[0].left_door.origin.x, -1);
        assert_eq!(windows[1].right_door.origin.x, 1);
    }

    #[test]
    fn new_tail_ship_is_inserted_before_the_next_occupied_slot() {
        let mut world = World::new();
        let mut queue = world.built_ships();
        let ids = [1, 6, 0].map(|slot| {
            let id = queue.alloc().unwrap();
            queue.get_mut(id).index = slot;
            queue.put(id);
            id
        });
        let (windows, count) = prepare_windows(
            &mut queue,
            crate::graphics::drawable::FrameId::NULL,
            3,
            None,
        );
        assert_eq!(count, 1);
        assert_eq!(windows[0].ship, Some(ids[2]));
        assert_eq!(queue.get(ids[2]).index, 3);
        assert_eq!(queue.succ(ids[0]), Some(ids[2]));
        assert_eq!(queue.succ(ids[2]), Some(ids[1]));
        assert_eq!(queue.tail(), Some(ids[1]));
        assert_eq!(windows[0].finished.x, HANGAR_X_COORDS[3]);
    }

    #[test]
    fn removed_ship_uses_open_doors_without_touching_the_queue() {
        let mut world = World::new();
        let mut queue = world.built_ships();
        let removed = queue.alloc().unwrap();
        let (windows, count) = prepare_windows(
            &mut queue,
            crate::graphics::drawable::FrameId::NULL,
            7,
            Some(removed),
        );
        assert_eq!(count, 1);
        assert_eq!(windows[0].ship, Some(removed));
        assert_eq!(windows[0].left_door.origin.x, -(SHIP_WIN_WIDTH >> 1));
        assert_eq!(windows[0].right_door.origin.x, SHIP_WIN_WIDTH >> 1);
        assert_eq!(windows[0].finished.x, HANGAR_X_COORDS[1]);
        assert_eq!(windows[0].finished.y, HANGAR_Y + HANGAR_DY);
        assert!(queue.head().is_none());
        assert!(queue.tail().is_none());
    }
}
