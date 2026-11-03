//! Game options, naming, and save/load selection, ported from `uqm/gameopt.c`.

const MAX_SAVED_GAMES: usize = 50;
const SAVES_PER_PAGE: u16 = 5;

fn picker_slot(current: u8, left: bool, right: bool, up: bool, down: bool) -> u8 {
    let mut next = current;
    if left {
        if next == 0 {
            next = (MAX_SAVED_GAMES - 1) as u8;
        } else if i32::from(next) - i32::from(SAVES_PER_PAGE) > 0 {
            next -= SAVES_PER_PAGE as u8;
        } else {
            next = 0;
        }
    } else if right {
        if next == (MAX_SAVED_GAMES - 1) as u8 {
            next = 0;
        } else if i32::from(next) + i32::from(SAVES_PER_PAGE) < (MAX_SAVED_GAMES - 1) as i32 {
            next += SAVES_PER_PAGE as u8;
        } else {
            next = (MAX_SAVED_GAMES - 1) as u8;
        }
    } else if up {
        if next == 0 {
            next = (MAX_SAVED_GAMES - 1) as u8;
        } else {
            next -= 1;
        }
    } else if down {
        if next == (MAX_SAVED_GAMES - 1) as u8 {
            next = 0;
        } else {
            next += 1;
        }
    }
    next
}

/// The original DoPickGame exit priority, independent of presentation and I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerAction {
    Continue,
    Cancel,
    Abort,
    Select,
    EmptySlot,
}

pub fn picker_action(
    abort: bool,
    cancel: bool,
    select: bool,
    saving: bool,
    occupied: bool,
) -> PickerAction {
    if abort {
        PickerAction::Abort
    } else if cancel {
        PickerAction::Cancel
    } else if select {
        if saving || occupied {
            PickerAction::Select
        } else {
            PickerAction::EmptySlot
        }
    } else {
        PickerAction::Continue
    }
}

