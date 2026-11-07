//! Owner-supplied PlanetSide session boundary.

use crate::engine::observe::InputView;
use std::ffi::c_void;

pub const STATUS_OK: i32 = 0;
pub const STATUS_INVALID_ARGUMENT: i32 = -4;
const DETAIL_RETURNED: u32 = 1;
const DETAIL_DESTROYED: u32 = 2;
const DETAIL_ABORTED: u32 = 3;
const DETAIL_ADAPTER_FAILURE: u32 = 4;
const DETAIL_FRAME_BUDGET: u32 = 5;

/// Borrowed resources and trip parameters supplied by the orbit owner.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanetSideRunContext {
    pub solar_system: *mut c_void,
    pub world: *mut c_void,
    pub misc_data_frame: *mut c_void,
    pub energy_frame: *mut c_void,
    pub life_frames: [*mut c_void; 3],
    pub landing_x: i32,
    pub landing_y: i32,
    pub facing: u8,
    pub _padding: [u8; 3],
    pub retrieval_masks: [u32; 3],
    pub tick_period: u32,
    pub frame_budget: u32,
    /// Live planet tectonics rating (0-7) from `PLANET_INFO.Tectonics`.
    pub tectonics_rating: u8,
    /// Live planet weather rating (0-7) from `PLANET_INFO.Weather`.
    pub weather_rating: u8,
    /// Live planet surface temperature from `PLANET_INFO.SurfaceTemperature`.
    pub temperature: i32,
}

/// Fixed-size reply. No Rust-owned pointer escapes through this record.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanetSideReply {
    pub status: i32,
    pub detail: u32,
    pub value0: i64,
    pub value1: i64,
}

use crate::graphics::pixmap_abi::GetFrameCount;

use super::adapters::{CffiPlanetSideClock, CffiPlanetSideInput, CffiShipStatus, ObservedClock};
use super::assembly::{assemble_surface, share_surface};
use super::assets::CffiPlanetSideAudio;
use super::collision_adapter::{CffiGameplayRandom, SurfaceCollisionAdapter};
use super::controller::PlanetSideController;
use super::generation::ScanPersistence;
use super::generation_adapter::CffiSurfaceGenerator;
use super::graphics_adapter::CffiPlanetSideGraphics;
use super::mask_adapter::extract_lander_masks;
use super::model::SurfacePoint;
use super::resources::{LanderGraphic, PlanetSideAssetAccess};
use super::runtime::{PlanetSideClock, RuntimeAdapters, Tick};
use super::session_factory::create_production_session;
use super::telemetry::TripTelemetry;
use super::visual_adapter::CffiSurfaceVisuals;
use crate::engine::observe::CheckpointView;

const fn failure(status: i32) -> PlanetSideReply {
    PlanetSideReply {
        status,
        detail: 0,
        value0: 0,
        value1: 0,
    }
}

/// # Safety
/// Native handles in context must remain live throughout the synchronous lander session.
pub unsafe fn run_session(
    controls: &crate::engine::input::InputSession,
    activity: crate::engine::world::activity::ActivityControl,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    context: &mut PlanetSideRunContext,
) -> PlanetSideReply {
    if !valid_context(context) {
        return failure(STATUS_INVALID_ARGUMENT);
    }

    let game = std::cell::RefCell::new(crate::engine::battle::Cx {
        b: cx.b,
        world: cx.world,
        kernel: cx.kernel,
        rng: cx.rng,
    });
    let observe = std::cell::RefCell::new(crate::engine::observe::Io {
        observer: &mut *io.observer,
    });
    let (trip, fixture) = admit_trip(&observe);
    let result = run_retained_session(controls, activity, &game, &observe, context, &trip, fixture);
    if matches!(result, Ok(Ok(_))) {
        // SAFETY: the outer ABI contract keeps the current solar record live through return.
        context.retrieval_masks = unsafe {
            (*context
                .solar_system
                .cast::<crate::c_bindings::global_state::SOLARSYS_STATE>())
            .SysInfo
            .planet_info
            .scan_retrieve_mask
        };
    }
    let reply = session_reply(&result);
    complete_trip(&trip, &observe, &result);
    reply
}

fn admit_trip(
    observe: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
) -> (
    std::rc::Rc<std::cell::RefCell<TripTelemetry>>,
    Option<crate::engine::observe_port::planet_side::FixtureLayout>,
) {
    let admission = observe
        .borrow_mut()
        .observer
        .input_admitted(&InputView::PlanetSideBegin, &None)
        .planet_side;
    let trip = std::rc::Rc::new(std::cell::RefCell::new(TripTelemetry::pending(
        admission.map_or(0, |admission| admission.generation),
    )));
    (trip, admission.and_then(|admission| admission.fixture))
}

