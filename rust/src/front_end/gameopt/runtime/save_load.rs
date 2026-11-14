use super::*;
use crate::engine::save::summary_record::SummaryDesc;
use crate::front_end::gameopt::{picker::PickerState, render::PickerDisplay};
use crate::savegame::live::{
    load::{load_game, load_game_descriptions, LoadRequest},
    save::save_game,
};
use crate::sound::heart_ffi::{fade_music, stop_sound};
use crate::ui::settings::{pause_music, resume_music};
use uqm_engine::time::counter_abi::GetTimeCounter;

struct RestoreInput<'a> {
    input: &'a InputSession,
    callback: Option<crate::engine::input::InputFrameHook>,
}
impl Drop for RestoreInput<'_> {
    fn drop(&mut self) {
        resume_music();
        self.input.replace_callback(self.callback.take());
    }
}

fn save_load_game(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: Option<SolarSnapshot>,
    input: &InputSession,
    display: &PickerDisplay,
    state: &mut PickerState,
) -> (bool, bool) {
    let slot = u16::from(state.slot);
    if state.saving {
        let mut name =
            super::super::naming::save_name_buffer(&state.summary[usize::from(slot)].save_name);
        if !name_save_game(cx, io, input, slot, &mut name) {
            return (false, true);
        }
        play_menu_sound(cx.kernel, cx.world, MENU_SUCCESS);
        let background =
            crate::ui::save_problem::save_confirmation(cx.kernel, &display.context, true);
        let name = std::ffi::CStr::from_bytes_until_nul(&name)
            .unwrap_or_else(|error| panic!("invalid entered save name: {error}"));
        let success = save_game(cx, solar, slot, &mut state.summary[usize::from(slot)], name) != 0;
        if let Some(saved) = background {
            saved.restore();
        }
        (success, false)
    } else {
        crate::ui::save_problem::save_confirmation(cx.kernel, &display.context, false);
        (load_game(slot, LoadRequest::Restore(cx)) != 0, false)
    }
}

fn picker_frame(
    cx: &mut Cx<'_>,
    input: &InputSession,
    display: &PickerDisplay,
    state: &mut PickerState,
) -> bool {
    let activity = cx.world.activity();
    let time_in = GetTimeCounter();
    let action = state.action(
        activity & CHECK_ABORT != 0,
        key(input, KEY_MENU_CANCEL),
        key(input, KEY_MENU_SELECT),
    );
    if action != PickerAction::Continue {
        log::info!(
            "RUST_PICKER: action={action:?} slot={} select={} cancel={} activity={activity:#x}",
            state.slot,
            key(input, KEY_MENU_SELECT),
            key(input, KEY_MENU_CANCEL)
        );
    }
    match action {
        PickerAction::Abort => return false,
        PickerAction::Cancel => {
            state.success = false;
            return false;
        }
        PickerAction::Select => {
            play_menu_sound(cx.kernel, cx.world, MENU_SUCCESS);
            state.success = true;
            return false;
        }
        PickerAction::EmptySlot => play_menu_sound(cx.kernel, cx.world, MENU_FAILURE),
        PickerAction::Continue => {
            if state.navigate(
                key(input, KEY_MENU_LEFT) || key(input, KEY_MENU_PAGE_UP),
                key(input, KEY_MENU_RIGHT) || key(input, KEY_MENU_PAGE_DOWN),
                key(input, KEY_MENU_UP),
                key(input, KEY_MENU_DOWN),
            ) {
                display.context.select();
                display.redraw(cx.kernel, cx.world, cx.screen_dimensions(), cx.rng, state);
            }
            cx.kernel
                .input_runtime()
                .sleep_until(cx, time_in.wrapping_add(ONE_SECOND / 30));
        }
    }
    true
}

