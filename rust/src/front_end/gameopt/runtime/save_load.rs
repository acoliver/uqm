use super::*;

pub(super) unsafe fn save_load_game(
    cx: &mut Cx<'_>,
    input: &crate::engine::input::InputSession,
    pick_state: &mut PickGameState,
    game_index: COUNT,
    canceled_by_user: &mut BOOLEAN,
) -> BOOLEAN {
    let desc = &mut pick_state.summary[game_index as usize];
    let mut name_buf: [UNICODE; 256] = [0; 256];
    let mut save_stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: ptr::null_mut(),
    };
    let success;
    if pick_state.saving != 0 {
        // SAFETY: both fixed arrays contain SAVE_NAME_SIZE bytes; strncpy preserves NUL padding.
        unsafe {
            libc::strncpy(
                name_buf.as_mut_ptr(),
                desc.SaveName.as_ptr(),
                SAVE_NAME_SIZE as usize,
            );
        }
        name_buf[SAVE_NAME_SIZE as usize] = 0;
        // SAFETY: the game-thread picker retains its graphics context and the writable name buffer.
        if cx.with_legacy(|| unsafe { name_save_game(input, game_index, name_buf.as_mut_ptr()) })
            != 0
        {
            play_menu_sound(cx.kernel, cx.world, MENU_SUCCESS as u16);
            // SAFETY: the game-thread picker retains the stamp, selected summary and name buffer.
            success = cx.with_legacy(|| unsafe {
                ConfirmSaveLoad(&mut save_stamp);
                SaveGame(game_index, ptr::from_mut(desc).cast(), name_buf.as_ptr())
            });
        } else {
            success = 0;
            *canceled_by_user = 1;
        }
    } else {
        // SAFETY: loading needs no saved stamp and uses the initialized game-thread loader.
        success = cx.with_legacy(|| unsafe {
            ConfirmSaveLoad(ptr::null_mut());
            LoadGame(game_index, ptr::null_mut())
        });
    }
    if pick_state.saving != 0 {
        // SAFETY: ConfirmSaveLoad initializes the retained stamp before drawing it.
        unsafe { DrawStamp(&mut save_stamp) };
    }
    // SAFETY: save_stamp holds the picker-owned drawable, or the supported null handle.
    unsafe { DestroyDrawable(ReleaseDrawable(save_stamp.frame)) };
    success
}

/// # Safety
/// The game thread retains the picker drawable and its initialized graphics context.
pub(super) unsafe fn restore_display(stamp: &mut STAMP, rect: &RECT, restore: bool) {
    // SAFETY: the caller retains the picker drawable and initialized game-thread graphics context.
    unsafe {
        if restore {
            SetTransitionSource(rect);
            BatchGraphics();
            DrawStamp(stamp);
            ScreenTransition(3, rect);
            UnbatchGraphics();
        }
        DestroyDrawable(ReleaseDrawable(stamp.frame));
    }
}

pub(super) fn pick_game(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
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
    let (frame, context) = (cx.kernel.play_frame, cx.kernel.space_context);
    // SAFETY: the synchronous picker owns these published native resources.
    let (old_callback, old_context, mut dlg_stamp, dlg_rect) = cx.with_legacy(|| unsafe {
        pick_state.summary_frame = SetAbsFrameIndex(frame, 39);
        menu_state.privData = ptr::addr_of_mut!(pick_state).cast();
        menu_state.CurState = LAST_USED_SLOT as BYTE;
        let time_out = FadeMusic(0, (ONE_SECOND / 2) as SIZE);
        let old_callback = SetInputCallback(runtime.input(), None);
        let (old_context, dlg_stamp, dlg_rect) = prepare_display(
            &mut pick_state,
            &menu_state,
            from_main_menu,
            context,
            time_out,
        );
        let navigation_sounds = MENU_SOUND_ARROWS | MENU_SOUND_PAGEUP | MENU_SOUND_PAGEDOWN;
        SetMenuSounds(input, navigation_sounds, 0);
        log::info!("RUST_PICKER: list_drawn slots=50");
        (old_callback, old_context, dlg_stamp, dlg_rect)
    });
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
        // SAFETY: the picker is the sole game-thread writer of this remembered slot.
        unsafe { LAST_USED_SLOT = menu_state.CurState as COUNT };
        // SAFETY: the synchronous picker owns these published native resources.
        if unsafe {
            save_load_game(
                cx,
                input,
                &mut pick_state,
                menu_state.CurState as COUNT,
                &mut canceled_by_user,
            )
        } != 0
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
            load_game_descriptions(ptr::addr_of_mut!(pick_state.summary).cast());
            redraw_pick_display(&mut pick_state, menu_state.CurState as COUNT);
        });
    }
    // SAFETY: the picker owns initialized game-thread menu audio.
    SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
    if pick_state.success != 0 && saving == 0 {
        cx.world
            .set_activity(cx.world.activity() | CHECK_LOAD as ACTIVITY);
    }
    let activity = cx.world.activity();
    let restore = activity & CHECK_ABORT as ACTIVITY == 0
        && (saving != 0 || (pick_state.success == 0 && from_main_menu == 0));
    // SAFETY: the synchronous picker owns these published native resources.
    cx.with_legacy(|| unsafe {
        save_load::restore_display(&mut dlg_stamp, &dlg_rect, restore);
        SetContext(old_context);
        resume_music();
        SetInputCallback(runtime.input(), old_callback);
    });
    pick_state.success
}

unsafe fn prepare_display(
    pick_state: &mut PickGameState,
    menu_state: &MENU_STATE,
    from_main_menu: BOOLEAN,
    space_context: CONTEXT,
    time_out: DWORD,
) -> (CONTEXT, STAMP, RECT) {
    // SAFETY: the caller retains the native picker state and graphics resources.
    unsafe {
        load_game_descriptions(ptr::addr_of_mut!(pick_state.summary).cast());
        let old_context = SetContext(space_context);
        let dlg_stamp = SaveContextFrame(ptr::null());
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
        redraw_pick_display(pick_state, menu_state.CurState as COUNT);
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
        (old_context, dlg_stamp, dlg_rect)
    }
}
