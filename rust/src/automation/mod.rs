//! Runtime automation subsystem — typed contracts, validation, pure
//! scheduler/watchdog reducers, trace/artifact/identity I/O primitives,
//! pure sticky-terminal runtime model, and CLI/lifecycle integration.
//!
//! # Phase ownership
//!
//! - P01: closed script parsing/types/validation (REQ-SCRIPT-001..006)
//! - P02: pure scheduler reducer, watchdog reducer, capture generation model
//!   (REQ-SCHED-001..003, REQ-WATCH-001..003, REQ-DET-001)
//! - P03: ordered trace records, safe/exclusive artifact naming, durable
//!   file helpers, SHA-256 manifests, identity metadata
//!   (REQ-IO-001..003, REQ-TRACE-001)
//! - P04: pure shell/fallback/mirror/finalization state model
//!   (REQ-STATE-001..004, REQ-WATCH-004 classification)
//! - P05: CLI/setup validation, lifecycle finalization, active receipt,
//!   outer terminal guard (REQ-MODE-001..003, REQ-BUILD-001,
//!   REQ-EXIT-006/008/009, REQ-FFI-005 finalization)
//!
//! Later phases add input/menu, capture, transport, and proof.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P01..P05

pub mod artifact;
pub mod battle_observer;
pub mod capture;
pub mod child_session;
pub mod coordinator;
pub mod error;
pub mod identity;
pub mod input;
pub mod input_ffi;
pub mod lifecycle;
#[cfg(feature = "debug-process")]
pub mod native_window;
pub mod navigation;
pub mod outcome;
pub mod proof;
pub mod runtime;
pub mod scenario;
pub mod scheduler;
pub mod script;
pub mod setup;
pub mod sync_model;
pub mod trace;
pub mod transport;
pub mod ui_observation;
pub mod watchdog;

pub use capture::{
    attempt_capture_completion, capture_trace_record, classify_present, present_trace_record,
    safe_row_copy, should_count_present, validate_surface, CaptureCompletion, CaptureMetadata,
    PresentClassification, SurfaceError, SurfaceMetadata,
};
#[cfg(unix)]
pub use child_session::{
    ChildSession, ChildSessionConfig, ChildSessionError, ChildSessionFailure, ChildSessionReceipt,
    StreamKind,
};
pub use child_session::{
    ChildSessionModel, HangClassification, ProcessIdentity, ProofResult, ProofType, SessionResult,
    SessionState,
};
pub use coordinator::{Coordinator, AUTOMATION_SEED};
pub use error::AutomationError;
pub use input::{
    combine_stops, menu_key_to_index, observe_main_menu_transition, observe_menu_key,
    setter_set_menu_key, CallbackControl, MainMenuTransitionEvent, MenuKeySnapshot, SetterResult,
    MENU_KEY_INDICES, NUM_MENU_KEYS,
};
pub use lifecycle::{
    check_terminal_guard, map_status, reassert_abort_if_terminal, run_lifecycle, GameLifecycle,
    LifecycleResult, TeardownReceipt,
};
#[cfg(feature = "debug-process")]
pub use native_window::{
    acknowledge_native_window_state, activate_native_window_proof, active_native_window_config,
    capture_native_window, native_acceptance_failure_inventory, native_acceptance_inventory,
    observe_native_window, publish_native_window_state, read_native_window_state,
    validate_native_acceptance_bundle, validate_native_acceptance_failure_bundle,
    validate_native_acceptance_setup_failure_bundle, validate_native_window_bundle,
    validate_native_window_receipt, ActiveNativeWindowConfig, NativeAcceptanceFailureManifest,
    NativeAcceptanceManifest, NativeAcceptanceSetupFailureContract,
    NativeAcceptanceSetupFailureManifest, NativeChildCleanupReceipt, NativeExecutionIdentity,
    NativeLinkedBuildReceipt, NativeProcessIdentity, NativeRetainedInput, NativeScreenshot,
    NativeScreenshotStage, NativeWindowAck, NativeWindowAckPublisher, NativeWindowBinding,
    NativeWindowBounds, NativeWindowChildState, NativeWindowConfigFile, NativeWindowObservation,
    NativeWindowObserverError, NativeWindowProof, NativeWindowProofError, NativeWindowPublication,
    NativeWindowReceipt, NativeWindowSemanticSnapshot, NativeWindowStateReader,
    ObservedNativeWindow, NATIVE_ACCEPTANCE_FAILURE_SCHEMA, NATIVE_ACCEPTANCE_SCHEMA,
    NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA, NATIVE_LINKED_BUILD_RECEIPT_SCHEMA,
    NATIVE_WINDOW_ACK_SCHEMA, NATIVE_WINDOW_CONFIG_SCHEMA, NATIVE_WINDOW_RECEIPT_SCHEMA,
    NATIVE_WINDOW_STATE_SCHEMA,
};
pub use outcome::TerminalClass;
pub use proof::{
    counter_paths_are_distinct, inactive_teardown_is_distinct, teardown_is_distinct,
    validate_proof_run, ArchRequirementStatus, ArchitectureReview, PreflightCheck, ProofIdentity,
    ProofReceipt, ProofValidationError,
};
pub use scenario::{AutomationScene, SceneActivationBoundary, SceneError, ScenePlan};
pub use script::{
    parse_script, validate_script, Action, ActivityAssertion, Budgets, CaptureStep,
    CommunicationResponsesAssertion, DispatchAssertion, GameOptionsAssertion, MainMenuTransition,
    MenuKey, NavigateToMoonStep, NavigateToOrbitStep, NavigateToPlanetStep, PlanetMenuPhaseName,
    PlanetSideOutcomeName, PlayerKey, RootDocument, SceneAssertion, ScriptStep,
    SelectCommunicationResponseStep, SelectPlanetMenuStep, SetMenuKeyStep, SetPlayerKeyStep,
    SetupPlanetSideCollisionFixtureStep, TapMenuKeyStep, TapPlayerKeyStep, ValidatedScript,
    WaitForCommunicationEndStep, WaitForCommunicationReplayStep, WaitForDispatchStep,
    WaitForPlanetSideEndStep, WaitForPlanetSideStartStep, WaitInputTicksStep,
    WaitPresentationsStep, CAPABILITY_REQUIRED_FLAGS,
};
pub use setup::{setup_automation, AutomationOptions, AutomationSetup, BuildCapabilities};
pub use trace::{PresentationEvidence, RecordKind, SeedDomain, TraceRecord};
pub use transport::{
    AckKind, AckRecord, CommandId, TransportCounters, TransportPacket, TransportState,
    MAX_SOCKET_PATH_LEN, PACKETS_PER_PUMP, PROTOCOL_VERSION,
};
