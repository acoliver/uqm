#![cfg(not(test))]

use super::palette::*;
use crate::engine::{Kernel, WorldData};
use crate::navigation::abi::*;

use super::menu_state::{
    alternate, end_state, PM_CHANGE_CAPTAIN, PM_CHANGE_SHIP, PM_CREW, PM_CYBORG_OFF,
    PM_CYBORG_SUPER, PM_EXIT_SETTINGS, PM_FUEL, PM_MUSIC_OFF, PM_MUSIC_ON, PM_SOUND_OFF,
    PM_SOUND_ON, PM_STARMAP,
};
const OPT_PC: i32 = 2;

/// # Safety
/// Menu state and game settings must be valid on the game thread.
pub unsafe extern "C" fn previous_menu_state(base: BYTE, current: BYTE) -> BYTE {
    super::menu_state::previous_state(
        base,
        current,
        WorldData::current().read(|owner| owner.game.glob_flags),
    )
}

unsafe fn draw_pc_frame(rect: &mut RECT) {
    SetContextForeGroundColor(PCMENU_TOP_LEFT_BORDER_COLOR);
    DrawRectangle(rect);
    rect.corner.x += 1;
    rect.corner.y += 1;
    rect.extent.height -= 1;
    rect.extent.width -= 1;
    SetContextForeGroundColor(PCMENU_BOTTOM_RIGHT_BORDER_COLOR);
    DrawRectangle(rect);
    rect.extent.height -= 1;
    rect.extent.width -= 1;
    SetContextForeGroundColor(PCMENU_BACKGROUND_COLOR);
    DrawFilledRectangle(rect);
}

unsafe fn draw_pc_menu(begin: u8, end: u8, selected: u8, highlight: u8, rect: &mut RECT) {
    const ITEM_HEIGHT: i16 = 8;
    let selected_item = begin.wrapping_add(selected);
    let count = 1 + end - begin;
    rect.corner.x -= 1;
    rect.extent.width += 1;
    DrawFilledRectangle(rect);
    if i16::from(count) * ITEM_HEIGHT > rect.extent.height {
        crate::log_abi::log_message(
            log_Level_log_Error as i32,
            "Warning, no room for all menu items!",
        );
    } else {
        rect.corner.y += (rect.extent.height - i16::from(count) * ITEM_HEIGHT) / 2;
    }
    rect.extent.height = i16::from(count) * ITEM_HEIGHT + 4;
    draw_pc_frame(rect);
    let old_font = SetContextFont(Kernel::current().read(|owner| owner.star_con_font));
    let mut text: TEXT = mem::zeroed();
    let mut buffer = [0 as UNICODE; 256];
    text.align = TEXT_ALIGN_ALIGN_LEFT;
    text.baseline.x = rect.corner.x + 2;
    text.baseline.y = rect.corner.y + ITEM_HEIGHT - 1;
    text.pStr = buffer.as_mut_ptr();
    text.CharCount = COUNT::MAX;
    rect.corner.x += 1;
    rect.extent.width -= 2;
    for item in begin..=end {
        let label = game_string(MAINMENU_STRING_BASE + item as u32);
        if item == PM_FUEL || item == PM_CREW {
            let cost = if item == PM_FUEL {
                WorldData::current().read(|owner| owner.game.fuel_cost)
            } else {
                WorldData::current().read(|owner| owner.game.crew_cost)
            };
            let mut rendered = [0 as UNICODE; 128];
            libc::snprintf(
                rendered.as_mut_ptr().cast(),
                rendered.len(),
                c"%s(%d)".as_ptr(),
                label,
                cost as i32,
            );
            utf8StringCopy(
                (buffer.as_mut_ptr()).cast(),
                mem::size_of_val(&buffer),
                (rendered.as_ptr()).cast(),
            );
        } else {
            utf8StringCopy(
                (buffer.as_mut_ptr()).cast(),
                mem::size_of_val(&buffer),
                (label).cast(),
            );
        }
        if highlight != 0 && selected_item == item {
            SetContextForeGroundColor(PCMENU_SELECTION_BACKGROUND_COLOR);
            rect.corner.y = text.baseline.y - ITEM_HEIGHT + 2;
            rect.extent.height = ITEM_HEIGHT - 1;
            DrawFilledRectangle(rect);
            SetContextForeGroundColor(PCMENU_SELECTION_TEXT_COLOR);
        } else {
            SetContextForeGroundColor(PCMENU_TEXT_COLOR);
        }
        font_DrawText(&mut text);
        text.baseline.y += ITEM_HEIGHT;
    }
    SetContextFont(old_font);
}

/// # Safety
/// Menu state and input globals must be initialized on the game thread.
pub unsafe fn do_menu_chooser(
    input: &crate::engine::input::InputSession,
    menu: *mut MENU_STATE,
    base_state: BYTE,
) -> BOOLEAN {
    let flags = WorldData::current().read(|owner| owner.game.glob_flags);
    choose_menu_state(input, &mut (*menu).CurState, base_state, flags)
}

