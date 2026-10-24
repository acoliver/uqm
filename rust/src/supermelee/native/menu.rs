//! Super Melee setup lifecycle and menu dispatch.
mod rules;
#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod live {
    use crate::automation::coordinator::rust_automation_seed_value;
    use crate::battle::{COMPUTER_CONTROL, HUMAN_CONTROL, NETWORK_CONTROL, PSYTRON_CONTROL};
    use crate::comm::hail::game_string;
    use crate::game_init::macro_abi::{LoadMusic, LoadSound};
    use crate::graphics::drawable_abi;
    use crate::graphics::resgfx::LoadGraphicInstance as LoadGraphic;
    use crate::io::uio_bridge;
    use crate::mainloop::restart_menu::c_extern::{DoPopupWindow, FadeMusic, SeedRandomNumbers};
    use crate::math::{delete_context, new_context};
    use crate::navigation::abi::{InitSpace, MeleeConnectDialog, UninitSpace, TEXTENTRY_STATE};
    use crate::navigation::globals::InitGlobData;
    use crate::ships::native::abi::*;
    use crate::supermelee::c_bridge::PlayerControl;
    use crate::ui::shipstat::ClearShipStatus;
    mod boundary;
    use boundary::*;
    const FADE_ALL_TO_COLOR: u32 = 253;
    const FADE_ALL_TO_BLACK: u32 = 252;
    use crate::supermelee::native::{load_teams::live::*, teams::*};
    use std::{
        ffi::{c_char, CString},
        mem, ptr,
    };
    mod battle;
    pub(crate) mod changes;
    pub(crate) mod drawing;
    mod editing;
    use battle::*;
    use changes::{
        view_name, Melee_LocalChange_ship, Melee_LocalChange_team, Melee_LocalChange_teamName,
    };
    use drawing::{
        controls, current, highlight, init_screen, name, redraw, rgb, ship_box, status,
        status_player, text, worth, DrawMeleeShipStrings, GetShipBox, MeleeFrame,
        Melee_flashSelection, RepairMeleeFrame, P_MELEE_STATE,
    };
    use editing::*;
    pub(super) unsafe fn key(key: u32) -> bool {
        PulsedInputState.menu[key as usize] != 0
    }
    unsafe fn select_option(state: *mut MELEE_STATE) {
        match (*state).MeleeOption {
            4 => start_button(state),
            3 | 5 => {
                (*state).Initialized = 0;
                (*state).side = u16::from((*state).MeleeOption != 3);
                DoLoadTeam(state);
            }
            2 | 6 => {
                (*state).side = u16::from((*state).MeleeOption != 2);
                if MeleeSetup_getFleetValue((*state).meleeSetup, (*state).side as usize) > 0 {
                    DoSaveTeam(state);
                } else {
                    PlayMenuSound(2);
                }
            }
            9 => GlobData.Game_state.CurrentActivity |= CHECK_ABORT as u16,
            0 | 8 => {
                let side = u16::from((*state).MeleeOption == 0);
                let confirmed = MeleeConnectDialog(side as i32);
                redraw();
                (*state).LastInputTime = GetTimeCounter();
                if confirmed != 0 {
                    (*state).Initialized = 0;
                    (*state).InputFunc = Some(connecting);
                }
            }
            1 | 7 => next_control(usize::from((*state).MeleeOption == 1)),
            _ => {}
        }
    }
    #[no_mangle]
    /// # Safety
    /// Call on the game thread after kernel/resource initialization. Any state
    /// pointer must remain exclusively accessible through the nested input loop.
    pub unsafe extern "C" fn DoMelee(state: *mut MELEE_STATE) -> u32 {
        let time = GetTimeCounter();
        GamePaused = 0;
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return 0;
        }
        SetMenuSounds(15, 16);
        if (*state).Initialized == 0 {
            if !(*state).hMusic.is_null() {
                StopMusic();
                DestroyMusic((*state).hMusic);
                (*state).hMusic = ptr::null_mut();
            }
            (*state).hMusic = LoadMusic(MELEE_MUSIC.as_ptr().cast()).cast();
            (*state).Initialized = 1;
            (*state).MeleeOption = 4;
            PlayMusic((*state).hMusic, 1, 1);
            init_screen();
            FadeScreen(FADE_ALL_TO_COLOR, 840 / 2);
            (*state).LastInputTime = GetTimeCounter();
            return 1;
        }
        netInput();
        if key(KEY_MENU_CANCEL) || key(KEY_MENU_LEFT) {
            (*state).LastInputTime = GetTimeCounter();
            highlight((*state).MeleeOption, false);
            (*state).MeleeOption = 10;
            (*state).Initialized = 0;
            (*state).side = 0;
            if key(KEY_MENU_CANCEL) {
                (*state).row = 0;
                (*state).col = 0;
            } else {
                (*state).row = 1;
                (*state).col = 6;
            }
            edit(state);
        } else {
            let mut next = (*state).MeleeOption;
            let mut force = false;
            if key(KEY_MENU_UP) {
                (*state).LastInputTime = GetTimeCounter();
                next = super::rules::next_option(next, true, false);
            } else if key(KEY_MENU_DOWN) {
                (*state).LastInputTime = GetTimeCounter();
                next = super::rules::next_option(next, false, true);
            }
            if PlayerControl[0] & PlayerControl[1] & PSYTRON_CONTROL != 0
                && GetTimeCounter().wrapping_sub((*state).LastInputTime) > 840 * 10
            {
                force = true;
                next = 4;
            }
            if next != (*state).MeleeOption {
                if (*state).MeleeOption == 1 || (*state).MeleeOption == 7 {
                    status_player(-1);
                }
                highlight((*state).MeleeOption, false);
                (*state).MeleeOption = next;
                highlight(next, true);
                if next == 1 || next == 7 {
                    let side = usize::from(next == 1);
                    status_player(if PlayerControl[side] & NETWORK_CONTROL != 0 {
                        side as isize
                    } else {
                        -1
                    });
                }
            }
            if key(KEY_MENU_SELECT) || force {
                select_option(state);
                if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
                    return 0;
                }
            }
        }
        flushPacketQueues();
        disconnects();
        Melee_flashSelection(state);
        SleepThreadUntil(time.wrapping_add(840 / 30));
        1
    }
    unsafe fn load_config(state: *mut MELEE_STATE) -> bool {
        let stream = uio_bridge::uio_fopen(configDir.cast(), c"melee.cfg".as_ptr(), c"rb".as_ptr());
        if stream.is_null() {
            return false;
        }
        let mut sb: libc::stat = mem::zeroed();
        let mut ok = uio_bridge::uio_fstat(uio_bridge::uio_streamHandle(stream), &mut sb) == 0
            && sb.st_size == 140;
        if ok {
            for side in [0usize, 1] {
                let byte = crate::resource::ffi_bridge::GetResFileChar(stream.cast());
                if byte == -1 {
                    ok = false;
                    break;
                }
                PlayerControl[side] = byte as u8;
                if MeleeSetup_deserializeTeam((*state).meleeSetup, side, stream.cast()) == -1 {
                    ok = false;
                    break;
                }
                if PlayerControl[side] & NETWORK_CONTROL != 0 {
                    PlayerControl[side] = HUMAN_CONTROL | 16;
                }
            }
        }
        uio_bridge::uio_fclose(stream);
        ok
    }
    unsafe fn write_config(state: *mut MELEE_STATE) -> bool {
        use crate::resource::ffi_bridge::{res_CloseResFile, res_OpenResFile, DeleteResFile};
        let stream = res_OpenResFile(configDir.cast(), c"melee.cfg".as_ptr(), c"wb".as_ptr());
        if stream.is_null() {
            return false;
        }
        let mut ok = true;
        for side in [0usize, 1] {
            if crate::resource::ffi_bridge::PutResFileChar(
                PlayerControl[side] as std::ffi::c_char,
                stream.cast(),
            ) == -1
                || MeleeSetup_serializeTeam((*state).meleeSetup, side, stream.cast()) == -1
            {
                ok = false;
                break;
            }
        }
        if ok {
            ok = res_CloseResFile(stream) != 0;
        }
        if !ok {
            res_CloseResFile(stream);
            DeleteResFile(configDir.cast(), c"melee.cfg".as_ptr());
        }
        ok
    }
    #[no_mangle]
    /// # Safety
    /// Call on the game thread after kernel/resource initialization. Any state
    /// pointer must remain exclusively accessible through the nested input loop.
    pub unsafe extern "C" fn Melee() {
        InitGlobData();
        let mut state: MELEE_STATE = mem::zeroed();
        P_MELEE_STATE = &mut state;
        state.InputFunc = Some(DoMelee);
        state.meleeSetup = MeleeSetup_new();
        state.randomContext = new_context();
        (*state.randomContext).reseed(rust_automation_seed_value(1, GetTimeCounter()));
        for player in [0usize, 1] {
            set_connection(player, ptr::null_mut());
        }
        state.currentShip = 255;
        state.CurIndex = u16::MAX;
        InitMeleeLoadState(&mut state);
        GlobData.Game_state.CurrentActivity = SUPER_MELEE as u16;
        GameSounds = CaptureSound(LoadSound(GAME_SOUNDS.as_ptr().cast()));
        crate::supermelee::native::battle_picker::live::BuildPickMeleeFrame();
        MeleeFrame = CaptureDrawable(LoadGraphic(MELEE_SCREEN_PMAP_ANIM.as_ptr().cast()).cast());
        crate::supermelee::native::build_picker::live::BuildBuildPickFrame();
        InitSpace();
        LoadTeamList(&mut state);
        if !load_config(&mut state) {
            PlayerControl[0] = HUMAN_CONTROL | 16;
            Melee_LocalChange_team(&mut state, 0, *state.load.preBuiltList);
            PlayerControl[1] = COMPUTER_CONTROL | 16;
            Melee_LocalChange_team(&mut state, 1, *state.load.preBuiltList.add(1));
        }
        state.side = 0;
        SetMenuSounds(15, 16);
        crate::mainloop::restart_menu::c_extern::DoInput(
            (&mut state as *mut MELEE_STATE).cast(),
            1,
        );
        StopMusic();
        WaitForSoundEnd(u16::MAX);
        write_config(&mut state);
        DestroyStringTable(ReleaseStringTable(state.load.dirEntries));
        state.load.dirEntries = ptr::null_mut();
        if !state.hMusic.is_null() {
            DestroyMusic(state.hMusic);
            state.hMusic = ptr::null_mut();
        }
        UninitSpace();
        crate::supermelee::native::battle_picker::live::DestroyPickMeleeFrame();
        DestroyDrawable(ReleaseDrawable(MeleeFrame));
        MeleeFrame = ptr::null_mut();
        crate::supermelee::native::build_picker::live::DestroyBuildPickFrame();
        closeAllConnections();
        setupInputDelay(0);
        DestroySound(ReleaseSound(GameSounds).cast());
        GameSounds = ptr::null_mut();
        UninitMeleeLoadState(&mut state);
        delete_context(state.randomContext);
        MeleeSetup_delete(state.meleeSetup);
        FlushInput();
    }
}

#[cfg(test)]
mod rules_tests;
