//! Discovery report paging and typewriter presentation from `planets/report.c`.

pub mod typography;

fn character_delay(character: u32, last_in_word: bool) -> u32 {
    let mut delay = 0;
    if character == u32::from(b',') {
        delay += 840 / 4;
    }
    if matches!(character, 46 | 33 | 63) {
        delay += 840 / 2;
    } else {
        delay += 840 / 20;
    }
    if last_in_word {
        delay += 840 / 20;
    }
    delay
}

fn needs_page_marker(row: i32, rows: i32, remaining: u16, columns: i32, lf_remaining: u16) -> bool {
    row == rows - 1 && (i32::from(remaining) > columns || lf_remaining > 1)
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::{character_delay, needs_page_marker};
    use crate::comm::{locdata::CRect, sis_graphics::CText};

    use crate::engine::{battle::Cx, observe::Io};
    use crate::mainloop::input_abi::live::FlushInput;

    use crate::starbase::{
        cffi::CffiPlatform,
        gfx::{Gfx, Point, Rgba, Stamp},
    };

    use crate::strings::{
        string_table::{GetStringAddress, GetStringLength},
        unicode::{
            get_char_from_string as getCharFromString, uni_char_is_graph as UniChar_isGraph,
            utf8_string_count as utf8StringCount, utf8_string_count_n as utf8StringCountN,
        },
    };
    use std::ffi::CString;

    use crate::time::counter_abi::GetTimeCounter;

    use crate::graphics::{
        context_abi::SetContextFontEffect,
        drawable_abi::GetFrameRect,
        font_abi::{font_DrawText, SetContextFont},
        pixmap_abi::SetAbsFrameIndex,
    };

    use crate::engine::graphics::context_abi::ContextLease;
    use crate::planets::scan::state::create_context;

    use crate::ui::util::{
        wait_for_any_button, wait_for_any_button_until, wait_for_no_input, SavedContext,
    };

    extern "C" {
        static mut SpaceJunkFrame: *mut crate::graphics::native_abi::frame_desc;

        static mut ScreenWidth: i32;

    }

    unsafe fn clear(input: &crate::engine::input::InputSession) {
        let g = CffiPlatform;
        let frame = SetAbsFrameIndex(
            (SpaceJunkFrame).cast(),
            if input.preferences().fonts == 2 {
                21
            } else {
                18
            },
        );
        let mut r = CRect::default();
        GetFrameRect(frame, (&mut r as *mut CRect).cast());
        g.batch();
        g.set_bg_color(Rgba::BLACK);
        g.clear_drawable();
        g.set_fg_color(Rgba::rgb15(0, 7, 0));
        let start_x = r.width >> 1;
        let mut y = 1;
        for _ in 0..75 / 6 {
            let mut x = start_x;
            for _ in 0..(ScreenWidth - 78) / 6 {
                let stamp = Stamp {
                    origin: Point { x, y },
                    frame: frame as usize,
                };
                if input.preferences().fonts == 2 {
                    g.draw_stamp(stamp);
                } else {
                    g.draw_filled_stamp(stamp);
                }
                x += r.width + 1;
            }
            y += r.height + 1;
        }
        g.unbatch();
    }

    use crate::battle::sounds::engine::play_sound;

    struct ReportPrinter {
        sound: Option<crate::sound::owned_bank::SoundClip>,
        advance: i16,
        sleepy: bool,
    }

    impl ReportPrinter {
        unsafe fn draw_word(
            &mut self,
            cx: &mut Cx<'_>,
            io: &mut Io<'_>,
            t: &mut CText,
            mut word_chars: u16,
            mut timeout: u32,
        ) {
            while word_chars != 0 {
                word_chars -= 1;
                let mut next = t.p_str.cast();
                let c = getCharFromString(&mut next);
                cx.with_legacy(|| font_DrawText(std::ptr::from_mut(t).cast()));
                if self.sleepy && cx.world.activity() & (1 << 14) == 0 {
                    if let Some(sound) = &self.sound {
                        play_sound(
                            cx.world,
                            &mut cx.kernel.channels.borrow_mut(),
                            sound,
                            crate::sound::types::SoundPosition::non_positional(),
                            None,
                            2,
                        );
                    }
                    timeout = timeout.wrapping_add(character_delay(c, word_chars == 0));
                    if wait_for_any_button_until(cx, io, 1, timeout, 0) != 0 {
                        self.sleepy = false;
                        cx.with_legacy(|| CffiPlatform.batch());
                    }
                }
                t.p_str = next.cast();
                t.baseline.x += self.advance;
            }
        }
    }

    unsafe fn make_report(
        input: &crate::engine::input::InputSession,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        sound: Option<crate::sound::owned_bank::SoundClip>,
        text: *const u8,
        mut remaining: u16,
    ) {
        let g = CffiPlatform;
        let (end_page, r) = cx.with_legacy(|| {
            let end_page = CString::new(format!("{}\n", g.game_string(305 + 3))).unwrap();
            let mut r = CRect::default();
            GetFrameRect(
                SetAbsFrameIndex((SpaceJunkFrame).cast(), 18),
                (&mut r as *mut CRect).cast(),
            );
            (end_page, r)
        });
        let end_len = utf8StringCount(end_page.as_ptr().cast()) as u16;
        let columns = (ScreenWidth - 78) / 6;
        let rows = 75 / 6;
        let mut t = CText::default();
        t.p_str = text.cast();
        t.char_count = 1;
        let mut printer = ReportPrinter {
            sound,
            advance: r.width + 1,
            sleepy: true,
        };
        let mut last_c = 0;
        let mut row = 0;
        cx.with_legacy(|| {
            FlushInput(input);
            if remaining != 0 {
                clear(input);
                g.set_fg_color(Rgba::rgb15(0, 31, 0));
            }
        });
        t.baseline.y = r.height + 1;
        while remaining != 0 {
            let last_string = t.p_str;
            let mut next = t.p_str.cast::<u8>();
            let mut lf_remaining = remaining;
            while lf_remaining != 0 && getCharFromString(&mut next) != u32::from(b'\n') {
                lf_remaining -= 1;
            }
            let mut col: u16 = 0;
            if needs_page_marker(row, rows, remaining, columns, lf_remaining) {
                col = ((columns >> 1) - (i32::from(end_len) >> 1)) as u16;
                t.p_str = end_page.as_ptr();
                remaining = remaining.wrapping_add(end_len);
            }
            t.baseline.x = ((r.width >> 1) as i32 + i32::from(col) * i32::from(r.width + 1)) as i16;
            loop {
                let mut end = t.p_str.cast::<u8>();
                next = end;
                while UniChar_isGraph(getCharFromString(&mut next)) != 0 {
                    end = next;
                }
                let word_chars = utf8StringCountN(t.p_str.cast(), end) as u16;
                col = col.wrapping_add(word_chars);
                if i32::from(col) <= columns {
                    remaining = remaining.wrapping_sub(word_chars);
                    remaining = remaining.saturating_sub(1);
                    printer.draw_word(cx, io, &mut t, word_chars, GetTimeCounter());
                    col += 1;
                    next = t.p_str.cast();
                    last_c = getCharFromString(&mut next);
                    t.p_str = next.cast();
                    t.baseline.x += r.width + 1;
                }
                if i32::from(col) > columns || last_c == u32::from(b'\n') || remaining == 0 {
                    break;
                }
            }
            t.baseline.y += r.height + 1;
            row += 1;
            if row == rows || remaining == 0 {
                t.p_str = last_string;
                if !printer.sleepy {
                    cx.with_legacy(|| g.unbatch());
                }
                if wait_for_any_button(cx, io, 1, u32::MAX, 0) == 0 {
                    break;
                }
                t.baseline.y = r.height + 1;
                row = 0;
                if remaining != 0 {
                    cx.with_legacy(|| {
                        if !printer.sleepy {
                            g.batch();
                        }
                        clear(input);
                        g.set_fg_color(Rgba::rgb15(0, 31, 0));
                    });
                }
            }
        }
    }

    /// # Safety
    /// The game thread publishes the active owner for the generator callback.
    pub unsafe fn discovery_report(
        system: *mut crate::ships::native::abi::SOLARSYS_STATE,
        scan: Option<&ContextLease>,
        typography: &super::typography::ReportTypography,
    ) {
        crate::app::bridge::enter(|cx| {
            let mut observer = cx.kernel.input_runtime().observer();
            let mut io = Io {
                observer: &mut *observer,
            };
            do_discovery_report(cx, &mut io, system, scan, typography);
        });
    }

    pub unsafe fn do_discovery_report(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        system: *mut crate::ships::native::abi::SOLARSYS_STATE,
        scan: Option<&ContextLease>,
        typography: &super::typography::ReportTypography,
    ) {
        #[cfg(feature = "debug-process")]
        if crate::front_end::uqmdebug::DISABLE_INTERACTIVITY != 0 {
            return;
        }
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let sound = cx.kernel.menu_sound_bank().map(|bank| bank.clip(0));
        let (context, owned) = match scan {
            Some(context) => (context.clone(), None),
            None => {
                let owner = create_context(cx.kernel, CffiPlatform.screen_size().0 as i16 - 78);
                (owner.lease(), Some(owner))
            }
        };
        context.with_selected(|| {
            let (saved, old_font, old_effect, text, length) = cx.with_legacy(|| {
                let saved = SavedContext::capture(&context, None);
                let info = std::ptr::addr_of!((*system).SysInfo.planet_info);
                let old_font = SetContextFont(typography.font.as_raw().cast());
                let old_effect = SetContextFontEffect(if input.preferences().fonts == 2 {
                    typography
                        .effect()
                        .frame(0)
                        .unwrap_or_else(|| panic!("report effect frame"))
                        .cast()
                } else {
                    std::ptr::null_mut()
                });
                (
                    saved,
                    old_font,
                    old_effect,
                    GetStringAddress((*info).discovery_string.cast()).cast(),
                    GetStringLength((*info).discovery_string.cast()),
                )
            });
            make_report(input, cx, io, sound, text, length);
            cx.with_legacy(|| {
                SetContextFontEffect(old_effect);
                SetContextFont(old_font);
                saved.restore();
            })
        });
        drop(owned);
        wait_for_no_input(cx, io, u32::MAX, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn punctuation_and_word_end_delays_match_the_readout() {
        assert_eq!(character_delay(u32::from(b'a'), false), 42);
        assert_eq!(character_delay(u32::from(b'a'), true), 84);
        assert_eq!(character_delay(u32::from(b','), false), 252);
        assert_eq!(character_delay(u32::from(b'.'), true), 462);
        assert_eq!(character_delay(u32::from(b'!'), false), 420);
        assert_eq!(character_delay(u32::from(b'?'), false), 420);
    }
    #[test]
    fn page_marker_reserves_last_row_only_when_text_does_not_fit() {
        assert!(!needs_page_marker(10, 12, 100, 40, 100));
        assert!(!needs_page_marker(11, 12, 40, 40, 0));
        assert!(!needs_page_marker(11, 12, 40, 40, 1));
        assert!(needs_page_marker(11, 12, 40, 40, 2));
        assert!(needs_page_marker(11, 12, 41, 40, 0));
    }
}
