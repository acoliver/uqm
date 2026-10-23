//! Escort selection and crew transfers, ported from `planets/roster.c`.

const SHIP_POS: [(i16, i16); 12] = [
    (3, 62),
    (45, 62),
    (3, 78),
    (45, 78),
    (3, 46),
    (45, 46),
    (3, 94),
    (45, 94),
    (3, 30),
    (45, 30),
    (3, 110),
    (45, 110),
];

fn navigate(
    positions: &[(i16, i16)],
    current: usize,
    horizontal: bool,
    down: bool,
    up: bool,
) -> usize {
    let count = positions.len();
    let top_right = (count + 1) >> 1;
    let mut next = current;
    if count < 2 {
        return next;
    } else if horizontal {
        if next == top_right - 1 {
            next = count - 1;
        } else if next >= top_right {
            next -= top_right;
            if positions[next].1 < positions[current].1 {
                next += 1;
            }
        } else {
            next += top_right;
            if next != top_right && positions[next].1 > positions[current].1 {
                next -= 1;
            }
        }
    } else if down {
        next += 1;
        if next == count {
            next = top_right;
        } else if next == top_right {
            next = 0;
        }
    } else if up {
        if next == 0 {
            next = top_right - 1;
        } else if next == top_right {
            next = count - 1;
        } else {
            next -= 1;
        }
    }
    next
}

fn transfer_crew(crew: &mut u16, maximum: u16, delta: i16) -> bool {
    *crew = crew.wrapping_add_signed(delta);
    if *crew == 0 {
        *crew = 1;
        false
    } else if *crew > maximum && delta > 0 {
        *crew = crew.wrapping_add_signed(-delta);
        false
    } else {
        true
    }
}

#[cfg(not(test))]
mod live {
    use super::{navigate, transfer_crew, SHIP_POS};
    use crate::battle::c_bridge::SleepThread;
    use crate::c_bindings::controller_input::*;
    use crate::collections::queue::{CountLinks, Queue};
    use crate::comm::dispatch::{get_star_ship_from_index, CFleetInfo, CShipFragment};
    use crate::comm::locdata::{CGlobData, CRect};
    use crate::mainloop::restart_menu::c_extern::DoInput;
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{ContextId, FlashTarget, Gfx, Point, Rect, Rgba, Stamp};
    use std::ffi::{c_char, c_void, CString};

    use crate::time::counter_abi::GetTimeCounter;
    extern "C" {
        static mut GlobData: CGlobData;
        fn GetCrewPodCapacity() -> u16;
        fn GetStatusMessageRect(rect: *mut CRect);
        fn DrawStatusMessage(message: *const c_char);
        fn DeltaSISGauges(crew: i16, fuel: i16, units: i32);
        fn SetMenuSounds(move_sound: u16, select_sound: u16);
        fn PlayMenuSound(sound: i32);
    }

    #[repr(C)]
    struct Menu {
        input: unsafe extern "C" fn(*mut Menu) -> i32,
        selected: usize,
        positions: Vec<(i16, i16)>,
        point: (i16, i16),
        frame: *mut c_void,
        modifying: bool,
    }

    unsafe fn support_ship(menu: &Menu) -> *mut CShipFragment {
        let mut link = GlobData.game_state.built_ship_q.head;
        let mut index = 0;
        while !link.is_null() {
            let ship = link.cast::<CShipFragment>();
            if SHIP_POS[index] == menu.point {
                return ship;
            }
            link = (*ship).succ;
            index += 1;
        }
        std::ptr::null_mut()
    }

    fn draw(menu: &Menu, filled: bool) {
        if menu.frame.is_null() {
            return;
        }
        let stamp = Stamp {
            origin: Point {
                x: menu.point.0,
                y: menu.point.1,
            },
            frame: menu.frame as usize,
        };
        if filled {
            CffiPlatform.draw_filled_stamp(stamp);
        } else {
            CffiPlatform.draw_stamp(stamp);
        }
    }

    unsafe fn flash_crew() {
        CffiPlatform.set_context(ContextId::Status);
        let mut r = CRect::default();
        GetStatusMessageRect(&mut r);
        CffiPlatform.set_flash(FlashTarget::Rect(Rect {
            corner: Point {
                x: r.corner.x,
                y: r.corner.y,
            },
            width: r.width,
            height: r.height,
        }));
    }

    unsafe fn delta_crew(menu: &Menu, delta: i16) -> bool {
        let ship = support_ship(menu);
        if ship.is_null() {
            return false;
        }
        let queue = std::ptr::addr_of!(GlobData.game_state.avail_race_q).cast::<Queue>();
        let template = get_star_ship_from_index(queue, (*ship).race_id).cast::<CFleetInfo>();
        if !transfer_crew(&mut (*ship).crew_level, (*template).crew_level, delta) {
            return false;
        }
        let value = if (*ship).crew_level >= (*template).crew_level {
            (*ship).crew_level.to_string()
        } else {
            format!("{}/{}", (*ship).crew_level, (*template).crew_level)
        };
        let message = CString::new(value).unwrap();
        CffiPlatform.pre_update_flash();
        DrawStatusMessage(message.as_ptr());
        CffiPlatform.post_update_flash();
        DeltaSISGauges(-delta, 0, 0);
        if delta != 0 {
            flash_crew();
        }
        true
    }

