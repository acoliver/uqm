#[cfg(not(test))]
use super::{lifecycle, Application};
#[cfg(not(test))]
use crate::engine::battle::{Battle, Cx};
#[cfg(not(test))]
use crate::engine::observe::{CheckpointView, Io, Observer};
#[cfg(not(test))]
use crate::mainloop::state_machine::{self, ActivityDecision, BreakAction};
#[cfg(not(test))]
use crate::mainloop::types::{activity_flags, ActivityValue};
#[cfg(not(test))]
use crate::mainloop::MainLoopError;
#[cfg(not(test))]
use std::ffi::{c_char, c_int, CString};
#[cfg(not(test))]
use std::sync::Arc;

/// Run initialization, the campaign loop and subsystem teardown on the game thread.
#[cfg(not(test))]
pub fn run(
    application: &mut Application,
    arguments: &[CString],
    parsed: &crate::mainloop::options::ParsedOptions,
) -> c_int {
    let mut argv: Vec<*mut c_char> = arguments
        .iter()
        .map(|arg| arg.as_ptr().cast_mut())
        .collect();
    argv.push(std::ptr::null_mut());
    let runtime = application.game.kernel.input_runtime();
    let observer = application.observer().cloned();
    // SAFETY: argv remains live while the application suspends Game for initialization.
    let initialized = unsafe {
        application.legacy(|| {
            crate::mainloop::startup::initialize(
                runtime.input(),
                parsed,
                (argv.len() - 1) as c_int,
                argv.as_mut_ptr(),
                observer,
            )
        })
    };
    if initialized != 0 {
        return initialized.max(0);
    }

    let observer = application.observer().cloned();
    let result = match run_game(&mut application.game, observer) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("app::run: fatal error: {error}");
            1
        }
    };
    // SAFETY: the campaign has stopped, and subsystem teardown returns before Game is released.
    unsafe { application.legacy(crate::mainloop::teardown::teardown_subsystems) };
    result
}

#[cfg(not(test))]
fn run_game(
    game: &mut crate::engine::Game,
    observer: Option<Arc<crate::automation::AutomationObserver>>,
) -> Result<(), MainLoopError> {
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    );
    cx.with_legacy(|| {
        // SAFETY: startup initialized the audio options before the game kernel is loaded.
        unsafe {
            crate::sound::audio_init::initAudio(
                crate::sound::volume_globals::snddriver,
                crate::sound::volume_globals::soundflags,
            );
        }
    });
    // SAFETY: the application retains both owners; argc zero permits a null argv.
    if unsafe {
        crate::game_init::kernel_abi::LoadKernel(cx.kernel, cx.world, 0, std::ptr::null_mut())
    } == 0
    {
        return Err(MainLoopError::LoadKernelFailed);
    }
    cx.world.set_activity(0);
    let runtime = cx.kernel.input_runtime();
    let mut input_observer = runtime.observer();
    let mut io = Io {
        observer: input_observer.as_mut(),
    };
    // SAFETY: the kernel and native input runtime are initialized on their owner thread.
    unsafe { crate::mainloop::lifecycle_abi::uqm_splash_with_bg_init_kernel(&mut cx, &mut io) };
    while crate::mainloop::restart_menu::orchestration::start_game_impl(
        &crate::mainloop::restart_menu::restart_ops::CffiOps::new(&mut cx, &mut io),
    ) {
        run_campaign(&mut cx, &mut io, observer.as_ref());
    }
    // SAFETY: all campaign modes and their borrowed resources have returned.
    unsafe {
        crate::game_init::cleanup::uninit_game_kernel(&mut cx);
        crate::ships::master_native::engine::free(cx.world, cx.kernel);
        crate::game_init::cleanup::free_kernel(cx.kernel);
    }
    cx.with_legacy(|| crate::log_abi::log_showBox(false, false));
    Ok(())
}

#[cfg(not(test))]
fn initialize_campaign(cx: &mut Cx<'_>) {
    cx.kernel.install_player_inputs();
    // SAFETY: the application retains initialized kernel resources for campaign setup.
    unsafe { crate::navigation::globals::init_game_structures(cx) };
    crate::time::clock_bridge::rust_clock_init(cx.world);
    let mut random = cx.rng;
    let mut services = crate::navigation::events::EventServices {
        geometry: crate::navigation::hyper_units::Geometry::new(cx.screen_dimensions()),
        random: &mut random,
    };
    crate::navigation::events::schedule_initial_events(cx.world, &mut services);
}

