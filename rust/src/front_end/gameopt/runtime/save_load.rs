use super::*;

pub(super) unsafe fn save_load_game(
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
        if unsafe { name_save_game(game_index, name_buf.as_mut_ptr()) } != 0 {
            // SAFETY: the game-thread picker retains the stamp, selected summary and name buffer.
            unsafe {
                PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_SUCCESS);
                ConfirmSaveLoad(&mut save_stamp);
                success = SaveGame(game_index, ptr::from_mut(desc).cast(), name_buf.as_ptr());
            }
        } else {
            success = 0;
            *canceled_by_user = 1;
        }
    } else {
        // SAFETY: loading needs no saved stamp and uses the initialized game-thread loader.
        unsafe {
            ConfirmSaveLoad(ptr::null_mut());
            success = LoadGame(game_index, ptr::null_mut());
        }
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
