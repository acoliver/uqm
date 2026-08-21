//! Single versioned transitional ABI for PlanetSide ownership migration.

use std::ffi::c_void;

use super::hazards::{hazard_chance, thermal_hazard_rating, HazardKind};

pub const PLANET_SIDE_ABI_VERSION: u32 = 2;

const OP_QUERY_ABI: u32 = 0;
const OP_THERMAL_RATING: u32 = 1;
const OP_HAZARD_CHANCE: u32 = 2;
const OP_RUN_SESSION: u32 = 3;

pub const STATUS_OK: i32 = 0;
pub const STATUS_NULL_POINTER: i32 = -1;
pub const STATUS_ABI_MISMATCH: i32 = -2;
pub const STATUS_UNKNOWN_OPERATION: i32 = -3;
pub const STATUS_INVALID_ARGUMENT: i32 = -4;
pub const STATUS_RUNTIME_ERROR: i32 = -5;

#[cfg(feature = "linked_c_archive")]
const DETAIL_RETURNED: u32 = 1;
#[cfg(feature = "linked_c_archive")]
const DETAIL_DESTROYED: u32 = 2;
#[cfg(feature = "linked_c_archive")]
const DETAIL_ABORTED: u32 = 3;
#[cfg(feature = "linked_c_archive")]
const DETAIL_ADAPTER_FAILURE: u32 = 4;
#[cfg(feature = "linked_c_archive")]
const DETAIL_FRAME_BUDGET: u32 = 5;

/// Fixed-size request for the sole temporary PlanetSide C entry point.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanetSideRequest {
    pub abi_version: u32,
    pub operation: u32,
    pub argument0: i32,
    pub argument1: i32,
    /// Operation-specific borrowed context. No Rust pointer is returned.
    pub context: *mut c_void,
}

/// Context for `OP_RUN_SESSION`, populated by the temporary C caller.
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

/// Sole transitional entry point for PlanetSide operations.
///
/// # Safety
///
/// `request` must point to a readable `PlanetSideRequest` and `reply` must
/// point to writable storage for a `PlanetSideReply`. They may be unaligned,
/// but must remain valid for the duration of this call and must not overlap.
#[no_mangle]
pub unsafe extern "C" fn uqm_rust_planet_side(
    request: *const PlanetSideRequest,
    reply: *mut PlanetSideReply,
) -> i32 {
    if request.is_null() || reply.is_null() {
        return STATUS_NULL_POINTER;
    }

    let request = request.read_unaligned();
    let result = dispatch(request);
    reply.write_unaligned(result);
    result.status
}

fn dispatch(request: PlanetSideRequest) -> PlanetSideReply {
    if request.abi_version != PLANET_SIDE_ABI_VERSION {
        return failure(STATUS_ABI_MISMATCH);
    }

    match request.operation {
        OP_QUERY_ABI => PlanetSideReply {
            status: STATUS_OK,
            value0: i64::from(PLANET_SIDE_ABI_VERSION),
            ..PlanetSideReply::default()
        },
        OP_THERMAL_RATING => PlanetSideReply {
            status: STATUS_OK,
            value0: i64::from(thermal_hazard_rating(request.argument0)),
            ..PlanetSideReply::default()
        },
        OP_HAZARD_CHANCE => {
            let Some(kind) = hazard_from_abi(request.argument0) else {
                return failure(STATUS_INVALID_ARGUMENT);
            };
            let Ok(rating) = u8::try_from(request.argument1) else {
                return failure(STATUS_INVALID_ARGUMENT);
            };
            if rating > 7 {
                return failure(STATUS_INVALID_ARGUMENT);
            }
            PlanetSideReply {
                status: STATUS_OK,
                value0: i64::from(hazard_chance(kind, rating)),
                ..PlanetSideReply::default()
            }
        }
        OP_RUN_SESSION => {
            if request.context.is_null() {
                return failure(STATUS_INVALID_ARGUMENT);
            }
            #[cfg(feature = "linked_c_archive")]
            unsafe {
                run_session(request.context.cast::<PlanetSideRunContext>())
            }
            #[cfg(not(feature = "linked_c_archive"))]
            failure(STATUS_RUNTIME_ERROR)
        }
        _ => failure(STATUS_UNKNOWN_OPERATION),
    }
}
#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn GetFrameCount(frame: *mut c_void) -> libc::c_int;
}

