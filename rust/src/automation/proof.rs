//! Proof receipts and proof run validation model.
//!
//! Validates that proof runs meet the requirements: fresh exclusive root,
//! SHA-256 identity, preflight process check, create-new-only proof report,
//! and teardown receipt validation.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
//! @requirement REQ-PROOF-001..008

// ===========================================================================
//  Proof identity (REQ-PROOF-003)
// ===========================================================================

/// SHA-256 identity for a proof run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-003
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofIdentity {
    /// SHA-256 digest of the executable binary.
    pub executable_digest: String,
    /// SHA-256 digest of the automation script.
    pub script_digest: String,
    /// SHA-256 digest of the content directory manifest.
    pub content_digest: String,
    /// SHA-256 digest of the build configuration.
    pub build_digest: String,
    /// SHA-256 digest of the initial config.
    pub initial_config_digest: String,
    /// SHA-256 digest of the final config.
    pub final_config_digest: String,
}

impl ProofIdentity {
    /// Returns `true` if all digests are non-empty and look like hex
    /// SHA-256 digests (64 hex chars).
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-003
    #[must_use]
    pub fn is_valid(&self) -> bool {
        fn is_sha256_hex(s: &str) -> bool {
            s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
        }
        is_sha256_hex(&self.executable_digest)
            && is_sha256_hex(&self.script_digest)
            && is_sha256_hex(&self.content_digest)
            && is_sha256_hex(&self.build_digest)
            && is_sha256_hex(&self.initial_config_digest)
            && is_sha256_hex(&self.final_config_digest)
    }
}

// ===========================================================================
//  Preflight check (REQ-PROOF-001)
// ===========================================================================

/// Preflight validation for a proof run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-001
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightCheck {
    /// Whether a fresh exclusive root was created.
    pub fresh_root_created: bool,
    /// Whether matching live processes were detected (should be false).
    pub no_matching_processes: bool,
    /// Whether the identity is valid.
    pub identity_valid: bool,
}

impl PreflightCheck {
    /// Returns `true` if the preflight passes.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-001
    #[must_use]
    pub fn passes(&self) -> bool {
        self.fresh_root_created && self.no_matching_processes && self.identity_valid
    }
}

// ===========================================================================
//  Proof receipt (REQ-PROOF-007)
// ===========================================================================

/// The receipt for a completed proof run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-007
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReceipt {
    /// The proof type that was run.
    pub proof_type: crate::automation::child_session::ProofType,
    /// Whether the proof passed.
    pub passed: bool,
    /// The exit code of the child.
    pub exit_code: Option<i32>,
    /// Whether the teardown receipt was created.
    pub teardown_receipt_created: bool,
    /// Whether the proof report was create-new only.
    pub proof_report_create_new: bool,
    /// Whether orphan check passed.
    pub orphan_check_passed: bool,
    /// The proof identity.
    pub identity: Option<ProofIdentity>,
}

impl ProofReceipt {
    /// Returns `true` if this receipt is valid for a passing proof.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-007
    #[must_use]
    pub fn is_valid_pass(&self) -> bool {
        self.passed
            && self.exit_code.is_some()
            && self.teardown_receipt_created
            && self.proof_report_create_new
            && self.orphan_check_passed
            && self.identity.as_ref().is_some_and(ProofIdentity::is_valid)
    }
}

// ===========================================================================
//  Teardown receipt validation (REQ-PROOF-006)
// ===========================================================================

/// Validates that the teardown receipt is distinct from the proof report.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-006
#[must_use]
pub fn teardown_is_distinct(teardown_path: &str, proof_report_path: &str) -> bool {
    teardown_path != proof_report_path
}

/// Validates that the inactive teardown receipt is distinct from both the
/// active teardown receipt and the proof report.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-006
#[must_use]
pub fn inactive_teardown_is_distinct(
    inactive_teardown: &str,
    active_teardown: &str,
    proof_report: &str,
) -> bool {
    inactive_teardown != active_teardown
        && inactive_teardown != proof_report
        && active_teardown != proof_report
}

