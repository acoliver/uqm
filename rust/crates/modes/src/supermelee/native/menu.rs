//! Super Melee setup lifecycle and menu dispatch.
pub(super) mod rules;
pub mod live {
    use crate::battle::{COMPUTER_CONTROL, HUMAN_CONTROL, NETWORK_CONTROL, PSYTRON_CONTROL};
    use crate::starbase::ids::MELEE_STRING_BASE;
    use crate::supermelee::native::{MELEE_MUSIC, MELEE_SCREEN_PMAP_ANIM};
    use crate::supermelee::session::MeleeSession;
    use uqm_engine::graphics::{context::ContextLease, drawable::LoadedDrawable};
    use uqm_engine::input_port::frames::{
        KEY_MENU_CANCEL, KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_SELECT, KEY_MENU_UP,
    };
    use uqm_engine::sound::heart_ffi::fade_music as FadeMusic;
    use uqm_engine::sound::heart_ffi::wait_for_sound_end;
    use uqm_engine::time::counter_abi::get_time_counter;
    use uqm_engine::world::activity::activity_flags::CHECK_ABORT;
    use uqm_engine::world::activity::activity_kinds::SUPER_MELEE;
    use uqm_engine::Kernel as KernelOwner;

    pub(crate) mod admission;
    mod boundary;
    use boundary::*;
    const FADE_ALL_TO_COLOR: u32 = 253;
    const FADE_ALL_TO_BLACK: u32 = 252;
    use crate::supermelee::native::load_teams::live::*;
    use std::ffi::CString;
    mod battle;
    pub(crate) mod changes;
    pub(crate) mod drawing;
    mod editing;
    use battle::*;
    use changes::{local_change_ship, local_change_team, local_change_team_name, view_name};
    use drawing::{
        connection_box, controls, current, draw_ship_strings, flash_selection, highlight,
        init_screen, name, redraw, repair_frame, ship_box, slot_rect, status, status_game,
        status_player, worth,
    };
    use editing::*;
    pub(crate) fn screen_context(kernel: &KernelOwner) -> ContextLease {
        ContextLease::retain(
            kernel
                .screen_context
                .unwrap_or_else(|| panic!("melee screen context")),
        )
    }

    pub(crate) fn fade(cx: &mut uqm_engine::battle::Cx<'_>, kind: u32) {
        let context = screen_context(cx.kernel);
        let deadline = context.fade_screen(kind, 840 / 2).wrapping_add(840 / 60);
        cx.kernel.input_runtime().sleep_until(cx, deadline);
        context.finish_color_transforms();
    }

    fn load_menu_graphics(
        content: &uqm_engine::resource::Content,
        state: &mut MeleeSession,
        offscreen: ContextLease,
    ) {
        state.menu_graphic = Some(
            LoadedDrawable::load(
                content,
                std::ffi::CStr::from_bytes_with_nul(MELEE_SCREEN_PMAP_ANIM)
                    .unwrap_or_else(|error| panic!("menu graphic resource: {error}")),
            )
            .unwrap_or_else(|error| panic!("melee menu graphic: {error}")),
        );
        state.build_picker = Some(
            crate::supermelee::native::build_picker::live::build_build_pick_frame(
                offscreen,
                state
                    .menu_context
                    .as_ref()
                    .unwrap_or_else(|| panic!("menu context not loaded"))
                    .clone(),
                state.menu_frames().context_frame(27),
                &state.catalog,
            ),
        );
    }

    fn load_setup(state: &mut MeleeSession, cx: &mut uqm_engine::battle::Cx<'_>) {
        if !state.load_config(&mut cx.kernel.player_control) {
            cx.kernel.player_control[0] = HUMAN_CONTROL | 16;
            let team = state.load.builtins[0].clone();
            local_change_team(cx, state, 0, &team);
            cx.kernel.player_control[1] = COMPUTER_CONTROL | 16;
            let team = state.load.builtins[1].clone();
            local_change_team(cx, state, 1, &team);
        }
    }

    fn change_option(state: &mut MeleeSession, cx: &mut uqm_engine::battle::Cx<'_>, next: u8) {
        if state.melee_option == 1 || state.melee_option == 7 {
            status_player(cx, state, -1);
        }
        highlight(cx, state, state.melee_option, false);
        state.melee_option = next;
        highlight(cx, state, next, true);
        if next == 1 || next == 7 {
            let side = usize::from(next == 1);
            let control = cx.kernel.player_control[side];
            status_player(cx, state, super::rules::network_status_side(side, control));
        }
    }

