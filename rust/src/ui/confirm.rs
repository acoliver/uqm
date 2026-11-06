#![cfg(not(test))]

use super::palette::*;
use crate::engine::{input::InputSession, Kernel};
use crate::navigation::abi::*;

use crate::ui::util::live::SaveContextFrame;
use std::{mem, ptr};

const CONFIRM_WIN_WIDTH: i16 = 80;
const CONFIRM_WIN_HEIGHT: i16 = 22;

unsafe fn draw_confirmation(kernel: &Kernel, answer: BOOLEAN) {
    let old_fg = SetContextForeGroundColor(MENU_TEXT_COLOR);
    let old_font = SetContextFont(kernel.star_con_font);
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let mut rect = RECT {
        corner: POINT {
            x: (ScreenWidth - CONFIRM_WIN_WIDTH as i32) as i16 >> 1,
            y: (ScreenHeight - CONFIRM_WIN_HEIGHT as i32) as i16 >> 1,
        },
        extent: EXTENT {
            width: CONFIRM_WIN_WIDTH,
            height: CONFIRM_WIN_HEIGHT,
        },
    };
    let mut text: TEXT = mem::zeroed();
    BatchGraphics();
    DrawShadowedBox(
        &mut rect,
        SHADOWBOX_BACKGROUND_COLOR,
        SHADOWBOX_DARK_COLOR,
        SHADOWBOX_MEDIUM_COLOR,
    );
    text.baseline = POINT {
        x: rect.corner.x + (rect.extent.width >> 1),
        y: rect.corner.y + 8,
    };
    text.pStr = kernel
        .game_text((QUITMENU_STRING_BASE) as usize)
        .as_ptr()
        .cast();
    text.align = TEXT_ALIGN_ALIGN_CENTER;
    text.CharCount = COUNT::MAX;
    font_DrawText(&mut text);
    text.baseline.y += 10;
    text.baseline.x = rect.corner.x + (rect.extent.width >> 2);
    text.pStr = kernel
        .game_text((QUITMENU_STRING_BASE + 1) as usize)
        .as_ptr()
        .cast();
    SetContextForeGroundColor(if answer != 0 {
        MENU_HIGHLIGHT_COLOR
    } else {
        MENU_TEXT_COLOR
    });
    font_DrawText(&mut text);
    text.baseline.x += rect.extent.width >> 1;
    text.pStr = kernel
        .game_text((QUITMENU_STRING_BASE + 2) as usize)
        .as_ptr()
        .cast();
    SetContextForeGroundColor(if answer != 0 {
        MENU_TEXT_COLOR
    } else {
        MENU_HIGHLIGHT_COLOR
    });
    font_DrawText(&mut text);
    UnbatchGraphics();
    SetContextFontEffect(old_effect);
    SetContextFont(old_font);
    SetContextForeGroundColor(old_fg);
}

/// # Safety
/// Input, graphics, flash, audio, and game state must be initialized on the game thread.
pub unsafe fn do_confirm_exit(
    cx: &mut crate::engine::battle::Cx<'_>,
    input: &InputSession,
) -> BOOLEAN {
    if PlayingTrack() != 0 {
        PauseTrack();
    }
    pause_flash();
    let mut rect: RECT = mem::zeroed();
    let mut context_rect: RECT = mem::zeroed();
    let mut old_rect: RECT = mem::zeroed();
    let old_context = SetContext(cx.kernel.screen_context);
    GetContextClipRect(&mut old_rect);
    SetContextClipRect(ptr::null_mut());
    GetContextClipRect(&mut context_rect);
    rect.extent.width = CONFIRM_WIN_WIDTH + 4;
    rect.extent.height = CONFIRM_WIN_HEIGHT + 4;
    rect.corner.x = (context_rect.extent.width - rect.extent.width) >> 1;
    rect.corner.y = (context_rect.extent.height - rect.extent.height) >> 1;
    let mut saved = SaveContextFrame(&rect);
    SetSystemRect(&rect);
    let mut response = 0;
    draw_confirmation(cx.kernel, response);
    FlushGraphics();
    FlushInput(input);
    let mut done = false;
    while !done {
        input.set_exit_requested(false);
        input.set_paused(false);
        UpdateInputState(cx, input);
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            done = true;
            response = 1;
        } else if input.pulsed().menu[KEY_MENU_SELECT as usize] != 0 {
            done = true;
            crate::ui::settings::play_menu_sound(
                cx.kernel,
                cx.world,
                MENU_SOUND_EFFECT_MENU_SOUND_SUCCESS as u16,
            );
        } else if input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0 {
            done = true;
            response = 0;
        } else if input.pulsed().menu[KEY_MENU_LEFT as usize] != 0
            || input.pulsed().menu[KEY_MENU_RIGHT as usize] != 0
        {
            response = u32::from(response == 0);
            draw_confirmation(cx.kernel, response);
            crate::ui::settings::play_menu_sound(
                cx.kernel,
                cx.world,
                MENU_SOUND_EFFECT_MENU_SOUND_MOVE as u16,
            );
        }
        SleepThread(ONE_SECOND / 30);
    }
    DrawStamp(&mut saved);
    DestroyDrawable(ReleaseDrawable(saved.frame));
    ClearSystemRect();
    let result = if response != 0 || cx.world.activity() & CHECK_ABORT as u16 != 0 {
        cx.world
            .set_activity(cx.world.activity() | CHECK_ABORT as u16);
        1
    } else {
        0
    };
    input.set_exit_requested(false);
    input.set_paused(false);
    FlushInput(input);
    SetContextClipRect(&mut old_rect);
    SetContext(old_context);
    continue_flash();
    if PlayingTrack() != 0 {
        ResumeTrack();
    }
    result
}

