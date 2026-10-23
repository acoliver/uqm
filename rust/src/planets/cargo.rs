//! Cargo display and mineral-discard menu, ported from `planets/cargo.c`.

fn next_element(current: u8, back: bool, forward: bool) -> u8 {
    if back {
        if current == 0 {
            7
        } else {
            current - 1
        }
    } else if forward {
        if current == 7 {
            0
        } else {
            current + 1
        }
    } else {
        current
    }
}

#[cfg(not(test))]
mod live {
    use super::next_element;
    use crate::battle::c_bridge::SleepThread;
    use crate::c_bindings::controller_input::*;
    use crate::comm::locdata::{CColor, CGlobData, CPoint, CRect};
    use crate::mainloop::restart_menu::c_extern::DoInput;
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{Align, ContextId, FontId, Gfx, Point, Rect, Rgba, Stamp, TextDraw};
    use std::ffi::{c_char, c_void, CString};

    extern "C" {
        static mut GlobData: CGlobData;
        static MiscDataFrame: *mut c_void;
        fn GetStorageBayCapacity() -> u16;
        fn GetFrameRect(frame: *mut c_void, rect: *mut c_void);
        fn DrawStatusMessage(message: *const c_char);
        fn DrawStarConBox(
            rect: *mut CRect,
            border: i16,
            medium: CColor,
            dark: CColor,
            fill: i32,
            background: CColor,
        );
        fn SetMenuSounds(move_sound: u16, select_sound: u16);
        fn PlayMenuSound(sound: i32);
        fn ClearSISRect(flags: u16);
    }

    const BACK: Rgba = Rgba::rgb15(0, 0, 20);
    const WORTH: Rgba = Rgba::rgb15(10, 10, 31);
    const AMOUNT: Rgba = Rgba::rgb15(0, 20, 20);
    const SELECTED: Rgba = Rgba::rgb15(10, 31, 31);
    // gamestr.h: DEVICE_STRING_BASE + DEVICE_STRING_COUNT.
    const CARGO_STRINGS: i32 = 162;

    fn text(x: i16, y: i16, align: Align, value: String) {
        CffiPlatform.draw_text(&TextDraw {
            baseline: Point { x, y },
            align,
            text: value,
        });
    }

    #[no_mangle]
    unsafe extern "C" fn ShowRemainingCapacity() {
        let g = CffiPlatform;
        let old = g.set_context(ContextId::Status);
        g.set_font(FontId::Tiny);
        let free = u32::from(GetStorageBayCapacity())
            .wrapping_sub(u32::from(GlobData.sis_state.total_element_mass));
        g.batch();
        g.set_fg_color(BACK);
        g.fill_rect(Rect {
            corner: Point { x: 40, y: 107 },
            width: 20,
            height: 7,
        });
        g.set_fg_color(WORTH);
        text(59, 113, Align::Right, free.to_string());
        g.unbatch();
        g.restore_context(old);
    }

    unsafe fn element_amount(element: u8, selected: bool) {
        let g = CffiPlatform;
        let y = if element == 8 {
            119
        } else {
            35 + i16::from(element) * 9
        };
        g.set_fg_color(if selected { WORTH } else { BACK });
        g.fill_rect(Rect {
            corner: Point { x: 19, y },
            width: 40,
            height: 7,
        });
        let amount = if element == 8 {
            GlobData.sis_state.total_bio_mass
        } else {
            g.set_fg_color(if selected { AMOUNT } else { WORTH });
            text(
                32,
                y + 6,
                Align::Right,
                GlobData.game_state.element_worth[usize::from(element)].to_string(),
            );
            GlobData.sis_state.element_amounts[usize::from(element)]
        };
        g.set_fg_color(if selected { SELECTED } else { AMOUNT });
        text(58, y + 6, Align::Right, amount.to_string());
    }

