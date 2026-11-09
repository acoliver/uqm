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

fn update_crew(
    world: &mut crate::engine::World,
    point: (i16, i16),
    delta: i16,
) -> Option<(Option<crate::graphics::context_abi::ContextFrame>, String)> {
    let index = SHIP_POS.iter().position(|candidate| *candidate == point)?;
    let id = world.built_ships_ref().at(index)?;
    let record = world.built_ships_ref();
    let ship = record.get(id);
    let frame = if delta == 0 {
        std::ptr::NonNull::new(ship.icons)
            .map(|frame| crate::graphics::context_abi::ContextFrame::retain(Some(frame)))
    } else {
        None
    };
    let fleets = world.avail_races_ref();
    let maximum = fleets
        .get(
            fleets
                .at(usize::from(ship.race_id))
                .unwrap_or_else(|| panic!("escort fleet")),
        )
        .crew_level;
    let mut ships = world.built_ships();
    let ship = ships.get_mut(id);
    if !transfer_crew(&mut ship.crew_level, maximum, delta) {
        return None;
    }
    let label = if ship.crew_level >= maximum {
        ship.crew_level.to_string()
    } else {
        format!("{}/{}", ship.crew_level, maximum)
    };
    Some((frame, label))
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::{navigate, update_crew, SHIP_POS};
    use crate::c_bindings::controller_input::*;
    use crate::engine::{battle::Cx, observe::Io};
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{ContextId, FlashTarget, Gfx, Point, Rect, Rgba};
    use std::ffi::CString;

    use crate::time::counter_abi::GetTimeCounter;

    use crate::ui::sis::capacity::crew_capacity;

    use crate::mainloop::input_abi::live::SetMenuSounds;

    struct Menu {
        selected: usize,
        positions: Vec<(i16, i16)>,
        point: (i16, i16),
        frame: Option<crate::graphics::context_abi::ContextFrame>,
        context: crate::graphics::context_abi::ContextLease,
        modifying: bool,
    }

    fn draw(menu: &Menu, filled: bool) {
        let Some(frame) = &menu.frame else {
            return;
        };
        let origin = crate::graphics::native_abi::POINT {
            x: menu.point.0,
            y: menu.point.1,
        };
        if filled {
            menu.context
                .draw_filled_stamp(frame, origin, menu.context.foreground());
        } else {
            menu.context.draw_stamp(frame, origin);
        }
    }

    unsafe fn flash_crew() {
        CffiPlatform.set_context(ContextId::Status);
        let r = crate::ui::sis::status_message_rect();
        CffiPlatform.set_flash(FlashTarget::Rect(Rect {
            corner: Point {
                x: r.corner.x,
                y: r.corner.y,
            },
            width: r.extent.width,
            height: r.extent.height,
        }));
    }

    unsafe fn delta_crew(cx: &mut Cx<'_>, menu: &mut Menu, delta: i16) -> bool {
        let Some((frame, value)) = update_crew(cx.world, menu.point, delta) else {
            return false;
        };
        if delta == 0 {
            menu.frame = frame;
        }
        let message = CString::new(value).unwrap();
        cx.with_legacy(|| CffiPlatform.pre_update_flash());
        crate::ui::sis::draw_status_message(cx.kernel, cx.world, Some(&message));
        cx.with_legacy(|| CffiPlatform.post_update_flash());
        crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, -delta, 0, 0);
        if delta != 0 {
            cx.with_legacy(|| flash_crew());
        }
        true
    }

    unsafe fn select(cx: &mut Cx<'_>, menu: &mut Menu, index: usize) {
        menu.point = menu.positions[index];
        menu.frame = None;
        delta_crew(cx, menu, 0);
    }

    fn flash_ship(flash: &mut crate::engine::kernel::roster_flash::RosterFlash, menu: &Menu) {
        if let Some(color) = flash.advance(GetTimeCounter) {
            menu.context.set_foreground(color);
            draw(menu, true);
        }
    }

    unsafe fn redraw_selection(cx: &mut Cx<'_>, menu: &mut Menu, next: usize) {
        let context = menu.context.clone();
        context.with_batch(|| {
            if next != menu.selected {
                draw(menu, false);
                select(cx, menu, next);
                menu.selected = next;
            }
            flash_ship(&mut cx.kernel.roster_flash, menu);
        });
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
                failed = !delta_crew(cx, menu, delta);
            }
            if failed {
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
            }
        } else {
            let next = navigate(&menu.positions, menu.selected, horizontal, down, up);
            // SAFETY: the roster retains all ship frames and the selected status context.
            redraw_selection(cx, menu, next);
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
        if count == 0 {
            return 0;
        }
        let mut menu = {
            let mut positions = SHIP_POS[..count].to_vec();
            positions.sort_by_key(|point| (point.0, point.1));
            let menu = Menu {
                selected: 0,
                positions,
                point: (0, 0),
                frame: None,
                context: crate::graphics::context_abi::ContextLease::require(
                    cx.kernel.status_context,
                ),
                modifying: false,
            };
            menu.context.select();
            menu
        };
        select(cx, &mut menu, 0);
        cx.with_legacy(|| SetMenuSounds(input, 15, 16));
        cx.run_input(io, true, |cx, _| modify(cx, input, &mut menu) != 0);
        cx.with_legacy(|| CffiPlatform.set_context(ContextId::Status));
        cx.with_legacy(|| draw(&menu, false));
        crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crew_updates_only_the_supplied_world_and_selected_escort() {
        fn world(crew: u16, maximum: u16) -> crate::engine::World {
            let mut world = crate::engine::World::new();
            let mut fleets = world.avail_races();
            let id = fleets.alloc().unwrap();
            fleets.get_mut(id).crew_level = maximum;
            fleets.put(id);
            let mut ships = world.built_ships();
            for value in [crew, 3] {
                let id = ships.alloc().unwrap();
                ships.get_mut(id).crew_level = value;
                ships.put(id);
            }
            world
        }
        let mut first = world(1, 10);
        let mut second = world(5, 8);
        assert!(update_crew(&mut first, SHIP_POS[0], -1).is_none());
        assert_eq!(update_crew(&mut second, SHIP_POS[0], 1).unwrap().1, "6/8");
        assert_eq!(update_crew(&mut first, SHIP_POS[0], 1).unwrap().1, "2/10");
        assert_eq!(update_crew(&mut first, SHIP_POS[1], 0).unwrap().1, "3/10");
        assert!(update_crew(&mut first, (0, 0), 1).is_none());
        assert!(update_crew(&mut first, SHIP_POS[2], 1).is_none());
    }

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
