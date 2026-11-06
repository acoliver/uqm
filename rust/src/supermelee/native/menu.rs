//! Super Melee setup lifecycle and menu dispatch.
mod rules;
#[cfg(not(test))]
pub(crate) mod live {
    use crate::battle::{COMPUTER_CONTROL, HUMAN_CONTROL, NETWORK_CONTROL, PSYTRON_CONTROL};
    use crate::comm::hail::game_string;
    use crate::engine::{Kernel as KernelOwner, World as WorldOwner};

    use crate::game_init::macro_abi::{LoadMusic, LoadSound};
    use crate::graphics::{drawable_abi, resgfx::LoadGraphicInstance as LoadGraphic};

    use crate::io::uio_bridge;
    use crate::mainloop::restart_menu::c_extern::{do_popup_window, FadeMusic, SeedRandomNumbers};
    use crate::math::{delete_context, new_context};
    use crate::navigation::abi::{init_space, uninit_space, MeleeConnectDialog, TEXTENTRY_STATE};

    use crate::ships::native::abi::*;

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
    unsafe fn run_menu_frame(
        state: *mut MELEE_STATE,
        cx: &mut crate::engine::battle::Cx<'_>,
        input: &crate::engine::input::InputSession,
    ) -> bool {
        use crate::supermelee::netplay::session::MenuPhase;
        match (*state).phase {
                MenuPhase::Confirm => confirm(input, state, cx) != 0,
                phase => cx.with_legacy(|| match phase {
                    MenuPhase::Menu => do_melee(state, input),
                    MenuPhase::Connecting => connecting(state, input),
                    MenuPhase::Editing => edit(state, input),
                    MenuPhase::Loading => do_load_team(state, input),
                    MenuPhase::Building => panic!("build picker escaped its nested input loop"),
                    MenuPhase::Confirm => unreachable!(),
                } != 0),
            }
    }
    pub(super) unsafe fn key(input: &crate::engine::input::InputSession, key: u32) -> bool {
        input.pulsed().menu[key as usize] != 0
    }
    unsafe fn select_option(input: &crate::engine::input::InputSession, state: *mut MELEE_STATE) {
        match (*state).melee_option {
            4 => start_button(state),
            3 | 5 => {
                (*state).initialized = 0;
                (*state).side = u16::from((*state).melee_option != 3);
                do_load_team(state, input);
            }
            2 | 6 => {
                (*state).side = u16::from((*state).melee_option != 2);
                if MeleeSetup_getFleetValue((*state).melee_setup, (*state).side as usize) > 0 {
                    DoSaveTeam(state);
                } else {
                    PlayMenuSound(2);
                }
            }
            9 => crate::engine::World::current()
                .write(|world| world.set_activity(world.activity() | CHECK_ABORT as u16)),
            0 | 8 => {
                let side = u16::from((*state).melee_option == 0);
                let confirmed = MeleeConnectDialog(&mut (*state).netplay, side as i32);
                redraw();
                (*state).last_input_time = GetTimeCounter();
                if confirmed != 0 {
                    (*state).initialized = 0;
                    (*state).phase = crate::supermelee::netplay::session::MenuPhase::Connecting;
                }
            }
            1 | 7 => next_control(usize::from((*state).melee_option == 1)),
            _ => {}
        }
    }
    /// # Safety
    /// Call on the game thread after kernel/resource initialization. Any state
    /// pointer must remain exclusively accessible through the nested input loop.
    pub unsafe fn do_melee(
        state: *mut MELEE_STATE,
        input: &crate::engine::input::InputSession,
    ) -> u32 {
        let time = GetTimeCounter();
        input.set_paused(false);
        let activity = crate::engine::World::current().read(|world| world.activity());
        if activity & CHECK_ABORT as u16 != 0 {
            return 0;
        }
        SetMenuSounds(input, 15, 16);
        if (*state).initialized == 0 {
            if !(*state).h_music.is_null() {
                stop_music();
                DestroyMusic((*state).h_music);
                (*state).h_music = ptr::null_mut();
            }
            (*state).h_music = LoadMusic(MELEE_MUSIC.as_ptr().cast()).cast();
            (*state).initialized = 1;
            (*state).melee_option = 4;
            PlayMusic((*state).h_music, 1, 1);
            init_screen();
            FadeScreen(FADE_ALL_TO_COLOR, 840 / 2);
            (*state).last_input_time = GetTimeCounter();
            return 1;
        }
        poll_menu(state, 0);
        if key(input, KEY_MENU_CANCEL) || key(input, KEY_MENU_LEFT) {
            (*state).last_input_time = GetTimeCounter();
            highlight((*state).melee_option, false);
            (*state).melee_option = 10;
            (*state).initialized = 0;
            (*state).side = 0;
            if key(input, KEY_MENU_CANCEL) {
                (*state).row = 0;
                (*state).col = 0;
            } else {
                (*state).row = 1;
                (*state).col = 6;
            }
            edit(state, input);
        } else {
            let mut next = (*state).melee_option;
            let mut force = false;
            if key(input, KEY_MENU_UP) {
                (*state).last_input_time = GetTimeCounter();
                next = super::rules::next_option(next, true, false);
            } else if key(input, KEY_MENU_DOWN) {
                (*state).last_input_time = GetTimeCounter();
                next = super::rules::next_option(next, false, true);
            }
            let controls = KernelOwner::current().read(|owner| owner.player_control);
            if controls[0] & controls[1] & PSYTRON_CONTROL != 0
                && GetTimeCounter().wrapping_sub((*state).last_input_time) > 840 * 10
            {
                force = true;
                next = 4;
            }
            if next != (*state).melee_option {
                if (*state).melee_option == 1 || (*state).melee_option == 7 {
                    status_player(-1);
                }
                highlight((*state).melee_option, false);
                (*state).melee_option = next;
                highlight(next, true);
                if next == 1 || next == 7 {
                    let side = usize::from(next == 1);
                    let control = KernelOwner::current().read(|owner| owner.player_control[side]);
                    status_player(super::rules::network_status_side(side, control));
                }
            }
            if key(input, KEY_MENU_SELECT) || force {
                select_option(input, state);
                let activity = crate::engine::World::current().read(|world| world.activity());
                if activity & CHECK_ABORT as u16 != 0 {
                    return 0;
                }
            }
        }
        // SAFETY: DoMelee owns the menu and its session on the game thread.
        unsafe { (*state).netplay.flush_packet_queues() };
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
                (**KernelOwner::current()).player_control[side] = byte as u8;
                if MeleeSetup_deserializeTeam((*state).melee_setup, side, stream.cast()) == -1 {
                    ok = false;
                    break;
                }
                if KernelOwner::current().read(|owner| owner.player_control[side]) & NETWORK_CONTROL
                    != 0
                {
                    (**KernelOwner::current()).player_control[side] = HUMAN_CONTROL | 16;
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
                KernelOwner::current().read(|owner| owner.player_control[side]) as std::ffi::c_char,
                stream.cast(),
            ) == -1
                || MeleeSetup_serializeTeam((*state).melee_setup, side, stream.cast()) == -1
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
    fn new_menu_state(
        kernel: &KernelOwner,
        input: &crate::engine::input::InputSession,
    ) -> MELEE_STATE {
        MELEE_STATE {
            phase: crate::supermelee::netplay::session::MenuPhase::Menu,
            initialized: 0,
            melee_started: 0,
            melee_option: 0,
            side: 0,
            row: 0,
            col: 0,
            melee_setup: ptr::null_mut(),
            // SAFETY: the native load-state record contains integers and nullable pointers.
            load: unsafe { mem::zeroed() },
            current_ship: 0,
            cur_index: 0,
            build_pick_confirmed: 0,
            random_context: ptr::null_mut(),
            last_input_time: 0,
            h_music: ptr::null_mut(),
            netplay: crate::supermelee::netplay::NetplaySession::with_input(input.clone()),
            catalog: crate::ships::master_native::choices(&kernel.master_queue),
        }
    }

    /// # Safety
    /// Run on the initialized game thread with exclusive access to native menu resources.
    pub unsafe fn melee(
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
    ) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        cx.world.reset_game_data();
        let mut state = new_menu_state(cx.kernel, input);
        // SAFETY: state remains in this stack frame through all synchronous callbacks and cleanup.
        cx.with_legacy(|| {
            P_MELEE_STATE = &mut state;
            state.phase = crate::supermelee::netplay::session::MenuPhase::Menu;
            state.melee_setup = MeleeSetup_new();
            state.random_context = new_context();
            let fallback = GetTimeCounter();
            let seed = io
                .observer
                .input_admitted(
                    &crate::engine::observe::InputView::Seed {
                        domain: 1,
                        fallback,
                    },
                    &None,
                )
                .seed
                .unwrap_or(fallback);
            (*state.random_context).reseed(seed);
            state.current_ship = 255;
            state.cur_index = u16::MAX;
            InitMeleeLoadState(&mut state);
            crate::engine::World::current().write(|world| world.set_activity(SUPER_MELEE as u16));
            (**KernelOwner::current()).game_sounds =
                capture_sound(LoadSound(GAME_SOUNDS.as_ptr().cast()));
            crate::supermelee::native::battle_picker::live::BuildPickMeleeFrame();
            MeleeFrame =
                CaptureDrawable(LoadGraphic(MELEE_SCREEN_PMAP_ANIM.as_ptr().cast()).cast());
            crate::supermelee::native::build_picker::live::build_build_pick_frame(&state.catalog);
            KernelOwner::current().write(|kernel| {
                init_space(kernel, WorldOwner::current().read(|world| world.activity()))
            });
            LoadTeamList(&mut state);
            if !load_config(&mut state) {
                (**KernelOwner::current()).player_control[0] = HUMAN_CONTROL | 16;
                Melee_LocalChange_team(&mut state, 0, *state.load.preBuiltList);
                (**KernelOwner::current()).player_control[1] = COMPUTER_CONTROL | 16;
                Melee_LocalChange_team(&mut state, 1, *state.load.preBuiltList.add(1));
            }
            state.side = 0;
            SetMenuSounds(input, 15, 16);
        });
        let state_ptr = &raw mut state;
        cx.run_input(io, true, |cx, _| {
            // SAFETY: state and its UI resources outlive the synchronous input loop.
            unsafe { run_menu_frame(state_ptr, cx, input) }
        });
        // SAFETY: the input session has stopped and no callback retains these resources.
        cx.with_legacy(|| {
            stop_music();
            WaitForSoundEnd(u16::MAX);
            write_config(&mut state);
            DestroyStringTable(ReleaseStringTable(state.load.dirEntries));
            state.load.dirEntries = ptr::null_mut();
            if !state.h_music.is_null() {
                DestroyMusic(state.h_music);
                state.h_music = ptr::null_mut();
            }
            KernelOwner::current().write(uninit_space);
            crate::supermelee::native::battle_picker::live::DestroyPickMeleeFrame();
            DestroyDrawable(ReleaseDrawable(MeleeFrame));
            MeleeFrame = ptr::null_mut();
            crate::supermelee::native::build_picker::live::destroy_build_pick_frame();
            state.netplay.close_all_connections();
            state.netplay.setup_input_delay(0);
            DestroySound(
                release_sound(KernelOwner::current().read(|owner| owner.game_sounds)).cast(),
            );
            (**KernelOwner::current()).game_sounds = ptr::null_mut();
            UninitMeleeLoadState(&mut state);
            delete_context(state.random_context);
            MeleeSetup_delete(state.melee_setup);
            FlushInput(input);
        });
    }
}

#[cfg(test)]
mod rules_tests;
