//! Fail-fast object, archive, provider, and strict-link validation.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Diagnostic, DiagnosticCode, OwnershipError};
use crate::manifest::{Manifest, ProviderKind};
use crate::report::{ArtifactDigest, ProductionArtifactReport, ProviderReport};

/// Link-time state observed for a first-party symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolState {
    /// The symbol has a concrete first-party definition.
    DefinedInternal,
    /// The first-party symbol has no definition.
    UnresolvedInternal,
    /// The symbol would be deferred to runtime lookup.
    DynamicInternal,
    /// The symbol is an explicitly modeled external import.
    ExternalImport,
}

/// A provider observation produced by `nm` or an equivalent linker tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSymbol {
    /// Linker symbol without a platform-leading underscore.
    pub symbol: String,
    /// Exact canonical provider path, or external provider identity.
    pub provider: String,
    /// Resolution state.
    pub state: SymbolState,
}
/// Raw `nm` observations for the three exact production artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionNm {
    /// Global symbols from the exact Rust static archive.
    pub rust_archive: String,
    /// Global symbols from the exact C static archive.
    pub c_archive: String,
    /// Global symbols from the exact executable.
    pub executable: String,
    /// Darwin-style executable details used to identify dynamic lookup.
    pub executable_details: String,
}

/// Exact production artifacts emitted by one Cargo invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionArtifacts {
    pub rust_archive: PathBuf,
    pub c_archive: PathBuf,
    pub executable: PathBuf,
}

/// Canonical tool paths recorded in production evidence for strict validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionToolPaths {
    /// Exact canonical `ar` executable path from production evidence.
    pub ar: PathBuf,
    /// Exact canonical `nm` executable path from production evidence.
    pub nm: PathBuf,
}

/// The ownership validator.
pub struct Validator {
    manifest: Manifest,
}

/// Validation inputs. Every enabled check is mandatory; missing input fails.
#[derive(Debug, Clone)]
pub struct ValidateOptions {
    /// Repository root.
    pub repo_root: PathBuf,
    /// Validate exact disk inventory and hashes.
    pub check_disk_objects: bool,
    /// Validate the exact repo-relative archive-input sidecar.
    pub check_archive: bool,
    /// Path to `uqm-c-objects.manifest`.
    pub archive_path: Option<PathBuf>,
    /// Reject permissive production linker modes.
    pub check_strict_link: bool,
}

impl ValidateOptions {
    /// Validate immutable identity and manifest invariants only.
    pub fn manifest_only() -> Self {
        Self {
            repo_root: PathBuf::new(),
            check_disk_objects: false,
            check_archive: false,
            archive_path: None,
            check_strict_link: false,
        }
    }

    /// Validate all pre-link production inputs.
    pub fn production(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            check_disk_objects: true,
            check_archive: false,
            archive_path: None,
            check_strict_link: true,
        }
    }
}

impl Validator {
    /// Construct a validator from a parsed manifest.
    pub fn new(manifest: Manifest) -> Self {
        Self { manifest }
    }

    /// Load a checked-in manifest.
    pub fn from_manifest_file(path: &Path) -> Result<Self, OwnershipError> {
        Ok(Self::new(Manifest::from_file(path)?))
    }

    /// Access the typed manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Run configured validation and return a deterministic report.
    pub fn validate(&self, options: &ValidateOptions) -> Result<ProviderReport, OwnershipError> {
        let mut diagnostics = Vec::new();
        if let Err(error) = self.manifest.validate_self() {
            diagnostics.extend(error.diagnostics);
        }
        if !options.repo_root.as_os_str().is_empty() {
            self.check_rust_provider_paths(options, &mut diagnostics);
        }
        if options.check_disk_objects {
            self.check_disk_objects(options, &mut diagnostics);
        }
        if options.check_archive {
            match options.archive_path.as_deref() {
                Some(sidecar) => self.check_archive_inputs(sidecar, &mut diagnostics),
                None => diagnostics.push(diagnostic(
                    DiagnosticCode::MissingProvider,
                    Some("archive_path"),
                    "archive validation requires the exact archive-input sidecar",
                )),
            }
        }
        if options.check_strict_link {
            self.check_strict_link_mode(options, &mut diagnostics);
        }
        sort_diagnostics(&mut diagnostics);
        if diagnostics.is_empty() {
            Ok(ProviderReport::from_manifest(&self.manifest, &[]))
        } else {
            Err(OwnershipError::multiple(diagnostics))
        }
    }

