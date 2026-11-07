//! Message editing uses bounded font measurements and retained drawing targets.
use super::*;
use crate::graphics::context_abi::ContextFrame;
use std::{ffi::CStr, ptr::NonNull};

fn draw_cursor(
    context: &ContextLease,
    measurement: &crate::graphics::native_font::TextMeasurement,
    cursor: i16,
    flags: u16,
    height: i16,
) {
    let count = usize::from(measurement.characters);
    if cursor >= 0 && cursor as usize <= count {
        let index = cursor as usize;
        let mut rect = measurement.rect;
        rect.corner.x = measurement.deltas[..index]
            .iter()
            .fold(rect.corner.x, |x, &delta| x.wrapping_add(i16::from(delta)));
        if index < count {
            rect.corner.x = rect.corner.x.wrapping_sub(1);
        }
        rect.extent.width = if flags & DSME_BLOCKCUR as u16 != 0 && index < count {
            i16::from(measurement.deltas[index]) + if index + 1 == count { 2 } else { 1 }
        } else {
            1
        };
        rect.corner.y = 0;
        rect.extent.height = height;
        context.fill_rectangle(rgb15(0, 0, 0), rect);
    }
}

fn draw_edit(
    context: &ContextLease,
    font: &crate::graphics::native_font::FontLease,
    text: &CStr,
    clip: RECT,
    edit: (i16, i16, u16),
) -> bool {
    let (cursor, extra, flags) = edit;
    let width = clip.extent.width;
    let mut baseline = POINT {
        x: 3,
        y: SIS_MESSAGE_HEIGHT - 2,
    };
    let measurement = font.measure(text, baseline, TEXT_ALIGN_ALIGN_LEFT, u16::MAX);
    if i32::from(measurement.rect.extent.width) + i32::from(baseline.x) + 2 >= i32::from(width) {
        return false;
    }
    context.clear();
    let count = usize::from(measurement.characters);
    draw_cursor(context, &measurement, cursor, flags, clip.extent.height);
    if extra >= 0 && (extra as usize) < count {
        context.draw_text_bounded(
            font,
            text,
            baseline,
            TEXT_ALIGN_ALIGN_LEFT,
            rgb15(27, 0, 27),
            extra as u16,
        );
        baseline.x = measurement.deltas[..extra as usize]
            .iter()
            .fold(baseline.x, |x, &delta| x.wrapping_add(i16::from(delta)));
        let (prefix, _) =
            crate::strings::unicode::split_utf8_prefix(text.to_bytes(), extra as usize);
        let suffix = CStr::from_bytes_with_nul(&text.to_bytes_with_nul()[prefix.len()..])
            .unwrap_or_else(|error| panic!("invalid message suffix: {error}"));
        context.draw_text(
            font,
            suffix,
            baseline,
            TEXT_ALIGN_ALIGN_LEFT,
            rgb15(18, 0, 18),
        );
    } else {
        context.draw_text(
            font,
            text,
            baseline,
            TEXT_ALIGN_ALIGN_LEFT,
            rgb15(27, 0, 27),
        );
    }
    true
}

pub fn draw_message(
    kernel: &Kernel,
    screen_width: i32,
    text: &CStr,
    cursor: i16,
    extra: i16,
    flags: u16,
) -> bool {
    let context = ContextLease::require(kernel.off_screen_context);
    let frame = ContextFrame::retain(NonNull::new(kernel.screen));
    let font = kernel
        .tiny_font_lease()
        .unwrap_or_else(|| panic!("SIS message requires its font"));
    let width =
        (screen_width - 78 - i32::from(SIS_TITLE_BOX_WIDTH) - i32::from(SIS_SPACER_BOX_WIDTH) - 2)
            as i16;
    let clip = RECT {
        corner: POINT {
            x: SIS_ORG_X + 1,
            y: SIS_ORG_Y - SIS_MESSAGE_HEIGHT,
        },
        extent: EXTENT {
            width,
            height: SIS_MESSAGE_HEIGHT - 1,
        },
    };
    context.with_selected(|| {
        context.set_frame(frame);
        context.set_clip(clip);
        context.set_background(rgb15(0, 0, 20));
        let color = if flags & DSME_MYCOLOR as u16 == 0 {
            rgb15(27, 0, 27)
        } else {
            context.foreground()
        };
        context.set_foreground(color);
        context.set_font(&font);
        if flags & DSME_CLEARFR as u16 != 0 {
            *kernel.sis_highlight.context.borrow_mut() = None;
        }
        let drawn = context.with_batch(|| {
            let baseline = POINT {
                x: width >> 1,
                y: SIS_MESSAGE_HEIGHT - 2,
            };
            if cursor < 0 && extra < 0 {
                context.clear();
                context.draw_text(&font, text, baseline, TEXT_ALIGN_ALIGN_CENTER, color);
            } else {
                if !draw_edit(&context, &font, text, clip, (cursor, extra, flags)) {
                    return false;
                }
            }
            if flags & DSME_SETFR as u16 != 0 {
                let screen = ContextLease::require(kernel.screen_context);
                set_flash_rect(
                    &kernel.sis_highlight,
                    &screen,
                    clip,
                    Some(RECT {
                        corner: POINT { x: 0, y: 0 },
                        extent: clip.extent,
                    }),
                );
            }
            true
        });
        context.clear_clip();
        drawn
    })
}