fn complete_trip(
    trip: &std::cell::RefCell<TripTelemetry>,
    observe: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
    result: &Result<
        Result<super::session::SessionOutcome, super::controller::ControllerError>,
        super::runtime::AdapterError,
    >,
) {
    {
        let mut trip = trip.borrow_mut();
        match &result {
            Ok(Ok(outcome)) => trip.finish(outcome),
            Ok(Err(super::controller::ControllerError::FrameBudgetExceeded)) => {
                trip.frame_budget_failure()
            }
            Ok(Err(super::controller::ControllerError::Runtime(
                super::runtime::RuntimeError::Adapter(error),
            )))
            | Err(error) => {
                trip.batch_depth_corrected(error.batch_depth_corrections);
                trip.adapter_failure(error.operation);
            }
        }
        observe
            .borrow_mut()
            .observer
            .checkpoint(&CheckpointView::PlanetSide(trip.snapshot()));
    }
}

fn run_retained_session(
    controls: &crate::engine::input::InputSession,
    activity: crate::engine::world::activity::ActivityControl,
    game: &std::cell::RefCell<crate::engine::battle::Cx<'_>>,
    observe: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
    context: &mut PlanetSideRunContext,
    telemetry: &std::rc::Rc<std::cell::RefCell<TripTelemetry>>,
    fixture: Option<crate::engine::observe_port::planet_side::FixtureLayout>,
) -> Result<
    Result<super::session::SessionOutcome, super::controller::ControllerError>,
    super::runtime::AdapterError,
> {
    let assets = game
        .borrow_mut()
        .with_legacy(super::init_lander::borrowed_assets)?;
    let mut generator =
        CffiSurfaceGenerator::for_planet_side(game, observe, context.solar_system, context.world)?;
    let mut visuals = CffiSurfaceVisuals::new(
        context.misc_data_frame,
        context.energy_frame,
        context.life_frames,
    )?;
    let persistence = ScanPersistence::from_masks(context.retrieval_masks);
    let lander_masks = unsafe { extract_lander_masks(assets.graphic(LanderGraphic::Lander))? };
    let assembly = assemble_surface(&mut generator, persistence, &mut visuals, lander_masks)?;
    let surface = share_surface(assembly);
    let mut ship = CffiShipStatus { game };
    let session = create_trip_session(&mut ship, context)?;
    let mut world_visuals = super::visual_adapter::CffiWorldVisuals::new(&assets, &mut visuals);
    let channels = game.borrow().kernel.channels.clone();
    install_fixture(fixture, &session, &surface, &mut world_visuals)?;
    let (earthquake_frame_count, lava_frame_count) = hazard_frame_counts(&assets);
    let publish = |trip: &TripTelemetry| {
        observe
            .borrow_mut()
            .observer
            .checkpoint(&CheckpointView::PlanetSide(trip.snapshot()))
    };
    let collision = SurfaceCollisionAdapter {
        telemetry: telemetry.clone(),
        publish,
        surface: surface.clone(),
        random: CffiGameplayRandom,
        generator,
        persistence,
        world_visuals,
        earthquake_frame_count,
        lava_frame_count,
    };
    // The orbit/scan menu underneath stays resident, so silence its
    // navigation sounds for the trip. Restored on every exit path by Drop.
    let _menu_silence = super::menu_sounds::MenuSoundSilence::acquire(controls);
    telemetry.borrow_mut().begin(&session);
    publish(&telemetry.borrow());
    let adapters = RuntimeAdapters {
        input: CffiPlanetSideInput::new(controls.controls().clone(), activity),
        collision,
        graphics: CffiPlanetSideGraphics {
            game,
            surface: surface.clone(),
            assets: &assets,
            last_scan_position: None,
        },
        audio: CffiPlanetSideAudio::new(assets.sounds(), channels, telemetry.clone()),
        clock: ObservedClock {
            clock: CffiPlanetSideClock,
            telemetry: telemetry.clone(),
            io: observe,
        },
        ship,
    };
    let mut controller =
        PlanetSideController::new(session, adapters, context.tick_period, context.frame_budget);
    let outcome = controller.run_with_frame(|session, adapters, tick_period| {
        publish(&telemetry.borrow());
        let (deadline, input) = prepare_frame(
            game,
            observe,
            session,
            &mut adapters.input,
            &adapters.clock,
            tick_period,
        )?;
        let step = super::runtime::run_frame_at(game, session, adapters, deadline, input)?;
        telemetry.borrow_mut().frame(session);
        Ok(step)
    });
    settle_session(
        &controller.session,
        &mut controller.adapters.ship,
        outcome.is_ok(),
    )?;
    Ok(outcome)
}

