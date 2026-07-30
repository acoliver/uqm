//! Deterministic machine-readable provider reports.
//!
//! After validation, a [`ProviderReport`] is generated that lists every
//! object with its decision, provider, and status. The report is sorted
//! deterministically and serialized to JSON for CI consumption.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{Diagnostic, DiagnosticCode};
use crate::manifest::{ArchiveDecision, Manifest, ProviderKind};

/// SHA-256 identity for one verified production artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactDigest {
    /// Exact artifact path supplied by the current build invocation.
    pub path: String,
    /// Lowercase SHA-256 of the artifact bytes.
    pub sha256: String,
}

/// Report binding provider validation to exact production artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductionArtifactReport {
    /// Report schema.
    pub schema: String,
    /// Validated declaration report.
    pub provider_report: ProviderReport,
    /// Exact Rust static archive identity.
    pub rust_archive: ArtifactDigest,
    /// Exact manifest-selected C archive identity.
    pub c_archive: ArtifactDigest,
    /// Exact final executable identity.
    pub executable: ArtifactDigest,
}

impl ProductionArtifactReport {
    /// Serialize to deterministic pretty JSON.
    pub fn to_json(&self) -> Result<String, crate::error::OwnershipError> {
        serde_json::to_string_pretty(self).map_err(crate::error::OwnershipError::from)
    }
}

/// Deterministic symbol ownership declaration included in every report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolReportEntry {
    /// Linker symbol.
    pub symbol: String,
    /// Canonical owner retained from ledger v3.
    pub canonical_owner: String,
    /// Active provider kind.
    pub provider_kind: ProviderKind,
    /// Exact active provider path.
    pub provider_path: String,
    /// Exact excluded provider paths.
    pub excluded_provider_paths: Vec<String>,
}

/// A per-object entry in the provider report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportEntry {
    /// Repo-relative path (relative to scan_root).
    pub path: String,
    /// Owning issue/domain.
    pub issue: String,
    /// Sole provider identity.
    pub provider: String,
    /// Archive decision.
    pub archive_decision: ArchiveDecision,
    /// Validation status: `ok` or `violation`.
    pub status: String,
    /// Diagnostic codes for this entry, if any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

/// Summary statistics for the report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportSummary {
    /// Total objects in the manifest.
    pub total_objects: usize,
    /// Objects included in the production archive.
    pub included: usize,
    /// Objects excluded (all reasons).
    pub excluded: usize,
    /// Objects excluded as duplicate providers.
    pub duplicate_providers_excluded: usize,
    /// Objects recompiled in build.rs.
    pub recompiled: usize,
    /// Objects replaced by Rust.
    pub replaced: usize,
    /// Number of validation violations.
    pub violations: usize,
    /// Whether validation passed overall.
    pub passed: bool,
}

/// The complete deterministic provider report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderReport {
    /// Schema of the report.
    pub schema: String,
    /// All per-object entries, sorted by path.
    pub entries: Vec<ReportEntry>,
    /// Full immutable ledger SHA-256 used to interpret this report.
    pub ledger_sha256: String,
    /// Symbol-provider contracts, sorted by symbol.
    pub symbols: Vec<SymbolReportEntry>,
    /// Validated tracked-native file delta.
    pub tracked_native_file_delta: i32,
    /// Summary statistics.
    pub summary: ReportSummary,
    /// All diagnostics, sorted by path then code.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

