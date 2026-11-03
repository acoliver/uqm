use super::*;

pub(super) unsafe fn save_load_game(
    pick_state: *mut PickGameState,
    game_index: COUNT,
    canceled_by_user: *mut BOOLEAN,
) -> BOOLEAN {
    // SAFETY: the caller supplies live picker records and writable cancellation state on the game thread.
    unsafe {
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