unsafe fn do_popup(cx: &mut crate::engine::battle::Cx<'_>, input: &InputSession) -> BOOLEAN {
    cx.with_legacy(|| SleepThread(ONE_SECOND / 20));
    u32::from(
        input.pulsed().menu[KEY_MENU_SELECT as usize] == 0
            && input.pulsed().menu[KEY_MENU_CANCEL as usize] == 0
            && cx.world.activity() & CHECK_ABORT as u16 == 0,
    )
}

/// # Safety
/// msg must be a NUL-terminated UTF-8 string; widget, graphics, and input systems must be initialized.
pub unsafe fn do_popup_window(runtime: &InputSession, msg: *const ::std::ffi::c_char) {
    let input = runtime;
    let bank = StringBank_Create();
    if bank.is_null() {
        crate::log_abi::log_message(
            log_Level_log_Fatal as i32,
            "FATAL: Memory exhaustion when preparing popup window",
        );
        std::process::exit(1);
    }
    let mut lines: [*const UNICODE; 30] = [ptr::null(); 30];
    let mut label: WIDGET_LABEL = mem::zeroed();
    label.tag = WIDGET_TYPE_WIDGET_TYPE_LABEL;
    label.parent = ptr::null_mut();
    label.handleEvent = Some(widget_handle_event_ignore_all);
    label.receiveFocus = Some(Widget_ReceiveFocusRefuseFocus);
    label.draw = Some(Widget_DrawLabel);
    label.height = Some(Widget_HeightLabel);
    label.width = Some(Widget_WidthFullScreen);
    label.line_count = SplitString(
        msg.cast(),
        b'\n' as std::ffi::c_char,
        30,
        lines.as_mut_ptr(),
        bank,
    );
    label.lines = lines.as_mut_ptr();
    pause_flash();
    let old_context = SetContext(Kernel::current().read(|owner| owner.screen_context));
    let mut old_rect: RECT = mem::zeroed();
    GetContextClipRect(&mut old_rect);
    SetContextClipRect(ptr::null_mut());
    let mut saved = SaveContextFrame(ptr::null());
    Widget_SetFont(Kernel::current().read(|owner| owner.star_con_font));
    Widget_SetWindowColors(
        SHADOWBOX_BACKGROUND_COLOR,
        SHADOWBOX_DARK_COLOR,
        SHADOWBOX_MEDIUM_COLOR,
    );
    let mut window_rect: RECT = mem::zeroed();
    DrawLabelAsWindow(&mut label, &mut window_rect);
    SetSystemRect(&window_rect);
    let mut sound0: MENU_SOUND_FLAGS = mem::zeroed();
    let mut sound1: MENU_SOUND_FLAGS = mem::zeroed();
    GetMenuSounds(input, &mut sound0, &mut sound1);
    SetMenuSounds(input, 0, 0);
    let old_callback = SetInputCallback(runtime, None);
    DoInput(
        1,
        &mut |cx: &mut crate::engine::battle::Cx<'_>, _: &mut crate::engine::observe::Io<'_>| {
            do_popup(cx, input) != 0
        },
    );
    SetInputCallback(runtime, old_callback);
    ClearSystemRect();
    DrawStamp(&mut saved);
    DestroyDrawable(ReleaseDrawable(saved.frame));
    SetContextClipRect(&mut old_rect);
    SetContext(old_context);
    continue_flash();
    SetMenuSounds(input, sound0, sound1);
    StringBank_Free(bank);
}