const fn failure(status: i32) -> PlanetSideReply {
    PlanetSideReply {
        status,
        detail: 0,
        value0: 0,
        value1: 0,
    }
}

#[cfg(feature = "linked_c_archive")]
unsafe fn run_session(context: *mut PlanetSideRunContext) -> PlanetSideReply {
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
    use super::runtime::{RuntimeAdapters, ShipStatusPort};
    use super::session::SessionOutcome;
    use super::session_factory::create_production_session;
    use super::visual_adapter::CffiSurfaceVisuals;

    let context = unsafe { &mut *context };
    if context.solar_system.is_null()
        || context.world.is_null()
        || context.misc_data_frame.is_null()
        || context.tick_period == 0
        || context.frame_budget == 0
    {
        return failure(STATUS_INVALID_ARGUMENT);
    }

    let result = (|| {
        let assets = super::init_lander::borrowed_assets()?;
        let mut generator = CffiSurfaceGenerator::new(context.solar_system, context.world)?;
        let mut visuals = CffiSurfaceVisuals::new(
            context.misc_data_frame,
            context.energy_frame,
            context.life_frames,
        )?;
        let persistence = ScanPersistence::from_masks(context.retrieval_masks);
        let lander_masks = unsafe { extract_lander_masks(assets.graphic(LanderGraphic::Lander))? };
        let assembly = assemble_surface(&mut generator, persistence, &mut visuals, lander_masks)?;
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
        let mut world_visuals = super::visual_adapter::CffiWorldVisuals::new(&assets, &mut visuals);
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
        // Checking `Coordinator::is_active()` happens before tapping: ordinary
        // gameplay never consumes or fails from a stale queue.  A request left
        // pending because no active owner tapped it stays queued.
        let fixture_request = if crate::automation::coordinator::Coordinator::is_active() {
            super::automation_fixture::tap_planet_side_fixture_request(session.lander.position)
        } else {
            None
        };
        if let Some(fixture) = fixture_request {
            fixture.install(
                super::automation_fixture::automation_gate(
                    crate::automation::coordinator::Coordinator::is_active(),
                ),
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
                GetFrameCount(assets.graphic(LanderGraphic::Earthquake)) as u16
            },
            lava_frame_count: unsafe { GetFrameCount(assets.graphic(LanderGraphic::Lava)) as u16 },
        };
        let graphics = CffiPlanetSideGraphics {
            surface,
            assets: &assets,
            last_scan_position: None,
        };
        // The orbit/scan menu underneath stays resident, so silence its
        // navigation sounds for the trip. Restored on every exit path by Drop.
        let _menu_silence = super::menu_sounds::MenuSoundSilence::acquire();
        super::telemetry::begin(&session);
        let adapters = RuntimeAdapters {
            input: CffiPlanetSideInput,
            collision,
            graphics,
            audio: CffiPlanetSideAudio::new(assets.sounds()),
            clock: CffiPlanetSideClock,
            ship,
        };
        let mut controller =
            PlanetSideController::new(session, adapters, context.tick_period, context.frame_budget);
        let outcome = controller.run();
        if outcome.is_ok() {
            context.retrieval_masks = controller.adapters.collision.persistence.to_masks();
        } else {
            let crew = controller.session.lander.crew.get();
            controller.adapters.ship.apply(&super::session::ShipDelta {
                crew: i16::from(crew),
                landers: if crew == 0 { -1 } else { 0 },
                ..super::session::ShipDelta::default()
            })?;
        }
        Ok::<_, super::runtime::AdapterError>(outcome)
    })();

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

const fn hazard_from_abi(value: i32) -> Option<HazardKind> {
    match value {
        1 => Some(HazardKind::Earthquake),
        2 => Some(HazardKind::Lightning),
        3 => Some(HazardKind::Lava),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(request: PlanetSideRequest) -> (i32, PlanetSideReply) {
        let mut reply = PlanetSideReply::default();
        let status = unsafe { uqm_rust_planet_side(&request, &mut reply) };
        (status, reply)
    }

    #[test]
    fn null_pointers_are_rejected_without_dereference() {
        assert_eq!(
            unsafe { uqm_rust_planet_side(std::ptr::null(), std::ptr::null_mut()) },
            STATUS_NULL_POINTER
        );
    }

    #[test]
    fn version_query_uses_fixed_reply_without_borrowed_pointer() {
        let (status, reply) = call(PlanetSideRequest {
            abi_version: PLANET_SIDE_ABI_VERSION,
            operation: OP_QUERY_ABI,
            ..PlanetSideRequest::default()
        });
        assert_eq!(status, STATUS_OK);
        assert_eq!(reply.value0, i64::from(PLANET_SIDE_ABI_VERSION));
        assert_eq!(std::mem::size_of::<PlanetSideReply>(), 24);
    }

    #[test]
    fn mismatched_version_and_unknown_operation_are_sticky_errors() {
        assert_eq!(call(PlanetSideRequest::default()).0, STATUS_ABI_MISMATCH);
        assert_eq!(
            call(PlanetSideRequest {
                abi_version: PLANET_SIDE_ABI_VERSION,
                operation: 999,
                ..PlanetSideRequest::default()
            })
            .0,
            STATUS_UNKNOWN_OPERATION
        );
    }

    #[test]
    fn thermal_rating_is_available_through_the_single_dispatcher() {
        let (_, reply) = call(PlanetSideRequest {
            abi_version: PLANET_SIDE_ABI_VERSION,
            operation: OP_THERMAL_RATING,
            argument0: 800,
            argument1: 0,
            context: std::ptr::null_mut(),
        });
        assert_eq!(reply.status, STATUS_OK);
        assert_eq!(reply.value0, 7);
    }

    #[test]
    fn hazard_chance_validates_kind_and_rating() {
        let (_, reply) = call(PlanetSideRequest {
            abi_version: PLANET_SIDE_ABI_VERSION,
            operation: OP_HAZARD_CHANCE,
            argument0: 3,
            argument1: 7,
            context: std::ptr::null_mut(),
        });
        assert_eq!(reply.value0, 144);

        assert_eq!(
            call(PlanetSideRequest {
                abi_version: PLANET_SIDE_ABI_VERSION,
                operation: OP_HAZARD_CHANCE,
                argument0: 0,
                argument1: 7,
                context: std::ptr::null_mut(),
            })
            .0,
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            call(PlanetSideRequest {
                abi_version: PLANET_SIDE_ABI_VERSION,
                operation: OP_HAZARD_CHANCE,
                argument0: 1,
                argument1: 8,
                context: std::ptr::null_mut(),
            })
            .0,
            STATUS_INVALID_ARGUMENT
        );
    }

    #[test]
    fn adapter_error_code_is_stable_and_distinguishes_operations() {
        assert_eq!(adapter_error_code("render"), adapter_error_code("render"));
        assert_ne!(adapter_error_code("render"), adapter_error_code("audio"));
    }

    #[test]
    fn run_session_requires_context_and_never_falls_back_when_unlinked() {
        assert_eq!(
            call(PlanetSideRequest {
                abi_version: PLANET_SIDE_ABI_VERSION,
                operation: OP_RUN_SESSION,
                ..PlanetSideRequest::default()
            })
            .0,
            STATUS_INVALID_ARGUMENT
        );

        let mut context = PlanetSideRunContext::default();
        assert_eq!(
            call(PlanetSideRequest {
                abi_version: PLANET_SIDE_ABI_VERSION,
                operation: OP_RUN_SESSION,
                context: (&mut context as *mut PlanetSideRunContext).cast(),
                ..PlanetSideRequest::default()
            })
            .0,
            STATUS_RUNTIME_ERROR
        );
    }
}