    /// Validate symbol definitions from a production archive or executable.
    pub fn validate_symbols(&self, observed: &[ObservedSymbol]) -> Result<(), OwnershipError> {
        let contracts: BTreeMap<_, _> = self
            .manifest
            .symbol_contracts
            .iter()
            .map(|contract| (contract.symbol.as_str(), contract))
            .collect();
        let externals: BTreeSet<_> = self
            .manifest
            .external_imports
            .iter()
            .map(|external| (external.symbol.as_str(), external.provider.as_str()))
            .collect();
        let mut definitions: BTreeMap<&str, Vec<&ObservedSymbol>> = BTreeMap::new();
        let mut diagnostics = Vec::new();

        for symbol in observed {
            match symbol.state {
                SymbolState::DefinedInternal => {
                    if contracts.contains_key(symbol.symbol.as_str()) {
                        definitions.entry(&symbol.symbol).or_default().push(symbol);
                    } else {
                        diagnostics.push(diagnostic(
                            DiagnosticCode::UnassignedObject,
                            Some(&symbol.symbol),
                            format!(
                                "internal definition from '{}' has no symbol contract",
                                symbol.provider
                            ),
                        ));
                    }
                }
                SymbolState::UnresolvedInternal | SymbolState::DynamicInternal => {
                    diagnostics.push(diagnostic(
                        DiagnosticCode::DynamicUnresolvedSymbol,
                        Some(&symbol.symbol),
                        format!(
                            "internal symbol is {:?} from '{}'",
                            symbol.state, symbol.provider
                        ),
                    ));
                }
                SymbolState::ExternalImport => {
                    if !externals.contains(&(symbol.symbol.as_str(), symbol.provider.as_str())) {
                        diagnostics.push(diagnostic(
                            DiagnosticCode::UnassignedObject,
                            Some(&symbol.symbol),
                            format!(
                                "external import provider '{}' is not allowlisted",
                                symbol.provider
                            ),
                        ));
                    }
                }
            }
        }

        for (name, contract) in contracts {
            match definitions.get(name).map(Vec::as_slice) {
                None | Some([]) => diagnostics.push(diagnostic(
                    DiagnosticCode::MissingProvider,
                    Some(name),
                    "required internal symbol has no definition",
                )),
                Some([definition]) if definition.provider == contract.active_provider.path => {}
                Some([definition]) => diagnostics.push(diagnostic(
                    DiagnosticCode::UnassignedObject,
                    Some(name),
                    format!(
                        "definition provider '{}' differs from canonical '{}'",
                        definition.provider, contract.active_provider.path
                    ),
                )),
                Some(definitions) => diagnostics.push(diagnostic(
                    DiagnosticCode::DuplicateProvider,
                    Some(name),
                    format!("{} active definitions were observed", definitions.len()),
                )),
            }
        }

        sort_diagnostics(&mut diagnostics);
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(OwnershipError::multiple(diagnostics))
        }
    }
    /// Validate parsed observations from the exact Rust archive, C archive, and executable.
    pub fn validate_production_nm(&self, nm: &ProductionNm) -> Result<(), OwnershipError> {
        let mut observed = parse_archive_definitions(&nm.rust_archive, "rust archive", true);
        for definition in &mut observed {
            if let Some(contract) = self
                .manifest
                .symbol_contracts
                .iter()
                .find(|contract| contract.symbol == definition.symbol)
            {
                definition
                    .provider
                    .clone_from(&contract.active_provider.path);
            }
        }
        observed.extend(parse_archive_definitions(
            &nm.c_archive,
            "libuqm_c.a",
            false,
        ));
        observed.extend(parse_executable_failures(
            &nm.executable,
            &nm.executable_details,
        ));
        self.validate_symbols(&observed)
    }

    /// Validate one exact set of production artifacts and bind their hashes to the report.
    pub fn validate_production_artifacts(
        &self,
        artifacts: &ProductionArtifacts,
        tools: &ProductionToolPaths,
    ) -> Result<ProductionArtifactReport, OwnershipError> {
        self.validate_archive_file(&artifacts.c_archive, &tools.ar)?;
        self.validate_symbol_artifacts(artifacts, tools)
    }

    /// Validate strict symbol ownership for a provenance-locked focused link fixture.
    pub fn validate_symbol_artifacts(
        &self,
        artifacts: &ProductionArtifacts,
        tools: &ProductionToolPaths,
    ) -> Result<ProductionArtifactReport, OwnershipError> {
        let nm = ProductionNm {
            rust_archive: run_nm_with(&tools.nm, &["-g", "-A"], &artifacts.rust_archive)?,
            c_archive: run_nm_with(&tools.nm, &["-g", "-A"], &artifacts.c_archive)?,
            executable: run_nm_with(&tools.nm, &["-g", "-A"], &artifacts.executable)?,
            executable_details: executable_details_with(&tools.nm, &artifacts.executable)?,
        };
        self.validate_production_nm(&nm)?;
        Ok(ProductionArtifactReport {
            schema: "uqm-production-artifact-report-v1".into(),
            provider_report: self.generate_report(),
            rust_archive: artifact_digest(&artifacts.rust_archive)?,
            c_archive: artifact_digest(&artifacts.c_archive)?,
            executable: artifact_digest(&artifacts.executable)?,
        })
    }

    /// Require exact equality with the authoritative typed production profile.
    pub fn validate_production_profile(
        &self,
        actual: &crate::native::ProductionProfile,
    ) -> Result<(), OwnershipError> {
        crate::native::validate_profile(actual).map_err(|detail| {
            OwnershipError::single(
                DiagnosticCode::MalformedManifest,
                Some("production_profile".into()),
                detail,
            )
        })?;
        if actual == &self.manifest.accepted_production_profile {
            Ok(())
        } else {
            Err(OwnershipError::single(
                DiagnosticCode::ManifestDrift,
                Some("production_profile".into()),
                "actual production profile does not exactly equal provider authority",
            ))
        }
    }

    /// Validate actual `ar -t` member names against the manifest-selected archive.
    pub fn validate_archive_file(
        &self,
        archive: &Path,
        ar_path: &Path,
    ) -> Result<(), OwnershipError> {
        let members = run_tool_path(ar_path, &["-t"], archive)?;
        let mut diagnostics = Vec::new();
        self.check_archive_members(&members, &mut diagnostics);
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(OwnershipError::multiple(diagnostics))
        }
    }

    /// Generate the declaration report without touching disk.
    pub fn generate_report(&self) -> ProviderReport {
        ProviderReport::from_manifest(&self.manifest, &[])
    }

    fn check_rust_provider_paths(
        &self,
        options: &ValidateOptions,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let providers: BTreeSet<_> = self
            .manifest
            .objects
            .iter()
            .filter(|object| object.provider.kind == ProviderKind::RustSource)
            .map(|object| object.provider.path.as_str())
            .chain(
                self.manifest
                    .symbol_contracts
                    .iter()
                    .filter(|contract| contract.active_provider.kind == ProviderKind::RustSource)
                    .map(|contract| contract.active_provider.path.as_str()),
            )
            .collect();
        for provider in providers {
            if !options.repo_root.join(provider).is_file() {
                diagnostics.push(diagnostic(
                    DiagnosticCode::MissingProvider,
                    Some(provider),
                    "replacement Rust provider path does not exist",
                ));
            }
        }
    }

    fn check_disk_objects(&self, options: &ValidateOptions, diagnostics: &mut Vec<Diagnostic>) {
        for object in &self.manifest.objects {
            let Some(source) = object.canonical_source.as_deref() else {
                continue;
            };
            let path = options.repo_root.join(source);
            if !path.is_file() {
                diagnostics.push(diagnostic(
                    DiagnosticCode::MissingFromDisk,
                    Some(source),
                    "tracked canonical source is absent",
                ));
                continue;
            }
            match sha256_file(&path) {
                Ok(hash) if hash == object.sha256 => {}
                Ok(hash) => diagnostics.push(diagnostic(
                    DiagnosticCode::StaleObject,
                    Some(source),
                    format!(
                        "canonical source SHA-256 drift: manifest={}, disk={hash}",
                        object.sha256
                    ),
                )),
                Err(error) => diagnostics.extend(error.diagnostics),
            }
        }
    }

    fn check_archive_inputs(&self, sidecar: &Path, diagnostics: &mut Vec<Diagnostic>) {
        let content = match fs::read_to_string(sidecar) {
            Ok(content) => content,
            Err(error) => {
                diagnostics.push(diagnostic(
                    DiagnosticCode::MissingProvider,
                    sidecar.to_str(),
                    format!("cannot read archive-input sidecar: {error}"),
                ));
                return;
            }
        };
        let lines: Vec<_> = content.lines().filter(|line| !line.is_empty()).collect();
        let actual: BTreeSet<_> = lines.iter().copied().collect();
        if actual.len() != lines.len() {
            diagnostics.push(diagnostic(
                DiagnosticCode::DuplicateObject,
                sidecar.to_str(),
                "archive-input sidecar contains duplicate lines",
            ));
        }
        let expected: BTreeSet<_> = self
            .manifest
            .included_objects()
            .into_iter()
            .map(|object| object.path.as_str())
            .chain(
                self.manifest
                    .recompiled_objects
                    .iter()
                    .map(|object| object.canonical_source.as_str()),
            )
            .collect();
        for path in actual.difference(&expected) {
            diagnostics.push(diagnostic(
                DiagnosticCode::ExcludedObjectInArchive,
                Some(path),
                "archive input is excluded, stale, or unknown",
            ));
        }
        for path in expected.difference(&actual) {
            diagnostics.push(diagnostic(
                DiagnosticCode::MissingProvider,
                Some(path),
                "required archive input is missing",
            ));
        }
    }

    fn check_archive_members(&self, output: &str, diagnostics: &mut Vec<Diagnostic>) {
        let members: Vec<_> = output
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with("__.SYMDEF"))
            .collect();
        let actual: BTreeSet<_> = members.iter().copied().collect();
        if actual.len() != members.len() {
            diagnostics.push(diagnostic(
                DiagnosticCode::DuplicateObject,
                Some("libuqm_c.a"),
                "actual archive contains duplicate member names",
            ));
        }
        let expected: BTreeSet<_> = self
            .manifest
            .included_objects()
            .into_iter()
            .filter_map(|object| {
                Path::new(&object.path)
                    .file_name()
                    .and_then(|name| name.to_str())
            })
            .chain(
                self.manifest
                    .recompiled_objects
                    .iter()
                    .map(|object| object.object_output.as_str()),
            )
            .collect();
        for member in actual.difference(&expected) {
            diagnostics.push(diagnostic(
                DiagnosticCode::ExcludedObjectInArchive,
                Some(member),
                "actual archive member is excluded, stale, or unknown",
            ));
        }
        for member in expected.difference(&actual) {
            diagnostics.push(diagnostic(
                DiagnosticCode::MissingProvider,
                Some(member),
                "required actual archive member is missing",
            ));
        }
    }

    fn check_strict_link_mode(&self, options: &ValidateOptions, diagnostics: &mut Vec<Diagnostic>) {
        let path = options.repo_root.join("rust/build.rs");
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                diagnostics.push(diagnostic(
                    DiagnosticCode::DynamicUnresolvedSymbol,
                    Some("rust/build.rs"),
                    format!("cannot inspect production linker configuration: {error}"),
                ));
                return;
            }
        };
        for line in content
            .lines()
            .filter(|line| line.contains("cargo:rustc-link-arg"))
        {
            for pattern in [
                crate::DYNAMIC_LOOKUP_FLAG,
                "-undefined dynamic_lookup",
                "-flat_namespace",
            ] {
                if line.contains(pattern) {
                    diagnostics.push(diagnostic(
                        DiagnosticCode::DynamicUnresolvedSymbol,
                        Some("rust/build.rs"),
                        format!("permissive production linker mode '{pattern}' is forbidden"),
                    ));
                }
            }
        }
    }
}

