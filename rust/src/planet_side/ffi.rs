//! Owner-supplied PlanetSide session boundary.

#[cfg(not(test))]
use crate::engine::observe::InputView;
use std::ffi::c_void;

pub const STATUS_OK: i32 = 0;
pub const STATUS_INVALID_ARGUMENT: i32 = -4;
#[cfg(not(test))]
const DETAIL_RETURNED: u32 = 1;
#[cfg(not(test))]
const DETAIL_DESTROYED: u32 = 2;
#[cfg(not(test))]
const DETAIL_ABORTED: u32 = 3;
#[cfg(not(test))]
const DETAIL_ADAPTER_FAILURE: u32 = 4;
#[cfg(not(test))]
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

#[cfg(not(test))]
use crate::graphics::pixmap_abi::GetFrameCount;

#[cfg(not(test))]
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
#[cfg(not(test))]
pub unsafe fn run_session(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
    context: &mut PlanetSideRunContext,
) -> PlanetSideReply {
    use super::adapters::{CffiPlanetSideClock, CffiPlanetSideInput, CffiShipStatus};
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
    use super::runtime::{PlanetSideClock, RuntimeAdapters, ShipStatusPort, Tick};
    use super::session_factory::create_production_session;
    use super::visual_adapter::CffiSurfaceVisuals;

    if !valid_context(context) {
        return failure(STATUS_INVALID_ARGUMENT);
    }

    let result = (|| {
        let assets = cx.with_legacy(super::init_lander::borrowed_assets)?;
        let (generator, mut visuals, persistence, surface, ship, session) =
            cx.with_legacy(|| {
                let mut generator =
                    CffiSurfaceGenerator::for_planet_side(context.solar_system, context.world)?;
                let mut visuals = CffiSurfaceVisuals::new(
                    context.misc_data_frame,
                    context.energy_frame,
                    context.life_frames,
                )?;
                let persistence = ScanPersistence::from_masks(context.retrieval_masks);
                let lander_masks =
                    unsafe { extract_lander_masks(assets.graphic(LanderGraphic::Lander))? };
                let assembly =
                    assemble_surface(&mut generator, persistence, &mut visuals, lander_masks)?;
                let surface = share_surface(assembly);
                let mut ship = CffiShipStatus;
                let mut session = create_production_session(
                    &mut ship,
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
                Ok::<_, super::runtime::AdapterError>((
                    generator,
                    visuals,
                    persistence,
                    surface,
                    ship,
                    session,
                ))
            })?;
        let mut world_visuals = super::visual_adapter::CffiWorldVisuals::new(&assets, &mut visuals);
        let automation_active = io.observer.input_admitted(&InputView::Active, &None).stop;
        let channels = cx.kernel.channels.clone();
        let (mut controller, _menu_silence) = cx.with_legacy(|| {
            // The issue #162 fixture is optional and explicitly requested.  Only the
            // `setup_planet_side_collision_fixture` script action queues the
            // request (the coordinator invokes
            // [`super::automation_fixture::coordinator_queues_fixture_request`]
            // when that action executes), so active automation with no such action runs
            // the normal generated PlanetSide session unchanged.  The request is
            // delivered exactly once per run_session, and install fails fast unless
            // this session is under an active automation coordinator.  The gate and
            // the install error are the sole outside-session protection; there is no
            // polling fallback.
            //
            // `AutomationObserver::is_active()` is sampled exactly once, before tapping.
            // That single observation decides both whether the request is consumed and
            // which [`AutomationGate`] the install receives, so a request cannot be
            // consumed under one observation and installed under another.  Ordinary
            // gameplay never consumes or fails from a stale queue: a request left
            // pending because no active owner tapped it stays queued.

            let fixture_request = if automation_active {
                super::automation_fixture::tap_planet_side_fixture_request(session.lander.position)
            } else {
                None
            };
            if let Some(fixture) = fixture_request {
                fixture.install(
                    super::automation_fixture::automation_gate(automation_active),
                    &session,
                    &surface,
                    &mut world_visuals,
                )?;
            }
            let collision = SurfaceCollisionAdapter {
                surface: surface.clone(),
                random: CffiGameplayRandom,
                generator,
                persistence,
                world_visuals,
                earthquake_frame_count: unsafe {
                    GetFrameCount((assets.graphic(LanderGraphic::Earthquake)).cast())
                },
                lava_frame_count: unsafe {
                    GetFrameCount((assets.graphic(LanderGraphic::Lava)).cast())
                },
            };
            // The orbit/scan menu underneath stays resident, so silence its
            // navigation sounds for the trip. Restored on every exit path by Drop.
            let menu_silence = super::menu_sounds::MenuSoundSilence::acquire();
            super::telemetry::begin(&session);
            let adapters = RuntimeAdapters {
                input: CffiPlanetSideInput,
                collision,
                graphics: CffiPlanetSideGraphics {
                    surface: surface.clone(),
                    assets: &assets,
                    last_scan_position: None,
                },
                audio: CffiPlanetSideAudio::new(assets.sounds(), channels),
                clock: CffiPlanetSideClock,
                ship,
            };
            let controller = PlanetSideController::new(
                session,
                adapters,
                context.tick_period,
                context.frame_budget,
            );
            Ok::<_, super::runtime::AdapterError>((controller, menu_silence))
        })?;
        let outcome = controller.run_with_frame(|session, adapters, tick_period| {
            let deadline = Tick(adapters.clock.now().0.wrapping_add(tick_period));
            let input = if session.phase == super::session::SessionPhase::Active {
                Some(adapters.input.poll_observed(cx, io)?)
            } else {
                None
            };
            cx.with_legacy(|| super::runtime::run_frame_at(session, adapters, deadline, input))
        });
        cx.with_legacy(|| {
            if outcome.is_ok() {
                // Pickup callbacks can also change other scan categories. Read the
                // live planet rather than replacing it with the trip's initial copy.
                context.retrieval_masks = unsafe {
                    (*context
                        .solar_system
                        .cast::<crate::c_bindings::global_state::SOLARSYS_STATE>())
                    .SysInfo
                    .PlanetInfo
                    .ScanRetrieveMask
                };
            } else {
                let crew = controller.session.lander.crew.get();
                controller.adapters.ship.apply(&super::session::ShipDelta {
                    crew: i16::from(crew),
                    landers: if crew == 0 { -1 } else { 0 },
                    ..super::session::ShipDelta::default()
                })?;
            }
            drop(controller);
            Ok::<_, super::runtime::AdapterError>(outcome)
        })
    })();

    session_reply(result)
}

#[cfg(not(test))]
fn session_reply(
    result: Result<
        Result<super::session::SessionOutcome, super::controller::ControllerError>,
        super::runtime::AdapterError,
    >,
) -> PlanetSideReply {
    use super::session::SessionOutcome;
    match result {
        Ok(Ok(SessionOutcome::Returned(delta))) => {
            let outcome = SessionOutcome::Returned(delta.clone());
            super::telemetry::finish(&outcome);
            PlanetSideReply {
                status: STATUS_OK,
                detail: DETAIL_RETURNED,
                value0: i64::from(delta.crew),
                value1: i64::from(delta.element_mass),
            }
        }
        Ok(Ok(SessionOutcome::LanderDestroyed(delta))) => {
            super::telemetry::finish(&SessionOutcome::LanderDestroyed(delta));
            PlanetSideReply {
                status: STATUS_OK,
                detail: DETAIL_DESTROYED,
                ..PlanetSideReply::default()
            }
        }
        Ok(Ok(SessionOutcome::Aborted)) => {
            super::telemetry::finish(&SessionOutcome::Aborted);
            PlanetSideReply {
                status: STATUS_OK,
                detail: DETAIL_ABORTED,
                ..PlanetSideReply::default()
            }
        }
        Ok(Err(super::controller::ControllerError::Runtime(
            super::runtime::RuntimeError::Adapter(error),
        )))
        | Err(error) => {
            super::telemetry::adapter_failure(error.operation);
            PlanetSideReply {
                status: STATUS_OK,
                detail: DETAIL_ADAPTER_FAILURE,
                value0: i64::from(adapter_error_code(error.operation)),
                ..PlanetSideReply::default()
            }
        }
        Ok(Err(super::controller::ControllerError::FrameBudgetExceeded)) => {
            super::telemetry::frame_budget_failure();
            PlanetSideReply {
                status: STATUS_OK,
                detail: DETAIL_FRAME_BUDGET,
                ..PlanetSideReply::default()
            }
        }
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
    fn adapter_error_code_is_stable_and_distinguishes_operations() {
        assert_eq!(adapter_error_code("render"), adapter_error_code("render"));
        assert_ne!(adapter_error_code("render"), adapter_error_code("audio"));
    }
}
