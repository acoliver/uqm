//! Typed machine-readable CI command authority.
//!
//! `rust/ci/gates.json` is the single checked-in source of truth for gate ids,
//! owners, exact command vectors, feature profiles, and thresholds. Both the gate
//! executor and the mutation suite deserialize this file and validate it against the
//! fixed contract shapes below; no other code declares a gate command vector.
//!
//! Supported-tuple identity and runner mapping come from `rust/ci/gates.json`.
//! `rust/build/supported-matrix.json` remains a compatibility input and must derive
//! exactly the authority tuple set before a plan can be emitted.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

pub const AUTHORITY_RELATIVE: &str = "rust/ci/gates.json";
pub const AUTHORITY_SCHEMA: &str = "uqm-s4-ci-authority-v1";
pub const MATRIX_RELATIVE: &str = "rust/build/supported-matrix.json";
pub const MATRIX_SCHEMA: &str = "uqm-supported-matrix-v1";

/// Mutation implementations supported by the controller. Ordering comes only
/// from `Authority::mutation_targets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MutationTarget {
    Format,
    Check,
    Clippy,
    Test,
    Ownership,
    Link,
    Harness,
    Complexity,
    Security,
    Coverage,
    Cache,
    Workflow,
    Artifact,
}

impl MutationTarget {
    pub const COUNT: usize = 13;

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "format" => Some(Self::Format),
            "check" => Some(Self::Check),
            "clippy" => Some(Self::Clippy),
            "test" => Some(Self::Test),
            "ownership" => Some(Self::Ownership),
            "link" => Some(Self::Link),
            "harness" => Some(Self::Harness),
            "complexity" => Some(Self::Complexity),
            "security" => Some(Self::Security),
            "coverage" => Some(Self::Coverage),
            "cache" => Some(Self::Cache),
            "workflow" => Some(Self::Workflow),
            "artifact" => Some(Self::Artifact),
            _ => None,
        }
    }

    pub fn contract(self) -> &'static str {
        match self {
            Self::Format => "mutations.format.rejects_unformatted",
            Self::Check => "mutations.check.rejects_compile_error",
            Self::Clippy => "mutations.clippy.rejects_warning",
            Self::Test => "mutations.test.rejects_failing_test",
            Self::Ownership => "mutations.ownership.rejects_authority_drift",
            Self::Link => "mutations.link.rejects_duplicate_provider",
            Self::Harness => "mutations.harness.rejects_missing_marker",
            Self::Complexity => "mutations.complexity.rejects_over_limit",
            Self::Security => "mutations.security.rejects_missing_deny_warnings",
            Self::Coverage => "mutations.coverage.rejects_below_floor",
            Self::Cache => "mutations.cache.rejects_prepopulated_registry",
            Self::Workflow => "mutations.workflow.rejects_trust_boundary_weakening",
            Self::Artifact => "mutations.artifact.rejects_coherently_rehashed_provenance_forgery",
        }
    }
}

/// Builtin gate ids that execute inside Rust rather than a raw argv.
pub const BUILTIN_GATES: [&str; 5] = [
    "complexity",
    "coverage",
    "bootstrap-proof",
    "workflow",
    "mutations",
];

/// Exact merge-deciding gate sequence. Order is part of the S4 authority contract.
pub const MANDATORY_GATE_IDS: [&str; 13] = [
    "format",
    "check",
    "clippy",
    "tests",
    "ownership-link",
    "probes-harnesses",
    "complexity",
    "security",
    "coverage",
    "package",
    "bootstrap-proof",
    "workflow",
    "mutations",
];

/// Cache modes the authority and evidence index may declare.
pub const CACHE_MODES: [&str; 2] = ["isolated-empty", "ambient-dev"];

/// Supported matrix rows parsed from `rust/build/supported-matrix.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matrix {
    pub schema: String,
    pub axes: MatrixAxes,
    pub supported: Vec<SupportedRow>,
    pub unsupported_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixAxes {
    pub os: Vec<String>,
    pub architecture: Vec<String>,
    pub renderer: Vec<String>,
    pub input: Vec<String>,
    pub content: Vec<String>,
    pub audio: Vec<String>,
    pub network: Vec<String>,
    pub package: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportedRow {
    pub os: String,
    pub architectures: Vec<String>,
    pub renderer: String,
    pub input: String,
    pub content: String,
    pub audio: String,
    pub network: String,
    pub package: String,
    pub prerequisites: Vec<String>,
}

const MATRIX_UNSUPPORTED_POLICY: &str = "Any tuple not represented by one supported row fails before compilation with all requested dimensions in the diagnostic.";

impl Matrix {
    /// Validate the compatibility matrix's complete fixed vocabulary.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != MATRIX_SCHEMA {
            return Err(format!("unsupported matrix schema '{}'", self.schema));
        }
        validate_axis("os", &self.axes.os, &["macos", "linux"])?;
        validate_axis(
            "architecture",
            &self.axes.architecture,
            &["aarch64", "x86_64"],
        )?;
        validate_axis("renderer", &self.axes.renderer, &["sdl2-software"])?;
        validate_axis("input", &self.axes.input, &["sdl2"])?;
        validate_axis("content", &self.axes.content, &["uqm-content-v0.8"])?;
        validate_axis("audio", &self.axes.audio, &["cpal", "cpal-alsa"])?;
        validate_axis("network", &self.axes.network, &["full"])?;
        validate_axis("package", &self.axes.package, &["directory-manifest"])?;
        if self.unsupported_policy != MATRIX_UNSUPPORTED_POLICY {
            return Err(
                "supported matrix unsupported_policy differs from its fixed contract".into(),
            );
        }
        if self.supported.len() != 2 {
            return Err(
                "supported matrix must contain exactly one row per operating system".into(),
            );
        }
        for row in &self.supported {
            validate_row(row, &self.axes)?;
        }
        Ok(())
    }

    /// The sorted tuple set this compatibility matrix yields.
    pub fn tuples(&self) -> BTreeSet<String> {
        self.supported
            .iter()
            .flat_map(|row| {
                row.architectures
                    .iter()
                    .map(move |architecture| format!("{}-{}", row.os, architecture))
            })
            .collect()
    }

    pub fn derive_contract_tuples(&self) -> Result<Vec<String>, String> {
        self.validate()?;
        let derived = self.tuples();
        let declared_count: usize = self
            .supported
            .iter()
            .map(|row| row.architectures.len())
            .sum();
        if derived.len() != declared_count {
            return Err("supported matrix contains a duplicate os/architecture tuple".into());
        }
        Ok(derived.into_iter().collect())
    }
}

fn validate_axis(name: &str, values: &[String], expected: &[&str]) -> Result<(), String> {
    let actual: BTreeSet<&str> = values.iter().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected || values.len() != expected.len() {
        return Err(format!(
            "matrix axis '{name}' differs from its fixed safe values"
        ));
    }
    Ok(())
}

fn safe_matrix_token(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    for byte in value.bytes() {
        if !byte.is_ascii_alphanumeric() && !matches!(byte, b'-' | b'_' | b'+' | b'.') {
            return false;
        }
    }
    true
}