fn parse_archive_definitions(
    output: &str,
    provider: &str,
    rust_archive: bool,
) -> Vec<ObservedSymbol> {
    output
        .lines()
        .filter_map(parse_nm_line)
        .filter_map(|(symbol, kind)| {
            let is_definition = kind != 'U' && kind != 'u';
            if !is_definition {
                return None;
            }
            let canonical_provider = if rust_archive {
                provider.to_string()
            } else {
                format!("{provider}:C-definition")
            };
            Some(ObservedSymbol {
                symbol,
                provider: canonical_provider,
                state: SymbolState::DefinedInternal,
            })
        })
        .collect()
}

fn parse_executable_failures(output: &str, details: &str) -> Vec<ObservedSymbol> {
    let mut observed: Vec<_> = output
        .lines()
        .filter_map(parse_nm_line)
        .filter(|(_, kind)| *kind == 'U' || *kind == 'u')
        .map(|(symbol, _)| ObservedSymbol {
            symbol,
            provider: "production executable".into(),
            state: SymbolState::UnresolvedInternal,
        })
        .collect();
    for line in details
        .lines()
        .filter(|line| line.contains("dynamically looked up"))
    {
        let symbol = dynamic_symbol(line).unwrap_or_else(|| "<dynamic-lookup>".into());
        observed.push(ObservedSymbol {
            symbol,
            provider: "production executable".into(),
            state: SymbolState::DynamicInternal,
        });
    }
    observed
}