// ===========================================================================
//  Counter path validation (REQ-TRANSPORT-002)
// ===========================================================================

/// Validates that counter paths are distinct and non-substitutable.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-TRANSPORT-002
#[must_use]
pub fn counter_paths_are_distinct(
    inactive_counters: &str,
    active_trace: &str,
    teardown_receipt: &str,
) -> bool {
    inactive_counters != active_trace
        && inactive_counters != teardown_receipt
        && active_trace != teardown_receipt
}

// ===========================================================================
//  Proof run validation (REQ-PROOF-008)
// ===========================================================================

/// The result of validating a complete proof run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-008
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofValidationError {
    /// Preflight failed.
    PreflightFailed,
    /// Identity is invalid.
    InvalidIdentity,
    /// Teardown receipt missing.
    TeardownMissing,
    /// Proof report not create-new.
    ProofReportNotCreateNew,
    /// Orphan check failed.
    OrphanCheckFailed,
    /// Trace records missing or reordered.
    TraceError,
    /// Counter validation failed.
    CounterValidation,
    /// Socket not removed.
    SocketNotRemoved,
    /// Output not drained.
    OutputNotDrained,
    /// Pending ack remains.
    PendingAck,
}

/// Validates a complete proof run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-008
pub fn validate_proof_run(
    preflight: &PreflightCheck,
    receipt: &ProofReceipt,
    socket_removed: bool,
    output_drained: bool,
    no_pending_ack: bool,
    trace_valid: bool,
    counters_valid: bool,
) -> Result<(), ProofValidationError> {
    if !preflight.passes() {
        return Err(ProofValidationError::PreflightFailed);
    }
    if !receipt.is_valid_pass() {
        return Err(ProofValidationError::InvalidIdentity);
    }
    if !receipt.teardown_receipt_created {
        return Err(ProofValidationError::TeardownMissing);
    }
    if !receipt.proof_report_create_new {
        return Err(ProofValidationError::ProofReportNotCreateNew);
    }
    if !receipt.orphan_check_passed {
        return Err(ProofValidationError::OrphanCheckFailed);
    }
    if !socket_removed {
        return Err(ProofValidationError::SocketNotRemoved);
    }
    if !output_drained {
        return Err(ProofValidationError::OutputNotDrained);
    }
    if !no_pending_ack {
        return Err(ProofValidationError::PendingAck);
    }
    if !trace_valid {
        return Err(ProofValidationError::TraceError);
    }
    if !counters_valid {
        return Err(ProofValidationError::CounterValidation);
    }
    Ok(())
}

// ===========================================================================
//  Architecture review (REQ-ARCH-001..004)
// ===========================================================================

/// Architecture requirements status.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-ARCH-001..004
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchRequirementStatus {
    /// Requirement is OPEN (not yet met; architecture review only).
    Open,
    /// Requirement is MET.
    Met,
}

/// Architecture review status for the project.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-ARCH-001..004
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureReview {
    /// REQ-ARCH-001: Full Rust ownership of main loop.
    pub arch_001_full_rust_main_loop: ArchRequirementStatus,
    /// REQ-ARCH-002: No C code in tree.
    pub arch_002_no_c_code: ArchRequirementStatus,
    /// REQ-ARCH-003: Complete FFI elimination.
    pub arch_003_complete_ffi_elimination: ArchRequirementStatus,
    /// REQ-ARCH-004: Production-quality graphics driver.
    pub arch_004_production_graphics: ArchRequirementStatus,
}

impl ArchitectureReview {
    /// Create the default architecture review with all requirements OPEN.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-ARCH-001..004
    #[must_use]
    pub fn all_open() -> Self {
        Self {
            arch_001_full_rust_main_loop: ArchRequirementStatus::Open,
            arch_002_no_c_code: ArchRequirementStatus::Open,
            arch_003_complete_ffi_elimination: ArchRequirementStatus::Open,
            arch_004_production_graphics: ArchRequirementStatus::Open,
        }
    }

