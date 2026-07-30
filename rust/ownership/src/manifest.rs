//! Strongly typed native-provider manifest model.

use crate::error::{Diagnostic, DiagnosticCode, OwnershipError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub schema: String,
    #[serde(default)]
    pub description: String,
    pub generated_from_ledger: LedgerRef,
    pub scan_root: String,
    pub accepted_production_profile: AcceptedProductionProfile,
    #[serde(default)]
    pub recompiled_objects: Vec<RecompiledObject>,
    #[serde(default)]
    pub symbol_contracts: Vec<SymbolContract>,
    #[serde(default)]
    pub external_imports: Vec<ExternalImport>,
    pub no_tracked_native_change: NoTrackedNativeChange,
    pub objects: Vec<ManifestObject>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LedgerRef {
    pub schema: String,
    pub assessment_commit: String,
    pub raw_url: String,
    pub gist_revision: String,
    pub sha256: String,
    pub projection_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcceptedProductionProfile {
    pub cargo_features: Vec<String>,
    pub required_build_vars_flags: Vec<String>,
    pub forbidden_build_vars_flags: Vec<String>,
    pub required_config_defines: Vec<String>,
    pub forbidden_config_defines: Vec<String>,
    pub required_recompile_flags: Vec<String>,
    pub forbidden_recompile_flags: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestObject {
    pub path: String,
    pub issue: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_source: Option<String>,
    pub scan_root: String,
    pub sha256: String,
    pub archive_decision: ArchiveDecision,
    pub provider: ObjectProvider,
    pub reason: String,
    pub producing_command: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveDecision {
    Include,
    ExcludeRecompiled,
    ExcludeReplaced,
    ExcludeDuplicateProvider,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    NativeObject,
    RecompiledNative,
    RustSource,
    ExternalImport,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct ObjectProvider {
    pub kind: ProviderKind,
    pub path: String,
}
impl ObjectProvider {
    pub fn new(kind: ProviderKind, path: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
        }
    }
}
impl TryFrom<String> for ObjectProvider {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let Some((kind, path)) = value.split_once(':') else {
            return Err("provider must be '<kind>:<canonical-path>'".to_string());
        };
        let kind = match kind {
            "native_object" => ProviderKind::NativeObject,
            "recompiled_native" => ProviderKind::RecompiledNative,
            "rust_source" => ProviderKind::RustSource,
            "external_import" => ProviderKind::ExternalImport,
            _ => return Err(format!("unknown provider kind '{kind}'")),
        };
        if path.is_empty() {
            return Err("provider path must not be empty".to_string());
        }
        Ok(Self::new(kind, path))
    }
}
impl From<ObjectProvider> for String {
    fn from(provider: ObjectProvider) -> Self {
        let kind = match provider.kind {
            ProviderKind::NativeObject => "native_object",
            ProviderKind::RecompiledNative => "recompiled_native",
            ProviderKind::RustSource => "rust_source",
            ProviderKind::ExternalImport => "external_import",
        };
        format!("{kind}:{}", provider.path)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRef {
    pub kind: ProviderKind,
    pub path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolContract {
    pub symbol: String,
    pub canonical_owner: String,
    pub active_provider: ProviderRef,
    pub excluded_providers: Vec<ProviderRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalImport {
    pub symbol: String,
    pub provider: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoTrackedNativeChange {
    pub declared: bool,
    pub tracked_native_file_delta: i32,
    pub removed_production_providers: Vec<String>,
    pub removed_permissive_link_modes: Vec<String>,
    pub retained_canonical_sources: Vec<String>,
    pub canonical_owner: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecompiledObject {
    pub repo_relative_path: String,
    pub object_output: String,
    pub producing_command: String,
}

impl Manifest {
    pub fn from_json(data: &[u8]) -> Result<Self, OwnershipError> {
        serde_json::from_slice(data).map_err(OwnershipError::from)
    }
    pub fn from_file(path: &Path) -> Result<Self, OwnershipError> {
        Self::from_json(&std::fs::read(path)?)
    }
    pub fn validate_self(&self) -> Result<(), OwnershipError> {
        let mut diagnostics = Vec::new();
        self.validate_identity(&mut diagnostics);
        self.validate_paths_and_objects(&mut diagnostics);
        self.validate_recompiled(&mut diagnostics);
        self.validate_symbols(&mut diagnostics);
        self.validate_native_change(&mut diagnostics);
        diagnostics.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then(a.code.as_str().cmp(b.code.as_str()))
                .then(a.detail.cmp(&b.detail))
        });
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(OwnershipError::multiple(diagnostics))
        }
    }
    fn validate_identity(&self, diagnostics: &mut Vec<Diagnostic>) {
        let ledger = &self.generated_from_ledger;
        let checks = [
            ("schema", self.schema.as_str(), crate::EXPECTED_SCHEMA),
            (
                "generated_from_ledger.schema",
                ledger.schema.as_str(),
                crate::EXPECTED_LEDGER_SCHEMA,
            ),
            (
                "generated_from_ledger.assessment_commit",
                ledger.assessment_commit.as_str(),
                crate::EXPECTED_ASSESSMENT_COMMIT,
            ),
            (
                "generated_from_ledger.raw_url",
                ledger.raw_url.as_str(),
                crate::EXPECTED_LEDGER_RAW_URL,
            ),
            (
                "generated_from_ledger.gist_revision",
                ledger.gist_revision.as_str(),
                crate::EXPECTED_LEDGER_GIST_REVISION,
            ),
            (
                "generated_from_ledger.sha256",
                ledger.sha256.as_str(),
                crate::EXPECTED_LEDGER_SHA256,
            ),
            (
                "generated_from_ledger.projection_sha256",
                ledger.projection_sha256.as_str(),
                crate::EXPECTED_LEDGER_PROJECTION_SHA256,
            ),
            (
                "scan_root",
                self.scan_root.as_str(),
                crate::EXPECTED_SCAN_ROOT,
            ),
        ];
        for (field, actual, expected) in checks {
            if actual != expected {
                diagnostics.push(diag(
                    DiagnosticCode::MalformedManifest,
                    Some(field),
                    format!("expected '{expected}', got '{actual}'"),
                ));
            }
        }
        if self.objects.len() != crate::EXPECTED_OBJECT_COUNT {
            diagnostics.push(diag(
                DiagnosticCode::ManifestDrift,
                Some("objects"),
                format!(
                    "expected {} ledger objects, got {}",
                    crate::EXPECTED_OBJECT_COUNT,
                    self.objects.len()
                ),
            ));
        }
        let projection = self.ledger_projection_sha256();
        if projection != ledger.projection_sha256 {
            diagnostics.push(diag(
                DiagnosticCode::ManifestDrift,
                Some("objects"),
                format!(
                    "ledger identity projection SHA-256 drift: expected {}, got {projection}",
                    ledger.projection_sha256
                ),
            ));
        }
    }

    fn ledger_projection_sha256(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut digest = Sha256::new();
        for object in &self.objects {
            for field in [
                object.path.as_str(),
                object.issue.as_str(),
                object.canonical_source.as_deref().unwrap_or(""),
                object.scan_root.as_str(),
            ] {
                digest.update(field.as_bytes());
                digest.update([0]);
            }
            digest.update(object.sha256.as_bytes());
            digest.update(b"\n");
        }
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn validate_paths_and_objects(&self, diagnostics: &mut Vec<Diagnostic>) {
        let mut paths = BTreeMap::new();
        let mut archive_names = BTreeMap::new();
        for (index, object) in self.objects.iter().enumerate() {
            check_path(&object.path, diagnostics);
            if !object.path.starts_with(&format!("{}/", self.scan_root)) {
                diagnostics.push(diag(
                    DiagnosticCode::PathViolation,
                    Some(&object.path),
                    format!("object must be beneath {}/", self.scan_root),
                ));
            }
            if object.scan_root != self.scan_root {
                diagnostics.push(diag(
                    DiagnosticCode::ManifestDrift,
                    Some(&object.path),
                    "object scan_root contradicts manifest scan_root",
                ));
            }
            if let Some(previous) = paths.insert(object.path.as_str(), index) {
                diagnostics.push(diag(
                    DiagnosticCode::DuplicateObject,
                    Some(&object.path),
                    format!("declared at indexes {previous} and {index}"),
                ));
            }
            if object.sha256.len() != 64
                || !object
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                diagnostics.push(diag(
                    DiagnosticCode::MalformedManifest,
                    Some(&object.path),
                    "sha256 must be 64 lowercase hexadecimal characters",
                ));
            }
            self.validate_provider(object, diagnostics);
            if object.archive_decision == ArchiveDecision::Include {
                if let Some(name) = Path::new(&object.path).file_name().and_then(|n| n.to_str()) {
                    if let Some(previous) = archive_names.insert(name, object.path.as_str()) {
                        diagnostics.push(diag(DiagnosticCode::DuplicateObject, Some(&object.path), format!("archive member name collides with '{previous}'; basename authority is forbidden")));
                    }
                }
            }
        }
    }
    fn validate_provider(&self, object: &ManifestObject, diagnostics: &mut Vec<Diagnostic>) {
        check_path(&object.provider.path, diagnostics);
        let valid = match object.archive_decision {
            ArchiveDecision::Include => {
                object.provider.kind == ProviderKind::NativeObject
                    && object.provider.path == object.path
            }
            ArchiveDecision::ExcludeRecompiled => {
                object.provider.kind == ProviderKind::RecompiledNative
                    && object.canonical_source.as_deref() == Some(object.provider.path.as_str())
            }
            ArchiveDecision::ExcludeReplaced | ArchiveDecision::ExcludeDuplicateProvider => {
                object.provider.kind == ProviderKind::RustSource
            }
        };
        if !valid {
            diagnostics.push(diag(
                DiagnosticCode::UnassignedObject,
                Some(&object.path),
                "archive decision and provider kind/path contradict each other",
            ));
        }
        if object.producing_command.trim().is_empty() {
            diagnostics.push(diag(
                DiagnosticCode::UnassignedObject,
                Some(&object.path),
                "producing command is required",
            ));
        }
    }
    fn validate_recompiled(&self, diagnostics: &mut Vec<Diagnostic>) {
        let excluded: BTreeSet<_> = self
            .objects
            .iter()
            .filter(|object| object.archive_decision == ArchiveDecision::ExcludeRecompiled)
            .map(|object| object.provider.path.as_str())
            .collect();
        let included_names: BTreeSet<_> = self
            .included_objects()
            .into_iter()
            .filter_map(|object| {
                Path::new(&object.path)
                    .file_name()
                    .and_then(|name| name.to_str())
            })
            .collect();
        let mut sources = BTreeSet::new();
        let mut outputs = BTreeSet::new();

        for item in &self.recompiled_objects {
            check_path(&item.repo_relative_path, diagnostics);
            if item.object_output.contains('/')
                || !item.object_output.ends_with(".o")
                || item.producing_command.trim().is_empty()
            {
                diagnostics.push(diag(DiagnosticCode::MalformedManifest, Some(&item.repo_relative_path), "recompiled output must be an OUT_DIR filename and include its producing command"));
            }
            if !sources.insert(item.repo_relative_path.as_str()) {
                diagnostics.push(diag(
                    DiagnosticCode::DuplicateObject,
                    Some(&item.repo_relative_path),
                    "recompiled source is declared more than once",
                ));
            }
            if !outputs.insert(item.object_output.as_str()) {
                diagnostics.push(diag(
                    DiagnosticCode::DuplicateObject,
                    Some(&item.object_output),
                    "recompiled object output is declared more than once",
                ));
            }
            if included_names.contains(item.object_output.as_str()) {
                diagnostics.push(diag(
                    DiagnosticCode::DuplicateObject,
                    Some(&item.object_output),
                    "recompiled output basename collides with an included archive member",
                ));
            }
        }
        for source in excluded.difference(&sources) {
            diagnostics.push(diag(
                DiagnosticCode::MissingProvider,
                Some(source),
                "excluded recompiled object has no exact recompiled source entry",
            ));
        }
        for source in sources.difference(&excluded) {
            diagnostics.push(diag(
                DiagnosticCode::UnassignedObject,
                Some(source),
                "recompiled source has no ExcludeRecompiled object decision",
            ));
        }
    }
    fn validate_symbols(&self, diagnostics: &mut Vec<Diagnostic>) {
        let mut symbols = BTreeSet::new();
        for contract in &self.symbol_contracts {
            if contract.symbol.is_empty() || !symbols.insert(contract.symbol.as_str()) {
                diagnostics.push(diag(
                    DiagnosticCode::DuplicateProvider,
                    Some(&contract.symbol),
                    "symbol contract must be non-empty and unique",
                ));
            }
            check_path(&contract.active_provider.path, diagnostics);
            if contract.active_provider.kind != ProviderKind::RustSource
                || contract.active_provider.path != crate::QUEUE_RUST_PROVIDER
            {
                diagnostics.push(diag(
                    DiagnosticCode::UnassignedObject,
                    Some(&contract.symbol),
                    "queue symbol active provider must be the canonical Rust queue source",
                ));
            }
            if !contract
                .excluded_providers
                .iter()
                .any(|p| p.kind == ProviderKind::NativeObject && p.path == crate::DISPLIST_OBJECT)
            {
                diagnostics.push(diag(
                    DiagnosticCode::MissingProvider,
                    Some(&contract.symbol),
                    "queue contract must record excluded displist provider",
                ));
            }
        }
        let expected: BTreeSet<_> = crate::QUEUE_SYMBOLS.iter().copied().collect();
        if symbols != expected {
            diagnostics.push(diag(
                DiagnosticCode::ManifestDrift,
                Some("symbol_contracts"),
                "queue symbol inventory differs from the canonical ten exports",
            ));
        }
        let external_names: BTreeSet<_> = self
            .external_imports
            .iter()
            .map(|e| e.symbol.as_str())
            .collect();
        if external_names.len() != self.external_imports.len() {
            diagnostics.push(diag(
                DiagnosticCode::DuplicateProvider,
                Some("external_imports"),
                "external import symbols must be unique",
            ));
        }
    }
    fn validate_native_change(&self, diagnostics: &mut Vec<Diagnostic>) {
        let declaration = &self.no_tracked_native_change;
        let valid = declaration.declared
            && declaration.tracked_native_file_delta == 0
            && declaration.removed_production_providers == [crate::DISPLIST_OBJECT]
            && declaration.removed_permissive_link_modes == [crate::DYNAMIC_LOOKUP_FLAG]
            && declaration.retained_canonical_sources
                == ["sc2/src/uqm/displist.c", "sc2/src/uqm/displist.h"]
            && declaration.canonical_owner == "COLLECTIONS/#37";
        if !valid {
            diagnostics.push(diag(
                DiagnosticCode::ManifestDrift,
                Some("no_tracked_native_change"),
                "declaration differs from ledger v3 S1 boundary",
            ));
        }
        let displist = self
            .objects
            .iter()
            .find(|o| o.path == crate::DISPLIST_OBJECT);
        if !matches!(displist, Some(object) if object.archive_decision == ArchiveDecision::ExcludeDuplicateProvider && object.provider.kind == ProviderKind::RustSource && object.provider.path == crate::QUEUE_RUST_PROVIDER)
        {
            diagnostics.push(diag(
                DiagnosticCode::DuplicateProvider,
                Some(crate::DISPLIST_OBJECT),
                "displist must be excluded with the Rust queue as sole active provider",
            ));
        }
    }
    pub fn sorted_paths(&self) -> Vec<&str> {
        let mut paths: Vec<_> = self.objects.iter().map(|o| o.path.as_str()).collect();
        paths.sort_unstable();
        paths
    }
    pub fn included_objects(&self) -> Vec<&ManifestObject> {
        self.objects
            .iter()
            .filter(|o| o.archive_decision == ArchiveDecision::Include)
            .collect()
    }
    pub fn excluded_objects(&self) -> Vec<&ManifestObject> {
        self.objects
            .iter()
            .filter(|o| o.archive_decision != ArchiveDecision::Include)
            .collect()
    }
    pub fn duplicate_provider_exclusions(&self) -> Vec<&ManifestObject> {
        self.objects
            .iter()
            .filter(|o| o.archive_decision == ArchiveDecision::ExcludeDuplicateProvider)
            .collect()
    }
}
fn check_path(path: &str, diagnostics: &mut Vec<Diagnostic>) {
    let candidate = Path::new(path);
    let canonical = !path.is_empty()
        && !path.contains('\\')
        && !path.ends_with('/')
        && candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !canonical {
        diagnostics.push(diag(
            DiagnosticCode::PathViolation,
            Some(path),
            "path must be a canonical repository-relative path",
        ));
    }
}
fn diag(code: DiagnosticCode, path: Option<&str>, detail: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code,
        path: path.map(str::to_string),
        detail: detail.into(),
    }
}
pub fn manifest_to_json(manifest: &Manifest) -> Result<String, OwnershipError> {
    serde_json::to_string_pretty(manifest).map_err(OwnershipError::from)
}
