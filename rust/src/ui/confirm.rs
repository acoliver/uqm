//! Confirmation and popup windows using retained graphics and the supplied input owner.
use super::{palette::*, util::SavedContext};
use crate::engine::{
    battle::Cx,
    input::{frames::*, InputSession},
    observe::Io,
    Kernel,
};
use crate::graphics::{
    context_abi::ContextLease,
    native_abi::{EXTENT, POINT, RECT, TEXT_ALIGN_ALIGN_CENTER},
    native_font::FontLease,
};
use std::ffi::{CStr, CString};
use uqm_engine::time::counter_abi::ONE_SECOND;

const CONFIRM_WIN_WIDTH: i16 = 80;
const CONFIRM_WIN_HEIGHT: i16 = 22;
const QUITMENU_STRING_BASE: usize = 464;

fn draw_confirmation(context: &ContextLease, font: &FontLease, kernel: &Kernel, answer: bool) {
    let (width, height) = context.screen_dimensions();
    let rect = RECT {
        corner: POINT {
            x: (width - i32::from(CONFIRM_WIN_WIDTH)) as i16 >> 1,
            y: (height - i32::from(CONFIRM_WIN_HEIGHT)) as i16 >> 1,
        },
        extent: EXTENT {
            width: CONFIRM_WIN_WIDTH,
            height: CONFIRM_WIN_HEIGHT,
        },
    };
    context.with_text_state(|| {
        context.with_font_effect(None, || {
            context.draw_batch(|| {
                context.draw_shadowed_box(
                    rect,
                    SHADOWBOX_BACKGROUND_COLOR,
                    SHADOWBOX_DARK_COLOR,
                    SHADOWBOX_MEDIUM_COLOR,
                );
                context.draw_text(
                    font,
                    kernel.game_text(QUITMENU_STRING_BASE),
                    POINT {
                        x: rect.corner.x + (rect.extent.width >> 1),
                        y: rect.corner.y + 8,
                    },
                    TEXT_ALIGN_ALIGN_CENTER,
                    MENU_TEXT_COLOR,
                );
                for (index, x, selected) in [
                    (1, rect.corner.x + (rect.extent.width >> 2), answer),
                    (
                        2,
                        rect.corner.x + (rect.extent.width >> 2) + (rect.extent.width >> 1),
                        !answer,
                    ),
                ] {
                    context.draw_text(
                        font,
                        kernel.game_text(QUITMENU_STRING_BASE + index),
                        POINT {
                            x,
                            y: rect.corner.y + 18,
                        },
                        TEXT_ALIGN_ALIGN_CENTER,
                        if selected {
                            MENU_HIGHLIGHT_COLOR
                        } else {
                            MENU_TEXT_COLOR
                        },
                    );
                }
            })
        })
    });
}

fn poll_confirmation(
    cx: &mut Cx<'_>,
    input: &InputSession,
    context: &ContextLease,
    font: &FontLease,
) -> bool {
    let runtime = cx.kernel.input_runtime();
    let mut response = false;
    loop {
        input.set_exit_requested(false);
        input.set_paused(false);
        runtime.update(cx);
        let pulse = input.pulsed();
        let done = if cx.world.activity() & 0x4000 != 0 {
            response = true;
            true
        } else if pulse.menu[KEY_MENU_SELECT as usize] != 0 {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 1);
            true
        } else if pulse.menu[KEY_MENU_CANCEL as usize] != 0 {
            response = false;
            true
        } else {
            if pulse.menu[KEY_MENU_LEFT as usize] != 0 || pulse.menu[KEY_MENU_RIGHT as usize] != 0 {
                response = !response;
                draw_confirmation(context, font, cx.kernel, response);
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 0);
            }
            false
        };
        runtime.sleep(cx, ONE_SECOND / 30);
        if done {
            break;
        }
    }
    response
}

