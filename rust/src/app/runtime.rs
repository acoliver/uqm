use super::{lifecycle, Application};
use crate::mainloop::state_machine::{self, ActivityDecision, BreakAction};
use crate::mainloop::types::{activity_flags, ActivityValue};
use crate::mainloop::MainLoopError;
use std::sync::Arc;
use uqm_engine::battle::{Battle, Cx};
use uqm_engine::observe::{CheckpointView, Io, Observer};
#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

/// Run initialization, the campaign loop and subsystem teardown on the game thread.
pub fn run(
    application: &mut Application,
    arguments: &[String],
    parsed: &crate::mainloop::options::ParsedOptions,
) -> i32 {
    let runtime = application.game.kernel.input_runtime();
    let observer = application.observer().cloned();
    let mut prepared =
        match crate::mainloop::startup::initialize(runtime.input(), parsed, arguments, observer) {
            Ok(prepared) => prepared,
            Err(status) => return status.max(0),
        };

    let observer = application.observer().cloned();
    let result = match run_game(&mut application.game, observer, &mut prepared) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("app::run: fatal error: {error}");
            1
        }
    };
    crate::mainloop::teardown::teardown_subsystems();
    result
}

fn run_game(
    game: &mut uqm_engine::Game,
    observer: Option<Arc<crate::automation::AutomationObserver>>,
    prepared: &mut crate::mainloop::startup::Prepared,
) -> Result<(), MainLoopError> {
    let mut battle = Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_engine::math::random,
    );
    uqm_engine::sound::audio_init::init_audio();
    if crate::game_init::kernel_abi::load_kernel(cx.kernel, cx.world, prepared) == 0 {
        return Err(MainLoopError::LoadKernelFailed);
    }
    cx.world.set_activity(0);
    let runtime = cx.kernel.input_runtime();
    let mut input_observer = runtime.observer();
    let mut io = Io {
        observer: input_observer.as_mut(),
    };
    crate::mainloop::lifecycle_abi::uqm_splash_with_bg_init_kernel(&mut cx, &mut io);
    let mut result = Ok(());
    while {
        let mut menu = crate::mainloop::restart_menu::restart_ops::RestartMenu::default();
        let mut host = RestartServices { prepared };
        let started = crate::mainloop::restart_menu::orchestration::start_game_impl(
            &mut menu.operations(&mut cx, &mut io, &mut host),
        );
        started
    } {
        if let Err(error) = run_campaign(&mut cx, &mut io, observer.as_ref()) {
            result = Err(error);
            break;
        }
    }
    crate::game_init::cleanup::uninit_game_kernel(&mut cx);
    uqm_content::ships::master_native::engine::free(cx.world, cx.kernel);
    crate::game_init::cleanup::free_kernel(cx.kernel);
    uqm_engine::log_abi::log_show_box(false, false);
    result
}

struct RestartServices<'a> {
    prepared: &'a mut crate::mainloop::startup::Prepared,
}

impl crate::mainloop::restart_menu::restart_ops::RestartHost for RestartServices<'_> {
    fn melee(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) {
        let config = self.prepared.config.clone();
        let teams = self.prepared.teams.clone();
        uqm_modes::supermelee::native::menu::live::melee(cx, io, config, teams);
    }
    fn setup(
        &mut self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        context: uqm_engine::graphics::context::ContextLease,
        font: uqm_engine::graphics::font::FontLease,
    ) {
        use uqm_modes::front_end::setupmenu::{setup_menu, SetupConfiguration, SetupResources};
        let runtime = uqm_engine::configuration::runtime::RuntimeSettings::capture(&context);
        let resources = SetupResources::load(context, font);
        let configuration = SetupConfiguration {
            input: cx.kernel.input_runtime().input().clone(),
            runtime,
            settings: &mut self.prepared.settings,
            bindings: &mut self.prepared.bindings,
            directory: self.prepared.config.clone(),
            names: uqm_engine::input_port::templates::input_templates(),
        };
        setup_menu(cx, io, configuration, resources);
    }
    fn credits(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, victory: bool) {
        uqm_modes::front_end::credits::credits(cx, io, victory);
    }
    fn free_game_data(&mut self, cx: &mut Cx<'_>) {
        crate::game_init::cleanup::free_game_data(cx);
    }
}

fn initialize_campaign(cx: &mut Cx<'_>) -> Result<(), MainLoopError> {
    cx.kernel.install_player_inputs();
    if let Err(error) = uqm_modes::navigation::globals::init_game_structures(cx) {
        cx.kernel.clear_player_inputs();
        return Err(error.into());
    }
    crate::time::clock_bridge::rust_clock_init(cx.world);
    let mut random = cx.rng;
    let mut services = uqm_modes::navigation::events::EventServices {
        geometry: uqm_modes::navigation::hyper_units::Geometry::new(cx.screen_dimensions()),
        random: &mut random,
    };
    uqm_modes::navigation::events::schedule_initial_events(cx.world, &mut services);
    Ok(())
}

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

fn run_campaign(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    observer: Option<&Arc<crate::automation::AutomationObserver>>,
) -> Result<(), MainLoopError> {
    initialize_campaign(cx)?;
    activate_start_scene(cx, observer);
    loop {
        cx.kernel.set_status_message_mode(1);
        let decision = lifecycle::prepare_dispatch(cx.world);
        dispatch(cx, io, decision);
        lifecycle::complete_encounter(cx.world, decision);
        uqm_modes::ui::sis::clear_flash(cx.kernel);
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
                uqm_modes::comm::dispatch::init_communication(cx, io, 21);
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
    uqm_engine::sound::heart_ffi::stop_sound();
    crate::time::clock_bridge::rust_clock_uninit(cx.world);
    uqm_modes::navigation::globals::uninit_game_structures(cx);
    cx.kernel.clear_player_inputs();
    Ok(())
}

fn dispatch(cx: &mut Cx<'_>, io: &mut Io<'_>, decision: ActivityDecision) {
    match decision {
        ActivityDecision::InstallBombAtEarth => {
            uqm_modes::ui::starbase::install_bomb_at_earth(cx);
        }
        ActivityDecision::VisitStarBase => {
            uqm_modes::ui::starbase::run_starbase_visit(
                cx,
                io,
                &uqm_modes::ui::MenuServices {
                    communicate: |cx, io, conversation, options| {
                        uqm_modes::comm::dispatch::init_communication_with_options(
                            cx,
                            io,
                            conversation,
                            options,
                        )
                    },
                    game_options: |cx, io| {
                        uqm_modes::front_end::gameopt::game_options(cx, io, None)
                    },
                    ship_spin: uqm_modes::front_end::fmv::do_ship_spin,
                },
            );
        }
        ActivityDecision::RaceCommunication => {
            uqm_modes::comm::dispatch::race_communication(cx, io);
        }
        ActivityDecision::ExploreSolarSystem | ActivityDecision::Battle => {
            cx.kernel.pilot_flash.borrow_mut().reset();
            let rate = if decision == ActivityDecision::ExploreSolarSystem {
                30
            } else {
                5
            };
            crate::time::clock_bridge::rust_clock_set_rate(cx.world, rate);
            if decision == ActivityDecision::ExploreSolarSystem {
                uqm_modes::planets::solarsys::lifecycle::run_explore_solar_sys(cx, io);
            } else {
                crate::mainloop::lifecycle_abi::battle_with_frame_callback(cx, io);
            }
        }
    }
}
