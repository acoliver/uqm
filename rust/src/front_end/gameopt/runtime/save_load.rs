use super::*;
use crate::savegame::live::load::{load_game_descriptions, LoadRequest};

pub(super) fn save_load_game(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: Option<SolarSnapshot>,
    input: &crate::engine::input::InputSession,
    pick_state: &mut PickGameState,
    game_index: COUNT,
    canceled_by_user: &mut BOOLEAN,
) -> BOOLEAN {
    let desc = &mut pick_state.summary[game_index as usize];
    let mut name_buf = super::super::naming::save_name_buffer(&desc.SaveName);
    let context = crate::graphics::context_abi::ContextLease::retain(
        std::ptr::NonNull::new(cx.kernel.space_context)
            .unwrap_or_else(|| panic!("save/load context")),
    );
    let mut background = None;
    let success;
    if pick_state.saving != 0 {
        if name_save_game(cx, io, input, game_index, &mut name_buf) {
            play_menu_sound(cx.kernel, cx.world, MENU_SUCCESS as u16);
            // SAFETY: the game-thread picker retains the stamp, selected summary and name buffer.
            background = crate::ui::save_problem::save_confirmation(cx.kernel, &context, true);
            let summary = ptr::from_mut(desc).cast();
            let name = name_buf.as_ptr().cast();
            // SAFETY: the picker retains its writable summary and terminated name.
            success = unsafe { SaveGame(cx, solar, game_index, summary, name) };
        } else {
            success = 0;
            *canceled_by_user = 1;
        }
    } else {
        // SAFETY: loading needs no saved stamp and uses the initialized game-thread loader.
        crate::ui::save_problem::save_confirmation(cx.kernel, &context, false);
        // SAFETY: the picker retains the initialized World and Kernel through loading.
        success = unsafe { LoadGame(game_index, LoadRequest::Restore(cx)) };
    }
    if let Some(saved) = background {
        saved.restore();
    }
    success
}

/// # Safety
/// The game thread retains the picker drawable and its initialized graphics context.
pub(super) unsafe fn restore_display(
    saved: crate::ui::util::SavedContext,
    rect: &RECT,
    restore: bool,
) {
    // SAFETY: the picker retains the selected context and all queued drawing resources.
    unsafe {
        if restore {
            SetTransitionSource(rect);
            BatchGraphics();
            saved.restore();
            ScreenTransition(3, rect);
            UnbatchGraphics();
        }
    }
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
    let activity = cx.world.activity();
    log::info!(
        "RUST_PICKER: enter saving={saving} main_menu={from_main_menu} activity={activity:#x}"
    );
    // SAFETY: this native record contains integers and nullable pointers.
    let mut pick_state: PickGameState = unsafe { mem::zeroed() };
    pick_state.saving = saving;
    // SAFETY: this native record contains integers and nullable pointers.
    let mut menu_state: MENU_STATE = unsafe { mem::zeroed() };
    // SAFETY: the synchronous picker retains its menu, summary and native contexts.
    let (old_context, dlg_stamp, dlg_rect, old_callback) =
        unsafe { prepare_display(cx, input, &mut pick_state, &mut menu_state, from_main_menu) };
    let navigation_sounds = MENU_SOUND_ARROWS | MENU_SOUND_PAGEUP | MENU_SOUND_PAGEDOWN;
    SetMenuSounds(input, navigation_sounds, 0);
    log::info!("RUST_PICKER: list_drawn slots=50");
    loop {
        let mut canceled_by_user: BOOLEAN = 0;
        pick_state.success = 0;
        cx.run_input(io, true, |cx, _| {
            // SAFETY: the synchronous picker retains its menu and graphics resources.
            unsafe { do_pick_game(cx, input, ptr::addr_of_mut!(menu_state)) != 0 }
        });
        if pick_state.success == 0 {
            break;
        }
        let slot = menu_state.CurState as COUNT;
        // SAFETY: the picker is the sole game-thread writer of this remembered slot.
        unsafe { LAST_USED_SLOT = slot };
        if save_load_game(
            cx,
            io,
            solar,
            input,
            &mut pick_state,
            slot,
            &mut canceled_by_user,
        ) != 0
        {
            let operation = if saving != 0 { "save" } else { "load" };
            log::info!("RUST_PICKER: exit={operation}_success");
            break;
        }
        log::info!("RUST_PICKER: operation_failed picker_remains_open");
        if saving != 0 && canceled_by_user == 0 {
            crate::ui::save_problem::save_problem(cx, io);
        }
        // SAFETY: the synchronous picker owns these published native resources.
        cx.with_legacy(|| unsafe {
            load_game_descriptions(&mut *ptr::addr_of_mut!(pick_state.summary).cast());
        });
        // SAFETY: the picker retains its initialized summary and drawing contexts.
        unsafe {
            redraw_pick_display(cx, &mut pick_state, menu_state.CurState as COUNT);
        }
    }
    // SAFETY: the picker owns initialized game-thread menu audio.
    SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
    if pick_state.success != 0 && saving == 0 {
        cx.world
            .set_activity(cx.world.activity() | CHECK_LOAD as ACTIVITY);
    }
    let restore = cx.world.activity() & CHECK_ABORT as ACTIVITY == 0
        && (saving != 0 || (pick_state.success == 0 && from_main_menu == 0));
    // SAFETY: the synchronous picker owns these published native resources.
    cx.with_legacy(|| unsafe {
        save_load::restore_display(dlg_stamp, &dlg_rect, restore);
        SetContext(old_context);
        resume_music();
        SetInputCallback(runtime.input(), old_callback);
    });
    pick_state.success
}

