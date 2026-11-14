//! Game options, naming, and save/load selection, ported from `uqm/gameopt.c`.

#[cfg(test)]
mod picker_tests;

mod layout;
mod naming;
mod preview;

mod picker;
#[cfg(not(test))]
use picker::SAVES_PER_PAGE;
pub use picker::{picker_action, PickerAction};
use picker::{picker_slot, MAX_SAVED_GAMES};

#[cfg(not(test))]
pub use crate::savegame::live::load::load_game_descriptions;
#[cfg(not(test))]
pub use runtime::game_options;

#[cfg(not(test))]
pub(crate) mod runtime {
    use super::layout::{BLACK_COLOR, CAPTAIN_NAME_RECT, SHIP_NAME_RECT};
    use super::naming::{draw_name_string, draw_save_name_string};

    mod save_load;

    use save_load::pick_game;

    use super::{picker_action, picker_slot, PickerAction, MAX_SAVED_GAMES, SAVES_PER_PAGE};
    use crate::engine::world::{generation::SolarSnapshot, MUSIC_DISABLED};
    use crate::engine::{battle::Cx, input::InputSession, observe::Io, Kernel};
    use crate::graphics::context_abi::ContextLease;
    use crate::navigation::abi::{
        MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as MENU_FAILURE,
        MENU_SOUND_EFFECT_MENU_SOUND_SUCCESS as MENU_SUCCESS, *,
    };
    use crate::planets::starmap::naming::name_near;
    use crate::ui::settings::play_menu_sound;
    use crate::ui::sis::{clear_sis_rect, draw_status_message, set_flash_rect, set_menu_flash};

    use std::{mem, ptr};

    const SUMMARY_X_OFFS: i16 = 14;
    const SUMMARY_SIDE_OFFS: i16 = 7;

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

    static mut LAST_USED_SLOT: COUNT = 0;

    use crate::ui::rgb15;

    unsafe fn game_string(index: u32) -> *mut UNICODE {
        GetStringAddress(SetAbsStringTableIndex(
            Kernel::current().read(|owner| owner.game_strings),
            index as COUNT,
        ))
    }

