#![cfg(not(test))]
#![allow(non_snake_case)]

use super::palette::*;
use crate::navigation::abi::*;
use crate::navigation::globals::{GlobData, PlayFrame};

use super::menu_state::{
    end_state, next_state, ALT_EXIT_MANIFEST, ALT_MANIFEST, PM_ALT_ASCAN, PM_ALT_BSCAN,
    PM_ALT_CARGO, PM_ALT_DEVICES, PM_ALT_DISPATCH, PM_ALT_ESCAN, PM_ALT_EXIT_MANIFEST,
    PM_ALT_EXIT_SCAN, PM_ALT_GAME_MENU, PM_ALT_MANIFEST, PM_ALT_MSCAN, PM_ALT_NAVIGATE,
    PM_ALT_ROSTER, PM_ALT_SCAN, PM_ALT_STARMAP, PM_AUTO_SCAN, PM_BIO_SCAN, PM_CARGO,
    PM_CHANGE_CAPTAIN, PM_CHANGE_SHIP, PM_CREW, PM_CYBORG_OFF, PM_CYBORG_SUPER, PM_DEVICES,
    PM_ENE_SCAN, PM_EXIT_SCAN, PM_EXIT_SETTINGS, PM_FUEL, PM_GAME_MENU, PM_LAUNCH_LANDER,
    PM_MIN_SCAN, PM_MUSIC_OFF, PM_MUSIC_ON, PM_NAVIGATE, PM_ROSTER, PM_SCAN, PM_SOUND_OFF,
    PM_SOUND_ON, PM_STARMAP,
};
const OPT_PC: i32 = 2;