fn parse_nm_line(line: &str) -> Option<(String, char)> {
    let tokens: Vec<_> = line.split_whitespace().collect();
    for (index, token) in tokens.iter().enumerate().rev() {
        let Some(symbol) = normalize_contract_symbol(token) else {
            continue;
        };
        let kind = index.checked_sub(1)?.checked_sub(0).and_then(|position| {
            tokens
                .get(position)
                .and_then(|value| value.chars().next())
                .filter(|_| tokens[position].chars().count() == 1)
        })?;
        return Some((symbol, kind));
    }
    None
}

fn dynamic_symbol(line: &str) -> Option<String> {
    let mut tokens = line.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == "external" {
            return tokens.next().map(|symbol| {
                let cleaned =
                    symbol.trim_matches(|character: char| matches!(character, ',' | '(' | ')'));
                cleaned.strip_prefix('_').unwrap_or(cleaned).to_string()
            });
        }
    }
    None
}

fn normalize_contract_symbol(token: &str) -> Option<String> {
    let cleaned = token
        .trim_matches(|character: char| matches!(character, ':' | ',' | '(' | ')' | '[' | ']'));
    let symbol = cleaned.strip_prefix('_').unwrap_or(cleaned);
    (crate::QUEUE_SYMBOLS.contains(&symbol) || crate::HASH_TABLE_SYMBOLS.contains(&symbol))
        .then(|| symbol.to_string())
}