    unsafe fn select(menu: &mut Menu, index: usize) {
        menu.point = menu.positions[index];
        menu.frame = std::ptr::null_mut();
        let ship = support_ship(menu);
        if !ship.is_null() {
            menu.frame = (*ship).icons;
        }
        delta_crew(menu, 0);
    }

    unsafe fn flash_ship(menu: &Menu) {
        static mut COLOR: Rgba = Rgba::rgb15(31, 0, 0);
        static mut NEXT_TIME: u32 = 0;
        if GetTimeCounter() >= NEXT_TIME {
            NEXT_TIME = GetTimeCounter().wrapping_add(840 / 15);
            let mut color = COLOR;
            if color.g >= Rgba::rgb15(0, 25, 25).g {
                color = Rgba::rgb15(31, 0, 0);
            } else {
                color.g += Rgba::rgb15(0, 2, 2).g;
                color.b += Rgba::rgb15(0, 2, 2).b;
            }
            COLOR = color;
            CffiPlatform.set_fg_color(color);
            draw(menu, true);
        }
    }

    unsafe extern "C" fn modify(menu: *mut Menu) -> i32 {
        let menu = &mut *menu;
        if GlobData.game_state.current_activity & (1 << 14) != 0 {
            return 0;
        }
        let pulse = std::ptr::addr_of!(PulsedInputState).read();
        let select_key = pulse.menu[KEY_MENU_SELECT as usize] != 0;
        let cancel = pulse.menu[KEY_MENU_CANCEL as usize] != 0;
        let up = pulse.menu[KEY_MENU_UP as usize] != 0;
        let down = pulse.menu[KEY_MENU_DOWN as usize] != 0;
        let horizontal =
            pulse.menu[KEY_MENU_LEFT as usize] != 0 || pulse.menu[KEY_MENU_RIGHT as usize] != 0;
        let g = CffiPlatform;
        if cancel && !menu.modifying {
            return 0;
        } else if select_key || cancel {
            menu.modifying ^= true;
            if !menu.modifying {
                g.set_flash(FlashTarget::Off);
                SetMenuSounds(15, 16);
            } else {
                g.set_context(ContextId::Status);
                g.set_fg_color(Rgba::rgb15(31, 31, 31));
                draw(menu, true);
                flash_crew();
                SetMenuSounds(3, 48);
            }
        } else if menu.modifying {
            let mut delta = 0;
            let mut failed = false;
            if up {
                if GlobData.sis_state.crew_enlisted != 0 {
                    delta = 1;
                } else {
                    failed = true;
                }
            } else if down {
                if GlobData.sis_state.crew_enlisted < GetCrewPodCapacity() {
                    delta = -1;
                } else {
                    failed = true;
                }
            }
            if delta != 0 {
                failed = !delta_crew(menu, delta);
            }
            if failed {
                PlayMenuSound(2);
            }
        } else {
            let next = navigate(&menu.positions, menu.selected, horizontal, down, up);
            g.batch();
            g.set_context(ContextId::Status);
            if next != menu.selected {
                draw(menu, false);
                select(menu, next);
                menu.selected = next;
            }
            flash_ship(menu);
            g.unbatch();
        }
        SleepThread(840 / 30);
        1
    }

    #[no_mangle]
    unsafe extern "C" fn RosterMenu() -> i32 {
        let count = CountLinks(std::ptr::addr_of!(GlobData.game_state.built_ship_q).cast::<Queue>())
            as usize;
        if count == 0 {
            return 0;
        }
        let mut positions = SHIP_POS[..count].to_vec();
        positions.sort_by_key(|point| (point.0, point.1));
        let mut menu = Menu {
            input: modify,
            selected: 0,
            positions,
            point: (0, 0),
            frame: std::ptr::null_mut(),
            modifying: false,
        };
        CffiPlatform.set_context(ContextId::Status);
        select(&mut menu, 0);
        SetMenuSounds(15, 16);
        DoInput((&mut menu as *mut Menu).cast(), 1);
        CffiPlatform.set_context(ContextId::Status);
        draw(&menu, false);
        DrawStatusMessage(std::ptr::null());
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crew_transfer_preserves_captain_and_template_capacity() {
        let mut crew = 1;
        assert!(!transfer_crew(&mut crew, 10, -1));
        assert_eq!(crew, 1);
        crew = 10;
        assert!(!transfer_crew(&mut crew, 10, 1));
        assert_eq!(crew, 10);
        assert!(transfer_crew(&mut crew, 10, -1));
        assert_eq!(crew, 9);
        assert!(transfer_crew(&mut crew, 10, 1));
        assert!(transfer_crew(&mut crew, 10, 0));
        crew = 12;
        assert!(transfer_crew(&mut crew, 10, -1));
        assert_eq!(crew, 11);
    }
    #[test]
    fn navigation_wraps_within_each_column_for_every_fleet_size() {
        for count in 1..=12 {
            let mut points = SHIP_POS[..count].to_vec();
            points.sort();
            for current in 0..count {
                let next = navigate(&points, current, false, true, false);
                assert_eq!(navigate(&points, next, false, false, true), current);
                assert!(navigate(&points, current, true, false, false) < count);
            }
        }
    }
    #[test]
    fn horizontal_navigation_uses_the_nearest_row_in_the_other_column() {
        let mut positions = SHIP_POS[..5].to_vec();
        positions.sort();
        assert_eq!(navigate(&positions, 0, true, false, false), 3);
        assert_eq!(navigate(&positions, 2, true, false, false), 4);
        assert_eq!(navigate(&positions, 4, true, false, false), 2);
    }
}