#[cfg(not(test))]
pub use runtime::{load_game_descriptions, ConfirmSaveLoad, GameOptions, SetNamingCallback};

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::{picker_action, picker_slot, PickerAction, MAX_SAVED_GAMES, SAVES_PER_PAGE};
    use crate::engine::{Kernel, StoryKey, World, WorldData};
    use crate::navigation::abi::*;

    use std::{mem, ptr};

    const SUMMARY_X_OFFS: i16 = 14;
    const SUMMARY_SIDE_OFFS: i16 = 7;
    const MAX_NAME_SIZE: usize = SIS_NAME_SIZE as usize;

    const SAVE_GAME: u8 = 0;
    const LOAD_GAME: u8 = 1;
    const QUIT_GAME: u8 = 2;
    const SETTINGS: u8 = 3;
    const EXIT_GAME_MENU: u8 = 4;

    const SOUND_ON_SETTING: u8 = 0;
    const SOUND_OFF_SETTING: u8 = 1;
    const MUSIC_ON_SETTING: u8 = 2;
    const MUSIC_OFF_SETTING: u8 = 3;
    const CYBORG_OFF_SETTING: u8 = 4;
    const CYBORG_NORMAL_SETTING: u8 = 5;
    const CYBORG_DOUBLE_SETTING: u8 = 6;
    const CYBORG_SUPER_SETTING: u8 = 7;
    const CHANGE_CAPTAIN_SETTING: u8 = 8;
    const CHANGE_SHIP_SETTING: u8 = 9;
    const EXIT_SETTINGS_MENU: u8 = 10;

    const DDSHS_NORMAL: COUNT = 0;
    const DDSHS_EDIT: COUNT = 1;
    const DDSHS_BLOCKCUR: COUNT = 2;

    // Cast-valued macros in sounds.h and sis.h are not emitted by bindgen.
    const MENU_SOUND_ARROWS: MENU_SOUND_FLAGS = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3);
    const MENU_SOUND_SELECT: MENU_SOUND_FLAGS = 1 << 4;
    const MENU_SOUND_PAGEUP: MENU_SOUND_FLAGS = 1 << 7;
    const MENU_SOUND_PAGEDOWN: MENU_SOUND_FLAGS = 1 << 8;
    const SFR_MENU_3DO: *const RECT = usize::MAX as *const RECT;

    static mut LAST_USED_SLOT: COUNT = 0;
    static mut NAMING_CB: NamingCallback = None;

    const CAPTAIN_NAME_RECT: RECT = RECT {
        corner: POINT { x: 3, y: 10 },
        extent: EXTENT {
            width: (SHIP_NAME_WIDTH - 2) as i16,
            height: SHIP_NAME_HEIGHT as i16,
        },
    };
    const SHIP_NAME_RECT: RECT = RECT {
        corner: POINT { x: 2, y: 20 },
        extent: EXTENT {
            width: SHIP_NAME_WIDTH as i16,
            height: SHIP_NAME_HEIGHT as i16,
        },
    };

    // BUILD_COLOR(col, c256) is col; CC5TO8 replicates the high bits.
    const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 0xff,
        }
    }
    const BLACK_COLOR: Color = rgb15(0, 0, 0);
    const CAPTAIN_NAME_TEXT_COLOR: Color = rgb15(0x16, 0x0b, 0x1f);

    unsafe fn game_string(index: u32) -> *mut UNICODE {
        GetStringAddress(SetAbsStringTableIndex(
            Kernel::current().read(|owner| owner.game_strings),
            index as COUNT,
        ))
    }

    unsafe fn key(index: u32) -> bool {
        PulsedInputState.menu[index as usize] != 0
    }

    unsafe fn sis_screen_width() -> i32 {
        ScreenWidth - STATUS_WIDTH as i32 - (SAFE_X as i32 * 2) - 14
    }

    unsafe fn logx_to_universe(lx: SDWORD) -> COORD {
        let space_width = ScreenWidth - STATUS_WIDTH as i32 - (SAFE_X as i32 * 2);
        let log_space_width = (space_width << ONE_SHIFT) << MAX_REDUCTION;
        let log_units_x = (log_space_width >> 4) * SECTOR_WIDTH as i32;
        (lx.wrapping_mul(UNIVERSE_UNITS_X as i32)
            .wrapping_add(log_units_x >> 1)
            / log_units_x) as COORD
    }

    unsafe fn logy_to_universe(ly: SDWORD) -> COORD {
        let space_height = ScreenHeight - (SAFE_Y as i32 * 2);
        let log_space_height = (space_height << ONE_SHIFT) << MAX_REDUCTION;
        let log_units_y = (log_space_height >> 4) * SECTOR_HEIGHT as i32;
        (MAX_Y_UNIVERSE as i32
            - ly.wrapping_mul(UNIVERSE_UNITS_Y as i32)
                .wrapping_add(log_units_y >> 1)
                / log_units_y) as COORD
    }

    /// # Safety
    /// Call on the game thread with a live graphics context. A non-null stamp
    /// must be writable; its captured drawable belongs to the caller.
    #[no_mangle]
    pub unsafe extern "C" fn ConfirmSaveLoad(msg_stamp: *mut STAMP) {
        let mut r: RECT = mem::zeroed();
        let mut clip_r: RECT = mem::zeroed();
        SetContextFont(Kernel::current().read(|owner| owner.star_con_font));
        GetContextClipRect(&mut clip_r);
        let mut t = TEXT {
            baseline: POINT {
                x: clip_r.extent.width >> 1,
                y: (clip_r.extent.height >> 1) + 3,
            },
            align: TEXT_ALIGN_ALIGN_CENTER,
            CharCount: COUNT::MAX,
            pStr: game_string(SAVEGAME_STRING_BASE + u32::from(msg_stamp.is_null())),
        };
        TextRect(&mut t, &mut r, ptr::null_mut());
        r.corner.x -= 4;
        r.corner.y -= 4;
        r.extent.width += 8;
        r.extent.height += 8;
        if !msg_stamp.is_null() {
            msg_stamp.write(SaveContextFrame(&r));
        }
        DrawStarConBox(
            &mut r,
            2,
            rgb15(0x10, 0x10, 0x10),
            rgb15(0x08, 0x08, 0x08),
            1,
            rgb15(0x0a, 0x0a, 0x0a),
        );
        SetContextForeGroundColor(rgb15(0x14, 0x14, 0x14));
        font_DrawText(&mut t);
    }

    unsafe fn feedback_setting(which_setting: BYTE) {
        let mut buf: [UNICODE; 128] = [0; 128];
        match which_setting {
            SOUND_ON_SETTING | SOUND_OFF_SETTING => {
                libc::snprintf(
                    buf.as_mut_ptr(),
                    buf.len() - 1,
                    c"%s %s".as_ptr(),
                    game_string(OPTION_STRING_BASE),
                    game_string(
                        OPTION_STRING_BASE
                            + if WorldData::current().read(|owner| owner.game.glob_flags)
                                & SOUND_DISABLED as u8
                                != 0
                            {
                                3
                            } else {
                                4
                            },
                    ),
                );
            }
            MUSIC_ON_SETTING | MUSIC_OFF_SETTING => {
                libc::snprintf(
                    buf.as_mut_ptr(),
                    buf.len() - 1,
                    c"%s %s".as_ptr(),
                    game_string(OPTION_STRING_BASE + 1),
                    game_string(
                        OPTION_STRING_BASE
                            + if WorldData::current().read(|owner| owner.game.glob_flags)
                                & MUSIC_DISABLED as u8
                                != 0
                            {
                                3
                            } else {
                                4
                            },
                    ),
                );
            }
            CYBORG_OFF_SETTING
            | CYBORG_NORMAL_SETTING
            | CYBORG_DOUBLE_SETTING
            | CYBORG_SUPER_SETTING => {
                let tmpstr =
                    if optWhichMenu == OPT_PC as i32 && which_setting > CYBORG_NORMAL_SETTING {
                        if which_setting == CYBORG_DOUBLE_SETTING {
                            c"+"
                        } else {
                            c"++"
                        }
                    } else {
                        c""
                    };
                libc::snprintf(
                    buf.as_mut_ptr(),
                    buf.len() - 1,
                    c"%s %s%s".as_ptr(),
                    game_string(OPTION_STRING_BASE + 2),
                    game_string(
                        OPTION_STRING_BASE
                            + if WorldData::current().read(|owner| owner.game.glob_flags)
                                & CYBORG_ENABLED as u8
                                == 0
                            {
                                3
                            } else {
                                4
                            },
                    ),
                    tmpstr.as_ptr(),
                );
            }
            CHANGE_CAPTAIN_SETTING | CHANGE_SHIP_SETTING => {
                utf8StringCopy(
                    (buf.as_mut_ptr()).cast(),
                    buf.len(),
                    (game_string(NAMING_STRING_BASE)).cast(),
                );
            }
            _ => {}
        }
        DrawStatusMessage(buf.as_ptr());
    }

    unsafe fn draw_name_string(
        name_captain: bool,
        str: *mut UNICODE,
        cursor_pos: COUNT,
        state: COUNT,
    ) -> BOOLEAN {
        let (font, mut r, back_ground, fore_ground) = if name_captain {
            (
                Kernel::current().read(|owner| owner.tiny_font),
                CAPTAIN_NAME_RECT,
                rgb15(0x0a, 0x0a, 0x1f),
                rgb15(0x0a, 0x1f, 0x1f),
            )
        } else {
            (
                Kernel::current().read(|owner| owner.star_con_font),
                SHIP_NAME_RECT,
                rgb15(0x0f, 0, 0),
                rgb15(0x1f, 0x0a, 0),
            )
        };
        let mut lf = TEXT {
            baseline: POINT {
                x: r.corner.x + (r.extent.width >> 1) - i16::from(name_captain),
                y: r.corner.y + r.extent.height - 1,
            },
            align: TEXT_ALIGN_ALIGN_CENTER,
            pStr: str,
            CharCount: COUNT::MAX,
        };
        SetContext(Kernel::current().read(|owner| owner.status_context));
        SetContextFont(font);
        if state & DDSHS_EDIT == 0 {
            if name_captain {
                draw_captains_name();
            } else {
                DrawFlagshipName(1);
            }
        } else {
            let mut text_r: RECT = mem::zeroed();
            let mut char_deltas = [0u8; MAX_NAME_SIZE];
            TextRect(&mut lf, &mut text_r, char_deltas.as_mut_ptr());
            if i32::from(text_r.extent.width) + 2 >= i32::from(r.extent.width) {
                return 0;
            }
            pre_update_flash_rect();
            SetContextForeGroundColor(back_ground);
            DrawFilledRectangle(&mut r);
            let mut pchar_deltas = char_deltas.as_ptr();
            for _ in 0..cursor_pos {
                text_r.corner.x = text_r.corner.x.wrapping_add(*pchar_deltas as i16);
                pchar_deltas = pchar_deltas.add(1);
            }
            if cursor_pos < lf.CharCount {
                text_r.corner.x -= 1;
            }
            if state & DDSHS_BLOCKCUR != 0 {
                if cursor_pos == lf.CharCount {
                    text_r.extent.width = 1;
                } else if i32::from(cursor_pos) + 1 == i32::from(lf.CharCount) {
                    text_r.extent.width = *pchar_deltas as SIZE + 2;
                } else {
                    text_r.extent.width = *pchar_deltas as SIZE + 1;
                }
            } else {
                text_r.extent.width = 1;
            }
            text_r.corner.y = r.corner.y;
            text_r.extent.height = r.extent.height;
            SetContextForeGroundColor(BLACK_COLOR);
            DrawFilledRectangle(&mut text_r);
            SetContextForeGroundColor(fore_ground);
            font_DrawText(&mut lf);
            post_update_flash_rect();
        }
        1
    }

    unsafe extern "C" fn on_name_change(p_tes: *mut TEXTENTRY_STATE) -> BOOLEAN {
        let name_captain = !(*p_tes).CbParam.is_null();
        let mut hl = DDSHS_EDIT;
        if (*p_tes).JoystickMode != 0 {
            hl |= DDSHS_BLOCKCUR;
        }
        draw_name_string(
            name_captain,
            (*p_tes).BaseStr,
            (*p_tes).CursorPos as COUNT,
            hl,
        )
    }

    unsafe fn name_captain_or_ship(name_captain: bool) {
        let world = WorldData::current();
        let mut buf: [UNICODE; MAX_NAME_SIZE] = [0; MAX_NAME_SIZE];
        let mut tes: TEXTENTRY_STATE = mem::zeroed();
        SetContext(Kernel::current().read(|owner| owner.status_context));
        SetFlashRect(if name_captain {
            &CAPTAIN_NAME_RECT
        } else {
            &SHIP_NAME_RECT
        });
        draw_name_string(name_captain, buf.as_mut_ptr(), 0, DDSHS_EDIT);
        DrawStatusMessage(game_string(NAMING_STRING_BASE));
        let setting = if name_captain {
            tes.MaxSize = mem::size_of::<[UNICODE; MAX_NAME_SIZE]>() as i32;
            ptr::addr_of_mut!((**world).flagship.commander_name).cast::<UNICODE>()
        } else {
            tes.MaxSize = mem::size_of::<[UNICODE; MAX_NAME_SIZE]>() as i32;
            ptr::addr_of_mut!((**world).flagship.ship_name).cast::<UNICODE>()
        };
        tes.Initialized = 0;
        tes.BaseStr = buf.as_mut_ptr();
        tes.CursorPos = 0;
        tes.CbParam = usize::from(name_captain) as *mut std::ffi::c_void;
        tes.ChangeCallback = Some(on_name_change);
        tes.FrameCallback = None;
        if DoTextEntry(&mut tes) != 0 {
            utf8StringCopy(
                (setting).cast(),
                tes.MaxSize as usize,
                (buf.as_ptr()).cast(),
            );
        } else {
            utf8StringCopy((buf.as_mut_ptr()).cast(), buf.len(), (setting).cast());
        }
        SetFlashRect(SFR_MENU_3DO);
        draw_name_string(name_captain, buf.as_mut_ptr(), 0, DDSHS_NORMAL);
        if let Some(callback) = NAMING_CB {
            callback();
        }
    }

    unsafe fn draw_save_name_string(
        str: *mut UNICODE,
        cursor_pos: COUNT,
        state: COUNT,
        game_index: COUNT,
    ) -> BOOLEAN {
        let mut full_str: [UNICODE; 256] = [0; 256];
        let mut date_str: [UNICODE; 80] = [0; 80];
        DateToString(
            date_str.as_mut_ptr(),
            date_str.len(),
            WorldData::current().read(|owner| owner.game.game_clock.month_index),
            WorldData::current().read(|owner| owner.game.game_clock.day_index),
            WorldData::current().read(|owner| owner.game.game_clock.year_index),
        );
        libc::strncat(
            date_str.as_mut_ptr(),
            c": ".as_ptr(),
            date_str.len() - libc::strlen(date_str.as_ptr()) - 1,
        );
        libc::snprintf(
            full_str.as_mut_ptr(),
            full_str.len(),
            c"%s%s".as_ptr(),
            date_str.as_ptr(),
            str,
        );
        SetContextForeGroundColor(rgb15(0x1b, 0, 0x1b));
        let mut r = RECT {
            corner: POINT {
                x: 8,
                y: (160 + ((game_index % SAVES_PER_PAGE) * 13)) as COORD,
            },
            extent: EXTENT {
                width: 15 + if MAX_SAVED_GAMES > 99 { 5 } else { 0 },
                height: 11,
            },
        };
        DrawRectangle(&mut r);
        r.extent.width = (204 - SAFE_X) as COORD;
        r.corner.x = (30 + SAFE_X) as COORD;
        DrawRectangle(&mut r);
        let mut lf = TEXT {
            baseline: POINT {
                x: r.corner.x + 3,
                y: r.corner.y + 8,
            },
            align: TEXT_ALIGN_ALIGN_LEFT,
            pStr: full_str.as_ptr(),
            CharCount: COUNT::MAX,
        };
        let back_ground = rgb15(0x1b, 0, 0x1b);
        let fore_ground = rgb15(0, 0, 0x14);
        SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
        if state & DDSHS_EDIT == 0 {
            SetContextForeGroundColor(BLACK_COLOR);
            DrawFilledRectangle(&mut r);
            // The C local `t` is unused: it draws `lf`, including the date.
            SetContextForeGroundColor(CAPTAIN_NAME_TEXT_COLOR);
            font_DrawText(&mut lf);
        } else {
            let mut text_r: RECT = mem::zeroed();
            let mut char_deltas = [0u8; 256];
            TextRect(&mut lf, &mut text_r, char_deltas.as_mut_ptr());
            if i32::from(text_r.extent.width) + 2 >= i32::from(r.extent.width) {
                return 0;
            }
            pre_update_flash_rect();
            SetContextForeGroundColor(back_ground);
            DrawFilledRectangle(&mut r);
            let mut pchar_deltas = char_deltas.as_ptr();
            let full_cursor_pos =
                (cursor_pos as usize + libc::strlen(date_str.as_ptr()) - 1) as COUNT;
            for _ in 0..full_cursor_pos {
                text_r.corner.x = text_r.corner.x.wrapping_add(*pchar_deltas as i16);
                pchar_deltas = pchar_deltas.add(1);
            }
            if full_cursor_pos < lf.CharCount {
                text_r.corner.x -= 1;
            }
            if state & DDSHS_BLOCKCUR != 0 {
                if full_cursor_pos == lf.CharCount {
                    text_r.extent.width = 1;
                } else if i32::from(full_cursor_pos) + 1 == i32::from(lf.CharCount) {
                    text_r.extent.width = *pchar_deltas as SIZE + 2;
                } else {
                    text_r.extent.width = *pchar_deltas as SIZE + 1;
                }
            } else {
                text_r.extent.width = 1;
            }
            text_r.corner.y = r.corner.y;
            text_r.extent.height = r.extent.height;
            SetContextForeGroundColor(BLACK_COLOR);
            DrawFilledRectangle(&mut text_r);
            SetContextForeGroundColor(fore_ground);
            font_DrawText(&mut lf);
            post_update_flash_rect();
        }
        1
    }

    unsafe extern "C" fn on_save_name_change(p_tes: *mut TEXTENTRY_STATE) -> BOOLEAN {
        let mut hl = DDSHS_EDIT;
        let game_index = (*p_tes).CbParam.cast::<COUNT>();
        if (*p_tes).JoystickMode != 0 {
            hl |= DDSHS_BLOCKCUR;
        }
        draw_save_name_string(
            (*p_tes).BaseStr,
            (*p_tes).CursorPos as COUNT,
            hl,
            *game_index,
        )
    }

    unsafe fn name_save_game(game_index: COUNT, buf: *mut UNICODE) -> BOOLEAN {
        let mut tes: TEXTENTRY_STATE = mem::zeroed();
        let curs_pos = libc::strlen(buf) as COUNT;
        let g_index = rust_hmalloc(mem::size_of::<COUNT>()).cast::<COUNT>();
        *g_index = game_index;
        draw_save_name_string(buf, curs_pos, DDSHS_EDIT, game_index);
        tes.MaxSize = SAVE_NAME_SIZE as i32;
        tes.Initialized = 0;
        tes.BaseStr = buf;
        tes.CursorPos = curs_pos as i32;
        tes.CbParam = g_index.cast();
        tes.ChangeCallback = Some(on_save_name_change);
        tes.FrameCallback = None;
        let r = RECT {
            extent: EXTENT {
                width: (204 - SAFE_X) as COORD,
                height: 11,
            },
            corner: POINT {
                x: (30 + SAFE_X) as COORD,
                y: (160 + ((game_index % SAVES_PER_PAGE) * 13)) as COORD,
            },
        };
        SetFlashRect(&r);
        if DoTextEntry(&mut tes) == 0 {
            *buf = 0;
        }
        SetFlashRect(ptr::null());
        draw_save_name_string(buf, curs_pos, DDSHS_NORMAL, game_index);
        if let Some(callback) = NAMING_CB {
            callback();
        }
        SetMenuSounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        rust_hfree(g_index.cast());
        SetFlashRect(ptr::null());
        u32::from(tes.Success != 0)
    }

    /// # Safety
    /// Call on the game thread. The callback must remain valid until replaced
    /// and may be invoked after any captain, ship, or save naming operation.
    #[no_mangle]
    pub unsafe extern "C" fn SetNamingCallback(callback: NamingCallback) {
        NAMING_CB = callback;
    }

    unsafe extern "C" fn do_settings(p_ms: *mut MENU_STATE) -> BOOLEAN {
        if WorldData::current().read(|owner| owner.game.current_activity) & CHECK_ABORT as ACTIVITY
            != 0
        {
            return 0;
        }
        let mut cur_speed = (WorldData::current().read(|owner| owner.game.glob_flags)
            & COMBAT_SPEED_MASK as u8)
            >> COMBAT_SPEED_SHIFT;
        if key(KEY_MENU_CANCEL) || (key(KEY_MENU_SELECT) && (*p_ms).CurState == EXIT_SETTINGS_MENU)
        {
            return 0;
        } else if key(KEY_MENU_SELECT) {
            match (*p_ms).CurState {
                SOUND_ON_SETTING | SOUND_OFF_SETTING => {
                    toggle_sound_effect();
                    (*p_ms).CurState ^= 1;
                    DrawMenuStateStrings(PM_SOUND_ON as BYTE, (*p_ms).CurState as SWORD);
                }
                MUSIC_ON_SETTING | MUSIC_OFF_SETTING => {
                    toggle_music();
                    (*p_ms).CurState ^= 1;
                    DrawMenuStateStrings(PM_SOUND_ON as BYTE, (*p_ms).CurState as SWORD);
                }
                CHANGE_CAPTAIN_SETTING | CHANGE_SHIP_SETTING => {
                    name_captain_or_ship((*p_ms).CurState == CHANGE_CAPTAIN_SETTING);
                }
                _ => {
                    let previous_speed = cur_speed;
                    cur_speed += 1;
                    if previous_speed < (NUM_COMBAT_SPEEDS - 1) as BYTE {
                        (**WorldData::current()).game.glob_flags |= CYBORG_ENABLED as BYTE;
                    } else {
                        cur_speed = 0;
                        (**WorldData::current()).game.glob_flags &= !(CYBORG_ENABLED as BYTE);
                    }
                    (**WorldData::current()).game.glob_flags = (WorldData::current()
                        .read(|owner| owner.game.glob_flags)
                        & !(COMBAT_SPEED_MASK as BYTE))
                        | (cur_speed << COMBAT_SPEED_SHIFT);
                    (*p_ms).CurState = CYBORG_OFF_SETTING + cur_speed;
                    DrawMenuStateStrings(PM_SOUND_ON as BYTE, (*p_ms).CurState as SWORD);
                }
            }
            feedback_setting((*p_ms).CurState);
        } else if DoMenuChooser(p_ms, PM_SOUND_ON as BYTE) != 0 {
            feedback_setting((*p_ms).CurState);
        }
        1
    }

    unsafe fn settings_menu() {
        let mut menu_state: MENU_STATE = mem::zeroed();
        menu_state.CurState = SOUND_ON_SETTING;
        DrawMenuStateStrings(PM_SOUND_ON as BYTE, menu_state.CurState as SWORD);
        feedback_setting(menu_state.CurState);
        menu_state.InputFunc = Some(do_settings);
        DoInput(ptr::addr_of_mut!(menu_state).cast(), 0);
        DrawStatusMessage(ptr::null());
    }

    // Private C-local state, containing only header-generated ABI records.
    #[repr(C)]
    struct PickGameState {
        summary: [SUMMARY_DESC; MAX_SAVED_GAMES],
        saving: BOOLEAN,
        success: BOOLEAN,
        summary_frame: FRAME,
    }

    unsafe fn draw_blank_savegame_display(pick_state: *mut PickGameState) {
        let mut s = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: SetAbsFrameIndex(
                (*pick_state).summary_frame,
                GetFrameCount((*pick_state).summary_frame).wrapping_sub(1),
            ),
        };
        DrawStamp(&mut s);
    }

    unsafe fn draw_save_load(pick_state: *mut PickGameState) {
        let mut s = STAMP {
            origin: POINT {
                x: SUMMARY_X_OFFS + 1,
                y: 0,
            },
            frame: SetAbsFrameIndex(
                (*pick_state).summary_frame,
                GetFrameCount((*pick_state).summary_frame).wrapping_sub(2),
            ),
        };
        if (*pick_state).saving != 0 {
            s.frame = DecFrameIndex(s.frame);
        }
        DrawStamp(&mut s);
    }

    unsafe fn draw_savegame_cargo(sis_state: *mut SIS_STATE) {
        const CARGO_COLORS: [Color; 9] = [
            rgb15(0x02, 0x0e, 0x13),
            rgb15(0x19, 0, 0),
            rgb15(0x10, 0x10, 0x10),
            rgb15(0x03, 0x05, 0x1e),
            rgb15(0, 0x18, 0),
            rgb15(0x1b, 0x1b, 0),
            rgb15(0x1e, 0x0d, 0),
            rgb15(0x14, 0, 0x14),
            rgb15(0x0f, 0, 0x19),
        ];
        const ELEMENT_ORG_Y: i16 = 17;
        const ELEMENT_SPACING_Y: i16 = 12;
        const ELEMENT_SPACING_X: i16 = 36;
        let mut buf: [UNICODE; 40] = [0; 40];
        SetContext(Kernel::current().read(|owner| owner.space_context));
        BatchGraphics();
        SetContextFont(Kernel::current().read(|owner| owner.star_con_font));
        let mut s = STAMP {
            frame: SetAbsFrameIndex(
                Kernel::current().read(|owner| owner.misc_data_frame),
                ((NUM_SCANDOT_TRANSITIONS << 1) + 3) as COUNT,
            ),
            origin: POINT {
                x: 7 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS + 3,
                y: ELEMENT_ORG_Y,
            },
        };
        let mut t = TEXT {
            baseline: POINT {
                x: 33 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS + 3,
                y: ELEMENT_ORG_Y + 3,
            },
            align: TEXT_ALIGN_ALIGN_RIGHT,
            pStr: buf.as_ptr(),
            CharCount: COUNT::MAX,
        };
        for (i, color) in CARGO_COLORS
            .iter()
            .enumerate()
            .take(NUM_ELEMENT_CATEGORIES as usize)
        {
            if i == NUM_ELEMENT_CATEGORIES as usize / 2 {
                s.origin.x += ELEMENT_SPACING_X;
                s.origin.y = ELEMENT_ORG_Y;
                t.baseline.x += ELEMENT_SPACING_X;
                t.baseline.y = ELEMENT_ORG_Y + 3;
            }
            DrawStamp(&mut s);
            s.frame = SetRelFrameIndex(s.frame, 5);
            s.origin.y += ELEMENT_SPACING_Y;
            SetContextForeGroundColor(*color);
            libc::snprintf(
                buf.as_mut_ptr(),
                buf.len(),
                c"%u".as_ptr(),
                (*sis_state).element_amounts[i] as u32,
            );
            t.CharCount = COUNT::MAX;
            font_DrawText(&mut t);
            t.baseline.y += ELEMENT_SPACING_Y;
        }
        s.origin.x = 24 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS;
        s.origin.y = 68;
        s.frame = SetAbsFrameIndex(s.frame, 68);
        DrawStamp(&mut s);
        t.baseline.x = 50 + SUMMARY_X_OFFS;
        t.baseline.y = s.origin.y + 3;
        SetContextForeGroundColor(CARGO_COLORS[NUM_ELEMENT_CATEGORIES as usize]);
        libc::snprintf(
            buf.as_mut_ptr(),
            buf.len(),
            c"%u".as_ptr(),
            (*sis_state).total_bio_mass as u32,
        );
        t.CharCount = COUNT::MAX;
        font_DrawText(&mut t);
        UnbatchGraphics();
    }

    unsafe fn draw_savegame_summary(pick_state: *mut PickGameState, game_index: COUNT) {
        let p_sd = ptr::addr_of_mut!((*pick_state).summary)
            .cast::<SUMMARY_DESC>()
            .add(game_index as usize);
        let mut r: RECT = mem::zeroed();
        let mut s: STAMP = mem::zeroed();
        BatchGraphics();
        if (*p_sd).year_index == 0 {
            s.origin.x = 0;
            s.origin.y = 0;
            s.frame = SetAbsFrameIndex(
                (*pick_state).summary_frame,
                GetFrameCount((*pick_state).summary_frame).wrapping_sub(4),
            );
            DrawStamp(&mut s);
        } else {
            let mut old_rect: RECT = mem::zeroed();
            let mut buf: [UNICODE; 256] = [0; 256];
            let save_ss = WorldData::current().read(|world| world.flagship);
            let player_q = WorldData::current().read(|world| world.game.built_ship_q);
            let old_context = SetContext(Kernel::current().read(|owner| owner.status_context));
            GetContextClipRect(&mut old_rect);
            r.corner.x = (SIS_ORG_X as i32
                + ((sis_screen_width() - STATUS_WIDTH as i32) >> 1)
                + SAFE_X as i32
                - 16
                + SUMMARY_X_OFFS as i32) as COORD;
            r.corner.y = SIS_ORG_Y as COORD;
            r.extent.width = STATUS_WIDTH as COORD;
            r.extent.height = (ScreenHeight - (SAFE_Y as i32 * 2)) as COORD;
            SetContextClipRect(&mut r);
            WorldData::current().write(|world| {
                world.flagship = (*p_sd).SS;
                let queue = &raw mut world.game.built_ship_q;
                InitQueue(
                    queue,
                    MAX_BUILT_SHIPS as COUNT,
                    mem::size_of::<SHIP_FRAGMENT>() as OBJ_SIZE,
                );
                for i in 0..(*p_sd).NumShips as usize {
                    CloneShipFragment(
                        &raw mut world.game.avail_race_q,
                        (*p_sd).ShipList[i] as COUNT,
                        queue,
                        0,
                    );
                }
            });
            DateToString(
                buf.as_mut_ptr(),
                buf.len(),
                (*p_sd).month_index,
                (*p_sd).day_index,
                (*p_sd).year_index,
            );
            ClearSISRect(DRAW_SIS_DISPLAY as BYTE);
            DrawStatusMessage(buf.as_ptr());
            WorldData::current().write(|world| UninitQueue(&raw mut world.game.built_ship_q));
            SetContextClipRect(&mut old_rect);
            SetContext(Kernel::current().read(|owner| owner.space_context));
            s.origin.y = 13;
            for i in 0..4 {
                s.origin.x = 140 + SUMMARY_X_OFFS + SUMMARY_SIDE_OFFS;
                for j in 0..4 {
                    let dev_index = i * 4 + j;
                    if dev_index < (*p_sd).NumDevices as usize {
                        s.frame = SetAbsFrameIndex(
                            Kernel::current().read(|owner| owner.misc_data_frame),
                            77 + (*p_sd).DeviceList[dev_index] as COUNT,
                        );
                        DrawStamp(&mut s);
                    }
                    s.origin.x += 18;
                }
                s.origin.y += 18;
            }
            SetContextFont(Kernel::current().read(|owner| owner.star_con_font));
            let mut t = TEXT {
                baseline: POINT {
                    x: 173 + SUMMARY_X_OFFS + SUMMARY_SIDE_OFFS,
                    y: 0,
                },
                align: TEXT_ALIGN_ALIGN_CENTER,
                CharCount: COUNT::MAX,
                pStr: buf.as_ptr(),
            };
            if (*p_sd).Flags & AFTER_BOMB_INSTALLED as BYTE != 0 {
                s.origin.x = SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS + 6;
                s.origin.y = 0;
                s.frame = SetRelFrameIndex((*pick_state).summary_frame, 0);
                DrawStamp(&mut s);
                s.origin.x = SUMMARY_X_OFFS + SUMMARY_SIDE_OFFS;
                s.frame = IncFrameIndex(s.frame);
                DrawStamp(&mut s);
            } else {
                draw_savegame_cargo(ptr::addr_of_mut!((*p_sd).SS));
                SetContext(Kernel::current().read(|owner| owner.radar_context));
                GetContextClipRect(&mut old_rect);
                r.corner.x = SIS_ORG_X as COORD + 10 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS;
                r.corner.y = SIS_ORG_Y as COORD + 84;
                r.extent = old_rect.extent;
                SetContextClipRect(&mut r);
                InitLander((*p_sd).Flags | OVERRIDE_LANDER_FLAGS as BYTE);
                SetContextClipRect(&mut old_rect);
                SetContext(Kernel::current().read(|owner| owner.space_context));
                libc::snprintf(
                    buf.as_mut_ptr(),
                    buf.len(),
                    c"%u".as_ptr(),
                    (*p_sd).SS.res_units,
                );
                t.baseline.y = 102;
                SetContextForeGroundColor(rgb15(0x10, 0, 0x10));
                font_DrawText(&mut t);
                t.CharCount = COUNT::MAX;
            }
            t.baseline.y = 126;
            libc::snprintf(
                buf.as_mut_ptr(),
                buf.len(),
                c"%u".as_ptr(),
                ((*p_sd).MCreditLo as u32) | ((*p_sd).MCreditHi as u32) << 8,
            );
            SetContextForeGroundColor(rgb15(0x10, 0, 0x10));
            font_DrawText(&mut t);
            t.baseline.x = 6;
            t.baseline.y = 139 + 6;
            t.align = TEXT_ALIGN_ALIGN_LEFT;
            t.pStr = buf.as_ptr();
            let mut star_pt = POINT {
                x: logx_to_universe((*p_sd).SS.log_x),
                y: logy_to_universe((*p_sd).SS.log_y),
            };
            match (*p_sd).Activity as u32 {
                IN_LAST_BATTLE | IN_INTERPLANETARY | IN_PLANET_ORBIT | IN_STARBASE => {
                    let quasi_state = World::current()
                        .read(|world| world.story_state(StoryKey::ARILOU_SPACE_SIDE));
                    World::current()
                        .write(|world| world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 0));
                    let sdptr = FindStar(ptr::null_mut(), &mut star_pt, 1, 1);
                    World::current().write(|world| {
                        world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, quasi_state)
                    });
                    if !sdptr.is_null() {
                        GetClusterName(sdptr, buf.as_mut_ptr());
                        star_pt = (*sdptr).star_pt;
                    } else {
                        buf[0] = 0;
                    }
                }
                IN_HYPERSPACE => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        (game_string(NAVIGATION_STRING_BASE)).cast(),
                    );
                }
                IN_QUASISPACE => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        (game_string(NAVIGATION_STRING_BASE + 1)).cast(),
                    );
                }
                _ => {
                    buf[0] = 0;
                }
            }
            SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
            SetContextForeGroundColor(rgb15(0x1b, 0, 0x1b));
            t.CharCount = COUNT::MAX;
            font_DrawText(&mut t);
            t.align = TEXT_ALIGN_ALIGN_CENTER;
            t.baseline.x = (sis_screen_width() - SIS_TITLE_BOX_WIDTH as i32 - 4
                + (SIS_TITLE_WIDTH >> 1) as i32) as COORD;
            match (*p_sd).Activity as u32 {
                IN_STARBASE => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        (game_string(STARBASE_STRING_BASE)).cast(),
                    );
                }
                IN_LAST_BATTLE => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        (game_string(PLANET_NUMBER_BASE + 32)).cast(),
                    );
                }
                IN_PLANET_ORBIT => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        ptr::addr_of!((*p_sd).SS.planet_name).cast(),
                    );
                }
                _ => {
                    libc::snprintf(
                        buf.as_mut_ptr(),
                        buf.len(),
                        c"%03u.%01u : %03u.%01u".as_ptr(),
                        (star_pt.x as i32 / 10) as u32,
                        (star_pt.x as i32 % 10) as u32,
                        (star_pt.y as i32 / 10) as u32,
                        (star_pt.y as i32 % 10) as u32,
                    );
                }
            }
            t.CharCount = COUNT::MAX;
            font_DrawText(&mut t);
            SetContext(old_context);
            WorldData::current().write(|world| {
                world.game.built_ship_q = player_q;
                world.flagship = save_ss;
            });
        }
        UnbatchGraphics();
    }

    unsafe fn draw_game_selection(pick_state: *mut PickGameState, sel_slot: COUNT) {
        let mut buf: [UNICODE; 256] = [0; 256];
        let mut buf2: [UNICODE; 80] = [0; 80];
        BatchGraphics();
        SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
        let mut r = RECT {
            extent: EXTENT {
                width: 240,
                height: 65,
            },
            corner: POINT { x: 1, y: 160 },
        };
        SetContextForeGroundColor(BLACK_COLOR);
        DrawFilledRectangle(&mut r);
        let mut t = TEXT {
            CharCount: COUNT::MAX,
            pStr: buf.as_ptr(),
            align: TEXT_ALIGN_ALIGN_LEFT,
            baseline: POINT { x: 0, y: 0 },
        };
        let mut cur_slot = sel_slot - (sel_slot % SAVES_PER_PAGE);
        let mut i = 0;
        while i < SAVES_PER_PAGE && cur_slot < MAX_SAVED_GAMES as COUNT {
            let desc = ptr::addr_of_mut!((*pick_state).summary)
                .cast::<SUMMARY_DESC>()
                .add(cur_slot as usize);
            SetContextForeGroundColor(if cur_slot == sel_slot {
                rgb15(0x1b, 0, 0x1b)
            } else {
                rgb15(0, 0, 0x14)
            });
            r.extent.width = 15;
            if MAX_SAVED_GAMES > 99 {
                r.extent.width += 5;
            }
            r.extent.height = 11;
            r.corner.x = 8;
            r.corner.y = (160 + i * 13) as COORD;
            DrawRectangle(&mut r);
            t.baseline.x = r.corner.x + 3;
            t.baseline.y = r.corner.y + 8;
            libc::snprintf(
                buf.as_mut_ptr(),
                buf.len(),
                if MAX_SAVED_GAMES > 99 {
                    c"%03u".as_ptr()
                } else {
                    c"%02u".as_ptr()
                },
                cur_slot as u32,
            );
            font_DrawText(&mut t);
            r.extent.width = (204 - SAFE_X) as COORD;
            r.corner.x = (30 + SAFE_X) as COORD;
            DrawRectangle(&mut r);
            t.baseline.x = r.corner.x + 3;
            if (*desc).year_index == 0 {
                utf8StringCopy(
                    (buf.as_mut_ptr()).cast(),
                    buf.len(),
                    (game_string(SAVEGAME_STRING_BASE + 3)).cast(),
                );
            } else {
                DateToString(
                    buf2.as_mut_ptr(),
                    buf2.len(),
                    (*desc).month_index,
                    (*desc).day_index,
                    (*desc).year_index,
                );
                let name = if (*desc).SaveName[0] != 0 {
                    ptr::addr_of!((*desc).SaveName).cast::<UNICODE>()
                } else {
                    game_string(SAVEGAME_STRING_BASE + 4)
                };
                libc::snprintf(
                    buf.as_mut_ptr(),
                    buf.len(),
                    c"%s: %s".as_ptr(),
                    buf2.as_ptr(),
                    name,
                );
            }
            font_DrawText(&mut t);
            i += 1;
            cur_slot += 1;
        }
        UnbatchGraphics();
    }

    unsafe fn redraw_pick_display(pick_state: *mut PickGameState, sel_slot: COUNT) {
        BatchGraphics();
        draw_blank_savegame_display(pick_state);
        draw_savegame_summary(pick_state, sel_slot);
        draw_game_selection(pick_state, sel_slot);
        UnbatchGraphics();
    }

    /// Enumerate the original 50-slot picker list synchronously.
    ///
    /// # Safety
    /// Game-thread caller supplies 50 writable summaries and owns the save directory.
    pub unsafe fn load_game_descriptions(mut p_sd: *mut SUMMARY_DESC) {
        for i in 0..MAX_SAVED_GAMES as COUNT {
            if LoadGame(i, (p_sd).cast()) == 0 {
                (*p_sd).year_index = 0;
            }
            log::info!(
                "RUST_PICKER: enumerated slot={i} state={}",
                if (*p_sd).year_index == 0 {
                    "unavailable"
                } else {
                    "occupied"
                },
            );
            p_sd = p_sd.add(1);
        }
    }

    unsafe extern "C" fn do_pick_game(p_ms: *mut MENU_STATE) -> BOOLEAN {
        let pick_state = (*p_ms).privData.cast::<PickGameState>();
        let time_in = GetTimeCounter();
        let occupied = (*pick_state).summary[(*p_ms).CurState as usize].year_index != 0;
        let activity = WorldData::current().read(|owner| owner.game.current_activity);
        let action = picker_action(
            activity & CHECK_ABORT as ACTIVITY != 0,
            key(KEY_MENU_CANCEL),
            key(KEY_MENU_SELECT),
            (*pick_state).saving != 0,
            occupied,
        );
        if action != PickerAction::Continue {
            log::info!(
                "RUST_PICKER: action={action:?} slot={} select={} cancel={} activity={:#x}",
                (*p_ms).CurState,
                key(KEY_MENU_SELECT),
                key(KEY_MENU_CANCEL),
                activity,
            );
        }
        match action {
            PickerAction::Abort => return 0,
            PickerAction::Cancel => {
                (*pick_state).success = 0;
                return 0;
            }
            PickerAction::Select => {
                PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_SUCCESS);
                (*pick_state).success = 1;
                return 0;
            }
            PickerAction::EmptySlot => PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE),
            PickerAction::Continue => {
                let new_state = picker_slot(
                    (*p_ms).CurState,
                    key(KEY_MENU_LEFT) || key(KEY_MENU_PAGE_UP),
                    key(KEY_MENU_RIGHT) || key(KEY_MENU_PAGE_DOWN),
                    key(KEY_MENU_UP),
                    key(KEY_MENU_DOWN),
                );
                if new_state != (*p_ms).CurState {
                    (*p_ms).CurState = new_state;
                    SetContext(Kernel::current().read(|owner| owner.space_context));
                    redraw_pick_display(pick_state, (*p_ms).CurState as COUNT);
                }
                SleepThreadUntil(time_in.wrapping_add(ONE_SECOND / 30));
            }
        }
        1
    }

    unsafe fn save_load_game(
        pick_state: *mut PickGameState,
        game_index: COUNT,
        canceled_by_user: *mut BOOLEAN,
    ) -> BOOLEAN {
        let desc = ptr::addr_of_mut!((*pick_state).summary)
            .cast::<SUMMARY_DESC>()
            .add(game_index as usize);
        let mut name_buf: [UNICODE; 256] = [0; 256];
        let mut save_stamp: STAMP = mem::zeroed();
        save_stamp.frame = ptr::null_mut();
        let success;
        if (*pick_state).saving != 0 {
            libc::strncpy(
                name_buf.as_mut_ptr(),
                ptr::addr_of!((*desc).SaveName).cast(),
                SAVE_NAME_SIZE as usize,
            );
            name_buf[SAVE_NAME_SIZE as usize] = 0;
            if name_save_game(game_index, name_buf.as_mut_ptr()) != 0 {
                PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_SUCCESS);
                ConfirmSaveLoad(if (*pick_state).saving != 0 {
                    &mut save_stamp
                } else {
                    ptr::null_mut()
                });
                success = SaveGame(game_index, (desc).cast(), name_buf.as_ptr());
            } else {
                success = 0;
                *canceled_by_user = 1;
            }
        } else {
            ConfirmSaveLoad(if (*pick_state).saving != 0 {
                &mut save_stamp
            } else {
                ptr::null_mut()
            });
            success = LoadGame(game_index, ptr::null_mut());
        }
        if (*pick_state).saving != 0 {
            DrawStamp(&mut save_stamp);
        }
        DestroyDrawable(ReleaseDrawable(save_stamp.frame));
        success
    }

    unsafe fn pick_game(saving: BOOLEAN, from_main_menu: BOOLEAN) -> BOOLEAN {
        let activity = WorldData::current().read(|owner| owner.game.current_activity);
        log::info!(
            "RUST_PICKER: enter saving={saving} main_menu={from_main_menu} activity={:#x}",
            activity,
        );
        let mut pick_state: PickGameState = mem::zeroed();
        pick_state.saving = saving;
        pick_state.summary_frame =
            SetAbsFrameIndex(Kernel::current().read(|owner| owner.play_frame), 39);
        let mut menu_state: MENU_STATE = mem::zeroed();
        menu_state.privData = ptr::addr_of_mut!(pick_state).cast();
        menu_state.CurState = LAST_USED_SLOT as BYTE;
        let time_out = FadeMusic(0, (ONE_SECOND / 2) as SIZE);
        let old_callback = SetInputCallback(None);
        load_game_descriptions(ptr::addr_of_mut!(pick_state.summary).cast());
        let old_context = SetContext(Kernel::current().read(|owner| owner.space_context));
        let mut dlg_stamp = SaveContextFrame(ptr::null());
        let mut dlg_rect: RECT = mem::zeroed();
        GetContextClipRect(&mut dlg_rect);
        SleepThreadUntil(time_out);
        pause_music();
        StopSound();
        FadeMusic(NORMAL_VOLUME as BYTE, 0);
        SetTransitionSource(ptr::null());
        BatchGraphics();
        SetContextBackGroundColor(BLACK_COLOR);
        ClearDrawable();
        redraw_pick_display(&mut pick_state, menu_state.CurState as COUNT);
        draw_save_load(&mut pick_state);
        if from_main_menu != 0 {
            UnbatchGraphics();
            FadeScreen(ScreenFadeType_FadeAllToColor, (ONE_SECOND / 2) as SIZE);
        } else {
            let mut ctx_rect: RECT = mem::zeroed();
            GetContextClipRect(&mut ctx_rect);
            ScreenTransition(3, &ctx_rect);
            UnbatchGraphics();
        }
        SetMenuSounds(
            MENU_SOUND_ARROWS | MENU_SOUND_PAGEUP | MENU_SOUND_PAGEDOWN,
            0,
        );
        log::info!("RUST_PICKER: list_drawn slots=50");
        menu_state.InputFunc = Some(do_pick_game);
        loop {
            let mut canceled_by_user: BOOLEAN = 0;
            pick_state.success = 0;
            DoInput(ptr::addr_of_mut!(menu_state).cast(), 1);
            if pick_state.success == 0 {
                break;
            }
            LAST_USED_SLOT = menu_state.CurState as COUNT;
            if save_load_game(
                &mut pick_state,
                menu_state.CurState as COUNT,
                &mut canceled_by_user,
            ) != 0
            {
                log::info!(
                    "{}",
                    if saving != 0 {
                        "RUST_PICKER: exit=save_success"
                    } else {
                        "RUST_PICKER: exit=load_success"
                    }
                );
                break;
            }
            log::info!("RUST_PICKER: operation_failed picker_remains_open");
            if saving != 0 && canceled_by_user == 0 {
                SaveProblem();
            }
            load_game_descriptions(ptr::addr_of_mut!(pick_state.summary).cast());
            redraw_pick_display(&mut pick_state, menu_state.CurState as COUNT);
        }
        SetMenuSounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        if pick_state.success != 0 && saving == 0 {
            (**WorldData::current()).game.current_activity |= CHECK_LOAD as ACTIVITY;
        }
        if WorldData::current().read(|owner| owner.game.current_activity) & CHECK_ABORT as ACTIVITY
            == 0
            && (saving != 0 || (pick_state.success == 0 && from_main_menu == 0))
        {
            SetTransitionSource(&dlg_rect);
            BatchGraphics();
            DrawStamp(&mut dlg_stamp);
            ScreenTransition(3, &dlg_rect);
            UnbatchGraphics();
        }
        DestroyDrawable(ReleaseDrawable(dlg_stamp.frame));
        SetContext(old_context);
        resume_music();
        SetInputCallback(old_callback);
        pick_state.success
    }

    unsafe extern "C" fn do_game_options(p_ms: *mut MENU_STATE) -> BOOLEAN {
        if WorldData::current().read(|owner| owner.game.current_activity) & CHECK_ABORT as ACTIVITY
            != 0
        {
            return 0;
        }
        if key(KEY_MENU_CANCEL) || (key(KEY_MENU_SELECT) && (*p_ms).CurState == EXIT_GAME_MENU) {
            return 0;
        } else if key(KEY_MENU_SELECT) {
            match (*p_ms).CurState {
                SAVE_GAME | LOAD_GAME => {
                    SetFlashRect(ptr::null());
                    if pick_game(u32::from((*p_ms).CurState == SAVE_GAME), 0) != 0 {
                        return 0;
                    }
                    SetFlashRect(SFR_MENU_3DO);
                }
                QUIT_GAME => {
                    if ConfirmExit() != 0 {
                        return 0;
                    }
                }
                SETTINGS => {
                    settings_menu();
                    DrawMenuStateStrings(PM_SAVE_GAME as BYTE, (*p_ms).CurState as SWORD);
                }
                _ => {}
            }
        } else {
            DoMenuChooser(p_ms, PM_SAVE_GAME as BYTE);
        }
        1
    }

    /// # Safety
    /// The game thread must have initialized the game, input, audio, and SIS
    /// contexts. Runs the modal menu and may replace the live game on load.
    #[no_mangle]
    pub unsafe extern "C" fn GameOptions() -> BOOLEAN {
        let mut menu_state: MENU_STATE = mem::zeroed();
        if Kernel::current().read(|owner| owner.last_activity) == CHECK_LOAD as ACTIVITY {
            DrawMenuStateStrings(PM_SAVE_GAME as BYTE, LOAD_GAME as SWORD);
            let success = pick_game(0, 1);
            if success == 0 {
                (**WorldData::current()).game.current_activity |= CHECK_ABORT as ACTIVITY;
            }
            return 0;
        }
        menu_state.CurState = SAVE_GAME;
        DrawMenuStateStrings(PM_SAVE_GAME as BYTE, menu_state.CurState as SWORD);
        SetFlashRect(SFR_MENU_3DO);
        SetMenuSounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        menu_state.InputFunc = Some(do_game_options);
        crate::automation::ui_observation::rust_automation_observe_game_options(1);
        DoInput(ptr::addr_of_mut!(menu_state).cast(), 1);
        crate::automation::ui_observation::rust_automation_observe_game_options(0);
        SetFlashRect(ptr::null());
        u32::from(
            WorldData::current().read(|owner| owner.game.current_activity)
                & (CHECK_ABORT | CHECK_LOAD) as ACTIVITY
                == 0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_pages_clamp_and_wrap_at_the_same_slots_as_c() {
        let left = |slot| picker_slot(slot, true, false, false, false);
        let right = |slot| picker_slot(slot, false, true, false, false);
        assert_eq!(left(0), 49);
        for slot in 1..=5 {
            assert_eq!(left(slot), 0);
        }
        assert_eq!(left(6), 1);
        assert_eq!(left(49), 44);
        assert_eq!(right(0), 5);
        assert_eq!(right(43), 48);
        for slot in 44..=48 {
            assert_eq!(right(slot), 49);
        }
        assert_eq!(right(49), 0);
    }

    #[test]
    fn picker_arrows_wrap_and_idle_keeps_selection() {
        for slot in 0..MAX_SAVED_GAMES as u8 {
            assert_eq!(
                picker_slot(slot, false, false, true, false),
                if slot == 0 { 49 } else { slot - 1 }
            );
            assert_eq!(
                picker_slot(slot, false, false, false, true),
                if slot == 49 { 0 } else { slot + 1 }
            );
            assert_eq!(picker_slot(slot, false, false, false, false), slot);
        }
    }

    #[test]
    fn simultaneous_picker_keys_keep_c_branch_priority() {
        assert_eq!(picker_slot(12, true, true, true, true), 7);
        assert_eq!(picker_slot(12, false, true, true, true), 17);
        assert_eq!(picker_slot(12, false, false, true, true), 11);
    }
}
