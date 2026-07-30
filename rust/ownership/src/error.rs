//! Error types for ownership validation.
//!
//! All errors are typed, deterministic, and include the exact repo-relative
//! path or field that violated an invariant. They are designed to be
//! machine-readable so that CI can parse them.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A single validation diagnostic with enough context to locate and fix
/// the problem deterministically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The category of invariant violation.
    pub code: DiagnosticCode,
    /// The repo-relative path or field name involved, if applicable.
    pub path: Option<String>,
    /// Human-readable explanation.
    pub detail: String,
}

/// Categorization of each invariant violation type.
///
/// These codes are stable and part of the machine-readable contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticCode {
    /// Manifest JSON schema or identity is malformed.
    MalformedManifest,
    /// A repo-relative path is not canonical (traversal, absolute, etc.).
    PathViolation,
    /// An object appears twice in the manifest.
    DuplicateObject,
    /// An object exists on disk but is missing from the manifest.
    MissingFromManifest,
    /// An object is declared in the manifest but not found on disk.
    MissingFromDisk,
    /// An object's SHA-256 does not match the manifest.
    StaleObject,
    /// An object's archive decision is unknown or unrecognized.
    UnknownDecision,
    /// An object is on disk but not assigned to any provider.
    UnassignedObject,
    /// The on-disk object set does not match the manifest.
    ManifestDrift,
    /// A provider is declared for an object but is not the sole provider.
    DuplicateProvider,
    /// A required provider object is missing.
    MissingProvider,
    /// A declared symbol is dynamic or unresolved.
    DynamicUnresolvedSymbol,
    /// An excluded object was found in the production archive.
    ExcludedObjectInArchive,
    /// A manifest, object, archive, or tool output could not be read.
    IoError,
}

impl DiagnosticCode {
    /// Returns the stable string identifier for this code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MalformedManifest => "MALFORMED_MANIFEST",
            Self::PathViolation => "PATH_VIOLATION",
            Self::DuplicateObject => "DUPLICATE_OBJECT",
            Self::MissingFromManifest => "MISSING_FROM_MANIFEST",
            Self::MissingFromDisk => "MISSING_FROM_DISK",
            Self::StaleObject => "STALE_OBJECT",
            Self::UnknownDecision => "UNKNOWN_DECISION",
            Self::UnassignedObject => "UNASSIGNED_OBJECT",
            Self::ManifestDrift => "MANIFEST_DRIFT",
            Self::DuplicateProvider => "DUPLICATE_PROVIDER",
            Self::MissingProvider => "MISSING_PROVIDER",
            Self::DynamicUnresolvedSymbol => "DYNAMIC_UNRESOLVED_SYMBOL",
            Self::ExcludedObjectInArchive => "EXCLUDED_OBJECT_IN_ARCHIVE",
            Self::IoError => "IO_ERROR",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(p) => write!(f, "[{}] {}: {}", self.code, p, self.detail),
            None => write!(f, "[{}]: {}", self.code, self.detail),
        }
    }
}

/// The top-level error type returned by validation.
///
/// Accumulates one or more diagnostics. The first diagnostic is always
/// available for fast fail; the full list provides a complete audit trail.
#[derive(Debug, Clone)]
pub struct OwnershipError {
    pub diagnostics: Vec<Diagnostic>,
}

impl OwnershipError {
    /// Create from a single diagnostic.
    pub fn single(code: DiagnosticCode, path: Option<String>, detail: impl Into<String>) -> Self {
        Self {
            diagnostics: vec![Diagnostic {
                code,
                path,
                detail: detail.into(),
            }],
        }
    }

    /// Create from multiple diagnostics.
    pub fn multiple(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    /// Returns true if there are no diagnostics (validation passed).
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Returns the number of diagnostics.
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Convert into the list of diagnostics.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

impl fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.diagnostics.is_empty() {
            return f.write_str("ownership validation passed");
        }
        writeln!(
            f,
            "ownership validation failed ({} violation(s)):",
            self.diagnostics.len()
        )?;
        for d in &self.diagnostics {
            writeln!(f, "  {d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}

impl std::error::Error for OwnershipError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<serde_json::Error> for OwnershipError {
    fn from(e: serde_json::Error) -> Self {
        Self::single(
            DiagnosticCode::MalformedManifest,
            None,
            format!(
                "JSON parse error at line {} column {}: {}",
                e.line(),
                e.column(),
                e
            ),
        )
    }
}

impl From<std::io::Error> for OwnershipError {
    fn from(e: std::io::Error) -> Self {
        Self::single(DiagnosticCode::IoError, None, format!("I/O error: {e}"))
    }
}