    fn key(input: &InputSession, index: u32) -> bool {
        input.pulsed().menu[index as usize] != 0
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

    struct NameCallbacks {
        captain: bool,
    }
    impl crate::ui::getchar::TextEntryCallbacks for NameCallbacks {
        fn changed(
            &mut self,
            entry: crate::ui::getchar::TextEntryView<'_>,
            cx: &mut Cx<'_>,
            _input: &InputSession,
        ) -> bool {
            draw_name_string(
                cx.kernel,
                cx.world,
                self.captain,
                entry.text,
                entry.cursor,
                DDSHS_EDIT | if entry.joystick { DDSHS_BLOCKCUR } else { 0 },
            )
        }
        fn frame(
            &mut self,
            _entry: crate::ui::getchar::TextEntryView<'_>,
            cx: &mut Cx<'_>,
            _io: &mut Io<'_>,
            _input: &InputSession,
        ) -> bool {
            cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
            true
        }
    }

    fn name_captain_or_ship(cx: &mut Cx<'_>, io: &mut Io<'_>, input: &InputSession, captain: bool) {
        let status = ContextLease::require(cx.kernel.status_context);
        let screen = ContextLease::require(cx.kernel.screen_context);
        status.select();
        set_flash_rect(
            &cx.kernel.sis_highlight,
            &screen,
            status.clip_rect(),
            Some(if captain {
                CAPTAIN_NAME_RECT
            } else {
                SHIP_NAME_RECT
            }),
        );
        let mut buffer = [0u8; SIS_NAME_SIZE as usize];
        draw_name_string(cx.kernel, cx.world, captain, c"", 0, DDSHS_EDIT);
        draw_status_message(
            cx.kernel,
            cx.world,
            Some(cx.kernel.game_text(NAMING_STRING_BASE as usize)),
        );
        let accepted = crate::ui::getchar::edit_text(
            cx,
            io,
            input,
            buffer.len(),
            &mut buffer,
            0,
            &mut NameCallbacks { captain },
        );
        let setting = if captain {
            &mut cx.world.data.flagship.commander_name
        } else {
            &mut cx.world.data.flagship.ship_name
        };
        if accepted {
            let text = std::ffi::CStr::from_bytes_until_nul(&buffer)
                .unwrap_or_else(|error| panic!("invalid entered name: {error}"));
            setting.fill(0);
            let count = text.to_bytes().len().min(setting.len() - 1);
            setting[..count].copy_from_slice(&text.to_bytes()[..count]);
        } else {
            buffer.copy_from_slice(setting);
        }
        set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
        let text = std::ffi::CStr::from_bytes_until_nul(&buffer)
            .unwrap_or_else(|error| panic!("invalid saved name: {error}"));
        draw_name_string(cx.kernel, cx.world, captain, text, 0, DDSHS_NORMAL);
        if let Some(callback) = cx.kernel.naming_callback {
            callback(cx.kernel, cx.world, cx.screen_dimensions().0);
        }
    }

    struct SaveNameCallbacks {
        index: u16,
    }
    impl crate::ui::getchar::TextEntryCallbacks for SaveNameCallbacks {
        fn changed(
            &mut self,
            entry: crate::ui::getchar::TextEntryView<'_>,
            cx: &mut Cx<'_>,
            _input: &InputSession,
        ) -> bool {
            draw_save_name_string(
                cx.kernel,
                cx.world,
                entry.text,
                entry.cursor,
                DDSHS_EDIT | if entry.joystick { DDSHS_BLOCKCUR } else { 0 },
                self.index,
            )
        }
        fn frame(
            &mut self,
            _entry: crate::ui::getchar::TextEntryView<'_>,
            cx: &mut Cx<'_>,
            _io: &mut Io<'_>,
            _input: &InputSession,
        ) -> bool {
            cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
            true
        }
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

    unsafe fn draw_savegame_summary(
        cx: &mut Cx<'_>,
        pick_state: *mut PickGameState,
        game_index: COUNT,
    ) {
        let dimensions = cx.screen_dimensions();
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
            let saved = super::preview::SavedState::capture(cx.world);
            let old_context = SetContext(cx.kernel.status_context);
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
            super::preview::install(cx.world, &*p_sd);
            let date = save_load::summary_date(cx.kernel, &*p_sd, 255);
            clear_sis_rect(cx.kernel, cx.world, dimensions, DRAW_SIS_DISPLAY as BYTE);
            draw_status_message(cx.kernel, cx.world, Some(&date));
            cx.world.built_ships().reinit();
            SetContextClipRect(&mut old_rect);
            SetContext(cx.kernel.space_context);
            s.origin.y = 13;
            for i in 0..4 {
                s.origin.x = 140 + SUMMARY_X_OFFS + SUMMARY_SIDE_OFFS;
                for j in 0..4 {
                    let dev_index = i * 4 + j;
                    if dev_index < (*p_sd).NumDevices as usize {
                        s.frame = SetAbsFrameIndex(
                            cx.kernel.misc_data_frame,
                            77 + (*p_sd).DeviceList[dev_index] as COUNT,
                        );
                        DrawStamp(&mut s);
                    }
                    s.origin.x += 18;
                }
                s.origin.y += 18;
            }
            SetContextFont(cx.kernel.star_con_font);
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
                cx.with_legacy(|| draw_savegame_cargo(ptr::addr_of_mut!((*p_sd).SS)));
                SetContext(cx.kernel.radar_context);
                GetContextClipRect(&mut old_rect);
                r.corner.x = SIS_ORG_X as COORD + 10 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS;
                r.corner.y = SIS_ORG_Y as COORD + 84;
                r.extent = old_rect.extent;
                SetContextClipRect(&mut r);
                super::preview::lander(cx.kernel, &*p_sd);
                SetContextClipRect(&mut old_rect);
                SetContext(cx.kernel.space_context);
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
                    let named = name_near(cx.kernel, cx.world.catalog(), star_pt);
                    if let Some((name, point)) = named {
                        utf8StringCopy(buf.as_mut_ptr().cast(), buf.len(), name.as_ptr().cast());
                        star_pt = point;
                    } else {
                        buf[0] = 0;
                    }
                }
                IN_HYPERSPACE => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        (cx.with_legacy(|| game_string(NAVIGATION_STRING_BASE))).cast(),
                    );
                }
                IN_QUASISPACE => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        (cx.with_legacy(|| game_string(NAVIGATION_STRING_BASE + 1))).cast(),
                    );
                }
                _ => {
                    buf[0] = 0;
                }
            }
            SetContextFont(cx.kernel.tiny_font);
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
                        (cx.with_legacy(|| game_string(STARBASE_STRING_BASE))).cast(),
                    );
                }
                IN_LAST_BATTLE => {
                    utf8StringCopy(
                        (buf.as_mut_ptr()).cast(),
                        buf.len(),
                        (cx.with_legacy(|| game_string(PLANET_NUMBER_BASE + 32))).cast(),
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
            saved.restore(cx.world);
        }
        UnbatchGraphics();
    }

    unsafe fn draw_game_selection(
        cx: &mut Cx<'_>,
        pick_state: *mut PickGameState,
        sel_slot: COUNT,
    ) {
        let mut buf: [UNICODE; 256] = [0; 256];
        BatchGraphics();
        SetContextFont(cx.kernel.tiny_font);
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
                    (cx.with_legacy(|| game_string(SAVEGAME_STRING_BASE + 3))).cast(),
                );
            } else {
                let date = save_load::summary_date(cx.kernel, &*desc, 79);
                let name = if (*desc).SaveName[0] != 0 {
                    ptr::addr_of!((*desc).SaveName).cast::<UNICODE>()
                } else {
                    cx.with_legacy(|| game_string(SAVEGAME_STRING_BASE + 4))
                };
                libc::snprintf(
                    buf.as_mut_ptr(),
                    buf.len(),
                    c"%s: %s".as_ptr(),
                    date.as_ptr(),
                    name,
                );
            }
            font_DrawText(&mut t);
            i += 1;
            cur_slot += 1;
        }
        UnbatchGraphics();
    }

    unsafe fn redraw_pick_display(
        cx: &mut Cx<'_>,
        pick_state: *mut PickGameState,
        sel_slot: COUNT,
    ) {
        BatchGraphics();
        cx.with_legacy(|| draw_blank_savegame_display(pick_state));
        draw_savegame_summary(cx, pick_state, sel_slot);
        draw_game_selection(cx, pick_state, sel_slot);
        UnbatchGraphics();
    }

    unsafe fn do_pick_game(
        cx: &mut Cx<'_>,
        input: &InputSession,
        p_ms: *mut MENU_STATE,
    ) -> BOOLEAN {
        let activity = cx.world.activity();
        let pick_state = (*p_ms).privData.cast::<PickGameState>();
        let time_in = GetTimeCounter();
        let occupied = (*pick_state).summary[(*p_ms).CurState as usize].year_index != 0;
        let action = picker_action(
            activity & CHECK_ABORT as ACTIVITY != 0,
            key(input, KEY_MENU_CANCEL),
            key(input, KEY_MENU_SELECT),
            (*pick_state).saving != 0,
            occupied,
        );
        if action != PickerAction::Continue {
            log::info!(
                "RUST_PICKER: action={action:?} slot={} select={} cancel={} activity={:#x}",
                (*p_ms).CurState,
                key(input, KEY_MENU_SELECT),
                key(input, KEY_MENU_CANCEL),
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
                play_menu_sound(cx.kernel, cx.world, MENU_SUCCESS as u16);
                (*pick_state).success = 1;
                return 0;
            }
            PickerAction::EmptySlot => play_menu_sound(cx.kernel, cx.world, MENU_FAILURE as u16),
            PickerAction::Continue => {
                let new_state = picker_slot(
                    (*p_ms).CurState,
                    key(input, KEY_MENU_LEFT) || key(input, KEY_MENU_PAGE_UP),
                    key(input, KEY_MENU_RIGHT) || key(input, KEY_MENU_PAGE_DOWN),
                    key(input, KEY_MENU_UP),
                    key(input, KEY_MENU_DOWN),
                );
                if new_state != (*p_ms).CurState {
                    (*p_ms).CurState = new_state;
                    SetContext(cx.kernel.space_context);
                    redraw_pick_display(cx, pick_state, (*p_ms).CurState as COUNT);
                }
                cx.with_legacy(|| SleepThreadUntil(time_in.wrapping_add(ONE_SECOND / 30)));
            }
        }
        1
    }

    fn do_game_options(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        solar: Option<SolarSnapshot>,
        state: &mut u8,
    ) -> bool {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        if cx.world.activity() & CHECK_ABORT as ACTIVITY != 0
            || key(input, KEY_MENU_CANCEL)
            || (key(input, KEY_MENU_SELECT) && *state == EXIT_GAME_MENU)
        {
            return false;
        }
        if key(input, KEY_MENU_SELECT) {
            match *state {
                SAVE_GAME | LOAD_GAME => {
                    cx.kernel.sis_highlight.context.borrow_mut().take();
                    if pick_game(cx, io, solar, u32::from(*state == SAVE_GAME), 0) != 0 {
                        return false;
                    }
                    set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
                }
                QUIT_GAME => {
                    if runtime.confirm_exit(cx) {
                        return false;
                    }
                }
                SETTINGS => {
                    settings_menu(cx, io);
                    crate::ui::menu::draw_menu_state(
                        cx.kernel,
                        cx.world,
                        cx.screen_dimensions(),
                        PM_SAVE_GAME as u8,
                        i16::from(*state),
                    );
                }
                _ => {}
            }
        } else {
            crate::ui::menu::choose_menu_state(cx, input, state, PM_SAVE_GAME as u8);
        }
        true
    }

    pub fn game_options(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        solar: Option<&crate::planets::solar_system::SolarSystem>,
    ) -> BOOLEAN {
        let solar = solar.map(|visit| visit.snapshot(cx.screen_dimensions()));
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        if cx.kernel.last_activity == CHECK_LOAD as ACTIVITY {
            crate::ui::menu::draw_menu_state(
                cx.kernel,
                cx.world,
                cx.screen_dimensions(),
                PM_SAVE_GAME as u8,
                LOAD_GAME as i16,
            );
            if pick_game(cx, io, solar, 0, 1) == 0 {
                cx.world
                    .set_activity(cx.world.activity() | CHECK_ABORT as ACTIVITY);
            }
            return 0;
        }
        let mut state = SAVE_GAME;
        crate::ui::menu::draw_menu_state(
            cx.kernel,
            cx.world,
            cx.screen_dimensions(),
            PM_SAVE_GAME as u8,
            i16::from(state),
        );
        set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
        SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        cx.with_legacy(|| {
            io.observer
                .checkpoint(&crate::engine::observe::CheckpointView::GameOptions(true))
        });
        cx.run_input(io, true, |cx, io| {
            do_game_options(cx, io, solar, &mut state)
        });
        cx.with_legacy(|| {
            io.observer
                .checkpoint(&crate::engine::observe::CheckpointView::GameOptions(false))
        });
        cx.kernel.sis_highlight.context.borrow_mut().take();
        u32::from(cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) as ACTIVITY == 0)
    }

    fn feedback_setting(cx: &Cx<'_>, input: &InputSession, setting: u8) {
        let text = setting_feedback(
            cx.kernel,
            cx.world.data.game.glob_flags,
            input.preferences().menu,
            setting,
        );
        draw_status_message(cx.kernel, cx.world, Some(&text));
    }

    fn setting_feedback(kernel: &Kernel, flags: u8, menu: i32, setting: u8) -> std::ffi::CString {
        let mut text = Vec::new();
        let option = match setting {
            SOUND_ON_SETTING | SOUND_OFF_SETTING => Some((0, SOUND_DISABLED as u8)),
            MUSIC_ON_SETTING | MUSIC_OFF_SETTING => Some((1, MUSIC_DISABLED)),
            CYBORG_OFF_SETTING..=CYBORG_SUPER_SETTING => Some((2, CYBORG_ENABLED as u8)),
            _ => None,
        };
        if let Some((offset, flag)) = option {
            text.extend_from_slice(
                kernel
                    .game_text((OPTION_STRING_BASE + offset) as usize)
                    .to_bytes(),
            );
            text.push(b' ');
            let enabled = if flag == CYBORG_ENABLED as u8 {
                flags & flag != 0
            } else {
                flags & flag == 0
            };
            text.extend_from_slice(
                kernel
                    .game_text((OPTION_STRING_BASE + if enabled { 4 } else { 3 }) as usize)
                    .to_bytes(),
            );
            if offset == 2 && menu == OPT_PC as i32 && setting > CYBORG_NORMAL_SETTING {
                text.extend_from_slice(if setting == CYBORG_DOUBLE_SETTING {
                    b"+"
                } else {
                    b"++"
                });
            }
        } else if setting == CHANGE_CAPTAIN_SETTING || setting == CHANGE_SHIP_SETTING {
            text.extend_from_slice(kernel.game_text(NAMING_STRING_BASE as usize).to_bytes());
        }
        text.truncate(126);
        std::ffi::CString::new(text)
            .unwrap_or_else(|error| panic!("invalid setting feedback: {error}"))
    }

    fn do_settings(cx: &mut Cx<'_>, io: &mut Io<'_>, input: &InputSession, state: &mut u8) -> bool {
        if cx.world.activity() & CHECK_ABORT as ACTIVITY != 0 {
            return false;
        }
        if key(input, KEY_MENU_CANCEL)
            || (key(input, KEY_MENU_SELECT) && *state == EXIT_SETTINGS_MENU)
        {
            return false;
        }
        if key(input, KEY_MENU_SELECT) {
            match *state {
                SOUND_ON_SETTING | SOUND_OFF_SETTING => {
                    crate::ui::settings::toggle_sound_effect(cx.world);
                    *state ^= 1;
                    crate::ui::menu::draw_menu_state(
                        cx.kernel,
                        cx.world,
                        cx.screen_dimensions(),
                        PM_SOUND_ON as u8,
                        i16::from(*state),
                    );
                }
                MUSIC_ON_SETTING | MUSIC_OFF_SETTING => {
                    crate::ui::settings::toggle_music(cx.world);
                    *state ^= 1;
                    crate::ui::menu::draw_menu_state(
                        cx.kernel,
                        cx.world,
                        cx.screen_dimensions(),
                        PM_SOUND_ON as u8,
                        i16::from(*state),
                    );
                }
                CHANGE_CAPTAIN_SETTING | CHANGE_SHIP_SETTING => {
                    name_captain_or_ship(cx, io, input, *state == CHANGE_CAPTAIN_SETTING)
                }
                _ => {
                    let flags = &mut cx.world.data.game.glob_flags;
                    let speed = (*flags & COMBAT_SPEED_MASK as u8) >> COMBAT_SPEED_SHIFT;
                    let next = if speed < (NUM_COMBAT_SPEEDS - 1) as u8 {
                        *flags |= CYBORG_ENABLED as u8;
                        speed + 1
                    } else {
                        *flags &= !(CYBORG_ENABLED as u8);
                        0
                    };
                    *flags = (*flags & !(COMBAT_SPEED_MASK as u8)) | (next << COMBAT_SPEED_SHIFT);
                    *state = CYBORG_OFF_SETTING + next;
                    crate::ui::menu::draw_menu_state(
                        cx.kernel,
                        cx.world,
                        cx.screen_dimensions(),
                        PM_SOUND_ON as u8,
                        i16::from(*state),
                    );
                }
            }
            feedback_setting(cx, input, *state);
        } else if crate::ui::menu::choose_menu_state(cx, input, state, PM_SOUND_ON as u8) != 0 {
            feedback_setting(cx, input, *state);
        }
        true
    }

    fn settings_menu(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let mut state = SOUND_ON_SETTING;
        crate::ui::menu::draw_menu_state(
            cx.kernel,
            cx.world,
            cx.screen_dimensions(),
            PM_SOUND_ON as u8,
            i16::from(state),
        );
        feedback_setting(cx, input, state);
        cx.run_input(io, false, |cx, io| do_settings(cx, io, input, &mut state));
        draw_status_message(cx.kernel, cx.world, None);
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
