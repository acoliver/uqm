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