pub(super) fn pick_game(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: Option<SolarSnapshot>,
    saving: BOOLEAN,
    from_main_menu: BOOLEAN,
) -> BOOLEAN {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    log::info!(
        "RUST_PICKER: enter saving={saving} main_menu={from_main_menu} activity={:#x}",
        cx.world.activity()
    );
    let display = PickerDisplay::new(cx.kernel);
    let mut summaries = [SummaryDesc::default(); MAX_SAVED_GAMES];
    let time_out = fade_music(0, (ONE_SECOND / 2) as i16);
    let restore = RestoreInput {
        input,
        callback: input.replace_callback(None),
    };
    load_game_descriptions(&mut summaries);
    let mut state = PickerState::new(summaries, saving != 0, cx.kernel.last_save_slot);
    display.context.with_selected(|| {
        display.context.with_text_state(|| {
            let saved = crate::ui::util::SavedContext::capture(&display.context, None);
            let rect = display.context.clip_rect();
            runtime.sleep_until(cx, time_out);
            pause_music();
            stop_sound();
            fade_music(160, 0);
            display.context.transition_source(None);
            display.context.with_batch(|| {
                display.context.set_background(BLACK_COLOR);
                display.context.clear();
                display.redraw(cx.kernel, cx.world, cx.screen_dimensions(), cx.rng, &state);
                display.header(state.saving);
                if from_main_menu == 0 {
                    display
                        .context
                        .transition(Some(display.context.clip_rect()));
                }
            });
            if from_main_menu != 0 {
                display.context.fade_screen(253, (ONE_SECOND / 2) as i16);
            }
            input.set_menu_sounds(
                MENU_SOUND_ARROWS | MENU_SOUND_PAGEUP | MENU_SOUND_PAGEDOWN,
                0,
            );
            log::info!("RUST_PICKER: list_drawn slots=50");
            run_picker(cx, io, solar, input, &display, &mut state);
            input.set_menu_sounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
            if state.success && !state.saving {
                cx.world.set_activity(cx.world.activity() | CHECK_LOAD);
            }
            let restore = cx.world.activity() & CHECK_ABORT == 0
                && (state.saving || (!state.success && from_main_menu == 0));
            if restore {
                display.context.transition_source(Some(rect));
                display.context.with_batch(|| {
                    saved.restore();
                    display.context.transition(Some(rect));
                });
            }
        })
    });
    drop(restore);
    u32::from(state.success)
}

fn run_picker(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: Option<SolarSnapshot>,
    input: &InputSession,
    display: &PickerDisplay,
    state: &mut PickerState,
) {
    loop {
        state.success = false;
        cx.run_input(io, true, |cx, _| picker_frame(cx, input, display, state));
        if !state.success {
            break;
        }
        cx.kernel.last_save_slot = state.slot;
        let (success, cancelled) = save_load_game(cx, io, solar, input, display, state);
        if success {
            log::info!(
                "RUST_PICKER: exit={}_success",
                if state.saving { "save" } else { "load" }
            );
            break;
        }
        log::info!("RUST_PICKER: operation_failed picker_remains_open");
        if state.saving && !cancelled {
            crate::ui::save_problem::save_problem(cx, io);
        }
        load_game_descriptions(&mut state.summary);
        display.redraw(cx.kernel, cx.world, cx.screen_dimensions(), cx.rng, state);
    }
}

fn name_save_game(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &InputSession,
    game_index: u16,
    buffer: &mut [u8; 256],
) -> bool {
    let text = std::ffi::CStr::from_bytes_until_nul(buffer)
        .unwrap_or_else(|error| panic!("invalid save name: {error}"));
    let cursor = text.to_bytes().len();
    draw_save_name_string(cx.kernel, cx.world, text, cursor, DDSHS_EDIT, game_index);
    let context = ContextLease::require(cx.kernel.space_context);
    let screen = ContextLease::require(cx.kernel.screen_context);
    let rect = RECT {
        corner: POINT {
            x: 30 + SAFE_X,
            y: (160 + ((game_index % SAVES_PER_PAGE) * 13)) as i16,
        },
        extent: EXTENT {
            width: 204 - SAFE_X,
            height: 11,
        },
    };
    set_flash_rect(
        &cx.kernel.sis_highlight,
        &screen,
        context.clip_rect(),
        Some(rect),
    );
    let accepted = crate::ui::getchar::edit_text(
        cx,
        io,
        input,
        SAVE_NAME_SIZE,
        &mut buffer[..SAVE_NAME_SIZE],
        cursor,
        &mut SaveNameCallbacks { index: game_index },
    );
    if !accepted {
        buffer[0] = 0;
    }
    cx.kernel.sis_highlight.context.borrow_mut().take();
    let text = std::ffi::CStr::from_bytes_until_nul(buffer)
        .unwrap_or_else(|error| panic!("invalid edited save name: {error}"));
    draw_save_name_string(cx.kernel, cx.world, text, cursor, DDSHS_NORMAL, game_index);
    if let Some(callback) = cx.kernel.naming_callback {
        callback(cx.kernel, cx.world, cx.screen_dimensions().0);
    }
    input.set_menu_sounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
    cx.kernel.sis_highlight.context.borrow_mut().take();
    accepted
}