    /// Returns `true` if all architecture requirements are OPEN (honestly
    /// reported as not yet met).
    #[must_use]
    pub fn is_all_open(&self) -> bool {
        self.arch_001_full_rust_main_loop == ArchRequirementStatus::Open
            && self.arch_002_no_c_code == ArchRequirementStatus::Open
            && self.arch_003_complete_ffi_elimination == ArchRequirementStatus::Open
            && self.arch_004_production_graphics == ArchRequirementStatus::Open
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::child_session::ProofType;

    fn valid_identity() -> ProofIdentity {
        let d = "a".repeat(64);
        ProofIdentity {
            executable_digest: d.clone(),
            script_digest: d.clone(),
            content_digest: d.clone(),
            build_digest: d.clone(),
            initial_config_digest: d.clone(),
            final_config_digest: d,
        }
    }

    fn invalid_identity() -> ProofIdentity {
        ProofIdentity {
            executable_digest: "short".to_string(),
            script_digest: "short".to_string(),
            content_digest: "short".to_string(),
            build_digest: "short".to_string(),
            initial_config_digest: "short".to_string(),
            final_config_digest: "short".to_string(),
        }
    }

    // --- Proof identity (REQ-PROOF-003) ---

    #[test]
    fn valid_identity_passes() {
        assert!(valid_identity().is_valid());
    }

    #[test]
    fn invalid_identity_fails() {
        assert!(!invalid_identity().is_valid());
    }

    #[test]
    fn empty_identity_fails() {
        let empty = ProofIdentity {
            executable_digest: String::new(),
            script_digest: String::new(),
            content_digest: String::new(),
            build_digest: String::new(),
            initial_config_digest: String::new(),
            final_config_digest: String::new(),
        };
        assert!(!empty.is_valid());
    }

    // --- Preflight (REQ-PROOF-001) ---

    #[test]
    fn preflight_passes_all_good() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: true,
        };
        assert!(pf.passes());
    }

    #[test]
    fn preflight_fails_no_fresh_root() {
        let pf = PreflightCheck {
            fresh_root_created: false,
            no_matching_processes: true,
            identity_valid: true,
        };
        assert!(!pf.passes());
    }

    #[test]
    fn preflight_fails_matching_processes() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: false,
            identity_valid: true,
        };
        assert!(!pf.passes());
    }

    #[test]
    fn preflight_fails_invalid_identity() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: false,
        };
        assert!(!pf.passes());
    }

    // --- Proof receipt (REQ-PROOF-007) ---

    #[test]
    fn valid_pass_receipt() {
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert!(receipt.is_valid_pass());
    }

    #[test]
    fn receipt_fails_no_exit_code() {
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: None,
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert!(!receipt.is_valid_pass());
    }

    #[test]
    fn receipt_fails_no_teardown() {
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: false,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert!(!receipt.is_valid_pass());
    }

    #[test]
    fn receipt_fails_no_proof_report() {
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: false,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert!(!receipt.is_valid_pass());
    }

    #[test]
    fn receipt_fails_orphan_check() {
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: false,
            identity: Some(valid_identity()),
        };
        assert!(!receipt.is_valid_pass());
    }

    #[test]
    fn receipt_fails_invalid_identity() {
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(invalid_identity()),
        };
        assert!(!receipt.is_valid_pass());
    }

    #[test]
    fn receipt_fails_no_identity() {
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: None,
        };
        assert!(!receipt.is_valid_pass());
    }

    // --- Teardown distinctness (REQ-PROOF-006) ---

    #[test]
    fn teardown_distinct_from_proof_report() {
        assert!(teardown_is_distinct(
            "/run/teardown-complete.json",
            "/run/proof-report.json"
        ));
    }

    #[test]
    fn teardown_not_distinct_if_same_path() {
        assert!(!teardown_is_distinct("/run/same.json", "/run/same.json"));
    }

    #[test]
    fn inactive_teardown_distinct_from_all() {
        assert!(inactive_teardown_is_distinct(
            "/run/inactive-teardown-complete.json",
            "/run/teardown-complete.json",
            "/run/proof-report.json"
        ));
    }

    #[test]
    fn inactive_teardown_not_distinct_if_same_as_active() {
        assert!(!inactive_teardown_is_distinct(
            "/run/same.json",
            "/run/same.json",
            "/run/other.json"
        ));
    }

    // --- Counter path distinctness (REQ-TRANSPORT-002) ---

    #[test]
    fn counter_paths_distinct() {
        assert!(counter_paths_are_distinct(
            "/run/inactive-counters.jsonl",
            "/run/trace.jsonl",
            "/run/teardown-complete.json"
        ));
    }

    #[test]
    fn counter_paths_not_distinct_if_same() {
        assert!(!counter_paths_are_distinct(
            "/run/same.jsonl",
            "/run/same.jsonl",
            "/run/other.json"
        ));
    }

    // --- Proof run validation (REQ-PROOF-008) ---

    #[test]
    fn validate_proof_run_all_pass() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: true,
        };
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert!(validate_proof_run(&pf, &receipt, true, true, true, true, true).is_ok());
    }

    #[test]
    fn validate_proof_run_fails_preflight() {
        let pf = PreflightCheck {
            fresh_root_created: false,
            no_matching_processes: true,
            identity_valid: true,
        };
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert_eq!(
            validate_proof_run(&pf, &receipt, true, true, true, true, true),
            Err(ProofValidationError::PreflightFailed)
        );
    }

    #[test]
    fn validate_proof_run_fails_socket_not_removed() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: true,
        };
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert_eq!(
            validate_proof_run(&pf, &receipt, false, true, true, true, true),
            Err(ProofValidationError::SocketNotRemoved)
        );
    }

    #[test]
    fn validate_proof_run_fails_pending_ack() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: true,
        };
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert_eq!(
            validate_proof_run(&pf, &receipt, true, true, false, true, true),
            Err(ProofValidationError::PendingAck)
        );
    }

    #[test]
    fn validate_proof_run_fails_trace_error() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: true,
        };
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert_eq!(
            validate_proof_run(&pf, &receipt, true, true, true, false, true),
            Err(ProofValidationError::TraceError)
        );
    }

    #[test]
    fn validate_proof_run_fails_counter_validation() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: true,
        };
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert_eq!(
            validate_proof_run(&pf, &receipt, true, true, true, true, false),
            Err(ProofValidationError::CounterValidation)
        );
    }

    #[test]
    fn validate_proof_run_fails_output_not_drained() {
        let pf = PreflightCheck {
            fresh_root_created: true,
            no_matching_processes: true,
            identity_valid: true,
        };
        let receipt = ProofReceipt {
            proof_type: ProofType::MainMenu,
            passed: true,
            exit_code: Some(0),
            teardown_receipt_created: true,
            proof_report_create_new: true,
            orphan_check_passed: true,
            identity: Some(valid_identity()),
        };
        assert_eq!(
            validate_proof_run(&pf, &receipt, true, false, true, true, true),
            Err(ProofValidationError::OutputNotDrained)
        );
    }

    // --- Architecture review (REQ-ARCH-001..004) ---

    #[test]
    fn architecture_review_all_open() {
        let review = ArchitectureReview::all_open();
        assert!(review.is_all_open());
    }

    #[test]
    fn architecture_review_not_all_open_if_one_met() {
        let mut review = ArchitectureReview::all_open();
        review.arch_001_full_rust_main_loop = ArchRequirementStatus::Met;
        assert!(!review.is_all_open());
    }
}