fn valid_runner_mapping(mapping: &RunnerMapping) -> bool {
    let tuple = format!("{}-{}", mapping.os, mapping.architecture);
    tuple == mapping.tuple
        && matches!(mapping.os.as_str(), "linux" | "macos")
        && matches!(mapping.architecture.as_str(), "aarch64" | "x86_64")
        && matches!(
            mapping.runner.as_str(),
            "macos-15" | "macos-15-intel" | "ubuntu-24.04-arm" | "ubuntu-24.04"
        )
        && matches!(
            mapping.expected_uname.as_str(),
            "arm64" | "aarch64" | "x86_64"
        )
        && matches!(
            (
                mapping.tuple.as_str(),
                mapping.runner.as_str(),
                mapping.expected_uname.as_str(),
            ),
            ("macos-aarch64", "macos-15", "arm64")
                | ("macos-x86_64", "macos-15-intel", "x86_64")
                | ("linux-aarch64", "ubuntu-24.04-arm", "aarch64")
                | ("linux-x86_64", "ubuntu-24.04", "x86_64")
        )
        && [
            mapping.os.as_str(),
            mapping.architecture.as_str(),
            mapping.tuple.as_str(),
            mapping.runner.as_str(),
            mapping.expected_uname.as_str(),
        ]
        .into_iter()
        .all(safe_matrix_token)
}

fn expected_gate_owner(gate_id: &str) -> Option<&'static str> {
    match gate_id {
        "format" | "check" | "clippy" | "complexity" | "security" | "coverage" | "workflow"
        | "mutations" => Some("S4"),
        "tests" | "probes-harnesses" | "bootstrap-proof" => Some("S3"),
        "ownership-link" => Some("S1"),
        "package" => Some("S2"),
        _ => None,
    }
}

fn validate_row(row: &SupportedRow, axes: &MatrixAxes) -> Result<(), String> {
    let dimensions = [
        (&row.renderer, &axes.renderer),
        (&row.input, &axes.input),
        (&row.content, &axes.content),
        (&row.audio, &axes.audio),
        (&row.network, &axes.network),
        (&row.package, &axes.package),
    ];
    let expected_audio = match row.os.as_str() {
        "macos" => "cpal",
        "linux" => "cpal-alsa",
        _ => return Err(format!("unsupported matrix operating system '{}'", row.os)),
    };
    if row.architectures.len() != 2
        || row.architectures.iter().collect::<BTreeSet<_>>().len() != row.architectures.len()
        || row
            .architectures
            .iter()
            .any(|value| !axes.architecture.contains(value))
        || dimensions.iter().any(|(value, axis)| !axis.contains(value))
        || row.renderer != "sdl2-software"
        || row.input != "sdl2"
        || row.content != "uqm-content-v0.8"
        || row.audio != expected_audio
        || row.network != "full"
        || row.package != "directory-manifest"
        || row.prerequisites.is_empty()
        || row.prerequisites.iter().collect::<BTreeSet<_>>().len() != row.prerequisites.len()
        || row
            .prerequisites
            .iter()
            .any(|value| !safe_matrix_token(value))
    {
        return Err(format!(
            "contradictory or unsafe supported matrix row for {}",
            row.os
        ));
    }
    Ok(())
}