fn hazard_frame_counts(assets: &impl PlanetSideAssetAccess) -> (u16, u16) {
    // SAFETY: the trip retains both hazard frames throughout this metadata read.
    unsafe {
        (
            GetFrameCount(assets.graphic(LanderGraphic::Earthquake).cast()),
            GetFrameCount(assets.graphic(LanderGraphic::Lava).cast()),
        )
    }
}

fn prepare_frame(
    game: &std::cell::RefCell<crate::engine::battle::Cx<'_>>,
    observe: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
    session: &super::session::PlanetSideSession,
    input: &mut CffiPlanetSideInput,
    clock: &impl PlanetSideClock,
    tick_period: u32,
) -> Result<(Tick, Option<super::simulation::FrameInput>), super::runtime::AdapterError> {
    let deadline = Tick(clock.now().0.wrapping_add(tick_period));
    let input = if session.phase == super::session::SessionPhase::Active {
        Some(input.poll_observed(&mut game.borrow_mut(), &mut observe.borrow_mut())?)
    } else {
        None
    };
    Ok((deadline, input))
}

fn create_trip_session(
    ship: &mut CffiShipStatus<'_, '_>,
    context: &PlanetSideRunContext,
) -> Result<super::session::PlanetSideSession, super::runtime::AdapterError> {
    let mut session = create_production_session(
        ship,
        SurfacePoint {
            x: context.landing_x,
            y: context.landing_y,
        },
        context.facing,
    )?;
    session.set_hazard_chances(super::world::hazard_chances(
        context.tectonics_rating,
        context.weather_rating,
        context.temperature,
    ));
    Ok(session)
}

fn install_fixture(
    layout: Option<crate::engine::observe_port::planet_side::FixtureLayout>,
    session: &super::session::PlanetSideSession,
    surface: &super::assembly::SharedSurface,
    world_visuals: &mut impl super::automation_fixture::FixtureVisualPort,
) -> Result<(), super::runtime::AdapterError> {
    if let Some(layout) = layout {
        let fixture = super::automation_fixture::PlanetSideFixture::from_layout(
            layout,
            session.lander.position,
        );
        fixture.install(
            super::automation_fixture::AutomationGate::Active,
            session,
            surface,
            world_visuals,
        )?;
    }
    Ok(())
}

fn settle_session(
    session: &super::session::PlanetSideSession,
    ship: &mut impl super::runtime::ShipStatusPort,
    succeeded: bool,
) -> Result<(), super::runtime::AdapterError> {
    if !succeeded {
        let crew = session.lander.crew.get();
        ship.apply(&super::session::ShipDelta {
            crew: i16::from(crew),
            landers: if crew == 0 { -1 } else { 0 },
            ..super::session::ShipDelta::default()
        })?;
    }
    Ok(())
}

fn session_reply(
    result: &Result<
        Result<super::session::SessionOutcome, super::controller::ControllerError>,
        super::runtime::AdapterError,
    >,
) -> PlanetSideReply {
    use super::session::SessionOutcome;
    match result {
        Ok(Ok(SessionOutcome::Returned(delta))) => PlanetSideReply {
            status: STATUS_OK,
            detail: DETAIL_RETURNED,
            value0: i64::from(delta.crew),
            value1: i64::from(delta.element_mass),
        },
        Ok(Ok(SessionOutcome::LanderDestroyed(_))) => PlanetSideReply {
            status: STATUS_OK,
            detail: DETAIL_DESTROYED,
            ..PlanetSideReply::default()
        },
        Ok(Ok(SessionOutcome::Aborted)) => PlanetSideReply {
            status: STATUS_OK,
            detail: DETAIL_ABORTED,
            ..PlanetSideReply::default()
        },
        Ok(Err(super::controller::ControllerError::Runtime(
            super::runtime::RuntimeError::Adapter(error),
        )))
        | Err(error) => PlanetSideReply {
            status: STATUS_OK,
            detail: DETAIL_ADAPTER_FAILURE,
            value0: i64::from(adapter_error_code(error.operation)),
            ..PlanetSideReply::default()
        },
        Ok(Err(super::controller::ControllerError::FrameBudgetExceeded)) => PlanetSideReply {
            status: STATUS_OK,
            detail: DETAIL_FRAME_BUDGET,
            ..PlanetSideReply::default()
        },
    }
}

