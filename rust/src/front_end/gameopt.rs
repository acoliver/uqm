//! Game options, naming, and save/load selection, ported from `uqm/gameopt.c`.

#[cfg(test)]
mod picker_tests;

mod layout;
mod naming;
mod preview;
mod render;
#[cfg(test)]
mod render_tests;

mod picker;
#[cfg(test)]
use picker::picker_slot;
use picker::MAX_SAVED_GAMES;
#[cfg(not(test))]
use picker::SAVES_PER_PAGE;
pub use picker::{picker_action, PickerAction};

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

    use super::{PickerAction, MAX_SAVED_GAMES, SAVES_PER_PAGE};
    use crate::engine::world::{
        activity::activity_flags::{CHECK_ABORT, CHECK_LOAD},
        SOUND_DISABLED,
    };
    use crate::engine::world::{generation::SolarSnapshot, MUSIC_DISABLED};
    use crate::engine::{battle::Cx, input::InputSession, observe::Io, Kernel};
    use crate::graphics::context_abi::ContextLease;
    use crate::graphics::native_abi::{BOOLEAN, COUNT, EXTENT, POINT, RECT};
    use crate::input::frames::*;
    use crate::starbase::ids::{NAMING_STRING_BASE, OPTION_STRING_BASE, SAFE_X};
    use crate::ui::menu_state::{PM_SAVE_GAME, PM_SOUND_ON};
    use crate::ui::settings::{play_menu_sound, OPT_PC};
    use crate::ui::sis::{draw_status_message, set_flash_rect, set_menu_flash};
    use uqm_engine::time::counter_abi::ONE_SECOND;

    const SIS_NAME_SIZE: usize = 16;
    const SAVE_NAME_SIZE: usize = crate::savegame::types::SAVE_NAME_SIZE;
    const CYBORG_ENABLED: u8 = 32;
    const COMBAT_SPEED_SHIFT: u32 = 6;
    const COMBAT_SPEED_MASK: u8 = 192;
    const NUM_COMBAT_SPEEDS: u8 = 4;
    const MENU_SUCCESS: u16 = 1;
    const MENU_FAILURE: u16 = 2;

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
    const MENU_SOUND_ARROWS: u16 = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3);
    const MENU_SOUND_SELECT: u16 = 1 << 4;
    const MENU_SOUND_PAGEUP: u16 = 1 << 7;
    const MENU_SOUND_PAGEDOWN: u16 = 1 << 8;

    fn key(input: &InputSession, index: u32) -> bool {
        input.pulsed().menu[index as usize] != 0
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
        let mut buffer = [0u8; SIS_NAME_SIZE];
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

    fn do_game_options(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        solar: Option<SolarSnapshot>,
        state: &mut u8,
    ) -> bool {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        if cx.world.activity() & CHECK_ABORT != 0
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
                        PM_SAVE_GAME,
                        i16::from(*state),
                    );
                }
                _ => {}
            }
        } else {
            crate::ui::menu::choose_menu_state(cx, input, state, PM_SAVE_GAME);
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
        if cx.kernel.last_activity == CHECK_LOAD {
            crate::ui::menu::draw_menu_state(
                cx.kernel,
                cx.world,
                cx.screen_dimensions(),
                PM_SAVE_GAME,
                LOAD_GAME as i16,
            );
            if pick_game(cx, io, solar, 0, 1) == 0 {
                cx.world.set_activity(cx.world.activity() | CHECK_ABORT);
            }
            return 0;
        }
        let mut state = SAVE_GAME;
        crate::ui::menu::draw_menu_state(
            cx.kernel,
            cx.world,
            cx.screen_dimensions(),
            PM_SAVE_GAME,
            i16::from(state),
        );
        set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
        input.set_menu_sounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        io.observer
            .checkpoint(&crate::engine::observe::CheckpointView::GameOptions(true));
        cx.run_input(io, true, |cx, io| {
            do_game_options(cx, io, solar, &mut state)
        });
        io.observer
            .checkpoint(&crate::engine::observe::CheckpointView::GameOptions(false));
        cx.kernel.sis_highlight.context.borrow_mut().take();
        u32::from(cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0)
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
            SOUND_ON_SETTING | SOUND_OFF_SETTING => Some((0, SOUND_DISABLED)),
            MUSIC_ON_SETTING | MUSIC_OFF_SETTING => Some((1, MUSIC_DISABLED)),
            CYBORG_OFF_SETTING..=CYBORG_SUPER_SETTING => Some((2, CYBORG_ENABLED)),
            _ => None,
        };
        if let Some((offset, flag)) = option {
            text.extend_from_slice(
                kernel
                    .game_text((OPTION_STRING_BASE + offset) as usize)
                    .to_bytes(),
            );
            text.push(b' ');
            let enabled = if flag == CYBORG_ENABLED {
                flags & flag != 0
            } else {
                flags & flag == 0
            };
            text.extend_from_slice(
                kernel
                    .game_text((OPTION_STRING_BASE + if enabled { 4 } else { 3 }) as usize)
                    .to_bytes(),
            );
            if offset == 2 && menu == OPT_PC && setting > CYBORG_NORMAL_SETTING {
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
        if cx.world.activity() & CHECK_ABORT != 0 {
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
                        PM_SOUND_ON,
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
                        PM_SOUND_ON,
                        i16::from(*state),
                    );
                }
                CHANGE_CAPTAIN_SETTING | CHANGE_SHIP_SETTING => {
                    name_captain_or_ship(cx, io, input, *state == CHANGE_CAPTAIN_SETTING)
                }
                _ => {
                    let flags = &mut cx.world.data.game.glob_flags;
                    let speed = (*flags & COMBAT_SPEED_MASK) >> COMBAT_SPEED_SHIFT;
                    let next = if speed < (NUM_COMBAT_SPEEDS - 1) {
                        *flags |= CYBORG_ENABLED;
                        speed + 1
                    } else {
                        *flags &= !(CYBORG_ENABLED);
                        0
                    };
                    *flags = (*flags & !(COMBAT_SPEED_MASK)) | (next << COMBAT_SPEED_SHIFT);
                    *state = CYBORG_OFF_SETTING + next;
                    crate::ui::menu::draw_menu_state(
                        cx.kernel,
                        cx.world,
                        cx.screen_dimensions(),
                        PM_SOUND_ON,
                        i16::from(*state),
                    );
                }
            }
            feedback_setting(cx, input, *state);
        } else if crate::ui::menu::choose_menu_state(cx, input, state, PM_SOUND_ON) != 0 {
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
            PM_SOUND_ON,
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
