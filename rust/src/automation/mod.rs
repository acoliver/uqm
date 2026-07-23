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
pub mod capture;
pub mod child_session;
pub mod error;
pub mod identity;
pub mod input;
pub mod input_ffi;
pub mod lifecycle;
pub mod outcome;
pub mod proof;
pub mod runtime;
pub mod scheduler;
pub mod script;
pub mod setup;
pub mod sync_model;
pub mod trace;
pub mod transport;
pub mod watchdog;

pub use capture::{
    attempt_capture_completion, capture_trace_record, classify_present, present_trace_record,
    safe_row_copy, should_count_present, validate_surface, CaptureCompletion, CaptureMetadata,
    PresentClassification, SurfaceError, SurfaceMetadata,
};
pub use child_session::{
    ChildSessionModel, HangClassification, ProcessIdentity, ProofResult, ProofType, SessionResult,
    SessionState,
};
pub use error::AutomationError;
pub use input::{
    combine_stops, menu_key_to_index, observe_main_menu_transition, observe_menu_key,
    setter_set_menu_key, CallbackControl, MainMenuTransitionEvent, MenuKeySnapshot, SetterResult,
    MENU_KEY_INDICES, NUM_MENU_KEYS,
};
pub use lifecycle::{
    check_terminal_guard, map_status, reassert_abort_if_terminal, run_lifecycle, GameLifecycle,
    LifecycleResult,
};
pub use proof::{
    counter_paths_are_distinct, inactive_teardown_is_distinct, teardown_is_distinct,
    validate_proof_run, ArchRequirementStatus, ArchitectureReview, PreflightCheck, ProofIdentity,
    ProofReceipt, ProofValidationError,
};
pub use script::{
    parse_script, validate_script, Action, ActivityAssertion, Budgets, CaptureStep,
    MainMenuTransition, MenuKey, RootDocument, ScriptStep, SetMenuKeyStep, TapMenuKeyStep,
    ValidatedScript, WaitInputTicksStep, CAPABILITY_REQUIRED_FLAGS,
};
pub use setup::{setup_automation, AutomationOptions, AutomationSetup, BuildCapabilities};
pub use transport::{
    AckKind, AckRecord, CommandId, TransportCounters, TransportPacket, TransportState,
    MAX_SOCKET_PATH_LEN, PACKETS_PER_PUMP, PROTOCOL_VERSION,
};