/// Validate the compatibility matrix and return the authority-owned tuple set.
pub fn derive_supported_tuples(root: &Path, authority: &Authority) -> Result<Vec<String>, String> {
    let path = root.join(&authority.matrix_file);
    let bytes = super::bounded_io::read_regular_nofollow(
        &path,
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    let matrix: Matrix = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let compatibility = matrix.derive_contract_tuples()?;
    let authority_tuples = authority.supported_tuples();
    if compatibility != authority_tuples {
        return Err(format!(
            "compatibility matrix tuple set differs from authority: {compatibility:?} vs {authority_tuples:?}"
        ));
    }
    Ok(authority_tuples)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub schema: String,
    pub ledger_identity: LedgerIdentity,
    pub zero_native_delta: ZeroNativeDelta,
    pub control_plane_paths: Vec<String>,
    pub tools: ToolsAuthority,
    pub actions: ActionsAuthority,
    pub workflow: WorkflowAuthority,
    pub evidence_roles: Vec<EvidenceRole>,
    pub matrix_file: String,
    pub runner_mapping: Vec<RunnerMapping>,
    pub complexity: ComplexityAuthority,
    pub coverage: CoverageAuthority,
    pub security: SecurityAuthority,
    pub cache: CacheAuthority,
    pub supervision: SupervisionAuthority,
    pub native_acceptance: NativeAcceptanceAuthority,
    pub package: PackageAuthority,
    pub bootstrap_proof: BootstrapProofAuthority,
    pub profiles: Profiles,
    pub gates: Vec<Gate>,
    pub mutation_targets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LedgerIdentity {
    pub schema: String,
    pub assessment_commit: String,
    pub history_revision: String,
    pub raw_revision: String,
    pub sha256: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ZeroNativeDelta {
    pub tracked_sources: u32,
    pub providers: u32,
    pub objects: u32,
    pub internal_symbols: u32,
    pub bridges: u32,
    pub generated_bindings: u32,
    pub transitional_flags: u32,
    pub maximum_transitional_native_inputs: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DistributionRequirement {
    pub requirement: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolIdentity {
    pub version: String,
    pub installation_integrity: String,
    pub integrity_identity: String,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default)]
    pub distribution_requirements: Vec<DistributionRequirement>,
    pub version_command: Vec<String>,
    pub expected_output_prefix: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolProbe {
    pub name: String,
    pub version_command: Vec<String>,
    pub expected_output_prefix: Option<String>,
    pub accepted_exit_codes: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativePrerequisites {
    pub linux: Vec<String>,
    pub macos: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolsAuthority {
    pub preflight: Vec<ToolProbe>,
    pub native_prerequisites: NativePrerequisites,
    pub rust: ToolIdentity,
    pub lizard: ToolIdentity,
    pub cargo_audit: ToolIdentity,
    pub cargo_llvm_cov: ToolIdentity,
    pub actionlint: ToolIdentity,
}

impl ToolsAuthority {
    pub fn entries(&self) -> [(&str, &ToolIdentity); 5] {
        [
            ("rust", &self.rust),
            ("lizard", &self.lizard),
            ("cargo-audit", &self.cargo_audit),
            ("cargo-llvm-cov", &self.cargo_llvm_cov),
            ("actionlint", &self.actionlint),
        ]
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActionsAuthority {
    pub checkout: String,
    pub upload_artifact: String,
    pub artifact_retention_days: u16,
    pub transport_member_limit_bytes: u64,
    pub evidence_snapshot_member_count_limit: u32,
    pub evidence_snapshot_member_limit_bytes: u64,
    pub evidence_snapshot_aggregate_limit_bytes: u64,
    pub evidence_snapshot_path_limit_bytes: u32,
    pub evidence_snapshot_aggregate_path_limit_bytes: u64,
    pub github_api_connect_timeout_seconds: u16,
    pub github_api_total_timeout_seconds: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAuthority {
    pub plan_job_timeout_minutes: u16,
    pub gates_job_timeout_minutes: u16,
    pub required_gates_job_timeout_minutes: u16,
    pub bootstrap_authority_retry_limit: u16,
    pub bootstrap_authority_retry_delay_seconds: u16,
    pub bootstrap_authority_connect_timeout_seconds: u16,
    pub bootstrap_authority_total_timeout_seconds: u16,
    pub bootstrap_authority_response_limit_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRole {
    pub role: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunnerMapping {
    pub os: String,
    pub architecture: String,
    pub tuple: String,
    pub runner: String,
    pub expected_uname: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComplexityAuthority {
    pub maximum: u32,
    pub source_roots: Vec<String>,
    pub lizard_arguments: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageAuthority {
    pub minimum_line_percent: f64,
    pub ignore_filename_regex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SecurityAuthority {
    pub advisory_database_repository: String,
    pub advisory_database_revision: String,
    pub advisory_database_path: String,
    pub advisory_database_pack_sha256: String,
    pub advisory_database_file_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CacheAuthority {
    pub mode: String,
    pub require_rust_target_absent: bool,
    pub require_sc2_obj_absent: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SupervisionAuthority {
    pub builtin_timeout_seconds: u64,
    pub aggregate_run_timeout_seconds: u64,
    pub termination_grace_milliseconds: u64,
    pub pipe_drain_timeout_milliseconds: u64,
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
    pub executable_member_limit_bytes: u64,
}

impl SupervisionAuthority {
    pub fn limits(&self, timeout_seconds: u64) -> super::exec::Limits {
        super::exec::Limits {
            timeout: std::time::Duration::from_secs(timeout_seconds),
            termination_grace: std::time::Duration::from_millis(
                self.termination_grace_milliseconds,
            ),
            pipe_drain_timeout: std::time::Duration::from_millis(
                self.pipe_drain_timeout_milliseconds,
            ),
            stdout_bytes: self.stdout_limit_bytes,
            stderr_bytes: self.stderr_limit_bytes,
            executable_bytes: self.executable_member_limit_bytes,
        }
    }

    pub fn builtin_limits(&self) -> super::exec::Limits {
        self.limits(self.builtin_timeout_seconds)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeRuntimeAuthority {
    pub capture_timeout_ms: u64,
    pub capture_kill_grace_ms: u64,
    pub observer_timeout_ms: u64,
    pub observer_kill_grace_ms: u64,
    pub acknowledgement_timeout_ms: u64,
    pub outer_child_timeout_ms: u64,
    pub outer_child_kill_grace_ms: u64,
    pub child_stdout_budget_bytes: u64,
    pub child_stderr_budget_bytes: u64,
    pub observer_response_budget_bytes: u64,
    pub capture_budget_bytes: u64,
    pub content_expansion_budget_bytes: u64,
    pub expected_client_bounds: uqm_rust::automation::native_window::NativeWindowBounds,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeContentTransportAuthority {
    pub attempt_limit: u16,
    pub read_timeout_seconds: u16,
    pub backoff_seconds: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeAcceptanceAuthority {
    pub platform: String,
    pub dedicated_execution_uid: u32,
    pub content_url: String,
    pub content_filename: String,
    pub content_sha256: String,
    pub content_byte_length: u64,
    pub content_version: String,
    pub content_transport: NativeContentTransportAuthority,
    pub script: String,
    pub script_sha256: String,
    pub script_byte_length: u64,
    pub acceptance_policy: uqm_rust::automation::native_window::NativeAcceptancePolicy,
    pub runtime_contract: NativeRuntimeAuthority,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageAuthority {
    pub manifest_schema: String,
    pub producing_command: String,
    pub profile: String,
    pub cargo_manifest_sha256: String,
    pub cargo_lock_sha256: String,
    pub features: Vec<String>,
    pub artifacts: Vec<PackageArtifactAuthority>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageArtifactAuthority {
    pub role: String,
    pub media_type: String,
    pub producing_command: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapProofAuthority {
    pub profile: String,
    pub profile_sha256: String,
    pub packaged_root: String,
    pub packaged_executable: String,
    pub packaged_manifest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profiles {
    pub pure_test: Vec<String>,
    pub linked_test: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub id: String,
    pub owner: String,
    pub kind: GateKind,
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    pub cwd: String,
    pub command: Vec<String>,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub native_profile: Option<String>,
}

impl Authority {
    pub fn gate(&self, id: &str) -> Option<&Gate> {
        self.gates.iter().find(|gate| gate.id == id)
    }

    pub fn supported_tuples(&self) -> Vec<String> {
        self.runner_mapping
            .iter()
            .map(|mapping| mapping.tuple.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn gate_ids(&self) -> Vec<&str> {
        self.gates.iter().map(|gate| gate.id.as_str()).collect()
    }

    pub fn native_runtime_contract(
        &self,
    ) -> uqm_rust::automation::native_window::NativeWindowRuntimeContract {
        let runtime = self.native_acceptance.runtime_contract;
        uqm_rust::automation::native_window::NativeWindowRuntimeContract {
            capture_timeout_ms: runtime.capture_timeout_ms,
            capture_kill_grace_ms: runtime.capture_kill_grace_ms,
            observer_timeout_ms: runtime.observer_timeout_ms,
            observer_kill_grace_ms: runtime.observer_kill_grace_ms,
            acknowledgement_timeout_ms: runtime.acknowledgement_timeout_ms,
            outer_child_timeout_ms: runtime.outer_child_timeout_ms,
            outer_child_kill_grace_ms: runtime.outer_child_kill_grace_ms,
            child_stdout_budget_bytes: runtime.child_stdout_budget_bytes,
            child_stderr_budget_bytes: runtime.child_stderr_budget_bytes,
            observer_response_budget_bytes: runtime.observer_response_budget_bytes,
            capture_budget_bytes: runtime.capture_budget_bytes,
            content_expansion_budget_bytes: runtime.content_expansion_budget_bytes,
            inventory_limits: uqm_rust::automation::native_window::NativeInventoryLimits {
                member_count: self.actions.evidence_snapshot_member_count_limit,
                member_bytes: self.actions.evidence_snapshot_member_limit_bytes,
                aggregate_bytes: self.actions.evidence_snapshot_aggregate_limit_bytes,
                path_bytes: self.actions.evidence_snapshot_path_limit_bytes,
                aggregate_path_bytes: self.actions.evidence_snapshot_aggregate_path_limit_bytes,
            },
            expected_client_bounds: runtime.expected_client_bounds,
        }
    }

    /// Explicit canonical toolchain environment for linked-test gate steps.
    ///
    /// Matches the production/linked build path: SOURCE_DATE_EPOCH,
    /// UQM_BUILD_DATE, canonical tool selection, and the
    /// UQM_CANONICAL_TOOLCHAIN marker are applied to the executing process and
    /// returned as explicit child overrides.
    pub fn linked_step_env(&self) -> Result<Vec<(String, String)>, String> {
        let root = crate::repository_root()?;
        crate::prepare_source_environment(&root)?;
        let toolchain = crate::canonical_toolchain(&root)?;
        crate::prepare_canonical_build(&toolchain)?;
        let mut vars = vec![
            (
                "SOURCE_DATE_EPOCH".into(),
                crate::source_date_epoch(&root)?.to_string(),
            ),
            ("UQM_BUILD_DATE".into(), crate::source_date(&root)?),
            ("UQM_NATIVE_PROFILE".into(), "linked-test".into()),
        ];
        vars.extend(crate::canonical_toolchain_subprocess_environment(
            &toolchain,
        )?);
        Ok(vars)
    }
}

/// Resolve `GateKind` from the authority JSON string form.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum GateKind {
    #[serde(rename = "process")]
    Process,
    #[serde(rename = "builtin")]
    Builtin,
}

fn runtime_authority_path(
    root: &Path,
    source_bound: bool,
    configured: Option<std::ffi::OsString>,
) -> Result<std::path::PathBuf, String> {
    match configured {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            if !path.is_absolute() {
                return Err("UQM_CI_AUTHORITY_PATH must be an absolute path".into());
            }
            Ok(path)
        }
        None if source_bound => Err(
            "UQM_CI_AUTHORITY_PATH is required when UQM_CI_SOURCE_ROOT binds runtime source".into(),
        ),
        None => Ok(root.join(AUTHORITY_RELATIVE)),
    }
}

/// Load the trusted runtime authority and its exact JSON contract.
pub fn load_authority_contract(root: &Path) -> Result<(Authority, serde_json::Value), String> {
    let path = runtime_authority_path(
        root,
        std::env::var_os("UQM_CI_SOURCE_ROOT").is_some(),
        std::env::var_os("UQM_CI_AUTHORITY_PATH"),
    )?;
    let bytes = super::bounded_io::read_regular_nofollow(
        &path,
        super::bounded_io::AUTHORITY_BOOTSTRAP_LIMIT_BYTES,
    )?;
    let authority = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid authority {}: {error}", path.display()))?;
    let contract = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid authority {}: {error}", path.display()))?;
    Ok((authority, contract))
}

/// Load the trusted runtime authority, or the checked-in authority outside CI.
pub fn load_authority(root: &Path) -> Result<Authority, String> {
    load_authority_contract(root).map(|(authority, _)| authority)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn safe_relative(value: &str) -> bool {
    let path = Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn safe_content_filename(value: &str) -> bool {
    let mut components = Path::new(value).components();
    matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
        && value.ends_with(".uqm")
}

fn valid_action_identity(value: &str) -> bool {
    let Some((repository, revision)) = value.rsplit_once('@') else {
        return false;
    };
    if repository.contains('@') {
        return false;
    }
    let mut repository_parts = repository.split('/');
    repository_parts.next().is_some_and(|part| !part.is_empty())
        && repository_parts.next().is_some_and(|part| !part.is_empty())
        && repository_parts.next().is_none()
        && valid_revision(revision)
}

/// Validate the authority against the fixed contract shapes.
pub fn validate_authority(authority: &Authority) -> Result<(), String> {
    if authority.schema != AUTHORITY_SCHEMA {
        return Err(format!(
            "unsupported authority schema '{}'",
            authority.schema
        ));
    }
    let ledger = &authority.ledger_identity;
    if ledger.schema.is_empty()
        || !valid_revision(&ledger.assessment_commit)
        || !valid_revision(&ledger.history_revision)
        || !valid_revision(&ledger.raw_revision)
        || !valid_sha256(&ledger.sha256)
        || !ledger.url.starts_with("https://")
    {
        return Err("authority ownership ledger identity is invalid".into());
    }
    let delta = &authority.zero_native_delta;
    if delta.tracked_sources
        + delta.providers
        + delta.objects
        + delta.internal_symbols
        + delta.bridges
        + delta.generated_bindings
        + delta.transitional_flags
        != 0
    {
        return Err("authority S4 native ownership delta must be zero".into());
    }
    let actual_paths: BTreeSet<_> = authority
        .control_plane_paths
        .iter()
        .map(String::as_str)
        .collect();
    if actual_paths.is_empty()
        || actual_paths.len() != authority.control_plane_paths.len()
        || actual_paths.iter().any(|path| !safe_relative(path))
    {
        return Err("authority control-plane paths must be nonempty, unique, and relative".into());
    }
    let preflight_names: BTreeSet<_> = authority
        .tools
        .preflight
        .iter()
        .map(|probe| probe.name.as_str())
        .collect();
    if authority.tools.preflight.is_empty()
        || preflight_names.len() != authority.tools.preflight.len()
        || authority.tools.preflight.iter().any(|probe| {
            probe.name.is_empty()
                || probe.version_command.is_empty()
                || probe.version_command.iter().any(String::is_empty)
                || probe.accepted_exit_codes.is_empty()
        })
    {
        return Err("authority preflight probes must be unique and executable".into());
    }
    for (platform, packages) in [
        ("linux", &authority.tools.native_prerequisites.linux),
        ("macos", &authority.tools.native_prerequisites.macos),
    ] {
        if packages.is_empty()
            || packages.windows(2).any(|pair| pair[0] >= pair[1])
            || packages.iter().any(|package| {
                package.is_empty()
                    || !package.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'+' | b'.')
                    })
            })
        {
            return Err(format!(
                "authority {platform} native prerequisites must be nonempty, sorted, unique package names"
            ));
        }
    }
    for (name, tool) in authority.tools.entries() {
        if tool.version.is_empty()
            || tool.version_command.is_empty()
            || tool.version_command.iter().any(String::is_empty)
            || tool.expected_output_prefix.is_empty()
            || tool.distribution_requirements.iter().any(|requirement| {
                requirement.requirement.is_empty()
                    || requirement.requirement.bytes().any(|byte| {
                        !byte.is_ascii_alphanumeric() && !matches!(byte, b'-' | b'_' | b'.' | b'=')
                    })
                    || !is_lower_hex(&requirement.sha256, 64)
            })
        {
            return Err(format!("authority tool identity '{name}' is incomplete"));
        }
    }
    let integrity_valid = authority.tools.rust.installation_integrity == "rustup-release-commit"
        && is_lower_hex(&authority.tools.rust.integrity_identity, 40)
        && authority.tools.lizard.installation_integrity == "pip-require-hashes"
        && authority.tools.lizard.integrity_identity == "embedded-distribution-requirements"
        && authority.tools.cargo_audit.installation_integrity == "cargo-registry-sha256"
        && is_lower_hex(&authority.tools.cargo_audit.integrity_identity, 64)
        && authority.tools.cargo_llvm_cov.installation_integrity == "cargo-registry-sha256"
        && is_lower_hex(&authority.tools.cargo_llvm_cov.integrity_identity, 64)
        && authority.tools.actionlint.installation_integrity == "github-release-sha256"
        && is_lower_hex(&authority.tools.actionlint.integrity_identity, 64);
    if !integrity_valid {
        return Err("authority tool installation integrity identities are incomplete".into());
    }
    let actionlint_distributions: BTreeSet<_> = authority
        .tools
        .actionlint
        .distribution_requirements
        .iter()
        .map(|requirement| requirement.requirement.as_str())
        .collect();
    if actionlint_distributions
        != BTreeSet::from(["darwin-amd64", "darwin-arm64", "linux-amd64", "linux-arm64"])
        || authority.tools.actionlint.distribution_requirements.len() != 4
    {
        return Err(
            "authority actionlint identity must pin exactly the four supported release archives"
                .into(),
        );
    }
    if authority.tools.lizard.distribution_requirements.is_empty()
        || !authority
            .tools
            .lizard
            .distribution_requirements
            .iter()
            .any(|requirement| {
                requirement.requirement == format!("lizard=={}", authority.tools.lizard.version)
            })
    {
        return Err(
            "authority lizard identity must pin its complete hashed distribution set".into(),
        );
    }
    let rust_components: BTreeSet<_> = authority
        .tools
        .rust
        .components
        .iter()
        .map(String::as_str)
        .collect();
    if rust_components.is_empty()
        || rust_components.len() != authority.tools.rust.components.len()
        || rust_components.iter().any(|component| component.is_empty())
    {
        return Err("authority Rust components must be nonempty and unique".into());
    }
    if !valid_action_identity(&authority.actions.checkout)
        || !valid_action_identity(&authority.actions.upload_artifact)
    {
        return Err(
            "authority actions must use owner/repository@40-lowercase-hex identities".into(),
        );
    }
    if authority.actions.artifact_retention_days == 0
        || authority.actions.transport_member_limit_bytes == 0
        || authority.actions.evidence_snapshot_member_count_limit == 0
        || authority.actions.evidence_snapshot_member_limit_bytes == 0
        || authority.actions.evidence_snapshot_aggregate_limit_bytes
            < authority.actions.evidence_snapshot_member_limit_bytes
        || authority.actions.evidence_snapshot_path_limit_bytes == 0
        || authority
            .actions
            .evidence_snapshot_aggregate_path_limit_bytes
            < u64::from(authority.actions.evidence_snapshot_path_limit_bytes)
        || authority.actions.github_api_connect_timeout_seconds == 0
        || authority.actions.github_api_total_timeout_seconds
            <= authority.actions.github_api_connect_timeout_seconds
    {
        return Err("actions transport and upload bounds are invalid".into());
    }
    let workflow = &authority.workflow;
    if !(1..=360).contains(&workflow.plan_job_timeout_minutes)
        || !(1..=360).contains(&workflow.gates_job_timeout_minutes)
        || !(1..=360).contains(&workflow.required_gates_job_timeout_minutes)
        || !(1..=10).contains(&workflow.bootstrap_authority_retry_limit)
        || !(1..=60).contains(&workflow.bootstrap_authority_retry_delay_seconds)
        || !(1..=300).contains(&workflow.bootstrap_authority_connect_timeout_seconds)
        || !(1..=600).contains(&workflow.bootstrap_authority_total_timeout_seconds)
        || workflow.bootstrap_authority_total_timeout_seconds
            <= workflow.bootstrap_authority_connect_timeout_seconds
        || !(1..=16_777_216).contains(&workflow.bootstrap_authority_response_limit_bytes)
    {
        return Err(
            "authority workflow runtime and bootstrap transport budgets are invalid".into(),
        );
    }
    let actual_roles: BTreeSet<_> = authority
        .evidence_roles
        .iter()
        .map(|entry| (entry.role.as_str(), entry.media_type.as_str()))
        .collect();
    if actual_roles.is_empty()
        || actual_roles.len() != authority.evidence_roles.len()
        || actual_roles
            .iter()
            .any(|(role, media)| role.is_empty() || media.is_empty())
    {
        return Err("authority evidence role/media contracts must be unique and nonempty".into());
    }
    if authority.matrix_file != MATRIX_RELATIVE {
        return Err("authority matrix_file must name the fixed compatibility input".into());
    }
    let runner_tuples: BTreeSet<_> = authority
        .runner_mapping
        .iter()
        .map(|mapping| mapping.tuple.as_str())
        .collect();
    let expected_tuples: BTreeSet<_> = [
        "linux-aarch64",
        "linux-x86_64",
        "macos-aarch64",
        "macos-x86_64",
    ]
    .into_iter()
    .collect();
    if authority.runner_mapping.len() != 4
        || runner_tuples != expected_tuples
        || authority
            .runner_mapping
            .iter()
            .any(|mapping| !valid_runner_mapping(mapping))
    {
        return Err(
            "authority runner_mapping must declare the exact four safe tuple/runner mappings"
                .into(),
        );
    }
    let mutation_targets = authority
        .mutation_targets
        .iter()
        .map(|target| {
            MutationTarget::parse(target)
                .ok_or_else(|| format!("authority has unsupported mutation target '{target}'"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if mutation_targets.len() != MutationTarget::COUNT
        || authority.mutation_targets.len() != MutationTarget::COUNT
    {
        return Err("authority mutation_targets must declare every supported target once".into());
    }
    let profile_features = authority
        .profiles
        .pure_test
        .iter()
        .chain(&authority.profiles.linked_test)
        .collect::<BTreeSet<_>>();
    if authority.profiles.pure_test.is_empty()
        || authority.profiles.linked_test.is_empty()
        || authority
            .profiles
            .pure_test
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != authority.profiles.pure_test.len()
        || authority
            .profiles
            .linked_test
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != authority.profiles.linked_test.len()
        || profile_features.iter().any(|feature| feature.is_empty())
    {
        return Err("authority test profiles must contain unique nonempty features".into());
    }
    if authority.complexity.maximum == 0
        || authority.complexity.source_roots.is_empty()
        || authority
            .complexity
            .source_roots
            .iter()
            .any(|root| !safe_relative(root))
        || authority.complexity.lizard_arguments.is_empty()
        || authority
            .complexity
            .lizard_arguments
            .iter()
            .any(String::is_empty)
    {
        return Err("authority complexity configuration is incomplete or unsafe".into());
    }
    if !authority.coverage.minimum_line_percent.is_finite()
        || !(0.0..=100.0).contains(&authority.coverage.minimum_line_percent)
        || authority.coverage.ignore_filename_regex.is_empty()
    {
        return Err("authority coverage configuration is invalid".into());
    }
    if authority.cache.mode != CACHE_MODES[0] {
        return Err("authority cache mode must be 'isolated-empty'".into());
    }
    let supervision = &authority.supervision;
    if supervision.builtin_timeout_seconds == 0
        || supervision.aggregate_run_timeout_seconds <= supervision.builtin_timeout_seconds
        || supervision.aggregate_run_timeout_seconds
            >= u64::from(authority.workflow.gates_job_timeout_minutes) * 60
        || supervision.termination_grace_milliseconds == 0
        || supervision.pipe_drain_timeout_milliseconds == 0
        || supervision.stdout_limit_bytes == 0
        || supervision.stderr_limit_bytes == 0
        || supervision.executable_member_limit_bytes == 0
    {
        return Err("authority subprocess supervision limits must be nonzero".into());
    }
    let native = &authority.native_acceptance;
    let content_transport = &native.content_transport;
    if native.dedicated_execution_uid == 0
        || native.platform != "macos"
        || !native.content_url.starts_with("https://")
        || !safe_content_filename(&native.content_filename)
        || !valid_sha256(&native.content_sha256)
        || native.content_byte_length == 0
        || native.content_version.is_empty()
        || !(1..=10).contains(&content_transport.attempt_limit)
        || !(1..=600).contains(&content_transport.read_timeout_seconds)
        || content_transport.backoff_seconds.len()
            != usize::from(content_transport.attempt_limit - 1)
        || content_transport
            .backoff_seconds
            .iter()
            .any(|seconds| !(1..=300).contains(seconds))
        || !safe_relative(&native.script)
        || !valid_sha256(&native.script_sha256)
        || native.script_byte_length == 0
        || !native.acceptance_policy.is_valid()
        || !authority
            .native_runtime_contract()
            .has_valid_deadline_order()
    {
        return Err("authority native acceptance configuration is invalid".into());
    }
    let package = &authority.package;
    let artifact_roles: BTreeSet<_> = package
        .artifacts
        .iter()
        .map(|artifact| artifact.role.as_str())
        .collect();
    if package.manifest_schema.is_empty()
        || package.producing_command.is_empty()
        || package.profile.is_empty()
        || !valid_sha256(&package.cargo_manifest_sha256)
        || !valid_sha256(&package.cargo_lock_sha256)
        || package.features.is_empty()
        || package.features.iter().any(String::is_empty)
        || package.artifacts.is_empty()
        || artifact_roles.len() != package.artifacts.len()
        || package.artifacts.iter().any(|artifact| {
            artifact.role.is_empty()
                || artifact.media_type.is_empty()
                || artifact.producing_command.is_empty()
        })
    {
        return Err("authority package configuration is incomplete or ambiguous".into());
    }
    let bootstrap = &authority.bootstrap_proof;
    if !safe_relative(&bootstrap.profile)
        || !valid_sha256(&bootstrap.profile_sha256)
        || !safe_relative(&bootstrap.packaged_root)
        || !safe_relative(&bootstrap.packaged_executable)
        || !safe_relative(&bootstrap.packaged_manifest)
    {
        return Err("authority bootstrap proof configuration is invalid".into());
    }
    let ids = authority.gate_ids();
    let unique_ids: BTreeSet<_> = ids.iter().copied().collect();
    if ids.is_empty() || ids.iter().any(|id| id.is_empty()) || unique_ids.len() != ids.len() {
        return Err("authority gate ids must be unique and nonempty".into());
    }
    if ids != MANDATORY_GATE_IDS {
        return Err(format!(
            "authority gates must exactly match the mandatory ordered sequence {MANDATORY_GATE_IDS:?}"
        ));
    }
    let builtin: BTreeSet<_> = BUILTIN_GATES.iter().copied().collect();
    for gate in &authority.gates {
        let expected_owner = expected_gate_owner(&gate.id)
            .ok_or_else(|| format!("gate '{}' has no assigned authority owner", gate.id))?;
        if gate.owner != expected_owner {
            return Err(format!(
                "gate '{}' owner must be '{}', found '{}'",
                gate.id, expected_owner, gate.owner
            ));
        }
        let expected_builtin = builtin.contains(gate.id.as_str());
        if matches!(gate.kind, GateKind::Builtin) != expected_builtin {
            return Err(format!(
                "gate '{}' must have kind '{}'",
                gate.id,
                if expected_builtin {
                    "builtin"
                } else {
                    "process"
                }
            ));
        }
        match gate.kind {
            GateKind::Builtin => {
                if !builtin.contains(gate.id.as_str()) {
                    return Err(format!(
                        "builtin gate '{}' is not a recognized builtin",
                        gate.id
                    ));
                }
                if !gate.steps.is_empty() {
                    return Err(format!(
                        "builtin gate '{}' must not declare raw steps",
                        gate.id
                    ));
                }
            }
            GateKind::Process => {
                if gate.steps.is_empty() {
                    return Err(format!("process gate '{}' has no steps", gate.id));
                }
                let mut step_ids = BTreeSet::new();
                for step in &gate.steps {
                    if step.id.is_empty()
                        || !step_ids.insert(step.id.as_str())
                        || step.cwd.is_empty()
                        || Path::new(&step.cwd).is_absolute()
                        || step.cwd.split('/').any(|part| part == "..")
                        || step.command.is_empty()
                        || step.command.iter().any(|token| token.is_empty())
                        || !(1..=7_200).contains(&step.timeout_seconds)
                        || step
                            .native_profile
                            .as_deref()
                            .is_some_and(|profile| !matches!(profile, "linked-test" | "production"))
                    {
                        return Err(format!("process gate '{}' has a malformed step", gate.id));
                    }
                }
            }
        }
    }
    validate_security_authority(&authority.security)?;
    Ok(())
}

fn validate_security_authority(security: &SecurityAuthority) -> Result<(), String> {
    let path = Path::new(&security.advisory_database_path);
    if !security
        .advisory_database_repository
        .starts_with("https://")
        || !is_lower_hex(&security.advisory_database_revision, 40)
        || security.advisory_database_path.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
        || !is_lower_hex(&security.advisory_database_pack_sha256, 64)
        || security.advisory_database_file_count == 0
    {
        return Err("security advisory database authority is malformed".into());
    }
    Ok(())
}

fn is_lower_hex(value: &str, expected_length: usize) -> bool {
    value.len() == expected_length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_authority() -> Authority {
        let bytes = include_bytes!("../../../ci/gates.json");
        serde_json::from_slice(bytes).unwrap()
    }

    #[test]
    fn runtime_source_binding_requires_a_separate_absolute_authority_path() {
        let root = Path::new("/exact-head");
        assert_eq!(
            runtime_authority_path(root, false, None).unwrap(),
            root.join(AUTHORITY_RELATIVE)
        );
        assert!(runtime_authority_path(root, true, None)
            .unwrap_err()
            .contains("UQM_CI_AUTHORITY_PATH is required"));
        assert!(
            runtime_authority_path(root, true, Some("relative.json".into()))
                .unwrap_err()
                .contains("must be an absolute path")
        );
        assert_eq!(
            runtime_authority_path(root, true, Some("/trusted/gates.json".into())).unwrap(),
            Path::new("/trusted/gates.json")
        );
    }

    #[test]
    fn checked_in_authority_is_valid() {
        let authority = fixture_authority();
        validate_authority(&authority).unwrap();
    }

    #[test]
    fn authority_rejects_unknown_top_level_and_nested_fields() {
        let mut top_level: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        top_level["unexpected"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Authority>(top_level).is_err());

        let mut nested: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        nested["actions"]["unexpected"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Authority>(nested).is_err());
    }

    #[test]
    fn structurally_valid_native_runtime_values_are_authority_owned() {
        let mut authority = fixture_authority();
        authority.native_acceptance.runtime_contract = NativeRuntimeAuthority {
            capture_timeout_ms: 100,
            capture_kill_grace_ms: 10,
            observer_timeout_ms: 200,
            observer_kill_grace_ms: 10,
            acknowledgement_timeout_ms: 500,
            outer_child_timeout_ms: 1_000,
            outer_child_kill_grace_ms: 1,
            child_stdout_budget_bytes: 1,
            child_stderr_budget_bytes: 1,
            observer_response_budget_bytes: 1,
            capture_budget_bytes: 1,
            content_expansion_budget_bytes: 1,
            expected_client_bounds: uqm_rust::automation::native_window::NativeWindowBounds {
                x: 1,
                y: 1,
                width: 1,
                height: 1,
            },
        };
        validate_authority(&authority).unwrap();
    }

    #[test]
    fn native_content_expansion_budget_must_be_nonzero() {
        let mut authority = fixture_authority();
        authority
            .native_acceptance
            .runtime_contract
            .content_expansion_budget_bytes = 0;
        assert!(validate_authority(&authority).is_err());
    }

    #[test]
    fn checked_in_native_script_matches_authority_and_presents_after_battle_assertion() {
        use sha2::{Digest, Sha256};

        let authority = fixture_authority();
        let bytes = include_bytes!("../../../scripts/linked-playable-v1.json");
        let digest = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            bytes.len() as u64,
            authority.native_acceptance.script_byte_length
        );
        assert_eq!(digest, authority.native_acceptance.script_sha256);

        let script: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let steps = script["steps"].as_array().unwrap();
        let assertion = steps
            .iter()
            .position(|step| step["action"] == "assert_battle_frames")
            .unwrap();
        assert_eq!(
            steps[assertion]["minimum"],
            authority
                .native_acceptance
                .acceptance_policy
                .battle_frame_floor
        );
        assert_eq!(steps[assertion + 1]["action"], "wait_presentations");
        assert_eq!(steps[assertion + 1]["count"], 1);
    }

    #[test]
    fn repeated_owners_are_allowed_when_keys_are_valid() {
        let authority = fixture_authority();
        assert!(
            authority
                .gates
                .iter()
                .filter(|gate| gate.owner == "S4")
                .count()
                > 1
        );
        assert!(authority.gates.iter().any(|gate| gate.owner == "S3"));
        assert!(authority.gates.iter().any(|gate| gate.owner == "S1"));
        validate_authority(&authority).unwrap();
    }

    #[test]
    fn authority_requires_the_exact_gate_owner_assignment() {
        let authority = fixture_authority();
        for gate in &authority.gates {
            assert_eq!(Some(gate.owner.as_str()), expected_gate_owner(&gate.id));
        }

        for owner in ["alternate-owner", "S1", ""] {
            let mut mutated = fixture_authority();
            mutated.gates[0].owner = owner.into();
            assert!(
                validate_authority(&mutated).is_err(),
                "accepted non-assigned owner {owner:?}"
            );
        }
    }

    #[test]
    fn authority_requires_the_mandatory_gate_set_kind_and_order() {
        let mut reordered = fixture_authority();
        reordered.gates.swap(0, 1);
        assert!(validate_authority(&reordered).is_err());

        let mut missing = fixture_authority();
        missing.gates.remove(0);
        assert!(validate_authority(&missing).is_err());

        let mut extra = fixture_authority();
        extra.gates.push(extra.gates.last().unwrap().clone());
        assert!(validate_authority(&extra).is_err());

        let mut wrong_kind = fixture_authority();
        wrong_kind.gates[0].kind = GateKind::Builtin;
        wrong_kind.gates[0].steps.clear();
        assert!(validate_authority(&wrong_kind).is_err());
    }

    #[test]
    fn mutation_targets_are_complete_unique_supported_authority() {
        let authority = fixture_authority();
        assert_eq!(authority.mutation_targets.len(), MutationTarget::COUNT);
        assert_eq!(
            authority
                .mutation_targets
                .iter()
                .filter_map(|target| MutationTarget::parse(target))
                .collect::<BTreeSet<_>>()
                .len(),
            MutationTarget::COUNT
        );
    }

    #[test]
    fn gate_lookup_returns_owner() {
        let authority = fixture_authority();
        assert_eq!(
            authority.gate("clippy").map(|gate| gate.owner.as_str()),
            Some("S4")
        );
        assert!(authority.gate("missing").is_none());
    }

    #[test]
    fn machine_authority_controls_exact_command_vectors() {
        let mut authority = fixture_authority();
        let format = authority
            .gates
            .iter_mut()
            .find(|gate| gate.id == "format")
            .unwrap();
        format.steps[0].command = vec!["cargo".into(), "fmt".into()];
        validate_authority(&authority).unwrap();
    }

    #[test]
    fn malformed_native_prerequisite_authority_is_rejected() {
        let mut authority = fixture_authority();
        authority
            .tools
            .native_prerequisites
            .linux
            .push("build-essential".into());
        assert!(validate_authority(&authority)
            .unwrap_err()
            .contains("native prerequisites"));
    }

    #[test]
    fn malformed_native_acceptance_content_identity_is_rejected() {
        let mut zero_uid = fixture_authority();
        zero_uid.native_acceptance.dedicated_execution_uid = 0;
        assert!(validate_authority(&zero_uid)
            .unwrap_err()
            .contains("native acceptance configuration"));

        let mut missing_filename = fixture_authority();
        missing_filename.native_acceptance.content_filename.clear();
        assert!(validate_authority(&missing_filename)
            .unwrap_err()
            .contains("native acceptance configuration"));

        let mut malformed_digest = fixture_authority();
        malformed_digest.native_acceptance.content_sha256 = "not-a-digest".into();
        assert!(validate_authority(&malformed_digest)
            .unwrap_err()
            .contains("native acceptance configuration"));

        let mut zero_length = fixture_authority();
        zero_length.native_acceptance.content_byte_length = 0;
        assert!(validate_authority(&zero_length)
            .unwrap_err()
            .contains("native acceptance configuration"));

        let mut malformed_script_digest = fixture_authority();
        malformed_script_digest.native_acceptance.script_sha256 = "not-a-digest".into();
        assert!(validate_authority(&malformed_script_digest)
            .unwrap_err()
            .contains("native acceptance configuration"));

        let mut zero_script_length = fixture_authority();
        zero_script_length.native_acceptance.script_byte_length = 0;
        assert!(validate_authority(&zero_script_length)
            .unwrap_err()
            .contains("native acceptance configuration"));

        let mut invalid_policy = fixture_authority();
        invalid_policy
            .native_acceptance
            .acceptance_policy
            .playable_presentation_floor = invalid_policy
            .native_acceptance
            .acceptance_policy
            .stable_presentation_floor;
        assert!(validate_authority(&invalid_policy)
            .unwrap_err()
            .contains("native acceptance configuration"));
    }

    #[test]
    fn security_command_and_revision_are_owned_by_machine_authority() {
        let mut authority = fixture_authority();
        let security = authority
            .gates
            .iter_mut()
            .find(|gate| gate.id == "security")
            .unwrap();
        security.steps.last_mut().unwrap().command = vec!["cargo".into(), "audit".into()];
        authority.security.advisory_database_revision = "b".repeat(40);
        validate_authority(&authority).unwrap();
    }

    #[test]
    fn malformed_security_advisory_database_identity_is_rejected() {
        let mut authority = fixture_authority();
        authority.security.advisory_database_revision = "latest".into();
        assert!(validate_authority(&authority)
            .unwrap_err()
            .contains("authority is malformed"));
    }

    #[test]
    fn builtin_gates_must_be_recognized_and_step_free() {
        let mut authority = fixture_authority();
        let builtin = authority
            .gates
            .iter_mut()
            .find(|gate| gate.kind == GateKind::Builtin)
            .unwrap();
        builtin.steps.push(Step {
            id: "extra".into(),
            cwd: ".".into(),
            command: vec!["true".into()],
            timeout_seconds: 60,
            native_profile: None,
        });
        assert!(validate_authority(&authority).is_err());
    }

    #[test]
    fn matrix_derives_exactly_the_four_contract_tuples() {
        let bytes = include_bytes!("../../../build/supported-matrix.json");
        let matrix: Matrix = serde_json::from_slice(bytes).unwrap();
        let tuples = matrix.derive_contract_tuples().unwrap();
        assert_eq!(
            tuples,
            vec![
                "linux-aarch64",
                "linux-x86_64",
                "macos-aarch64",
                "macos-x86_64"
            ]
        );
    }

    #[test]
    fn runner_mapping_tuple_set_exactly_matches_supported_matrix() {
        let authority = fixture_authority();
        let bytes = include_bytes!("../../../build/supported-matrix.json");
        let matrix: Matrix = serde_json::from_slice(bytes).unwrap();
        let expected = matrix.tuples();
        let actual: BTreeSet<String> = authority
            .runner_mapping
            .iter()
            .map(|mapping| mapping.tuple.clone())
            .collect();
        assert_eq!(actual, expected);

        let mut duplicate = authority;
        duplicate.runner_mapping[1].tuple = duplicate.runner_mapping[0].tuple.clone();
        assert!(validate_authority(&duplicate).is_err());
    }

    #[test]
    fn authority_rejects_shell_metacharacters_in_every_plan_tuple_field() {
        for field in ["os", "architecture", "tuple", "runner", "expected_uname"] {
            let mut authority = fixture_authority();
            let mapping = &mut authority.runner_mapping[0];
            let injection = "macos;touch${IFS}/tmp/uqm-injected".to_string();
            match field {
                "os" => mapping.os = injection,
                "architecture" => mapping.architecture = injection,
                "tuple" => mapping.tuple = injection,
                "runner" => mapping.runner = injection,
                "expected_uname" => mapping.expected_uname = injection,
                _ => unreachable!(),
            }
            assert!(
                validate_authority(&authority).is_err(),
                "unsafe authority field {field} was accepted"
            );
        }
    }

    #[test]
    fn compatibility_matrix_rejects_shell_metacharacters_in_all_consumable_fields() {
        let fixture = || {
            serde_json::from_slice::<Matrix>(include_bytes!("../../../build/supported-matrix.json"))
                .unwrap()
        };
        let mutations: [fn(&mut Matrix); 17] = [
            |matrix| matrix.axes.os[0] = "macos;touch".into(),
            |matrix| matrix.axes.architecture[0] = "aarch64$(touch)".into(),
            |matrix| matrix.axes.renderer[0] = "sdl2;software".into(),
            |matrix| matrix.axes.input[0] = "sdl2`touch`".into(),
            |matrix| matrix.axes.content[0] = "uqm content".into(),
            |matrix| matrix.axes.audio[0] = "cpal|touch".into(),
            |matrix| matrix.axes.network[0] = "full&touch".into(),
            |matrix| matrix.axes.package[0] = "directory;manifest".into(),
            |matrix| matrix.supported[0].os = "macos;touch".into(),
            |matrix| matrix.supported[0].architectures[0] = "aarch64;touch".into(),
            |matrix| matrix.supported[0].renderer = "sdl2;software".into(),
            |matrix| matrix.supported[0].input = "sdl2;touch".into(),
            |matrix| matrix.supported[0].content = "uqm;touch".into(),
            |matrix| matrix.supported[0].audio = "cpal;touch".into(),
            |matrix| matrix.supported[0].network = "full;touch".into(),
            |matrix| matrix.supported[0].package = "directory;touch".into(),
            |matrix| matrix.supported[0].prerequisites[0] = "cc;touch".into(),
        ];
        for (index, mutation) in mutations.into_iter().enumerate() {
            let mut matrix = fixture();
            mutation(&mut matrix);
            assert!(
                matrix.validate().is_err(),
                "unsafe matrix mutation {index} was accepted"
            );
        }
    }

    #[test]
    fn workflow_and_native_content_budgets_must_be_bounded() {
        let mut workflow = fixture_authority();
        workflow.workflow.plan_job_timeout_minutes = 0;
        assert!(validate_authority(&workflow).is_err());

        let mut transport = fixture_authority();
        transport
            .native_acceptance
            .content_transport
            .backoff_seconds
            .clear();
        assert!(validate_authority(&transport).is_err());

        let mut read = fixture_authority();
        read.native_acceptance
            .content_transport
            .read_timeout_seconds = 0;
        assert!(validate_authority(&read).is_err());
    }

    #[test]
    fn lizard_distribution_identity_requires_complete_valid_hashes() {
        let mut missing = fixture_authority();
        missing.tools.lizard.distribution_requirements.clear();
        assert!(validate_authority(&missing).is_err());

        let mut malformed = fixture_authority();
        malformed.tools.lizard.distribution_requirements[0].sha256 = "not-a-digest".into();
        assert!(validate_authority(&malformed).is_err());
    }

    #[test]
    fn downloaded_tool_integrity_contracts_are_exact() {
        let mut rust = fixture_authority();
        rust.tools.rust.integrity_identity = "0".repeat(40);
        rust.tools.rust.installation_integrity = "version-output-only".into();
        assert!(validate_authority(&rust).is_err());

        let mut cargo = fixture_authority();
        cargo.tools.cargo_audit.integrity_identity = "not-a-digest".into();
        assert!(validate_authority(&cargo).is_err());

        let mut actionlint = fixture_authority();
        actionlint.tools.actionlint.integrity_identity = "not-a-digest".into();
        assert!(validate_authority(&actionlint).is_err());

        let mut missing_actionlint_archive = fixture_authority();
        missing_actionlint_archive
            .tools
            .actionlint
            .distribution_requirements
            .pop();
        assert!(validate_authority(&missing_actionlint_archive).is_err());
    }

    #[test]
    fn control_plane_identity_drift_is_rejected() {
        let authority = fixture_authority();

        let mut ledger = authority.clone();
        ledger.ledger_identity.sha256 = "not-a-digest".into();
        assert!(validate_authority(&ledger).is_err());

        let mut delta = authority.clone();
        delta.zero_native_delta.providers = 1;
        assert!(validate_authority(&delta).is_err());

        let mut paths = authority.clone();
        paths.control_plane_paths.push("../outside".into());
        assert!(validate_authority(&paths).is_err());

        let mut tools = authority.clone();
        tools.tools.lizard.version.clear();
        assert!(validate_authority(&tools).is_err());

        let mut actions = authority.clone();
        actions.actions.checkout = "v4".into();
        assert!(validate_authority(&actions).is_err());

        let mut ambiguous_action = authority.clone();
        ambiguous_action.actions.checkout = format!("actions/check@out@{}", "0".repeat(40));
        assert!(validate_authority(&ambiguous_action).is_err());

        let mut cache = authority.clone();
        cache.cache.mode = "ambient-dev".into();
        assert!(validate_authority(&cache).is_err());

        let mut roles = authority.clone();
        roles.evidence_roles[0].media_type = "application/octet-stream".into();
        validate_authority(&roles).unwrap();

        let mut profile = authority;
        profile.gates[1].steps[0].native_profile = Some("ambient".into());
        assert!(validate_authority(&profile).is_err());
    }

    #[test]
    fn subprocess_supervision_authority_is_bounded() {
        let authority = fixture_authority();

        let mut no_timeout = authority.clone();
        no_timeout.gates[0].steps[0].timeout_seconds = 0;
        assert!(validate_authority(&no_timeout).is_err());

        let mut changed_timeout = authority.clone();
        changed_timeout.gates[0].steps[0].timeout_seconds = 3_599;
        validate_authority(&changed_timeout).unwrap();

        let mut unbounded_output = authority.clone();
        unbounded_output.supervision.stdout_limit_bytes = 0;
        assert!(validate_authority(&unbounded_output)
            .unwrap_err()
            .contains("supervision limits"));

        let mut unbounded_executable = authority;
        unbounded_executable
            .supervision
            .executable_member_limit_bytes = 0;
        assert!(validate_authority(&unbounded_executable)
            .unwrap_err()
            .contains("supervision limits"));
    }

    #[test]
    fn gate_scope_and_handoff_drift_is_rejected() {
        let authority = fixture_authority();

        let mut duplicate_step = authority.clone();
        duplicate_step.gates[1].steps[1].id = duplicate_step.gates[1].steps[0].id.clone();
        assert!(validate_authority(&duplicate_step).is_err());

        let mut absolute_cwd = authority.clone();
        absolute_cwd.gates[0].steps[0].cwd = "/tmp".into();
        assert!(validate_authority(&absolute_cwd).is_err());

        let mut traversing_cwd = authority.clone();
        traversing_cwd.gates[0].steps[0].cwd = "rust/../sc2".into();
        assert!(validate_authority(&traversing_cwd).is_err());

        let mut handoff = authority.clone();
        handoff.bootstrap_proof.packaged_executable = "../outside".into();
        assert!(validate_authority(&handoff).is_err());

        let mut arbitrary_command = authority;
        arbitrary_command.gates[1].steps[0]
            .command
            .push("--release".into());
        validate_authority(&arbitrary_command).unwrap();
    }

    #[test]
    fn compatibility_matrix_requires_all_authority_operating_system_rows() {
        let bytes = include_bytes!("../../../build/supported-matrix.json");
        let mut matrix: Matrix = serde_json::from_slice(bytes).unwrap();
        matrix.supported.truncate(1);
        assert!(matrix.derive_contract_tuples().is_err());
    }

    #[test]
    fn contradictory_matrix_row_is_rejected() {
        let bytes = include_bytes!("../../../build/supported-matrix.json");
        let mut matrix: Matrix = serde_json::from_slice(bytes).unwrap();
        matrix.supported[0].renderer = "opengl".into();
        assert!(matrix.derive_contract_tuples().is_err());
    }
}