/// # Safety
/// The game thread owns initialized menu input and display resources.
pub(crate) unsafe fn choose_menu_state(
    input: &crate::engine::input::InputSession,
    state: &mut u8,
    base_state: BYTE,
    flags: u8,
) -> BOOLEAN {
    use super::menu_state::{navigate, MenuKeys};
    let pulsed = input.pulsed();
    let keys = MenuKeys {
        previous: pulsed.menu[KEY_MENU_LEFT as usize] != 0
            || pulsed.menu[KEY_MENU_UP as usize] != 0,
        next: pulsed.menu[KEY_MENU_RIGHT as usize] != 0 || pulsed.menu[KEY_MENU_DOWN as usize] != 0,
        select: pulsed.menu[KEY_MENU_SELECT as usize] != 0,
        cancel: pulsed.menu[KEY_MENU_CANCEL as usize] != 0,
    };
    let Some(step) = navigate(
        base_state,
        *state,
        input.preferences().menu == OPT_PC,
        flags,
        keys,
    ) else {
        return 0;
    };
    DrawMenuStateStrings(step.display.0, step.display.1);
    *state = step.state;
    if step.paced {
        SleepThread(ONE_SECOND / 20);
    }
    1
}

/// # Safety
/// The status context, graphics resources, and menu string tables must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawMenuStateStrings(mut begin: BYTE, mut state: SWORD) {
    let (retained_play_frame, preferences) =
        Kernel::current().read(|owner| (owner.play_frame, owner.configuration.preferences()));
    let highlight = BYTE::from(state >= 0);
    if state < 0 {
        state = -state;
    }
    if preferences.menu == OPT_PC {
        let mut temp = state as u8;
        alternate(&mut begin, &mut temp);
        state = i16::from(temp);
    }
    if begin == PM_STARMAP {
        state -= 1;
    }
    let mut frame: FRAME = ptr::null_mut();
    if state <= i16::from(end_state(begin)) - i16::from(begin) {
        frame = SetAbsFrameIndex(retained_play_frame, (i16::from(begin) + state) as u16);
    }
    pre_update_flash_rect();
    let old_context = SetContext(Kernel::current().read(|owner| owner.status_context));
    let mut rect: RECT = mem::zeroed();
    GetContextClipRect(&mut rect);
    let mut stamp = STAMP {
        origin: POINT {
            x: (ScreenWidth - 60) as i16 - rect.corner.x,
            y: (ScreenHeight - 56) as i16 - rect.corner.y,
        },
        frame,
    };
    rect.corner.x = stamp.origin.x - 1;
    rect.corner.y = stamp.origin.y - 11;
    rect.extent.width = 56 + 2;
    BatchGraphics();
    SetContextForeGroundColor(Color {
        r: 82,
        g: 82,
        b: 82,
        a: 255,
    });
    if !frame.is_null() && preferences.menu == OPT_PC {
        let mut end = end_state(begin);
        let mut selected = state as u8;
        if begin == PM_SOUND_ON {
            end = begin + 5;
            selected = match begin.wrapping_add(selected) {
                PM_SOUND_ON | PM_SOUND_OFF => 0,
                PM_MUSIC_ON | PM_MUSIC_OFF => 1,
                PM_CYBORG_OFF..=PM_CYBORG_SUPER => 2,
                PM_CHANGE_CAPTAIN => 3,
                PM_CHANGE_SHIP => 4,
                PM_EXIT_SETTINGS => 5,
                _ => selected,
            };
        }
        rect.extent.height = 53 + 11;
        draw_pc_menu(begin, end, selected, highlight, &mut rect);
        stamp.frame = ptr::null_mut();
    } else {
        if preferences.menu == OPT_PC {
            rect.corner.x -= 1;
            rect.extent.width += 1;
            rect.extent.height = 53 + 11;
        } else {
            rect.extent.height = 11;
        }
        DrawFilledRectangle(&mut rect);
    }
    if !stamp.frame.is_null() {
        DrawStamp(&mut stamp);
        if begin == PM_CREW || begin == PM_FUEL {
            let value = if begin == PM_CREW {
                WorldData::current().read(|owner| owner.game.crew_cost)
            } else {
                WorldData::current().read(|owner| owner.game.fuel_cost)
            };
            let mut cost = [0 as UNICODE; 20];
            let rendered = format!("{value}");
            for (dst, src) in cost.iter_mut().zip(rendered.bytes()) {
                *dst = src as UNICODE;
            }
            let mut text: TEXT = mem::zeroed();
            text.baseline.x = stamp.origin.x + 56 - 2;
            text.baseline.y = stamp.origin.y + 53 - 2;
            text.align = TEXT_ALIGN_ALIGN_RIGHT;
            text.CharCount = COUNT::MAX;
            text.pStr = cost.as_mut_ptr();
            SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
            SetContextForeGroundColor(THREEDOMENU_TEXT_COLOR);
            font_DrawText(&mut text);
        }
    }
    UnbatchGraphics();
    SetContext(old_context);
    post_update_flash_rect();
}

use std::{mem, ptr};

unsafe fn game_string(index: u32) -> *mut UNICODE {
    GetStringAddress(SetAbsStringTableIndex(
        Kernel::current().read(|owner| owner.game_strings),
        index as u16,
    ))
    .cast()
}