pub fn do_confirm_exit(cx: &mut Cx<'_>, input: &InputSession) -> u32 {
    let highlight = cx.kernel.sis_highlight.clone();
    if crate::sound::heart_ffi::playing_track() != 0 {
        crate::sound::heart_ffi::pause_track();
    }
    highlight.pause_flash();
    let context = ContextLease::require(cx.kernel.screen_context);
    let font = FontLease::retain(
        std::ptr::NonNull::new(cx.kernel.star_con_font.cast())
            .unwrap_or_else(|| panic!("confirmation font")),
    );
    let runtime = cx.kernel.input_runtime();
    let result = context.with_selected(|| {
        context.with_unclipped(|| {
            let bounds = context.clip_rect();
            let rect = RECT {
                corner: POINT {
                    x: (bounds.extent.width - CONFIRM_WIN_WIDTH - 4) >> 1,
                    y: (bounds.extent.height - CONFIRM_WIN_HEIGHT - 4) >> 1,
                },
                extent: EXTENT {
                    width: CONFIRM_WIN_WIDTH + 4,
                    height: CONFIRM_WIN_HEIGHT + 4,
                },
            };
            let saved = SavedContext::capture(&context, Some(rect));
            context.system_rect(Some(rect));
            draw_confirmation(&context, &font, cx.kernel, false);
            context.flush();
            runtime.flush();
            let response = poll_confirmation(cx, input, &context, &font);
            saved.restore();
            drop(saved);
            context.system_rect(None);
            let result = response || cx.world.activity() & 0x4000 != 0;
            if result {
                cx.world.set_activity(cx.world.activity() | 0x4000);
            }
            input.set_exit_requested(false);
            input.set_paused(false);
            runtime.flush();
            u32::from(result)
        })
    });
    highlight.continue_flash();
    if crate::sound::heart_ffi::playing_track() != 0 {
        crate::sound::heart_ffi::resume_track();
    }
    result
}

fn draw_popup(context: &ContextLease, font: &FontLease, lines: &[CString]) -> RECT {
    let (width, height) = context.screen_dimensions();
    let window_width = lines
        .iter()
        .map(|line| crate::strings::unicode::count_utf8_bytes(line.to_bytes()))
        .max()
        .unwrap_or(0) as i32
        * 6
        + 16;
    let window_height = lines.len() as i32 * 8 + 16;
    let mut rect = RECT {
        corner: POINT {
            x: ((width - window_width) >> 1) as i16,
            y: ((height - window_height) >> 1) as i16,
        },
        extent: EXTENT {
            width: window_width as i16,
            height: window_height as i16,
        },
    };
    context.with_text_state(|| {
        context.with_font_effect(None, || {
            context.draw_batch(|| {
                context.draw_shadowed_box(
                    rect,
                    SHADOWBOX_BACKGROUND_COLOR,
                    SHADOWBOX_DARK_COLOR,
                    SHADOWBOX_MEDIUM_COLOR,
                );
                for (i, line) in lines.iter().enumerate() {
                    context.draw_text(
                        font,
                        line,
                        POINT {
                            x: rect.corner.x + (rect.extent.width >> 1),
                            y: rect.corner.y + 16 + i as i16 * 8,
                        },
                        TEXT_ALIGN_ALIGN_CENTER,
                        super::rgb15(0, 0, 0),
                    );
                }
            })
        })
    });
    rect.corner.x -= 2;
    rect.corner.y -= 2;
    rect.extent.width += 4;
    rect.extent.height += 4;
    rect
}

pub fn do_popup_window(cx: &mut Cx<'_>, io: &mut Io<'_>, message: &CStr) {
    let highlight = cx.kernel.sis_highlight.clone();
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let context = ContextLease::require(cx.kernel.screen_context);
    let font = FontLease::retain(
        std::ptr::NonNull::new(cx.kernel.star_con_font.cast())
            .unwrap_or_else(|| panic!("popup font")),
    );
    let lines: Vec<_> = message
        .to_bytes()
        .splitn(30, |byte| *byte == b'\n')
        .map(|line| CString::new(line).unwrap_or_else(|error| panic!("popup line: {error}")))
        .collect();
    highlight.pause_flash();
    context.with_selected(|| {
        context.with_unclipped(|| {
            let saved = SavedContext::capture(&context, None);
            let rect = draw_popup(&context, &font, &lines);
            context.system_rect(Some(rect));
            let sounds = input.menu_sounds();
            input.set_menu_sounds(0, 0);
            let old_callback = input.replace_callback(None);
            cx.run_input(io, true, |cx, _| {
                runtime.sleep(cx, ONE_SECOND / 20);
                input.pulsed().menu[KEY_MENU_SELECT as usize] == 0
                    && input.pulsed().menu[KEY_MENU_CANCEL as usize] == 0
                    && cx.world.activity() & 0x4000 == 0
            });
            input.replace_callback(old_callback);
            context.system_rect(None);
            saved.restore();
            drop(saved);
            highlight.continue_flash();
            input.set_menu_sounds(sounds.0, sounds.1);
        })
    });
}
