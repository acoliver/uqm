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

fn take_character(text: &mut &std::ffi::CStr) -> u32 {
    let Some((character, bytes)) = crate::strings::unicode::utf8_character(text.to_bytes()) else {
        return 0;
    };
    *text = &text[bytes..];
    character
}

fn word_character_count(mut text: &std::ffi::CStr) -> usize {
    let mut count = 0;
    while crate::strings::unicode::uni_char_is_graph(take_character(&mut text)) != 0 {
        count += 1;
    }
    count
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::{character_delay, needs_page_marker, take_character, word_character_count};

    use crate::engine::{battle::Cx, observe::Io};
    use crate::mainloop::input_abi::live::FlushInput;

    use crate::starbase::{
        cffi::CffiPlatform,
        gfx::{Gfx, Rgba},
    };

    use crate::engine::strings::owned_table::TextEntry;
    use std::ffi::{CStr, CString};

    use crate::time::counter_abi::GetTimeCounter;

    use crate::graphics::{
        context_abi::SetContextFontEffect, drawable_abi::DrawableLease, font_abi::SetContextFont,
    };

    use crate::engine::graphics::context_abi::ContextLease;
    use crate::planets::scan::state::create_context;

    use crate::ui::util::{
        wait_for_any_button, wait_for_any_button_until, wait_for_no_input, SavedContext,
    };

    extern "C" {

        static mut ScreenWidth: i32;

    }

    unsafe fn clear(
        input: &crate::engine::input::InputSession,
        junk: &DrawableLease,
        context: &ContextLease,
    ) {
        let g = CffiPlatform;
        let frame = junk
            .context_frame(0)
            .wrapped_frame(if input.preferences().fonts == 2 {
                21
            } else {
                18
            });
        let r = frame.rect();
        g.batch();
        g.set_bg_color(Rgba::BLACK);
        g.clear_drawable();
        g.set_fg_color(Rgba::rgb15(0, 7, 0));
        let start_x = r.extent.width >> 1;
        let mut y = 1;
        for _ in 0..75 / 6 {
            let mut x = start_x;
            for _ in 0..(ScreenWidth - 78) / 6 {
                let point = crate::graphics::native_abi::POINT { x, y };
                if input.preferences().fonts == 2 {
                    context.draw_stamp(&frame, point);
                } else {
                    context.draw_filled_stamp(&frame, point, context.foreground());
                }
                x += r.extent.width + 1;
            }
            y += r.extent.height + 1;
        }
        g.unbatch();
    }

    use crate::battle::sounds::engine::play_sound;

    struct ReportText<'a> {
        string: &'a CStr,
        baseline: crate::comm::locdata::CPoint,
    }

    struct ReportPrinter {
        context: ContextLease,
        font: crate::graphics::native_font::FontLease,
        sound: Option<crate::sound::owned_bank::SoundClip>,
        advance: i16,
        sleepy: bool,
    }

    impl ReportPrinter {
        fn draw_word(
            &mut self,
            cx: &mut Cx<'_>,
            io: &mut Io<'_>,
            t: &mut ReportText<'_>,
            mut word_chars: u16,
            mut timeout: u32,
        ) {
            while word_chars != 0 {
                word_chars -= 1;
                let mut next = t.string;
                let c = take_character(&mut next);
                self.context.draw_text_bounded(
                    &self.font,
                    t.string,
                    crate::graphics::native_abi::POINT {
                        x: t.baseline.x,
                        y: t.baseline.y,
                    },
                    crate::graphics::native_abi::TEXT_ALIGN_ALIGN_LEFT,
                    crate::graphics::native_abi::Color {
                        r: 0,
                        g: 255,
                        b: 0,
                        a: 255,
                    },
                    1,
                );
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
                t.string = next;
                t.baseline.x += self.advance;
            }
        }
    }

    unsafe fn make_report(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        sound: Option<crate::sound::owned_bank::SoundClip>,
        context: &ContextLease,
        font: &crate::graphics::native_font::FontLease,
        text: &CStr,
        mut remaining: u16,
    ) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let g = CffiPlatform;
        let junk = crate::planets::solarsys::resources::retained(cx.kernel)
            .junk
            .lease();
        let (end_page, r) = cx.with_legacy(|| {
            let end_page = CString::new(format!("{}\n", g.game_string(305 + 3))).unwrap();
            let r = junk.context_frame(0).wrapped_frame(18).rect();
            (end_page, r)
        });
        let end_len = crate::strings::unicode::count_utf8_bytes(end_page.as_bytes()) as u16;
        let columns = (ScreenWidth - 78) / 6;
        let rows = 75 / 6;
        let mut t = ReportText {
            string: text,
            baseline: Default::default(),
        };
        let mut printer = ReportPrinter {
            context: context.clone(),
            font: font.clone(),
            sound,
            advance: r.extent.width + 1,
            sleepy: true,
        };
        let mut last_c = 0;
        let mut row = 0;
        cx.with_legacy(|| {
            FlushInput(input);
            if remaining != 0 {
                clear(input, &junk, context);
                g.set_fg_color(Rgba::rgb15(0, 31, 0));
            }
        });
        t.baseline.y = r.extent.height + 1;
        while remaining != 0 {
            let last_string = t.string;
            let mut next = t.string;
            let mut lf_remaining = remaining;
            while lf_remaining != 0 && take_character(&mut next) != u32::from(b'\n') {
                lf_remaining -= 1;
            }
            let mut col: u16 = 0;
            if needs_page_marker(row, rows, remaining, columns, lf_remaining) {
                col = ((columns >> 1) - (i32::from(end_len) >> 1)) as u16;
                t.string = end_page.as_c_str();
                remaining = remaining.wrapping_add(end_len);
            }
            t.baseline.x = ((r.extent.width >> 1) as i32
                + i32::from(col) * i32::from(r.extent.width + 1)) as i16;
            loop {
                let word_chars = word_character_count(t.string) as u16;
                col = col.wrapping_add(word_chars);
                if i32::from(col) <= columns {
                    remaining = remaining.wrapping_sub(word_chars);
                    remaining = remaining.saturating_sub(1);
                    printer.draw_word(cx, io, &mut t, word_chars, GetTimeCounter());
                    col += 1;
                    next = t.string;
                    last_c = take_character(&mut next);
                    t.string = next;
                    t.baseline.x += r.extent.width + 1;
                }
                if i32::from(col) > columns || last_c == u32::from(b'\n') || remaining == 0 {
                    break;
                }
            }
            t.baseline.y += r.extent.height + 1;
            row += 1;
            if row == rows || remaining == 0 {
                t.string = last_string;
                if !printer.sleepy {
                    cx.with_legacy(|| g.unbatch());
                }
                if wait_for_any_button(cx, io, 1, u32::MAX, 0) == 0 {
                    break;
                }
                t.baseline.y = r.extent.height + 1;
                row = 0;
                if remaining != 0 {
                    cx.with_legacy(|| {
                        if !printer.sleepy {
                            g.batch();
                        }
                        clear(input, &junk, context);
                        g.set_fg_color(Rgba::rgb15(0, 31, 0));
                    });
                }
            }
        }
    }

    /// # Safety
    /// The game thread retains its initialized report graphics providers throughout presentation.
    pub unsafe fn discovery_report(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        report: &TextEntry,
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
        let (text, length) = report.current_label();
        let font = typography.font();
        context.with_selected(|| {
            let (saved, old_font, old_effect) = cx.with_legacy(|| {
                let saved = SavedContext::capture(&context, None);
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
                (saved, old_font, old_effect)
            });
            make_report(cx, io, sound, &context, &font, text, length);
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
    fn borrowed_report_cursor_advances_utf8_bytes_without_losing_the_suffix() {
        let mut text = c"Aé中";
        assert_eq!(take_character(&mut text), u32::from(b'A'));
        assert_eq!(text, c"é中");
        assert_eq!(take_character(&mut text), 0xe9);
        assert_eq!(take_character(&mut text), 0x4e2d);
        assert_eq!(text, c"");
        assert_eq!(take_character(&mut text), 0);
        let mut extended =
            std::ffi::CStr::from_bytes_with_nul(&[0xf4, 0x90, 0x80, 0x80, 0]).unwrap();
        assert_eq!(take_character(&mut extended), 0x110000);
        assert_eq!(extended, c"");
    }

    #[test]
    fn borrowed_report_words_keep_engine_graphical_character_boundaries() {
        assert_eq!(word_character_count(c"é中, next"), 3);
        assert_eq!(word_character_count(c"A\nB"), 1);
        assert_eq!(word_character_count(c"A\u{e000}B"), 1);
        assert_eq!(word_character_count(c""), 0);
    }

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