#[cfg(not(test))]
fn activate_start_scene(
    cx: &mut Cx<'_>,
    observer: Option<&Arc<crate::automation::AutomationObserver>>,
) {
    let Some(observer) = observer else {
        return;
    };
    observer.activate_save_capture();
    let Some(plan) = observer.prepare_start_scene(cx) else {
        return;
    };
    for &(key, value) in plan.game_state {
        cx.world.set_story_state(key, value);
    }
    let screen_context = cx.kernel.screen_context;
    crate::automation::scenario::enter_scene(
        &plan,
        &mut crate::automation::scenario::LinkedSceneEntry {
            world: cx.world,
            kernel: cx.kernel,
            screen_context,
        },
    );
    observer.finish_start_scene(&plan);
}

#[cfg(not(test))]
fn run_campaign(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    observer: Option<&Arc<crate::automation::AutomationObserver>>,
) {
    initialize_campaign(cx);
    activate_start_scene(cx, observer);
    loop {
        cx.kernel.set_status_message_mode(1);
        let decision = lifecycle::prepare_dispatch(cx.world);
        dispatch(cx, io, decision);
        lifecycle::complete_encounter(cx.world, decision);
        crate::ui::sis::clear_flash(cx.kernel);
        let action = lifecycle::finish_dispatch(cx.world, cx.kernel);
        if action != BreakAction::Continue && cx.world.data.flagship.crew_enlisted == u16::MAX {
            if let Some(observer) = observer {
                crate::automation::observer::Handle(observer.clone())
                    .checkpoint(&CheckpointView::FlagshipDestroyed);
            }
        }
        match action {
            BreakAction::Continue => {}
            BreakAction::InitBlackUrqCommunication => {
                // SAFETY: the retained owners keep communication resources live through the mode.
                unsafe { crate::comm::dispatch::init_communication(cx, io, 21) };
                break;
            }
            BreakAction::ClearRestart => {
                cx.world
                    .set_activity(cx.world.activity() & !activity_flags::CHECK_RESTART);
                break;
            }
            BreakAction::JustBreak => break,
        }
        if !state_machine::should_continue(ActivityValue::new(cx.world.activity())) {
            break;
        }
    }
    cx.with_legacy(crate::sound::heart_ffi::stop_sound);
    crate::time::clock_bridge::rust_clock_uninit(cx.world);
    // SAFETY: the active mode has returned before its campaign resources are released.
    unsafe { crate::navigation::globals::uninit_game_structures(cx) };
    cx.kernel.clear_player_inputs();
}

#[cfg(not(test))]
fn dispatch(cx: &mut Cx<'_>, io: &mut Io<'_>, decision: ActivityDecision) {
    match decision {
        ActivityDecision::InstallBombAtEarth => {
            crate::ui::starbase::install_bomb_at_earth(cx);
        }
        ActivityDecision::VisitStarBase => {
            // SAFETY: the campaign and display remain live through the starbase mode.
            unsafe {
                crate::ui::starbase::run_starbase_visit(
                    cx,
                    io,
                    &crate::ui::MenuServices {
                        communicate: |cx, io, conversation, options| {
                            crate::comm::dispatch::init_communication_with_options(
                                cx,
                                io,
                                conversation,
                                options,
                            )
                        },
                        game_options: |cx, io| {
                            crate::front_end::gameopt::game_options(cx, io, None)
                        },
                        ship_spin: |cx, _io, index, music| {
                            let music =
                                music.map_or(std::ptr::null_mut(), |music| music.as_raw().cast());
                            // SAFETY: the supplied music and campaign remain retained during the presentation.
                            cx.with_legacy(|| {
                                crate::front_end::fmv::runtime::DoShipSpin(index, music)
                            });
                        },
                    },
                )
            };
        }
        ActivityDecision::RaceCommunication => {
            // SAFETY: the campaign retains the communication mode's owners and resources.
            unsafe { crate::comm::dispatch::race_communication(cx, io) };
        }
        ActivityDecision::ExploreSolarSystem | ActivityDecision::Battle => {
            cx.kernel.pilot_flash.borrow_mut().reset();
            let rate = if decision == ActivityDecision::ExploreSolarSystem {
                30
            } else {
                5
            };
            crate::time::clock_bridge::rust_clock_set_rate(cx.world, rate);
            // SAFETY: the application retains both owners and the observer through the mode.
            unsafe {
                if decision == ActivityDecision::ExploreSolarSystem {
                    crate::planets::solarsys::lifecycle::run_explore_solar_sys(cx, io);
                } else {
                    crate::mainloop::lifecycle_abi::battle_with_frame_callback(cx, io);
                }
            }
        }
    }
}
