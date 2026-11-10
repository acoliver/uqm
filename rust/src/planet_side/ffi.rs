//! Owner-supplied PlanetSide session boundary.

use crate::engine::observe::InputView;
use crate::graphics::context_abi::ContextFrame;
use crate::planets::solar_system::SolarSystem;
use std::num::NonZeroU32;

/// Retained resources and validated parameters for one synchronous lander trip.
pub struct TripInput {
    pub misc_data_frame: ContextFrame,
    pub energy_frame: Option<ContextFrame>,
    pub life_frames: [Option<ContextFrame>; 3],
    pub landing: super::model::SurfacePoint,
    pub facing: u8,
    pub tick_period: NonZeroU32,
    pub frame_budget: NonZeroU32,
    pub tectonics_rating: u8,
    pub weather_rating: u8,
    pub temperature: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TripError {
    Adapter(super::runtime::AdapterError),
    FrameBudgetExceeded,
}

impl From<super::runtime::AdapterError> for TripError {
    fn from(error: super::runtime::AdapterError) -> Self {
        Self::Adapter(error)
    }
}

impl From<super::controller::ControllerError> for TripError {
    fn from(error: super::controller::ControllerError) -> Self {
        match error {
            super::controller::ControllerError::FrameBudgetExceeded => Self::FrameBudgetExceeded,
            super::controller::ControllerError::Runtime(super::runtime::RuntimeError::Adapter(
                error,
            )) => Self::Adapter(error),
        }
    }
}

pub type TripResult = Result<super::session::SessionOutcome, TripError>;

use super::adapters::{CffiPlanetSideClock, CffiPlanetSideInput, CffiShipStatus, ObservedClock};
use super::assembly::{assemble_surface, share_surface};
use super::assets::CffiPlanetSideAudio;
use super::collision_adapter::{CffiGameplayRandom, SurfaceCollisionAdapter};
use super::controller::PlanetSideController;
use super::generation::ScanPersistence;
use super::generation_adapter::CffiSurfaceGenerator;
use super::graphics_adapter::{CffiPlanetSideGraphics, SurfaceDrawing};
use super::mask_adapter::extract_lander_masks;
use super::resources::{LanderGraphic, PlanetSideAssets};
use super::runtime::{PlanetSideClock, RuntimeAdapters, Tick};
use super::session_factory::create_production_session;
use super::telemetry::TripTelemetry;
use super::visual_adapter::CffiSurfaceVisuals;

pub fn run_session(
    drawing: SurfaceDrawing<'_>,
    controls: &crate::engine::input::InputSession,
    activity: crate::engine::world::activity::ActivityControl,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    solar: &mut SolarSystem,
    input: TripInput,
) -> TripResult {
    let game = std::cell::RefCell::new(cx.reborrow());
    let observe = std::cell::RefCell::new(crate::engine::observe::Io {
        observer: &mut *io.observer,
    });
    let admission = admit_trip(&observe);
    let result = run_retained_session(
        drawing, controls, activity, &game, &observe, solar, &input, &admission,
    );
    complete_trip(&admission.0, &observe, &result);
    result
}

type TripAdmission = (
    std::rc::Rc<std::cell::RefCell<TripTelemetry>>,
    Option<crate::engine::observe_port::planet_side::FixtureLayout>,
);

fn admit_trip(observe: &std::cell::RefCell<crate::engine::observe::Io<'_>>) -> TripAdmission {
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
    result: &TripResult,
) {
    {
        let mut trip = trip.borrow_mut();
        match &result {
            Ok(outcome) => trip.finish(outcome),
            Err(TripError::FrameBudgetExceeded) => trip.frame_budget_failure(),
            Err(TripError::Adapter(error)) => {
                trip.batch_depth_corrected(error.batch_depth_corrections);
                trip.adapter_failure(error.operation);
            }
        }
        super::adapters::publish_trip(observe, &trip);
    }
}

fn run_retained_session(
    drawing: SurfaceDrawing<'_>,
    controls: &crate::engine::input::InputSession,
    activity: crate::engine::world::activity::ActivityControl,
    game: &std::cell::RefCell<crate::engine::battle::Cx<'_>>,
    observe: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
    solar: &mut SolarSystem,
    context: &TripInput,
    admission: &TripAdmission,
) -> TripResult {
    let (telemetry, fixture) = admission;
    let assets = PlanetSideAssets::retain(game.borrow().kernel.lander_resources.as_ref().ok_or(
        super::runtime::AdapterError::new("lander_assets_not_loaded"),
    )?);
    let reports = drawing.scan.report_context();
    let mut generator = CffiSurfaceGenerator::for_planet_side(game, observe, solar, reports);
    let mut visuals = retained_visuals(context)?;
    let persistence = ScanPersistence::from_masks(generator.retrieval_masks());
    let lander_masks = extract_lander_masks(&assets.graphic(LanderGraphic::Lander))?;
    let assembly = assemble_surface(&mut generator, persistence, &mut visuals, lander_masks)?;
    let surface = share_surface(assembly);
    let mut ship = CffiShipStatus { game };
    let session = create_trip_session(&mut ship, context)?;
    let mut world_visuals = super::visual_adapter::CffiWorldVisuals::new(&assets, &mut visuals);
    let channels = game.borrow().kernel.channels.clone();
    install_fixture(*fixture, &session, &surface, &mut world_visuals)?;
    let (earthquake_frame_count, lava_frame_count) = hazard_frame_counts(&assets);
    let publish = |trip: &TripTelemetry| super::adapters::publish_trip(observe, trip);
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
    let clock = CffiPlanetSideClock(crate::threading::pacing::GameThreadPacer::new(
        drawing.planet.clone(),
    ));
    let adapters = RuntimeAdapters {
        input: CffiPlanetSideInput::new(controls.controls().clone(), activity),
        collision,
        graphics: CffiPlanetSideGraphics {
            drawing,
            game,
            surface: surface.clone(),
            assets: &assets,
            last_scan_position: None,
        },
        audio: CffiPlanetSideAudio::new(assets.sounds(), channels, telemetry.clone()),
        clock: ObservedClock {
            clock,
            telemetry: telemetry.clone(),
            io: observe,
        },
        ship,
    };
    let mut controller = PlanetSideController::new(
        session,
        adapters,
        context.tick_period.get(),
        context.frame_budget.get(),
    );
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
    outcome.map_err(TripError::from)
}

fn retained_visuals(
    context: &TripInput,
) -> Result<CffiSurfaceVisuals, super::runtime::AdapterError> {
    CffiSurfaceVisuals::new(
        context.misc_data_frame.clone(),
        context.energy_frame.clone(),
        context.life_frames.clone(),
    )
}

fn hazard_frame_counts(assets: &PlanetSideAssets) -> (u16, u16) {
    (
        assets.graphic(LanderGraphic::Earthquake).frame_count(),
        assets.graphic(LanderGraphic::Lava).frame_count(),
    )
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
    context: &TripInput,
) -> Result<super::session::PlanetSideSession, super::runtime::AdapterError> {
    let mut session = create_production_session(ship, context.landing, context.facing)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::observe::CheckpointView;

    #[test]
    fn typed_trip_failures_preserve_the_operation_and_batch_corrections() {
        let adapter = super::super::runtime::AdapterError {
            operation: "render",
            batch_depth_corrections: 2,
        };
        assert_eq!(
            TripError::from(adapter.clone()),
            TripError::Adapter(adapter.clone())
        );
        assert_eq!(
            TripError::from(super::super::controller::ControllerError::FrameBudgetExceeded),
            TripError::FrameBudgetExceeded
        );
        assert_eq!(
            TripError::from(super::super::controller::ControllerError::Runtime(
                super::super::runtime::RuntimeError::Adapter(adapter.clone())
            )),
            TripError::Adapter(adapter)
        );
    }

    #[test]
    fn finalization_publishes_each_outcome_as_a_copy_of_its_own_trip() {
        use super::super::{runtime::AdapterError, session::SessionOutcome};
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
                Ok(SessionOutcome::Returned(Default::default())),
                terminal::RETURNED,
            ),
            (
                Ok(SessionOutcome::LanderDestroyed(Default::default())),
                terminal::DESTROYED,
            ),
            (Ok(SessionOutcome::Aborted), terminal::ABORTED),
            (Err(TripError::FrameBudgetExceeded), terminal::FRAME_BUDGET),
            (
                Err(TripError::Adapter(AdapterError::new("render"))),
                terminal::ADAPTER_FAILURE,
            ),
            (
                Err(TripError::Adapter(AdapterError::new(
                    "lander_assets_not_loaded",
                ))),
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
