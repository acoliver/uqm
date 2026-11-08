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
pub(crate) mod live {
    use super::{navigate, transfer_crew, SHIP_POS};
    use crate::c_bindings::controller_input::*;
    use crate::comm::locdata::CRect;
    use crate::engine::battle::link_queue::LinkId;
    use crate::engine::world::queues::Fragment;
    use crate::engine::{battle::Cx, observe::Io, World};
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{ContextId, FlashTarget, Gfx, Point, Rect, Rgba, Stamp};
    use std::ffi::{c_void, CString};

    use crate::time::counter_abi::GetTimeCounter;

    use crate::ui::sis::capacity::crew_capacity;

    use crate::ui::sis::GetStatusMessageRect;

    use crate::ui::sis::DrawStatusMessage;

    use crate::ui::sis::DeltaSISGauges;

    use crate::mainloop::input_abi::live::SetMenuSounds;

    struct Menu {
        selected: usize,
        positions: Vec<(i16, i16)>,
        point: (i16, i16),
        frame: *mut c_void,
        modifying: bool,
    }

    fn support_ship(world: &World, menu: &Menu) -> Option<LinkId<Fragment>> {
        let index = SHIP_POS.iter().position(|point| *point == menu.point)?;
        world.built_ships_ref().at(index)
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
        GetStatusMessageRect(std::ptr::from_mut(&mut r).cast());
        CffiPlatform.set_flash(FlashTarget::Rect(Rect {
            corner: Point {
                x: r.corner.x,
                y: r.corner.y,
            },
            width: r.width,
            height: r.height,
        }));
    }

    unsafe fn delta_crew(menu: &mut Menu, delta: i16) -> bool {
        let value = World::current().write(|world| {
            let id = support_ship(world, menu)?;
            if delta == 0 {
                menu.frame = world.built_ships_ref().get(id).icons.cast();
            }
            let race = world.built_ships_ref().get(id).race_id;
            let fleets = world.avail_races_ref();
            let maximum = fleets
                .get(
                    fleets
                        .at(usize::from(race))
                        .unwrap_or_else(|| panic!("escort fleet")),
                )
                .crew_level;
            let mut ships = world.built_ships();
            let ship = ships.get_mut(id);
            if !transfer_crew(&mut ship.crew_level, maximum, delta) {
                return None;
            }
            Some(if ship.crew_level >= maximum {
                ship.crew_level.to_string()
            } else {
                format!("{}/{}", ship.crew_level, maximum)
            })
        });
        let Some(value) = value else {
            return false;
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

    unsafe fn redraw_selection(menu: &mut Menu, next: usize) {
        let g = CffiPlatform;
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

    unsafe fn modify(
        cx: &mut Cx<'_>,
        input: &crate::engine::input::InputSession,
        menu: &mut Menu,
    ) -> i32 {
        if cx.world.activity() & (1 << 14) != 0 {
            return 0;
        }
        let pulse = input.pulsed();
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
            // SAFETY: the roster retains its selected ship, fonts and display resources.
            cx.with_legacy(|| {
                if !menu.modifying {
                    g.set_flash(FlashTarget::Off);
                    SetMenuSounds(input, 15, 16);
                } else {
                    g.set_context(ContextId::Status);
                    g.set_fg_color(Rgba::rgb15(31, 31, 31));
                    draw(menu, true);
                    flash_crew();
                    SetMenuSounds(input, 3, 48);
                }
            });
        } else if menu.modifying {
            let mut delta = 0;
            let mut failed = false;
            if up {
                if cx.world.data.flagship.crew_enlisted != 0 {
                    delta = 1;
                } else {
                    failed = true;
                }
            } else if down {
                let crew = cx.world.data.flagship.crew_enlisted;
                let capacity = crew_capacity(&cx.world.data.flagship);
                if crew < capacity {
                    delta = -1;
                } else {
                    failed = true;
                }
            }
            if delta != 0 {
                // SAFETY: the selected roster entry and its campaign fleet remain live.
                failed = !cx.with_legacy(|| delta_crew(menu, delta));
            }
            if failed {
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
            }
        } else {
            let next = navigate(&menu.positions, menu.selected, horizontal, down, up);
            // SAFETY: the roster retains all ship frames and the selected status context.
            cx.with_legacy(|| redraw_selection(menu, next));
        }
        // SAFETY: the admitted menu keeps initialized game-thread pacing callbacks live.
        cx.with_legacy(|| crate::battle::c_bridge::SleepThread(840 / 30));
        1
    }

    /// # Safety
    /// Call on the game thread with campaign globals and screen resources initialized.
    pub unsafe fn roster_menu(cx: &mut Cx<'_>, io: &mut Io<'_>) -> i32 {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let count = cx.world.built_ships_ref().count();
        let Some(mut menu) = cx.with_legacy(|| {
            if count == 0 {
                return None;
            }
            let mut positions = SHIP_POS[..count].to_vec();
            positions.sort_by_key(|point| (point.0, point.1));
            let mut menu = Menu {
                selected: 0,
                positions,
                point: (0, 0),
                frame: std::ptr::null_mut(),
                modifying: false,
            };
            CffiPlatform.set_context(ContextId::Status);
            select(&mut menu, 0);
            SetMenuSounds(input, 15, 16);
            Some(menu)
        }) else {
            return 0;
        };
        cx.run_input(io, true, |cx, _| modify(cx, input, &mut menu) != 0);
        cx.with_legacy(|| {
            CffiPlatform.set_context(ContextId::Status);
            draw(&menu, false);
            DrawStatusMessage(std::ptr::null());
        });
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