fn run_tool_path(
    tool: &Path,
    arguments: &[&str],
    artifact: &Path,
) -> Result<String, OwnershipError> {
    let output = Command::new(tool)
        .args(arguments)
        .arg(artifact)
        .output()
        .map_err(|error| tool_error(&tool.to_string_lossy(), artifact, error.to_string()))?;
    if !output.status.success() {
        return Err(tool_error(
            &tool.to_string_lossy(),
            artifact,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| tool_error(&tool.to_string_lossy(), artifact, error.to_string()))
}

fn run_nm_with(
    nm_path: &Path,
    arguments: &[&str],
    artifact: &Path,
) -> Result<String, OwnershipError> {
    let output = Command::new(nm_path)
        .args(arguments)
        .arg(artifact)
        .output()
        .map_err(|error| tool_error(&nm_path.to_string_lossy(), artifact, error.to_string()))?;
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| tool_error(&nm_path.to_string_lossy(), artifact, error.to_string()))?;
    if output.status.success() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let known_reader_diagnostics = stderr
        .lines()
        .filter(|line| !line.trim().is_empty())
        .all(|line| line.contains("Unknown attribute kind") || line.ends_with("no symbols"));
    if known_reader_diagnostics && !stdout.is_empty() {
        Ok(stdout)
    } else {
        Err(tool_error(&nm_path.to_string_lossy(), artifact, stderr))
    }
}

fn executable_details_with(nm_path: &Path, executable: &Path) -> Result<String, OwnershipError> {
    if cfg!(target_os = "macos") {
        run_nm_with(nm_path, &["-g", "-m", "-A"], executable)
    } else {
        Ok(String::new())
    }
}

fn artifact_digest(path: &Path) -> Result<ArtifactDigest, OwnershipError> {
    let bytes = fs::read(path).map_err(|error| hash_io_error(path, error))?;
    Ok(ArtifactDigest {
        path: path.to_string_lossy().into_owned(),
        sha256: hex_sha256(&bytes),
    })
}

fn tool_error(tool: &str, path: &Path, detail: impl Into<String>) -> OwnershipError {
    OwnershipError::single(
        DiagnosticCode::IoError,
        Some(path.to_string_lossy().into_owned()),
        format!("{tool} failed: {}", detail.into()),
    )
}

fn sha256_file(path: &Path) -> Result<String, OwnershipError> {
    fs::read(path)
        .map(|bytes| hex_sha256(&bytes))
        .map_err(|error| hash_io_error(path, error))
}

fn hash_io_error(path: &Path, error: std::io::Error) -> OwnershipError {
    OwnershipError::single(
        DiagnosticCode::IoError,
        Some(path.to_string_lossy().into_owned()),
        format!("cannot hash file: {error}"),
    )
}

/// Compute lowercase SHA-256 for manifest drift checks.
pub fn hex_sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn diagnostic(code: DiagnosticCode, path: Option<&str>, detail: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code,
        path: path.map(str::to_string),
        detail: detail.into(),
    }
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.code.as_str().cmp(right.code.as_str()))
            .then(left.detail.cmp(&right.detail))
    });
}