impl ProviderReport {
    /// Build a report from a manifest and an optional set of diagnostics.
    ///
    /// The diagnostics are associated with their corresponding object path
    /// where possible. Entries are sorted deterministically by path.
    pub fn from_manifest(manifest: &Manifest, diagnostics: &[Diagnostic]) -> Self {
        // Group diagnostics by path
        let mut diags_by_path: BTreeMap<&str, Vec<&Diagnostic>> = BTreeMap::new();
        for d in diagnostics {
            if let Some(p) = &d.path {
                diags_by_path.entry(p.as_str()).or_default().push(d);
            }
        }

        // Build entries, sorted by path
        let mut entries: Vec<ReportEntry> = manifest
            .objects
            .iter()
            .map(|obj| {
                let obj_diags = diags_by_path.get(obj.path.as_str());
                let status = if obj_diags.is_some() {
                    "violation".to_string()
                } else {
                    "ok".to_string()
                };
                let diag_codes = obj_diags
                    .map(|v| v.iter().map(|d| d.code.as_str().to_string()).collect())
                    .unwrap_or_default();
                ReportEntry {
                    path: obj.path.clone(),
                    issue: obj.issue.clone(),
                    provider: String::from(obj.provider.clone()),
                    archive_decision: obj.archive_decision,
                    status,
                    diagnostics: diag_codes,
                }
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));

        let mut symbols: Vec<_> = manifest
            .symbol_contracts
            .iter()
            .map(|contract| {
                let mut excluded_provider_paths: Vec<_> = contract
                    .excluded_providers
                    .iter()
                    .map(|provider| provider.path.clone())
                    .collect();
                excluded_provider_paths.sort();
                SymbolReportEntry {
                    symbol: contract.symbol.clone(),
                    canonical_owner: contract.canonical_owner.clone(),
                    provider_kind: contract.active_provider.kind,
                    provider_path: contract.active_provider.path.clone(),
                    excluded_provider_paths,
                }
            })
            .collect();
        symbols.sort_by(|left, right| left.symbol.cmp(&right.symbol));

        // Build summary
        let included = manifest
            .objects
            .iter()
            .filter(|o| o.archive_decision == ArchiveDecision::Include)
            .count();
        let duplicate = manifest
            .objects
            .iter()
            .filter(|o| o.archive_decision == ArchiveDecision::ExcludeDuplicateProvider)
            .count();
        let recompiled = manifest
            .objects
            .iter()
            .filter(|o| o.archive_decision == ArchiveDecision::ExcludeRecompiled)
            .count();
        let replaced = manifest
            .objects
            .iter()
            .filter(|o| o.archive_decision == ArchiveDecision::ExcludeReplaced)
            .count();

        let total_violations = diagnostics.len();
        let passed = diagnostics.is_empty();

        // Sort diagnostics deterministically
        let mut sorted_diagnostics: Vec<Diagnostic> = diagnostics.to_vec();
        sorted_diagnostics.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.code.as_str().cmp(b.code.as_str()))
                .then_with(|| a.detail.cmp(&b.detail))
        });

        Self {
            schema: "uqm-provider-report-v1".to_string(),
            entries,
            ledger_sha256: manifest.generated_from_ledger.sha256.clone(),
            symbols,
            tracked_native_file_delta: manifest.no_tracked_native_change.tracked_native_file_delta,
            summary: ReportSummary {
                total_objects: manifest.objects.len(),
                included,
                excluded: duplicate + recompiled + replaced,
                duplicate_providers_excluded: duplicate,
                recompiled,
                replaced,
                violations: total_violations,
                passed,
            },
            diagnostics: sorted_diagnostics,
        }
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, crate::error::OwnershipError> {
        serde_json::to_string_pretty(self).map_err(crate::error::OwnershipError::from)
    }

    /// Returns true if the report indicates validation passed.
    pub fn passed(&self) -> bool {
        self.summary.passed
    }
}

impl fmt::Display for ProviderReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Provider Report: {} objects, {} included, {} excluded, {} violations, passed={}",
            self.summary.total_objects,
            self.summary.included,
            self.summary.excluded,
            self.summary.violations,
            self.summary.passed
        )
    }
}

/// Check if a given object would be flagged as a specific diagnostic code.
pub fn has_diagnostic_for(diagnostics: &[Diagnostic], path: &str, code: DiagnosticCode) -> bool {
    diagnostics
        .iter()
        .any(|d| d.path.as_deref() == Some(path) && d.code == code)
}