pub fn set_menu_flash(kernel: &Kernel, dimensions: (i32, i32), any: bool) {
    if !any && kernel.configuration.preferences().menu == OPT_PC {
        *kernel.sis_highlight.context.borrow_mut() = None;
    } else {
        let status = ContextLease::require(kernel.status_context);
        let screen = ContextLease::require(kernel.screen_context);
        status.with_selected(|| {
            let clip = status.clip_rect();
            set_flash_rect(
                &kernel.sis_highlight,
                &screen,
                clip,
                Some(RECT {
                    corner: POINT {
                        x: (dimensions.0 - 60) as i16 - clip.corner.x,
                        y: (dimensions.1 - 56) as i16 - clip.corner.y,
                    },
                    extent: EXTENT {
                        width: RADAR_WIDTH,
                        height: RADAR_HEIGHT,
                    },
                }),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::graphics::{
        context_abi::{ContextFrame, ContextLease},
        dcqueue_abi::TFB_FlushGraphics,
        drawable_abi::GetFramePixel,
        native_abi::*,
    };
    use crate::ships::native::abi::{
        DSME_BLOCKCUR, DSME_CLEARFR, DSME_MYCOLOR, DSME_SETFR, OPT_3DO, OPT_PC,
    };
    use crate::{campaign_display_test as campaign_display, engine::Game, ui};
    /// Run process-global Flash fixtures without retaining allocations on another test's thread.
    fn own_graphics_process(case: &str) -> bool {
        let case = case.strip_prefix("uqm_rust::").unwrap_or(case);
        const CHILD: &str = "UQM_CAMPAIGN_DISPLAY_CASE";
        if std::env::var(CHILD).as_deref() == Ok(case) {
            return true;
        }
        let status = std::process::Command::new(
            std::env::current_exe()
                .unwrap_or_else(|error| panic!("test executable unavailable: {error}")),
        )
        .args(["--exact", case, "--nocapture"])
        .env(CHILD, case)
        .status()
        .unwrap_or_else(|error| panic!("campaign panel child failed: {error}"));
        assert!(status.success(), "campaign panel child: {status}");
        false
    }

    #[test]
    #[serial_test::serial]
    fn ui_message_edits_draw_bounded_cursor_and_split_color_then_restore_the_context() {
        if !own_graphics_process(concat!(
            module_path!(),
            "::ui_message_edits_draw_bounded_cursor_and_split_color_then_restore_the_context"
        )) {
            return;
        }
        let absent = Game::new();
        let Err(error) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ui::sis::draw_message(&absent.kernel, 320, c"A", -1, -1, 0)
        })) else {
            panic!("missing message context was accepted")
        };
        assert_eq!(
            error.downcast_ref::<&str>().copied(),
            Some("required context is absent")
        );

        let mut game = Game::new();
        let display = campaign_display::CampaignDisplay::install(&mut game)
            .unwrap_or_else(|error| panic!("campaign panel fixture failed: {error}"));
        uqm_rust::callback::ffi::Alarm_init();
        game.kernel.screen_context = game.kernel.off_screen_context;
        let context = ContextLease::require(game.kernel.off_screen_context);
        context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
            game.kernel.screen,
        )));
        let font = game
            .kernel
            .tiny_font_lease()
            .unwrap_or_else(|| panic!("campaign panel fixture requires its tiny font"));
        let measurement =
            font.measure(c"AA", POINT { x: 3, y: 6 }, TEXT_ALIGN_ALIGN_LEFT, u16::MAX);
        let glyph = measurement.rect.corner;
        assert_eq!(measurement.characters, 2);
        let second = i16::from(measurement.deltas[0]);
        for (cursor, extra, flags) in [
            (0, 0, DSME_BLOCKCUR),
            (1, 1, DSME_BLOCKCUR | DSME_CLEARFR),
            (2, -1, 0),
            (99, 99, 0),
            (-1, 0, 0),
            (0, -1, 0),
        ] {
            assert!(ui::sis::draw_message(
                &game.kernel,
                320,
                c"AA",
                cursor,
                extra,
                flags as u16
            ));
            assert_eq!(context.clip_rect().extent.width, 320);
            // SAFETY: the fixture owns the retained screen; flushing completes the edit before pixel reads.
            unsafe {
                TFB_FlushGraphics();
                for (offset, color) in [
                    (
                        0,
                        if extra == 0 {
                            (148, 0, 148)
                        } else {
                            (222, 0, 222)
                        },
                    ),
                    (
                        second,
                        if extra == 0 || extra == 1 {
                            (148, 0, 148)
                        } else {
                            (222, 0, 222)
                        },
                    ),
                ] {
                    let pixel = GetFramePixel(
                        game.kernel.screen,
                        POINT {
                            x: 8 + glyph.x + offset,
                            y: 2 + glyph.y,
                        },
                    );
                    assert_eq!((pixel.r, pixel.g, pixel.b), color);
                }
                if (0..=2).contains(&cursor) {
                    let x = 8
                        + glyph.x
                        + measurement.deltas[..cursor as usize]
                            .iter()
                            .map(|&x| i16::from(x))
                            .sum::<i16>()
                        - i16::from(cursor < 2);
                    let pixel = GetFramePixel(game.kernel.screen, POINT { x, y: 2 });
                    assert_eq!((pixel.r, pixel.g, pixel.b), (0, 0, 0));
                }
            }
            assert_eq!(
                game.kernel.sis_highlight.context.borrow().is_some(),
                flags & DSME_SETFR != 0
            );
        }
        assert!(ui::sis::draw_message(
            &game.kernel,
            320,
            c"AA",
            -1,
            -1,
            DSME_SETFR as u16
        ));
        assert!(game.kernel.sis_highlight.context.borrow().is_some());
        assert!(ui::sis::draw_message(
            &game.kernel,
            320,
            c"AA",
            -1,
            -1,
            DSME_CLEARFR as u16
        ));
        assert!(game.kernel.sis_highlight.context.borrow().is_none());
        context.set_foreground(Color {
            r: 19,
            g: 37,
            b: 53,
            a: 255,
        });
        assert!(ui::sis::draw_message(
            &game.kernel,
            320,
            c"AA",
            -1,
            -1,
            DSME_MYCOLOR as u16
        ));
        assert_eq!(context.foreground().r, 19);
        game.kernel
            .configuration
            .update_preferences(|preferences| preferences.menu = OPT_PC as i32);
        ui::sis::set_menu_flash(&game.kernel, (320, 240), true);
        assert!(game.kernel.sis_highlight.context.borrow().is_some());
        ui::sis::set_menu_flash(&game.kernel, (320, 240), false);
        assert!(game.kernel.sis_highlight.context.borrow().is_none());
        game.kernel
            .configuration
            .update_preferences(|preferences| preferences.menu = OPT_3DO as i32);
        ui::sis::set_menu_flash(&game.kernel, (320, 240), false);
        assert!(game.kernel.sis_highlight.context.borrow().is_some());
        game.kernel.sis_highlight.context.borrow_mut().take();
        uqm_rust::callback::ffi::Alarm_uninit();
        let long_label = std::ffi::CString::new("A".repeat(100))
            .unwrap_or_else(|error| panic!("fixture label: {error}"));
        assert!(!ui::sis::draw_message(
            &game.kernel,
            320,
            &long_label,
            0,
            -1,
            0
        ));
        assert_eq!(context.clip_rect().extent.width, 320);
        let screen = game.kernel.screen;
        game.kernel.screen = std::ptr::null_mut();
        assert!(ui::sis::draw_message(&game.kernel, 320, c"A", -1, -1, 0));
        game.kernel.screen = screen;
        context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(screen)));
        drop(font);
        game.kernel.release_tiny_font();
        let Err(error) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ui::sis::draw_message(&game.kernel, 320, c"A", -1, -1, 0)
        })) else {
            panic!("missing message font was accepted")
        };
        assert_eq!(
            error.downcast_ref::<&str>().copied(),
            Some("SIS message requires its font")
        );
        drop(context);
        display.finish(game);
    }
}