unsafe fn prepare_display(
    cx: &mut Cx<'_>,
    input: &InputSession,
    pick_state: &mut PickGameState,
    menu_state: &mut MENU_STATE,
    from_main_menu: BOOLEAN,
) -> (
    CONTEXT,
    crate::ui::util::SavedContext,
    RECT,
    Option<crate::engine::input::InputFrameHook>,
) {
    let (frame, space_context) = (cx.kernel.play_frame, cx.kernel.space_context);
    // SAFETY: the picker retains its summary, source context and drawable resources.
    let (old_context, dlg_stamp, dlg_rect, old_callback) = cx.with_legacy(|| unsafe {
        pick_state.summary_frame = SetAbsFrameIndex(frame, 39);
        menu_state.privData = ptr::from_mut(pick_state).cast();
        menu_state.CurState = LAST_USED_SLOT as BYTE;
        let time_out = FadeMusic(0, (ONE_SECOND / 2) as SIZE);
        let old_callback = SetInputCallback(input, None);
        load_game_descriptions(&mut *ptr::addr_of_mut!(pick_state.summary).cast());
        let old_context = SetContext(space_context);
        let dlg_stamp = crate::ui::util::SavedContext::capture(
            &crate::graphics::context_abi::ContextLease::require(space_context.cast()),
            None,
        );
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
        (old_context, dlg_stamp, dlg_rect, old_callback)
    });
    // SAFETY: the initialized picker state and drawing contexts remain live.
    unsafe { redraw_pick_display(cx, pick_state, menu_state.CurState as COUNT) };
    // SAFETY: the picker retains the initialized drawable through the transition.
    cx.with_legacy(|| unsafe {
        draw_save_load(pick_state);
        if from_main_menu != 0 {
            UnbatchGraphics();
            FadeScreen(ScreenFadeType_FadeAllToColor, (ONE_SECOND / 2) as SIZE);
        } else {
            let mut ctx_rect: RECT = mem::zeroed();
            GetContextClipRect(&mut ctx_rect);
            ScreenTransition(3, &ctx_rect);
            UnbatchGraphics();
        }
    });
    (old_context, dlg_stamp, dlg_rect, old_callback)
}

/// Preserve each preview label's existing byte limit, including partial UTF-8 bytes.
pub(super) fn summary_date(
    kernel: &Kernel,
    summary: &SUMMARY_DESC,
    max_bytes: usize,
) -> std::ffi::CString {
    let mut bytes = crate::ui::sis::campaign_date(
        kernel,
        summary.month_index,
        summary.day_index,
        summary.year_index,
    )
    .into_bytes();
    bytes.truncate(max_bytes);
    std::ffi::CString::new(bytes).unwrap_or_else(|error| panic!("invalid preview date: {error}"))
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
            x: (30 + SAFE_X) as i16,
            y: (160 + ((game_index % SAVES_PER_PAGE) * 13)) as i16,
        },
        extent: EXTENT {
            width: (204 - SAFE_X) as i16,
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
        SAVE_NAME_SIZE as usize,
        &mut buffer[..SAVE_NAME_SIZE as usize],
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
    SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
    cx.kernel.sis_highlight.context.borrow_mut().take();
    accepted
}