    fn run_menu_frame(
        state: &mut MeleeSession,
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
        input: &uqm_engine::input::InputSession,
    ) -> bool {
        use crate::supermelee::session::MenuPhase;
        (match state.phase {
            MenuPhase::Menu => do_melee(state, cx, io, input),
            MenuPhase::Confirm => confirm(input, state, cx, io),
            MenuPhase::Connecting => connecting(state, cx, io, input),
            MenuPhase::Editing => edit(state, cx, io, input),
            MenuPhase::Loading => do_load_team(state, cx, input),
            MenuPhase::Building => panic!("build picker escaped its nested input loop"),
        }) != 0
    }
    pub(super) fn key(input: &uqm_engine::input::InputSession, key: u32) -> bool {
        input.pulsed().menu[key as usize] != 0
    }
    fn select_option(
        input: &uqm_engine::input::InputSession,
        state: &mut MeleeSession,
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
    ) {
        match state.melee_option {
            4 => start_button(state, cx),
            3 | 5 => {
                state.initialized = 0;
                state.side = u16::from(state.melee_option != 3);
                do_load_team(state, cx, input);
            }
            2 | 6 => {
                state.side = u16::from(state.melee_option != 2);
                if state.melee_setup.fleet_value[state.side as usize] > 0 {
                    let screen = ContextLease::retain(
                        cx.kernel
                            .screen_context
                            .unwrap_or_else(|| panic!("melee save context")),
                    );
                    do_save_team(state, &screen, cx, io);
                } else {
                    crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
                }
            }
            9 => cx.world.set_activity(cx.world.activity() | CHECK_ABORT),
            0 | 8 => {
                let side = u16::from(state.melee_option == 0);
                let confirmed = crate::front_end::cnctdlg::melee_connect_dialog(
                    cx,
                    io,
                    input,
                    &mut state.netplay,
                    usize::from(side),
                );
                redraw(cx, state);
                state.last_input_time = get_time_counter();
                if confirmed {
                    state.initialized = 0;
                    state.phase = crate::supermelee::session::MenuPhase::Connecting;
                }
            }
            1 | 7 => next_control(state, cx, io, usize::from(state.melee_option == 1)),
            _ => {}
        }
    }
    /// State and its graphics remain live through this synchronous menu frame.
    pub fn do_melee(
        state: &mut MeleeSession,
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
        input: &uqm_engine::input::InputSession,
    ) -> u32 {
        let time = get_time_counter();
        input.set_paused(false);
        if cx.world.activity() & CHECK_ABORT != 0 {
            return 0;
        }
        input.set_menu_sounds(15, 16);
        if state.initialized == 0 {
            if state.h_music.is_some() {
                crate::ui::settings::stop_loaded_music(state.h_music.as_ref());
                drop(state.h_music.take());
            }
            let music_name = std::ffi::CStr::from_bytes_with_nul(MELEE_MUSIC)
                .unwrap_or_else(|error| panic!("melee music resource name: {error}"));
            state.h_music = uqm_engine::sound::resource_types::LoadedMusic::load(
                &cx.kernel.content(),
                music_name,
            )
            .ok();
            state.initialized = 1;
            state.melee_option = 4;
            crate::ui::settings::play_loaded_music(cx.world, state.h_music.as_ref(), 1, 1);
            {
                init_screen(cx, state);
                screen_context(cx.kernel).fade_screen(FADE_ALL_TO_COLOR, 840 / 2);
            };
            state.last_input_time = get_time_counter();
            return 1;
        }
        poll_menu(state, cx, io, 0);
        if key(input, KEY_MENU_CANCEL) || key(input, KEY_MENU_LEFT) {
            state.last_input_time = get_time_counter();
            highlight(cx, state, state.melee_option, false);
            state.melee_option = 10;
            state.initialized = 0;
            state.side = 0;
            if key(input, KEY_MENU_CANCEL) {
                state.row = 0;
                state.col = 0;
            } else {
                state.row = 1;
                state.col = 6;
            }
            edit(state, cx, io, input);
        } else {
            let mut next = state.melee_option;
            let mut force = false;
            if key(input, KEY_MENU_UP) {
                state.last_input_time = get_time_counter();
                next = super::rules::next_option(next, true, false);
            } else if key(input, KEY_MENU_DOWN) {
                state.last_input_time = get_time_counter();
                next = super::rules::next_option(next, false, true);
            }
            let controls = cx.kernel.player_control;
            if controls[0] & controls[1] & PSYTRON_CONTROL != 0
                && get_time_counter().wrapping_sub(state.last_input_time) > 840 * 10
            {
                force = true;
                next = 4;
            }
            if next != state.melee_option {
                change_option(state, cx, next);
            }
            if key(input, KEY_MENU_SELECT) || force {
                select_option(input, state, cx, io);
                if cx.world.activity() & CHECK_ABORT != 0 {
                    return 0;
                }
            }
        }
        flush_menu(state, cx, io);
        disconnects(state, cx);
        {
            flash_selection(cx, state);
            cx.kernel
                .input_runtime()
                .sleep_until(cx, time.wrapping_add(840 / 30));
        };
        1
    }
    fn admit_menu_rendering(state: &mut MeleeSession, kernel: &KernelOwner) -> ContextLease {
        let offscreen = ContextLease::retain(
            kernel
                .off_screen_context
                .unwrap_or_else(|| panic!("melee offscreen context")),
        );
        state.menu_context = Some(ContextLease::retain(
            kernel
                .space_context
                .unwrap_or_else(|| panic!("melee drawing context")),
        ));
        state.menu_target = Some(uqm_engine::graphics::drawable::ContextFrame::retain(
            kernel
                .screen
                .non_null()
                .unwrap_or_else(|| panic!("melee screen frame")),
        ));
        state.menu_font = Some(uqm_engine::graphics::font::FontLease::retain(
            kernel
                .micro_font
                .unwrap_or_else(|| panic!("melee menu font")),
        ));
        offscreen
    }
    /// Run on the initialized game thread with exclusive access to native menu resources.
    pub fn melee(
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
        config: uqm_engine::io::Dir,
        teams: uqm_engine::io::Dir,
    ) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        cx.world.reset_game_data();
        let mut state = MeleeSession::new(
            cx.world.activity_control(),
            uqm_content::ships::master_native::choices(&cx.kernel.master_queue),
        );
        state.files = Some(crate::supermelee::session::MeleeFiles { config, teams });
        let offscreen = admit_menu_rendering(&mut state, cx.kernel);
        state.phase = crate::supermelee::session::MenuPhase::Menu;
        let fallback = get_time_counter();
        // Seed admission is part of the recorded input sequence.
        io.observer.input_admitted(
            &uqm_engine::observe::InputView::Seed {
                domain: 1,
                fallback,
            },
            cx,
        );
        state.current_ship = 255;
        state.cur_index = u16::MAX;
        for (index, team) in state.load.builtins.iter_mut().take(5).enumerate() {
            team.rename(cx.kernel.game_text(MELEE_STRING_BASE as usize + 4 + index));
        }
        cx.world.set_activity(SUPER_MELEE);
        cx.kernel.game_sounds = uqm_engine::sound::owned_bank::LoadedSoundBank::load(
            &cx.kernel.content(),
            c"sounds.battle",
        )
        .ok();
        state.fleet_picker = Some(
            crate::supermelee::native::battle_picker::live::FleetPicker::new(
                &cx.kernel.content(),
                &offscreen,
            ),
        );
        load_menu_graphics(&cx.kernel.content(), &mut state, offscreen);
        cx.kernel.init_space(cx.world.activity());
        load_team_list(&mut state);
        load_setup(&mut state, cx);
        state.side = 0;
        input.set_menu_sounds(15, 16);
        {
            let mut frame = admission::SessionFrame::new(
                &mut state,
                |state: &mut MeleeSession,
                 cx: &mut uqm_engine::battle::Cx<'_>,
                 io: &mut uqm_engine::observe::Io<'_>| {
                    {
                        run_menu_frame(state, cx, io, input)
                    }
                },
            );
            cx.run_input_frame(io, true, &mut frame);
        }
        {
            crate::ui::settings::stop_loaded_music(state.h_music.as_ref());
            wait_for_sound_end(u16::MAX);
        };
        state.save_config(cx.kernel.player_control);
        let session = state.netplay.clone();
        session.close_all_connections(&mut |event| {
            changes::apply_protocol_event(&mut state, cx, io, event)
        });
        drop(state.h_music.take());
        cx.kernel.uninit_space();
        cx.kernel.game_sounds = None;
        {
            drop(state.fleet_picker.take());
            drop(state.menu_graphic.take());
            drop(state.build_picker.take());
            state.netplay.setup_input_delay(0);
        };
        runtime.flush();
    }
}

#[cfg(test)]
mod rules_tests;