    unsafe fn display() {
        let g = CffiPlatform;
        let color = |v: Rgba| CColor {
            r: v.r,
            g: v.g,
            b: v.b,
            a: v.a,
        };
        let mut r = CRect {
            corner: CPoint { x: 2, y: 20 },
            width: 60,
            height: 109,
        };
        DrawStarConBox(
            &mut r,
            1,
            color(Rgba::rgb15(16, 16, 16)),
            color(Rgba::rgb15(8, 8, 8)),
            1,
            color(BACK),
        );
        g.set_font(FontId::StarCon);
        g.set_fg_color(SELECTED);
        text(31, 27, Align::Center, g.game_string(CARGO_STRINGS));
        g.set_font(FontId::Tiny);
        let mut frame = g.abs_frame(MiscDataFrame as usize, 11);
        GetFrameRect(frame as *mut c_void, (&mut r as *mut CRect).cast());
        g.set_fg_color(WORTH);
        text(32, 34, Align::Right, "$".into());
        text(58, 34, Align::Right, "#".into());
        for i in 0..8 {
            let y = 35 + i16::from(i) * 9;
            g.set_fg_color(Rgba::BLACK);
            g.fill_rect(Rect {
                corner: Point { x: 7, y },
                width: r.width,
                height: r.height,
            });
            g.draw_stamp(Stamp {
                origin: Point {
                    x: 7 + (r.width >> 1),
                    y: y + (r.height >> 1),
                },
                frame,
            });
            frame = g.rel_frame(frame, 5);
            element_amount(i, false);
        }
        g.set_fg_color(Rgba::BLACK);
        g.fill_rect(Rect {
            corner: Point { x: 7, y: 119 },
            width: r.width,
            height: r.height,
        });
        g.draw_stamp(Stamp {
            origin: Point {
                x: 7 + (r.width >> 1),
                y: 119 + (r.height >> 1),
            },
            frame: g.abs_frame(frame, 68),
        });
        element_amount(8, false);
        g.set_fg_color(WORTH);
        g.fill_rect(Rect {
            corner: Point { x: 4, y: 117 },
            width: 56,
            height: 1,
        });
        text(5, 113, Align::Left, g.game_string(CARGO_STRINGS + 1));
        ShowRemainingCapacity();
    }

    #[no_mangle]
    unsafe extern "C" fn DrawCargoStrings(mut old_element: u8, new_element: u8) {
        let g = CffiPlatform;
        let old = g.set_context(ContextId::Status);
        g.set_font(FontId::Tiny);
        g.batch();
        if old_element > 8 {
            display();
            old_element = new_element;
        }
        if old_element != new_element {
            element_amount(old_element, false);
        }
        if new_element != u8::MAX {
            element_amount(new_element, true);
        }
        g.unbatch();
        g.restore_context(old);
    }

    fn description(element: u8) {
        let message =
            CString::new(CffiPlatform.game_string(CARGO_STRINGS + 2 + i32::from(element))).unwrap();
        unsafe {
            DrawStatusMessage(message.as_ptr());
        }
    }

    #[repr(C)]
    struct Menu {
        input: unsafe extern "C" fn(*mut Menu) -> i32,
        current: u8,
    }

    unsafe extern "C" fn discard(menu: *mut Menu) -> i32 {
        let menu = &mut *menu;
        let pulse = std::ptr::addr_of!(PulsedInputState).read();
        if GlobData.game_state.current_activity & (1 << 14) != 0
            || pulse.menu[KEY_MENU_CANCEL as usize] != 0
        {
            return 0;
        }
        if pulse.menu[KEY_MENU_SELECT as usize] != 0 {
            let amount = &mut GlobData.sis_state.element_amounts[usize::from(menu.current)];
            if *amount != 0 {
                *amount -= 1;
                DrawCargoStrings(menu.current, menu.current);
                GlobData.sis_state.total_element_mass -= 1;
                ShowRemainingCapacity();
            } else {
                PlayMenuSound(2);
            }
        } else {
            let next = next_element(
                menu.current,
                pulse.menu[KEY_MENU_UP as usize] != 0 || pulse.menu[KEY_MENU_LEFT as usize] != 0,
                pulse.menu[KEY_MENU_DOWN as usize] != 0 || pulse.menu[KEY_MENU_RIGHT as usize] != 0,
            );
            if next != menu.current {
                DrawCargoStrings(menu.current, next);
                description(next);
                menu.current = next;
            }
        }
        SleepThread(840 / 30);
        1
    }

    #[no_mangle]
    unsafe extern "C" fn CargoMenu() {
        let mut menu = Menu {
            input: discard,
            current: 0,
        };
        DrawCargoStrings(u8::MAX, 0);
        description(0);
        SetMenuSounds(15, 16);
        DoInput((&mut menu as *mut Menu).cast(), 1);
        ClearSISRect(8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mineral_navigation_wraps_and_back_takes_priority() {
        assert_eq!(next_element(0, true, false), 7);
        assert_eq!(next_element(7, false, true), 0);
        assert_eq!(next_element(3, true, true), 2);
        assert_eq!(next_element(3, false, false), 3);
        for i in 0..8 {
            assert!(next_element(i, false, true) < 8);
        }
    }
}
