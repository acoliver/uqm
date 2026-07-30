//! Native provider manifest validator and strict-link enforcement.
//!
//! This crate owns the machine-readable native provider manifest used by the
//! transitional UQM production binary. It validates:
//!
//! - Schema and identity of the manifest
//! - Repo-relative path canonicality (no basename matching, no traversal)
//! - Exact object inventory decisions (include/exclude per object)
//! - Provider ownership uniqueness (no duplicate/missing/unassigned symbols)
//! - Stale, unknown, and excluded object detection
//! - Manifest drift between manifest and on-disk object tree
//! - Dynamic or unresolved internal symbols (strict-link enforcement)
//!
//! It generates deterministic machine-readable provider reports.
//!
//! # Architecture
//!
//! The manifest is a checked-in JSON file (`native-provider-manifest.json`)
//! generated from the immutable ownership ledger v3. The validator loads the
//! manifest, optionally scans the on-disk object tree, and checks that every
//! discovered object is declared, every declared object matches its SHA-256,
//! and the archive decisions are internally consistent.
//!
//! All checks are fail-fast: the first invariant violation produces a typed
//! error that halts the build before linking.

pub mod error;
pub mod manifest;
pub mod report;
pub mod validate;

pub use error::{Diagnostic, DiagnosticCode, OwnershipError};
pub use manifest::{
    AcceptedProductionProfile, ArchiveDecision, ExternalImport, LedgerRef, Manifest,
    ManifestObject, NoTrackedNativeChange, ObjectProvider, ProviderKind, ProviderRef,
    RecompiledObject, SymbolContract,
};
pub use report::{
    ArtifactDigest, ProductionArtifactReport, ProviderReport, ReportEntry, ReportSummary,
    SymbolReportEntry,
};
pub use validate::{
    ObservedSymbol, ProductionArtifacts, ProductionNm, SymbolState, ValidateOptions, Validator,
};

/// The canonical path to the checked-in manifest relative to the `rust/`
/// crate root.
pub const MANIFEST_RELATIVE_PATH: &str = "ownership/native-provider-manifest.json";

/// The schema version string this validator expects.
pub const EXPECTED_SCHEMA: &str = "uqm-native-provider-manifest-v1";
pub const EXPECTED_LEDGER_SCHEMA: &str = "uqm-native-ownership-ledger-v3";
pub const EXPECTED_ASSESSMENT_COMMIT: &str = "54e1dba5f56e9f20a3aa773d5f151470a8cf0662";
pub const EXPECTED_LEDGER_RAW_URL: &str = "https://gist.githubusercontent.com/acoliver/03378acffcc0d62e7cfd094fc77c223c/raw/74c5f94716665c3cc649478cf69ac3e60c2687c2/uqm-native-ownership-ledger.json";
pub const EXPECTED_LEDGER_GIST_REVISION: &str = "d1a2a7c00ef4960fd592fdced63592a7c240b979";
pub const EXPECTED_LEDGER_SHA256: &str =
    "9acad7ab2963c6dd4237e14e4ff72cdac2e9adc4ef82c1c32a40c6f8d5d7e746";
pub const EXPECTED_LEDGER_PROJECTION_SHA256: &str =
    "d4ca0fbe2660f4e2048c8a267ffe48186634fb10e26e9cb6dba535a99c6a2a74";
pub const EXPECTED_SCAN_ROOT: &str = "sc2/obj/release";
pub const EXPECTED_OBJECT_COUNT: usize = 339;
pub const DISPLIST_OBJECT: &str = "sc2/obj/release/src/uqm/displist.c.o";
pub const QUEUE_RUST_PROVIDER: &str = "rust/src/collections/queue.rs";
pub const DYNAMIC_LOOKUP_FLAG: &str = "-undefined,dynamic_lookup";
pub const QUEUE_SYMBOLS: [&str; 10] = [
    "AllocLink",
    "CountLinks",
    "ForAllLinks",
    "FreeLink",
    "InitQueue",
    "InsertQueue",
    "PutQueue",
    "ReinitQueue",
    "RemoveQueue",
    "UninitQueue",
];