/// # Safety
/// Menu state and game settings must be valid on the game thread.
pub unsafe extern "C" fn PreviousMenuState(base: BYTE, current: BYTE) -> BYTE {
    super::menu_state::previous_state(base, current, GlobData.Game_state.glob_flags)
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
    let old_font = SetContextFont(StarConFont);
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
                GlobData.Game_state.FuelCost
            } else {
                GlobData.Game_state.CrewCost
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

unsafe fn alternate(base: &mut u8, current: &mut u8) -> bool {
    let adj = u8::from(*base == PM_STARMAP);
    let adjusted_base = base.wrapping_sub(adj);
    if *current & 0x80 != 0 {
        match *current {
            ALT_MANIFEST => {
                *base = PM_ALT_SCAN + adj;
                *current = PM_ALT_MANIFEST - PM_ALT_SCAN - adj;
                true
            }
            ALT_EXIT_MANIFEST => {
                *base = PM_ALT_CARGO;
                *current = PM_ALT_EXIT_MANIFEST - PM_ALT_CARGO;
                true
            }
            _ => {
                crate::log_abi::log_message(
                    log_Level_log_Error as i32,
                    &format!(
                        "Unknown state combination: {}, {}",
                        *base as i32, *current as i32
                    ),
                );
                false
            }
        }
    } else {
        let result = match adjusted_base.wrapping_add(*current) {
            PM_SCAN => Some((PM_ALT_SCAN, 0)),
            PM_STARMAP => Some((PM_ALT_SCAN + adj, PM_ALT_STARMAP - PM_ALT_SCAN - adj)),
            PM_DEVICES => Some((PM_ALT_CARGO, PM_ALT_DEVICES - PM_ALT_CARGO)),
            PM_CARGO => Some((PM_ALT_CARGO, 0)),
            PM_ROSTER => Some((PM_ALT_CARGO, PM_ALT_ROSTER - PM_ALT_CARGO)),
            PM_GAME_MENU => Some((PM_ALT_SCAN + adj, PM_ALT_GAME_MENU - PM_ALT_SCAN - adj)),
            PM_NAVIGATE => Some((PM_ALT_SCAN + adj, PM_ALT_NAVIGATE - PM_ALT_SCAN - adj)),
            PM_MIN_SCAN => Some((PM_ALT_MSCAN, 0)),
            PM_ENE_SCAN => Some((PM_ALT_MSCAN, PM_ALT_ESCAN - PM_ALT_MSCAN)),
            PM_BIO_SCAN => Some((PM_ALT_MSCAN, PM_ALT_BSCAN - PM_ALT_MSCAN)),
            PM_EXIT_SCAN => Some((PM_ALT_MSCAN, PM_ALT_EXIT_SCAN - PM_ALT_MSCAN)),
            PM_AUTO_SCAN => Some((PM_ALT_MSCAN, PM_ALT_ASCAN - PM_ALT_MSCAN)),
            PM_LAUNCH_LANDER => Some((PM_ALT_MSCAN, PM_ALT_DISPATCH - PM_ALT_MSCAN)),
            _ => None,
        };
        if let Some((new_base, new_offset)) = result {
            *base = new_base;
            *current = new_offset;
            true
        } else {
            false
        }
    }
}

fn convert_alternate(base: u8, state: u8) -> u8 {
    match base.wrapping_add(state) {
        PM_ALT_SCAN => 0,
        PM_ALT_STARMAP => PM_STARMAP - PM_SCAN,
        PM_ALT_MANIFEST => ALT_MANIFEST,
        PM_ALT_GAME_MENU => PM_GAME_MENU - PM_SCAN,
        PM_ALT_NAVIGATE => PM_NAVIGATE - PM_SCAN,
        PM_ALT_CARGO => PM_CARGO - PM_SCAN,
        PM_ALT_DEVICES => PM_DEVICES - PM_SCAN,
        PM_ALT_ROSTER => PM_ROSTER - PM_SCAN,
        PM_ALT_EXIT_MANIFEST => ALT_EXIT_MANIFEST,
        PM_ALT_MSCAN => 0,
        PM_ALT_ESCAN => PM_ENE_SCAN - PM_MIN_SCAN,
        PM_ALT_BSCAN => PM_BIO_SCAN - PM_MIN_SCAN,
        PM_ALT_ASCAN => PM_AUTO_SCAN - PM_MIN_SCAN,
        PM_ALT_DISPATCH => PM_LAUNCH_LANDER - PM_MIN_SCAN,
        PM_ALT_EXIT_SCAN => PM_EXIT_SCAN - PM_MIN_SCAN,
        _ => state,
    }
}

/// # Safety
/// Menu state and input globals must be initialized on the game thread.
#[no_mangle]
pub unsafe extern "C" fn DoMenuChooser(menu: *mut MENU_STATE, base_state: BYTE) -> BOOLEAN {
    let mut new_state = (*menu).CurState;
    let original_base = base_state;
    let mut base = base_state;
    let use_alt = if optWhichMenu == OPT_PC {
        alternate(&mut base, &mut new_state)
    } else {
        false
    };
    if PulsedInputState.menu[KEY_MENU_LEFT as usize] != 0
        || PulsedInputState.menu[KEY_MENU_UP as usize] != 0
    {
        new_state = PreviousMenuState(base, new_state);
    } else if PulsedInputState.menu[KEY_MENU_RIGHT as usize] != 0
        || PulsedInputState.menu[KEY_MENU_DOWN as usize] != 0
    {
        new_state = next_state(base, new_state, GlobData.Game_state.glob_flags);
    } else if use_alt && PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0 {
        new_state = convert_alternate(base, new_state);
        if new_state == ALT_MANIFEST {
            DrawMenuStateStrings(PM_ALT_CARGO, 0);
            (*menu).CurState = PM_CARGO - PM_SCAN;
            return 1;
        }
        if new_state == ALT_EXIT_MANIFEST {
            if original_base == PM_SCAN {
                DrawMenuStateStrings(PM_ALT_SCAN, (PM_ALT_MANIFEST - PM_ALT_SCAN) as i16);
            } else {
                DrawMenuStateStrings(PM_ALT_STARMAP, (PM_ALT_MANIFEST - PM_ALT_STARMAP) as i16);
            }
            (*menu).CurState = ALT_MANIFEST;
            return 1;
        }
        return 0;
    } else if optWhichMenu == OPT_PC
        && PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0
        && base == PM_ALT_CARGO
    {
        if original_base == PM_SCAN {
            DrawMenuStateStrings(PM_ALT_SCAN, (PM_ALT_MANIFEST - PM_ALT_SCAN) as i16);
        } else {
            DrawMenuStateStrings(PM_ALT_STARMAP, (PM_ALT_MANIFEST - PM_ALT_STARMAP) as i16);
        }
        (*menu).CurState = ALT_MANIFEST;
        return 1;
    } else {
        return 0;
    }
    DrawMenuStateStrings(base, i16::from(new_state));
    if use_alt {
        new_state = convert_alternate(base, new_state);
    }
    (*menu).CurState = new_state;
    SleepThread(ONE_SECOND / 20);
    1
}

/// # Safety
/// The status context, graphics resources, and menu string tables must be initialized.
#[no_mangle]
pub unsafe extern "C" fn DrawMenuStateStrings(begin: BYTE, state: SWORD) {
    let mut begin = begin;
    let mut state = state;
    let mut highlight = 1;
    if state < 0 {
        state = -state;
        highlight = 0;
    }
    if optWhichMenu == OPT_PC {
        let mut temp = state as u8;
        alternate(&mut begin, &mut temp);
        state = i16::from(temp);
    }
    if begin == PM_STARMAP {
        state -= 1;
    }
    let mut frame: FRAME = ptr::null_mut();
    if state <= i16::from(end_state(begin)) - i16::from(begin) {
        frame = SetAbsFrameIndex(PlayFrame, (i16::from(begin) + state) as u16);
    }
    PreUpdateFlashRect();
    let old_context = SetContext(StatusContext);
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
    if !frame.is_null() && optWhichMenu == OPT_PC {
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
        if optWhichMenu == OPT_PC {
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
                GlobData.Game_state.CrewCost
            } else {
                GlobData.Game_state.FuelCost
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
            SetContextFont(TinyFont);
            SetContextForeGroundColor(THREEDOMENU_TEXT_COLOR);
            font_DrawText(&mut text);
        }
    }
    UnbatchGraphics();
    SetContext(old_context);
    PostUpdateFlashRect();
}

use std::{mem, ptr};

unsafe fn game_string(index: u32) -> *mut UNICODE {
    GetStringAddress(SetAbsStringTableIndex(GameStrings, index as u16)).cast()
}
