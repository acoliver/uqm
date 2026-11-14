//! Discovery report paging and typewriter presentation from `planets/report.c`.

pub mod typography;

#[cfg(test)]
mod presentation_tests;

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

pub(crate) mod live {
    use super::{character_delay, needs_page_marker, take_character, word_character_count};
    use crate::engine::{battle::Cx, observe::Io, strings::owned_table::TextEntry};
    use crate::graphics::{
        context_abi::{BatchGuard, ContextFrame, ContextLease},
        native_abi::{Color, POINT},
        native_font::FontLease,
    };
    use crate::planets::scan::state::create_context;
    use crate::time::counter_abi::GetTimeCounter;
    use crate::ui::util::{
        wait_for_any_button, wait_for_any_button_until, wait_for_no_input, SavedContext,
    };
    use std::ffi::{CStr, CString};

    const GREEN: Color = Color {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    const DARK_GREEN: Color = Color {
        r: 0,
        g: 57,
        b: 0,
        a: 255,
    };

    fn clear(printer: &ReportPrinter, input: &crate::engine::input::InputSession) {
        let frame = printer
            .junk
            .wrapped_frame(if input.preferences().fonts == 2 {
                21
            } else {
                18
            });
        let r = frame.rect();
        printer.context.with_batch(|| {
            printer.context.set_background(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            });
            printer.context.clear();
            printer.context.set_foreground(DARK_GREEN);
            let mut y = 1;
            for _ in 0..75 / 6 {
                let mut x = r.extent.width >> 1;
                for _ in 0..(printer.width - 78) / 6 {
                    if input.preferences().fonts == 2 {
                        printer.context.draw_stamp(&frame, POINT { x, y });
                    } else {
                        printer
                            .context
                            .draw_filled_stamp(&frame, POINT { x, y }, DARK_GREEN);
                    }
                    x += r.extent.width + 1;
                }
                y += r.extent.height + 1;
            }
        });
    }

    use crate::battle::sounds::engine::play_sound;

    struct ReportText<'a> {
        string: &'a CStr,
        baseline: crate::comm::locdata::CPoint,
    }

    struct ReportPrinter {
        context: ContextLease,
        font: FontLease,
        junk: ContextFrame,
        width: i32,
        batch: Option<BatchGuard>,
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
                    GREEN,
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
                        self.batch = Some(self.context.batch());
                    }
                }
                t.string = next;
                t.baseline.x += self.advance;
            }
        }
    }

    fn make_report(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        printer: &mut ReportPrinter,
        text: &CStr,
        mut remaining: u16,
    ) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let end_page = CString::new(format!("{}\n", cx.kernel.game_text(308).to_string_lossy()))
            .unwrap_or_else(|error| panic!("report page marker contains NUL: {error}"));
        let r = printer.junk.wrapped_frame(18).rect();
        let end_len = crate::strings::unicode::count_utf8_bytes(end_page.as_bytes()) as u16;
        let columns = (printer.width - 78) / 6;
        let rows = 75 / 6;
        let mut t = ReportText {
            string: text,
            baseline: Default::default(),
        };
        let mut last_c = 0;
        let mut row = 0;
        runtime.flush();
        if remaining != 0 {
            clear(printer, input);
            printer.context.set_foreground(GREEN);
        }
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
                printer.batch.take();
                if wait_for_any_button(cx, io, 1, u32::MAX, 0) == 0 {
                    break;
                }
                t.baseline.y = r.extent.height + 1;
                row = 0;
                if remaining != 0 {
                    if !printer.sleepy {
                        printer.batch = Some(printer.context.batch());
                    }
                    clear(printer, input);
                    printer.context.set_foreground(GREEN);
                }
            }
        }
    }

    pub fn discovery_report(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        report: &TextEntry,
        scan: Option<&ContextLease>,
        typography: &super::typography::ReportTypography,
        junk: ContextFrame,
    ) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let sound = cx.kernel.menu_sound_bank().map(|bank| bank.clip(0));
        let width = ContextLease::require(cx.kernel.space_context)
            .screen_dimensions()
            .0;
        let (context, owned) = match scan {
            Some(context) => (context.clone(), None),
            None => {
                let owner = create_context(cx.kernel, width as i16 - 78);
                (owner.lease(), Some(owner))
            }
        };
        let (text, length) = report.current_label();
        let effect = (input.preferences().fonts == 2).then(|| typography.effect().context_frame(0));
        let mut printer = ReportPrinter {
            context: context.clone(),
            font: typography.font(),
            advance: junk.wrapped_frame(18).rect().extent.width + 1,
            junk,
            width,
            batch: None,
            sound,
            sleepy: true,
        };
        context.with_selected(|| {
            let saved = SavedContext::capture(&context, None);
            context.with_text_state(|| {
                context.with_font_effect(effect.as_ref(), || {
                    make_report(cx, io, &mut printer, text, length);
                });
            });
            if length != 0 {
                context.set_foreground(GREEN);
            }
            saved.restore();
        });
        drop(owned);
        wait_for_no_input(cx, io, u32::MAX, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn retained_report_batch_releases_on_return_and_panic() {
        use crate::graphics::{batch_depth::depth, context_abi::LoadedContext, dcqueue_abi};
        struct Queue;
        impl Drop for Queue {
            fn drop(&mut self) {
                // SAFETY: this serialized fixture owns the queue.
                unsafe { dcqueue_abi::Uninit_DrawCommandQueue() };
            }
        }
        // SAFETY: this serialized fixture owns the queue until its guard drops.
        unsafe { dcqueue_abi::Init_DrawCommandQueue() };
        let _queue = Queue;
        let owner = LoadedContext::create(c"report-batch-test");
        let context = owner.lease();
        {
            let _batch = context.batch();
            assert_eq!(depth(&context), 1);
        }
        assert_eq!(depth(&context), 0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _batch = context.batch();
            assert_eq!(depth(&context), 1);
            panic!("interrupted report");
        }));
        assert!(result.is_err());
        assert_eq!(depth(&context), 0);
    }

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
