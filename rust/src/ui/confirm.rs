#![cfg(any(not(test), feature = "linked_c_archive"))]
#![allow(non_snake_case)]

use super::palette::*;
use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use crate::ui::util::live::SaveContextFrame;
use std::{mem, ptr};

const CONFIRM_WIN_WIDTH: i16 = 80;
const CONFIRM_WIN_HEIGHT: i16 = 22;

unsafe fn draw_confirmation(answer: BOOLEAN) {
    let old_fg = SetContextForeGroundColor(MENU_TEXT_COLOR);
    let old_font = SetContextFont(StarConFont);
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
    text.pStr = game_string(QUITMENU_STRING_BASE);
    text.align = TEXT_ALIGN_ALIGN_CENTER;
    text.CharCount = COUNT::MAX;
    font_DrawText(&mut text);
    text.baseline.y += 10;
    text.baseline.x = rect.corner.x + (rect.extent.width >> 2);
    text.pStr = game_string(QUITMENU_STRING_BASE + 1);
    SetContextForeGroundColor(if answer != 0 {
        MENU_HIGHLIGHT_COLOR
    } else {
        MENU_TEXT_COLOR
    });
    font_DrawText(&mut text);
    text.baseline.x += rect.extent.width >> 1;
    text.pStr = game_string(QUITMENU_STRING_BASE + 2);
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
#[no_mangle]
pub unsafe extern "C" fn DoConfirmExit() -> BOOLEAN {
    if PlayingTrack() != 0 {
        PauseTrack();
    }
    PauseFlash();
    let mut rect: RECT = mem::zeroed();
    let mut context_rect: RECT = mem::zeroed();
    let mut old_rect: RECT = mem::zeroed();
    let old_context = SetContext(ScreenContext);
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
    draw_confirmation(response);
    FlushGraphics();
    FlushInput();
    let mut done = false;
    while !done {
        ExitRequested = 0;
        GamePaused = 0;
        UpdateInputState();
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            done = true;
            response = 1;
        } else if PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0 {
            done = true;
            PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_SUCCESS);
        } else if PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0 {
            done = true;
            response = 0;
        } else if PulsedInputState.menu[KEY_MENU_LEFT as usize] != 0
            || PulsedInputState.menu[KEY_MENU_RIGHT as usize] != 0
        {
            response = u32::from(response == 0);
            draw_confirmation(response);
            PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_MOVE);
        }
        SleepThread(ONE_SECOND / 30);
    }
    DrawStamp(&mut saved);
    DestroyDrawable(ReleaseDrawable(saved.frame));
    ClearSystemRect();
    let result = if response != 0 || GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        GlobData.Game_state.CurrentActivity |= CHECK_ABORT as u16;
        1
    } else {
        0
    };
    ExitRequested = 0;
    GamePaused = 0;
    FlushInput();
    SetContextClipRect(&mut old_rect);
    SetContext(old_context);
    ContinueFlash();
    if PlayingTrack() != 0 {
        ResumeTrack();
    }
    result
}

#[repr(C)]
struct PopupState {
    InputFunc: unsafe extern "C" fn(*mut PopupState) -> BOOLEAN,
}
unsafe extern "C" fn do_popup(_state: *mut PopupState) -> BOOLEAN {
    SleepThread(ONE_SECOND / 20);
    u32::from(
        PulsedInputState.menu[KEY_MENU_SELECT as usize] == 0
            && PulsedInputState.menu[KEY_MENU_CANCEL as usize] == 0
            && GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0,
    )
}

/// # Safety
/// msg must be a NUL-terminated UTF-8 string; widget, graphics, and input systems must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DoPopupWindow(msg: *const ::std::ffi::c_char) {
    let bank = StringBank_Create();
    if bank.is_null() {
        log_add(
            log_Level_log_Fatal,
            c"FATAL: Memory exhaustion when preparing popup window".as_ptr(),
        );
        std::process::exit(1);
    }
    let mut lines: [*const UNICODE; 30] = [ptr::null(); 30];
    let mut label: WIDGET_LABEL = mem::zeroed();
    label.tag = WIDGET_TYPE_WIDGET_TYPE_LABEL;
    label.parent = ptr::null_mut();
    label.handleEvent = Some(Widget_HandleEventIgnoreAll);
    label.receiveFocus = Some(Widget_ReceiveFocusRefuseFocus);
    label.draw = Some(Widget_DrawLabel);
    label.height = Some(Widget_HeightLabel);
    label.width = Some(Widget_WidthFullScreen);
    label.line_count = SplitString(msg.cast(), b'\n' as i8, 30, lines.as_mut_ptr(), bank);
    label.lines = lines.as_mut_ptr();
    PauseFlash();
    let old_context = SetContext(ScreenContext);
    let mut old_rect: RECT = mem::zeroed();
    GetContextClipRect(&mut old_rect);
    SetContextClipRect(ptr::null_mut());
    let mut saved = SaveContextFrame(ptr::null());
    Widget_SetFont(StarConFont);
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
    GetMenuSounds(&mut sound0, &mut sound1);
    SetMenuSounds(0, 0);
    let old_callback = SetInputCallback(None);
    let mut state = PopupState {
        InputFunc: do_popup,
    };
    DoInput((&mut state as *mut PopupState).cast(), 1);
    SetInputCallback(old_callback);
    ClearSystemRect();
    DrawStamp(&mut saved);
    DestroyDrawable(ReleaseDrawable(saved.frame));
    SetContextClipRect(&mut old_rect);
    SetContext(old_context);
    ContinueFlash();
    SetMenuSounds(sound0, sound1);
    StringBank_Free(bank);
}

unsafe fn game_string(index: u32) -> *mut UNICODE {
    GetStringAddress(SetAbsStringTableIndex(GameStrings, index as u16)).cast()
}