pub(super) const fn adapter_error_code(operation: &str) -> u32 {
    let bytes = operation.as_bytes();
    let mut hash = 2_166_136_261_u32;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u32;
        hash = hash.wrapping_mul(16_777_619);
        index += 1;
    }
    hash
}

fn valid_context(context: &PlanetSideRunContext) -> bool {
    !context.solar_system.is_null()
        && !context.world.is_null()
        && !context.misc_data_frame.is_null()
        && context.tick_period != 0
        && context.frame_budget != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trip_requires_live_resources_and_nonzero_cadence_and_budget() {
        let resource = std::ptr::dangling_mut::<c_void>();
        let context = PlanetSideRunContext {
            solar_system: resource,
            world: resource,
            misc_data_frame: resource,
            tick_period: 24,
            frame_budget: 1,
            ..PlanetSideRunContext::default()
        };
        assert!(valid_context(&context));
        for invalid in [
            PlanetSideRunContext {
                solar_system: std::ptr::null_mut(),
                ..context
            },
            PlanetSideRunContext {
                world: std::ptr::null_mut(),
                ..context
            },
            PlanetSideRunContext {
                misc_data_frame: std::ptr::null_mut(),
                ..context
            },
            PlanetSideRunContext {
                tick_period: 0,
                ..context
            },
            PlanetSideRunContext {
                frame_budget: 0,
                ..context
            },
        ] {
            assert!(!valid_context(&invalid));
        }
    }

    #[test]
    fn finalization_publishes_each_outcome_as_a_copy_of_its_own_trip() {
        use super::super::{
            controller::ControllerError,
            runtime::{AdapterError, RuntimeError},
            session::SessionOutcome,
        };
        use crate::engine::observe::{CampaignView, Injected, Observer, Presented};
        use crate::engine::observe_port::planet_side::{terminal, PlanetSideSnapshot};

        #[derive(Default)]
        struct Recording(Vec<PlanetSideSnapshot>);
        impl Observer<Option<CampaignView>> for Recording {
            fn input_admitted(&mut self, _: &InputView, _: &Option<CampaignView>) -> Injected {
                Default::default()
            }
            fn checkpoint(&mut self, view: &CheckpointView<'_>) {
                if let CheckpointView::PlanetSide(snapshot) = view {
                    self.0.push((*snapshot).clone());
                }
            }
            fn presented(&mut self, _: &mut Presented) {}
        }
        let mut observer = Recording::default();
        let cases = [
            (
                Ok(Ok(SessionOutcome::Returned(Default::default()))),
                terminal::RETURNED,
            ),
            (
                Ok(Ok(SessionOutcome::LanderDestroyed(Default::default()))),
                terminal::DESTROYED,
            ),
            (Ok(Ok(SessionOutcome::Aborted)), terminal::ABORTED),
            (
                Ok(Err(ControllerError::FrameBudgetExceeded)),
                terminal::FRAME_BUDGET,
            ),
            (
                Ok(Err(ControllerError::Runtime(RuntimeError::Adapter(
                    AdapterError::new("render"),
                )))),
                terminal::ADAPTER_FAILURE,
            ),
            (
                Err(AdapterError::new("lander_assets_not_loaded")),
                terminal::ADAPTER_FAILURE,
            ),
        ];
        for (index, (result, terminal)) in cases.into_iter().enumerate() {
            let trip = std::cell::RefCell::new(TripTelemetry::pending(index as u64 + 1));
            {
                let io = std::cell::RefCell::new(crate::engine::observe::Io {
                    observer: &mut observer,
                });
                complete_trip(&trip, &io, &result);
            }
            let copied = observer.0.last().unwrap();
            assert_eq!(copied.observation.generation, index as u64 + 1);
            assert_eq!(copied.observation.terminal, terminal);
            assert!(!copied.observation.active);
            assert_eq!(copied, trip.borrow().snapshot());
            trip.borrow_mut().mineral_pickup();
            assert_eq!(observer.0.last().unwrap().observation.mineral_pickups, 0);
        }
        assert_eq!(observer.0.len(), 6);
        assert_eq!(observer.0[0].observation.terminal, terminal::RETURNED);
    }

    #[test]
    fn adapter_error_code_is_stable_and_distinguishes_operations() {
        assert_eq!(adapter_error_code("render"), adapter_error_code("render"));
        assert_ne!(adapter_error_code("render"), adapter_error_code("audio"));
    }
}
